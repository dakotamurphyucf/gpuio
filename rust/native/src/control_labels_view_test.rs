//! Production layout/input routing on TestPlatform, not OS keyboard/AX acceptance.
use super::*;
use gpui::TestAppContext;
use gpuio_protocol::{
    HandlerId, avatar,
    control_appearance::{Config, LabelPosition},
};
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
fn value(kind: Kind, disabled: bool) -> Op {
    match kind {
        Kind::Checkbox => Op::SetControl(id(1), Control::Checkbox(CheckState::Checked, disabled)),
        Kind::Switch => Op::SetControl(id(1), Control::Switch(true, disabled)),
        // Per-option disable must reach decorative descendants independently of
        // the group, which remains focusable for its other options.
        Kind::RadioGroup | Kind::TabBar => Op::SetChoice(
            id(1),
            ChoiceConfig {
                label: "Choice".into(),
                items: vec![ChoiceItem {
                    id: "one".into(),
                    label: "Option".into(),
                    disabled,
                }],
                selected: None,
                disabled: false,
            },
        ),
        _ => unreachable!(),
    }
}
fn block(color: u32, disabled_color: u32, width: f64, height: f64) -> Vec<Style> {
    vec![
        Style::Fields(vec![
            Field::Width(Length::Px(width)),
            Field::Height(Length::Px(height)),
            Field::Shrink(0.),
            Field::Background(Fill::Solid(Color::Rgba(i64::from(color)))),
        ]),
        Style::State(
            6,
            vec![Field::Background(Fill::Solid(Color::Rgba(i64::from(
                disabled_color,
            ))))],
        ),
    ]
}

