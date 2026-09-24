use gpuio_native::{mailbox::Mailbox, session::Session};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[path = "../../protocol/tests/common/pointer_fixture.rs"]
mod fixture;
#[test]
fn pointer_validation_is_atomic_and_cancellation_survives_disable() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Pointers", 400., 300.).unwrap();
    let Message::Apply(tx) = fixture::request() else {
        panic!()
    };
    session.apply(&tx).unwrap();
    assert_eq!(
        session.pointer_event(
            window,
            node,
            handler,
            1,
            fixture::sample(PointerPhase::Started)
        ),
        Some(fixture::events()[0].clone())
    );
    let before = session.retained_bytes();
    for operations in [
        vec![Op::Bind(node, None)],
        vec![Op::SetPointer(
            node,
            PointerConfig {
                label: " ".into(),
                ..fixture::config()
            },
        )],
        vec![Op::SetText(node, "untyped label".into())],
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
        assert_eq!(session.retained_bytes(), before);
    }
    session
        .apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations: vec![Op::SetPointer(
                node,
                PointerConfig {
                    disabled: true,
                    ..fixture::config()
                },
            )],
        })
        .unwrap();
    assert!(
        session
            .pointer_event(
                window,
                node,
                handler,
                2,
                fixture::sample(PointerPhase::Moved)
            )
            .is_none()
    );
    assert!(
        session
            .pointer_event(
                window,
                node,
                handler,
                2,
                fixture::sample(PointerPhase::Cancelled(PointerCancel::Disabled))
            )
            .is_some()
    );
    for sample in [
        PointerSample {
            gesture: 0,
            ..fixture::sample(PointerPhase::Cancelled(PointerCancel::Hidden))
        },
        PointerSample {
            local_x: f64::NAN,
            ..fixture::sample(PointerPhase::Cancelled(PointerCancel::Hidden))
        },
    ] {
        assert!(
            session
                .pointer_event(window, node, handler, 2, sample)
                .is_none()
        );
    }
    session
        .apply(&Transaction {
            window,
            base: 2,
            revision: 3,
            operations: vec![Op::SetRoot(None), Op::Remove(node)],
        })
        .unwrap();
    assert!(
        session
            .pointer_event(
                window,
                node,
                handler,
                3,
                fixture::sample(PointerPhase::Cancelled(PointerCancel::Removed))
            )
            .is_none()
    );
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
#[test]
fn motion_coalesces_without_crossing_gesture_revision_or_lifecycle_barriers() {
    let mut mailbox = Mailbox::default();
    let events = fixture::events();
    mailbox.input(events[0].clone()).unwrap();
    for i in 0..10000 {
        let mut event = events[1].clone();
        let Event::PointerEvent(_, _, _, _, sample) = &mut event else {
            panic!()
        };
        sample.local_x = f64::from(i);
        mailbox.input(event).unwrap();
    }
    mailbox.input(events[2].clone()).unwrap();
    let mut next = events[1].clone();
    if let Event::PointerEvent(_, _, _, _, sample) = &mut next {
        sample.gesture += 1;
    }
    mailbox.input(next.clone()).unwrap();
    let mut revision = next.clone();
    if let Event::PointerEvent(_, _, _, r, _) = &mut revision {
        *r += 1;
    }
    mailbox.input(revision.clone()).unwrap();
    let output = mailbox.drain(128);
    assert_eq!(output.len(), 5);
    assert_eq!(output[0], events[0]);
    assert!(matches!(output[1],Event::PointerEvent(_,_,_,_,sample) if sample.local_x == 9999.));
    assert_eq!(output[2], events[2]);
    assert_eq!(output[3], next);
    assert_eq!(output[4], revision);
}
