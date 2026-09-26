use gpuio_native::{
    mailbox::{MAX_INPUT_BYTES, MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, calendar_input as c, v1::*};
use std::sync::Arc;

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn date(day: i64) -> c::Date {
    c::Date::from_ymd(2024, 2, day).unwrap()
}
fn month() -> c::Month {
    c::Month::from_date(date(1))
}
fn config() -> c::Config {
    c::Config {
        mode: c::Mode::Single,
        constraints: c::Constraints::unrestricted(),
        first_weekday: 1,
        labels: c::Labels::english(),
        today: Some(date(29)),
        label: "Calendar".into(),
        disabled: false,
        read_only: false,
        auto_focus: false,
    }
}
fn snapshot(revision: i64) -> c::Snapshot {
    c::Snapshot {
        revision,
        mode: c::Mode::Single,
        selection: c::Selection::Single(date(29)),
        selection_allowed: true,
        month: month(),
        focused_date: date(29),
        presentation: c::Presentation::Days,
        focused: false,
    }
}
fn event(event: c::Event) -> Event {
    Event::CalendarEvent(window(), node(), handler(), 1, event)
}
fn pair(revision: i64) -> [Event; 2] {
    [
        event(c::Event::Changed(snapshot(revision))),
        event(c::Event::Selected(snapshot(revision + 1))),
    ]
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn create() -> Op {
    Op::Create(node(), Kind::Calendar, "".into(), Some(handler()))
}
fn set(config: c::Config, initial: c::Selection) -> Op {
    Op::SetCalendar(node(), Box::new(config), initial, month())
}
fn session() -> Session {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "Calendar", 320., 300.).unwrap();
    s
}
fn mount(s: &mut Session) {
    s.apply(&tx(
        0,
        vec![
            create(),
            set(config(), c::Selection::Single(date(29))),
            Op::SetRoot(Some(node())),
        ],
    ))
    .unwrap();
}

#[test]
fn calendar_admission_is_atomic_and_configuration_preserves_historical_seeds() {
    let mut s = session();
    let disabled = c::Constraints::new(
        c::Date::MIN,
        c::Date::MAX,
        vec![date(29)],
        vec![],
        vec![],
        c::RangePolicy::EveryDay,
    )
    .unwrap();
    for ops in [
        vec![create()],
        vec![
            Op::Create(node(), Kind::Calendar, "".into(), None),
            set(config(), c::Selection::Empty),
        ],
        vec![
            Op::Create(node(), Kind::Container, "".into(), Some(handler())),
            set(config(), c::Selection::Empty),
        ],
        vec![create(), set(config(), c::Selection::RangeStart(date(29)))],
        vec![
            create(),
            set(
                c::Config {
                    constraints: disabled.clone(),
                    ..config()
                },
                c::Selection::Single(date(29)),
            ),
        ],
    ] {
        assert!(s.apply(&tx(0, ops)).is_err());
        assert_eq!(s.retained_bytes(), 0);
        assert_eq!(s.tree(window()).unwrap().revision(), 0);
    }
    mount(&mut s);
    let retained = s.retained_bytes();
    assert_eq!(retained, 32 + config().retained_bytes());
    let old_config = Arc::downgrade(
        &s.tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .calendar
            .as_ref()
            .unwrap()
            .config,
    );
    let child = NodeId::from_parts(1, 1).unwrap();
    for ops in [
        vec![set(
            c::Config {
                mode: c::Mode::Range,
                ..config()
            },
            c::Selection::Empty,
        )],
        vec![set(
            c::Config {
                label: "".into(),
                ..config()
            },
            c::Selection::Empty,
        )],
        vec![set(config(), c::Selection::RangeStart(date(1)))],
        vec![Op::Bind(node(), None)],
        vec![
            set(
                c::Config {
                    label: "Changed".into(),
                    ..config()
                },
                c::Selection::Empty,
            ),
            Op::SetText(node(), "bad leaf".into()),
        ],
        vec![
            Op::Create(child, Kind::Text, "child".into(), None),
            Op::Splice(node(), 0, 0, vec![child]),
        ],
    ] {
        assert!(s.apply(&tx(1, ops)).is_err());
        assert_eq!(s.retained_bytes(), retained);
        assert_eq!(s.tree(window()).unwrap().revision(), 1);
        let mounted = s
            .tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .calendar
            .as_ref()
            .unwrap();
        assert_eq!(*mounted.config, config());
        assert_eq!(mounted.initial, c::Selection::Single(date(29)));
        assert_eq!(mounted.initial_month, month());
    }
    let next_config = c::Config {
        constraints: disabled,
        label: "History".into(),
        disabled: true,
        read_only: true,
        ..config()
    };
    s.apply(&tx(
        1,
        vec![Op::SetCalendar(
            node(),
            Box::new(next_config.clone()),
            c::Selection::Single(date(1)),
            c::Month::new(2030, 1).unwrap(),
        )],
    ))
    .unwrap();
    assert!(old_config.upgrade().is_none());
    assert_eq!(s.retained_bytes(), 32 + next_config.retained_bytes());
    let mounted = s
        .tree(window())
        .unwrap()
        .get(node())
        .unwrap()
        .calendar
        .as_ref()
        .unwrap();
    assert_eq!(mounted.initial, c::Selection::Single(date(29)));
    assert_eq!(mounted.initial_month, month());
    assert!(
        !mounted
            .config
            .constraints
            .allows_selection(mounted.initial, mounted.config.mode)
    );
    s.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node())]))
        .unwrap();
    assert_eq!(s.retained_bytes(), 0);
    let fresh = NodeId::from_parts(0, 2).unwrap();
    s.apply(&tx(
        3,
        vec![
            Op::Create(fresh, Kind::Calendar, "".into(), Some(handler())),
            Op::SetCalendar(
                fresh,
                Box::new(c::Config {
                    mode: c::Mode::Range,
                    ..config()
                }),
                c::Selection::RangeStart(date(1)),
                month(),
            ),
            Op::SetRoot(Some(fresh)),
        ],
    ))
    .unwrap();
    s.close(window()).unwrap();
    assert_eq!(s.retained_bytes(), 0);
}

