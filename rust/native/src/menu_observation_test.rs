//! Production menu rendering/state on TestPlatform, not OS/AX acceptance.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::TestAppContext;
use gpuio_protocol::{HandlerId, WindowId};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn config(label: &str, disabled: bool) -> MenuConfig {
    MenuConfig {
        presentation: MenuPresentation::Button,
        menus: vec![MenuDefinition {
            label: label.into(),
            disabled,
            items: vec![
                MenuItem::Command("run".into()),
                MenuItem::Submenu(MenuDefinition {
                    label: "More".into(),
                    disabled: false,
                    items: vec![MenuItem::Command("run".into())],
                }),
            ],
        }],
    }
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
fn observations(transport: &Transport) -> Vec<(NodeId, HandlerId, bool)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::MenuOpenChanged(_, node, handler, _, open) => Some((node, handler, open)),
            _ => None,
        })
        .collect()
}
fn key(cx: &mut gpui::VisualTestContext, name: &str) {
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse(name).unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn positioned_menu(x: f64, y: f64) -> Op {
    Op::SetStyle(
        id(1),
        vec![Style::Fields(vec![
            Field::Position(1),
            Field::Left(Length::Px(x)),
            Field::Top(Length::Px(y)),
            Field::Width(Length::Px(80.)),
            Field::Height(Length::Px(32.)),
        ])],
    )
}

#[test]
fn open_menu_placement_tracks_all_sides_alignment_edges_resize_and_scale() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Placement", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.simulate_resize(gpui::size(px(800.), px(600.)));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(
                        id(0),
                        Kind::CommandScope,
                        "".into(),
                        Some(HandlerId::from_parts(1, 1).unwrap()),
                    ),
                    Op::SetCommands(
                        id(0),
                        vec![CommandConfig {
                            id: "run".into(),
                            label: "Run".into(),
                            generation: 1,
                            enabled: true,
                            checked: None,
                            shortcuts: vec![],
                            target: CommandTarget::Callback,
                        }],
                    ),
                    Op::Create(id(1), Kind::Menu, "".into(), Some(handler(1))),
                    Op::SetMenu(id(1), config("Placement", false)),
                    Op::SetChoiceAppearance(
                        id(1),
                        ChoiceAppearance {
                            popup_width: 160.,
                            ..crate::appearance::default().as_ref().clone()
                        },
                    ),
                    positioned_menu(300., 250.),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
        });
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| view.open_menu(id(1), 0, None, window, cx));
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        observations(&transport),
        vec![(id(1), handler(1), false), (id(1), handler(1), true)]
    );
    let focus = owner.read_with(cx, |view, _| view.menus[&id(1)].borrow().focus.clone());
    for scale in [1., 1.5, 2.] {
        cx.simulate_scale_factor_change(scale);
        for side in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
            for align in [Align::Start, Align::Center, Align::End] {
                cx.update(|window, cx| {
                    owner.update(cx, |view, cx| apply(view, window, cx, vec![
                        Op::SetPlacement(id(1), Some(Placement { side, align, offset: 8. })),
                    ]));
                    window.draw(cx).clear(cx);
                    assert_eq!(window.scale_factor(), scale);
                    assert!(focus.is_focused(window));
                    owner.read_with(cx, |view, _| {
                        let state = view.menus[&id(1)].borrow();
                        assert_eq!(state.focus, focus);
                        assert_eq!(state.path, vec![0]);
                        let trigger = state.triggers[0].get();
                        // Recorded hit bounds exclude the default one-pixel border.
                        let content = state.panels[0].get();
                        let panel = Bounds::new(content.origin - gpui::point(px(1.), px(1.)),
                            content.size + gpui::size(px(2.), px(2.)));
                        let near = match side {
                            Side::Top => trigger.top() - panel.bottom(),
                            Side::Bottom => panel.top() - trigger.bottom(),
                            Side::Left => trigger.left() - panel.right(),
                            Side::Right => panel.left() - trigger.right(),
                        };
                        // Layout/border edges are snapped to device pixels.
                        let tolerance = px(1. / scale + 0.0001);
                        assert!((near - px(8.)).abs() < tolerance,
                            "side={side:?} align={align:?} scale={scale} trigger={trigger:?} panel={panel:?}");
                        let vertical = matches!(side, Side::Top | Side::Bottom);
                        let delta = match (vertical, align) {
                            (true, Align::Start) => panel.left() - trigger.left(),
                            (true, Align::Center) => panel.center().x - trigger.center().x,
                            (true, Align::End) => panel.right() - trigger.right(),
                            (false, Align::Start) => panel.top() - trigger.top(),
                            (false, Align::Center) => panel.center().y - trigger.center().y,
                            (false, Align::End) => panel.bottom() - trigger.bottom(),
                        };
                        assert!(delta.abs() < tolerance, "cross-axis alignment: {delta:?}");
                    });
                });
            }
        }
    }
    // Moving the live trigger and changing the viewport must use this frame's
    // geometry without dismissing the menu or asking OCaml to reopen it.
    for (width, height) in [(800., 600.), (400., 300.)] {
        cx.simulate_resize(gpui::size(px(width), px(height)));
        for (x, y, preferred, actual) in [
            (0., 0., Side::Top, Side::Bottom),
            (0., 0., Side::Left, Side::Right),
            (width - 80., height - 32., Side::Bottom, Side::Top),
            (width - 80., height - 32., Side::Right, Side::Left),
        ] {
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![
                            positioned_menu(f64::from(x), f64::from(y)),
                            Op::SetPlacement(
                                id(1),
                                Some(Placement {
                                    side: preferred,
                                    align: Align::End,
                                    offset: 8.,
                                }),
                            ),
                        ],
                    )
                });
                window.draw(cx).clear(cx);
                assert!(focus.is_focused(window));
                owner.read_with(cx, |view, _| {
                    let state = view.menus[&id(1)].borrow();
                    let trigger = state.triggers[0].get();
                    let panel = state.panels[0].get();
                    assert!((trigger.left() - px(x)).abs() < px(0.1));
                    assert!((trigger.top() - px(y)).abs() < px(0.1));
                    assert!(panel.left() >= px(9.) && panel.top() >= px(9.));
                    assert!(panel.right() <= px(width - 9.) && panel.bottom() <= px(height - 9.));
                    let gap = match actual {
                        Side::Top => trigger.top() - panel.bottom(),
                        Side::Bottom => panel.top() - trigger.bottom(),
                        Side::Left => trigger.left() - panel.right(),
                        Side::Right => panel.left() - trigger.right(),
                    };
                    assert!(
                        (gap - px(9.)).abs() < px(0.1),
                        "flip: {preferred:?} trigger={trigger:?} panel={panel:?}"
                    );
                });
            });
        }
    }
    assert!(
        observations(&transport).is_empty(),
        "geometry changes are not visibility changes"
    );
    cx.update(|window, cx| owner.update(cx, |view, cx| view.close_menu(id(1), false, window, cx)));
    assert_eq!(observations(&transport), vec![(id(1), handler(1), false)]);
}

