use super::*;
use std::cell::Cell;

fn config() -> Arc<Config> {
    Arc::new(Config {
        policy: Policy::new(6, Alphabet::Digits).unwrap(),
        label: "Code".into(),
        masked: false,
        disabled: false,
        read_only: false,
        auto_focus: false,
    })
}
fn state(value: &str) -> State {
    State::new(config(), value).unwrap()
}
fn command(s: &mut State, command: Command) -> Outcome {
    s.execute(&command, || panic!("unexpected focus side effect"))
}
fn replace(value: &str, undo: UndoPolicy, guard: Option<i64>) -> Command {
    Command::Replace {
        value: value.into(),
        selection: SelectionPolicy::End,
        undo,
        if_revision: guard,
    }
}
fn commit(s: &mut State, text: &str) -> Vec<Event> {
    s.native(
        Action::Commit {
            range_utf16: None,
            text,
        },
        Access::Allowed,
    )
    .unwrap()
}
fn select(s: &mut State, anchor: i64, head: i64) {
    s.native(Action::Select(Selection { anchor, head }), Access::Allowed)
        .unwrap();
}
fn mark(s: &mut State, text: &str) -> Vec<Event> {
    s.native(
        Action::Mark {
            range_utf16: None,
            text,
            selected_utf16: None,
        },
        Access::Allowed,
    )
    .unwrap()
}
fn failed(outcome: Outcome, error: Error) {
    assert!(outcome.events.is_empty());
    assert_eq!(outcome.response, Response::Failed(error));
}
fn only_observed(outcome: Outcome) -> Snapshot {
    assert_eq!(outcome.events.len(), 1);
    let Event::Observed(snapshot) = &outcome.events[0] else {
        panic!("expected observation");
    };
    assert!(outcome.events[0].is_valid());
    assert_eq!(outcome.response, Response::Applied(snapshot.clone()));
    snapshot.clone()
}

#[test]
fn initial_values_and_command_replacements_never_complete() {
    let mut s = state("123456");
    assert_eq!(s.revision(), 0);
    assert!(s.snapshot().is_complete());
    assert!(Event::Observed(s.snapshot()).is_valid());
    assert!(!s.editor().can_undo());
    assert!(State::new(config(), "ABC").is_err());
    assert!(
        State::new(
            Arc::new(Config {
                label: "".into(),
                ..(*config()).clone()
            }),
            ""
        )
        .is_err()
    );
    let same = command(&mut s, replace("123456", UndoPolicy::Record, Some(0)));
    assert!(same.events.is_empty());
    assert_eq!(s.revision(), 0);
    let full = only_observed(command(
        &mut s,
        replace("654321", UndoPolicy::Record, Some(0)),
    ));
    assert!(full.is_complete());
    assert_eq!(full.revision, 1);
    let empty = only_observed(command(
        &mut s,
        Command::Clear {
            undo: UndoPolicy::Record,
            if_revision: Some(1),
        },
    ));
    assert!(empty.value.is_empty());
    assert_eq!(empty.selection, Selection { anchor: 0, head: 0 });
    let undo = only_observed(command(&mut s, Command::Undo));
    assert_eq!(undo.value, "654321");
    assert!(undo.is_complete() && undo.can_redo);
    let redo = only_observed(command(&mut s, Command::Redo));
    assert!(redo.value.is_empty());
    let reset = only_observed(command(
        &mut s,
        Command::Clear {
            undo: UndoPolicy::Reset,
            if_revision: None,
        },
    ));
    assert!(!reset.can_undo && !reset.can_redo);
}