#[test]
fn calendar_routes_fence_leases_modes_and_invalid_events_but_allow_cleanup() {
    let mut s = session();
    mount(&mut s);
    let sample = c::Event::Changed(snapshot(1));
    assert_eq!(
        s.calendar_event(window(), node(), handler(), 1, sample.clone()),
        Some(event(sample.clone()))
    );
    assert!(s.press(window(), node(), handler(), 1).is_none());
    for (w, n, h, r) in [
        (WindowId::from_parts(0, 2).unwrap(), node(), handler(), 1),
        (window(), NodeId::from_parts(0, 2).unwrap(), handler(), 1),
        (window(), node(), HandlerId::from_parts(0, 2).unwrap(), 1),
        (window(), node(), handler(), -1),
        (window(), node(), handler(), 2),
    ] {
        assert!(s.calendar_event(w, n, h, r, sample.clone()).is_none());
    }
    for invalid in [
        c::Event::Changed(snapshot(0)),
        c::Event::Selected(c::Snapshot {
            selection_allowed: false,
            ..snapshot(2)
        }),
        c::Event::Observed(c::Snapshot {
            mode: c::Mode::Range,
            selection: c::Selection::Empty,
            ..snapshot(2)
        }),
        c::Event::Observed(c::Snapshot {
            month: c::Month::new(2024, 3).unwrap(),
            ..snapshot(2)
        }),
    ] {
        assert!(
            s.calendar_event(window(), node(), handler(), 1, invalid)
                .is_none()
        );
    }
    s.apply(&tx(
        1,
        vec![set(
            c::Config {
                disabled: true,
                read_only: true,
                ..config()
            },
            c::Selection::Empty,
        )],
    ))
    .unwrap();
    assert!(
        s.calendar_event(
            window(),
            node(),
            handler(),
            1,
            c::Event::Observed(snapshot(2))
        )
        .is_some()
    );
    let next = HandlerId::from_parts(0, 2).unwrap();
    s.apply(&tx(2, vec![Op::Bind(node(), Some(next))])).unwrap();
    assert!(
        s.calendar_event(window(), node(), handler(), 1, sample.clone())
            .is_none()
    );
    assert!(
        s.calendar_event(window(), node(), next, 3, sample.clone())
            .is_some()
    );
    assert!(s.overload(window()));
    assert!(
        s.calendar_event(window(), node(), next, 3, sample.clone())
            .is_none()
    );
    s.close(window()).unwrap();
    assert!(
        s.calendar_event(window(), node(), next, 3, sample)
            .is_none()
    );
}

