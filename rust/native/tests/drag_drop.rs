use gpuio_native::{mailbox::Mailbox, session::Session};
use gpuio_protocol::{HandlerId, NodeId, WindowId, drag_drop::*, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn source(disabled: bool) -> Source {
    Source::new(
        "Source".into(),
        Payload::text("snapshot".into()).unwrap(),
        disabled,
        false,
    )
    .unwrap()
}
fn target(disabled: bool) -> Target {
    Target::new("Target".into(), vec![Format::Text], disabled).unwrap()
}
fn target_sample(gesture: i64, phase: TargetPhase) -> TargetSample {
    TargetSample {
        gesture,
        phase,
        window_x: 1.,
        window_y: 2.,
        local_x: 3.,
        local_y: 4.,
        modifiers: Default::default(),
    }
}
#[test]
fn atomic_tree_validation_and_current_owner_checks_cover_both_event_routes() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Drag/drop", 400., 300.).unwrap();
    session
        .apply(&Transaction {
            window,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(node(0), Kind::DragSource, "".into(), Some(handler(0))),
                Op::SetDragSource(node(0), source(false)),
                Op::Create(node(1), Kind::DropTarget, "".into(), Some(handler(1))),
                Op::SetDropTarget(node(1), target(false)),
                Op::Splice(node(0), 0, 0, vec![node(1)]),
                Op::SetRoot(Some(node(0))),
            ],
        })
        .unwrap();
    let retained = session.retained_bytes();
    for operations in [
        vec![Op::Bind(node(0), None)],
        vec![Op::SetText(node(1), "bad".into())],
        vec![Op::SetDropTarget(node(0), target(false))],
    ] {
        assert_eq!(
            session.apply(&Transaction {
                window,
                base: 1,
                revision: 2,
                operations
            }),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), retained);
    }
    let start = SourceSample {
        gesture: 7,
        phase: SourcePhase::Started(Payload::text("snapshot".into()).unwrap()),
    };
    assert!(
        session
            .drag_source_event(window, node(0), handler(0), 1, start.clone())
            .is_some()
    );
    let drop = target_sample(
        7,
        TargetPhase::Dropped(Payload::text("snapshot".into()).unwrap()),
    );
    assert!(
        session
            .drop_target_event(window, node(1), handler(1), 1, drop.clone())
            .is_some()
    );
    assert!(
        session
            .drop_target_event(window, node(1), handler(1), 2, drop.clone())
            .is_none()
    );
    session
        .apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations: vec![
                Op::SetDragSource(node(0), source(true)),
                Op::SetDropTarget(node(1), target(true)),
            ],
        })
        .unwrap();
    assert!(
        session
            .drag_source_event(window, node(0), handler(0), 1, start)
            .is_none()
    );
    assert!(
        session
            .drag_source_event(
                window,
                node(0),
                handler(0),
                1,
                SourceSample {
                    gesture: 7,
                    phase: SourcePhase::Ended(Outcome::Cancelled(CancelReason::Disabled))
                }
            )
            .is_some()
    );
    assert!(
        session
            .drop_target_event(window, node(1), handler(1), 1, drop)
            .is_none()
    );
    assert!(
        session
            .drop_target_event(
                window,
                node(1),
                handler(1),
                1,
                target_sample(7, TargetPhase::Left)
            )
            .is_some()
    );
    session.close(window).unwrap();
    assert!(
        session
            .drop_target_event(
                window,
                node(1),
                handler(1),
                1,
                target_sample(7, TargetPhase::Left)
            )
            .is_none()
    );
    assert_eq!(session.retained_bytes(), 0);
}
#[test]
fn motion_coalesces_only_for_one_consecutive_gesture_and_payloads_count_toward_limits() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let event = |gesture, phase| {
        Event::DropTargetEvent(
            window,
            node(0),
            handler(0),
            1,
            target_sample(gesture, phase),
        )
    };
    let mut mailbox = Mailbox::default();
    for _ in 0..1000 {
        mailbox.input(event(1, TargetPhase::Moved)).unwrap();
    }
    mailbox.input(event(1, TargetPhase::Left)).unwrap();
    mailbox.input(event(2, TargetPhase::Moved)).unwrap();
    assert_eq!(mailbox.drain(128).len(), 3);
    let payload = Payload::text("a".repeat(MAX_DATA_BYTES)).unwrap();
    let mut accepted = 0;
    while mailbox
        .input(event(3, TargetPhase::Dropped(payload.clone())))
        .is_ok()
    {
        accepted += 1;
        assert!(accepted < 128);
    }
    assert!(accepted > 1);
    let output = mailbox.drain(128);
    assert!(
        output.len() < accepted,
        "one drain remains under the encoded batch byte cap"
    );
}