#[test]
fn native_completion_is_an_ordered_pair_with_distinct_revisions() {
    let mut s = state("1234");
    let events = commit(&mut s, "５６");
    assert_eq!(events.len(), 2);
    let (Event::Changed(changed), Event::Complete(complete)) = (&events[0], &events[1]) else {
        panic!("missing completion pair");
    };
    assert_eq!(changed.revision, 1);
    assert_eq!(complete.revision, 2);
    assert_eq!(changed.value, "123456");
    let mut expected = changed.clone();
    expected.revision = 2;
    assert_eq!(*complete, expected);
    assert!(events.iter().all(Event::is_valid));
    assert_eq!(s.snapshot(), *complete);
    select(&mut s, 6, 4);
    let same = commit(&mut s, "56");
    assert!(matches!(&same[..], [Event::Changed(_)])); // selection collapse only
    select(&mut s, 6, 4);
    assert!(matches!(
        &commit(&mut s, "９８")[..],
        [Event::Changed(_), Event::Complete(_)]
    ));
    let undo = s.native(Action::Undo, Access::Allowed).unwrap();
    assert!(matches!(&undo[..], [Event::Changed(_), Event::Complete(_)]));
    assert_eq!(s.editor().value(), "123456");
    let redo = s.native(Action::Redo, Access::Allowed).unwrap();
    assert!(matches!(&redo[..], [Event::Changed(_), Event::Complete(_)]));
    assert_eq!(s.editor().value(), "123498");
}

#[test]
fn alphanumeric_paste_preserves_case_and_empty_paste_preserves_selection() {
    let config = Arc::new(Config {
        policy: Policy::new(4, Alphabet::AsciiAlphanumeric).unwrap(),
        ..(*config()).clone()
    });
    let mut s = State::new(config, "").unwrap();
    let events = s
        .native(Action::Paste("Ａb-１ ２"), Access::Allowed)
        .unwrap();
    assert!(
        matches!(&events[..], [Event::Changed(_), Event::Complete(snapshot)] if snapshot.value == "Ab12")
    );
    select(&mut s, 4, 2);
    let before = s.snapshot();
    assert!(
        s.native(Action::Paste(" - \t"), Access::Allowed)
            .unwrap()
            .is_empty()
    );
    assert_eq!(s.snapshot(), before);
    let rejected = s.native(Action::Paste("3🙂"), Access::Allowed).unwrap();
    assert!(
        matches!(&rejected[..], [Event::Rejected(InputError::UnexpectedCharacter { byte_offset: 1 }, snapshot)] if snapshot.value == "Ab12" && snapshot.selection == before.selection)
    );
    let removed = s
        .native(Action::Delete(edit::Delete::Backward), Access::Allowed)
        .unwrap();
    assert!(matches!(&removed[..], [Event::Changed(snapshot)] if snapshot.value == "Ab"));
    s.native(
        Action::Move {
            movement: edit::Movement::Start,
            extend: true,
        },
        Access::Allowed,
    )
    .unwrap();
    assert_eq!(
        s.editor().selection(),
        edit::Selection { anchor: 2, head: 0 }
    );
    mark(&mut s, "かな");
    let cancelled = s
        .native(Action::CancelComposition, Access::Allowed)
        .unwrap();
    assert!(matches!(&cancelled[..], [Event::Changed(snapshot)] if snapshot.value == "Ab"));
    assert_eq!(
        s.editor().selection(),
        edit::Selection { anchor: 2, head: 0 }
    );
}

#[test]
fn rejected_input_is_a_boundary_and_ime_rejection_restores_checkpoint() {
    let mut s = state("123456");
    let rejected = commit(&mut s, "7");
    assert!(
        matches!(&rejected[..], [Event::Rejected(InputError::TooLong, snapshot)] if snapshot.revision == 1)
    );
    assert!(!s.editor().can_undo());
    select(&mut s, 4, 2);
    let before = s.snapshot();
    let events = mark(&mut s, "かな");
    assert!(matches!(&events[..], [Event::Changed(snapshot)] if snapshot.composition.is_some()));
    assert_eq!(s.editor().value(), before.value);
    let rejected = commit(&mut s, "漢字");
    assert!(
        matches!(&rejected[..], [Event::Rejected(InputError::UnexpectedCharacter { byte_offset: 2 }, snapshot)] if snapshot.composition.is_none())
    );
    assert_eq!(
        s.editor().selection(),
        edit::Selection { anchor: 4, head: 2 }
    );
    assert_eq!(s.editor().value(), "123456");
    assert!(!s.editor().can_undo());
    assert!(rejected[0].is_valid());
    mark(&mut s, "９８");
    let events = s.unmark(Access::Allowed).unwrap();
    assert!(matches!(
        &events[..],
        [Event::Changed(_), Event::Complete(_)]
    ));
    assert_eq!(s.editor().value(), "129856");
    s.native(Action::Undo, Access::Allowed).unwrap();
    assert_eq!(
        s.editor().selection(),
        edit::Selection { anchor: 4, head: 2 }
    );
}