#[test]
fn calendar_completion_count_admission_and_validation_never_publish_half_a_pair() {
    let mut mailbox = Mailbox::default();
    let observed = event(c::Event::Observed(snapshot(0)));
    for _ in 0..MAX_INPUT_EVENTS - 1 {
        mailbox.input(observed.clone()).unwrap();
    }
    assert!(mailbox.calendar_completion(pair(1)).is_err());
    assert_eq!(
        mailbox.drain(128),
        vec![observed.clone(); MAX_INPUT_EVENTS - 1]
    );
    for _ in 0..MAX_INPUT_EVENTS - 2 {
        mailbox.input(observed.clone()).unwrap();
    }
    mailbox.calendar_completion(pair(1)).unwrap();
    assert!(mailbox.has_window_output(window().slot()));
    let drained = mailbox.drain(128);
    assert_eq!(drained.len(), MAX_INPUT_EVENTS);
    assert_eq!(&drained[MAX_INPUT_EVENTS - 2..], &pair(1));
    assert!(!mailbox.has_window_output(window().slot()));
    let selected = |s| event(c::Event::Selected(s));
    for invalid in [
        [pair(1)[1].clone(), pair(1)[0].clone()],
        [pair(1)[0].clone(), selected(snapshot(3))],
        [
            pair(1)[0].clone(),
            selected(c::Snapshot {
                focused: true,
                ..snapshot(2)
            }),
        ],
        [
            pair(1)[0].clone(),
            selected(c::Snapshot {
                selection_allowed: false,
                ..snapshot(2)
            }),
        ],
        [
            pair(1)[0].clone(),
            selected(c::Snapshot {
                selection: c::Selection::Empty,
                ..snapshot(2)
            }),
        ],
        [
            pair(1)[0].clone(),
            Event::CalendarEvent(
                window(),
                node(),
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                c::Event::Selected(snapshot(2)),
            ),
        ],
        [event(c::Event::Changed(snapshot(0))), selected(snapshot(1))],
        [
            event(c::Event::Changed(snapshot(i64::MAX))),
            selected(snapshot(1)),
        ],
    ] {
        mailbox.input(observed.clone()).unwrap();
        assert!(mailbox.calendar_completion(invalid).is_err());
        assert_eq!(
            mailbox.drain(128).as_slice(),
            std::slice::from_ref(&observed)
        );
    }
    mailbox.close();
    assert!(mailbox.calendar_completion(pair(1)).is_err());
}