#[test]
fn menu_observation_overflow_faults_after_render_without_reentrant_session_borrow() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Overflow", 400., 300.)
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
                    Op::Create(id(1), Kind::Menu, "".into(), Some(handler(1))),
                    Op::SetMenu(
                        id(1),
                        MenuConfig {
                            presentation: MenuPresentation::Button,
                            menus: vec![MenuDefinition {
                                label: "Empty".into(),
                                disabled: false,
                                items: vec![],
                            }],
                        },
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(observations(&transport), vec![(id(1), handler(1), false)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            for _ in 0..crate::mailbox::MAX_INPUT_EVENTS / 2 {
                view.open_menu(id(1), 0, None, window, cx);
                view.close_menu(id(1), false, window, cx);
            }
            // Attach the replacement while the input lane is exactly full.
            // Its initial snapshot is published from View::render, which has
            // a shared Session borrow. The fault must wait until render ends.
            apply(view, window, cx, vec![Op::Bind(id(1), Some(handler(2)))]);
        });
        window.draw(cx).clear(cx);
    });
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert_eq!(events.len(), crate::mailbox::MAX_INPUT_EVENTS + 1);
    for (index, event) in events[..crate::mailbox::MAX_INPUT_EVENTS]
        .iter()
        .enumerate()
    {
        assert!(
            matches!(event, Event::MenuOpenChanged(w, n, h, _, open)
            if *w == window_id && *n == id(1) && *h == handler(1) && *open == (index % 2 == 0)),
            "edge {index}: {event:?}"
        );
    }
    assert!(matches!(events.last(), Some(Event::Overloaded(w)) if *w == window_id));
    assert!(
        session
            .borrow()
            .menu_open_changed(window_id, id(1), handler(2), 2, false)
            .is_none()
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.open_menu(id(1), 0, None, window, cx);
            view.close_menu(id(1), false, window, cx);
        });
        window.draw(cx).clear(cx);
    });
    assert!(
        transport.mailbox.lock().unwrap().drain(256).is_empty(),
        "an overloaded window cannot resume ordinary input or emit duplicate faults"
    );
    session.borrow_mut().close(window_id).unwrap();
}

