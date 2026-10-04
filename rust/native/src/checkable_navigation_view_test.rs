//! Production focus/paint/activation on TestPlatform; no OS input/AX claim.
use super::*;
use gpui::{KeyDownEvent, KeyUpEvent, Keystroke, PlatformInput, TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, checkable::TabOrder};
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
#[test]
fn standalone_radio_and_tab_order_keep_focus_identity_and_fence_activation() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Tab order", 400., 200.)
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
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::Checkbox, "Check".into(), Some(handler)),
                    Op::SetControl(id(1), Control::Checkbox(CheckState::Unchecked, false)),
                    Op::SetTabOrder(
                        id(1),
                        Some(TabOrder {
                            tab_stop: true,
                            index: 20,
                        }),
                    ),
                    Op::Create(id(2), Kind::Radio, "Radio".into(), Some(handler)),
                    Op::SetControl(id(2), Control::Radio(false, None, false)),
                    Op::SetTabOrder(
                        id(2),
                        Some(TabOrder {
                            tab_stop: true,
                            index: -1,
                        }),
                    ),
                    Op::Create(id(3), Kind::Switch, "Switch".into(), Some(handler)),
                    Op::SetControl(id(3), Control::Switch(false, false)),
                    Op::SetTabOrder(
                        id(3),
                        Some(TabOrder {
                            tab_stop: true,
                            index: 20,
                        }),
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(2), id(3)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    let focus = owner.read_with(cx, |view, _| view.buttons[&id(2)].focus.clone());
    for expected in [2, 1, 3, 2] {
        cx.update(|window, cx| {
            let gate = owner.read_with(cx, |view, _| view.focus.clone());
            gate.borrow().traverse(false, window, cx);
            owner.read_with(cx, |view, _| {
                assert!(view.buttons[&id(expected)].focus.is_focused(window))
            });
            window.draw(cx).clear(cx);
        });
    }
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(view.buttons[&id(1)].focus.is_focused(window))
        })
    });
    cx.simulate_keystrokes("shift-tab");
    cx.update(|window, _| assert!(focus.is_focused(window)));
    let point = owner.read_with(cx, |view, _| view.probes.borrow()[&id(2)].bounds.center());
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_click(point, Default::default());
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .into_iter()
            .filter(|e| matches!(e, Event::Press(..)))
            .count(),
        1
    );
    activate(cx, "space");
    activate(cx, "enter");
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .into_iter()
            .filter(|event| matches!(event, Event::Press(_, node, _, _) if *node == id(2)))
            .count(),
        2,
        "unchecked radio accepts keyboard selection intents"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetControl(id(2), Control::Radio(true, None, false)),
                    Op::Bind(id(2), None),
                    Op::SetTabOrder(
                        id(2),
                        Some(TabOrder {
                            tab_stop: false,
                            index: -1,
                        }),
                    ),
                ],
            )
        });
        window.draw(cx).clear(cx);
        assert!(
            focus.is_focused(window),
            "changing Tab eligibility must not blur pointer/programmatic focus"
        );
        let gate = owner.read_with(cx, |view, _| view.focus.clone());
        gate.borrow().traverse(false, window, cx);
        assert!(!focus.is_focused(window));
        window.focus(&focus, cx);
        assert!(focus.is_focused(window));
    });
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_click(point, Default::default());
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|e| matches!(e, Event::Press(..)))
    );
    activate(cx, "space");
    activate(cx, "enter");
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|event| matches!(event, Event::Press(..))),
        "checked radio suppresses keyboard selection intents"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetTabOrder(id(2), None),
                    Op::SetControl(id(2), Control::Radio(false, None, false)),
                    Op::Bind(id(2), Some(handler)),
                ],
            )
        });
        window.draw(cx).clear(cx);
        owner.read_with(cx, |view, _| assert_eq!(view.buttons[&id(2)].focus, focus));
        let gate = owner.read_with(cx, |view, _| view.focus.clone());
        gate.borrow().traverse(true, window, cx);
        owner.read_with(cx, |view, _| {
            assert!(view.buttons[&id(3)].focus.is_focused(window))
        });
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Splice(id(0), 0, 3, vec![]),
                    Op::Remove(id(1)),
                    Op::Remove(id(2)),
                    Op::Remove(id(3)),
                    Op::Remove(id(0)),
                ],
            )
        });
        window.draw(cx).clear(cx);
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
        assert_eq!(window.simulate_next_frame(cx), 0);
    });
    owner.read_with(cx, |view, _| assert!(view.buttons.is_empty()));
    assert_eq!(session.borrow().retained_bytes(), 0);
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_next_frame(cx);
        });
        cx.run_until_parked();
    }
}
fn presses(transport: &Transport) -> Vec<NodeId> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| {
            if let Event::Press(_, node, _, _) = event {
                Some(node)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn modal_radio_order_preserves_nonstop_anchor_and_restores_outside_focus() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Modal radios", 400., 240.)
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
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::Radio, "Outside".into(), Some(handler)),
                    Op::SetControl(id(1), Control::Radio(false, None, false)),
                    Op::SetTabOrder(
                        id(1),
                        Some(TabOrder {
                            tab_stop: true,
                            index: -100,
                        }),
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
    });
    settle(cx);
    let outside = owner.read_with(cx, |view, _| view.buttons[&id(1)].focus.clone());
    let outside_owner = owner.read_with(cx, |view, _| Rc::downgrade(&view.buttons[&id(1)]));
    cx.update(|window, cx| window.focus(&outside, cx));
    settle(cx);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(2), Kind::FocusScope, "".into(), None),
                    Op::SetFocusScope(
                        id(2),
                        FocusScopeConfig {
                            trap: true,
                            auto_focus: true,
                            restore_focus: true,
                        },
                    ),
                    Op::Create(id(3), Kind::Radio, "Pointer choice".into(), Some(handler)),
                    Op::SetControl(id(3), Control::Radio(false, None, false)),
                    Op::SetTabOrder(
                        id(3),
                        Some(TabOrder {
                            tab_stop: false,
                            index: -20,
                        }),
                    ),
                    Op::Create(id(4), Kind::Radio, "Keyboard choice".into(), Some(handler)),
                    Op::SetControl(id(4), Control::Radio(false, None, false)),
                    Op::SetTabOrder(
                        id(4),
                        Some(TabOrder {
                            tab_stop: true,
                            index: 10,
                        }),
                    ),
                    Op::Splice(id(2), 0, 0, vec![id(3), id(4)]),
                    Op::Splice(id(0), 1, 0, vec![id(2)]),
                ],
            )
        });
    });
    settle(cx);
    let skipped = owner.read_with(cx, |view, _| view.buttons[&id(3)].focus.clone());
    for key in ["tab", "shift-tab", "tab"] {
        cx.simulate_keystrokes(key);
        cx.update(|window, cx| {
            owner.read_with(cx, |view, _| {
                assert!(view.buttons[&id(4)].focus.is_focused(window));
                assert!(!view.focus.borrow().can_focus(&outside, window));
                assert!(view.focus.borrow().can_focus(&skipped, window));
            })
        });
    }
    presses(&transport);
    let point = owner.read_with(cx, |view, _| view.probes.borrow()[&id(3)].bounds.center());
    cx.simulate_click(point, Default::default());
    assert_eq!(presses(&transport), vec![id(3)]);
    cx.update(|window, _| assert!(skipped.is_focused(window)));
    cx.simulate_keystrokes("shift-tab");
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(view.buttons[&id(4)].focus.is_focused(window))
        })
    });
    let outside_point = owner.read_with(cx, |view, _| view.probes.borrow()[&id(1)].bounds.center());
    cx.simulate_click(outside_point, Default::default());
    assert!(
        presses(&transport).is_empty(),
        "modal gate rejects outside activation despite earlier Tab index"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Splice(id(0), 1, 1, vec![]),
                    Op::Splice(id(2), 0, 2, vec![]),
                    Op::Remove(id(3)),
                    Op::Remove(id(4)),
                    Op::Remove(id(2)),
                ],
            )
        });
    });
    settle(cx);
    cx.update(|window, cx| {
        assert!(outside.is_focused(window));
        owner.read_with(cx, |view, _| {
            assert_eq!(view.buttons.len(), 1);
            assert_eq!(view.buttons[&id(1)].focus, outside);
        });
    });
    activate(cx, "enter");
    assert_eq!(presses(&transport), vec![id(1)]);
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    session.borrow_mut().close(window_id).unwrap();
    assert_eq!(session.borrow().retained_bytes(), 0);
    assert!(
        session
            .borrow()
            .press(window_id, id(1), handler, 1)
            .is_none()
    );
    owner.read_with(cx, |view, _| assert!(!view.focus.borrow().allows(id(1))));
    drop(owner);
    // Entity destruction is flushed by an App update, not the background executor.
    cx.cx.update(|_| ());
    cx.run_until_parked();
    assert!(
        outside_owner.upgrade().is_none(),
        "closed root must not retain button state"
    );
}

