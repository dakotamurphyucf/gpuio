//! Production View/native pointer paint, not OS input or GPU acceptance.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{
    Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext,
    VisualTestContext, px,
};
use gpuio_protocol::{
    HandlerId, WindowId,
    split_button::{Config, Parts},
    v1::*,
};
use std::{cell::RefCell, os::fd::AsRawFd, os::unix::net::UnixStream, rc::Rc, sync::Arc};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn background(color: i64) -> Field {
    Field::Background(Fill::Solid(Color::Rgba(color)))
}
fn menu(label: &str) -> MenuConfig {
    MenuConfig {
        presentation: MenuPresentation::Button,
        menus: vec![MenuDefinition {
            label: label.into(),
            disabled: false,
            items: vec![],
        }],
    }
}
fn config() -> Config {
    Config {
        parts: Parts::Split,
        surface: vec![Style::Fields(vec![background(0x555555ff)])],
        menu_open: vec![Style::Fields(vec![background(0x0000ffff)])],
    }
}
fn pair(start: i64) -> Vec<Op> {
    let color = if start == 1 { 0x444444ff } else { 0x777777ff };
    let style = vec![
        Style::Width(Length::Px(80.)),
        Style::Height(Length::Px(32.)),
        Style::Fields(vec![background(color)]),
        Style::State(2, vec![background(0x00ff00ff)]),
        Style::State(3, vec![background(0xff0000ff)]),
        Style::State(6, vec![background(0xffff00ff), Field::Opacity(1.)]),
    ];
    vec![
        Op::Create(id(start), Kind::Container, "".into(), None),
        Op::SetStyle(
            id(start),
            vec![
                Style::Direction(0),
                Style::Width(Length::Px(160.)),
                Style::Height(Length::Px(32.)),
            ],
        ),
        Op::Create(id(start + 1), Kind::Container, "".into(), None),
        Op::Create(
            id(start + 2),
            Kind::Button,
            "Run".into(),
            Some(HandlerId::from_parts(start, 1).unwrap()),
        ),
        Op::SetStyle(id(start + 2), style.clone()),
        Op::Create(id(start + 3), Kind::Container, "".into(), None),
        Op::Create(id(start + 4), Kind::Menu, "".into(), None),
        Op::SetStyle(id(start + 4), style),
        Op::SetMenu(id(start + 4), menu("More")),
        Op::Splice(id(start + 1), 0, 0, vec![id(start + 2)]),
        Op::Splice(id(start + 3), 0, 0, vec![id(start + 4)]),
        Op::Splice(id(start), 0, 0, vec![id(start + 1), id(start + 3)]),
        Op::SetSplitButton(id(start), Some(config())),
    ]
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
    cx.notify();
}
fn move_pointer(cx: &mut VisualTestContext, x: f32, y: f32) {
    cx.simulate_event(MouseMoveEvent {
        position: gpui::point(px(x), px(y)),
        pressed_button: None,
        modifiers: Default::default(),
    });
}
fn click(cx: &mut VisualTestContext, x: f32, y: f32) {
    move_pointer(cx, x, y);
    cx.simulate_event(MouseDownEvent {
        position: gpui::point(px(x), px(y)),
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        position: gpui::point(px(x), px(y)),
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
}
fn paint(cx: &mut VisualTestContext, expected: &[(u32, usize)]) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let quads = window.painted_quads();
        for &(color, count) in expected {
            let background: gpui::Background = gpui::rgba(color).into();
            assert_eq!(
                quads
                    .iter()
                    .filter(|quad| quad.background == background)
                    .count(),
                count,
                "color {color:08x}, backgrounds={:?}",
                quads
                    .iter()
                    .map(|q| (&q.bounds, &q.background))
                    .collect::<Vec<_>>()
            );
        }
    });
}
fn compact_pair_style(extra: Vec<Field>) -> Op {
    let mut fields = vec![Field::AlignSelf(0)];
    fields.extend(extra);
    Op::SetStyle(id(1), vec![Style::Direction(0), Style::Fields(fields)])
}

