use super::viewport_tests::{apply as update, request, setup, state};
use super::*;
use gpuio_protocol::editor_search::{Case, Command, Mode, Snapshot};

fn node() -> NodeId {
    NodeId::from_parts(1, 1).unwrap()
}

#[test]
fn search_opt_in_admission_is_multiline_only_and_atomic() {
    use crate::tree::Tree;
    let id = NodeId::from_parts(0, 1).unwrap();
    for kind in [Kind::Input, Kind::Textarea, Kind::Container] {
        let window = WindowId::from_parts(0, 1).unwrap();
        let mut tree = Tree::new(window);
        let mut ops = vec![
            Op::Create(
                id,
                kind,
                "seed".into(),
                (kind != Kind::Container)
                    .then(|| gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetRoot(Some(id)),
        ];
        if kind != Kind::Container {
            ops.push(Op::SetEditor(
                id,
                EditorConfig {
                    label: "Draft".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ));
        }
        tree.apply(&Transaction {
            window,
            base: 0,
            revision: 1,
            operations: ops,
        })
        .unwrap();
        let before = tree.get(id).unwrap().clone();
        let result = tree.apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations: vec![
                Op::SetStyle(id, vec![Style::Width(Length::Px(42.))]),
                Op::SetEditorSearchable(id, true),
            ],
        });
        if kind == Kind::Textarea {
            result.unwrap();
            assert!(tree.get(id).unwrap().editor_searchable);
        } else {
            assert_eq!(result.err(), Some(ErrorCode::InvalidTree));
            assert_eq!(tree.revision(), 1);
            assert_eq!(tree.get(id).unwrap(), &before);
            assert!(!tree.get(id).unwrap().editor_searchable);
        }
    }
}
fn observed(result: EditorResult) -> Snapshot {
    let EditorResult::SearchObserved(value) = result else {
        panic!("unexpected {result:?}");
    };
    assert!(value.is_valid());
    value
}

#[test]
fn checked_search_replacement_rejects_stale_navigation_and_text() {
    setup("é é é", |owner, cx| {
        let search = |cx: &mut gpui::VisualTestContext, command| {
            request(&owner, cx, EditorCommand::Search(command))
        };
        assert_eq!(
            search(cx, Command::Read),
            EditorResult::Failed(EditorError::SearchUnavailable)
        );
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), true)]);
        assert_eq!(observed(search(cx, Command::Open(false))).mode, Mode::Find);
        let before = observed(search(cx, Command::SetQuery("é".into(), Case::Sensitive)));
        assert_eq!(before.match_count, 3);
        let moved = observed(search(cx, Command::Next));
        assert_eq!(moved.current.unwrap().index, 1);
        assert_eq!(
            search(cx, Command::ReplaceCurrent(before.stamp, "界".into())),
            EditorResult::Failed(EditorError::StaleSearch)
        );
        let result = search(cx, Command::ReplaceCurrent(moved.stamp, "界".into()));
        let EditorResult::SearchReplaced(editor, after, 1) = result else {
            panic!("unexpected {result:?}");
        };
        assert_eq!(editor.text, "é 界 é");
        assert_eq!(after.stamp.editor_revision, editor.revision);
        assert_eq!(
            search(cx, Command::ReplaceAll(moved.stamp, "x".into())),
            EditorResult::Failed(EditorError::StaleRevision)
        );
        let EditorResult::Applied(editor) = request(&owner, cx, EditorCommand::Undo) else {
            panic!();
        };
        assert_eq!(editor.text, "é é é");
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), false)]);
        assert_eq!(
            search(cx, Command::Read),
            EditorResult::Failed(EditorError::SearchUnavailable)
        );
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), true)]);
        let resumed = observed(search(cx, Command::Read));
        assert_eq!(resumed.mode, Mode::Closed);
        assert_eq!(resumed.query, "é");
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                let base = view.session.borrow().tree(view.id).unwrap().revision();
                view.session
                    .borrow_mut()
                    .apply(&Transaction {
                        window: view.id,
                        base,
                        revision: base + 1,
                        operations: vec![
                            Op::Splice(NodeId::from_parts(0, 1).unwrap(), 0, 1, vec![]),
                            Op::Remove(node()),
                        ],
                    })
                    .unwrap();
                let editor = view.editors.get_mut(&node()).unwrap();
                assert_eq!(
                    editor.command(&EditorCommand::Search(Command::Read), window, cx),
                    EditorResult::Failed(EditorError::StaleEditor)
                );
                assert_eq!(
                    editor.command(
                        &EditorCommand::Search(Command::ReplaceAll(resumed.stamp, "x".into())),
                        window,
                        cx
                    ),
                    EditorResult::Failed(EditorError::StaleEditor)
                );
            })
        });
    });
}

