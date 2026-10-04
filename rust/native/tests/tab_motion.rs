use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, tab_motion::Config, v1::*};
fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base,
        revision: base + 1,
        operations,
    }
}
#[test]
fn motion_admission_is_tab_only_atomic_and_accounts_native_capacity() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    let choice = ChoiceConfig {
        label: "Tabs".into(),
        items: vec![ChoiceItem {
            id: "a".into(),
            label: "A".into(),
            disabled: false,
        }],
        selected: None,
        disabled: false,
    };
    tree.apply(&tx(
        0,
        vec![
            Op::Create(
                n(0),
                Kind::TabBar,
                String::new(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetChoice(n(0), choice.clone()),
            Op::SetRoot(Some(n(0))),
        ],
    ))
    .unwrap();
    let baseline = tree.retained_bytes();
    tree.apply(&tx(
        1,
        vec![Op::SetTabMotion(n(0), Some(Config::default()))],
    ))
    .unwrap();
    assert!(tree.retained_bytes() >= baseline + Config::OWNER_RESERVED_BYTES);
    let retained = tree.retained_bytes();
    for duration in [-1, 60001] {
        assert!(
            tree.apply(&tx(
                2,
                vec![Op::SetTabMotion(
                    n(0),
                    Some(Config {
                        color_duration_ms: duration,
                        ..Default::default()
                    })
                )]
            ))
            .is_err()
        );
        assert_eq!(tree.retained_bytes(), retained);
        assert_eq!(tree.revision(), 2);
    }
    tree.apply(&tx(
        2,
        vec![Op::SetTabMotion(
            n(0),
            Some(Config {
                color_duration_ms: 0,
                ..Default::default()
            }),
        )],
    ))
    .unwrap();
    tree.apply(&tx(3, vec![Op::SetTabMotion(n(0), None)]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), baseline);
    assert!(
        tree.apply(&tx(
            4,
            vec![
                Op::Create(n(1), Kind::Container, String::new(), None),
                Op::SetTabMotion(n(1), Some(Config::default()))
            ]
        ))
        .is_err()
    );
    assert!(tree.get(n(1)).is_none());
    tree.apply(&tx(4, vec![Op::SetRoot(None), Op::Remove(n(0))]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
    assert!(
        tree.apply(&tx(
            5,
            vec![Op::SetTabMotion(n(0), Some(Config::default()))]
        ))
        .is_err()
    );
}