#[test]
fn configurations_retain_preedit_and_history_and_policy_changes_reject() {
    let mut s = state("12");
    commit(&mut s, "34");
    select(&mut s, 4, 2);
    mark(&mut s, "に🙂");
    let before = s.snapshot();
    let history = s.editor().history_bytes();
    let next = Arc::new(Config {
        label: "Verification code".into(),
        masked: true,
        read_only: true,
        disabled: true,
        auto_focus: true,
        ..(*config()).clone()
    });
    let events = s.configure(next.clone()).unwrap();
    assert!(matches!(&events[..], [Event::Observed(_)]));
    let mut expected = before.clone();
    expected.revision += 1;
    assert_eq!(s.snapshot(), expected);
    assert_eq!(s.editor().history_bytes(), history);
    assert!(s.configure(next.clone()).unwrap().is_empty());
    let invalid = Arc::new(Config {
        policy: Policy::new(5, Alphabet::Digits).unwrap(),
        ..(*next).clone()
    });
    assert_eq!(s.configure(invalid), Err(Error::InvalidConfig));
    let invalid = Arc::new(Config {
        policy: Policy::new(6, Alphabet::AsciiAlphanumeric).unwrap(),
        ..(*next).clone()
    });
    assert_eq!(s.configure(invalid), Err(Error::InvalidConfig));
    assert_eq!(s.config(), next.as_ref());
    assert_eq!(s.snapshot(), expected);
    let cancelled = s.unmark(Access::Allowed).unwrap();
    assert!(
        matches!(&cancelled[..], [Event::Changed(snapshot)] if snapshot.value == "1234" && snapshot.composition.is_none())
    );
    assert_eq!(
        s.editor().selection(),
        edit::Selection { anchor: 4, head: 2 }
    );
    assert_eq!(s.editor().history_bytes(), history);
}

#[test]
fn content_commands_guard_composition_and_native_cleanup_is_always_available() {
    let mut s = state("12");
    mark(&mut s, "３");
    let before = s.snapshot();
    for c in [
        replace("45", UndoPolicy::Reset, None),
        Command::Clear {
            undo: UndoPolicy::Reset,
            if_revision: None,
        },
        Command::Select(Selection { anchor: 0, head: 0 }),
        Command::Undo,
        Command::Redo,
    ] {
        failed(command(&mut s, c), Error::Composing);
        assert_eq!(s.snapshot(), before);
    }
    let cancelled = only_observed(command(&mut s, Command::CancelComposition));
    assert_eq!(cancelled.value, "12");
    assert!(cancelled.composition.is_none());
    mark(&mut s, "３");
    let denied = s.native(Action::Paste("4"), Access::Allowed);
    assert_eq!(denied, Err(NativeError::Denied(Error::Composing)));
    s.configure(Arc::new(Config {
        disabled: true,
        ..(*config()).clone()
    }))
    .unwrap();
    let cancelled = s
        .native(
            Action::Mark {
                range_utf16: None,
                text: "",
                selected_utf16: None,
            },
            Access::Blocked,
        )
        .unwrap();
    assert!(matches!(&cancelled[..], [Event::Changed(snapshot)] if snapshot.composition.is_none()));
    assert_eq!(s.editor().value(), "12");
    assert!(s.cancel_for_lifecycle().unwrap().is_empty());
    assert!(s.unmark(Access::Blocked).unwrap().is_empty());
}

