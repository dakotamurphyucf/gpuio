use super::*;

fn date(day: i64) -> Date {
    Date::from_ymd(2024, 2, day).unwrap()
}
fn config(mode: Mode) -> Arc<Config> {
    Arc::new(Config {
        mode,
        constraints: Constraints::unrestricted(),
        first_weekday: 1,
        labels: Labels::english(),
        today: Some(date(29)),
        label: "Calendar".into(),
        disabled: false,
        read_only: false,
        auto_focus: false,
    })
}
fn state(mode: Mode) -> State {
    State::new(config(mode), Selection::Empty, Month::from_date(date(1))).unwrap()
}
fn command(state: &mut State, command: Command) -> Outcome {
    state.execute(&command, || panic!("non-focus command called native focus"))
}
fn applied(outcome: Outcome) -> Snapshot {
    for event in &outcome.events {
        assert!(matches!(event, Event::Observed(_)));
        assert!(event.is_valid());
    }
    match outcome.response {
        Response::Applied(snapshot) => snapshot,
        Response::Failed(error) => panic!("command failed: {error:?}"),
    }
}

#[test]
fn initial_month_and_cursor_are_explicit_and_programmatic_changes_never_select() {
    let mut s = state(Mode::Single);
    assert_eq!(s.snapshot().focused_date, date(29));
    assert_eq!(s.snapshot().revision, 0);
    let initial = Selection::Single(date(4));
    let selected = applied(command(
        &mut s,
        Command::Replace {
            selection: initial,
            if_revision: Some(0),
        },
    ));
    assert_eq!(selected.selection, initial);
    assert_eq!(selected.focused_date, date(29)); // Replace does not navigate.
    assert_eq!(selected.revision, 1);
    let before = s.snapshot();
    assert_eq!(
        command(
            &mut s,
            Command::Clear {
                if_revision: Some(0)
            }
        )
        .response,
        Response::Failed(Error::StaleRevision)
    );
    assert_eq!(s.snapshot(), before);
    let same = command(
        &mut s,
        Command::Replace {
            selection: initial,
            if_revision: Some(1),
        },
    );
    assert!(same.events.is_empty());
    assert_eq!(
        applied(command(&mut s, Command::Clear { if_revision: None })).selection,
        Selection::Empty
    );
    let next = Month::new(2024, 3).unwrap();
    let s = State::new(config(Mode::Single), initial, next).unwrap();
    assert_eq!(s.snapshot().focused_date, next.first_day());
    assert_eq!(s.snapshot().month, next);
    assert_eq!(s.snapshot().selection, initial);
}

