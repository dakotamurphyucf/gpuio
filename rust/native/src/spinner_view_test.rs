//! Production View/Session on TestPlatform. No physical GPU or OS AX claim.
use super::*;
use gpui::TestAppContext;
use gpuio_protocol::{ResourceId, animation::Easing, asset::Format, spinner::Config};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let applied = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations,
        })
        .unwrap();
    view.update_editors(&applied.dirty, window, cx);
    cx.notify();
}
fn upload(session: &mut crate::session::Session, format: Format, bytes: &[u8]) -> ResourceId {
    let store = session.assets().unwrap();
    let id = store.begin(format, bytes.len()).unwrap();
    store.append(id, 0, bytes).unwrap();
    store.finish(id).unwrap();
    id
}
fn config(source: Option<ImageSource>) -> Config {
    Config {
        label: "Building".into(),
        animated: true,
        period_ms: 1000,
        easing: Easing::Linear,
        source,
    }
}

#[test]
fn retained_spinner_uses_masks_publishes_asset_state_and_cleans_up_on_legacy_reset_and_close() {
    let mut app = TestAppContext::single();
    app.update(image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(crate::transport::Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(crate::session::Session::default()));
    let window_id = gpuio_protocol::WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Spinner", 100., 100.)
        .unwrap();
    let svg = upload(&mut session.borrow_mut(), Format::Svg,
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="8"><path fill="red" d="M0 0 L16 4 L0 8 Z"/></svg>"#);
    let configured = config(Some(ImageSource::Reference(svg)));
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::Loading, "".into(), Some(handler(1))),
                    Op::SetSpinner(id(1), configured.clone()),
                    Op::SetStyle(
                        id(0),
                        vec![Style::Fields(vec![Field::Foreground(Color::Rgba(
                            0x22bb44ff,
                        ))])],
                    ),
                    Op::SetStyle(
                        id(1),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(40.)),
                            Field::Height(Length::Px(40.)),
                        ])],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        // The binding acquired a native lease during accepted application.
        session.borrow_mut().assets().unwrap().release(svg).unwrap();
    });
    for _ in 0..4 {
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
    }
    cx.run_until_parked();
    owner.read_with(cx, |view, _| {
        assert!(view.images[&id(1)].binding.borrow().mask);
        assert!(matches!(
            view.images[&id(1)].emitted,
            Some((_, ImageState::Ready(_)))
        ));
        assert_eq!(view.spinners.len(), 1);
    });
    cx.update(|window, _| {
        let sprites = window.painted_monochrome_sprites();
        assert_eq!(sprites.len(), 1);
        let expected: gpui::Hsla = gpui::rgba(0x22bb44ff).into();
        assert_eq!(sprites[0].color, expected);
    });
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|event| matches!(event, Event::ImageState(_, _, h, _, ImageState::Ready(_)) if *h == handler(1))));
    let driver = owner.update(cx, |view, _| view.spinners[&id(1)].driver());
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Bind(id(1), None),
                    Op::SetLoading(id(1), configured.loading()),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    assert!(driver.sample(false).is_none());
    owner.read_with(cx, |view, _| {
        assert!(view.spinners.is_empty() && view.images.is_empty())
    });
    // A non-SVG reference passes wire validation but must fail native acquisition.
    let raster = upload(
        &mut session.borrow_mut(),
        Format::Pnm,
        b"P6\n1 1\n255\n\xff\x00\x00",
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Bind(id(1), Some(handler(2))),
                    Op::SetSpinner(id(1), config(Some(ImageSource::Reference(raster)))),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    owner.read_with(cx, |view, _| {
        assert_eq!(
            view.images[&id(1)].emitted,
            Some((handler(2), ImageState::Failed(ImageError::Unsupported)))
        );
    });
    let driver = owner.update(cx, |view, _| view.spinners[&id(1)].driver());
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    assert!(
        driver.sample(false).is_none(),
        "closed window clears owners despite a retained View Entity"
    );
    owner.read_with(cx, |view, _| {
        assert!(view.spinners.is_empty() && view.images.is_empty())
    });
}

