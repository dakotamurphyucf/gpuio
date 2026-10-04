use super::*;
use gpuio_protocol::numeric::Domain;

fn setup(text: &str) -> (State, EditorSnapshot) {
    let config = Arc::new(Config {
        domain: Domain::new(0., 100., 0.5).unwrap(),
        label: "Amount".into(),
        placeholder: String::new(),
        increment_label: "Increase".into(),
        decrement_label: "Decrease".into(),
        step_controls: StepControls::Sides,
        allow_empty: false,
        disabled: false,
        read_only: false,
        auto_focus: false,
    });
    let editor = EditorSnapshot {
        revision: 0,
        text: text.into(),
        selection: EditorSelection {
            anchor: text.len() as i64,
            head: text.len() as i64,
        },
        composition: None,
        focused: true,
    };
    (
        State::with_initial_draft(config, Value::Number(12.), &editor).unwrap(),
        editor,
    )
}
fn request(state: &mut State, editor: &EditorSnapshot) -> Request {
    let outcome = state
        .request_application_step(editor, Direction::Increase, Source::Stepper)
        .unwrap();
    assert!(outcome.events.iter().all(Event::is_valid));
    outcome.request.unwrap()
}
fn no_edit(_: &EditorCommand) -> Result<EditorSnapshot, EditorError> {
    panic!("operation must not edit")
}
fn resolve(
    state: &mut State,
    editor: &EditorSnapshot,
    request: &Request,
    value: Option<Value>,
) -> Outcome {
    state
        .resolve_application_step(
            editor,
            request.id,
            request.snapshot.revision,
            value,
            no_edit,
        )
        .unwrap()
}
fn stale(outcome: Outcome) {
    assert!(matches!(
        outcome.response,
        Response::Failed(Error::StaleRevision)
    ));
}
#[test]
fn request_does_not_step_single_flight_and_resolution_commits_with_original_source() {
    let (mut state, mut editor) = setup("12");
    let before = state.snapshot().clone();
    let first = request(&mut state, &editor);
    assert_eq!(first.snapshot, before);
    assert_eq!(state.snapshot(), &before);
    let busy = state
        .request_application_step(&editor, Direction::Decrease, Source::Keyboard)
        .unwrap();
    assert!(matches!(busy.request, Err(Error::Busy)));
    assert!(busy.events.is_empty());
    state
        .execute(
            &editor,
            &Command::ReadSnapshot,
            Source::Programmatic,
            no_edit,
        )
        .unwrap();
    assert!(state.has_step_request());
    let live = editor.clone();
    let outcome = state
        .resolve_application_step(
            &live,
            first.id,
            first.snapshot.revision,
            Some(Value::Number(17.3)),
            |command| {
                let EditorCommand::Replace(
                    text,
                    EditorSelectionPolicy::End,
                    EditorUndoPolicy::Record,
                    Some(revision),
                ) = command
                else {
                    panic!("unexpected edit {command:?}")
                };
                assert_eq!(*revision, editor.revision);
                assert_eq!(text, "17.5");
                editor.revision += 1;
                editor.text = text.clone();
                editor.selection = EditorSelection { anchor: 4, head: 4 };
                Ok(editor.clone())
            },
        )
        .unwrap();
    assert!(
        matches!(outcome.events.as_slice(),[Event::Committed(Source::Stepper,s)] if s.committed==Value::Number(17.5))
    );
    assert!(outcome.events.iter().all(Event::is_valid));
    assert!(!state.has_step_request());
    stale(resolve(
        &mut state,
        &editor,
        &first,
        Some(Value::Number(90.)),
    ));
    let next = request(&mut state, &editor);
    assert!(next.id > first.id);
    stale(resolve(&mut state, &editor, &first, None));
    assert!(
        state.has_step_request(),
        "late duplicate must not consume newer intent"
    );
    let declined = resolve(&mut state, &editor, &next, None);
    assert!(declined.events.is_empty());
    assert!(matches!(declined.response, Response::Applied(_)));
    assert!(!state.has_step_request());
}
#[test]
fn every_intervening_native_edit_or_policy_change_invalidates_the_reply() {
    for change in 0..7 {
        let (mut state, mut editor) = setup("12");
        let pending = request(&mut state, &editor);
        match change {
            0 => {
                editor.revision += 1;
                editor.text = "13".into();
            }
            1 => {
                editor.selection = EditorSelection { anchor: 0, head: 2 };
            }
            2 => {
                editor.composition = Some(EditorSelection { anchor: 0, head: 2 });
            }
            3 => {
                editor.focused = false;
            }
            // Even a text edit undone before observation invalidates the reply.
            4 => {
                editor.revision += 2;
            }
            5 | 6 => {
                let mut config = state.config().clone();
                if change == 5 {
                    config.read_only = true;
                } else {
                    config.domain = Domain::new(0., 10., 1.).unwrap();
                }
                state.configure(Arc::new(config), &editor).unwrap();
            }
            _ => unreachable!(),
        }
        let before = editor.clone();
        stale(resolve(
            &mut state,
            &editor,
            &pending,
            Some(Value::Number(99.)),
        ));
        assert_eq!(editor, before);
        assert!(!state.has_step_request());
    }
}
#[test]
fn invalid_drafts_reject_before_queuing_and_resolution_errors_release_only_the_matching_request() {
    for text in ["1e-", "abc", "1e999"] {
        let (mut state, editor) = setup(text);
        let outcome = state
            .request_application_step(&editor, Direction::Increase, Source::Keyboard)
            .unwrap();
        assert!(matches!(outcome.request, Err(Error::Rejected(_))));
        assert!(matches!(outcome.events.as_slice(),[Event::Rejected(_,s)] if s.draft==text));
        assert!(outcome.events.iter().all(Event::is_valid));
        assert!(!state.has_step_request());
    }
    let (mut state, editor) = setup("12");
    for value in [Value::Number(f64::NAN), Value::Empty] {
        let pending = request(&mut state, &editor);
        let result = resolve(&mut state, &editor, &pending, Some(value));
        assert!(matches!(
            result.response,
            Response::Failed(Error::InvalidValue | Error::Rejected(Rejection::EmptyRequired))
        ));
        assert_eq!(state.snapshot().draft, "12");
        assert!(!state.has_step_request());
    }
    let pending = request(&mut state, &editor);
    let result = state
        .resolve_application_step(
            &editor,
            pending.id,
            pending.snapshot.revision,
            Some(Value::Number(20.)),
            |_| Err(EditorError::Busy),
        )
        .unwrap();
    assert!(matches!(result.response, Response::Failed(Error::Busy)));
    assert!(!state.has_step_request());
    assert_eq!(state.snapshot().committed, Value::Number(12.));
}
#[test]
fn explicit_commands_cancellation_and_token_exhaustion_do_not_reuse_requests() {
    let (mut state, editor) = setup("12");
    let first = request(&mut state, &editor);
    assert!(state.cancel_step_request());
    assert!(!state.cancel_step_request());
    let next = request(&mut state, &editor);
    assert!(next.id > first.id);
    // Even an admitted same-selection operation supersedes an application step.
    state
        .execute(
            &editor,
            &Command::Select(Selection { anchor: 2, head: 2 }),
            Source::Programmatic,
            |_| Ok(editor.clone()),
        )
        .unwrap();
    stale(resolve(&mut state, &editor, &next, None));
    state.step_requests.last_id = i64::MAX;
    let outcome = state
        .request_application_step(&editor, Direction::Increase, Source::Keyboard)
        .unwrap();
    assert!(matches!(outcome.request, Err(Error::LimitExceeded)));
    assert!(!state.has_step_request());
    assert_eq!(state.snapshot().draft, "12");
}

