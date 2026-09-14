use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, animation::*, v1::*};
fn config(generation: i64, width: f64) -> Config {
    Config {
        generation,
        targets: vec![Target {
            property: Property::Width,
            value: width,
        }],
        initial: None,
        duration_ms: 100,
        delay_ms: 0,
        easing: Easing::Linear,
        repeat: Repeat::Once,
    }
}
#[test]
fn animation_generation_validation_and_lifecycle_are_atomic() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "motion", 400., 300.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Animated, "".into(), Some(handler)),
                Op::SetAnimation(node, config(1, 100.)),
                Op::SetRoot(Some(node)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    for invalid in [config(1, 200.), config(0, 200.), config(2, -1.)] {
        assert!(
            session
                .apply(&tx(1, vec![Op::SetAnimation(node, invalid)]))
                .is_err()
        );
        assert_eq!(session.tree(window).unwrap().revision(), 1);
        assert_eq!(session.retained_bytes(), bytes);
    }
    session
        .apply(&tx(1, vec![Op::SetAnimation(node, config(2, 200.))]))
        .unwrap();
    let previous = Endpoint {
        generation: 1,
        outcome: Outcome::Cancelled(CancelReason::Replaced),
    };
    assert!(
        session
            .animation_endpoint(window, node, handler, 2, previous)
            .is_some()
    );
    assert!(
        session
            .animation_endpoint(window, node, handler, 3, previous)
            .is_none()
    );
    assert!(
        session
            .animation_endpoint(
                window,
                node,
                handler,
                2,
                Endpoint {
                    generation: 3,
                    outcome: Outcome::Finished
                }
            )
            .is_none()
    );
    assert!(
        session
            .animation_endpoint(
                window,
                NodeId::from_parts(0, 2).unwrap(),
                handler,
                2,
                previous
            )
            .is_none()
    );
    session
        .apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
    assert!(
        session
            .animation_endpoint(window, node, handler, 3, previous)
            .is_none()
    );
}