#[test]
fn tooltip_parts_compact_hover_and_pair_lifecycle_keep_the_surviving_menu() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Lifecycle", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        let mut operations = vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(
                id(0),
                vec![
                    Style::Direction(1),
                    Style::Width(Length::Px(800.)),
                    Style::Padding(20.),
                ],
            ),
        ];
        operations.extend(pair(1));
        operations.extend([
            compact_pair_style(vec![]),
            Op::Create(id(6), Kind::Tooltip, "".into(), None),
            Op::SetTooltip(
                id(6),
                TooltipConfig {
                    label: "Primary help".into(),
                    width: 180.,
                    open_state: TooltipOpenState::Controlled(false),
                    disabled: false,
                    hoverable: false,
                    show_delay_ns: 0,
                    hide_delay_ns: 0,
                    skip_delay_ns: 0,
                },
            ),
            Op::Create(id(7), Kind::Text, "Primary help".into(), None),
            Op::Splice(id(2), 0, 1, vec![]),
            Op::Splice(id(6), 0, 0, vec![id(3), id(7)]),
            Op::Splice(id(2), 0, 0, vec![id(6)]),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ]);
        owner.update(cx, |view, cx| apply(view, window, cx, operations));
        window.draw(cx).clear(cx);
    });
    move_pointer(cx, 700., 36.);
    paint(cx, &[(0x444444ff, 2), (0x555555ff, 0)]);
    let primary_semantic = |cx: &VisualTestContext| {
        let tree = cx.a11y_tree().unwrap();
        let (id, node) = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == gpui::accesskit::Role::Button && node.label() == Some("Run")
            })
            .expect("primary action remains exposed");
        (*id, node.is_disabled())
    };
    let primary_ax = primary_semantic(cx).0;
    let primary_focus = owner.read_with(cx, |view, _| view.buttons[&id(3)].focus.clone());
    cx.update(|window, cx| window.focus(&primary_focus, cx));
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("space").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("space").unwrap(),
    });
    let presses: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::Press(_, node, _, _) => Some(node),
            _ => None,
        })
        .collect();
    assert_eq!(
        presses,
        vec![id(3)],
        "the tooltip-wrapped primary retains one keyboard action owner"
    );
    move_pointer(cx, 40., 36.);
    paint(cx, &[(0x00ff00ff, 1), (0x555555ff, 1)]);
    move_pointer(cx, 700., 500.);
    for (field, base, disabled) in [
        (Field::Disabled(true), 0, 2),
        (Field::Inert(true), 2, 0),
        (Field::Visibility(1), 0, 0),
    ] {
        let disabling = matches!(field, Field::Disabled(true));
        click(cx, 120., 36.);
        move_pointer(cx, 700., 500.);
        paint(cx, &[(0x555555ff, 1), (0x0000ffff, 1)]);
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(view, window, cx, vec![compact_pair_style(vec![field])])
            })
        });
        paint(
            cx,
            &[
                (0x444444ff, base),
                (0xffff00ff, disabled),
                (0x555555ff, 0),
                (0x0000ffff, 0),
            ],
        );
        if disabling {
            assert_eq!(
                primary_semantic(cx),
                (primary_ax, true),
                "ancestor disabling preserves the tooltip anchor's semantic identity"
            );
        }
        assert_eq!(
            owner.read_with(cx, |view, _| view.buttons[&id(3)].focus.clone()),
            primary_focus
        );
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(view, window, cx, vec![compact_pair_style(vec![])])
            })
        });
        paint(cx, &[(0x444444ff, 2), (0x555555ff, 0), (0x0000ffff, 0)]);
        assert_eq!(primary_semantic(cx), (primary_ax, false));
    }
    click(cx, 120., 36.);
    move_pointer(cx, 700., 500.);
    let menu_focus = owner.read_with(cx, |view, _| view.menus[&id(5)].borrow().focus.clone());
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Splice(id(1), 0, 1, vec![]),
                    Op::Splice(id(2), 0, 1, vec![]),
                    Op::Splice(id(6), 0, 2, vec![]),
                    Op::Remove(id(3)),
                    Op::Remove(id(7)),
                    Op::Remove(id(6)),
                    Op::Remove(id(2)),
                    Op::SetSplitButton(
                        id(1),
                        Some(Config {
                            parts: Parts::Menu,
                            ..config()
                        }),
                    ),
                ],
            )
        });
        window.draw(cx).clear(cx);
        assert!(menu_focus.is_focused(window));
        assert_eq!(owner.read(cx).menus[&id(5)].borrow().focus, menu_focus);
    });
    paint(cx, &[(0x444444ff, 0), (0x555555ff, 0), (0x0000ffff, 1)]);
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("escape").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    paint(cx, &[(0x444444ff, 1), (0x0000ffff, 0)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Splice(id(0), 0, 1, vec![]),
                    Op::Splice(id(1), 0, 1, vec![]),
                    Op::Splice(id(4), 0, 1, vec![]),
                    Op::Remove(id(5)),
                    Op::Remove(id(4)),
                    Op::Remove(id(1)),
                    Op::Remove(id(0)),
                ],
            )
        });
        window.draw(cx).clear(cx);
        assert!(owner.read(cx).menus.is_empty());
        assert!(owner.read(cx).buttons.is_empty());
    });
    session.borrow_mut().close(window_id).unwrap();
}

