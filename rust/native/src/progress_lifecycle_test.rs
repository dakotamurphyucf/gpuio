//! Production View regression; TestPlatform is not native GPU/AX acceptance.
use super::super::*;
use gpui::TestAppContext;
use gpuio_protocol::HandlerId;
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
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

#[test]
fn inert_progress_retains_determinate_and_static_indeterminate_artwork() {
    for fraction in [Some(0.5), None] {
        let mut app = TestAppContext::single();
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Progress", 100., 100.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(window_id, session.clone(), transport));
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::Create(id(0), Kind::Container, String::new(), None),
                        Op::Create(id(1), Kind::Progress, String::new(), None),
                        Op::SetProgress(
                            id(1),
                            ProgressConfig {
                                label: "Downloading".into(),
                                fraction,
                            },
                        ),
                        Op::Splice(id(0), 0, 0, vec![id(1)]),
                        Op::SetRoot(Some(id(0))),
                    ],
                );
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        let weak = owner.read_with(cx, |view, _| Rc::downgrade(&view.progress_probes[&id(1)]));
        assert!(weak.upgrade().unwrap().get().count > 0);
        for inert_owner in [id(0), id(1)] {
            let before = weak.upgrade().unwrap().get().count;
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![Op::SetStyle(
                            inert_owner,
                            vec![Style::Fields(vec![Field::Inert(true)])],
                        )],
                    );
                });
                window.draw(cx).clear(cx);
            });
            let paint = weak.upgrade().unwrap().get();
            assert!(
                paint.count > before,
                "inert progress must keep its artwork: {fraction:?}"
            );
            assert_eq!(
                paint.bounds.size.width,
                px(if fraction.is_some() { 100. } else { 50. })
            );
            owner.read_with(cx, |view, _| {
                assert!(!view.focus.borrow().visible(id(1)));
                assert!(Rc::ptr_eq(
                    &weak.upgrade().unwrap(),
                    &view.progress_probes[&id(1)]
                ));
            });
            for _ in 0..3 {
                cx.update(|window, cx| {
                    window.simulate_next_frame(cx);
                    window.draw(cx).clear(cx);
                });
                cx.run_until_parked();
            }
            cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
            let count = weak.upgrade().unwrap().get().count;
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![Op::SetStyle(
                            inert_owner,
                            vec![Style::Fields(vec![
                                Field::Inert(true),
                                Field::Visibility(1),
                            ])],
                        )],
                    );
                });
                window.draw(cx).clear(cx);
            });
            assert_eq!(
                weak.upgrade().unwrap().get().count,
                count,
                "hidden must not paint"
            );
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(view, window, cx, vec![Op::SetStyle(inert_owner, vec![])])
                });
                window.draw(cx).clear(cx);
                assert_eq!(window.simulate_next_frame(cx) > 0, fraction.is_none());
            });
        }
        cx.update(|window, cx| {
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
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        assert!(weak.upgrade().is_none());
        assert_eq!(session.borrow().retained_bytes(), 0);
    }
}

#[test]
fn circular_center_retains_native_control_and_owner_closes_with_retained_view() {
    use gpuio_protocol::animation::Easing;
    use gpuio_protocol::progress_presentation::{Config, Shape, Transition};
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Circle", 200., 200.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(window_id, session.clone(), transport));
    let config = |fraction| Config {
        progress: ProgressConfig {
            label: "Circle".into(),
            fraction,
        },
        shape: Shape::Circle,
        transition: Transition::Tween {
            duration_ms: 200,
            easing: Easing::Linear,
        },
    };
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.progress_clock.set_test_time(std::time::Duration::ZERO);
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Progress, "".into(), None),
                    Op::SetProgressPresentation(id(0), config(Some(0.25))),
                    Op::SetStyle(
                        id(0),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(160.)),
                            Field::Height(Length::Px(160.)),
                        ])],
                    ),
                    Op::Create(
                        id(1),
                        Kind::Button,
                        "Cancel".into(),
                        Some(HandlerId::from_parts(0, 1).unwrap()),
                    ),
                    Op::SetStyle(
                        id(1),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(60.)),
                            Field::Height(Length::Px(24.)),
                        ])],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let focus = owner.read_with(cx, |view, _| {
        let probes = view.probes.borrow();
        assert_eq!(
            probes[&id(0)].bounds.center(),
            probes[&id(1)].bounds.center()
        );
        assert_eq!(view.progresses.len(), 1);
        view.buttons[&id(1)].focus.clone()
    });
    cx.update(|window, cx| {
        window.focus(&focus, cx);
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetProgressPresentation(id(0), config(Some(1.)))],
            )
        });
        window.draw(cx).clear(cx);
        assert!(focus.is_focused(window));
    });
    cx.run_until_parked();
    owner.read_with(cx, |view, _| {
        // Semantic target changes immediately; native artwork is still tweening.
        assert_eq!(
            view.session
                .borrow()
                .tree(window_id)
                .unwrap()
                .get(id(0))
                .unwrap()
                .progress
                .as_ref()
                .unwrap()
                .fraction,
            Some(1.)
        );
        assert_eq!(
            view.progresses[&id(0)]
                .driver()
                .sample(false)
                .unwrap()
                .value,
            crate::progress_clock::Value::Determinate(0.25)
        );
    });
    cx.update(|window, cx| {
        owner.update(cx, |view, _| {
            view.progress_clock
                .set_test_time(std::time::Duration::from_millis(200))
        });
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
        assert!(focus.is_focused(window));
    });
    cx.run_until_parked();
    let stale = owner.read_with(cx, |view, _| view.progresses[&id(0)].driver());
    assert_eq!(
        stale.sample(false).unwrap().value,
        crate::progress_clock::Value::Determinate(1.)
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetProgressPresentation(id(0), config(None))],
            )
        });
        window.draw(cx).clear(cx);
    });
    let pending = owner.read_with(cx, |view, _| view.progresses[&id(0)].driver());
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    owner.read_with(cx, |view, _| assert!(view.progresses.is_empty()));
    assert!(pending.sample(false).is_none());
    assert!(stale.sample(false).is_none());
    session.borrow_mut().close(window_id).unwrap();
    assert_eq!(session.borrow().retained_bytes(), 0);
}