#[test]
fn menu_subscriptions_snapshot_transition_replace_and_retire_without_idle_duplicates() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Menus", 800., 600.)
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
                    Op::Create(
                        id(0),
                        Kind::CommandScope,
                        "".into(),
                        Some(HandlerId::from_parts(1, 1).unwrap()),
                    ),
                    Op::SetCommands(
                        id(0),
                        vec![CommandConfig {
                            id: "run".into(),
                            label: "Run".into(),
                            generation: 1,
                            enabled: true,
                            checked: None,
                            shortcuts: vec![],
                            target: CommandTarget::Callback,
                        }],
                    ),
                    Op::SetStyle(id(0), vec![Style::Padding(20.)]),
                    Op::Create(id(1), Kind::Menu, "".into(), Some(handler(1))),
                    Op::SetMenu(id(1), config("First", false)),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Width(Length::Px(80.)),
                            Style::Height(Length::Px(32.)),
                        ],
                    ),
                    Op::Create(id(2), Kind::Menu, "".into(), None),
                    Op::SetMenu(id(2), config("Second", false)),
                    Op::SetStyle(
                        id(2),
                        vec![
                            Style::Width(Length::Px(80.)),
                            Style::Height(Length::Px(32.)),
                        ],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(observations(&transport), vec![(id(1), handler(1), false)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| view.open_menu(id(1), 0, None, window, cx));
        window.draw(cx).clear(cx);
        window.draw(cx).clear(cx);
    });
    assert_eq!(observations(&transport), vec![(id(1), handler(1), true)]);
    let focus = owner.read_with(cx, |view, _| view.menus[&id(1)].borrow().focus.clone());
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            // An invalid target must not dismiss an already open peer.
            view.open_menu(id(2), 10, None, window, cx);
            assert_eq!(view.menus[&id(1)].borrow().path, vec![0]);
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Bind(id(1), Some(handler(2))),
                    Op::SetPlacement(
                        id(1),
                        Some(Placement {
                            side: Side::Right,
                            align: Align::Start,
                            offset: 8.,
                        }),
                    ),
                ],
            );
        });
        window.draw(cx).clear(cx);
        assert!(focus.is_focused(window));
        owner.read_with(cx, |view, _| {
            let state = view.menus[&id(1)].borrow();
            assert_eq!(state.focus, focus);
            assert_eq!(state.path, vec![0]);
            // Panel hit geometry is inside the default one-pixel border.
            assert!(
                (state.panels[0].get().left() - state.triggers[0].get().right() - px(9.)).abs()
                    < px(0.1),
                "panel={:?} trigger={:?} viewport={:?}",
                state.panels[0].get(),
                state.triggers[0].get(),
                window.viewport_size()
            );
        });
    });
    assert_eq!(
        observations(&transport),
        vec![(id(1), handler(2), true)],
        "replacement snapshots current open state"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.close_menu(id(1), true, window, cx);
            view.open_menu(id(1), 0, None, window, cx);
            view.close_menu(id(1), true, window, cx);
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        observations(&transport),
        vec![
            (id(1), handler(2), false),
            (id(1), handler(2), true),
            (id(1), handler(2), false)
        ],
        "same-revision edges are retained"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| view.open_menu(id(1), 0, None, window, cx));
        window.draw(cx).clear(cx);
    });
    assert_eq!(observations(&transport), vec![(id(1), handler(2), true)]);
    key(cx, "end");
    key(cx, "right");
    owner.read_with(cx, |view, _| {
        assert_eq!(view.menus[&id(1)].borrow().path.len(), 2)
    });
    key(cx, "left");
    assert!(
        observations(&transport).is_empty(),
        "submenu movement does not toggle root visibility"
    );
    key(cx, "home");
    key(cx, "enter");
    let order: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::MenuOpenChanged(_, _, _, _, false) => Some("closed"),
            Event::CommandInvoked(_, _, _, _, command, _, CommandSource::Menu(_))
                if command == "run" =>
            {
                Some("invoked")
            }
            _ => None,
        })
        .collect();
    assert_eq!(order, vec!["closed", "invoked"]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| view.open_menu(id(1), 0, None, window, cx));
        window.blur(cx);
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        observations(&transport),
        vec![(id(1), handler(2), true), (id(1), handler(2), false)],
        "focus loss reports closure"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.open_menu(id(1), 0, None, window, cx);
            view.visited.remove(&id(1));
            view.retire_unvisited_menus(window, cx);
            assert!(!view.menus.contains_key(&id(1)));
        });
    });
    assert_eq!(
        observations(&transport),
        vec![
            (id(1), handler(2), true),
            (id(1), handler(2), false),
            (id(1), handler(2), false)
        ],
        "culling reports closure; the notified frame remounts with a closed snapshot"
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        observations(&transport).is_empty(),
        "redraw after remount is silent"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.open_menu(id(1), 0, None, window, cx);
            view.open_menu(id(2), 0, None, window, cx);
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        observations(&transport),
        vec![(id(1), handler(2), true), (id(1), handler(2), false)],
        "peer replacement closes the old observer"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![Op::Bind(id(2), Some(handler(3)))]);
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        observations(&transport),
        vec![(id(2), handler(3), true)],
        "late attachment gets an open snapshot"
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetMenu(id(2), config("Disabled", true))],
            )
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(observations(&transport), vec![(id(2), handler(3), false)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::Bind(id(1), None), Op::Bind(id(2), None)],
            )
        });
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| view.open_menu(id(1), 0, None, window, cx));
        window.draw(cx).clear(cx);
    });
    assert!(observations(&transport).is_empty());
    assert!(
        session
            .borrow()
            .menu_open_changed(window_id, id(1), handler(2), 1, true)
            .is_none()
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Splice(id(0), 0, 2, vec![]),
                    Op::Remove(id(1)),
                    Op::Remove(id(2)),
                    Op::Remove(id(0)),
                ],
            )
        });
        window.draw(cx).clear(cx);
        assert!(owner.read(cx).menus.is_empty());
    });
    assert!(observations(&transport).is_empty());
    session.borrow_mut().close(window_id).unwrap();
    assert!(
        session
            .borrow()
            .menu_open_changed(window_id, id(1), handler(2), 1, false)
            .is_none()
    );
}

