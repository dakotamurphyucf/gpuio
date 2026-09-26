use gpuio_native::tree::Tree;
use gpuio_protocol::{
    NodeId, WindowId,
    navigation_stack::{Config, Motion},
    v1::*,
};
fn node(id: i64) -> NodeId {
    NodeId::from_parts(id, 1).unwrap()
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base,
        revision: base + 1,
        operations,
    }
}
fn config(selected: Option<i64>, retain: bool) -> Config {
    Config {
        selected,
        retain,
        motion: Motion::Slide,
        duration_ms: 200,
    }
}
fn tree() -> Tree {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(node(0), Kind::NavigationStack, "Routes".into(), None),
            Op::Create(node(1), Kind::Panel, "First".into(), None),
            Op::Create(node(2), Kind::Panel, "Second".into(), None),
            Op::Create(node(3), Kind::Text, "Retained content".into(), None),
            Op::Splice(node(1), 0, 0, vec![node(3)]),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetNavigationStack(node(0), config(Some(0), true)),
            Op::SetRoot(Some(node(0))),
        ],
    ))
    .unwrap();
    tree
}
#[test]
fn invalid_selection_shape_policy_and_labels_are_atomic() {
    let mut tree = tree();
    let bytes = tree.retained_bytes();
    for operations in [
        vec![Op::SetNavigationStack(node(0), config(None, true))],
        vec![Op::SetNavigationStack(node(0), config(Some(2), true))],
        vec![Op::SetNavigationStack(node(0), config(Some(1), false))],
        vec![Op::SetNavigationStack(node(1), config(Some(0), true))],
        vec![Op::SetText(node(0), "".into())],
        vec![Op::SetText(node(0), "x\0y".into())],
        vec![
            Op::Splice(node(1), 0, 1, vec![]),
            Op::Splice(node(0), 0, 0, vec![node(3)]),
        ],
    ] {
        assert_eq!(tree.apply(&tx(1, operations)), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), bytes);
        assert_eq!(
            tree.get(node(0)).unwrap().navigation_stack,
            Some(config(Some(0), true))
        );
    }
    tree.apply(&tx(
        1,
        vec![
            Op::SetNavigationStack(node(0), config(Some(1), false)),
            Op::Splice(node(1), 0, 1, vec![]),
            Op::Remove(node(3)),
        ],
    ))
    .unwrap();
    assert!(tree.retained_bytes() < bytes);
}
#[test]
fn maximum_pages_empty_selection_and_disposal() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    let mut operations = vec![Op::Create(
        node(0),
        Kind::NavigationStack,
        "Routes".into(),
        None,
    )];
    for id in 1..=128 {
        operations.push(Op::Create(
            node(id),
            Kind::Panel,
            format!("Page {id}"),
            None,
        ));
    }
    operations.extend([
        Op::Splice(node(0), 0, 0, (1..=128).map(node).collect()),
        Op::SetNavigationStack(node(0), config(Some(127), true)),
        Op::SetRoot(Some(node(0))),
    ]);
    tree.apply(&tx(0, operations)).unwrap();
    assert_eq!(
        tree.apply(&tx(
            1,
            vec![
                Op::Create(node(129), Kind::Panel, "Too many".into(), None),
                Op::Splice(node(0), 128, 0, vec![node(129)])
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    let mut clear = vec![
        Op::Splice(node(0), 0, 128, vec![]),
        Op::SetNavigationStack(node(0), config(None, true)),
    ];
    clear.extend((1..=128).map(|id| Op::Remove(node(id))));
    tree.apply(&tx(1, clear)).unwrap();
    assert_eq!(tree.len(), 1);
    tree.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node(0))]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}