#[test]
fn search_bridge_enforces_composition_readonly_limits_and_no_match_acknowledgement() {
    setup("a a a", |owner, cx| {
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), true)]);
        let search = |cx: &mut gpui::VisualTestContext, command| {
            request(&owner, cx, EditorCommand::Search(command))
        };
        observed(search(cx, Command::Open(true)));
        let current = observed(search(cx, Command::SetQuery("a".into(), Case::Sensitive)));
        assert_eq!(
            search(
                cx,
                Command::ReplaceAll(current.stamp, "x".repeat(MAX_TEXT_BYTES))
            ),
            EditorResult::Failed(EditorError::LimitExceeded)
        );
        assert_eq!(observed(search(cx, Command::Read)).stamp, current.stamp);
        let field = state(&owner, cx);
        cx.update(|window, cx| {
            field.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(Some(0..1), "a", Some(0..1), window, cx)
            })
        });
        let composing = observed(search(cx, Command::Read));
        assert!(!composing.can_replace);
        assert_eq!(
            search(cx, Command::Open(false)),
            EditorResult::Failed(EditorError::Composing)
        );
        assert_eq!(
            observed(search(cx, Command::Read)).activation_revision,
            composing.activation_revision
        );
        assert_eq!(
            search(cx, Command::ReplaceAll(composing.stamp, "x".into())),
            EditorResult::Failed(EditorError::Composing)
        );
        cx.update(|window, cx| field.update(cx, |state, cx| state.unmark_text(window, cx)));
        let mut config = cx.update(|_, cx| owner.read(cx).editors[&node()].config.clone());
        config.read_only = true;
        update(&owner, cx, vec![Op::SetEditor(node(), config.clone())]);
        let current = observed(search(cx, Command::Read));
        assert_eq!(
            search(cx, Command::ReplaceAll(current.stamp, "x".into())),
            EditorResult::Failed(EditorError::NotEditable)
        );
        config.read_only = false;
        config.disabled = true;
        update(&owner, cx, vec![Op::SetEditor(node(), config.clone())]);
        assert_eq!(
            search(cx, Command::Next),
            EditorResult::Failed(EditorError::NotEditable)
        );
        observed(search(cx, Command::Read));
        config.disabled = false;
        update(&owner, cx, vec![Op::SetEditor(node(), config)]);
        let empty = observed(search(
            cx,
            Command::SetQuery("absent".into(), Case::Sensitive),
        ));
        let EditorResult::SearchReplaced(editor, after, count) =
            search(cx, Command::ReplaceAll(empty.stamp, "x".into()))
        else {
            panic!();
        };
        assert_eq!(count, 0);
        assert_eq!(after.stamp, empty.stamp);
        assert_eq!(editor.text, "a a a");
    });
}