#[test]
fn virtual_radio_rows_preserve_mounted_focus_and_retire_evicted_generations() {
    use gpuio_protocol::list::{
        Config, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget,
    };
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Virtual radios", 400., 240.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    let rows = |generation, first| {
        (0..3)
            .map(|offset| Row {
                id: first + offset,
                node: NodeId::from_parts(1 + offset, generation).unwrap(),
            })
            .collect::<Vec<_>>()
    };
    let row_ops = |generation, first| {
        let mut ops = vec![];
        for (index, row) in rows(generation, first).iter().enumerate() {
            ops.extend([
                Op::Create(
                    row.node,
                    Kind::Radio,
                    format!("Choice {}", row.id),
                    Some(handler),
                ),
                Op::SetControl(
                    row.node,
                    Control::Radio(
                        false,
                        Some(gpuio_protocol::checkable::Position {
                            index: row.id - 1,
                            count: 100_000,
                        }),
                        false,
                    ),
                ),
                Op::SetTabOrder(
                    row.node,
                    Some(TabOrder {
                        tab_stop: true,
                        index: -(index as i64),
                    }),
                ),
                Op::SetStyle(
                    row.node,
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(200.)),
                        Field::Height(Length::Px(40.)),
                        Field::Shrink(0.),
                    ])],
                ),
            ]);
        }
        ops.extend([
            Op::SetListRows(id(0), rows(generation, first)),
            Op::Splice(
                id(0),
                0,
                0,
                rows(generation, first).iter().map(|row| row.node).collect(),
            ),
        ]);
        ops
    };
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let mut ops = vec![
                Op::Create(id(0), Kind::VirtualList, "".into(), None),
                Op::SetStyle(
                    id(0),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(240.)),
                        Field::Height(Length::Px(80.)),
                        Field::Shrink(0.),
                    ])],
                ),
                Op::SetListConfig(
                    id(0),
                    Config {
                        estimated_height: 40.,
                        overscan: 0.,
                        max_active: 4,
                        scroll_policy: ScrollPolicy::KeepPosition,
                        scrollbar: false,
                        managed: true,
                    },
                ),
                Op::SetListOrder(
                    id(0),
                    Order {
                        revision: 1,
                        runs: vec![IdRun {
                            first: 1,
                            count: 100_000,
                        }],
                    },
                ),
                Op::SetRoot(Some(id(0))),
            ];
            ops.extend(row_ops(1, 1));
            apply(view, window, cx, ops);
        })
    });
    settle(cx);
    let first = owner.read_with(cx, |view, _| Rc::downgrade(&view.buttons[&id(1)]));
    let focus = first.upgrade().unwrap().focus.clone();
    cx.update(|window, cx| {
        window.focus(&focus, cx);
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetText(id(1), "Updated row".into()),
                    Op::SetTabOrder(
                        id(1),
                        Some(TabOrder {
                            tab_stop: false,
                            index: -10,
                        }),
                    ),
                ],
            )
        });
    });
    settle(cx);
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(focus.is_focused(window));
            assert_eq!(view.buttons[&id(1)].focus, focus);
            assert_eq!(
                view.lists[&id(0)]
                    .borrow()
                    .observed
                    .as_ref()
                    .unwrap()
                    .visible_first,
                0
            );
        })
    });
    cx.simulate_keystrokes("tab");
    cx.update(|window, _| assert!(!focus.is_focused(window)));
    cx.update(|window, cx| {
        window.blur(cx);
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::ScrollList(
                    id(0),
                    ScrollRequest {
                        serial: 1,
                        target: ScrollTarget::Offset(50_001, 0.),
                    },
                )],
            )
        });
    });
    settle(cx);
    owner.read_with(cx, |view, _| {
        assert_eq!(
            view.lists[&id(0)]
                .borrow()
                .observed
                .as_ref()
                .unwrap()
                .visible_first,
            50_000
        )
    });
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(!view.focus.borrow().can_focus(&focus, window))
        })
    });
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let mut ops = vec![
                Op::SetListRows(id(0), vec![]),
                Op::Splice(id(0), 0, 3, vec![]),
            ];
            ops.extend((1..4).map(|slot| Op::Remove(id(slot))));
            ops.extend(row_ops(2, 50_001));
            apply(view, window, cx, ops);
        })
    });
    settle(cx);
    assert!(
        first.upgrade().is_none(),
        "evicted button owner must retire despite a held old FocusHandle"
    );
    assert!(
        session
            .borrow()
            .press(window_id, id(1), handler, 1)
            .is_none()
    );
    let next = NodeId::from_parts(1, 2).unwrap();
    let next_focus = owner.read_with(cx, |view, _| {
        assert!(view.buttons.len() <= 3);
        view.buttons[&next].focus.clone()
    });
    let next_owner = owner.read_with(cx, |view, _| Rc::downgrade(&view.buttons[&next]));
    assert_ne!(focus, next_focus);
    cx.update(|window, cx| window.focus(&next_focus, cx));
    settle(cx);
    presses(&transport);
    activate(cx, "space");
    assert_eq!(presses(&transport), vec![next]);
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    session.borrow_mut().close(window_id).unwrap();
    assert_eq!(session.borrow().retained_bytes(), 0);
    assert!(
        session
            .borrow()
            .press(window_id, next, handler, 1)
            .is_none()
    );
    drop(owner);
    // Entity destruction is flushed by an App update, not the background executor.
    cx.cx.update(|_| ());
    cx.run_until_parked();
    assert!(
        next_owner.upgrade().is_none(),
        "closed virtual root must release its row button"
    );
}
