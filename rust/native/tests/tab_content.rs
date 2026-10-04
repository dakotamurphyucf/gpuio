use gpuio_native::tree::Tree;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    tab_content::{Config, Label},
    v1::*,
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
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
fn structured_parts_are_atomic_and_only_labels_are_passive() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    let h = Some(HandlerId::from_parts(0, 1).unwrap());
    let mut ops = vec![Op::Create(id(0), Kind::TabBar, String::new(), h)];
    for i in 1..=4 {
        ops.push(Op::Create(id(i), Kind::Container, String::new(), None));
    }
    ops.extend([
        Op::Create(id(5), Kind::Text, "Custom".into(), None),
        Op::Create(id(6), Kind::Button, "Close".into(), h),
        Op::Splice(id(3), 0, 0, vec![id(5)]),
        Op::Splice(id(4), 0, 0, vec![id(6)]),
        Op::Splice(id(1), 0, 0, vec![id(2), id(3), id(4)]),
        Op::Splice(id(0), 0, 0, vec![id(1)]),
        Op::SetChoice(
            id(0),
            ChoiceConfig {
                label: "Tabs".into(),
                items: vec![ChoiceItem {
                    id: "one".into(),
                    label: "One".into(),
                    disabled: false,
                }],
                selected: None,
                disabled: false,
            },
        ),
        Op::SetTabContent(
            id(0),
            Some(Config {
                max_width: Some(160.),
                labels: vec![Label::Custom],
            }),
        ),
        Op::SetRoot(Some(id(0))),
    ]);
    tree.apply(&tx(0, ops)).unwrap();
    let baseline = tree.retained_bytes();
    for invalid in [
        vec![Op::Bind(id(5), h)],
        vec![Op::SetHoverObserver(id(5), h)],
        vec![Op::SetHoverObserver(id(3), h)],
        vec![Op::SetAccessibility(
            id(3),
            Some(gpuio_protocol::accessibility::Config {
                role: None,
                label: Some("Unexpected semantic owner".into()),
                description: None,
                live: gpuio_protocol::accessibility::Live::Off,
                field: None,
                current: None,
            }),
        )],
        vec![Op::Bind(id(3), h)],
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![Field::Disabled(true)])],
        )],
        vec![Op::SetTabContent(id(0), None)],
        vec![Op::SetTabContent(
            id(0),
            Some(Config {
                max_width: None,
                labels: vec![Label::Default],
            }),
        )],
        vec![Op::SetTabContent(
            id(6),
            Some(Config {
                max_width: None,
                labels: vec![],
            }),
        )],
        vec![
            Op::Splice(id(3), 0, 1, vec![id(6)]),
            Op::Splice(id(4), 0, 1, vec![id(5)]),
        ],
    ] {
        assert!(tree.apply(&tx(1, invalid)).is_err());
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), baseline);
        assert!(tree.get(id(5)).unwrap().handler.is_none());
    }
    tree.apply(&tx(
        1,
        vec![
            Op::SetText(id(6), "Close updated".into()),
            Op::Bind(id(6), Some(HandlerId::from_parts(1, 1).unwrap())),
        ],
    ))
    .unwrap();
    tree.apply(&tx(
        2,
        vec![
            Op::Splice(id(3), 0, 1, vec![]),
            Op::Remove(id(5)),
            Op::SetTabContent(
                id(0),
                Some(Config {
                    max_width: Some(1.),
                    labels: vec![Label::Hidden],
                }),
            ),
        ],
    ))
    .unwrap();
    let mut cleanup = vec![Op::SetRoot(None)];
    for i in [6, 2, 3, 4, 1, 0] {
        cleanup.push(Op::Remove(id(i)));
    }
    tree.apply(&tx(3, cleanup)).unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn exact_public_core_transactions_create_reorder_reset_and_dispose() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    let mut close = None;
    for (index, line) in include_str!("../../../test/fixtures/tab-content-transactions.hex")
        .lines()
        .enumerate()
    {
        let bytes: Vec<_> = line
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect();
        let Message::Apply(transaction) = gpuio_protocol::decode(&bytes).unwrap() else {
            panic!("expected public transaction")
        };
        tree.apply(&transaction).unwrap();
        if index == 0 {
            let node = tree.get(id(7)).unwrap();
            assert_eq!(node.kind, Kind::Button);
            close = Some(node.clone());
            assert_eq!(
                tree.get(id(0))
                    .unwrap()
                    .tab_content
                    .as_ref()
                    .unwrap()
                    .labels,
                vec![Label::Custom, Label::Default]
            );
        } else if index == 1 {
            assert_eq!(tree.get(id(7)), close.as_ref());
            assert_eq!(
                tree.get(id(0))
                    .unwrap()
                    .tab_content
                    .as_ref()
                    .unwrap()
                    .labels,
                vec![Label::Default, Label::Custom]
            );
        } else if index == 2 {
            assert!(tree.get(id(0)).unwrap().tab_content.is_none());
            assert!(tree.get(id(0)).unwrap().children.is_empty());
            assert!(tree.get(id(7)).is_none());
        }
    }
    assert_eq!(tree.retained_bytes(), 0);
}