#[test]
fn retained_spinner_inert_and_hidden_branches_pause_without_losing_the_owner() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(crate::transport::Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(crate::session::Session::default()));
    let window_id = gpuio_protocol::WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Spinner", 100., 100.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(window_id, session.clone(), transport));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::Loading, "".into(), None),
                    Op::SetSpinner(id(1), config(None)),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        window.draw(cx).clear(cx);
        assert!(window.simulate_next_frame(cx) > 0);
    });
    for fields in [
        vec![Field::Inert(true)],
        vec![Field::Visibility(1)],
        vec![Field::Opacity(0.)],
    ] {
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetStyle(id(0), vec![Style::Fields(fields)])],
                )
            });
        });
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                window.simulate_next_frame(cx);
            });
            cx.run_until_parked();
        }
        cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
        owner.read_with(cx, |view, _| assert_eq!(view.spinners.len(), 1));
    }
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![Op::SetStyle(id(0), vec![])])
        });
        window.draw(cx).clear(cx);
        assert!(window.simulate_next_frame(cx) > 0);
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Splice(id(0), 0, 1, vec![]),
                    Op::Remove(id(1)),
                    Op::Remove(id(0)),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    owner.read_with(cx, |view, _| assert!(view.spinners.is_empty()));
}

#[test]
fn closing_a_never_painted_spinner_releases_its_retired_source() {
    let mut app = TestAppContext::single();
    app.update(image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(crate::transport::Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(crate::session::Session::default()));
    let window_id = gpuio_protocol::WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Hidden spinner", 100., 100.)
        .unwrap();
    let source = upload(&mut session.borrow_mut(), Format::Svg,
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8"/></svg>"#);
    let (owner, cx) = app.add_window_view(|_, _| View::new(window_id, session.clone(), transport));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Loading, "".into(), None),
                    Op::SetSpinner(id(0), config(Some(ImageSource::Reference(source)))),
                    Op::SetStyle(id(0), vec![Style::Fields(vec![Field::Visibility(1)])]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        session
            .borrow_mut()
            .assets()
            .unwrap()
            .release(source)
            .unwrap();
        window.draw(cx).clear(cx);
    });
    owner.read_with(cx, |view, _| {
        assert!(view.spinners.is_empty());
        assert_eq!(view.images.len(), 1);
    });
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    owner.read_with(cx, |view, _| {
        assert!(view.images.is_empty() && view.spinners.is_empty())
    });
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
}

#[test]
fn spinner_observations_revalidate_source_handler_and_unmount_before_publication() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(crate::transport::Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(crate::session::Session::default()));
    let window_id = gpuio_protocol::WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Events", 100., 100.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Loading, "".into(), Some(handler(1))),
                    Op::SetSpinner(
                        id(0),
                        config(Some(ImageSource::Unavailable(ImageError::Released))),
                    ),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            {
                let session = session.borrow();
                let tree = session.tree(window_id).unwrap();
                view.observe_image(tree, tree.get(id(0)).unwrap(), window, cx);
            }
            // Same callback, different source: the queued Released observation must
            // not escape just because its callback still exists.
            apply(
                view,
                window,
                cx,
                vec![Op::SetSpinner(
                    id(0),
                    config(Some(ImageSource::Unavailable(ImageError::WrongApplication))),
                )],
            );
        })
    });
    cx.run_until_parked();
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ImageState(_, _, _, _, ImageState::Failed(ImageError::Released))
    )));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            // Queue a current observation, then replace only the handler before its
            // deferred publication. No decode race or sleep is needed.
            apply(view, window, cx, vec![Op::Bind(id(0), Some(handler(2)))]);
            {
                let session = session.borrow();
                let tree = session.tree(window_id).unwrap();
                view.observe_image(tree, tree.get(id(0)).unwrap(), window, cx);
            }
            apply(view, window, cx, vec![Op::Bind(id(0), Some(handler(3)))]);
            let session = session.borrow();
            let tree = session.tree(window_id).unwrap();
            view.observe_image(tree, tree.get(id(0)).unwrap(), window, cx);
        })
    });
    cx.run_until_parked();
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ImageState(_, _, h, ..) if *h == handler(2)))
    );
    assert_eq!(events.iter().filter(|event| matches!(event,
        Event::ImageState(_, _, h, _, ImageState::Failed(ImageError::WrongApplication)) if *h == handler(3))).count(), 1);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![Op::Bind(id(0), Some(handler(4)))]);
            {
                let session = session.borrow();
                let tree = session.tree(window_id).unwrap();
                view.observe_image(tree, tree.get(id(0)).unwrap(), window, cx);
            }
            apply(view, window, cx, vec![Op::SetRoot(None), Op::Remove(id(0))]);
        })
    });
    cx.run_until_parked();
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::ImageState(..)))
    );
}