#[test]
fn independent_window_tweens_survive_other_window_close_and_shape_replacement() {
    use crate::progress_clock::Value;
    use gpuio_protocol::{
        animation::Easing,
        progress_presentation::{Config, Shape, Transition},
    };
    use std::time::Duration;
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let config = |duration_ms, fraction, shape| Config {
        progress: ProgressConfig {
            label: "Window progress".into(),
            fraction: Some(fraction),
        },
        shape,
        transition: Transition::Tween {
            duration_ms,
            easing: Easing::Linear,
        },
    };
    let mut windows = Vec::new();
    for slot in 0..2 {
        let id_window = WindowId::from_parts(slot, 1).unwrap();
        session
            .borrow_mut()
            .open(slot + 1, id_window, "Progress", 200., 200.)
            .unwrap();
        let (view, cx) =
            app.add_window_view(|_, _| View::new(id_window, session.clone(), transport.clone()));
        let window = cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.progress_clock.set_test_time(Duration::ZERO);
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::Create(id(0), Kind::Progress, "".into(), None),
                        Op::SetProgressPresentation(
                            id(0),
                            config((slot + 1) * 1000, 0., Shape::Circle),
                        ),
                        Op::SetRoot(Some(id(0))),
                    ],
                );
            });
            window.draw(cx).clear(cx);
            view.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetProgressPresentation(
                        id(0),
                        config((slot + 1) * 1000, 1., Shape::Circle),
                    )],
                )
            });
            window.draw(cx).clear(cx);
            window.window_handle()
        });
        windows.push((view, window, id_window));
    }
    app.run_until_parked();
    for (index, (view, window, _)) in windows.iter().enumerate() {
        window
            .update(&mut app, |_, window, cx| {
                view.update(cx, |view, _| {
                    view.progress_clock
                        .set_test_time(Duration::from_millis(500))
                });
                window.simulate_next_frame(cx);
                window.draw(cx).clear(cx);
                view.read_with(cx, |view, _| {
                    assert_eq!(
                        view.progresses[&id(0)]
                            .driver()
                            .sample(false)
                            .unwrap()
                            .value,
                        Value::Determinate(if index == 0 { 0.5 } else { 0.25 })
                    )
                });
            })
            .unwrap();
    }
    app.run_until_parked();
    let stale = windows[0]
        .0
        .read_with(&app, |view, _| view.progresses[&id(0)].driver());
    windows[0]
        .1
        .update(&mut app, |_, window, _| window.remove_window())
        .unwrap();
    app.run_until_parked();
    assert!(stale.sample(false).is_none());
    session.borrow_mut().close(windows[0].2).unwrap();
    windows[1]
        .1
        .update(&mut app, |_, window, cx| {
            windows[1].0.update(cx, |view, cx| {
                // Shape alone preserves the active tween; the other window's disposal
                // must neither finish nor restart this native owner.
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetProgressPresentation(
                        id(0),
                        config(2000, 1., Shape::Linear),
                    )],
                );
                view.progress_clock
                    .set_test_time(Duration::from_millis(1000));
            });
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            windows[1].0.read_with(cx, |view, _| {
                let sample = view.progresses[&id(0)].driver().sample(false).unwrap();
                assert_eq!(sample.shape, Shape::Linear);
                assert_eq!(sample.value, Value::Determinate(0.5));
            });
        })
        .unwrap();
    app.run_until_parked();
    windows[1]
        .1
        .update(&mut app, |_, window, cx| {
            cx.set_reduce_motion(true);
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            windows[1].0.read_with(cx, |view, _| {
                assert_eq!(
                    view.progresses[&id(0)].driver().sample(true).unwrap().value,
                    Value::Determinate(1.)
                )
            });
        })
        .unwrap();
    app.run_until_parked();
    windows[1]
        .1
        .update(&mut app, |_, window, cx| {
            assert_eq!(window.simulate_next_frame(cx), 0);
            window.remove_window();
        })
        .unwrap();
    app.run_until_parked();
    windows[1]
        .0
        .read_with(&app, |view, _| assert!(view.progresses.is_empty()));
    session.borrow_mut().close(windows[1].2).unwrap();
    assert_eq!(session.borrow().retained_bytes(), 0);
}