#[test]
fn split_native_hover_is_scoped_and_menu_held_paint_uses_current_state() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Split", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        let mut operations = vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(
                id(0),
                vec![Style::Direction(1), Style::Gap(20.), Style::Padding(20.)],
            ),
        ];
        operations.extend(pair(1));
        operations.extend(pair(6));
        operations.extend([
            Op::Splice(id(0), 0, 0, vec![id(1), id(6)]),
            Op::SetRoot(Some(id(0))),
        ]);
        owner.update(cx, |view, cx| apply(view, window, cx, operations));
        window.draw(cx).clear(cx);
    });
    paint(cx, &[(0x444444ff, 2), (0x777777ff, 2), (0x555555ff, 0)]);
    let primary = owner.read_with(cx, |view, _| view.buttons[&id(3)].focus.clone());
    move_pointer(cx, 40., 36.);
    paint(cx, &[(0x00ff00ff, 1), (0x555555ff, 1), (0x777777ff, 2)]);
    click(cx, 120., 36.);
    move_pointer(cx, 700., 500.);
    paint(cx, &[(0x555555ff, 1), (0x0000ffff, 1), (0x777777ff, 2)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetButtonPresentation(
                    id(3),
                    Some(gpuio_protocol::button::Config {
                        policy: gpuio_protocol::button::Policy {
                            loading: true,
                            ..Default::default()
                        },
                        content: Default::default(),
                    }),
                )],
            )
        })
    });
    paint(cx, &[(0x444444ff, 1), (0x0000ffff, 1), (0x555555ff, 0)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetButtonPresentation(id(3), None),
                    Op::SetControl(id(3), Control::Button(true)),
                ],
            )
        })
    });
    paint(cx, &[(0xffff00ff, 1), (0x0000ffff, 1)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetControl(id(3), Control::Button(false)),
                    Op::SetMenu(id(5), menu("Changed")),
                ],
            )
        });
        // Assert the first explicit draw before TestAppContext can perform a
        // follow-up draw at the update boundary and hide a stale first frame.
        window.draw(cx).clear(cx);
        let held: gpui::Background = gpui::rgba(0x555555ff).into();
        assert!(
            !window
                .painted_quads()
                .iter()
                .any(|quad| quad.background == held),
            "definition replacement left held paint in its first frame"
        );
    });
    // Primary renders before Menu: replacement must clear held paint this frame.
    paint(cx, &[(0x444444ff, 2), (0x555555ff, 0), (0x0000ffff, 0)]);
    assert_eq!(
        owner.read_with(cx, |view, _| view.buttons[&id(3)].focus.clone()),
        primary
    );
    click(cx, 120., 36.);
    move_pointer(cx, 700., 500.);
    paint(cx, &[(0x555555ff, 1), (0x0000ffff, 1)]);
    cx.update(|window, cx| window.blur(cx));
    paint(cx, &[(0x444444ff, 2), (0x555555ff, 0), (0x0000ffff, 0)]);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![Op::SetSplitButton(id(1), None)])
        })
    });
    move_pointer(cx, 40., 36.);
    paint(cx, &[(0x00ff00ff, 1), (0x444444ff, 1), (0x555555ff, 0)]);
    session.borrow_mut().close(window_id).unwrap();
}
