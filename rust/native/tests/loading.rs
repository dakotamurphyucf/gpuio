use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    loading::{Config, Kind as LoadingKind},
    v1::*,
};
#[test]
fn loading_is_a_bounded_noninteractive_leaf_and_failed_updates_are_atomic() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Loading", 300., 200.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let config = Config {
        kind: LoadingKind::Spinner,
        label: "Load".into(),
        animated: true,
        period_ms: 1200,
    };
    assert!(
        session
            .apply(&tx(
                0,
                vec![Op::Create(node, Kind::Loading, "".into(), None)]
            ))
            .is_err()
    );
    assert_eq!(session.retained_bytes(), 0);
    assert!(
        session
            .apply(&tx(
                0,
                vec![
                    Op::Create(
                        node,
                        Kind::Loading,
                        "".into(),
                        Some(HandlerId::from_parts(0, 1).unwrap())
                    ),
                    Op::SetLoading(node, config.clone())
                ]
            ))
            .is_err()
    );
    assert_eq!(session.retained_bytes(), 0);
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Loading, "".into(), None),
                Op::SetLoading(node, config.clone()),
                Op::SetRoot(Some(node)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    assert!(bytes >= config.retained_bytes());
    assert!(
        session
            .apply(&tx(
                1,
                vec![
                    Op::SetLoading(
                        node,
                        Config {
                            label: "A new label".into(),
                            ..config.clone()
                        }
                    ),
                    Op::SetText(node, "invalid child content".into())
                ]
            ))
            .is_err()
    );
    assert_eq!(session.retained_bytes(), bytes);
    assert_eq!(
        session
            .tree(window)
            .unwrap()
            .get(node)
            .unwrap()
            .loading
            .as_deref(),
        Some(&config)
    );
    session
        .apply(&tx(
            1,
            vec![Op::SetLoading(
                node,
                Config {
                    animated: false,
                    ..config
                },
            )],
        ))
        .unwrap();
    session
        .apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
