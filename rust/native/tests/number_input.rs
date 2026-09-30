use gpuio_native::{
    mailbox::{MAX_INPUT_BYTES, MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, number_input as n, numeric::Domain, v1::*};
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn config() -> n::Config {
    n::Config {
        domain: Domain::new(-2., 8., 0.5).unwrap(),
        label: "Value".into(),
        placeholder: "".into(),
        increment_label: "Increase".into(),
        decrement_label: "Decrease".into(),
        step_controls: n::StepControls::Sides,
        allow_empty: false,
        disabled: false,
        read_only: false,
        auto_focus: false,
    }
}
fn snapshot(revision: i64, draft: &str) -> n::Snapshot {
    n::Snapshot {
        revision,
        domain: config().domain,
        draft: draft.into(),
        committed: n::Value::Number(1.),
        selection: n::Selection { anchor: 0, head: 0 },
        composition: None,
        focused: false,
    }
}
fn event(event: n::Event) -> Event {
    Event::NumberInputEvent(window(), node(), handler(), 1, event)
}
fn changed(revision: i64, draft: &str) -> Event {
    event(n::Event::Changed(snapshot(revision, draft)))
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn session() -> Session {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "Numbers", 320., 200.).unwrap();
    session
}
fn mount(session: &mut Session) {
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(), Kind::NumberInput, "".into(), Some(handler())),
                Op::SetNumberInput(node(), config(), n::Value::Number(1.)),
                Op::SetRoot(Some(node())),
            ],
        ))
        .unwrap();
}
#[test]
fn admission_rollback_configuration_accounting_and_disposal_are_atomic() {
    let mut session = session();
    for ops in [
        vec![Op::Create(
            node(),
            Kind::NumberInput,
            "".into(),
            Some(handler()),
        )],
        vec![
            Op::Create(node(), Kind::NumberInput, "".into(), None),
            Op::SetNumberInput(node(), config(), n::Value::Empty),
        ],
        vec![
            Op::Create(node(), Kind::Container, "".into(), Some(handler())),
            Op::SetNumberInput(node(), config(), n::Value::Empty),
        ],
    ] {
        assert!(session.apply(&tx(0, ops)).is_err());
        assert_eq!(session.retained_bytes(), 0);
        assert_eq!(session.tree(window()).unwrap().revision(), 0);
    }
    mount(&mut session);
    let retained = session.retained_bytes();
    assert_eq!(retained, config().retained_bytes());
    let old = std::sync::Arc::downgrade(
        &session
            .tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .number_input
            .as_ref()
            .unwrap()
            .config,
    );
    for ops in [
        vec![Op::SetNumberInput(
            node(),
            config(),
            n::Value::Number(f64::NAN),
        )],
        vec![Op::SetNumberInput(
            node(),
            n::Config {
                label: "".into(),
                ..config()
            },
            n::Value::Empty,
        )],
        vec![Op::Bind(node(), None)],
        vec![
            Op::SetNumberInput(
                node(),
                n::Config {
                    placeholder: "longer".into(),
                    ..config()
                },
                n::Value::Empty,
            ),
            Op::SetText(node(), "invalid leaf text".into()),
        ],
        vec![
            Op::Create(
                NodeId::from_parts(1, 1).unwrap(),
                Kind::Text,
                "child".into(),
                None,
            ),
            Op::Splice(node(), 0, 0, vec![NodeId::from_parts(1, 1).unwrap()]),
        ],
    ] {
        assert!(session.apply(&tx(1, ops)).is_err());
        assert_eq!(session.retained_bytes(), retained);
        assert_eq!(session.tree(window()).unwrap().revision(), 1);
        assert_eq!(
            *session
                .tree(window())
                .unwrap()
                .get(node())
                .unwrap()
                .number_input
                .as_ref()
                .unwrap()
                .config,
            config()
        );
    }
    session
        .apply(&tx(
            1,
            vec![Op::SetNumberInput(
                node(),
                n::Config {
                    placeholder: "longer".into(),
                    ..config()
                },
                n::Value::Empty,
            )],
        ))
        .unwrap();
    assert!(old.upgrade().is_none());
    assert_eq!(session.retained_bytes(), retained + 6);
    session
        .apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node())]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
