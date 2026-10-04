use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, WindowId, toast_motion::Config, v1::*};
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
fn motion_admission_is_atomic_and_reset_releases_optional_metadata() {
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
    let p = Config::default();
    assert!(
        tree.apply_with_budget(
            &tx(1, vec![Op::SetToastMotion(n(0), Some(p))]),
            baseline + p.retained_bytes(0) - 1
        )
        .is_err()
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&tx(1, vec![Op::SetToastMotion(n(0), Some(p))]))
        .unwrap();
    let bytes = tree.retained_bytes();
    assert_eq!(bytes - baseline, p.retained_bytes(0));
    for bad in [-1., 16385., f64::NAN, f64::INFINITY] {
        assert!(
            tree.apply(&tx(
                2,
                vec![Op::SetToastMotion(n(0), Some(Config { offset: bad, ..p }))]
            ))
            .is_err()
        );
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.retained_bytes(), bytes);
        assert_eq!(tree.get(n(0)).unwrap().toast_motion, Some(p));
    }
    assert!(
        tree.apply(&tx(
            2,
            vec![
                Op::Create(n(1), Kind::Container, "".into(), None),
                Op::SetToastMotion(n(1), Some(p))
            ]
        ))
        .is_err()
    );
    assert!(tree.get(n(1)).is_none());
    // Child-list edits grow the parent reservation atomically, even though
    // motion metadata itself is unchanged in that transaction.
    let child = vec![
        Op::Create(
            n(1),
            Kind::Toast,
            "".into(),
            Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetToast(
            n(1),
            ToastConfig {
                label: "Saved".into(),
                close_label: "Dismiss".into(),
                timeout_ns: None,
                politeness: ToastPoliteness::Polite,
            },
        ),
        Op::Splice(n(0), 0, 0, vec![n(1)]),
    ];
    assert!(tree.apply_with_budget(&tx(2, child), bytes + 4095).is_err());
    assert!(tree.get(n(1)).is_none());
    assert!(tree.get(n(0)).unwrap().children.is_empty());
    assert_eq!(tree.retained_bytes(), bytes);
    tree.apply(&tx(2, vec![Op::SetToastMotion(n(0), None)]))
        .unwrap();
    assert!(tree.get(n(0)).unwrap().toast_motion.is_none());
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(n(0))]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}
