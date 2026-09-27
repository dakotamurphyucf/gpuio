use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, ResourceId, WindowId, chart_view::*, v1::*};

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node(slot: u32) -> NodeId {
    NodeId::from_parts(slot as i64, 1).unwrap()
}
fn config() -> Config {
    Config {
        source: Some(ResourceId::from_parts(0, 1).unwrap()),
        label: "Diagram".into(),
        options: Default::default(),
        sampling: Default::default(),
        style: Default::default(),
    }
}
fn apply(tree: &mut Tree, operations: Vec<Op>) -> Result<gpuio_native::tree::Applied, ErrorCode> {
    tree.apply(&Transaction {
        window: window(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    })
}

#[test]
fn chart_leaf_validation_and_failed_updates_are_atomic() {
    let mut tree = Tree::new(window());
    assert_eq!(
        apply(
            &mut tree,
            vec![
                Op::Create(node(0), Kind::ChartView, "".into(), None),
                Op::SetRoot(Some(node(0)))
            ]
        ),
        Err(ErrorCode::InvalidTree)
    );
    assert!(tree.is_empty());
    apply(
        &mut tree,
        vec![
            Op::Create(node(0), Kind::ChartView, "".into(), None),
            Op::SetChart(node(0), config()),
            Op::SetRoot(Some(node(0))),
        ],
    )
    .unwrap();
    let retained = tree.retained_bytes();
    let revision = tree.revision();
    let invalid = Config {
        label: "\n".into(),
        ..config()
    };
    for ops in [
        vec![Op::SetChart(node(0), invalid)],
        vec![Op::SetText(node(0), "not a text leaf".into())],
        vec![
            Op::Create(node(1), Kind::Text, "child".into(), None),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
        ],
        vec![
            Op::SetChart(node(0), config()),
            Op::SetChart(node(0), config()),
        ],
    ] {
        assert_eq!(apply(&mut tree, ops), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), revision);
        assert_eq!(tree.retained_bytes(), retained);
        assert_eq!(tree.get(node(0)).unwrap().chart.as_deref(), Some(&config()));
    }
    // Unavailable owner is valid configuration; the view reports a typed failure.
    apply(
        &mut tree,
        vec![Op::SetChart(
            node(0),
            Config {
                source: None,
                ..config()
            },
        )],
    )
    .unwrap();
    apply(&mut tree, vec![Op::SetRoot(None), Op::Remove(node(0))]).unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn chart_mount_quota_rolls_back_and_is_reusable_after_removal() {
    let mut tree = Tree::new(window());
    let children: Vec<_> = (1..=128).map(node).collect();
    let mut ops = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
    for child in &children {
        ops.extend([
            Op::Create(*child, Kind::ChartView, "".into(), None),
            Op::SetChart(*child, config()),
        ]);
    }
    ops.extend([
        Op::Splice(node(0), 0, 0, children.clone()),
        Op::SetRoot(Some(node(0))),
    ]);
    apply(&mut tree, ops).unwrap();
    let retained = tree.retained_bytes();
    assert_eq!(
        apply(
            &mut tree,
            vec![Op::Create(node(129), Kind::ChartView, "".into(), None)]
        ),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.retained_bytes(), retained);
    let replacement = NodeId::from_parts(1, 2).unwrap();
    let mut next_children = children;
    next_children[0] = replacement;
    apply(
        &mut tree,
        vec![
            Op::Remove(node(1)),
            Op::Create(replacement, Kind::ChartView, "".into(), None),
            Op::SetChart(replacement, config()),
            Op::Splice(node(0), 0, 128, next_children),
        ],
    )
    .unwrap();
    assert_eq!(tree.retained_bytes(), retained);
}
