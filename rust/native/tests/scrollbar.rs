use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, WindowId, scrollbar::*, v1::*};
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
fn config() -> Config {
    Config {
        label: "Viewport".into(),
        axis: Axis::Both,
        mode: Mode::Scrolling,
        appearance: Appearance::default(),
        motion: Motion::default(),
    }
}
#[test]
fn admission_reserves_native_state_and_rejects_invalid_updates_atomically() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(n(0), Kind::Container, "".into(), None),
            Op::Create(n(1), Kind::Text, "Retained child".into(), None),
            Op::Splice(n(0), 0, 0, vec![n(1)]),
            Op::SetRoot(Some(n(0))),
        ],
    ))
    .unwrap();
    let baseline = tree.retained_bytes();
    let config = config();
    assert!(
        tree.apply_with_budget(
            &tx(
                1,
                vec![Op::SetScrollbar(n(0), Some(Box::new(config.clone())))]
            ),
            baseline + config.retained_bytes() - 1
        )
        .is_err()
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), baseline);
    assert!(tree.get(n(0)).unwrap().scrollbar.is_none());
    // A dormant container is legal: the metadata never forces native overflow.
    tree.apply(&tx(
        1,
        vec![Op::SetScrollbar(n(0), Some(Box::new(config.clone())))],
    ))
    .unwrap();
    let bytes = tree.retained_bytes();
    assert_eq!(bytes - baseline, config.retained_bytes());
    for width in [-1., 16385., f64::NAN, f64::INFINITY] {
        let mut bad = config.clone();
        bad.appearance.track_pressed.width = Some(width);
        assert!(
            tree.apply(&tx(
                2,
                vec![
                    Op::SetText(n(1), "must roll back".into()),
                    Op::SetScrollbar(n(0), Some(Box::new(bad)))
                ]
            ))
            .is_err()
        );
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.retained_bytes(), bytes);
        assert_eq!(tree.get(n(1)).unwrap().text.as_ref(), "Retained child");
        assert_eq!(tree.get(n(0)).unwrap().scrollbar.as_deref(), Some(&config));
    }
    for override_ in [None, Some(Box::new(config.clone()))] {
        assert!(
            tree.apply(&tx(2, vec![Op::SetScrollbar(n(1), override_)]))
                .is_err()
        );
        assert_eq!(tree.retained_bytes(), bytes);
    }
    tree.apply(&tx(2, vec![Op::SetScrollbar(n(0), None)]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), baseline);
    assert!(tree.get(n(0)).unwrap().scrollbar.is_none());
    assert_eq!(tree.get(n(0)).unwrap().children.as_ref(), &[n(1)]);
    tree.apply(&tx(
        3,
        vec![
            Op::SetRoot(None),
            Op::Splice(n(0), 0, 1, vec![]),
            Op::Remove(n(1)),
            Op::Remove(n(0)),
        ],
    ))
    .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}