#[test]
fn rich_labels_paint_once_route_to_owner_and_project_disabled_into_avatar_fallbacks() {
    for kind in [Kind::Checkbox, Kind::Switch, Kind::RadioGroup, Kind::TabBar] {
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
            .open(1, window_id, "Labels", 400., 200.)
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
                        Op::SetStyle(id(0), vec![Style::Fields(vec![Field::UserSelect(true)])]),
                        Op::Create(id(1), kind, "Semantic owner".into(), Some(handler)),
                        value(kind, false),
                        Op::SetStyle(
                            id(1),
                            vec![Style::Fields(vec![Field::Display(1), Field::Direction(0)])],
                        ),
                        Op::Create(id(2), Kind::Container, "".into(), None),
                        Op::Create(id(3), Kind::Container, "".into(), None),
                        Op::SetStyle(id(3), block(0x00ff00ff, 0x0000ffff, 64., 48.)),
                        Op::Create(id(4), Kind::Avatar, "".into(), None),
                        Op::SetAvatar(
                            id(4),
                            avatar::Config {
                                source: None,
                                fit: ImageFit::Cover,
                                label: Some("Decorative avatar".into()),
                                fallback: "?".into(),
                            },
                        ),
                        Op::SetStyle(
                            id(4),
                            vec![Style::Fields(vec![
                                Field::Shrink(0.),
                                Field::Width(Length::Px(12.)),
                                Field::Height(Length::Px(12.)),
                            ])],
                        ),
                        Op::Create(id(5), Kind::Container, "".into(), None),
                        Op::SetStyle(id(5), block(0xff00ffff, 0xffff00ff, 8., 8.)),
                        Op::Create(id(6), Kind::Text, "Rich caption".into(), None),
                        Op::Splice(id(3), 0, 0, vec![id(4), id(6)]),
                        Op::Splice(id(4), 0, 0, vec![id(5)]),
                        Op::Splice(id(2), 0, 0, vec![id(3)]),
                        Op::Splice(id(1), 0, 0, vec![id(2)]),
                        Op::Splice(id(0), 0, 0, vec![id(1)]),
                        Op::SetRoot(Some(id(0))),
                    ],
                )
            });
            window.draw(cx).clear(cx);
        });
        let focus = owner.read_with(cx, |view, _| {
            assert!(
                view.selections.is_empty(),
                "label text cannot inherit selectable content"
            );
            view.buttons[&id(1)].focus.clone()
        });
        let mut label_positions = Vec::new();
        let mut last_click = None;
        for position in [LabelPosition::After, LabelPosition::Before] {
            let click = cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        if kind == Kind::TabBar {
                            vec![Op::SetText(id(6), "Rich caption".into())]
                        } else {
                            vec![Op::SetControlAppearance(
                                id(1),
                                Some(Config {
                                    label_position: position,
                                    ..Config::default()
                                }),
                            )]
                        },
                    )
                });
                window.focus(&focus, cx);
                window.draw(cx).clear(cx);
                assert!(focus.is_focused(window));
                let green: gpui::Background = rgba(0x00ff00ff).into();
                let magenta: gpui::Background = rgba(0xff00ffff).into();
                let quads = window.painted_quads();
                let labels: Vec<_> = quads
                    .iter()
                    .filter(|quad| quad.background == green)
                    .collect();
                assert_eq!(
                    labels.len(),
                    1,
                    "label must not also paint via generic children: {kind:?}"
                );
                assert_eq!(
                    quads
                        .iter()
                        .filter(|quad| quad.background == magenta)
                        .count(),
                    1
                );
                let bounds = labels[0].bounds;
                let scale = window.scale_factor();
                label_positions.push(bounds.origin.x.0);
                gpui::point(
                    px((bounds.origin.x.0 + 40. * scale) / scale),
                    px((bounds.origin.y.0 + 8. * scale) / scale),
                )
            });
            transport.mailbox.lock().unwrap().drain(256);
            last_click = Some(click);
            cx.simulate_click(click, Default::default());
            let events = transport.mailbox.lock().unwrap().drain(256);
            let activations: Vec<_> = events
                .into_iter()
                .filter(|event| matches!(event, Event::Press(..) | Event::Choice(..)))
                .collect();
            let revision = session.borrow().tree(window_id).unwrap().revision();
            let expected = if matches!(kind, Kind::RadioGroup | Kind::TabBar) {
                Event::Choice(window_id, id(1), handler, revision, "one".into())
            } else {
                Event::Press(window_id, id(1), handler, revision)
            };
            assert_eq!(
                activations,
                vec![expected],
                "one owner activation: {kind:?}"
            );
        }
        if kind == Kind::TabBar {
            assert_eq!(label_positions[0], label_positions[1]);
        } else {
            assert!(label_positions[0] > label_positions[1]);
        }
        for source in [None, Some(ImageSource::Unavailable(ImageError::Released))] {
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![
                            value(kind, true),
                            Op::SetAvatar(
                                id(4),
                                avatar::Config {
                                    source,
                                    fit: ImageFit::Cover,
                                    label: Some("Decorative avatar".into()),
                                    fallback: "?".into(),
                                },
                            ),
                        ],
                    )
                });
                window.draw(cx).clear(cx);
                // Radio applies 0.5 opacity to disabled options. Check hue, including
                // the delayed avatar fallback renderer, without assuming full alpha.
                let quads = window.painted_quads();
                for color in [0x0000ffff, 0xffff00ff] {
                    let expected: gpui::Background = rgba(color).into();
                    assert!(
                        quads.iter().any(|quad| quad
                            .background
                            .as_solid()
                            .map(|color| color.alpha(1.))
                            == expected.as_solid()),
                        "disabled label/fallback color {color:x}: {kind:?}"
                    );
                }
            });
        }
        transport.mailbox.lock().unwrap().drain(256);
        cx.simulate_click(last_click.unwrap(), Default::default());
        assert!(
            !transport
                .mailbox
                .lock()
                .unwrap()
                .drain(256)
                .iter()
                .any(|event| matches!(event, Event::Press(..) | Event::Choice(..)))
        );
        owner.read_with(cx, |view, _| assert_eq!(view.buttons[&id(1)].focus, focus));
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::Splice(id(1), 0, 1, vec![]),
                        Op::Splice(id(2), 0, 1, vec![]),
                        Op::Splice(id(3), 0, 2, vec![]),
                        Op::Splice(id(4), 0, 1, vec![]),
                        Op::Remove(id(6)),
                        Op::Remove(id(5)),
                        Op::Remove(id(4)),
                        Op::Remove(id(3)),
                        Op::Remove(id(2)),
                    ],
                )
            });
            window.draw(cx).clear(cx);
        });
        owner.read_with(cx, |view, _| {
            assert!(view.avatar_fallbacks.is_empty());
            assert!(view.selections.is_empty());
            assert_eq!(view.buttons.len(), 1);
            assert_eq!(view.buttons[&id(1)].focus, focus);
        });
        // Hidden label semantics must not suppress its native animation clock.
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        value(kind, false),
                        Op::Create(id(7), Kind::Container, "".into(), None),
                        Op::Create(id(8), Kind::Loading, "".into(), None),
                        Op::SetSpinner(
                            id(8),
                            gpuio_protocol::spinner::Config {
                                label: "Decorative activity".into(),
                                animated: true,
                                period_ms: 1000,
                                easing: gpuio_protocol::animation::Easing::Linear,
                                source: None,
                            },
                        ),
                        Op::Splice(id(7), 0, 0, vec![id(8)]),
                        Op::Splice(id(1), 0, 0, vec![id(7)]),
                    ],
                )
            });
            window.draw(cx).clear(cx);
            assert!(window.simulate_next_frame(cx) > 0);
        });
        owner.read_with(cx, |view, _| assert_eq!(view.spinners.len(), 1));
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::SetRoot(None),
                        Op::Splice(id(7), 0, 1, vec![]),
                        Op::Splice(id(1), 0, 1, vec![]),
                        Op::Splice(id(0), 0, 1, vec![]),
                        Op::Remove(id(8)),
                        Op::Remove(id(7)),
                        Op::Remove(id(1)),
                        Op::Remove(id(0)),
                    ],
                )
            });
            window.draw(cx).clear(cx);
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            assert_eq!(window.simulate_next_frame(cx), 0);
        });
        owner.read_with(cx, |view, _| {
            assert!(view.spinners.is_empty() && view.buttons.is_empty() && view.radios.is_empty());
        });
        assert_eq!(session.borrow().retained_bytes(), 0);
    }
}