#[test]
fn shared_spinner_asset_keeps_independent_window_clocks_and_close_lifetimes() {
    let mut app = TestAppContext::single();
    app.update(image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(crate::transport::Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(crate::session::Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let source = upload(&mut session.borrow_mut(), Format::Svg,
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8"/></svg>"#);
    let mut windows = Vec::new();
    for slot in 0..2 {
        let window_id = gpuio_protocol::WindowId::from_parts(slot, 1).unwrap();
        session
            .borrow_mut()
            .open(slot + 1, window_id, "Shared spinner", 100., 100.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
        let window = cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                view.spinner_clock.set_test_time(std::time::Duration::ZERO);
                let mut config = config(Some(ImageSource::Reference(source)));
                config.period_ms = (slot + 1) * 1000;
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::Create(id(0), Kind::Loading, "".into(), Some(handler(1))),
                        Op::SetSpinner(id(0), config),
                        Op::SetRoot(Some(id(0))),
                    ],
                );
            });
            window.draw(cx).clear(cx);
            window.window_handle()
        });
        windows.push((owner, window));
    }
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(source)
        .unwrap();
    for _ in 0..4 {
        app.run_until_parked();
        for (_, window) in &windows {
            window
                .update(&mut app, |_, window, cx| {
                    window.draw(cx).clear(cx);
                })
                .unwrap();
        }
    }
    for (index, (owner, window)) in windows.iter().enumerate() {
        window
            .update(&mut app, |_, window, cx| {
                owner.update(cx, |view, cx| {
                    assert!(matches!(
                        view.images[&id(0)].emitted,
                        Some((_, ImageState::Ready(_)))
                    ));
                    view.spinner_clock
                        .set_test_time(std::time::Duration::from_millis(500));
                    cx.notify();
                });
                window.draw(cx).clear(cx);
                owner.read_with(cx, |view, _| {
                    assert_eq!(
                        view.spinners[&id(0)].driver().sample(false),
                        Some(if index == 0 { 0.5 } else { 0.25 })
                    )
                });
            })
            .unwrap();
    }
    windows[0]
        .1
        .update(&mut app, |_, window, _| window.remove_window())
        .unwrap();
    app.run_until_parked();
    windows[0].0.read_with(&app, |view, _| {
        assert!(view.spinners.is_empty() && view.images.is_empty())
    });
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 1);
    windows[1]
        .1
        .update(&mut app, |_, window, cx| {
            windows[1].0.update(cx, |view, cx| {
                view.spinner_clock
                    .set_test_time(std::time::Duration::from_millis(1000));
                cx.notify();
            });
            window.draw(cx).clear(cx);
            windows[1].0.read_with(cx, |view, _| {
                assert_eq!(view.spinners[&id(0)].driver().sample(false), Some(0.5));
                assert_eq!(view.images.len(), 1);
            });
            window.remove_window();
        })
        .unwrap();
    app.run_until_parked();
    windows[1].0.read_with(&app, |view, _| {
        assert!(view.spinners.is_empty() && view.images.is_empty())
    });
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
}
