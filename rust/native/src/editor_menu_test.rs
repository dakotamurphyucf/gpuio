//! Bound menu behavior on TestPlatform, not OS clipboard/keyboard acceptance.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::TestAppContext;
use gpuio_protocol::{HandlerId, WindowId};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config(read_only: bool) -> EditorConfig {
    EditorConfig {
        label: "Draft".into(),
        placeholder: "".into(),
        read_only,
        disabled: false,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    }
}
fn menu() -> MenuConfig {
    MenuConfig {
        presentation: MenuPresentation::EditorContext,
        menus: vec![MenuDefinition {
            label: "Edit draft".into(),
            disabled: false,
            items: ["cut", "copy", "paste", "select"]
                .map(|id| MenuItem::Command(id.into()))
                .to_vec(),
        }],
    }
}
fn create_input(slot: i64, text: &str) -> Vec<Op> {
    vec![
        Op::Create(
            id(slot),
            Kind::Input,
            text.into(),
            Some(HandlerId::from_parts(slot, 1).unwrap()),
        ),
        Op::SetEditor(id(slot), config(false)),
        Op::SetStyle(
            id(slot),
            vec![
                Style::Width(Length::Px(250.)),
                Style::Height(Length::Px(40.)),
            ],
        ),
    ]
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
fn command(
    view: &mut View,
    node: NodeId,
    command: EditorCommand,
    window: &mut Window,
    cx: &mut App,
) {
    assert!(matches!(
        view.editors
            .get_mut(&node)
            .unwrap()
            .command(&command, window, cx),
        EditorResult::Applied(_)
    ));
}
fn rows(view: &View, window: &Window, cx: &App) -> Vec<Row> {
    let session = view.session.borrow();
    view.menu_rows(
        session.tree(view.id).unwrap(),
        id(1),
        &menu().menus[0],
        window,
        cx,
    )
}
fn activate(view: &mut View, index: usize, row: &Row, window: &mut Window, cx: &mut Context<View>) {
    view.activate_menu_row(
        Hit {
            id: id(1),
            path: &[0],
            index,
            row,
            expected: &menu(),
        },
        window,
        cx,
    );
}

#[test]
fn bound_editor_menu_uses_current_policy_and_never_falls_back_to_another_editor() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Bound menu", 600., 400.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let mut ops = vec![
                Op::Create(
                    id(0),
                    Kind::CommandScope,
                    "".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetCommands(
                    id(0),
                    [
                        ("cut", NativeCommand::Cut),
                        ("copy", NativeCommand::Copy),
                        ("paste", NativeCommand::Paste),
                        ("select", NativeCommand::SelectAll),
                    ]
                    .map(|(id, action)| CommandConfig {
                        id: id.into(),
                        generation: 1,
                        label: id.into(),
                        enabled: true,
                        checked: None,
                        shortcuts: vec![],
                        target: CommandTarget::Native(action),
                    })
                    .to_vec(),
                ),
                Op::Create(id(1), Kind::Menu, "".into(), None),
                Op::SetMenu(id(1), menu()),
            ];
            ops.extend(create_input(2, "alpha"));
            ops.extend(create_input(3, "beta"));
            ops.extend([
                Op::Splice(id(1), 0, 0, vec![id(2)]),
                Op::Splice(id(0), 0, 0, vec![id(1), id(3)]),
                Op::SetRoot(Some(id(0))),
            ]);
            apply(view, window, cx, ops);
            for (node, end) in [(id(2), 5), (id(3), 4)] {
                command(
                    view,
                    node,
                    EditorCommand::Select(EditorSelection {
                        anchor: end,
                        head: 0,
                    }),
                    window,
                    cx,
                );
            }
            command(view, id(3), EditorCommand::Focus, window, cx);
        });
        window.draw(cx).clear(cx);
    });
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.open_menu(id(1), 0, None, window, cx);
            let rows = rows(view, window, cx);
            assert!(rows[0].enabled && rows[1].enabled);
            assert!(!rows[2].enabled, "empty clipboard cannot paste");
            activate(view, 1, &rows[1], window, cx);
        });
    });
    cx.run_until_parked();
    for (index, expected) in [(0, ""), (2, "alpha"), (3, "alpha")] {
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                view.open_menu(id(1), 0, None, window, cx);
                let current = rows(view, window, cx);
                assert!(current[index].enabled);
                activate(view, index, &current[index], window, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let snapshot = owner.read(cx).editors[&id(2)].snapshot(window, cx);
            assert_eq!(snapshot.text, expected);
            assert_eq!(
                owner.read(cx).editors[&id(3)].snapshot(window, cx).text,
                "beta"
            );
            if index == 3 {
                assert_eq!(snapshot.selection, EditorSelection { anchor: 0, head: 5 });
            }
        });
    }
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("alpha")
            );
            assert!(view.editors[&id(2)].focus_handle(cx).is_focused(window));
            assert_eq!(view.editors[&id(3)].snapshot(window, cx).text, "beta");
            view.open_menu(id(1), 0, None, window, cx);
            let stale_copy = rows(view, window, cx)[1].clone();
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorPrivacy(id(2), EditorPrivacy::PasswordHidden)],
            );
            cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".into()));
            let current = self::rows(view, window, cx);
            assert!(!current[0].enabled && !current[1].enabled && current[2].enabled);
            activate(view, 1, &stale_copy, window, cx);
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("sentinel")
            );
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetEditorPrivacy(id(2), EditorPrivacy::Plain),
                    Op::SetEditor(id(2), config(true)),
                ],
            );
            view.open_menu(id(1), 0, None, window, cx);
            let current = self::rows(view, window, cx);
            assert!(!current[0].enabled && current[1].enabled && !current[2].enabled);
            view.close_menu(id(1), true, window, cx);
            apply(view, window, cx, vec![Op::SetEditor(id(2), config(false))]);
        });
        window.draw(cx).clear(cx);
    });
    // The click is admitted while copyable, but delivery observes the newer mask.
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.open_menu(id(1), 0, None, window, cx);
            let current = rows(view, window, cx);
            assert!(current[1].enabled);
            activate(view, 1, &current[1], window, cx);
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorPrivacy(id(2), EditorPrivacy::PasswordHidden)],
            );
        });
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("sentinel")
        );
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorPrivacy(id(2), EditorPrivacy::Plain)],
            )
        });
        window.draw(cx).clear(cx);
    });
    let position = cx.update(|_, cx| owner.read(cx).editors[&id(2)].input_bounds(cx).center());
    cx.simulate_mouse_down(
        position,
        gpui::MouseButton::Right,
        gpui::Modifiers::default(),
    );
    cx.simulate_mouse_up(
        position,
        gpui::MouseButton::Right,
        gpui::Modifiers::default(),
    );
    cx.update(|_, cx| assert_eq!(owner.read(cx).menus[&id(1)].borrow().path, vec![0]));
    cx.simulate_keystrokes("escape");
    // Keyboard opens the owning field's menu without changing directed selection.
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            command(
                view,
                id(2),
                EditorCommand::Select(EditorSelection { anchor: 5, head: 0 }),
                window,
                cx,
            );
        })
    });
    let before = cx.update(|window, cx| owner.read(cx).editors[&id(2)].snapshot(window, cx));
    cx.simulate_keystrokes("shift-f10");
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert_eq!(view.menus[&id(1)].borrow().path, vec![0]);
            let snapshot = view.editors[&id(2)].snapshot(window, cx);
            assert_eq!(snapshot.selection, before.selection);
            assert_eq!(snapshot.revision, before.revision);
        });
    });
    cx.simulate_keystrokes("escape");
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert!(view.menus[&id(1)].borrow().path.is_empty());
            assert!(view.editors[&id(2)].focus_handle(cx).is_focused(window));
            view.open_menu(id(1), 0, None, window, cx);
            let current = rows(view, window, cx);
            let old = current[1].route.clone().unwrap();
            assert!(current[1].enabled);
            activate(view, 1, &current[1], window, cx);
            let mut ops = create_input(4, "replacement");
            ops.extend([Op::Splice(id(1), 0, 1, vec![id(4)]), Op::Remove(id(2))]);
            apply(view, window, cx, ops);
            command(view, id(4), EditorCommand::Focus, window, cx);
            assert!(!view.invoke_command(&old, window, cx));
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("sentinel")
            );
        });
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| {
            assert!(view.menus[&id(1)].borrow().path.is_empty());
            view.editors[&id(4)].mark_test_text("界", window, cx);
            view.open_menu(id(1), 0, None, window, cx);
            assert!(view.menus[&id(1)].borrow().path.is_empty());
            assert!(view.editors[&id(4)].is_composing(cx));
        });
    });
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("sentinel")
        );
    });
}
