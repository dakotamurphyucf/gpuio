//! Production focus/paint/activation on TestPlatform; no OS input/AX claim.
use super::*;
use gpui::{KeyDownEvent, KeyUpEvent, Keystroke, PlatformInput, TestAppContext, VisualTestContext};
use gpuio_protocol::{
    HandlerId,
    button::{Config, Content, Focus, Policy},
    checkable::TabOrder,
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let result = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations,
        })
        .unwrap();
    view.update_editors(&result.dirty, window, cx);
    view.list_actions(&result.lists, window, cx);
    cx.notify();
}
// simulate_keystrokes sends key-down only; native clicks require key-up too.
fn activate(cx: &mut VisualTestContext, key: &str) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let keystroke = Keystroke::parse(key).unwrap();
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke }), cx);
    });
}
fn config(loading: bool, focus: Focus) -> Op {
    Op::SetButtonPresentation(
        id(2),
        Some(Config {
            policy: Policy { loading, focus },
            content: Content::Rich,
        }),
    )
}
#[test]
fn loading_and_disabled_styles_override_live_hover_and_press_without_replacing_owner() {
    loading_styles(Kind::Button);
}
#[test]
fn link_loading_keeps_focus_blocks_held_and_keyboard_activation_and_restores_styles() {
    loading_styles(Kind::Link);
}
fn loading_styles(kind: Kind) {
    use gpui::{MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent};
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Styles", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    let background = |rgba| Field::Background(Fill::Solid(Color::Rgba(rgba)));
    let link_config = |loading, disabled| gpuio_protocol::link::Config {
        label: "Guide".into(),
        loading,
        disabled,
        tab_stop: true,
        tab_index: -1,
    };
    let policy = |loading| {
        if kind == Kind::Link {
            return Op::SetLink(id(1), link_config(loading, false));
        }
        Op::SetButtonPresentation(
            id(1),
            Some(Config {
                policy: Policy {
                    loading,
                    focus: Focus::default(),
                },
                content: Content::IconSlots,
            }),
        )
    };
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(
                        id(0),
                        Kind::Container,
                        "".into(),
                        (kind == Kind::Link).then_some(handler),
                    ),
                    Op::SetStyle(id(0), vec![Style::Padding(20.)]),
                    Op::Create(
                        id(1),
                        kind,
                        if kind == Kind::Link {
                            String::new()
                        } else {
                            "Styled action".into()
                        },
                        Some(handler),
                    ),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Fields(vec![
                                Field::Width(Length::Px(100.)),
                                Field::Height(Length::Px(40.)),
                                background(0x444444ff),
                            ]),
                            Style::State(1, vec![background(0x0000ffff)]),
                            Style::State(2, vec![background(0x00ff00ff)]),
                            Style::State(3, vec![background(0xff0000ff)]),
                            Style::State(6, vec![background(0xffff00ff), Field::Opacity(1.)]),
                        ],
                    ),
                    policy(false),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        if kind == Kind::Link {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::Create(id(2), Kind::Text, "Read guide".into(), None),
                        Op::Splice(id(1), 0, 0, vec![id(2)]),
                    ],
                )
            });
        }
        window.draw(cx).clear(cx);
    });
    let focus = owner.read_with(cx, |view, _| view.buttons[&id(1)].focus.clone());
    let point = gpui::point(px(40.), px(40.));
    let paint = |cx: &mut VisualTestContext, color: u32| {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let expected: gpui::Background = rgba(color).into();
            assert!(
                window
                    .painted_quads()
                    .iter()
                    .any(|quad| quad.background == expected),
                "button did not paint expected color {color:08x}"
            );
        });
    };
    paint(cx, 0x444444ff);
    cx.update(|window, cx| window.focus(&focus, cx));
    paint(cx, 0x0000ffff);
    cx.simulate_event(MouseMoveEvent {
        position: point,
        pressed_button: None,
        modifiers: Default::default(),
    });
    paint(cx, 0x00ff00ff);
    cx.simulate_event(MouseDownEvent {
        position: point,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    paint(cx, 0xff0000ff);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| apply(view, window, cx, vec![policy(true)]));
        assert!(focus.is_focused(window));
    });
    paint(cx, 0x0000ffff);
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_event(MouseUpEvent {
        position: point,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    paint(cx, 0x0000ffff);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|event| matches!(event, Event::Press(..)))
    );
    if kind == Kind::Link {
        activate(cx, "enter");
        activate(cx, "space");
        assert!(
            !transport
                .mailbox
                .lock()
                .unwrap()
                .drain(256)
                .iter()
                .any(|event| matches!(event, Event::Press(..)))
        );
        cx.update(|window, cx| {
            window.blur(cx);
            let gate = owner.read_with(cx, |view, _| view.focus.clone());
            gate.borrow().traverse(false, window, cx);
            assert!(focus.is_focused(window), "busy link remains a Tab stop");
        });
    }
    cx.update(|window, cx| window.blur(cx));
    paint(cx, 0x444444ff);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![if kind == Kind::Link {
                    Op::SetLink(id(1), link_config(true, true))
                } else {
                    Op::SetControl(id(1), Control::Button(true))
                }],
            )
        });
    });
    paint(cx, 0xffff00ff);
    cx.simulate_click(point, Default::default());
    paint(cx, 0xffff00ff);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|event| matches!(event, Event::Press(..)))
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                if kind == Kind::Link {
                    vec![policy(false)]
                } else {
                    vec![Op::SetControl(id(1), Control::Button(false)), policy(false)]
                },
            )
        });
        assert_eq!(owner.read(cx).buttons[&id(1)].focus, focus);
    });
    paint(cx, 0x00ff00ff);
    cx.simulate_click(point, Default::default());
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .filter(|event| matches!(event, Event::Press(_, node, _, _) if *node == id(1)))
            .count(),
        1
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.session.borrow_mut().close(view.id).unwrap();
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert!(owner.read_with(cx, |view, _| view.buttons.is_empty()));
    assert_eq!(session.borrow().retained_bytes(), 0);
}
#[test]
fn rich_button_keeps_owner_and_separates_loading_skip_and_preserve_focus() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Buttons", 400., 200.)
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
                    Op::Create(id(0), Kind::Container, "".into(), Some(handler)),
                    Op::Create(id(1), Kind::Button, "Sibling".into(), Some(handler)),
                    Op::Create(id(2), Kind::Button, "Submit".into(), Some(handler)),
                    config(
                        false,
                        Focus::Focusable(TabOrder {
                            tab_stop: true,
                            index: -1,
                        }),
                    ),
                    Op::Create(id(3), Kind::Text, "Rich label".into(), None),
                    Op::Splice(id(2), 0, 0, vec![id(3)]),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        window.draw(cx).clear(cx);
        let gate = owner.read_with(cx, |view, _| view.focus.clone());
        gate.borrow().traverse(false, window, cx);
    });
    let focus = owner.read_with(cx, |view, _| view.buttons[&id(2)].focus.clone());
    cx.update(|window, _| assert!(focus.is_focused(window)));
    transport.mailbox.lock().unwrap().drain(256);
    activate(cx, "enter");
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Press(_, node, _, _) if *node == id(2)))
            .count(),
        1
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Press(_, node, _, _) if *node == id(0)))
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![config(true, Focus::default())])
        });
        window.draw(cx).clear(cx);
        assert!(focus.is_focused(window), "loading preserves focus");
    });
    let point = owner.read_with(cx, |view, _| view.probes.borrow()[&id(3)].bounds.center());
    cx.simulate_click(point, Default::default());
    activate(cx, "space");
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|e| matches!(e, Event::Press(..))),
        "busy content must not activate its ancestor"
    );
    assert!(
        session
            .borrow()
            .press(window_id, id(2), handler, 1)
            .is_none()
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![config(false, Focus::Preserve)])
        });
        window.draw(cx).clear(cx);
        assert!(!focus.is_focused(window));
        let gate = owner.read_with(cx, |view, _| view.focus.clone());
        gate.borrow().traverse(false, window, cx);
    });
    assert!(
        session
            .borrow()
            .press(window_id, id(2), handler, 1)
            .is_none(),
        "old callback stays fenced after loading ends"
    );
    cx.simulate_click(point, Default::default());
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(
                view.buttons[&id(1)].focus.is_focused(window),
                "auxiliary pointer action preserves sibling focus"
            );
            assert_eq!(view.buttons[&id(2)].focus, focus);
        })
    });
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .filter(|e| matches!(e, Event::Press(_, node, _, _) if *node == id(2)))
            .count(),
        1
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![config(
                    false,
                    Focus::Focusable(TabOrder {
                        tab_stop: false,
                        index: -2,
                    }),
                )],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.simulate_click(point, Default::default());
    cx.update(|window, _| {
        assert!(
            focus.is_focused(window),
            "skipping Tab still allows pointer focus"
        )
    });
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetButtonPresentation(id(2), None),
                    Op::Splice(id(2), 0, 1, vec![]),
                    Op::Remove(id(3)),
                ],
            )
        });
        window.draw(cx).clear(cx);
        assert!(
            focus.is_focused(window),
            "legacy reset keeps focus identity"
        );
    });
    let weak = owner.read_with(cx, |view, _| Rc::downgrade(&view.buttons[&id(2)]));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.session.borrow_mut().close(view.id).unwrap();
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert!(weak.upgrade().is_none());
    assert_eq!(session.borrow().retained_bytes(), 0);
}