#[test]
fn partial_range_restart_completion_order_and_rejection_are_native_observations() {
    let mut s = state(Mode::Range);
    let first = s
        .native(Action::Activate(date(10)), Access::Allowed)
        .unwrap();
    assert!(
        matches!(first.as_slice(),[Event::Changed(snapshot)] if snapshot.selection==Selection::RangeStart(date(10)))
    );
    let earlier = s
        .native(Action::Activate(date(5)), Access::Allowed)
        .unwrap();
    assert_eq!(
        earlier[0].snapshot().selection,
        Selection::RangeStart(date(5))
    );
    let complete = s
        .native(Action::Activate(date(12)), Access::Allowed)
        .unwrap();
    assert!(
        matches!(complete.as_slice(),[Event::Changed(a),Event::Selected(b)] if a.revision+1==b.revision && a.selection==b.selection)
    );
    assert!(complete.iter().all(Event::is_valid));
    let mut c = (*config(Mode::Range)).clone();
    c.constraints = Constraints::new(
        Date::MIN,
        Date::MAX,
        vec![date(8)],
        vec![],
        vec![],
        RangePolicy::EveryDay,
    )
    .unwrap();
    s.configure(Arc::new(c)).unwrap();
    assert!(!s.snapshot().selection_allowed);
    s.native(Action::Activate(date(5)), Access::Allowed)
        .unwrap();
    let before = s.snapshot();
    let rejected = s
        .native(Action::Activate(date(12)), Access::Allowed)
        .unwrap();
    assert!(matches!(
        rejected.as_slice(),
        [Event::Rejected(SelectionError::DisabledInterior, _)]
    ));
    let after = s.snapshot();
    assert_eq!(after.selection, before.selection);
    assert_eq!(after.month, before.month);
    assert_eq!(after.focused_date, before.focused_date);
    assert_eq!(after.revision, before.revision + 1);
    assert!(rejected[0].is_valid());
    let mut single = state(Mode::Single);
    assert_eq!(
        single
            .native(Action::Activate(date(5)), Access::Allowed)
            .unwrap()
            .len(),
        2
    );
    assert!(
        single
            .native(Action::Activate(date(5)), Access::Allowed)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn configuration_preserves_historical_selection_and_permission_gates_are_explicit() {
    let mut s = state(Mode::Single);
    s.native(Action::Activate(date(5)), Access::Allowed)
        .unwrap();
    let initial = s.snapshot();
    let mut c = (*config(Mode::Single)).clone();
    c.labels.today = "Heute".into();
    c.today = None;
    c.first_weekday = 0;
    c.read_only = true;
    c.constraints = Constraints::new(
        date(6),
        date(29),
        vec![],
        vec![],
        vec![],
        RangePolicy::EveryDay,
    )
    .unwrap();
    let c = Arc::new(c);
    let changed = s.configure(c.clone()).unwrap();
    assert!(
        matches!(changed.as_slice(),[Event::Observed(snapshot)] if !snapshot.selection_allowed)
    );
    assert_eq!(s.snapshot().selection, initial.selection);
    assert_eq!(s.snapshot().month, initial.month);
    assert!(s.configure(c.clone()).unwrap().is_empty());
    assert_eq!(
        s.native(Action::Activate(date(10)), Access::Allowed),
        Err(Error::ReadOnly)
    );
    assert!(s.native(Action::MoveDays(1), Access::Allowed).is_ok());
    assert_eq!(
        s.native(Action::MoveDays(1), Access::Blocked),
        Err(Error::FocusBlocked)
    );
    applied(command(
        &mut s,
        Command::Replace {
            selection: Selection::Single(date(10)),
            if_revision: None,
        },
    ));
    assert_eq!(
        command(
            &mut s,
            Command::Replace {
                selection: Selection::Single(date(5)),
                if_revision: None
            }
        )
        .response,
        Response::Failed(Error::DisabledDate)
    );
    let mut disabled = (*c).clone();
    disabled.disabled = true;
    s.configure(Arc::new(disabled)).unwrap();
    assert_eq!(
        s.native(Action::MoveDays(1), Access::Allowed),
        Err(Error::Disabled)
    );
    let result = s.execute(&Command::Focus, || {
        panic!("disabled field attempted platform focus")
    });
    assert_eq!(result.response, Response::Failed(Error::FocusBlocked));
    applied(command(&mut s, Command::Clear { if_revision: None }));
    let before = s.snapshot();
    assert_eq!(s.configure(config(Mode::Range)), Err(Error::InvalidConfig));
    assert_eq!(s.snapshot(), before);
}

#[test]
fn navigation_clamps_leap_days_and_focus_failure_is_atomic() {
    let mut s = state(Mode::Single);
    let snapshot = applied(command(&mut s, Command::MoveMonths(12)));
    assert_eq!(snapshot.focused_date, Date::from_ymd(2025, 2, 28).unwrap());
    assert_eq!(snapshot.selection, Selection::Empty);
    let before = s.snapshot();
    let result = s.execute(&Command::FocusDate(Date::MAX), || Err(Error::FocusBlocked));
    assert_eq!(result.response, Response::Failed(Error::FocusBlocked));
    assert_eq!(s.snapshot(), before);
    let focused = applied(s.execute(&Command::FocusDate(Date::MAX), || Ok(())));
    assert!(focused.focused);
    assert_eq!(focused.month, Month::new(9999, 12).unwrap());
    assert_eq!(focused.focused_date, Date::MAX);
    assert_eq!(
        command(&mut s, Command::MoveMonths(1)).response,
        Response::Failed(Error::InvalidValue)
    );
    assert_eq!(
        s.native(Action::MoveDays(1), Access::Allowed),
        Err(Error::InvalidValue)
    );
    assert_eq!(s.snapshot(), focused);
    let lost = s.observe_focus(false).unwrap();
    assert!(matches!(lost.as_slice(),[Event::Changed(snapshot)] if !snapshot.focused));
    assert!(s.observe_focus(false).unwrap().is_empty());
}

#[test]
fn revision_exhaustion_cannot_partially_mutate_or_invoke_focus() {
    let mut s = state(Mode::Single);
    s.revision = i64::MAX - 1;
    let before = s.snapshot();
    assert_eq!(
        s.native(Action::Activate(date(5)), Access::Allowed),
        Err(Error::LimitExceeded)
    );
    assert_eq!(s.snapshot(), before);
    applied(command(
        &mut s,
        Command::SetPresentation(Presentation::Years),
    ));
    assert_eq!(s.snapshot().revision, i64::MAX);
    let before = s.snapshot();
    assert_eq!(
        s.execute(&Command::Focus, || panic!(
            "focus before capacity reservation"
        ))
        .response,
        Response::Failed(Error::LimitExceeded)
    );
    assert_eq!(s.observe_focus(true), Err(Error::LimitExceeded));
    let mut c = (*config(Mode::Single)).clone();
    c.label = "Changed".into();
    assert_eq!(s.configure(Arc::new(c)), Err(Error::LimitExceeded));
    assert_eq!(s.snapshot(), before);
    assert_eq!(applied(command(&mut s, Command::ReadSnapshot)), before);
}