#[test]
fn radio_reorder_moves_retained_labels_and_their_disabled_state_with_option_identity() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Radio labels", 400., 200.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(window_id, session.clone(), transport));
    let config = |reversed| {
        let mut items = vec![
            ChoiceItem {
                id: "a".into(),
                label: "Alpha".into(),
                disabled: false,
            },
            ChoiceItem {
                id: "b".into(),
                label: "Beta".into(),
                disabled: true,
            },
        ];
        if reversed {
            items.reverse();
        }
        ChoiceConfig {
            label: "Mode".into(),
            items,
            selected: Some("a".into()),
            disabled: false,
        }
    };
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::RadioGroup, "".into(), Some(handler)),
                    Op::SetChoice(id(1), config(false)),
                    Op::SetStyle(id(1), vec![Style::Fields(vec![Field::Direction(0)])]),
                    Op::Create(id(2), Kind::Container, "".into(), None),
                    Op::Create(id(3), Kind::Container, "".into(), None),
                    Op::SetStyle(id(3), block(0x00ff00ff, 0x0000ffff, 32., 20.)),
                    Op::Create(id(4), Kind::Container, "".into(), None),
                    Op::Create(id(5), Kind::Container, "".into(), None),
                    Op::SetStyle(id(5), block(0xff0000ff, 0x0000ffff, 32., 20.)),
                    Op::Splice(id(2), 0, 0, vec![id(3)]),
                    Op::Splice(id(4), 0, 0, vec![id(5)]),
                    Op::Splice(id(1), 0, 0, vec![id(2), id(4)]),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    let (focus, radio) = owner.read_with(cx, |view, _| {
        (
            view.buttons[&id(1)].focus.clone(),
            view.radios[&id(1)].clone(),
        )
    });
    for reversed in [false, true, false] {
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::SetChoice(id(1), config(reversed)),
                        Op::Splice(
                            id(1),
                            0,
                            2,
                            if reversed {
                                vec![id(4), id(2)]
                            } else {
                                vec![id(2), id(4)]
                            },
                        ),
                    ],
                )
            });
            window.focus(&focus, cx);
            window.draw(cx).clear(cx);
            let quads = window.painted_quads();
            let position = |color| {
                let expected = gpui::Hsla::from(rgba(color));
                let matching: Vec<_> = quads
                    .iter()
                    .filter(|quad| {
                        quad.background.as_solid().map(|color| color.alpha(1.)) == Some(expected)
                    })
                    .collect();
                assert_eq!(matching.len(), 1);
                matching[0].bounds.origin.x.0
            };
            assert_eq!(position(0x00ff00ff) > position(0x0000ffff), reversed);
            assert!(focus.is_focused(window));
        });
        owner.read_with(cx, |view, _| {
            assert_eq!(view.buttons[&id(1)].focus, focus);
            assert!(Rc::ptr_eq(&view.radios[&id(1)], &radio));
        });
    }
}

#[test]
fn radio_navigation_preserves_rapid_requests_back_to_the_committed_option() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Radio requests", 400., 200.)
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
                    Op::Create(id(0), Kind::RadioGroup, "".into(), Some(handler)),
                    Op::SetChoice(
                        id(0),
                        ChoiceConfig {
                            label: "Mode".into(),
                            selected: Some("a".into()),
                            disabled: false,
                            items: [("a", false), ("disabled", true), ("b", false)]
                                .into_iter()
                                .map(|(id, disabled)| ChoiceItem {
                                    id: id.into(),
                                    label: id.into(),
                                    disabled,
                                })
                                .collect(),
                        },
                    ),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        window.draw(cx).clear(cx);
        let focus = owner.read_with(cx, |view, _| view.buttons[&id(0)].focus.clone());
        window.focus(&focus, cx);
    });
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_keystrokes("down down");
    let events: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter(|event| matches!(event, Event::Choice(..)))
        .collect();
    assert_eq!(
        events,
        vec![
            Event::Choice(window_id, id(0), handler, 1, "b".into()),
            Event::Choice(window_id, id(0), handler, 1, "a".into()),
        ]
    );
    assert_eq!(
        session
            .borrow()
            .tree(window_id)
            .unwrap()
            .get(id(0))
            .unwrap()
            .choice
            .as_ref()
            .unwrap()
            .selected
            .as_deref(),
        Some("a")
    );
}