#[test]
fn calendar_completion_byte_admission_and_discrete_selection_order() {
    let mut mailbox = Mailbox::default();
    let filler = |len| {
        Event::EditorEvent(
            window(),
            node(),
            handler(),
            1,
            EditorEventKind::Submitted,
            EditorSnapshot {
                revision: 1,
                text: "a".repeat(len),
                selection: EditorSelection { anchor: 0, head: 0 },
                composition: None,
                focused: false,
            },
        )
    };
    let mut remaining = MAX_INPUT_BYTES;
    while remaining > MAX_TEXT_BYTES + 256 {
        mailbox.input(filler(MAX_TEXT_BYTES)).unwrap();
        remaining -= MAX_TEXT_BYTES + 256;
    }
    mailbox.input(filler(remaining - 256 - 511)).unwrap();
    assert!(mailbox.calendar_completion(pair(1)).is_err());
    while mailbox.has_output() {
        let drained = mailbox.drain(128);
        assert!(!drained.is_empty());
        assert!(drained.iter().all(|e| matches!(e, Event::EditorEvent(..))));
    }
    assert!(!mailbox.has_window_output(window().slot()));
    // A partial range and its completion are distinct boundaries. Navigation and
    // focus observations are also discrete until a separate coalescing contract exists.
    let partial = event(c::Event::Changed(c::Snapshot {
        mode: c::Mode::Range,
        selection: c::Selection::RangeStart(date(1)),
        ..snapshot(1)
    }));
    mailbox.input(partial.clone()).unwrap();
    let complete = c::Snapshot {
        mode: c::Mode::Range,
        selection: c::Selection::Range(c::Range::new(date(1), date(29)).unwrap()),
        ..snapshot(2)
    };
    let completed = [
        event(c::Event::Changed(complete.clone())),
        event(c::Event::Selected(c::Snapshot {
            revision: 3,
            ..complete.clone()
        })),
    ];
    let observed = event(c::Event::Observed(c::Snapshot {
        revision: 4,
        ..complete
    }));
    mailbox.calendar_completion(completed.clone()).unwrap();
    mailbox.input(observed.clone()).unwrap();
    let mut expected = vec![partial];
    expected.extend(completed);
    expected.push(observed);
    assert_eq!(mailbox.drain(128), expected);
}

#[test]
fn correlated_calendar_reply_reservation_and_window_retirement_barrier() {
    let mut mailbox = Mailbox::default();
    let request = Message::CalendarCommand(9, window(), node(), c::Command::ReadSnapshot);
    mailbox.submit(request.clone(), 7).unwrap();
    assert_eq!(mailbox.pop(), Some(request));
    let reply = Event::CalendarResult(9, window(), node(), c::Response::Applied(snapshot(1)));
    mailbox.input(Event::Rendered(window(), 1)).unwrap();
    mailbox.respond(reply.clone());
    mailbox.input(Event::Rendered(window(), 2)).unwrap();
    assert_eq!(mailbox.drain(1), vec![Event::Rendered(window(), 1)]);
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(mailbox.drain(1), vec![reply]);
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(mailbox.drain(1), vec![Event::Rendered(window(), 2)]);
    assert!(!mailbox.has_window_output(window().slot()));
    // Pending responses alone must prevent slot reuse, including failed commands.
    mailbox
        .submit(
            Message::CalendarCommand(10, window(), node(), c::Command::Focus),
            7,
        )
        .unwrap();
    mailbox.pop().unwrap();
    let failed = Event::CalendarResult(
        10,
        window(),
        node(),
        c::Response::Failed(c::Error::FocusBlocked),
    );
    mailbox.respond(failed.clone());
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(mailbox.drain(128), vec![failed]);
    assert!(!mailbox.has_window_output(window().slot()));
}

#[test]
fn form_metadata_updates_preserve_the_retained_seed_and_reject_role_overrides() {
    use gpuio_protocol::accessibility as a;
    let mut s = session();
    mount(&mut s);
    let metadata = a::Config {
        role: None,
        label: None,
        description: None,
        live: a::Live::Off,
        current: None,
        field: Some(a::Field {
            label: "Selection".into(),
            help: Some("Choose a value".into()),
            error: Some("Required by this form".into()),
            required: true,
        }),
    };
    s.apply(&tx(
        1,
        vec![Op::SetAccessibility(node(), Some(metadata.clone()))],
    ))
    .unwrap();
    assert_eq!(
        s.tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .accessibility
            .as_deref(),
        Some(&metadata)
    );
    let mut invalid = metadata.clone();
    invalid.role = Some(a::Role::Link);
    assert!(
        s.apply(&tx(2, vec![Op::SetAccessibility(node(), Some(invalid))]))
            .is_err()
    );
    assert_eq!(s.tree(window()).unwrap().revision(), 2);
    assert_eq!(
        s.tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .accessibility
            .as_deref(),
        Some(&metadata)
    );
}
