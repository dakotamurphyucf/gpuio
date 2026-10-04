//! Native retained focus/AX behavior on TestPlatform, not OS autofill acceptance.
use super::*;
use gpui::TestAppContext;
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn id(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn config(label: &str, read_only: bool) -> EditorConfig {
    EditorConfig {
        label: label.into(),
        placeholder: "".into(),
        read_only,
        disabled: false,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    }
}
fn field(n: i64, label: &str, hint: Hint) -> Vec<Op> {
    vec![
        Op::Create(
            id(n),
            Kind::Input,
            "seed".into(),
            Some(gpuio_protocol::HandlerId::from_parts(n, 1).unwrap()),
        ),
        Op::SetEditor(id(n), config(label, false)),
        Op::SetEditorContentHint(id(n), Some(hint)),
        Op::SetStyle(
            id(n),
            vec![
                Style::Width(Length::Px(240.)),
                Style::Height(Length::Px(40.)),
            ],
        ),
    ]
}
fn unavailable(hint: Hint) -> EditorResult {
    #[cfg(target_os = "macos")]
    let reason = if hint.macos_value().is_none() {
        Unavailability::Mapping
    } else {
        Unavailability::NativeView
    };
    #[cfg(not(target_os = "macos"))]
    let reason = Unavailability::Backend;
    EditorResult::ContentHintStatus(Status::Unavailable(hint, reason))
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
fn hints_follow_native_focus_and_preserve_composition_across_updates() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Content hints", 500., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let mut ops = vec![Op::Create(id(0), Kind::Container, "".into(), None)];
            ops.extend(field(1, "Email", Hint::EmailAddress));
            ops.extend(field(2, "Website", Hint::Url));
            ops.extend([
                Op::Create(
                    id(3),
                    Kind::Button,
                    "Continue".into(),
                    Some(gpuio_protocol::HandlerId::from_parts(3, 1).unwrap()),
                ),
                Op::Splice(id(0), 0, 0, vec![id(1), id(2), id(3)]),
                Op::SetRoot(Some(id(0))),
            ]);
            apply(view, window, cx, ops);
            assert!(matches!(
                view.editors
                    .get_mut(&id(1))
                    .unwrap()
                    .command(&EditorCommand::Focus, window, cx),
                EditorResult::Applied(_)
            ));
        });
        window.draw(cx).clear(cx);
        assert_eq!(
            owner.read(cx).input_content.desired.as_ref().unwrap().node,
            id(1)
        );
    });
    cx.run_until_parked();
    let tree = cx.a11y_tree().unwrap();
    for (label, role) in [
        ("Email", gpui::accesskit::Role::EmailInput),
        ("Website", gpui::accesskit::Role::UrlInput),
    ] {
        assert_eq!(
            tree.nodes
                .iter()
                .find(|(_, n)| n.label() == Some(label))
                .unwrap()
                .1
                .role(),
            role
        );
    }
    for expected in [Some(id(2)), None] {
        cx.simulate_keystrokes("tab");
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                owner
                    .read(cx)
                    .input_content
                    .desired
                    .as_ref()
                    .map(|d| d.node),
                expected
            );
        });
    }
    cx.simulate_keystrokes("shift-tab");
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| {
            assert_eq!(view.input_content.desired.as_ref().unwrap().node, id(2));
            assert_eq!(
                view.read_input_content_status(id(1), window, cx),
                EditorResult::ContentHintStatus(Status::Inactive(Some(Hint::EmailAddress)))
            );
            assert_eq!(
                view.read_input_content_status(id(2), window, cx),
                unavailable(Hint::Url)
            );
            assert_eq!(
                view.read_input_content_status(id(3), window, cx),
                EditorResult::Failed(EditorError::StaleEditor)
            );
            let focus = view.editors[&id(2)].focus_handle(cx);
            view.editors[&id(2)].mark_test_text("界", window, cx);
            let before = view.editors[&id(2)].snapshot(window, cx);
            assert!(before.composition.is_some());
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorContentHint(id(2), Some(Hint::GivenName))],
            );
            assert_eq!(
                view.read_input_content_status(id(2), window, cx),
                unavailable(Hint::GivenName)
            );
            assert_eq!(view.editors[&id(2)].snapshot(window, cx), before);
            assert_eq!(view.editors[&id(2)].focus_handle(cx), focus);
            assert_eq!(
                view.input_content.desired.as_ref().unwrap().hint,
                Hint::GivenName
            );
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditor(id(2), config("Website", true))],
            );
            assert!(view.input_content.desired.is_none());
            assert_eq!(
                view.read_input_content_status(id(2), window, cx),
                EditorResult::ContentHintStatus(Status::Inactive(Some(Hint::GivenName)))
            );
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditor(id(2), config("Website", false))],
            );
            assert_eq!(view.input_content.desired.as_ref().unwrap().node, id(2));
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorContentHint(id(2), Some(Hint::CellularImei))],
            );
            assert_eq!(
                view.read_input_content_status(id(2), window, cx),
                unavailable(Hint::CellularImei)
            );
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorContentHint(id(2), None)],
            );
            assert_eq!(
                view.read_input_content_status(id(2), window, cx),
                EditorResult::ContentHintStatus(Status::Inactive(None))
            );
            apply(
                view,
                window,
                cx,
                vec![Op::Splice(id(0), 1, 1, vec![]), Op::Remove(id(2))],
            );
            assert!(view.input_content.desired.is_none());
            assert_eq!(
                view.read_input_content_status(id(2), window, cx),
                EditorResult::Failed(EditorError::StaleEditor)
            );
        });
        window.draw(cx).clear(cx);
    });
}