#[test]
fn native_search_changes_publish_once_and_stop_after_removal() {
    setup("one two one", |owner, cx| {
        let transport = cx.update(|_, cx| owner.read(cx).transport.clone());
        let drain = || transport.mailbox.lock().unwrap().drain(256);
        assert!(
            !drain()
                .iter()
                .any(|e| matches!(e, Event::EditorSearchObserved(..)))
        );
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), true)]);
        let initial = drain();
        assert_eq!(initial.iter().filter(|e| matches!(e, Event::EditorSearchObserved(_, _, _, _, s) if s.mode == Mode::Closed)).count(), 1);
        let entity = state(&owner, cx);
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-f"
        } else {
            "ctrl-f"
        });
        cx.run_until_parked();
        assert!(drain().iter().any(|e| matches!(e,
            Event::EditorSearchObserved(_, _, _, _, s) if s.mode == Mode::Find && s.activation_revision == 1)));
        cx.update(|_, cx| entity.update(cx, |state, cx| state.set_search_query("one", false, cx)));
        cx.run_until_parked();
        let values = drain();
        let latest = values
            .iter()
            .filter_map(|e| match e {
                Event::EditorSearchObserved(_, _, _, _, value) => Some(value),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].match_count, 2);
        assert_eq!(latest[0].mode, Mode::Find);
        assert!(latest[0].can_replace);
        cx.update(|_, cx| entity.update(cx, |_, cx| cx.notify()));
        cx.run_until_parked();
        assert!(
            drain().is_empty(),
            "unchanged layout/blink must not emit observations"
        );
        cx.update(|_, cx| {
            entity.update(cx, |state, cx| {
                state.next_search_match(cx);
            })
        });
        cx.run_until_parked();
        assert!(drain().iter().any(|e| matches!(e,
            Event::EditorSearchObserved(_, _, _, _, s) if s.current.unwrap().index == 1)));
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), false)]);
        let values = drain();
        assert_eq!(
            values
                .iter()
                .filter(|e| matches!(e,
            Event::EditorSearchObserved(_, _, _, _, s) if s.mode == Mode::Closed && !s.can_replace))
                .count(),
            1
        );
        update(
            &owner,
            cx,
            vec![
                Op::Splice(NodeId::from_parts(0, 1).unwrap(), 0, 1, vec![]),
                Op::Remove(node()),
            ],
        );
        drain();
        cx.update(|_, cx| {
            entity.update(cx, |state, cx| {
                state.set_searchable(true, cx);
                state.open_search(false, cx);
            })
        });
        cx.run_until_parked();
        assert!(drain().is_empty(), "retired entity must not publish");
    });
}

#[test]
fn close_and_focus_is_atomic_and_cannot_close_a_later_opening() {
    setup("one two one", |owner, cx| {
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), true)]);
        let search = |cx: &mut gpui::VisualTestContext, command| {
            request(&owner, cx, EditorCommand::Search(command))
        };
        let first = observed(search(cx, Command::Open(false)));
        observed(search(cx, Command::SetQuery("one".into(), Case::Sensitive)));
        let second = observed(search(cx, Command::Open(false)));
        assert!(second.activation_revision > first.activation_revision);
        cx.update(|window, cx| window.blur(cx));
        let before = request(&owner, cx, EditorCommand::ReadSnapshot);
        assert_eq!(
            search(cx, Command::CloseAndFocus(first.activation_revision)),
            EditorResult::Failed(EditorError::StaleSearch)
        );
        assert_eq!(observed(search(cx, Command::Read)).mode, Mode::Find);
        assert_eq!(request(&owner, cx, EditorCommand::ReadSnapshot), before);
        // Query/navigation updates within this opening must not make Close unusable.
        observed(search(cx, Command::Next));
        let before = request(&owner, cx, EditorCommand::ReadSnapshot);
        let closed = observed(search(
            cx,
            Command::CloseAndFocus(second.activation_revision),
        ));
        assert_eq!(closed.mode, Mode::Closed);
        assert_eq!(closed.activation_revision, second.activation_revision);
        let EditorResult::Applied(before) = before else {
            panic!()
        };
        let EditorResult::Applied(after) = request(&owner, cx, EditorCommand::ReadSnapshot) else {
            panic!()
        };
        assert!(!before.focused && after.focused);
        assert_eq!(after.text, before.text);
        assert_eq!(after.selection, before.selection);
        assert_eq!(after.revision, before.revision);
        cx.update(|window, cx| window.blur(cx));
        assert_eq!(
            search(cx, Command::CloseAndFocus(second.activation_revision)),
            EditorResult::Failed(EditorError::SearchUnavailable)
        );
        let EditorResult::Applied(after) = request(&owner, cx, EditorCommand::ReadSnapshot) else {
            panic!()
        };
        assert!(!after.focused, "repeated close cannot steal focus");
    });
}

