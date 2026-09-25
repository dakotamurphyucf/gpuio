use gpuio_native::{
    mailbox::{MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, numeric::Domain, slider as s, v1::*};

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn config() -> s::Config {
    s::Config {
        domain: Domain::new(-2., 8., 0.5).unwrap(),
        label: "Temperature".into(),
        lower_label: "Minimum".into(),
        upper_label: "Maximum".into(),
        axis: s::Axis::Horizontal,
        scale: s::Scale::Linear,
        disabled: false,
        read_only: false,
    }
}
fn value() -> s::Value {
    s::Value::Range {
        lower: 2.,
        upper: 7.,
    }
}
fn snapshot(revision: i64) -> s::Snapshot {
    s::Snapshot {
        revision,
        value: s::Value::Range {
            lower: 1.5,
            upper: 7.,
        },
        committed: value(),
        dragging: Some(s::Thumb::Lower),
    }
}
fn event(event: s::Event) -> Event {
    Event::SliderEvent(window(), node(), handler(), 1, event)
}
fn preview(revision: i64) -> Event {
    event(s::Event::Preview(snapshot(revision)))
}
fn transaction(base: i64, operations: Vec<Op>) -> Transaction {
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
    session.open(1, window(), "Slider", 320., 200.).unwrap();
    session
}

#[test]
fn admission_is_atomic_mode_is_fixed_and_config_storage_is_released() {
    let mut session = session();
    for operations in [
        vec![Op::Create(node(), Kind::Slider, "".into(), Some(handler()))],
        vec![
            Op::Create(node(), Kind::Slider, "".into(), None),
            Op::SetSlider(node(), config(), value()),
        ],
        vec![
            Op::Create(node(), Kind::Container, "".into(), Some(handler())),
            Op::SetSlider(node(), config(), value()),
        ],
    ] {
        assert!(session.apply(&transaction(0, operations)).is_err());
        assert_eq!(session.retained_bytes(), 0);
        assert_eq!(session.tree(window()).unwrap().revision(), 0);
    }
    session
        .apply(&transaction(
            0,
            vec![
                Op::Create(node(), Kind::Slider, "".into(), Some(handler())),
                Op::SetSlider(node(), config(), value()),
                Op::SetRoot(Some(node())),
            ],
        ))
        .unwrap();
    let retained = session.retained_bytes();
    assert!(retained >= config().retained_bytes());
    let old = std::sync::Arc::downgrade(
        &session
            .tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .slider
            .as_ref()
            .unwrap()
            .config,
    );
    for operations in [
        vec![Op::SetSlider(node(), config(), s::Value::Single(3.))],
        vec![Op::SetSlider(node(), config(), s::Value::Single(f64::NAN))],
        vec![Op::SetSlider(
            node(),
            s::Config {
                label: "".into(),
                ..config()
            },
            value(),
        )],
        vec![Op::Bind(node(), None)],
        vec![
            Op::SetSlider(
                node(),
                s::Config {
                    label: "Longer label".into(),
                    ..config()
                },
                value(),
            ),
            Op::SetText(node(), "invalid leaf text".into()),
        ],
    ] {
        assert!(session.apply(&transaction(1, operations)).is_err());
        assert_eq!(session.retained_bytes(), retained);
        assert_eq!(session.tree(window()).unwrap().revision(), 1);
        assert_eq!(
            *session
                .tree(window())
                .unwrap()
                .get(node())
                .unwrap()
                .slider
                .as_ref()
                .unwrap()
                .config,
            config()
        );
    }
    session
        .apply(&transaction(
            1,
            vec![Op::SetSlider(
                node(),
                s::Config {
                    label: "A much longer label".into(),
                    ..config()
                },
                value(),
            )],
        ))
        .unwrap();
    assert!(old.upgrade().is_none());
    assert_eq!(
        session.retained_bytes(),
        retained + "A much longer label".len() - "Temperature".len()
    );
    session
        .apply(&transaction(2, vec![Op::SetRoot(None), Op::Remove(node())]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}

#[test]
fn routing_validates_identity_and_phase_but_keeps_old_domain_cancellation() {
    let mut session = session();
    session
        .apply(&transaction(
            0,
            vec![
                Op::Create(node(), Kind::Slider, "".into(), Some(handler())),
                Op::SetSlider(node(), config(), value()),
                Op::SetRoot(Some(node())),
            ],
        ))
        .unwrap();
    let sample = s::Event::Preview(snapshot(1));
    let route =
        |session: &Session, w, n, h, rev, sample| session.slider_event(w, n, h, rev, sample);
    assert_eq!(
        route(&session, window(), node(), handler(), 1, sample),
        Some(event(sample))
    );
    assert!(session.press(window(), node(), handler(), 1).is_none());
    for (w, n, h, rev) in [
        (WindowId::from_parts(0, 2).unwrap(), node(), handler(), 1),
        (window(), NodeId::from_parts(0, 2).unwrap(), handler(), 1),
        (window(), node(), HandlerId::from_parts(0, 2).unwrap(), 1),
        (window(), node(), handler(), -1),
        (window(), node(), handler(), 2),
    ] {
        assert!(route(&session, w, n, h, rev, sample).is_none());
    }
    for sample in [
        s::Event::Preview(s::Snapshot {
            revision: 0,
            ..snapshot(1)
        }),
        s::Event::Committed(s::Source::Pointer, snapshot(1)),
        s::Event::Observed(s::Snapshot {
            revision: 1,
            value: s::Value::Single(1.),
            committed: s::Value::Single(1.),
            dragging: None,
        }),
    ] {
        assert!(route(&session, window(), node(), handler(), 1, sample).is_none());
    }
    session
        .apply(&transaction(
            1,
            vec![Op::SetSlider(
                node(),
                s::Config {
                    domain: Domain::new(0., 1., 0.1).unwrap(),
                    disabled: true,
                    read_only: true,
                    ..config()
                },
                value(),
            )],
        ))
        .unwrap();
    let cancelled = s::Event::Cancelled(
        s::CancelReason::ConfigurationChanged,
        s::Snapshot {
            value: value(),
            dragging: None,
            ..snapshot(2)
        },
    );
    assert_eq!(
        route(&session, window(), node(), handler(), 1, cancelled),
        Some(event(cancelled))
    );
    assert!(session.overload(window()));
    assert!(route(&session, window(), node(), handler(), 1, cancelled).is_none());
    session.close(window()).unwrap();
    assert_eq!(session.retained_bytes(), 0);
    assert!(route(&session, window(), node(), handler(), 1, cancelled).is_none());
}

#[test]
fn adjacent_previews_are_bounded_without_crossing_lifecycle_or_response_barriers() {
    let mut mailbox = Mailbox::default();
    let start = event(s::Event::DragStarted(s::Snapshot {
        value: value(),
        ..snapshot(1)
    }));
    let finish = event(s::Event::Committed(
        s::Source::Pointer,
        s::Snapshot {
            value: value(),
            dragging: None,
            ..snapshot(10_001)
        },
    ));
    mailbox.input(start.clone()).unwrap();
    for revision in 2..=10_000 {
        mailbox.input(preview(revision)).unwrap();
    }
    mailbox.input(finish.clone()).unwrap();
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(mailbox.drain(128), [start, preview(10_000), finish]);
    assert!(!mailbox.has_window_output(window().slot()));

    let idle = s::Snapshot {
        value: value(),
        dragging: None,
        ..snapshot(2)
    };
    for barrier in [
        event(s::Event::Cancelled(s::CancelReason::Escape, idle)),
        event(s::Event::Committed(s::Source::Keyboard, idle)),
        event(s::Event::Observed(idle)),
        event(s::Event::DragStarted(s::Snapshot {
            value: value(),
            ..snapshot(2)
        })),
        Event::Rendered(window(), 1),
    ] {
        mailbox.input(preview(1)).unwrap();
        mailbox.input(barrier.clone()).unwrap();
        mailbox.input(preview(3)).unwrap();
        assert_eq!(mailbox.drain(128), [preview(1), barrier, preview(3)]);
    }
    mailbox.input(preview(1)).unwrap();
    mailbox.submit(Message::Hello(VERSION, 0), 2).unwrap();
    mailbox.pop().unwrap();
    mailbox.respond(Event::Failed(1, ErrorCode::NotReady));
    mailbox.input(preview(3)).unwrap();
    assert_eq!(
        mailbox.drain(128),
        [
            preview(1),
            Event::Failed(1, ErrorCode::NotReady),
            preview(3)
        ]
    );
}

#[test]
fn coalescing_respects_owners_revisions_thumbs_committed_values_and_capacity() {
    let sample = s::Event::Preview(snapshot(2));
    for next in [
        Event::SliderEvent(
            WindowId::from_parts(0, 2).unwrap(),
            node(),
            handler(),
            1,
            sample,
        ),
        Event::SliderEvent(
            window(),
            NodeId::from_parts(0, 2).unwrap(),
            handler(),
            1,
            sample,
        ),
        Event::SliderEvent(
            window(),
            node(),
            HandlerId::from_parts(0, 2).unwrap(),
            1,
            sample,
        ),
        Event::SliderEvent(window(), node(), handler(), 2, sample),
        preview(1),
        event(s::Event::Preview(s::Snapshot {
            value: s::Value::Range {
                lower: 2.,
                upper: 7.5,
            },
            dragging: Some(s::Thumb::Upper),
            ..snapshot(2)
        })),
        event(s::Event::Preview(s::Snapshot {
            committed: s::Value::Range {
                lower: 3.,
                upper: 7.,
            },
            ..snapshot(2)
        })),
    ] {
        let mut mailbox = Mailbox::default();
        mailbox.input(preview(1)).unwrap();
        mailbox.input(next.clone()).unwrap();
        assert_eq!(mailbox.drain(128), [preview(1), next]);
    }
    let mut mailbox = Mailbox::default();
    let observed = event(s::Event::Observed(s::Snapshot {
        value: value(),
        dragging: None,
        ..snapshot(1)
    }));
    for _ in 0..MAX_INPUT_EVENTS - 1 {
        mailbox.input(observed.clone()).unwrap();
    }
    mailbox.input(preview(1)).unwrap();
    // Replacement is allowed at capacity; discrete commit must report pressure.
    mailbox.input(preview(2)).unwrap();
    let commit = event(s::Event::Committed(
        s::Source::Pointer,
        s::Snapshot {
            value: value(),
            dragging: None,
            ..snapshot(3)
        },
    ));
    assert_eq!(mailbox.input(commit.clone()), Err(Box::new(commit.clone())));
    let retained = mailbox.drain(128);
    assert_eq!(retained.len(), MAX_INPUT_EVENTS);
    assert_eq!(retained.last(), Some(&preview(2)));
    mailbox.input(commit.clone()).unwrap();
    assert_eq!(mailbox.drain(128), [commit]);
}

#[test]
fn command_responses_survive_full_input_lane_and_fence_window_reuse_until_drained() {
    let mut mailbox = Mailbox::default();
    let snapshot = s::Snapshot {
        value: value(),
        dragging: None,
        ..snapshot(1)
    };
    for _ in 0..MAX_INPUT_EVENTS {
        mailbox.input(event(s::Event::Observed(snapshot))).unwrap();
    }
    mailbox
        .submit(
            Message::SliderCommand(7, window(), node(), s::Command::ReadSnapshot),
            12,
        )
        .unwrap();
    assert!(matches!(mailbox.pop(), Some(Message::SliderCommand(7, ..))));
    let response = Event::SliderResult(7, window(), node(), s::Response::Applied(snapshot));
    mailbox.respond(response.clone());
    assert_eq!(mailbox.drain(MAX_INPUT_EVENTS).len(), MAX_INPUT_EVENTS);
    assert!(mailbox.has_window_output(window().slot()));
    assert_eq!(mailbox.drain(1), [response]);
    assert!(!mailbox.has_window_output(window().slot()));
}