#[test]
fn rich_tabs_keep_names_keyboard_routing_and_semantic_identity_across_label_changes() {
    let mut app = TestAppContext::single();
    app.update(crate::image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Rich tabs", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let mut config = ChoiceConfig {
        label: "Workspace tabs".into(),
        items: ["Notes", "Disabled", "Draft"]
            .into_iter()
            .enumerate()
            .map(|(i, label)| ChoiceItem {
                id: label.into(),
                label: label.into(),
                disabled: i == 1,
            })
            .collect(),
        selected: Some("Notes".into()),
        disabled: false,
    };
    let mut ops = vec![
        Op::Create(id(0), Kind::TabBar, String::new(), Some(handler)),
        Op::SetChoice(id(0), config.clone()),
        Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![
                Field::Direction(0),
                Field::Width(Length::Px(350.)),
                Field::Height(Length::Px(60.)),
            ])],
        ),
        Op::SetRoot(Some(id(0))),
    ];
    for i in 0..3 {
        ops.extend([
            Op::Create(id(1 + 2 * i), Kind::Container, String::new(), None),
            Op::Create(id(2 + 2 * i), Kind::Text, format!("Decorative {i}"), None),
            Op::Splice(id(1 + 2 * i), 0, 0, vec![id(2 + 2 * i)]),
        ]);
    }
    ops.push(Op::Splice(id(0), 0, 0, vec![id(1), id(3), id(5)]));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| apply(view, window, cx, ops));
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let nodes = cx.a11y_tree().unwrap().nodes;
    let tabs: Vec<_> = nodes
        .iter()
        .filter(|(_, node)| node.role() == gpui::accesskit::Role::Tab)
        .map(|(id, node)| (*id, node.label().unwrap().to_owned()))
        .collect();
    assert_eq!(tabs.len(), 3);
    assert!(tabs.iter().any(|(_, label)| label == "Notes"));
    // Decorative captions remain underneath hidden semantic wrappers, never a
    // second tab or accessibility name replacing the configured label.
    for (_, node) in nodes
        .iter()
        .filter(|(_, node)| node.role() == gpui::accesskit::Role::Tab)
    {
        assert!(!node.label().unwrap().starts_with("Decorative"));
    }
    let focus = owner.read_with(cx, |view, _| view.buttons[&id(0)].focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    transport.mailbox.lock().unwrap().drain(100);
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    let selected: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(100)
        .into_iter()
        .filter_map(|event| match event {
            Event::Choice(_, _, _, _, id) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(selected, vec!["Draft"]);
    config.selected = Some("Draft".into());
    config.items.reverse();
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoice(id(0), config.clone()),
                    Op::Splice(id(0), 0, 3, vec![id(5), id(3), id(1)]),
                    Op::SetText(id(2), "Updated decoration".into()),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let reordered = cx.a11y_tree().unwrap().nodes;
    for (tab_id, label) in &tabs {
        let node = reordered
            .iter()
            .find(|(_, node)| {
                node.role() == gpui::accesskit::Role::Tab && node.label() == Some(label.as_str())
            })
            .unwrap();
        assert_eq!(*tab_id, node.0, "tab identity follows choice ID");
        assert_eq!(node.1.is_selected(), Some(label == "Draft"));
    }
    let notes = tabs.iter().find(|(_, label)| label == "Notes").unwrap().0;
    transport.mailbox.lock().unwrap().drain(100);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: notes,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .filter(|event| matches!(event,Event::Choice(_,_,_,_,selected) if selected=="Notes"))
            .count(),
        1
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Splice(id(0), 0, 3, vec![]),
                    Op::Remove(id(2)),
                    Op::Remove(id(4)),
                    Op::Remove(id(6)),
                    Op::Remove(id(1)),
                    Op::Remove(id(3)),
                    Op::Remove(id(5)),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let plain = cx.a11y_tree().unwrap().nodes;
    for (tab_id, label) in tabs {
        assert!(
            plain
                .iter()
                .any(|(id, node)| *id == tab_id && node.label() == Some(label.as_str()))
        );
    }
    owner.read_with(cx, |view, _| assert_eq!(view.buttons[&id(0)].focus, focus));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![Op::SetRoot(None), Op::Remove(id(0))])
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(session.borrow().retained_bytes(), 0);
    owner.read_with(cx, |view, _| {
        assert!(view.buttons.is_empty() && view.radios.is_empty())
    });
}

#[test]
fn tab_variants_style_actual_targets_and_preserve_native_focus_and_selection() {
    for rich in [false, true] {
        use gpuio_protocol::tab_appearance::{Config as Tabs, Variant};
        let mut app = TestAppContext::single();
        app.update(crate::image_host::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        let wid = WindowId::from_parts(0, 1).unwrap();
        let handler = HandlerId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, wid, "Styled tabs", 480., 200.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        let mut choices = ChoiceConfig {
            label: "Tabs".into(),
            items: ["Notes", "Draft"]
                .into_iter()
                .map(|label| ChoiceItem {
                    id: label.into(),
                    label: label.into(),
                    disabled: false,
                })
                .collect(),
            selected: Some("Notes".into()),
            disabled: false,
        };
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                apply(
                    v,
                    window,
                    cx,
                    vec![
                        Op::Create(id(0), Kind::TabBar, String::new(), Some(handler)),
                        Op::SetChoice(id(0), choices.clone()),
                        Op::SetStyle(
                            id(0),
                            vec![Style::Fields(vec![
                                Field::Direction(0),
                                Field::Width(Length::Px(420.)),
                                Field::Height(Length::Px(64.)),
                            ])],
                        ),
                        Op::SetRoot(Some(id(0))),
                    ],
                )
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        if rich {
            cx.update(|window, cx| {
                owner.update(cx, |v, cx| {
                    apply(
                        v,
                        window,
                        cx,
                        vec![
                            Op::Create(id(1), Kind::Container, String::new(), None),
                            Op::Create(id(2), Kind::Text, "Rich Notes".into(), None),
                            Op::Create(id(3), Kind::Container, String::new(), None),
                            Op::Create(id(4), Kind::Text, "Rich Draft".into(), None),
                            Op::Splice(id(1), 0, 0, vec![id(2)]),
                            Op::Splice(id(3), 0, 0, vec![id(4)]),
                            Op::Splice(id(0), 0, 0, vec![id(1), id(3)]),
                        ],
                    )
                });
                window.draw(cx).clear(cx);
            });
            cx.run_until_parked();
        }
        let focus = owner.read_with(cx, |v, _| v.buttons[&id(0)].focus.clone());
        cx.update(|window, cx| window.focus(&focus, cx));
        cx.run_until_parked();
        let identities: Vec<_> = cx
            .a11y_tree()
            .unwrap()
            .nodes
            .into_iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::Tab)
            .map(|(id, n)| (id, n.label().unwrap().to_owned()))
            .collect();
        let paint = |color| Field::Background(Fill::Solid(Color::Rgba(color)));
        let mut presentation = Tabs {
            height: 40.,
            tab_style: vec![
                Style::Fields(vec![Field::Width(Length::Px(110.)), paint(0x102030ff)]),
                Style::State(7, vec![paint(0xff0000ff)]),
            ],
            item_styles: vec![
                (
                    "Notes".into(),
                    vec![
                        Style::Fields(vec![Field::Width(Length::Px(150.))]),
                        Style::State(7, vec![paint(0x00ff00ff)]),
                    ],
                ),
                (
                    "Draft".into(),
                    vec![
                        Style::State(1, vec![paint(0xff00ffff)]),
                        Style::State(2, vec![paint(0xff8800ff)]),
                        Style::State(6, vec![paint(0x0000ffff), Field::Opacity(1.)]),
                    ],
                ),
            ],
            ..Tabs::default()
        };
        for variant in [
            Variant::Tab,
            Variant::Outline,
            Variant::Pill,
            Variant::Segmented,
            Variant::Underline,
        ] {
            presentation.variant = variant;
            transport.mailbox.lock().unwrap().drain(100);
            cx.update(|window, cx| {
                owner.update(cx, |v, cx| {
                    apply(
                        v,
                        window,
                        cx,
                        vec![Op::SetTabAppearance(id(0), Some(presentation.clone()))],
                    )
                });
                window.draw(cx).clear(cx);
                let selected: gpui::Background = rgba(0x00ff00ff).into();
                assert_eq!(
                    window
                        .painted_quads()
                        .iter()
                        .filter(|q| q.background == selected)
                        .count(),
                    1
                );
                assert!(focus.is_focused(window));
            });
            cx.run_until_parked();
            let tree = cx.a11y_tree().unwrap();
            let scale = cx.update(|window, _| window.scale_factor()) as f64;
            for (identity, label) in &identities {
                let (_, node) = tree.nodes.iter().find(|(id, _)| id == identity).unwrap();
                assert_eq!(node.label(), Some(label.as_str()));
                let bounds = node.bounds().unwrap();
                assert!(
                    (bounds.width() / scale - if label == "Notes" { 150. } else { 110. }).abs()
                        < 0.1
                );
                assert!((bounds.height() / scale - 40.).abs() < 0.1);
                assert_eq!(node.is_selected(), Some(label == "Notes"));
            }
            assert!(
                !transport
                    .mailbox
                    .lock()
                    .unwrap()
                    .drain(100)
                    .iter()
                    .any(|e| matches!(e, Event::Choice(..) | Event::Press(..)))
            );
        }
        let tab_center = |cx: &mut gpui::VisualTestContext, label: &str| {
            let tree = cx.a11y_tree().unwrap();
            let bounds = tree
                .nodes
                .iter()
                .find(|(_, node)| {
                    node.role() == gpui::accesskit::Role::Tab && node.label() == Some(label)
                })
                .unwrap()
                .1
                .bounds()
                .unwrap();
            let scale = cx.update(|window, _| window.scale_factor()) as f64;
            gpui::point(
                px(((bounds.x0 + bounds.x1) / 2. / scale) as f32),
                px(((bounds.y0 + bounds.y1) / 2. / scale) as f32),
            )
        };
        let pointer = tab_center(cx, "Draft");
        cx.simulate_mouse_move(pointer, None, Default::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let hover: gpui::Background = rgba(0xff8800ff).into();
            assert!(
                window
                    .painted_quads()
                    .iter()
                    .any(|quad| quad.background == hover)
            );
        });
        cx.simulate_mouse_move(gpui::point(px(450.), px(150.)), None, Default::default());
        cx.run_until_parked();
        cx.simulate_keystrokes("right");
        cx.run_until_parked();
        assert_eq!(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .iter()
                .filter(|e| matches!(e,Event::Choice(_,_,_,_,value) if value=="Draft"))
                .count(),
            1
        );
        // Appearance changes preserve the immediate active item even before OCaml
        // accepts its selection request. The selected item and active item differ.
        presentation.variant = Variant::Pill;
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                apply(
                    v,
                    window,
                    cx,
                    vec![Op::SetTabAppearance(id(0), Some(presentation.clone()))],
                )
            });
            window.draw(cx).clear(cx);
            let active: gpui::Background = rgba(0xff00ffff).into();
            assert!(
                window
                    .painted_quads()
                    .iter()
                    .any(|q| q.background == active)
            );
        });
        choices.items.reverse();
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                apply(v, window, cx, {
                    let mut ops = vec![Op::SetChoice(id(0), choices.clone())];
                    if rich {
                        ops.push(Op::Splice(id(0), 0, 2, vec![id(3), id(1)]));
                    }
                    ops
                })
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        owner.read_with(cx, |v, _| {
            assert_eq!(v.radios[&id(0)].borrow().active.as_deref(), Some("Draft"))
        });
        choices.items[0].disabled = true;
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                apply(v, window, cx, vec![Op::SetChoice(id(0), choices)])
            });
            window.draw(cx).clear(cx);
            let disabled: gpui::Background = rgba(0x0000ffff).into();
            assert!(
                window
                    .painted_quads()
                    .iter()
                    .any(|q| q.background == disabled)
            );
        });
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                apply(v, window, cx, vec![Op::SetTabAppearance(id(0), None)])
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        owner.read_with(cx, |v, _| {
            assert_eq!(v.buttons[&id(0)].focus, focus);
            assert!(
                v.session
                    .borrow()
                    .tree(wid)
                    .unwrap()
                    .get(id(0))
                    .unwrap()
                    .tab_appearance
                    .is_none()
            );
        });
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            assert_eq!(window.simulate_next_frame(cx), 0);
            owner.update(cx, |v, cx| {
                apply(v, window, cx, {
                    let mut ops = vec![Op::SetRoot(None)];
                    if rich {
                        ops.extend([
                            Op::Remove(id(2)),
                            Op::Remove(id(4)),
                            Op::Remove(id(1)),
                            Op::Remove(id(3)),
                        ]);
                    }
                    ops.push(Op::Remove(id(0)));
                    ops
                })
            });
            window.draw(cx).clear(cx);
        });
        assert_eq!(session.borrow().retained_bytes(), 0);
    }
}

