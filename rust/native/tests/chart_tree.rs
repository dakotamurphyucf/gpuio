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
        version: -1,
        radar_labels: vec![],
        source: Some(ResourceId::from_parts(0, 1).unwrap()),
        label: "Diagram".into(),
        legend: true,
        disabled: false,
        options: Default::default(),
        sampling: Default::default(),
        style: Default::default(),
    }
}

#[test]
fn radar_slots_validate_complete_tree_and_preserve_identity_on_reorder() {
    let mut tree = Tree::new(window());
    let configured = |axes| {
        Box::new(Config {
            radar_labels: axes,
            ..config()
        })
    };
    apply(
        &mut tree,
        vec![
            Op::Create(node(0), Kind::ChartView, "".into(), None),
            Op::SetChart(node(0), configured(vec![7, 9])),
            Op::Create(node(1), Kind::Container, "".into(), None),
            Op::Create(node(2), Kind::Text, "First".into(), None),
            Op::Create(node(3), Kind::Container, "".into(), None),
            Op::Create(node(4), Kind::Text, "Second".into(), None),
            Op::Splice(node(1), 0, 0, vec![node(2)]),
            Op::Splice(node(3), 0, 0, vec![node(4)]),
            Op::Splice(node(0), 0, 0, vec![node(1), node(3)]),
            Op::SetRoot(Some(node(0))),
        ],
    )
    .unwrap();
    let revision = tree.revision();
    let retained = tree.retained_bytes();
    for operations in [
        vec![Op::SetChart(node(0), configured(vec![7]))],
        vec![Op::SetChart(node(0), configured(vec![7, 7]))],
        vec![Op::SetChart(node(0), configured(vec![0, 9]))],
        vec![Op::Splice(node(1), 0, 1, vec![]), Op::Remove(node(2))],
    ] {
        assert_eq!(apply(&mut tree, operations), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), revision);
        assert_eq!(tree.retained_bytes(), retained);
        assert_eq!(tree.get(node(1)).unwrap().children.as_ref(), &[node(2)]);
    }
    apply(
        &mut tree,
        vec![
            Op::SetChart(node(0), configured(vec![9, 7])),
            Op::Splice(node(0), 0, 2, vec![node(3), node(1)]),
        ],
    )
    .unwrap();
    assert_eq!(
        tree.get(node(0))
            .unwrap()
            .chart
            .as_ref()
            .unwrap()
            .radar_labels,
        vec![9, 7]
    );
    assert_eq!(tree.get(node(1)).unwrap().children.as_ref(), &[node(2)]);
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
            Op::SetChart(node(0), Box::new(config())),
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
        vec![Op::SetChart(node(0), Box::new(invalid))],
        vec![Op::SetText(node(0), "not a text leaf".into())],
        vec![
            Op::Create(node(1), Kind::Text, "child".into(), None),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
        ],
        vec![
            Op::SetChart(node(0), Box::new(config())),
            Op::SetChart(node(0), Box::new(config())),
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
            Box::new(Config {
                source: None,
                ..config()
            }),
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
            Op::SetChart(*child, Box::new(config())),
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
            Op::SetChart(replacement, Box::new(config())),
            Op::Splice(node(0), 0, 128, next_children),
        ],
    )
    .unwrap();
    assert_eq!(tree.retained_bytes(), retained);
}
