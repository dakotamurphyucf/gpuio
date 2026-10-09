//! Actual retained native editor on TestPlatform. No physical OS IME is claimed.
use super::*;
use crate::session::Session;
use gpui::TestAppContext;
use gpuio_protocol::{
    HandlerId,
    input_format::{Config, Number},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn with_input(
    seed: &str,
    format: Option<Config>,
    f: impl FnOnce(&mut Instance, &mut Window, &mut App),
) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Format", 400., 200.)
        .unwrap();
    session
        .borrow_mut()
        .apply(&Transaction {
            window: wid,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(
                    node(),
                    Kind::Input,
                    seed.into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetEditor(
                    node(),
                    EditorConfig {
                        label: "Draft".into(),
                        placeholder: "Original placeholder".into(),
                        read_only: false,
                        disabled: false,
                        submit_on_enter: false,
                        auto_focus: false,
                        min_rows: 1,
                        max_rows: 1,
                    },
                ),
                Op::SetEditorFormat(node(), format),
                Op::SetRoot(Some(node())),
            ],
        })
        .unwrap();
    let cx = app.add_empty_window();
    cx.update(|window, cx| {
        let description = session
            .borrow()
            .tree(wid)
            .unwrap()
            .get(node())
            .unwrap()
            .clone();
        let gate = super::super::focus::Manager::new(wid, session.clone());
        let entity =
            cx.new(|cx| Instance::new(wid, &description, session, gate, transport, window, cx));
        entity.update(cx, |input, cx| f(input, window, cx));
    });
}
fn set_format(input: &mut Instance, format: Option<Config>, window: &mut Window, cx: &mut App) {
    let base = input
        .route
        .session
        .borrow()
        .tree(input.route.window)
        .unwrap()
        .revision();
    input
        .route
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: input.route.window,
            base,
            revision: base + 1,
            operations: vec![Op::SetEditorFormat(node(), format)],
        })
        .unwrap();
    let description = input
        .route
        .session
        .borrow()
        .tree(input.route.window)
        .unwrap()
        .get(node())
        .unwrap()
        .clone();
    input.configure(&description, window, cx);
}
fn state(input: &Instance) -> &Entity<InputState> {
    let State::Input(state) = &input.state else {
        panic!("expected single line")
    };
    state
}

fn filter(pattern: &str, allow_empty: bool) -> gpuio_protocol::input_validation::Rule {
    use gpuio_protocol::input_validation::{Matching, Rule, Source};
    Rule {
        regex: Source {
            pattern: pattern.into(),
            matching: Matching::WholeValue,
            case_sensitive: true,
        },
        allow_empty,
    }
}

fn set_filter(
    input: &mut Instance,
    rule: Option<gpuio_protocol::input_validation::Rule>,
    window: &mut Window,
    cx: &mut App,
) {
    let base = input
        .route
        .session
        .borrow()
        .tree(input.route.window)
        .unwrap()
        .revision();
    input
        .route
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: input.route.window,
            base,
            revision: base + 1,
            operations: vec![Op::SetEditorValidation(node(), rule)],
        })
        .unwrap();
    let description = input
        .route
        .session
        .borrow()
        .tree(input.route.window)
        .unwrap()
        .get(node())
        .unwrap()
        .clone();
    input.configure(&description, window, cx);
}

#[test]
fn input_validation_retains_drafts_history_and_identity_but_filters_valid_edits() {
    with_input("incompatible", None, |input, window, cx| {
        let before = input.snapshot(window, cx);
        let identity = state(input).entity_id();
        set_filter(input, Some(filter("[0-9]*", true)), window, cx);
        assert_eq!(input.snapshot(window, cx), before);
        assert_eq!(state(input).entity_id(), identity);
        let compiled = input.validation.as_ref().unwrap().clone();
        let retired = Arc::downgrade(&compiled);
        set_filter(input, Some(filter("[0-9]*", true)), window, cx);
        assert!(Arc::ptr_eq(&compiled, input.validation.as_ref().unwrap()));
        drop(compiled);
        state(input).update(cx, |state, cx| {
            state.replace_text_in_range(Some(0..12), "repair", window, cx);
            assert_eq!(state.value().as_str(), "repair");
            state.replace_text_in_range(Some(0..6), "12", window, cx);
            assert_eq!(state.value().as_str(), "12");
        });
        let accepted = input.snapshot(window, cx);
        state(input).update(cx, |state, cx| {
            state.replace_text_in_range(Some(2..2), "x", window, cx)
        });
        assert_eq!(input.snapshot(window, cx), accepted);
        assert_eq!(
            input.command(&replace("x", EditorSelectionPolicy::End), window, cx),
            EditorResult::Failed(EditorError::InvalidText)
        );
        assert_eq!(input.snapshot(window, cx), accepted);
        assert_eq!(
            applied(input, EditorCommand::Undo, window, cx)
                .text
                .as_str(),
            "repair"
        );
        assert_eq!(
            applied(input, EditorCommand::Redo, window, cx)
                .text
                .as_str(),
            "12"
        );
        set_filter(input, Some(filter("[A-Z]*", true)), window, cx);
        assert!(
            retired.upgrade().is_none(),
            "replaced policy and cache are released"
        );
        // Edit filtering is deliberately not business/submission validity.
        assert_eq!(
            applied(input, EditorCommand::Submit, window, cx)
                .text
                .as_str(),
            "12"
        );
        set_filter(input, None, window, cx);
        assert_eq!(
            applied(
                input,
                replace("free", EditorSelectionPolicy::End),
                window,
                cx
            )
            .text
            .as_str(),
            "free"
        );
        assert_eq!(state(input).entity_id(), identity);
    });
}