#[test]
fn rich_button_clocks_keep_painting_while_busy_pause_when_hidden_and_retire_on_reset_or_close() {
    use gpuio_protocol::{
        animation::Easing,
        asset::Format,
        progress_presentation::{Shape, Transition},
    };
    use std::time::Duration;
    for close in [false, true] {
        let mut app = TestAppContext::single();
        app.update(crate::image_host::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        let window_id = WindowId::from_parts(0, 1).unwrap();
        let handler = HandlerId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Rich resources", 400., 200.)
            .unwrap();
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M0 0 L16 8 L0 16 Z"/></svg>"#;
        let svg = {
            let mut session = session.borrow_mut();
            let assets = session.assets().unwrap();
            let id = assets.begin(Format::Svg, bytes.len()).unwrap();
            assets.append(id, 0, bytes).unwrap();
            assets.finish(id).unwrap();
            id
        };
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(window_id, session.clone(), transport));
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                view.spinner_clock.set_test_time(Duration::ZERO);
                view.progress_clock.set_test_time(Duration::ZERO);
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::Create(id(0), Kind::Container, "".into(), None),
                        Op::Create(id(1), Kind::Text, "Upload status".into(), None),
                        Op::Create(id(2), Kind::Button, "Upload".into(), Some(handler)),
                        config(true, Focus::default()),
                        Op::Create(id(3), Kind::Container, "".into(), None),
                        Op::Create(id(4), Kind::Loading, "".into(), None),
                        Op::SetSpinner(
                            id(4),
                            gpuio_protocol::spinner::Config {
                                label: "Spinner".into(),
                                animated: true,
                                period_ms: 1000,
                                easing: Easing::Linear,
                                source: Some(ImageSource::Reference(svg)),
                            },
                        ),
                        Op::SetStyle(
                            id(4),
                            vec![Style::Fields(vec![
                                Field::Width(Length::Px(24.)),
                                Field::Height(Length::Px(24.)),
                            ])],
                        ),
                        Op::Create(id(5), Kind::Progress, "".into(), None),
                        Op::SetProgressPresentation(
                            id(5),
                            gpuio_protocol::progress_presentation::Config {
                                progress: ProgressConfig {
                                    label: "Progress".into(),
                                    fraction: None,
                                },
                                shape: Shape::Circle,
                                transition: Transition::Immediate,
                            },
                        ),
                        Op::SetStyle(
                            id(5),
                            vec![Style::Fields(vec![
                                Field::Width(Length::Px(32.)),
                                Field::Height(Length::Px(32.)),
                            ])],
                        ),
                        Op::Splice(id(3), 0, 0, vec![id(4), id(5)]),
                        Op::Splice(id(2), 0, 0, vec![id(3)]),
                        Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                        Op::SetRoot(Some(id(0))),
                    ],
                );
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        let (spinner, progress, focus) = owner.read_with(cx, |view, _| {
            assert_eq!(view.spinners.len(), 1);
            assert_eq!(view.progresses.len(), 1);
            assert_eq!(view.images.len(), 1);
            (
                view.spinners[&id(4)].driver(),
                view.progresses[&id(5)].driver(),
                view.buttons[&id(2)].focus.clone(),
            )
        });
        let first_spinner = spinner.sample(false).unwrap();
        let first_progress = progress.sample(false).unwrap();
        cx.update(|window, cx| {
            owner.update(cx, |view, _| {
                view.spinner_clock.set_test_time(Duration::from_millis(200));
                view.progress_clock
                    .set_test_time(Duration::from_millis(200));
            });
            assert!(window.simulate_next_frame(cx) > 0);
            window.draw(cx).clear(cx);
        });
        assert_ne!(
            spinner.sample(false).unwrap(),
            first_spinner,
            "busy is not animation-inert"
        );
        assert_ne!(progress.sample(false).unwrap(), first_progress);
        for fields in [
            vec![Field::Visibility(1)],
            vec![Field::Inert(true)],
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
                })
            });
            for _ in 0..3 {
                cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    window.simulate_next_frame(cx);
                });
                cx.run_until_parked();
            }
            cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
            assert!(spinner.sample(false).is_some() && progress.sample(false).is_some());
        }
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetStyle(id(0), vec![]), config(false, Focus::default())],
                )
            });
            window.draw(cx).clear(cx);
            assert!(window.simulate_next_frame(cx) > 0);
            owner.read_with(cx, |view, _| assert_eq!(view.buttons[&id(2)].focus, focus));
        });
        session.borrow_mut().assets().unwrap().release(svg).unwrap();
        assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 1);
        if close {
            cx.update(|window, _| window.remove_window());
        } else {
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![
                            Op::SetButtonPresentation(id(2), None),
                            Op::Splice(id(2), 0, 1, vec![]),
                            Op::Splice(id(3), 0, 2, vec![]),
                            Op::Remove(id(4)),
                            Op::Remove(id(5)),
                            Op::Remove(id(3)),
                        ],
                    )
                });
                window.draw(cx).clear(cx);
                window.simulate_next_frame(cx);
                window.draw(cx).clear(cx);
            });
        }
        cx.run_until_parked();
        assert!(spinner.sample(false).is_none() && progress.sample(false).is_none());
        owner.read_with(cx, |view, _| {
            assert!(
                view.spinners.is_empty() && view.progresses.is_empty() && view.images.is_empty()
            )
        });
        assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
        if !close {
            cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
            cx.update(|window, _| window.remove_window());
            cx.run_until_parked();
        }
        session.borrow_mut().close(window_id).unwrap();
        assert_eq!(session.borrow().retained_bytes(), 0);
    }
}

#[path = "hover_observation_test.rs"]
mod hover_tests;
