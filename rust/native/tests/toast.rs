use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[path = "../../protocol/tests/common/toast_fixture.rs"]
mod fixture;
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
#[test]
fn toast_graph_bounds_permissions_and_lifetimes_are_validated_atomically() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session
        .open(1, window, "Notifications", 400., 300.)
        .unwrap();
    let Message::Apply(tx) = fixture::request() else {
        panic!()
    };
    session.apply(&tx).unwrap();
    assert_eq!(
        session.toast_dismissed(window, node(1), handler, 1, ToastDismissal::Timeout),
        Some(fixture::events()[0].clone())
    );
    let before = session.retained_bytes();
    let mut invalid = vec![];
    for timeout_ns in [0, -1, MAX_TOAST_TIMEOUT_NS + 1] {
        invalid.push(vec![Op::SetToast(
            node(1),
            ToastConfig {
                timeout_ns: Some(timeout_ns),
                ..fixture::config()
            },
        )]);
    }
    for max_visible in [0, 9] {
        invalid.push(vec![Op::SetToastStack(
            node(0),
            ToastStackConfig {
                max_visible,
                ..fixture::stack()
            },
        )]);
    }
    invalid.push(vec![
        Op::Splice(node(1), 0, 1, vec![]),
        Op::Splice(node(0), 1, 0, vec![node(2)]),
    ]);
    invalid.push(vec![
        Op::Create(node(3), Kind::Container, "".into(), None),
        Op::Splice(node(0), 0, 1, vec![]),
        Op::Splice(node(3), 0, 0, vec![node(1)]),
        Op::SetRoot(Some(node(3))),
        Op::Remove(node(0)),
    ]);
    for operations in invalid {
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
        assert_eq!(session.tree(window).unwrap().revision(), 1);
    }
    session
        .apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations: vec![Op::SetToast(
                node(1),
                ToastConfig {
                    timeout_ns: None,
                    ..fixture::config()
                },
            )],
        })
        .unwrap();
    assert!(
        session
            .toast_dismissed(window, node(1), handler, 2, ToastDismissal::Timeout)
            .is_none()
    );
    assert!(
        session
            .toast_dismissed(window, node(1), handler, 2, ToastDismissal::CloseButton)
            .is_some()
    );
    session
        .apply(&Transaction {
            window,
            base: 2,
            revision: 3,
            operations: vec![
                Op::SetRoot(None),
                Op::Remove(node(2)),
                Op::Remove(node(1)),
                Op::Remove(node(0)),
            ],
        })
        .unwrap();
    assert!(
        session
            .toast_dismissed(window, node(1), handler, 2, ToastDismissal::Escape)
            .is_none()
    );
    assert_eq!(session.retained_bytes(), 0);
}

#[test]
fn visible_limit_does_not_replace_the_separate_32_item_submission_bound() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session
        .open(1, window, "Notifications", 400., 300.)
        .unwrap();
    let Message::Apply(tx) = fixture::request() else {
        panic!()
    };
    session.apply(&tx).unwrap();
    let mut operations = vec![];
    for slot in 3..34 {
        operations.push(Op::Create(
            node(slot),
            Kind::Toast,
            "".into(),
            Some(HandlerId::from_parts(slot, 1).unwrap()),
        ));
        operations.push(Op::SetToast(node(slot), fixture::config()));
    }
    operations.push(Op::Splice(node(0), 1, 0, (3..34).map(node).collect()));
    session
        .apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations,
        })
        .unwrap();
    let before = session.retained_bytes();
    assert_eq!(
        session.apply(&Transaction {
            window,
            base: 2,
            revision: 3,
            operations: vec![
                Op::Create(
                    node(34),
                    Kind::Toast,
                    "".into(),
                    Some(HandlerId::from_parts(34, 1).unwrap())
                ),
                Op::SetToast(node(34), fixture::config()),
                Op::Splice(node(0), 32, 0, vec![node(34)]),
            ]
        }),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(session.retained_bytes(), before);
    assert_eq!(
        session
            .tree(window)
            .unwrap()
            .get(node(0))
            .unwrap()
            .children
            .len(),
        32
    );
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
