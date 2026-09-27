use super::*;
use gpuio_protocol::numeric::{Direction, Domain};

fn config() -> Arc<Config> {
    Arc::new(Config {
        domain: Domain::new(-2., 8., 0.5).unwrap(),
        label: "Value".into(),
        placeholder: "".into(),
        increment_label: "Increase".into(),
        decrement_label: "Decrease".into(),
        step_controls: StepControls::Sides,
        allow_empty: false,
        disabled: false,
        read_only: false,
        auto_focus: false,
    })
}
fn native_snapshot(text: &str) -> EditorSnapshot {
    EditorSnapshot {
        revision: 0,
        text: text.into(),
        selection: EditorSelection {
            anchor: text.len() as i64,
            head: text.len() as i64,
        },
        composition: None,
        focused: false,
    }
}
fn new_state(value: Value) -> (State, EditorSnapshot) {
    let config = config();
    let editor = native_snapshot(&initial_text(&config, value).unwrap());
    (State::new(config, value, &editor).unwrap(), editor)
}
fn change(editor: &mut EditorSnapshot, text: &str) {
    editor.revision += 1;
    editor.text = text.into();
    editor.selection = EditorSelection {
        anchor: text.len() as i64,
        head: text.len() as i64,
    };
    editor.composition = None;
}
// Only an adapter stub, not a text editor or undo implementation. Tests of
// actual InputState history/IME are the responsibility of the mounted adapter.
fn apply(
    editor: &mut EditorSnapshot,
    command: &EditorCommand,
) -> Result<EditorSnapshot, EditorError> {
    match command {
        EditorCommand::Replace(text, policy, _, revision) => {
            assert_eq!(*revision, Some(editor.revision));
            change(editor, text);
            match policy {
                EditorSelectionPolicy::End => (),
                EditorSelectionPolicy::Start => {
                    editor.selection = EditorSelection { anchor: 0, head: 0 }
                }
                EditorSelectionPolicy::Select(s) => editor.selection = s.clone(),
                EditorSelectionPolicy::Preserve => {
                    panic!("test must supply the expected native selection explicitly")
                }
            }
        }
        EditorCommand::Select(s) => editor.selection = s.clone(),
        EditorCommand::Focus => editor.focused = true,
        _ => panic!("test must supply this native action's result explicitly"),
    }
    Ok(editor.clone())
}
fn run(
    state: &mut State,
    editor: &mut EditorSnapshot,
    command: &Command,
    source: Source,
) -> Outcome {
    let live = editor.clone();
    let outcome = state
        .execute(&live, command, source, |cmd| apply(editor, cmd))
        .unwrap();
    assert!(outcome.events.len() <= 2);
    assert!(outcome.events.iter().all(Event::is_valid));
    assert!(outcome.response.is_valid());
    outcome
}
fn denied(
    state: &mut State,
    editor: &EditorSnapshot,
    command: &Command,
    source: Source,
) -> Outcome {
    let outcome = state
        .execute(editor, command, source, |_| {
            panic!("failed command mutated the native editor")
        })
        .unwrap();
    assert!(matches!(outcome.response, Response::Failed(_)));
    assert!(outcome.events.iter().all(Event::is_valid));
    outcome
}
fn replace(text: &str, revision: Option<i64>) -> Command {
    Command::ReplaceDraft {
        text: text.into(),
        selection: SelectionPolicy::End,
        undo: UndoPolicy::Record,
        if_revision: revision,
    }
}
fn number(state: &State) -> f64 {
    match state.snapshot().committed {
        Value::Number(n) => n,
        Value::Empty => panic!("no committed number"),
    }
}