#[test]
fn input_validation_policy_changes_during_composition_use_the_current_rule() {
    with_input("", None, |input, window, cx| {
        state(input).update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "jie", Some(0..3), window, cx);
        });
        let provisional = input.snapshot(window, cx);
        set_filter(input, Some(filter("[0-9]*", true)), window, cx);
        assert_eq!(input.snapshot(window, cx), provisional);
        state(input).update(cx, |state, cx| {
            state.replace_text_in_range(None, "界", window, cx);
            assert_eq!(state.value().as_str(), "");
            assert!(state.bridge_composition().is_none());
        });
        assert_eq!(
            applied(input, EditorCommand::Undo, window, cx)
                .text
                .as_str(),
            ""
        );
        state(input).update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "jie", Some(0..3), window, cx);
        });
        set_filter(input, None, window, cx);
        state(input).update(cx, |state, cx| {
            state.replace_text_in_range(None, "界", window, cx);
            assert_eq!(state.value().as_str(), "界");
            assert!(state.bridge_composition().is_none());
        });
        assert_eq!(
            applied(input, EditorCommand::Undo, window, cx)
                .text
                .as_str(),
            ""
        );
    });
}

#[test]
fn input_validation_runs_after_formatting_and_defines_empty_and_composition_behavior() {
    with_input(
        "",
        Some(Config::Pattern("99-99".into())),
        |input, window, cx| {
            set_filter(input, Some(filter("[0-9-]*", true)), window, cx);
            state(input).update(cx, |state, cx| {
                state.replace_text_in_range(None, "1234", window, cx);
                assert_eq!(state.value().as_str(), "12-34");
                assert_eq!(state.bridge_selection(), (5, 5));
            });
            set_filter(input, Some(filter("[0-9]*", true)), window, cx);
            assert_eq!(
                input.command(&replace("12-34", EditorSelectionPolicy::End), window, cx),
                EditorResult::Failed(EditorError::InvalidText)
            );
            assert_eq!(
                applied(input, replace("", EditorSelectionPolicy::End), window, cx)
                    .text
                    .as_str(),
                ""
            );
            set_format(input, None, window, cx);
            set_filter(input, Some(filter("界+", false)), window, cx);
            applied(input, replace("界", EditorSelectionPolicy::End), window, cx);
            let before = input.snapshot(window, cx);
            assert_eq!(
                input.command(&replace("", EditorSelectionPolicy::End), window, cx),
                EditorResult::Failed(EditorError::InvalidText)
            );
            assert_eq!(input.snapshot(window, cx), before);
            state(input).update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(Some(0..1), "jie", Some(0..3), window, cx);
                assert_eq!(state.value().as_str(), "jie");
                assert!(state.bridge_composition().is_some());
                state.replace_text_in_range(None, "界界", window, cx);
                assert_eq!(state.value().as_str(), "界界");
                assert!(state.bridge_composition().is_none());
            });
            assert_eq!(
                applied(input, EditorCommand::Undo, window, cx)
                    .text
                    .as_str(),
                "界"
            );
            state(input).update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(Some(0..1), "bad", Some(0..3), window, cx);
                state.unmark_text(window, cx);
                assert_eq!(state.value().as_str(), "界");
                assert!(state.bridge_composition().is_none());
            });
        },
    );
}
fn replace(text: &str, selection: EditorSelectionPolicy) -> EditorCommand {
    EditorCommand::Replace(text.into(), selection, EditorUndoPolicy::Record, None)
}
fn applied(
    input: &mut Instance,
    command: EditorCommand,
    window: &mut Window,
    cx: &mut App,
) -> EditorSnapshot {
    match input.command(&command, window, cx) {
        EditorResult::Applied(snapshot) => snapshot,
        other => panic!("unexpected command result: {other:?}"),
    }
}

