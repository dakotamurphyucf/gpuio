use gpuio_native::{
    mailbox::{MAX_INPUT_BYTES, MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, otp_input as o, v1::*};
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
fn config() -> o::Config {
    o::Config {
        policy: o::Policy::new(6, o::Alphabet::Digits).unwrap(),
        label: "Code".into(),
        masked: false,
        disabled: false,
        read_only: false,
        auto_focus: false,
    }
}
fn snapshot(revision: i64, value: &str) -> o::Snapshot {
    o::Snapshot {
        revision,
        policy: config().policy,
        value: value.into(),
        draft: value.into(),
        selection: o::Selection {
            anchor: value.len() as i64,
            head: value.len() as i64,
        },
        composition: None,
        focused: false,
        can_undo: false,
        can_redo: false,
    }
}
fn event(event: o::Event) -> Event {
    Event::OtpInputEvent(window(), node(), handler(), 1, event)
}
fn changed(revision: i64, value: &str) -> Event {
    event(o::Event::Changed(snapshot(revision, value)))
}
fn pair(revision: i64) -> [Event; 2] {
    [
        changed(revision, "123456"),
        event(o::Event::Complete(snapshot(revision + 1, "123456"))),
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
fn session() -> Session {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "OTP", 320., 200.).unwrap();
    session
}
fn mount(session: &mut Session) {
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(), Kind::OtpInput, "".into(), Some(handler())),
                Op::SetOtpInput(node(), config(), "12".into()),
                Op::SetRoot(Some(node())),
            ],
        ))
        .unwrap();
}

#[test]
fn retained_admission_policy_seed_accounting_and_disposal() {
    let mut s = session();
    for ops in [
        vec![Op::Create(
            node(),
            Kind::OtpInput,
            "".into(),
            Some(handler()),
        )],
        vec![
            Op::Create(node(), Kind::OtpInput, "".into(), None),
            Op::SetOtpInput(node(), config(), "".into()),
        ],
        vec![
            Op::Create(node(), Kind::Container, "".into(), Some(handler())),
            Op::SetOtpInput(node(), config(), "".into()),
        ],
        vec![
            Op::Create(node(), Kind::OtpInput, "".into(), Some(handler())),
            Op::SetOtpInput(node(), config(), "AB".into()),
        ],
    ] {
        assert!(s.apply(&tx(0, ops)).is_err());
        assert_eq!(s.retained_bytes(), 0);
        assert_eq!(s.tree(window()).unwrap().revision(), 0);
    }
    mount(&mut s);
    let retained = s.retained_bytes();
    assert_eq!(retained, config().retained_bytes() + 2);
    let (old_config, seed) = {
        let mount = s
            .tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .otp_input
            .as_ref()
            .unwrap();
        (
            Arc::downgrade(&mount.config),
            Arc::downgrade(&mount.initial),
        )
    };
    for ops in [
        vec![Op::SetOtpInput(node(), config(), "1234567".into())],
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                label: "".into(),
                ..config()
            },
            "".into(),
        )],
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                policy: o::Policy::new(5, o::Alphabet::Digits).unwrap(),
                ..config()
            },
            "".into(),
        )],
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                policy: o::Policy::new(6, o::Alphabet::AsciiAlphanumeric).unwrap(),
                ..config()
            },
            "".into(),
        )],
        vec![Op::Bind(node(), None)],
        vec![
            Op::SetOtpInput(
                node(),
                o::Config {
                    label: "Changed".into(),
                    ..config()
                },
                "45".into(),
            ),
            Op::SetText(node(), "bad leaf".into()),
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
        assert!(s.apply(&tx(1, ops)).is_err());
        assert_eq!(s.retained_bytes(), retained);
        assert_eq!(s.tree(window()).unwrap().revision(), 1);
        let mount = s
            .tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .otp_input
            .as_ref()
            .unwrap();
        assert_eq!(*mount.config, config());
        assert_eq!(&*mount.initial, "12");
    }
    s.apply(&tx(
        1,
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                label: "New code".into(),
                masked: true,
                disabled: true,
                read_only: true,
                ..config()
            },
            "654321".into(),
        )],
    ))
    .unwrap();
    assert!(old_config.upgrade().is_none());
    assert!(seed.upgrade().is_some());
    assert_eq!(s.retained_bytes(), retained + 4);
    assert_eq!(
        &*s.tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .otp_input
            .as_ref()
            .unwrap()
            .initial,
        "12"
    );
    s.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node())]))
        .unwrap();
    assert_eq!(s.retained_bytes(), 0);
    assert!(seed.upgrade().is_none());
    let fresh = NodeId::from_parts(0, 2).unwrap();
    s.apply(&tx(
        3,
        vec![
            Op::Create(
                fresh,
                Kind::OtpInput,
                "".into(),
                Some(HandlerId::from_parts(0, 2).unwrap()),
            ),
            Op::SetOtpInput(
                fresh,
                o::Config {
                    policy: o::Policy::new(4, o::Alphabet::AsciiAlphanumeric).unwrap(),
                    ..config()
                },
                "Ab12".into(),
            ),
            Op::SetRoot(Some(fresh)),
        ],
    ))
    .unwrap();
    assert_eq!(
        &*s.tree(window())
            .unwrap()
            .get(fresh)
            .unwrap()
            .otp_input
            .as_ref()
            .unwrap()
            .initial,
        "Ab12"
    );
    s.close(window()).unwrap();
    assert_eq!(s.retained_bytes(), 0);
}