#[test]
fn malformed_platform_edits_leave_preedit_and_revisions_intact() {
    let mut s = state("12");
    mark(&mut s, "🙂");
    let before = s.snapshot();
    assert_eq!(
        s.native(
            Action::Commit {
                range_utf16: Some(3..4),
                text: "3"
            },
            Access::Allowed
        ),
        Err(NativeError::Input(InputError::InvalidSelection))
    );
    assert_eq!(
        s.native(
            Action::Mark {
                range_utf16: None,
                text: "🙂",
                selected_utf16: Some(1..2)
            },
            Access::Allowed
        ),
        Err(NativeError::Input(InputError::InvalidSelection))
    );
    assert_eq!(
        s.native(
            Action::Mark {
                range_utf16: None,
                text: &"a".repeat(4097),
                selected_utf16: None
            },
            Access::Allowed
        ),
        Err(NativeError::Input(InputError::InputTooLarge))
    );
    assert_eq!(
        s.native(
            Action::Move {
                movement: edit::Movement::Left,
                extend: false
            },
            Access::Allowed
        ),
        Err(NativeError::Denied(Error::Composing))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(
        s.native(
            Action::Select(Selection {
                anchor: -1,
                head: 0
            }),
            Access::Allowed
        ),
        Err(NativeError::Input(InputError::InvalidSelection))
    );
    assert_eq!(s.snapshot(), before);
}

#[test]
fn replacement_selection_guards_and_validation_are_atomic() {
    let mut s = state("123456");
    select(&mut s, 6, 2);
    let before = s.snapshot();
    failed(
        command(
            &mut s,
            replace("4", UndoPolicy::Reset, Some(before.revision - 1)),
        ),
        Error::StaleRevision,
    );
    failed(
        command(&mut s, replace("4", UndoPolicy::Reset, Some(-1))),
        Error::StaleRevision,
    );
    failed(
        command(&mut s, replace("ABC", UndoPolicy::Reset, None)),
        Error::InvalidValue,
    );
    failed(
        command(
            &mut s,
            Command::Replace {
                value: "1".into(),
                selection: SelectionPolicy::Select(Selection { anchor: 2, head: 0 }),
                undo: UndoPolicy::Reset,
                if_revision: None,
            },
        ),
        Error::InvalidSelection,
    );
    failed(
        command(
            &mut s,
            Command::Select(Selection {
                anchor: 4097,
                head: 0,
            }),
        ),
        Error::InvalidSelection,
    );
    assert_eq!(s.snapshot(), before);
    let preserved = only_observed(command(
        &mut s,
        Command::Replace {
            value: "789".into(),
            selection: SelectionPolicy::Preserve,
            undo: UndoPolicy::Record,
            if_revision: Some(before.revision),
        },
    ));
    assert_eq!(preserved.selection, Selection { anchor: 3, head: 2 });
    let start = only_observed(command(
        &mut s,
        Command::Replace {
            value: "789".into(),
            selection: SelectionPolicy::Start,
            undo: UndoPolicy::Record,
            if_revision: None,
        },
    ));
    assert_eq!(start.selection, Selection { anchor: 0, head: 0 });
    assert_eq!(s.editor().history_edits(), 1); // selection-only operation did not record
    let selected = only_observed(command(
        &mut s,
        Command::Replace {
            value: "789".into(),
            selection: SelectionPolicy::Select(Selection { anchor: 3, head: 1 }),
            undo: UndoPolicy::Reset,
            if_revision: None,
        },
    ));
    assert_eq!(selected.selection, Selection { anchor: 3, head: 1 });
    assert!(!selected.can_undo && !selected.can_redo);
}

#[test]
fn permissions_distinguish_read_only_selection_from_editing_and_programmatic_replace() {
    let mut s = state("123456");
    s.configure(Arc::new(Config {
        read_only: true,
        ..(*config()).clone()
    }))
    .unwrap();
    select(&mut s, 4, 2);
    assert_eq!(s.copy_selection(Access::Allowed), Some("34"));
    assert_eq!(
        s.native(Action::Paste("9"), Access::Allowed),
        Err(NativeError::Denied(Error::ReadOnly))
    );
    assert_eq!(
        s.native(Action::Delete(edit::Delete::Backward), Access::Allowed),
        Err(NativeError::Denied(Error::ReadOnly))
    );
    failed(command(&mut s, Command::Undo), Error::ReadOnly);
    assert_eq!(
        s.cut(Access::Allowed, |_| panic!("read-only cut")),
        Err(Error::ReadOnly)
    );
    only_observed(command(&mut s, replace("654321", UndoPolicy::Record, None)));
    s.configure(Arc::new(Config {
        disabled: true,
        ..(*config()).clone()
    }))
    .unwrap();
    assert_eq!(
        s.native(
            Action::Select(Selection { anchor: 2, head: 0 }),
            Access::Allowed
        ),
        Err(NativeError::Denied(Error::Disabled))
    );
    failed(
        s.execute(&Command::Focus, || panic!("disabled focus")),
        Error::FocusBlocked,
    );
    failed(command(&mut s, Command::Redo), Error::Disabled);
    only_observed(command(
        &mut s,
        Command::Clear {
            undo: UndoPolicy::Reset,
            if_revision: None,
        },
    ));
    s.configure(config()).unwrap();
    assert_eq!(
        s.native(Action::Paste("12"), Access::Blocked),
        Err(NativeError::Denied(Error::FocusBlocked))
    );
    assert_eq!(
        s.native(
            Action::Move {
                movement: edit::Movement::End,
                extend: false
            },
            Access::Blocked
        ),
        Err(NativeError::Denied(Error::FocusBlocked))
    );
    assert_eq!(s.copy_selection(Access::Blocked), None);
    mark(&mut s, "３");
    let cancel = s.unmark(Access::Blocked).unwrap();
    assert!(matches!(&cancel[..], [Event::Changed(snapshot)] if snapshot.value.is_empty()));
}

#[test]
fn focus_is_confirmed_by_adapter_and_observations_deduplicate() {
    let mut s = state("123456");
    let count = Cell::new(0);
    failed(
        s.execute(&Command::Focus, || {
            count.set(count.get() + 1);
            Err(Error::FocusBlocked)
        }),
        Error::FocusBlocked,
    );
    assert_eq!(s.revision(), 0);
    assert!(!s.focused());
    let focused = only_observed(s.execute(&Command::Focus, || {
        count.set(count.get() + 1);
        Ok(())
    }));
    assert!(focused.focused);
    let repeated = s.execute(&Command::Focus, || {
        count.set(count.get() + 1);
        Ok(())
    });
    assert!(repeated.events.is_empty());
    assert_eq!(count.get(), 3);
    assert_eq!(s.revision(), 1);
    assert!(s.observe_focus(true).unwrap().is_empty());
    assert!(
        matches!(&s.observe_focus(false).unwrap()[..], [Event::Changed(snapshot)] if !snapshot.focused)
    );
    assert_eq!(s.editor().value(), "123456");
    assert!(s.observe_focus(false).unwrap().is_empty());
}

#[test]
fn copy_cut_masks_and_failed_clipboard_writes_preserve_native_state() {
    let mut s = state("123456");
    assert_eq!(s.copy_selection(Access::Allowed), None);
    assert!(
        s.cut(Access::Allowed, |_| panic!("empty cut"))
            .unwrap()
            .is_empty()
    );
    select(&mut s, 4, 2);
    s.configure(Arc::new(Config {
        masked: true,
        ..(*config()).clone()
    }))
    .unwrap();
    assert_eq!(s.copy_selection(Access::Allowed), None);
    assert!(
        s.cut(Access::Allowed, |_| panic!("masked cut"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(s.editor().value(), "123456");
    s.configure(config()).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.cut(Access::Allowed, |text| {
            assert_eq!(text, "34");
            Err(Error::NativeFailure)
        }),
        Err(Error::NativeFailure)
    );
    assert_eq!(s.snapshot(), before);
    let calls = Cell::new(0);
    let events = s
        .cut(Access::Allowed, |text| {
            calls.set(calls.get() + 1);
            assert_eq!(text, "34");
            Ok(())
        })
        .unwrap();
    assert_eq!(calls.get(), 1);
    assert!(matches!(&events[..], [Event::Changed(snapshot)] if snapshot.value == "1256"));
    assert_eq!(s.editor().selection(), edit::Selection::caret(2));
    let undo = s.native(Action::Undo, Access::Allowed).unwrap();
    assert!(matches!(&undo[..], [Event::Changed(_), Event::Complete(_)]));
    assert_eq!(
        s.editor().selection(),
        edit::Selection { anchor: 4, head: 2 }
    );
}

#[test]
fn exhausted_revisions_cannot_partially_edit_complete_or_touch_native_callbacks() {
    let mut s = state("1234");
    s.revision = i64::MAX - 1;
    let before = s.snapshot();
    assert_eq!(
        s.native(
            Action::Commit {
                range_utf16: None,
                text: "56"
            },
            Access::Allowed
        ),
        Err(NativeError::Denied(Error::LimitExceeded))
    );
    assert_eq!(s.snapshot(), before);
    assert!(!s.editor().can_undo());
    s.revision = i64::MAX - 2;
    let events = commit(&mut s, "56");
    assert_eq!(events[0].snapshot().revision, i64::MAX - 1);
    assert_eq!(events[1].snapshot().revision, i64::MAX);
    let before = s.snapshot();
    failed(
        command(
            &mut s,
            Command::Clear {
                undo: UndoPolicy::Reset,
                if_revision: None,
            },
        ),
        Error::LimitExceeded,
    );
    failed(
        s.execute(&Command::Focus, || panic!("exhausted focus")),
        Error::LimitExceeded,
    );
    assert_eq!(s.observe_focus(true), Err(Error::LimitExceeded));
    assert_eq!(
        s.configure(Arc::new(Config {
            masked: true,
            ..(*config()).clone()
        })),
        Err(Error::LimitExceeded)
    );
    assert_eq!(s.snapshot(), before);
    let read = command(&mut s, Command::ReadSnapshot);
    assert!(read.events.is_empty());
    assert_eq!(read.response, Response::Applied(before));
    // Clipboard side effects and preedit cancellation also require capacity.
    let mut s = state("123456");
    select(&mut s, 6, 0);
    s.revision = i64::MAX;
    assert_eq!(
        s.cut(Access::Allowed, |_| panic!("exhausted clipboard")),
        Err(Error::LimitExceeded)
    );
    assert_eq!(s.editor().value(), "123456");
    s.revision = 0;
    mark(&mut s, "かな");
    s.revision = i64::MAX;
    let before = s.snapshot();
    assert_eq!(s.cancel_for_lifecycle(), Err(Error::LimitExceeded));
    assert_eq!(s.unmark(Access::Blocked), Err(Error::LimitExceeded));
    assert_eq!(s.snapshot(), before);
}

#[test]
fn coalescing_preserves_complete_rejected_and_programmatic_boundaries() {
    let mut s = state("");
    let mut events = Vec::new();
    for text in ["1", "2", "3", "4", "5", "6", "7"] {
        events.extend(commit(&mut s, text));
    }
    events.extend(
        command(
            &mut s,
            Command::Clear {
                undo: UndoPolicy::Reset,
                if_revision: None,
            },
        )
        .events,
    );
    events.extend(commit(&mut s, "12"));
    events.extend(commit(&mut s, "3"));
    let mut queue = Vec::<Event>::new();
    for event in events {
        if queue
            .last()
            .is_some_and(|prior| can_coalesce(prior, &event))
        {
            queue.pop();
        }
        queue.push(event);
    }
    assert!(
        matches!(&queue[..], [Event::Changed(a), Event::Complete(b), Event::Rejected(InputError::TooLong, _), Event::Observed(empty), Event::Changed(last)] if a.value == "123456" && b.revision == a.revision + 1 && empty.value.is_empty() && last.value == "123")
    );
    assert!(queue.iter().all(Event::is_valid));
    let first = Event::Changed(s.snapshot());
    assert!(!can_coalesce(&first, &first));
    let mut other = s.snapshot();
    other.revision += 1;
    other.policy = Policy::new(6, Alphabet::AsciiAlphanumeric).unwrap();
    assert!(!can_coalesce(&first, &Event::Changed(other)));
}