#[test]
fn input_format_updates_retain_draft_identity_selection_and_undo() {
    with_input("abcd", None, |input, window, cx| {
        applied(
            input,
            EditorCommand::Select(EditorSelection { anchor: 4, head: 0 }),
            window,
            cx,
        );
        let before = input.snapshot(window, cx);
        let focus = input.focus_handle(cx);
        let entity = state(input).entity_id();
        set_format(input, Some(Config::Pattern("99".into())), window, cx);
        assert_eq!(input.snapshot(window, cx), before);
        assert_eq!(input.focus_handle(cx), focus);
        assert_eq!(state(input).entity_id(), entity);
        state(input).update(cx, |state, cx| {
            state.replace_text_in_range(Some(0..4), "abc", window, cx);
            assert_eq!(state.value().as_str(), "abc");
            state.replace_text_in_range(Some(0..3), "12", window, cx);
            assert_eq!(state.value().as_str(), "12");
        });
        let before = input.snapshot(window, cx);
        assert_eq!(
            input.command(&replace("x", EditorSelectionPolicy::End), window, cx),
            EditorResult::Failed(EditorError::InvalidText)
        );
        assert_eq!(input.snapshot(window, cx), before);
        assert_eq!(
            applied(input, EditorCommand::Undo, window, cx)
                .text
                .as_str(),
            "abc"
        );
        assert_eq!(
            applied(input, EditorCommand::Undo, window, cx)
                .text
                .as_str(),
            "abcd"
        );
        assert_eq!(
            applied(input, EditorCommand::Redo, window, cx)
                .text
                .as_str(),
            "abc"
        );
        let before = input.snapshot(window, cx);
        set_format(input, None, window, cx);
        assert_eq!(input.snapshot(window, cx), before);
        assert_eq!(
            applied(
                input,
                replace("free text", EditorSelectionPolicy::End),
                window,
                cx
            )
            .text
            .as_str(),
            "free text"
        );
    });
}

#[test]
fn input_format_exact_commands_reject_raw_values_before_selection_or_history_changes() {
    with_input(
        "",
        Some(Config::Pattern("*–99".into())),
        |input, window, cx| {
            let snapshot = applied(
                input,
                replace(
                    "界–12",
                    EditorSelectionPolicy::Select(EditorSelection { anchor: 8, head: 3 }),
                ),
                window,
                cx,
            );
            assert_eq!(snapshot.selection, EditorSelection { anchor: 8, head: 3 });
            for undo in [EditorUndoPolicy::Record, EditorUndoPolicy::Reset] {
                assert_eq!(
                    input.command(
                        &EditorCommand::Replace(
                            "界12".into(),
                            EditorSelectionPolicy::End,
                            undo,
                            Some(snapshot.revision)
                        ),
                        window,
                        cx
                    ),
                    EditorResult::Failed(EditorError::InvalidText)
                );
                assert_eq!(input.snapshot(window, cx), snapshot);
            }
            assert_eq!(
                input.command(
                    &EditorCommand::Replace(
                        "invalid".into(),
                        EditorSelectionPolicy::End,
                        EditorUndoPolicy::Reset,
                        Some(snapshot.revision - 1)
                    ),
                    window,
                    cx
                ),
                EditorResult::Failed(EditorError::StaleRevision)
            );
            assert_eq!(
                applied(input, EditorCommand::Undo, window, cx)
                    .text
                    .as_str(),
                ""
            );
            assert_eq!(
                applied(input, EditorCommand::Redo, window, cx)
                    .text
                    .as_str(),
                "界–12"
            );
        },
    );
}

#[test]
fn input_format_numeric_typing_preserves_middle_caret_and_rejects_fraction_overflow() {
    with_input(
        "1,234.50",
        Some(Config::Number(Number {
            separator: Some(",".into()),
            fraction_digits: Some(2),
        })),
        |input, window, cx| {
            state(input).update(cx, |state, cx| {
                state.replace_text_in_range(Some(3..3), "９", window, cx);
                assert_eq!(state.value().as_str(), "12,934.50");
                assert_eq!(state.bridge_selection(), (4, 4));
                let revision = state.bridge_revision();
                state.replace_text_in_range(Some(8..8), "1", window, cx);
                assert_eq!(state.value().as_str(), "12,934.50");
                assert_eq!(state.bridge_revision(), revision);
            });
            assert_eq!(
                applied(input, EditorCommand::Undo, window, cx)
                    .text
                    .as_str(),
                "1,234.50"
            );
        },
    );
}

