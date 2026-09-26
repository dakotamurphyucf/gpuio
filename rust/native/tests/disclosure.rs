use binprot::BinProtWrite;
use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn mount() -> Transaction {
    tx(
        0,
        vec![
            Op::Create(node(0), Kind::Accordion, "".into(), None),
            Op::Create(node(1), Kind::Disclosure, "".into(), None),
            Op::Create(
                node(2),
                Kind::Button,
                "Details".into(),
                Some(HandlerId::from_parts(2, 1).unwrap()),
            ),
            Op::Create(node(3), Kind::Panel, "Details".into(), None),
            Op::SetControl(node(2), Control::Button(false)),
            Op::Splice(node(1), 0, 0, vec![node(2), node(3)]),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    )
}
#[test]
fn independent_disclosure_envelope_fixture_and_strict_decode() {
    let message = Message::Apply(mount());
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/disclosure-request.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
    for length in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..length]).is_err());
    }
    bytes.push(0);
    assert!(gpuio_protocol::decode(&bytes).is_err());
}
#[test]
fn malformed_relationships_and_labels_roll_back_atomically() {
    let mut tree = Tree::new(window());
    tree.apply(&mount()).unwrap();
    let retained = tree.retained_bytes();
    for operations in [
        vec![Op::Splice(node(1), 0, 2, vec![node(3), node(2)])],
        vec![
            Op::Splice(node(1), 0, 2, vec![node(2)]),
            Op::Remove(node(3)),
        ],
        vec![
            Op::Splice(node(0), 0, 1, vec![node(2)]),
            Op::Remove(node(1)),
        ],
        vec![Op::SetText(node(3), "".into())],
        vec![Op::SetText(node(3), "bad\0label".into())],
        vec![Op::SetText(node(3), "x".repeat(4097))],
    ] {
        assert_eq!(tree.apply(&tx(1, operations)), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), retained);
        assert_eq!(
            tree.get(node(1)).unwrap().children.as_ref(),
            [node(2), node(3)]
        );
    }
    tree.apply(&tx(1, vec![Op::SetText(node(3), "改行 ✓".into())]))
        .unwrap();
}
#[test]
fn repeated_visibility_updates_preserve_children_and_dispose_all_storage() {
    let mut tree = Tree::new(window());
    tree.apply(&mount()).unwrap();
    for revision in 1..65 {
        let style = if revision % 2 == 0 {
            vec![]
        } else {
            vec![Style::Fields(vec![Field::Display(3)])]
        };
        tree.apply(&tx(revision, vec![Op::SetStyle(node(3), style)]))
            .unwrap();
        assert_eq!(
            tree.get(node(1)).unwrap().children.as_ref(),
            [node(2), node(3)]
        );
    }
    tree.apply(&tx(
        65,
        std::iter::once(Op::SetRoot(None))
            .chain((0..=3).map(|i| Op::Remove(node(i))))
            .collect(),
    ))
    .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}