#[test]
fn initial_values_are_normalized_and_format_roundtrips_extreme_finite_floats() {
    let (state, editor) = new_state(Value::Number(1.25));
    assert_eq!(editor.text, "1.5");
    assert_eq!(number(&state), 1.5);
    assert_eq!(state.snapshot().revision, 0);
    assert_eq!(
        initial_text(&config(), Value::Number(f64::NAN)),
        Err(Error::InvalidValue)
    );
    assert_eq!(initial_text(&config(), Value::Empty), Ok("".into()));
    assert!(State::new(config(), Value::Number(1.), &native_snapshot("2")).is_err());
    for value in [
        0.,
        -0.,
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
        1e-200,
        1e200,
    ] {
        let config = Arc::new(Config {
            domain: Domain::new(value, value, 1.).unwrap(),
            ..config().as_ref().clone()
        });
        let text = initial_text(&config, Value::Number(value)).unwrap();
        assert!(valid_text(&text));
        assert_eq!(text.parse::<f64>().unwrap(), value);
        let state = State::new(config, Value::Number(value), &native_snapshot(&text)).unwrap();
        assert!(state.snapshot().is_settled());
    }
}
#[test]
fn pending_edits_fence_replacements_and_final_observations_do_not_duplicate() {
    let (mut state, mut editor) = new_state(Value::Number(1.));
    assert!(state.observe(&editor).unwrap().is_none());
    change(&mut editor, "1e-");
    let outcome = denied(
        &mut state,
        &editor,
        &replace("5", Some(0)),
        Source::Programmatic,
    );
    assert_eq!(outcome.response, Response::Failed(Error::StaleRevision));
    assert!(
        matches!(outcome.events.as_slice(), [Event::Changed(s)] if s.draft == "1e-" && s.revision == 1)
    );
    assert_eq!(number(&state), 1.);
    let outcome = run(
        &mut state,
        &mut editor,
        &replace("é", Some(1)),
        Source::Programmatic,
    );
    assert!(
        matches!(outcome.events.as_slice(), [Event::Changed(s)] if s.revision == 2 && s.classification() == Draft::Invalid(DraftError::Syntax))
    );
    assert!(state.observe(&editor).unwrap().is_none());
    editor.revision += 1; // Inner history-only work is not an exposed state change.
    assert!(state.observe(&editor).unwrap().is_none());
    let selection = Command::Select(Selection { anchor: 2, head: 0 });
    let outcome = run(&mut state, &mut editor, &selection, Source::Programmatic);
    assert!(
        matches!(outcome.events.as_slice(), [Event::Changed(s)] if s.selection.anchor == 2 && s.selection.head == 0 && s.revision == 3)
    );
    assert_eq!(number(&state), 1.);
}
#[test]
fn commits_reject_transient_or_invalid_drafts_without_editing_then_normalize_valid_values() {
    let (mut state, mut editor) = new_state(Value::Number(2.));
    for (draft, reason) in [
        ("", Rejection::EmptyRequired),
        ("-", Rejection::Incomplete),
        ("1e-", Rejection::Incomplete),
        ("é", Rejection::Syntax),
        ("1e999", Rejection::NonFinite),
    ] {
        change(&mut editor, draft);
        let before = editor.clone();
        let outcome = denied(&mut state, &editor, &Command::Commit, Source::Keyboard);
        assert_eq!(outcome.response, Response::Failed(Error::Rejected(reason)));
        assert!(
            matches!(outcome.events.as_slice(), [Event::Changed(_), Event::Rejected(r, _)] if *r == reason)
        );
        let revision = state.snapshot().revision;
        let repeated = denied(&mut state, &editor, &Command::Commit, Source::Programmatic);
        assert!(
            matches!(repeated.events.as_slice(), [Event::Rejected(_, s)] if s.revision == revision+1)
        );
        assert_eq!(editor, before);
        assert_eq!(number(&state), 2.);
    }
    for (draft, expected, source) in [
        (" 1.25 ", 1.5, Source::Keyboard),
        ("99", 8., Source::Programmatic),
        ("-99", -2., Source::Accessibility),
        ("1e0", 1., Source::Stepper),
    ] {
        change(&mut editor, draft);
        let outcome = run(&mut state, &mut editor, &Command::Commit, source);
        assert!(
            matches!(outcome.events.as_slice(), [Event::Changed(_), Event::Committed(s, _)] if *s == source)
        );
        assert_eq!(number(&state), expected);
        assert_eq!(editor.text.parse::<f64>().unwrap(), expected);
        assert_eq!(editor.selection.head, editor.text.len() as i64);
        let revision = state.snapshot().revision;
        let repeated = run(&mut state, &mut editor, &Command::Commit, source);
        assert!(
            matches!(repeated.events.as_slice(), [Event::Committed(_, s)] if s.revision == revision+1)
        );
        assert!(state.observe(&editor).unwrap().is_none());
    }
    editor.focused = true;
    state.observe(&editor).unwrap();
    change(&mut editor, "-");
    editor.focused = false;
    assert!(matches!(
        state.observe(&editor).unwrap(),
        Some(Event::Changed(_))
    ));
    assert_eq!(editor.text, "-");
    assert_eq!(number(&state), 1.); // Blur is not commit.
}
#[test]
fn step_normalizes_before_advancing_seeds_empty_and_saturates() {
    let (mut state, mut editor) = new_state(Value::Empty);
    for direction in [Direction::Increase, Direction::Decrease] {
        change(&mut editor, "");
        run(
            &mut state,
            &mut editor,
            &Command::Step(direction),
            Source::Stepper,
        );
        assert_eq!(number(&state), 0.);
    }
    for (draft, direction, expected) in [
        ("1.25", Direction::Increase, 2.),
        ("1.25", Direction::Decrease, 1.),
        ("99", Direction::Decrease, 7.5),
        ("99", Direction::Increase, 8.),
        ("-99", Direction::Decrease, -2.),
    ] {
        change(&mut editor, draft);
        let outcome = run(
            &mut state,
            &mut editor,
            &Command::Step(direction),
            Source::Accessibility,
        );
        assert!(matches!(
            outcome.events.last(),
            Some(Event::Committed(Source::Accessibility, _))
        ));
        assert_eq!(number(&state), expected);
    }
    change(&mut editor, "-");
    let result = denied(
        &mut state,
        &editor,
        &Command::Step(Direction::Increase),
        Source::Programmatic,
    );
    assert_eq!(
        result.response,
        Response::Failed(Error::Rejected(Rejection::Incomplete))
    );
    assert_eq!(editor.text, "-");
}
#[test]
fn undo_observations_do_not_revert_committed_value_and_cancel_restores_it() {
    let (mut state, mut editor) = new_state(Value::Number(1.));
    change(&mut editor, "2.25");
    run(&mut state, &mut editor, &Command::Commit, Source::Keyboard);
    assert_eq!(number(&state), 2.5);
    let live = editor.clone();
    let outcome = state
        .execute(&live, &Command::Undo, Source::Programmatic, |command| {
            assert_eq!(command, &EditorCommand::Undo);
            change(&mut editor, "2.25");
            editor.selection = EditorSelection { anchor: 4, head: 1 };
            Ok(editor.clone())
        })
        .unwrap();
    assert!(
        matches!(outcome.events.as_slice(), [Event::Changed(s)] if s.draft == "2.25" && s.committed == Value::Number(2.5))
    );
    assert_eq!(number(&state), 2.5);
    let outcome = run(&mut state, &mut editor, &Command::Cancel, Source::Keyboard);
    assert!(matches!(
        outcome.events.as_slice(),
        [Event::Cancelled(CancelReason::Escape, _)]
    ));
    assert_eq!(editor.text, "2.5");
    let outcome = run(
        &mut state,
        &mut editor,
        &Command::Cancel,
        Source::Programmatic,
    );
    assert!(matches!(
        outcome.events.as_slice(),
        [Event::Cancelled(CancelReason::Programmatic, _)]
    ));
}
#[test]
fn composition_is_preserved_across_policy_changes_and_protected_from_commands() {
    let (mut state, mut editor) = new_state(Value::Number(7.));
    change(&mut editor, "é1e-");
    editor.selection = EditorSelection { anchor: 5, head: 2 };
    editor.composition = Some(EditorSelection { anchor: 2, head: 5 });
    let updated = Arc::new(Config {
        domain: Domain::new(0., 3., 1.).unwrap(),
        ..config().as_ref().clone()
    });
    let outcome = state.configure(updated.clone(), &editor).unwrap();
    assert!(
        matches!(outcome.events.as_slice(), [Event::Changed(old), Event::Observed(new)] if old.domain.max() == 8. && old.committed == Value::Number(7.) && new.domain.max() == 3. && new.committed == Value::Number(3.))
    );
    assert_eq!(state.snapshot().draft, editor.text);
    assert_eq!(state.snapshot().selection, selection(&editor.selection));
    assert_eq!(
        state.snapshot().composition,
        editor.composition.as_ref().map(selection)
    );
    assert!(state.configure(updated, &editor).unwrap().events.is_empty());
    for command in [
        replace("2", None),
        Command::Select(Selection { anchor: 0, head: 0 }),
        Command::Undo,
        Command::Redo,
        Command::Cancel,
    ] {
        let result = denied(&mut state, &editor, &command, Source::Programmatic);
        assert_eq!(result.response, Response::Failed(Error::Composing));
        assert!(result.events.is_empty());
    }
    for command in [Command::Commit, Command::Step(Direction::Increase)] {
        let result = denied(&mut state, &editor, &command, Source::Programmatic);
        assert_eq!(result.response, Response::Failed(Error::Composing));
        assert!(matches!(
            result.events.as_slice(),
            [Event::Rejected(Rejection::Composing, _)]
        ));
    }
    let before = editor.clone();
    run(
        &mut state,
        &mut editor,
        &Command::Focus,
        Source::Programmatic,
    );
    assert_eq!(editor.text, before.text);
    assert_eq!(editor.composition, before.composition);
    let result = run(
        &mut state,
        &mut editor,
        &Command::ReadSnapshot,
        Source::Programmatic,
    );
    assert!(result.events.is_empty());
    // The adapter unmarks IME on Escape first; only a later Escape restores value.
    editor.composition = None;
    state.observe(&editor).unwrap();
    run(&mut state, &mut editor, &Command::Cancel, Source::Keyboard);
    assert_eq!(editor.text, "3");
}
#[test]
fn disabled_and_readonly_gate_user_mutation_but_allow_explicit_replacement_commit_cancel() {
    for (disabled, read_only, error) in [
        (true, false, Error::Disabled),
        (false, true, Error::ReadOnly),
    ] {
        let (mut state, mut editor) = new_state(Value::Number(1.));
        state
            .configure(
                Arc::new(Config {
                    disabled,
                    read_only,
                    ..config().as_ref().clone()
                }),
                &editor,
            )
            .unwrap();
        for source in [Source::Keyboard, Source::Stepper, Source::Accessibility] {
            for command in [
                replace("5", None),
                Command::Commit,
                Command::Cancel,
                Command::Undo,
                Command::Redo,
                Command::Step(Direction::Increase),
            ] {
                assert_eq!(
                    denied(&mut state, &editor, &command, source).response,
                    Response::Failed(error)
                );
            }
        }
        assert_eq!(
            denied(
                &mut state,
                &editor,
                &Command::Step(Direction::Increase),
                Source::Programmatic
            )
            .response,
            Response::Failed(error)
        );
        run(
            &mut state,
            &mut editor,
            &replace("2.25", None),
            Source::Programmatic,
        );
        run(
            &mut state,
            &mut editor,
            &Command::Commit,
            Source::Programmatic,
        );
        assert_eq!(number(&state), 2.5);
        run(
            &mut state,
            &mut editor,
            &replace("-", None),
            Source::Programmatic,
        );
        run(
            &mut state,
            &mut editor,
            &Command::Cancel,
            Source::Programmatic,
        );
        assert_eq!(editor.text, "2.5");
        let reset = Command::ReplaceValue {
            value: Value::Empty,
            selection: SelectionPolicy::End,
            undo: UndoPolicy::Reset,
            if_revision: None,
        };
        let outcome = run(&mut state, &mut editor, &reset, Source::Programmatic);
        assert!(
            matches!(outcome.events.as_slice(), [Event::Observed(s)] if s.committed == Value::Empty && s.draft.is_empty())
        );
        if disabled {
            assert_eq!(
                denied(&mut state, &editor, &Command::Focus, Source::Programmatic).response,
                Response::Failed(Error::FocusBlocked)
            );
        } else {
            run(
                &mut state,
                &mut editor,
                &Command::Focus,
                Source::Programmatic,
            );
        }
    }
}
#[test]
fn selection_and_replace_validation_precede_native_mutation() {
    let (mut state, mut editor) = new_state(Value::Empty);
    run(
        &mut state,
        &mut editor,
        &replace("é", None),
        Source::Programmatic,
    );
    for (command, error) in [
        (
            Command::Select(Selection { anchor: 1, head: 2 }),
            Error::InvalidSelection,
        ),
        (replace("a\n", None), Error::InvalidText),
        (replace(&"a".repeat(4097), None), Error::InvalidText),
        (
            Command::ReplaceValue {
                value: Value::Number(f64::NAN),
                selection: SelectionPolicy::End,
                undo: UndoPolicy::Reset,
                if_revision: None,
            },
            Error::InvalidValue,
        ),
        (
            Command::ReplaceValue {
                value: Value::Number(1.),
                selection: SelectionPolicy::Select(Selection { anchor: 2, head: 2 }),
                undo: UndoPolicy::Reset,
                if_revision: None,
            },
            Error::InvalidSelection,
        ),
    ] {
        let before = state.snapshot().clone();
        assert_eq!(
            denied(&mut state, &editor, &command, Source::Programmatic).response,
            Response::Failed(error)
        );
        assert_eq!(state.snapshot(), &before);
    }
    // Preserve offsets must clip down to UTF8 boundaries of the replacement.
    run(
        &mut state,
        &mut editor,
        &replace("123", None),
        Source::Programmatic,
    );
    run(
        &mut state,
        &mut editor,
        &Command::Select(Selection { anchor: 3, head: 1 }),
        Source::Programmatic,
    );
    let live = editor.clone();
    let command = Command::ReplaceDraft {
        text: "é".into(),
        selection: SelectionPolicy::Preserve,
        undo: UndoPolicy::Reset,
        if_revision: None,
    };
    let outcome = state.execute(&live, &command, Source::Programmatic, |cmd| {
        assert!(matches!(cmd, EditorCommand::Replace(text, EditorSelectionPolicy::Preserve, EditorUndoPolicy::Reset, _) if text == "é"));
        change(&mut editor, "é");
        editor.selection = EditorSelection {anchor: 2, head: 0};
        Ok(editor.clone())
    }).unwrap();
    assert!(
        matches!(outcome.response, Response::Applied(s) if s.selection == Selection {anchor: 2, head: 0})
    );
}
#[test]
fn command_overflow_is_atomic_and_observation_overflow_faults_without_rewinding() {
    let (mut state, mut editor) = new_state(Value::Number(1.));
    state.snapshot.revision = i64::MAX - 1;
    run(
        &mut state,
        &mut editor,
        &Command::Commit,
        Source::Programmatic,
    );
    assert_eq!(state.snapshot().revision, i64::MAX);
    assert!(state.observe(&editor).unwrap().is_none());
    for command in [
        replace("5", None),
        Command::Commit,
        Command::Cancel,
        Command::Focus,
        Command::Step(Direction::Increase),
        Command::Undo,
    ] {
        assert_eq!(
            denied(&mut state, &editor, &command, Source::Programmatic).response,
            Response::Failed(Error::LimitExceeded)
        );
    }
    let current = state.config.clone();
    let updated = Arc::new(Config {
        label: "Updated".into(),
        ..current.as_ref().clone()
    });
    assert_eq!(
        state.configure(updated, &editor).unwrap().response,
        Response::Failed(Error::LimitExceeded)
    );
    assert_eq!(state.config, current);
    run(
        &mut state,
        &mut editor,
        &Command::ReadSnapshot,
        Source::Programmatic,
    );
    change(&mut editor, "5");
    assert_eq!(state.observe(&editor), Err(Fault::RevisionExhausted));
    assert_eq!(number(&state), 1.);
    assert_eq!(editor.text, "5"); // Never undo an ordinary native edit to hide overflow.
    assert_eq!(state.observe(&editor), Err(Fault::RevisionExhausted));
}
#[test]
fn native_failures_do_not_publish_numeric_success_and_invalid_success_is_sticky_fatal() {
    let (mut state, mut editor) = new_state(Value::Number(1.));
    change(&mut editor, "2");
    let outcome = state
        .execute(&editor, &Command::Commit, Source::Keyboard, |_| {
            Err(EditorError::LimitExceeded)
        })
        .unwrap();
    assert_eq!(outcome.response, Response::Failed(Error::LimitExceeded));
    assert!(matches!(outcome.events.as_slice(), [Event::Changed(_)]));
    assert_eq!(number(&state), 1.);
    assert_eq!(state.snapshot().revision, 1);
    let live = editor.clone();
    let result = state.execute(&live, &Command::Commit, Source::Keyboard, |_| {
        change(&mut editor, "7");
        Ok(editor.clone())
    });
    assert!(matches!(result, Err(Fault::UnexpectedEdit)));
    assert_eq!(number(&state), 1.);
    let result = state.execute(
        &editor,
        &Command::ReadSnapshot,
        Source::Programmatic,
        |_| panic!("faulted owner executed a command"),
    );
    assert!(matches!(result, Err(Fault::UnexpectedEdit)));
    let (mut state, mut editor) = new_state(Value::Number(1.));
    editor.selection.head = 2;
    assert_eq!(state.observe(&editor), Err(Fault::InvalidObservation));
}

