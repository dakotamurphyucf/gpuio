use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, WindowId, icon_transform::Transform, v1::*};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn tx(tree: &Tree, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    }
}
#[test]
fn icon_transform_admission_is_atomic_kind_checked_and_resettable() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    tree.apply(&tx(
        &tree,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::Create(id(1), Kind::Icon, "".into(), None),
            Op::SetImage(
                id(1),
                ImageConfig {
                    source: ImageSource::Unavailable(ImageError::Released),
                    fit: ImageFit::Contain,
                    label: Some("Arrow".into()),
                },
            ),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    ))
    .unwrap();
    let transform = Transform {
        rotation_degrees: 90.,
        ..Default::default()
    };
    tree.apply(&tx(
        &tree,
        vec![Op::SetIconTransform(id(1), Some(transform))],
    ))
    .unwrap();
    let original = tree.get(id(1)).unwrap().image.clone();
    for bad in [
        Op::SetIconTransform(id(0), Some(transform)),
        Op::SetIconTransform(
            id(1),
            Some(Transform {
                scale_x: f64::NAN,
                ..transform
            }),
        ),
        Op::SetIconTransform(
            id(1),
            Some(Transform {
                translate_y: 16385.,
                ..transform
            }),
        ),
    ] {
        let revision = tree.revision();
        assert_eq!(
            tree.apply(&tx(&tree, vec![Op::SetIconTransform(id(1), None), bad])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.revision(), revision);
        assert_eq!(tree.get(id(1)).unwrap().icon_transform, Some(transform));
        assert_eq!(tree.get(id(1)).unwrap().image, original);
    }
    tree.apply(&tx(&tree, vec![Op::SetIconTransform(id(1), None)]))
        .unwrap();
    assert_eq!(tree.get(id(1)).unwrap().icon_transform, None);
    assert_eq!(tree.get(id(1)).unwrap().image, original);
}