#[test]
fn structured_tabs_preserve_child_input_close_actions_and_capped_layout() {
    use gpuio_protocol::tab_content::{Config as Content, Label};
    let mut app = TestAppContext::single();
    app.update(crate::image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    let h = |n| HandlerId::from_parts(n, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Structured tabs", 480., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let long = "A very long workspace name that needs truncation";
    let mut choices = ChoiceConfig {
        label: "Workspaces".into(),
        items: vec![
            ChoiceItem {
                id: "one".into(),
                label: long.into(),
                disabled: false,
            },
            ChoiceItem {
                id: "two".into(),
                label: "Other".into(),
                disabled: false,
            },
        ],
        selected: Some("two".into()),
        disabled: false,
    };
    let mut content = Content {
        max_width: Some(160.),
        labels: vec![Label::Default, Label::Default],
    };
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            let mut ops = vec![Op::Create(id(0), Kind::TabBar, String::new(), Some(h(0)))];
            for i in 1..=4 {
                ops.push(Op::Create(id(i), Kind::Container, String::new(), None));
            }
            ops.extend([
                Op::Create(id(5), Kind::Input, "abc".into(), Some(h(5))),
                Op::Create(id(6), Kind::Button, "×".into(), Some(h(6))),
            ]);
            for i in 7..=10 {
                ops.push(Op::Create(id(i), Kind::Container, String::new(), None));
            }
            ops.extend([
                Op::SetEditor(
                    id(5),
                    EditorConfig {
                        label: "Rename".into(),
                        placeholder: String::new(),
                        read_only: false,
                        disabled: false,
                        submit_on_enter: false,
                        auto_focus: false,
                        min_rows: 1,
                        max_rows: 1,
                    },
                ),
                Op::SetStyle(
                    id(5),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(60.)),
                        Field::Height(Length::Px(24.)),
                    ])],
                ),
                Op::SetStyle(
                    id(6),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(24.)),
                        Field::Height(Length::Px(24.)),
                        Field::AccessibleName("Close workspace".into()),
                    ])],
                ),
                Op::Splice(id(2), 0, 0, vec![id(5)]),
                Op::Splice(id(4), 0, 0, vec![id(6)]),
                Op::Splice(id(1), 0, 0, vec![id(2), id(3), id(4)]),
                Op::Splice(id(7), 0, 0, vec![id(8), id(9), id(10)]),
                Op::Splice(id(0), 0, 0, vec![id(1), id(7)]),
                Op::SetChoice(id(0), choices.clone()),
                Op::SetTabContent(id(0), Some(content.clone())),
                Op::SetTabAppearance(
                    id(0),
                    Some(gpuio_protocol::tab_appearance::Config::default()),
                ),
                Op::SetStyle(
                    id(0),
                    vec![Style::Fields(vec![
                        Field::Direction(0),
                        Field::Width(Length::Px(440.)),
                        Field::Height(Length::Px(50.)),
                    ])],
                ),
                Op::SetRoot(Some(id(0))),
            ]);
            apply(v, window, cx, ops);
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let named = |cx: &mut gpui::VisualTestContext, label: &str| {
        cx.a11y_tree()
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, n)| n.label() == Some(label))
            .unwrap()
    };
    let center = |cx: &mut gpui::VisualTestContext, label: &str| {
        let bounds = named(cx, label).1.bounds().unwrap();
        let scale = cx.update(|w, _| w.scale_factor()) as f64;
        gpui::point(
            px(((bounds.x0 + bounds.x1) / 2. / scale) as f32),
            px(((bounds.y0 + bounds.y1) / 2. / scale) as f32),
        )
    };
    let scale = cx.update(|w, _| w.scale_factor()) as f64;
    let tab = named(cx, long);
    let close = named(cx, "Close workspace");
    let editor = named(cx, "Rename");
    let bounds = tab.1.bounds().unwrap();
    assert!((bounds.width() / scale - 160.).abs() < 0.1, "{bounds:?}");
    for (child, width) in [(&close.1, 24.), (&editor.1, 60.)] {
        let child = child.bounds().unwrap();
        assert!((child.width() / scale - width).abs() < 0.1, "{child:?}");
        assert!(
            child.x0 >= bounds.x0 && child.x1 <= bounds.x1,
            "child {child:?} tab {bounds:?}"
        );
    }
    let drain = || transport.mailbox.lock().unwrap().drain(100);
    drain();
    let p = center(cx, "Close workspace");
    cx.simulate_click(p, Default::default());
    cx.run_until_parked();
    let events = drain();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e,Event::Press(_,n,..) if *n==id(6)))
            .count(),
        1
    );
    assert!(
        !events.iter().any(|e| matches!(e, Event::Choice(..))),
        "{events:?}"
    );
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: close.0,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    let events = drain();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e,Event::Press(_,n,..) if *n==id(6)))
            .count(),
        1
    );
    assert!(!events.iter().any(|e| matches!(e, Event::Choice(..))));
    let close_focus = owner.read_with(cx, |v, _| v.buttons[&id(6)].focus.clone());
    cx.update(|window, cx| {
        window.focus(&close_focus, cx);
        let keystroke = gpui::Keystroke::parse("space").unwrap();
        window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }),
            cx,
        );
    });
    cx.run_until_parked();
    let events = drain();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e,Event::Press(_,n,..) if *n==id(6)))
            .count(),
        1
    );
    assert!(!events.iter().any(|e| matches!(e, Event::Choice(..))));
    let p = center(cx, "Rename");
    cx.simulate_click(p, Default::default());
    cx.run_until_parked();
    assert!(!drain().iter().any(|e| matches!(e, Event::Choice(..))));
    cx.simulate_keystrokes("left");
    cx.simulate_keystrokes("x");
    cx.run_until_parked();
    assert!(!drain().iter().any(|e| matches!(e, Event::Choice(..))));
    let saved = cx.update(|window, cx| {
        let s = owner.read(cx).editors[&id(5)].snapshot(window, cx);
        assert!(s.focused);
        assert!(s.text.contains('x'));
        s.text
    });
    // Reorder and styling updates keep the same controls and native editor draft.
    choices.items.reverse();
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            apply(
                v,
                window,
                cx,
                vec![
                    Op::SetChoice(id(0), choices.clone()),
                    Op::Splice(id(0), 0, 2, vec![id(7), id(1)]),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    assert_eq!(named(cx, "Close workspace").0, close.0);
    cx.update(|window, cx| {
        let s = owner.read(cx).editors[&id(5)].snapshot(window, cx);
        assert_eq!(s.text, saved);
        assert!(s.focused);
    });
    // Disabled selection does not disable an independent Close control.
    choices.items[1].disabled = true;
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            apply(v, window, cx, vec![Op::SetChoice(id(0), choices.clone())])
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    drain();
    let p = center(cx, "Close workspace");
    cx.simulate_click(p, Default::default());
    cx.run_until_parked();
    let events = drain();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e,Event::Press(_,n,..) if *n==id(6)))
            .count(),
        1
    );
    assert!(!events.iter().any(|e| matches!(e, Event::Choice(..))));
    // An explicitly disabled ancestor blocks both the selection and its parts.
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            apply(
                v,
                window,
                cx,
                vec![Op::SetStyle(
                    id(0),
                    vec![Style::Fields(vec![
                        Field::Disabled(true),
                        Field::Direction(0),
                        Field::Width(Length::Px(440.)),
                    ])],
                )],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    drain();
    let p = center(cx, "Close workspace");
    cx.simulate_click(p, Default::default());
    cx.run_until_parked();
    assert!(
        !drain()
            .iter()
            .any(|e| matches!(e, Event::Press(..) | Event::Choice(..)))
    );
    choices.items[1].disabled = false;
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            apply(
                v,
                window,
                cx,
                vec![
                    Op::SetChoice(id(0), choices.clone()),
                    Op::SetStyle(
                        id(0),
                        vec![Style::Fields(vec![
                            Field::Direction(0),
                            Field::Width(Length::Px(440.)),
                        ])],
                    ),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    drain();
    let bounds = named(cx, long).1.bounds().unwrap();
    let p = gpui::point(
        px(((bounds.x0 + bounds.x1) / 2. / scale) as f32),
        px(((bounds.y0 + bounds.y1) / 2. / scale) as f32),
    );
    cx.simulate_click(p, Default::default());
    cx.run_until_parked();
    assert_eq!(
        drain()
            .iter()
            .filter(|e| matches!(e,Event::Choice(_,_,_,_,choice) if choice=="one"))
            .count(),
        1
    );
    // Decorative custom labels share the cap without gaining a semantic action.
    content.labels[1] = Label::Custom;
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            apply(
                v,
                window,
                cx,
                vec![
                    Op::Create(id(11), Kind::Text, "Custom long decoration".into(), None),
                    Op::SetStyle(
                        id(11),
                        vec![Style::Fields(vec![Field::Width(Length::Px(300.))])],
                    ),
                    Op::Splice(id(3), 0, 0, vec![id(11)]),
                    Op::SetTabContent(id(0), Some(content.clone())),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let bounds = named(cx, long).1.bounds().unwrap();
    assert!((bounds.width() / scale - 160.).abs() < 0.1);
    assert!(named(cx, "Close workspace").1.bounds().unwrap().x1 <= bounds.x1);
    assert_eq!(named(cx, "Close workspace").0, close.0);
    // TestPlatform exposes the raw AccessKit tree, including hidden descendants.
    // Verify the caption has a hidden ancestor rather than expecting its raw
    // node to be absent from the unfiltered snapshot.
    let semantics = cx.a11y_tree().unwrap().nodes;
    let mut caption = semantics
        .iter()
        .find(|(_, n)| n.label() == Some("Custom long decoration"))
        .unwrap();
    while !caption.1.is_hidden() {
        assert!(!caption.1.supports_action(gpui::accesskit::Action::Click));
        caption = semantics
            .iter()
            .find(|(_, n)| n.children().contains(&caption.0))
            .expect("decorative label must reach a hidden semantic ancestor");
    }
    // Whole-tab clipping participates in normal keyboard focus traversal.
    content.max_width = Some(1.);
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            apply(
                v,
                window,
                cx,
                vec![Op::SetTabContent(id(0), Some(content.clone()))],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let root_focus = owner.read_with(cx, |v, _| v.buttons[&id(0)].focus.clone());
    cx.update(|window, cx| window.focus(&root_focus, cx));
    cx.simulate_keystrokes("tab");
    cx.run_until_parked();
    cx.update(|window, cx| {
        let v = owner.read(cx);
        assert!(!v.buttons[&id(6)].focus.is_focused(window)); // The prefix can remain partially visible because padding imposes a minimum width.
        assert!(!v.focus.borrow().can_focus(&v.buttons[&id(6)].focus, window));
    });
    // Hidden/icon-only content is exempt from the cap.
    content.labels = vec![Label::Hidden, Label::Hidden];
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            apply(
                v,
                window,
                cx,
                vec![
                    Op::Splice(id(3), 0, 1, vec![]),
                    Op::Remove(id(11)),
                    Op::SetTabContent(id(0), Some(content.clone())),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    assert!(named(cx, long).1.bounds().unwrap().width() / scale > 80.);
    drain();
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
        assert_eq!(window.simulate_next_frame(cx), 0);
        owner.update(cx, |v, cx| {
            let mut ops = vec![Op::SetRoot(None)];
            for i in [5, 6, 2, 3, 4, 8, 9, 10, 1, 7, 0] {
                ops.push(Op::Remove(id(i)));
            }
            apply(v, window, cx, ops);
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(session.borrow().retained_bytes(), 0);
    owner.read_with(cx, |v, _| {
        assert!(v.buttons.is_empty() && v.editors.is_empty() && v.radios.is_empty())
    });
}
