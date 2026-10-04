use gpuio_native::tree::Tree;
use gpuio_protocol::{
    NodeId, WindowId,
    toast_placement::{Anchor, Placement},
    v1::*,
};
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
fn placement_admission_is_atomic_and_reset_releases_optional_metadata() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(n(0), Kind::ToastStack, "".into(), None),
            Op::SetToastStack(
                n(0),
                ToastStackConfig {
                    label: "Stack".into(),
                    corner: ToastCorner::BottomRight,
                    width: 120.,
                    max_visible: 3,
                },
            ),
            Op::SetRoot(Some(n(0))),
        ],
    ))
    .unwrap();
    let baseline = tree.retained_bytes();
    let p = Placement {
        anchor: Anchor::RightCenter,
        top: 1.,
        right: 2.,
        bottom: 3.,
        left: 4.,
    };
    tree.apply(&tx(1, vec![Op::SetToastPlacement(n(0), Some(p))]))
        .unwrap();
    let bytes = tree.retained_bytes();
    assert_eq!(bytes - baseline, std::mem::size_of::<Placement>());
    for bad in [-1., 16385., f64::NAN, f64::INFINITY] {
        assert!(
            tree.apply(&tx(
                2,
                vec![Op::SetToastPlacement(
                    n(0),
                    Some(Placement { top: bad, ..p })
                )]
            ))
            .is_err()
        );
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.retained_bytes(), bytes);
        assert_eq!(tree.get(n(0)).unwrap().toast_placement, Some(p));
    }
    assert!(
        tree.apply(&tx(
            2,
            vec![
                Op::Create(n(1), Kind::Container, "".into(), None),
                Op::SetToastPlacement(n(1), Some(p))
            ]
        ))
        .is_err()
    );
    assert!(tree.get(n(1)).is_none());
    tree.apply(&tx(2, vec![Op::SetToastPlacement(n(0), None)]))
        .unwrap();
    assert!(tree.get(n(0)).unwrap().toast_placement.is_none());
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(n(0))]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}