#[test]
fn routing_fences_identity_policy_and_shape_without_dropping_disabled_cleanup() {
    let mut s = session();
    mount(&mut s);
    let sample = o::Event::Changed(snapshot(1, "12"));
    let route = |s: &Session, w, n, h, r, e| s.otp_input_event(w, n, h, r, e);
    assert_eq!(
        route(&s, window(), node(), handler(), 1, sample.clone()),
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
        assert!(route(&s, w, n, h, r, sample.clone()).is_none());
    }
    for invalid in [
        o::Event::Changed(snapshot(0, "12")),
        o::Event::Complete(snapshot(2, "12")),
        o::Event::Observed(o::Snapshot {
            policy: o::Policy::new(6, o::Alphabet::AsciiAlphanumeric).unwrap(),
            ..snapshot(2, "12")
        }),
    ] {
        assert!(route(&s, window(), node(), handler(), 1, invalid).is_none());
    }
    s.apply(&tx(
        1,
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                disabled: true,
                read_only: true,
                ..config()
            },
            "".into(),
        )],
    ))
    .unwrap();
    assert!(
        route(
            &s,
            window(),
            node(),
            handler(),
            1,
            o::Event::Rejected(o::InputError::TooLong, snapshot(2, "12"))
        )
        .is_some()
    );
    let next_handler = HandlerId::from_parts(0, 2).unwrap();
    s.apply(&tx(2, vec![Op::Bind(node(), Some(next_handler))]))
        .unwrap();
    assert!(route(&s, window(), node(), handler(), 1, sample.clone()).is_none());
    assert!(route(&s, window(), node(), next_handler, 3, sample.clone()).is_some());
    assert!(s.overload(window()));
    assert!(route(&s, window(), node(), next_handler, 3, sample.clone()).is_none());
    s.close(window()).unwrap();
    assert!(route(&s, window(), node(), next_handler, 3, sample).is_none());
}