#[test]
fn allowed_empty_commit_is_a_boundary_and_required_config_does_not_invent_a_number() {
    let (mut state, mut editor) = new_state(Value::Empty);
    state
        .configure(
            Arc::new(Config {
                allow_empty: true,
                ..config().as_ref().clone()
            }),
            &editor,
        )
        .unwrap();
    change(&mut editor, " \t");
    let outcome = run(&mut state, &mut editor, &Command::Commit, Source::Keyboard);
    assert!(
        matches!(outcome.events.last(), Some(Event::Committed(Source::Keyboard, s)) if s.committed == Value::Empty && s.draft.is_empty())
    );
    let previous = state.snapshot().clone();
    state.configure(config(), &editor).unwrap();
    assert_eq!(state.snapshot().committed, Value::Empty);
    assert_eq!(state.snapshot().draft, previous.draft);
    assert_eq!(state.snapshot().selection, previous.selection);
    let result = denied(&mut state, &editor, &Command::Commit, Source::Keyboard);
    assert_eq!(
        result.response,
        Response::Failed(Error::Rejected(Rejection::EmptyRequired))
    );
    let before = state.snapshot().clone();
    let invalid = Arc::new(Config {
        label: "".into(),
        ..config().as_ref().clone()
    });
    assert_eq!(
        state.configure(invalid, &editor).unwrap().response,
        Response::Failed(Error::InvalidConfig)
    );
    assert_eq!(state.snapshot(), &before);
}