#[test]
fn unchanged_policy_preserves_requests_but_disabled_and_composing_inputs_cannot_request() {
    let (mut state, mut editor) = setup("12");
    let pending = request(&mut state, &editor);
    let unchanged = state
        .configure(Arc::new(state.config().clone()), &editor)
        .unwrap();
    assert!(unchanged.events.is_empty());
    assert!(state.has_step_request());
    let declined = resolve(&mut state, &editor, &pending, None);
    assert!(matches!(declined.response, Response::Applied(_)));
    let mut disabled = state.config().clone();
    disabled.disabled = true;
    state.configure(Arc::new(disabled), &editor).unwrap();
    let outcome = state
        .request_application_step(&editor, Direction::Increase, Source::Accessibility)
        .unwrap();
    assert!(matches!(outcome.request, Err(Error::Disabled)));
    assert!(!state.has_step_request());
    let mut enabled = state.config().clone();
    enabled.disabled = false;
    state.configure(Arc::new(enabled), &editor).unwrap();
    editor.composition = Some(EditorSelection { anchor: 0, head: 2 });
    let outcome = state
        .request_application_step(&editor, Direction::Increase, Source::Keyboard)
        .unwrap();
    assert!(matches!(outcome.request, Err(Error::Composing)));
    assert!(matches!(
        outcome.events.last(),
        Some(Event::Rejected(Rejection::Composing, _))
    ));
    assert!(outcome.events.iter().all(Event::is_valid));
    assert!(!state.has_step_request());
}

#[test]
fn resolution_command_uses_guarded_user_commit_instead_of_programmatic_replacement() {
    let (mut state, editor) = setup("12");
    let request = request(&mut state, &editor);
    let command = Command::ResolveStep {
        request_id: request.id,
        revision: request.snapshot.revision,
        value: None,
    };
    let outcome = state
        .execute(&editor, &command, Source::Programmatic, no_edit)
        .unwrap();
    assert!(matches!(outcome.response, Response::Applied(_)));
    assert!(!state.has_step_request());
    let repeated = state
        .execute(&editor, &command, Source::Programmatic, no_edit)
        .unwrap();
    stale(repeated);
}