#[test]
fn routing_fences_generations_and_malformed_events_but_keeps_historical_domains() {
    let mut session = session();
    mount(&mut session);
    let route = |session: &Session, w, n, h, rev, e| session.number_input_event(w, n, h, rev, e);
    let sample = n::Event::Changed(snapshot(1, "1e-"));
    assert_eq!(
        route(&session, window(), node(), handler(), 1, sample.clone()),
        Some(event(sample.clone()))
    );
    assert!(session.press(window(), node(), handler(), 1).is_none());
    for (w, n, h, rev) in [
        (WindowId::from_parts(0, 2).unwrap(), node(), handler(), 1),
        (window(), NodeId::from_parts(0, 2).unwrap(), handler(), 1),
        (window(), node(), HandlerId::from_parts(0, 2).unwrap(), 1),
        (window(), node(), handler(), -1),
        (window(), node(), handler(), 2),
    ] {
        assert!(route(&session, w, n, h, rev, sample.clone()).is_none());
    }
    for bad in [
        n::Event::Changed(snapshot(0, "1e-")),
        n::Event::Committed(n::Source::Keyboard, snapshot(2, "-")),
        n::Event::Observed(n::Snapshot {
            selection: n::Selection { anchor: 1, head: 1 },
            ..snapshot(2, "é")
        }),
    ] {
        assert!(route(&session, window(), node(), handler(), 1, bad).is_none());
    }
    session
        .apply(&tx(
            1,
            vec![Op::SetNumberInput(
                node(),
                n::Config {
                    domain: Domain::new(0., 0.5, 0.1).unwrap(),
                    disabled: true,
                    read_only: true,
                    ..config()
                },
                n::Value::Empty,
            )],
        ))
        .unwrap();
    let old_domain = n::Event::Committed(n::Source::Keyboard, snapshot(2, "1"));
    assert_eq!(
        route(&session, window(), node(), handler(), 1, old_domain.clone()),
        Some(event(old_domain.clone()))
    );
    assert!(session.overload(window()));
    assert!(route(&session, window(), node(), handler(), 1, old_domain.clone()).is_none());
    session.close(window()).unwrap();
    assert_eq!(session.retained_bytes(), 0);
    assert!(route(&session, window(), node(), handler(), 1, old_domain).is_none());
}
#[test]
fn changes_coalesce_only_with_adjacent_matching_owner_domain_and_committed_value() {
    let mut mailbox = Mailbox::default();
    let observed = event(n::Event::Observed(snapshot(0, "1")));
    mailbox.input(observed.clone()).unwrap();
    for revision in 1..=10_000 {
        mailbox.input(changed(revision, "1e-")).unwrap();
    }
    let committed = event(n::Event::Committed(
        n::Source::Keyboard,
        snapshot(10_001, "1"),
    ));
    mailbox.input(committed.clone()).unwrap();
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(
        mailbox.drain(128),
        [observed, changed(10_000, "1e-"), committed]
    );
    assert!(!mailbox.has_window_output(window().slot()));
    for barrier in [
        event(n::Event::Observed(snapshot(2, "1"))),
        event(n::Event::Committed(n::Source::Keyboard, snapshot(2, "1"))),
        event(n::Event::Cancelled(
            n::CancelReason::Escape,
            snapshot(2, "1"),
        )),
        event(n::Event::Rejected(
            n::Rejection::Incomplete,
            snapshot(2, "1e-"),
        )),
        Event::Rendered(window(), 1),
    ] {
        mailbox.input(changed(1, "1")).unwrap();
        mailbox.input(barrier.clone()).unwrap();
        mailbox.input(changed(3, "12")).unwrap();
        assert_eq!(
            mailbox.drain(128),
            [changed(1, "1"), barrier, changed(3, "12")]
        );
    }
    for next in [
        Event::NumberInputEvent(
            WindowId::from_parts(0, 2).unwrap(),
            node(),
            handler(),
            1,
            n::Event::Changed(snapshot(2, "1")),
        ),
        Event::NumberInputEvent(
            window(),
            NodeId::from_parts(0, 2).unwrap(),
            handler(),
            1,
            n::Event::Changed(snapshot(2, "1")),
        ),
        Event::NumberInputEvent(
            window(),
            node(),
            HandlerId::from_parts(0, 2).unwrap(),
            1,
            n::Event::Changed(snapshot(2, "1")),
        ),
        Event::NumberInputEvent(
            window(),
            node(),
            handler(),
            2,
            n::Event::Changed(snapshot(2, "1")),
        ),
        changed(1, "1"),
        event(n::Event::Changed(n::Snapshot {
            domain: Domain::new(0., 8., 1.).unwrap(),
            ..snapshot(2, "1")
        })),
        event(n::Event::Changed(n::Snapshot {
            committed: n::Value::Number(2.),
            ..snapshot(2, "1")
        })),
    ] {
        mailbox.input(changed(1, "1")).unwrap();
        mailbox.input(next.clone()).unwrap();
        assert_eq!(mailbox.drain(128), [changed(1, "1"), next]);
    }
    mailbox.input(changed(1, "1")).unwrap();
    mailbox.submit(Message::Hello(VERSION, 0), 2).unwrap();
    mailbox.pop().unwrap();
    mailbox.respond(Event::Failed(1, ErrorCode::NotReady));
    mailbox.input(changed(3, "12")).unwrap();
    assert_eq!(
        mailbox.drain(128),
        [
            changed(1, "1"),
            Event::Failed(1, ErrorCode::NotReady),
            changed(3, "12")
        ]
    );
}
#[test]
fn coalescing_accounts_for_variable_draft_bytes_and_preserves_discrete_backpressure() {
    let mut mailbox = Mailbox::default();
    for _ in 0..MAX_INPUT_EVENTS - 1 {
        mailbox
            .input(event(n::Event::Observed(snapshot(0, "1"))))
            .unwrap();
    }
    mailbox.input(changed(1, "1")).unwrap();
    mailbox.input(changed(2, "12")).unwrap();
    let commit = event(n::Event::Committed(n::Source::Keyboard, snapshot(3, "1")));
    assert!(mailbox.input(commit.clone()).is_err());
    assert_eq!(mailbox.drain(128).last(), Some(&changed(2, "12")));
    mailbox.input(commit.clone()).unwrap();
    assert_eq!(mailbox.drain(128), [commit]);
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
    mailbox.input(filler(remaining - 256 - 257 - 100)).unwrap();
    mailbox.input(changed(1, "x")).unwrap();
    // Submitted editor events are discrete; Changed fillers would coalesce.
    assert!(mailbox.input(Event::Rendered(window(), 2)).is_err());
    // Replacing an adjacent event must charge growth, not just its old size.
    assert!(mailbox.input(changed(2, &"x".repeat(4096))).is_err());
    let mut drained = Vec::new();
    while mailbox.has_window_output(window().slot()) {
        let batch = mailbox.drain(128);
        assert!(!batch.is_empty());
        drained.extend(batch);
    }
    assert_eq!(drained.last(), Some(&changed(1, "x")));
    mailbox.input(changed(2, &"x".repeat(4096))).unwrap();
    assert_eq!(mailbox.drain(128), [changed(2, &"x".repeat(4096))]);
}

