use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, container_query::*, v1::*};
fn node(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn config(generation: i64) -> Config {
    Config {
        generation,
        branches: vec!["small".into(), "large".into()],
        default: 0,
        rules: vec![Rule {
            condition: Predicate {
                width: Range {
                    minimum: 500.,
                    maximum: None,
                },
                height: Range::ALL,
            },
            branch: 1,
        }],
    }
}
#[test]
fn query_children_generation_and_event_fences_are_atomic() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "query", 400., 300.).unwrap();
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
                Op::Create(node(0), Kind::ContainerQuery, "".into(), Some(handler)),
                Op::Create(node(1), Kind::Text, "small".into(), None),
                Op::Create(node(2), Kind::Text, "large".into(), None),
                Op::SetContainerQuery(node(0), config(1)),
                Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    let mut invalid = config(1);
    invalid.default = 1;
    let wrong_count = Config {
        generation: 2,
        branches: vec!["small".into()],
        rules: vec![],
        default: 0,
    };
    for operations in [
        vec![Op::SetContainerQuery(node(0), invalid)],
        vec![Op::SetContainerQuery(node(0), wrong_count)],
        vec![Op::SetContainerQuery(node(1), config(2))],
        vec![
            Op::SetContainerQuery(node(0), config(2)),
            Op::SetContainerQuery(node(0), config(3)),
        ],
        vec![Op::Splice(node(0), 0, 1, vec![]), Op::Remove(node(1))],
    ] {
        assert!(session.apply(&tx(1, operations)).is_err());
        assert_eq!(session.tree(window).unwrap().revision(), 1);
        assert_eq!(session.retained_bytes(), bytes);
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(node(0))
                .unwrap()
                .children
                .as_ref(),
            &[node(1), node(2)]
        );
    }
    let first = Snapshot {
        generation: 1,
        sequence: 1,
        branch: 0,
        width: 499.75,
        height: 200.,
    };
    assert!(
        session
            .container_selected(window, node(0), handler, 1, first)
            .is_some()
    );
    for invalid in [
        Snapshot {
            generation: 2,
            ..first
        },
        Snapshot { branch: 1, ..first },
        Snapshot { branch: 2, ..first },
        Snapshot {
            width: -1.,
            ..first
        },
        Snapshot {
            sequence: 0,
            ..first
        },
    ] {
        assert!(
            session
                .container_selected(window, node(0), handler, 1, invalid)
                .is_none()
        );
    }
    assert!(
        session
            .container_selected(window, node(0), handler, 2, first)
            .is_none()
    );
    assert!(
        session
            .container_selected(
                window,
                node(0),
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                first
            )
            .is_none()
    );
    assert!(
        session
            .container_selected(window, NodeId::from_parts(0, 2).unwrap(), handler, 1, first)
            .is_none()
    );
    session
        .apply(&tx(1, vec![Op::SetContainerQuery(node(0), config(2))]))
        .unwrap();
    assert!(
        session
            .container_selected(window, node(0), handler, 1, first)
            .is_none()
    );
    let next = Snapshot {
        generation: 2,
        sequence: 2,
        width: 500.,
        branch: 1,
        ..first
    };
    assert!(
        session
            .container_selected(window, node(0), handler, 2, next)
            .is_some()
    );
    session
        .apply(&tx(
            2,
            vec![
                Op::SetRoot(None),
                Op::Remove(node(2)),
                Op::Remove(node(1)),
                Op::Remove(node(0)),
            ],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
    assert!(
        session
            .container_selected(window, node(0), handler, 2, next)
            .is_none()
    );
    session.close(window).unwrap();
    assert!(
        session
            .container_selected(window, node(0), handler, 2, next)
            .is_none()
    );
}
