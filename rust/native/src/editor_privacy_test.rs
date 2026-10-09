//! Actual retained editor and rendered AX tree on TestPlatform; no OS window.
use super::*;
use gpui::TestAppContext;
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, ops: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let applied = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations: ops,
        })
        .unwrap();
    view.update_editors(&applied.dirty, window, cx);
    cx.notify();
}
fn command(view: &mut View, window: &mut Window, cx: &mut App, command: EditorCommand) {
    assert!(matches!(
        view.editors
            .get_mut(&node())
            .unwrap()
            .command(&command, window, cx),
        EditorResult::Applied(_)
    ));
}

#[test]
fn password_reveal_preserves_editing_and_enforces_clipboard_and_ax_privacy() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Password test", 400., 200.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(
                        node(),
                        Kind::Input,
                        "seed".into(),
                        Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
                    ),
                    Op::SetEditor(
                        node(),
                        EditorConfig {
                            label: "Password".into(),
                            placeholder: "Enter password".into(),
                            read_only: false,
                            disabled: false,
                            submit_on_enter: false,
                            auto_focus: false,
                            min_rows: 1,
                            max_rows: 1,
                        },
                    ),
                    Op::SetEditorPrivacy(node(), EditorPrivacy::PasswordHidden),
                    Op::SetRoot(Some(node())),
                ],
            );
            command(view, window, cx, EditorCommand::Focus);
            command(
                view,
                window,
                cx,
                EditorCommand::Replace(
                    "secret λ".into(),
                    EditorSelectionPolicy::End,
                    EditorUndoPolicy::Record,
                    None,
                ),
            );
            command(
                view,
                window,
                cx,
                EditorCommand::Select(EditorSelection { anchor: 9, head: 0 }),
            );
        });
        window.draw(cx).clear(cx);
    });
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    let original = cx.update(|window, cx| entity.read(cx).editors[&node()].snapshot(window, cx));
    let focus = cx.update(|_, cx| entity.read(cx).editors[&node()].focus_handle(cx));
    let mut ax_id = None;
    for privacy in [
        EditorPrivacy::PasswordHidden,
        EditorPrivacy::PasswordRevealed,
        EditorPrivacy::PasswordHidden,
        EditorPrivacy::Plain,
    ] {
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetEditorPrivacy(node(), privacy)],
                );
                assert_eq!(view.editors[&node()].snapshot(window, cx), original);
                assert_eq!(view.editors[&node()].focus_handle(cx), focus);
            });
            cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".into()));
            window.draw(cx).clear(cx);
        });
        let tree = cx.a11y_tree().unwrap();
        let (id, input) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Password"))
            .unwrap();
        assert_eq!(*ax_id.get_or_insert(*id), *id);
        if privacy == EditorPrivacy::Plain {
            assert_eq!(input.role(), gpui::accesskit::Role::TextInput);
            assert_eq!(input.value(), Some("secret λ"));
        } else {
            assert_eq!(input.role(), gpui::accesskit::Role::PasswordInput);
            assert_eq!(input.value(), None);
            assert!(input.text_selection().is_none());
            assert!(!input.supports_action(gpui::accesskit::Action::SetTextSelection));
            assert!(
                tree.nodes
                    .iter()
                    .all(|(_, n)| n.value().is_none_or(|v| !v.contains("secret")))
            );
        }
        cx.dispatch_action(gpui_base::input::Copy);
        cx.update(|_, cx| {
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some(if privacy == EditorPrivacy::PasswordHidden {
                    "sentinel"
                } else {
                    "secret λ"
                })
            );
        });
        if privacy == EditorPrivacy::PasswordHidden {
            cx.dispatch_action(gpui_base::input::Cut);
            cx.update(|window, cx| {
                assert_eq!(
                    entity.read(cx).editors[&node()].snapshot(window, cx),
                    original
                );
                assert_eq!(
                    cx.read_from_clipboard()
                        .and_then(|item| item.text())
                        .as_deref(),
                    Some("sentinel")
                );
            });
        }
    }
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            // Reveal/hide must not clear or append undo history.
            command(view, window, cx, EditorCommand::Undo);
            assert_eq!(view.editors[&node()].snapshot(window, cx).text, "seed");
            command(view, window, cx, EditorCommand::Redo);
            assert_eq!(view.editors[&node()].snapshot(window, cx).text, "secret λ");
            command(
                view,
                window,
                cx,
                EditorCommand::Select(EditorSelection { anchor: 9, head: 0 }),
            );
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorPrivacy(
                    node(),
                    EditorPrivacy::PasswordRevealed,
                )],
            );
        });
        window.draw(cx).clear(cx);
    });
    cx.dispatch_action(gpui_base::input::Cut);
    cx.update(|window, cx| {
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("secret λ")
        );
        entity.update(cx, |view, cx| {
            assert_eq!(view.editors[&node()].snapshot(window, cx).text, "");
            command(view, window, cx, EditorCommand::Undo);
            assert_eq!(view.editors[&node()].snapshot(window, cx).text, "secret λ");
            view.editors[&node()].mark_test_text("界", window, cx);
            let composing = view.editors[&node()].snapshot(window, cx);
            assert!(composing.composition.is_some());
            for privacy in [
                EditorPrivacy::PasswordHidden,
                EditorPrivacy::PasswordRevealed,
            ] {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetEditorPrivacy(node(), privacy)],
                );
                assert_eq!(view.editors[&node()].snapshot(window, cx), composing);
                assert_eq!(view.editors[&node()].focus_handle(cx), focus);
            }
        });
    });
}