#[test]
fn input_format_composition_accepts_provisional_text_and_commits_or_cancels_atomically() {
    with_input(
        "",
        Some(Config::Pattern("*".into())),
        |input, window, cx| {
            state(input).update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "jie", Some(0..3), window, cx);
                assert_eq!(state.value().as_str(), "jie");
                assert!(state.bridge_composition().is_some());
                state.replace_text_in_range(None, "界", window, cx);
                assert_eq!(state.value().as_str(), "界");
                assert!(state.bridge_composition().is_none());
            });
            assert_eq!(
                applied(input, EditorCommand::Undo, window, cx)
                    .text
                    .as_str(),
                ""
            );
            assert_eq!(
                applied(input, EditorCommand::Redo, window, cx)
                    .text
                    .as_str(),
                "界"
            );
            applied(
                input,
                EditorCommand::Select(EditorSelection { anchor: 3, head: 0 }),
                window,
                cx,
            );
            state(input).update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "two", Some(0..3), window, cx);
                state.unmark_text(window, cx);
                assert_eq!(state.value().as_str(), "界");
                assert_eq!(state.bridge_selection(), (3, 0));
                assert!(state.bridge_composition().is_none());
            });
            // Rejected commit must not add a no-op or clear the prior undo history.
            assert_eq!(
                applied(input, EditorCommand::Undo, window, cx)
                    .text
                    .as_str(),
                ""
            );
            assert_eq!(
                applied(input, EditorCommand::Redo, window, cx)
                    .text
                    .as_str(),
                "界"
            );
            applied(
                input,
                EditorCommand::Select(EditorSelection { anchor: 0, head: 3 }),
                window,
                cx,
            );
            state(input).update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "another", None, window, cx);
                state.replace_and_mark_text_in_range(None, "", None, window, cx);
                assert_eq!(state.value().as_str(), "界");
                assert_eq!(state.bridge_selection(), (0, 3));
                assert!(state.bridge_composition().is_none());
            });
        },
    );
}

#[test]
fn input_format_changes_during_composition_retain_the_mark_and_use_current_policy_on_commit() {
    with_input(
        "",
        Some(Config::Pattern("*".into())),
        |input, window, cx| {
            state(input).update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "jie", Some(0..3), window, cx);
            });
            let before = input.snapshot(window, cx);
            set_format(input, Some(Config::Pattern("99".into())), window, cx);
            assert_eq!(input.snapshot(window, cx), before);
            assert_eq!(
                input.command(&replace("12", EditorSelectionPolicy::End), window, cx),
                EditorResult::Failed(EditorError::Composing)
            );
            state(input).update(cx, |state, cx| {
                state.replace_text_in_range(None, "12", window, cx);
                assert_eq!(state.value().as_str(), "12");
                assert!(state.bridge_composition().is_none());
            });
            assert_eq!(
                applied(input, EditorCommand::Undo, window, cx)
                    .text
                    .as_str(),
                ""
            );
            state(input).update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "marked", None, window, cx);
                state.set_readonly(true, cx);
                state.unmark_text(window, cx);
                assert_eq!(state.value().as_str(), "");
                assert!(state.bridge_composition().is_none());
                state.replace_text_in_range(None, "12", window, cx);
                assert_eq!(state.value().as_str(), "");
                state.set_readonly(false, cx);
                state.replace_and_mark_text_in_range(None, "12", None, window, cx);
            });
            let before = input.snapshot(window, cx);
            set_format(input, None, window, cx);
            assert_eq!(input.snapshot(window, cx), before);
            state(input).update(cx, |state, cx| {
                state.replace_text_in_range(None, "unrestricted", window, cx);
                assert_eq!(state.value().as_str(), "unrestricted");
                assert!(state.bridge_composition().is_none());
            });
        },
    );
}

#[test]
fn input_format_first_enabled_during_composition_can_reject_commit_without_stranding_mark() {
    with_input("", None, |input, window, cx| {
        state(input).update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "12", Some(0..2), window, cx);
        });
        let before = input.snapshot(window, cx);
        set_format(input, Some(Config::Pattern("99".into())), window, cx);
        assert_eq!(input.snapshot(window, cx), before);
        state(input).update(cx, |state, cx| {
            state.replace_text_in_range(None, "x", window, cx);
            assert_eq!(state.value().as_str(), "");
            assert!(state.bridge_composition().is_none());
        });
        let snapshot = applied(input, replace("34", EditorSelectionPolicy::End), window, cx);
        assert_eq!(snapshot.text, "34");
        assert_eq!(applied(input, EditorCommand::Undo, window, cx).text, "");
    });
}