#[test]
fn coalescing_respects_routes_policies_and_semantic_response_boundaries() {
    let mut mailbox = Mailbox::default();
    let observed = event(o::Event::Observed(snapshot(0, "")));
    mailbox.input(observed.clone()).unwrap();
    for revision in 1..=10_000 {
        mailbox.input(changed(revision, "12")).unwrap();
    }
    mailbox.otp_completion(pair(10_001)).unwrap();
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(
        mailbox.drain(128),
        [observed, pair(10_001)[0].clone(), pair(10_001)[1].clone()]
    );
    assert!(!mailbox.has_window_output(window().slot()));
    for barrier in [
        event(o::Event::Observed(snapshot(2, "12"))),
        event(o::Event::Complete(snapshot(2, "123456"))),
        event(o::Event::Rejected(
            o::InputError::TooLong,
            snapshot(2, "12"),
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
        Event::OtpInputEvent(
            WindowId::from_parts(0, 2).unwrap(),
            node(),
            handler(),
            1,
            o::Event::Changed(snapshot(2, "1")),
        ),
        Event::OtpInputEvent(
            window(),
            NodeId::from_parts(0, 2).unwrap(),
            handler(),
            1,
            o::Event::Changed(snapshot(2, "1")),
        ),
        Event::OtpInputEvent(
            window(),
            node(),
            HandlerId::from_parts(0, 2).unwrap(),
            1,
            o::Event::Changed(snapshot(2, "1")),
        ),
        Event::OtpInputEvent(
            window(),
            node(),
            handler(),
            2,
            o::Event::Changed(snapshot(2, "1")),
        ),
        changed(1, "1"),
        event(o::Event::Changed(o::Snapshot {
            policy: o::Policy::new(6, o::Alphabet::AsciiAlphanumeric).unwrap(),
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
fn completion_pairs_have_atomic_count_admission_and_validate_the_entire_pair() {
    let mut mailbox = Mailbox::default();
    for _ in 0..MAX_INPUT_EVENTS - 1 {
        mailbox
            .input(event(o::Event::Observed(snapshot(0, ""))))
            .unwrap();
    }
    assert!(mailbox.otp_completion(pair(1)).is_err());
    assert_eq!(mailbox.drain(128).len(), MAX_INPUT_EVENTS - 1);
    for _ in 0..MAX_INPUT_EVENTS - 2 {
        mailbox
            .input(event(o::Event::Observed(snapshot(0, ""))))
            .unwrap();
    }
    mailbox.input(changed(1, "1")).unwrap();
    mailbox.otp_completion(pair(2)).unwrap(); // replacing the tail saves one slot
    let drained = mailbox.drain(128);
    assert_eq!(drained.len(), MAX_INPUT_EVENTS);
    assert_eq!(&drained[MAX_INPUT_EVENTS - 2..], &pair(2));
    for _ in 0..MAX_INPUT_EVENTS - 1 {
        mailbox
            .input(event(o::Event::Observed(snapshot(0, ""))))
            .unwrap();
    }
    mailbox.input(changed(1, "1")).unwrap();
    assert!(mailbox.otp_completion(pair(2)).is_err());
    assert_eq!(mailbox.drain(128).last(), Some(&changed(1, "1")));
    for invalid in [
        [pair(2)[1].clone(), pair(2)[0].clone()],
        [
            changed(2, "123456"),
            event(o::Event::Complete(snapshot(4, "123456"))),
        ],
        [
            changed(2, "123456"),
            event(o::Event::Complete(snapshot(3, "654321"))),
        ],
        [
            changed(2, "123456"),
            Event::OtpInputEvent(
                window(),
                node(),
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                o::Event::Complete(snapshot(3, "123456")),
            ),
        ],
        [
            changed(2, "12"),
            event(o::Event::Complete(snapshot(3, "12"))),
        ],
    ] {
        assert!(mailbox.otp_completion(invalid).is_err());
        assert!(mailbox.drain(128).is_empty());
    }
    mailbox.close();
    assert!(mailbox.otp_completion(pair(1)).is_err());
}

#[test]
fn completion_byte_admission_cannot_overwrite_a_tail_before_failing() {
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
    // Charge accepted value AND draft: Changed("1") occupies 256 + 1 + 1.
    mailbox.input(filler(remaining - 256 - 258 - 200)).unwrap();
    mailbox.input(changed(1, "1")).unwrap();
    assert!(mailbox.otp_completion(pair(2)).is_err());
    let long = event(o::Event::Changed(o::Snapshot {
        draft: "x".repeat(4096),
        selection: o::Selection { anchor: 0, head: 0 },
        composition: Some(o::Selection {
            anchor: 0,
            head: 4096,
        }),
        ..snapshot(2, "1")
    }));
    assert!(mailbox.input(long.clone()).is_err());
    let mut drained = Vec::new();
    while mailbox.has_window_output(window().slot()) {
        let next = mailbox.drain(128);
        assert!(!next.is_empty());
        drained.extend(next);
    }
    assert_eq!(drained.last(), Some(&changed(1, "1")));
    mailbox.input(long).unwrap();
    mailbox.otp_completion(pair(3)).unwrap();
    assert_eq!(mailbox.drain(128), pair(3));
}

#[test]
fn correlated_responses_keep_observation_barriers_and_bound_drain_bytes() {
    use binprot::BinProtWrite;
    let mut mailbox = Mailbox::default();
    mailbox
        .submit(
            Message::OtpInputCommand(1, window(), node(), o::Command::ReadSnapshot),
            16,
        )
        .unwrap();
    mailbox.pop().unwrap();
    mailbox.input(changed(1, "1")).unwrap();
    let response =
        Event::OtpInputResult(1, window(), node(), o::Response::Applied(snapshot(1, "1")));
    mailbox.respond(response.clone());
    mailbox.input(changed(2, "12")).unwrap();
    mailbox.input(changed(3, "123")).unwrap();
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(
        mailbox.drain(128),
        vec![changed(1, "1"), response, changed(3, "123")]
    );
    assert!(!mailbox.has_window_output(window().slot()));
    let mut large = snapshot(4, "12");
    large.draft = "9".repeat(4096);
    large.selection = o::Selection {
        anchor: 4096,
        head: 4096,
    };
    large.composition = Some(o::Selection {
        anchor: 0,
        head: 4096,
    });
    assert!(large.is_valid());
    for request in 1..=128 {
        mailbox
            .submit(
                Message::OtpInputCommand(request, window(), node(), o::Command::ReadSnapshot),
                16,
            )
            .unwrap();
        mailbox.pop().unwrap();
        mailbox.respond(Event::OtpInputResult(
            request,
            window(),
            node(),
            o::Response::Applied(large.clone()),
        ));
        mailbox
            .input(event(o::Event::Observed(large.clone())))
            .unwrap();
    }
    let mut count = 0;
    let mut batches = 0;
    while mailbox.has_output() {
        let batch = mailbox.drain(256);
        assert!(!batch.is_empty());
        let mut encoded = Vec::new();
        batch.binprot_write(&mut encoded).unwrap();
        assert!(encoded.len() <= MAX_MESSAGE_BYTES);
        count += batch.len();
        batches += 1;
    }
    assert_eq!(count, 256);
    assert!(batches > 1);
    assert!(!mailbox.has_window_output(window().slot()));
}
