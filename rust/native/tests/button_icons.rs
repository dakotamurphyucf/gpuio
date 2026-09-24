use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, ResourceId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
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
fn image(label: Option<String>) -> ImageConfig {
    ImageConfig {
        source: ImageSource::Reference(ResourceId::from_parts(0, 1).unwrap()),
        fit: ImageFit::Contain,
        label,
    }
}
#[test]
fn button_icon_slots_reject_interactive_or_named_children_atomically() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(
                node(0),
                Kind::Button,
                "Send".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetControl(node(0), Control::Button(false)),
            Op::Create(node(1), Kind::Container, "".into(), None),
            Op::Create(node(2), Kind::Container, "".into(), None),
            Op::Create(node(3), Kind::Icon, "".into(), None),
            Op::SetImage(node(3), image(None)),
            Op::Splice(node(1), 0, 0, vec![node(3)]),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    ))
    .unwrap();
    let retained = tree.retained_bytes();
    for operations in [
        vec![Op::Bind(
            node(3),
            Some(HandlerId::from_parts(1, 1).unwrap()),
        )],
        vec![Op::SetImage(
            node(3),
            image(Some("Separate image label".into())),
        )],
        vec![Op::Bind(
            node(1),
            Some(HandlerId::from_parts(1, 1).unwrap()),
        )],
        vec![Op::SetText(node(1), "Not an icon slot".into())],
        vec![Op::Splice(node(0), 0, 2, vec![node(3)])],
    ] {
        assert_eq!(tree.apply(&tx(1, operations)), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), retained);
    }
    // Nonstructural source/appearance updates remain valid and retain the slot.
    tree.apply(&tx(
        1,
        vec![Op::SetImage(
            node(3),
            ImageConfig {
                source: ImageSource::Unavailable(ImageError::Released),
                ..image(None)
            },
        )],
    ))
    .unwrap();
    tree.apply(&tx(
        2,
        vec![Op::Splice(node(1), 0, 1, vec![]), Op::Remove(node(3))],
    ))
    .unwrap();
    assert_eq!(
        tree.get(node(0)).unwrap().children.as_ref(),
        &[node(1), node(2)]
    );
}