#[test]
fn draft_mount_metadata_is_bounded_accounted_and_atomic() {
    let mut session = session();
    mount(&mut session);
    let base = session.tree(window()).unwrap().revision();
    let bytes_before = session.tree(window()).unwrap().retained_bytes();
    session
        .apply(&tx(
            base,
            vec![Op::SetNumberInputDraft(node(), Some("1e-".into()))],
        ))
        .unwrap();
    let tree = session.tree(window()).unwrap();
    assert_eq!(
        tree.get(node())
            .unwrap()
            .number_input
            .as_ref()
            .unwrap()
            .initial_draft
            .as_deref(),
        Some("1e-")
    );
    assert_eq!(tree.retained_bytes(), bytes_before + 3);
    let revision = tree.revision();
    for invalid in ["\0".to_owned(), "bad\n".into(), "x".repeat(4097)] {
        assert!(
            session
                .apply(&tx(
                    revision,
                    vec![
                        Op::SetNumberInputDraft(node(), Some("replacement".into())),
                        Op::SetNumberInputDraft(node(), Some(invalid)),
                    ]
                ))
                .is_err()
        );
        let tree = session.tree(window()).unwrap();
        assert_eq!(tree.revision(), revision);
        assert_eq!(tree.retained_bytes(), bytes_before + 3);
        assert_eq!(
            tree.get(node())
                .unwrap()
                .number_input
                .as_ref()
                .unwrap()
                .initial_draft
                .as_deref(),
            Some("1e-")
        );
    }
    session
        .apply(&tx(revision, vec![Op::SetNumberInputDraft(node(), None)]))
        .unwrap();
    assert_eq!(
        session.tree(window()).unwrap().retained_bytes(),
        bytes_before
    );
}