#[test]
fn revisioned_mixed_operations_preserve_validity_and_bounded_publication() {
    let (mut state, mut editor) = new_state(Value::Empty);
    let mut revision = 0;
    for cycle in 0..256 {
        change(
            &mut editor,
            ["1e-", "1.25", "é", "", "99", "-99", "1e999", "2.25"][cycle % 8],
        );
        let command = match cycle % 4 {
            0 => Command::Commit,
            1 => Command::Step(Direction::Increase),
            2 => Command::Cancel,
            _ => Command::Step(Direction::Decrease),
        };
        let outcome = run(&mut state, &mut editor, &command, Source::Programmatic);
        for event in outcome.events {
            assert!(event.snapshot().revision > revision);
            revision = event.snapshot().revision;
        }
        assert!(state.snapshot().is_valid());
        assert_eq!(state.snapshot().draft, editor.text);
        assert_eq!(state.snapshot().revision, revision);
        assert!(state.observe(&editor).unwrap().is_none());
        if cycle % 8 == 0 {
            let next = Arc::new(Config {
                domain: Domain::new(1., 3., 0.5).unwrap(),
                ..config().as_ref().clone()
            });
            let outcome = state.configure(next, &editor).unwrap();
            for event in outcome.events {
                assert!(event.is_valid());
                assert!(event.snapshot().revision > revision);
                revision = event.snapshot().revision;
            }
        }
    }
    editor.revision -= 1;
    assert_eq!(state.observe(&editor), Err(Fault::InvalidObservation));
}
