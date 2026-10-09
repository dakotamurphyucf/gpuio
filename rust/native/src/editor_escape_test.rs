//! Native action routing on TestPlatform, not physical keyboard/IME acceptance.
use super::super::View;
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use std::{cell::Cell, os::fd::AsRawFd, os::unix::net::UnixStream};

fn id() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
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
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
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
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn command(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
    command: EditorCommand,
) -> EditorSnapshot {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            match view
                .editors
                .get_mut(&id())
                .unwrap()
                .command(&command, window, cx)
            {
                EditorResult::Applied(s) => s,
                other => panic!("command failed: {other:?}"),
            }
        })
    })
}
fn snapshot(owner: &Entity<View>, cx: &mut VisualTestContext) -> EditorSnapshot {
    cx.update(|window, cx| owner.read(cx).editors[&id()].snapshot(window, cx))
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn setup(kind: Kind, test: impl FnOnce(Entity<View>, Rc<Cell<usize>>, &mut VisualTestContext)) {
    let mut app = TestAppContext::single();
    let propagated = Rc::new(Cell::new(0));
    app.update(|cx| {
        gpui_base::init(cx);
        let count = propagated.clone();
        cx.on_action(move |_: &gpui_base::input::Escape, _| count.set(count.get() + 1));
    });
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Escape test", 400., 200.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session, transport));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                id(),
                kind,
                "draft".into(),
                Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetEditor(id(), config(false)),
            Op::SetRoot(Some(id())),
        ],
    );
    command(&owner, cx, EditorCommand::Focus);
    draw(cx);
    test(owner, propagated, cx);
}

#[test]
fn editor_escape_clear_is_opt_in_undoable_and_preserves_readonly_and_empty_propagation() {
    for kind in [Kind::Input, Kind::Textarea] {
        setup(kind, |owner, count, cx| {
            let before = snapshot(&owner, cx);
            cx.dispatch_action(gpui_base::input::Escape);
            assert_eq!(snapshot(&owner, cx), before);
            assert_eq!(count.get(), 1);
            apply(&owner, cx, vec![Op::SetEditorClearOnEscape(id(), true)]);
            assert_eq!(snapshot(&owner, cx), before);
            cx.dispatch_action(gpui_base::input::Escape);
            let cleared = snapshot(&owner, cx);
            assert_eq!(cleared.text, "");
            assert_eq!(cleared.selection, EditorSelection { anchor: 0, head: 0 });
            assert!(cleared.focused && cleared.revision > before.revision);
            assert_eq!(count.get(), 1);
            draw(cx);
            cx.dispatch_action(gpui_base::input::Escape);
            assert_eq!(count.get(), 2);
            assert_eq!(snapshot(&owner, cx), cleared);
            assert_eq!(command(&owner, cx, EditorCommand::Undo).text, "draft");
            assert_eq!(command(&owner, cx, EditorCommand::Redo).text, "");
            command(&owner, cx, EditorCommand::Undo);
            apply(&owner, cx, vec![Op::SetEditor(id(), config(true))]);
            let readonly = snapshot(&owner, cx);
            cx.dispatch_action(gpui_base::input::Escape);
            assert_eq!(snapshot(&owner, cx), readonly);
            assert_eq!(count.get(), 3);
            apply(
                &owner,
                cx,
                vec![
                    Op::SetEditor(id(), config(false)),
                    Op::SetEditorClearOnEscape(id(), false),
                ],
            );
            let disabled_policy = snapshot(&owner, cx);
            cx.dispatch_action(gpui_base::input::Escape);
            assert_eq!(snapshot(&owner, cx), disabled_policy);
            assert_eq!(count.get(), 4);
            let mut disabled = config(false);
            disabled.disabled = true;
            apply(
                &owner,
                cx,
                vec![
                    Op::SetEditor(id(), disabled),
                    Op::SetEditorClearOnEscape(id(), true),
                ],
            );
            let before = snapshot(&owner, cx);
            assert!(!before.focused);
            cx.dispatch_action(gpui_base::input::Escape);
            assert_eq!(snapshot(&owner, cx), before);
            assert_eq!(count.get(), 5);
        });
    }
}

#[test]
fn editor_escape_composition_takes_precedence_and_filter_rejection_preserves_selection() {
    for kind in [Kind::Input, Kind::Textarea] {
        setup(kind, |owner, count, cx| {
            apply(&owner, cx, vec![Op::SetEditorClearOnEscape(id(), true)]);
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    view.editors[&id()].mark_test_text("marked", window, cx)
                })
            });
            draw(cx);
            let marked = snapshot(&owner, cx);
            assert!(marked.composition.is_some());
            cx.dispatch_action(gpui_base::input::Escape);
            let unmarked = snapshot(&owner, cx);
            assert!(unmarked.composition.is_none());
            assert_eq!(unmarked.text, marked.text);
            assert_eq!(count.get(), 0);
            draw(cx);
            cx.dispatch_action(gpui_base::input::Escape);
            assert_eq!(snapshot(&owner, cx).text, "");
            assert_eq!(count.get(), 0);
            assert_eq!(command(&owner, cx, EditorCommand::Undo).text, marked.text);
        });
    }
    setup(Kind::Input, |owner, count, cx| {
        use gpuio_protocol::input_validation::{Matching, Rule, Source};
        apply(
            &owner,
            cx,
            vec![
                Op::SetEditorClearOnEscape(id(), true),
                Op::SetEditorValidation(
                    id(),
                    Some(Rule {
                        regex: Source {
                            pattern: ".+".into(),
                            matching: Matching::WholeValue,
                            case_sensitive: true,
                        },
                        allow_empty: false,
                    }),
                ),
            ],
        );
        command(
            &owner,
            cx,
            EditorCommand::Select(EditorSelection { anchor: 4, head: 1 }),
        );
        draw(cx);
        let before = snapshot(&owner, cx);
        cx.dispatch_action(gpui_base::input::Escape);
        assert_eq!(snapshot(&owner, cx), before);
        assert_eq!(
            count.get(),
            0,
            "rejected clear must not dismiss an ancestor"
        );
        apply(&owner, cx, vec![Op::SetEditorValidation(id(), None)]);
        cx.dispatch_action(gpui_base::input::Escape);
        assert_eq!(snapshot(&owner, cx).text, "");
        assert_eq!(command(&owner, cx, EditorCommand::Undo).text, before.text);
    });
}