#[test]
fn section_labels_expose_text_without_actions_and_navigation_skips_them() {
    check_section_labels(false);
}

#[test]
fn rich_menu_rows_paint_once_keep_owner_semantics_and_native_navigation() {
    check_section_labels(true);
}

fn check_section_labels(rich: bool) {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Labels", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    let mut labeled = config("Sections", false);
    labeled.menus[0].items = vec![
        MenuItem::Label("Workflow".into()),
        MenuItem::Command("run".into()),
        MenuItem::Label("Review".into()),
        MenuItem::Command("next".into()),
    ];
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::CommandScope, "".into(), Some(handler(1))),
                    Op::SetCommands(
                        id(0),
                        ["run", "next"]
                            .into_iter()
                            .map(|id| CommandConfig {
                                id: id.into(),
                                label: id.into(),
                                generation: 1,
                                enabled: true,
                                checked: None,
                                shortcuts: vec![],
                                target: CommandTarget::Callback,
                            })
                            .collect(),
                    ),
                    Op::Create(id(1), Kind::Menu, "".into(), None),
                    Op::SetMenu(id(1), labeled.clone()),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            )
        });
        if rich {
            owner.update(cx, |view, cx| {
                let mut ops = Vec::new();
                for slot in 2..6 {
                    ops.push(Op::Create(id(slot), Kind::Container, "".into(), None));
                }
                ops.extend([
                    Op::Create(id(6), Kind::Text, "Decorative artwork".into(), None),
                    Op::SetStyle(
                        id(6),
                        vec![Style::Fields(vec![
                            Field::Background(Fill::Solid(Color::Rgba(0xff00ffff))),
                            Field::Width(Length::Px(35.)),
                            Field::Height(Length::Px(20.)),
                        ])],
                    ),
                    Op::Create(id(7), Kind::Loading, "".into(), None),
                    Op::SetSpinner(
                        id(7),
                        gpuio_protocol::spinner::Config {
                            label: "Decorative activity".into(),
                            animated: true,
                            period_ms: 1000,
                            easing: gpuio_protocol::animation::Easing::Linear,
                            source: None,
                        },
                    ),
                    Op::Splice(id(4), 0, 0, vec![id(7)]),
                    Op::Splice(id(3), 0, 0, vec![id(6)]),
                    Op::Splice(id(1), 0, 0, (2..6).map(id).collect()),
                ]);
                apply(view, window, cx, ops);
            });
        }
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| view.open_menu(id(1), 0, None, window, cx));
        window.draw(cx).clear(cx);
    });
    let panel_width = owner.read_with(cx, |view, _| {
        view.menus[&id(1)].borrow().panels[0].get().size.width
    });
    cx.update(|window, _| {
        let fills: [gpui::Background; 2] = [rgba(0xdbeaffff).into(), rgba(0x385477ff).into()];
        assert!(
            window
                .painted_quads()
                .iter()
                .any(|quad| fills.contains(&quad.background)
                    && quad.bounds.size.width.0
                        >= (f32::from(panel_width) - 2.) * window.scale_factor()),
            "selection highlight spans the menu row width"
        );
    });
    if rich {
        cx.update(|window, _| {
            let color: gpui::Background = rgba(0xff00ffff).into();
            assert_eq!(
                window
                    .painted_quads()
                    .iter()
                    .filter(|q| q.background == color)
                    .count(),
                1
            );
        });
        cx.update(|window, cx| assert!(window.simulate_next_frame(cx) > 0));
        owner.read_with(cx, |view, _| assert_eq!(view.spinners.len(), 1));
    }
    let ax = cx.a11y_tree().unwrap();
    for text in ["Workflow", "Review"] {
        let matching: Vec<_> = ax
            .nodes
            .iter()
            .filter(|(_, node)| node.label() == Some(text))
            .collect();
        assert_eq!(matching.len(), 1);
        let node = &matching[0].1;
        assert_eq!(node.role(), gpui::accesskit::Role::Label);
        assert!(!node.is_disabled());
        assert!(!node.supports_action(gpui::accesskit::Action::Click));
        assert!(!node.supports_action(gpui::accesskit::Action::Focus));
    }
    let selected = |cx: &mut gpui::VisualTestContext| {
        owner.read_with(cx, |view, _| view.menus[&id(1)].borrow().selected[0])
    };
    assert_eq!(selected(cx), Some(1));
    key(cx, "down");
    assert_eq!(selected(cx), Some(3));
    key(cx, "down");
    assert_eq!(selected(cx), Some(1));
    key(cx, "home");
    assert_eq!(selected(cx), Some(1));
    key(cx, "w");
    assert_eq!(selected(cx), Some(1), "typeahead must not select Workflow");
    key(cx, "end");
    assert_eq!(selected(cx), Some(3));
    key(cx, "enter");
    if rich {
        cx.update(|window, cx| {
            // Drain the already requested frame; a closed popup must not keep
            // its still-mounted decorative spinner requesting new frames.
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            assert_eq!(window.simulate_next_frame(cx), 0);
        });
    }
    let events = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| {
            if let Event::CommandInvoked(_, _, _, _, id, _, _) = event {
                Some(id)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(events, vec!["next"]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let mut only_labels = labeled;
            only_labels.menus[0].items = vec![MenuItem::Label("Only text".into())];
            let mut ops = vec![Op::SetMenu(id(1), only_labels)];
            if rich {
                ops.push(Op::Splice(id(1), 0, 4, vec![]));
                ops.extend((2..8).map(|slot| Op::Remove(id(slot))));
            }
            apply(view, window, cx, ops);
        });
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| view.open_menu(id(1), 0, None, window, cx));
        window.draw(cx).clear(cx);
    });
    key(cx, "down");
    key(cx, "enter");
    assert_eq!(selected(cx), None);
    key(cx, "escape");
    owner.read_with(cx, |view, _| {
        assert!(view.menus[&id(1)].borrow().path.is_empty());
        assert!(view.spinners.is_empty());
    });
}