#[test]
fn query_and_case_updates_do_not_echo_stale_other_fields() {
    setup("ONE one", |owner, cx| {
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), true)]);
        let search = |cx: &mut gpui::VisualTestContext, command| {
            request(&owner, cx, EditorCommand::Search(command))
        };
        observed(search(cx, Command::Open(false)));
        observed(search(cx, Command::SetCase(Case::Sensitive)));
        let typed = observed(search(cx, Command::SetQueryText("one".into())));
        assert_eq!(typed.case, Case::Sensitive);
        assert_eq!(typed.match_count, 1);
        let folded = observed(search(cx, Command::SetCase(Case::AsciiInsensitive)));
        assert_eq!(folded.query, "one");
        assert_eq!(folded.match_count, 2);
        let first = observed(search(cx, Command::ToggleCase));
        assert_eq!(first.case, Case::Sensitive);
        let second = observed(search(cx, Command::ToggleCase));
        assert_eq!(second.case, Case::AsciiInsensitive);
        assert!(second.stamp.search_revision > first.stamp.search_revision);
        let typed = observed(search(cx, Command::SetQueryText("ONE".into())));
        assert_eq!(typed.case, Case::AsciiInsensitive);
        assert_eq!(typed.match_count, 2);
        assert!(typed.stamp.search_revision > folded.stamp.search_revision);
        assert_eq!(
            observed(search(cx, Command::SetQueryText("ONE".into()))).stamp,
            typed.stamp
        );
    });
}

#[test]
fn close_and_focus_preserves_composition_and_skips_unavailable_focus() {
    setup("one two one", |owner, cx| {
        update(&owner, cx, vec![Op::SetEditorSearchable(node(), true)]);
        let search = |cx: &mut gpui::VisualTestContext, command| {
            request(&owner, cx, EditorCommand::Search(command))
        };
        let opened = observed(search(cx, Command::Open(false)));
        let entity = state(&owner, cx);
        cx.update(|window, cx| {
            entity.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(Some(0..1), "o", Some(0..1), window, cx);
            })
        });
        let before = request(&owner, cx, EditorCommand::ReadSnapshot);
        assert_eq!(
            search(cx, Command::CloseAndFocus(opened.activation_revision)),
            EditorResult::Failed(EditorError::Composing)
        );
        assert_eq!(request(&owner, cx, EditorCommand::ReadSnapshot), before);
        assert_eq!(observed(search(cx, Command::Read)).mode, Mode::Find);
        cx.update(|window, cx| entity.update(cx, |state, cx| state.unmark_text(window, cx)));
        update(
            &owner,
            cx,
            vec![Op::SetStyle(
                node(),
                vec![Style::Fields(vec![Field::Display(3)])],
            )],
        );
        assert_eq!(
            observed(search(
                cx,
                Command::CloseAndFocus(opened.activation_revision)
            ))
            .mode,
            Mode::Closed
        );
        update(
            &owner,
            cx,
            vec![Op::SetStyle(
                node(),
                vec![
                    Style::Width(Length::Px(240.)),
                    Style::Height(Length::Px(200.)),
                ],
            )],
        );
        let opened = observed(search(cx, Command::Open(false)));
        let mut config = cx.update(|_, cx| owner.read(cx).editors[&node()].config.clone());
        config.disabled = true;
        update(&owner, cx, vec![Op::SetEditor(node(), config.clone())]);
        assert_eq!(
            observed(search(
                cx,
                Command::CloseAndFocus(opened.activation_revision)
            ))
            .mode,
            Mode::Closed
        );
        let EditorResult::Applied(disabled) = request(&owner, cx, EditorCommand::ReadSnapshot)
        else {
            panic!()
        };
        assert!(!disabled.focused);
        config.disabled = false;
        config.read_only = true;
        update(&owner, cx, vec![Op::SetEditor(node(), config)]);
        let opened = observed(search(cx, Command::Open(false)));
        assert_eq!(
            observed(search(
                cx,
                Command::CloseAndFocus(opened.activation_revision)
            ))
            .mode,
            Mode::Closed
        );
        let EditorResult::Applied(value) = request(&owner, cx, EditorCommand::ReadSnapshot) else {
            panic!()
        };
        assert!(value.focused);
    });
}
