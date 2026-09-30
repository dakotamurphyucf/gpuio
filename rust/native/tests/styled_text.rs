use gpuio_native::tree::Tree;
use gpuio_protocol::{
    NodeId, WindowId,
    text_content::{Content, Span},
    v1::*,
};

fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn tx(tree: &Tree, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    }
}
fn initial(kind: Kind) -> Tree {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    tree.apply(&tx(
        &tree,
        vec![
            Op::Create(node(), kind, "Aé世界".into(), None),
            Op::SetRoot(Some(node())),
        ],
    ))
    .unwrap();
    tree
}
fn content() -> Content {
    Content {
        text: "Aé世界".into(),
        spans: vec![Span {
            start_byte: 1,
            end_byte: 3,
            foreground: 0xaabbccdd,
        }],
    }
}
#[test]
fn styled_text_is_atomic_typed_bounded_and_plain_text_clears_runs() {
    let mut tree = initial(Kind::Text);
    let plain_bytes = tree.retained_bytes();
    tree.apply(&tx(&tree, vec![Op::SetStyledText(node(), content())]))
        .unwrap();
    let expected_bytes = plain_bytes + std::mem::size_of::<Span>();
    assert_eq!(tree.retained_bytes(), expected_bytes);
    let original = tree.get(node()).unwrap().clone();
    let revision = tree.revision();
    let mut invalid = content();
    invalid.spans[0].start_byte = 2;
    assert_eq!(
        tree.apply(&tx(
            &tree,
            vec![
                Op::SetText(node(), "rollback".into()),
                Op::SetStyledText(node(), invalid)
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(tree.get(node()).unwrap(), &original);
    assert_eq!(tree.revision(), revision);
    assert_eq!(tree.retained_bytes(), expected_bytes);
    assert_eq!(
        tree.apply_with_budget(
            &tx(&tree, vec![Op::SetStyledText(node(), content())]),
            expected_bytes - 1
        ),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.get(node()).unwrap(), &original);
    assert_eq!(tree.revision(), revision);
    tree.apply_with_budget(
        &tx(&tree, vec![Op::SetStyledText(node(), content())]),
        expected_bytes,
    )
    .unwrap();
    tree.apply(&tx(&tree, vec![Op::SetText(node(), "Aé世界".into())]))
        .unwrap();
    assert!(tree.get(node()).unwrap().text_spans.is_empty());
    assert_eq!(tree.retained_bytes(), plain_bytes);
    tree.apply(&tx(
        &tree,
        vec![Op::SetStyledText(
            node(),
            Content {
                text: String::new(),
                spans: vec![],
            },
        )],
    ))
    .unwrap();
    assert!(tree.get(node()).unwrap().text.is_empty());
    tree.apply(&tx(&tree, vec![Op::SetRoot(None), Op::Remove(node())]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);

    for kind in [Kind::Container, Kind::Button] {
        let mut tree = initial(kind);
        let original = tree.get(node()).unwrap().clone();
        assert_eq!(
            tree.apply(&tx(&tree, vec![Op::SetStyledText(node(), content())])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.get(node()).unwrap(), &original);
    }
}
#[test]
fn replacement_drops_old_ranges_and_removed_generation_cannot_receive_them() {
    let mut tree = initial(Kind::Text);
    tree.apply(&tx(&tree, vec![Op::SetStyledText(node(), content())]))
        .unwrap();
    let next = NodeId::from_parts(0, 2).unwrap();
    tree.apply(&tx(
        &tree,
        vec![
            Op::Remove(node()),
            Op::Create(next, Kind::Text, "replacement".into(), None),
            Op::SetRoot(Some(next)),
        ],
    ))
    .unwrap();
    let retained = tree.retained_bytes();
    assert_eq!(
        tree.apply(&tx(&tree, vec![Op::SetStyledText(node(), content())])),
        Err(ErrorCode::StaleHandle)
    );
    assert!(tree.get(next).unwrap().text_spans.is_empty());
    assert_eq!(tree.get(next).unwrap().text.as_ref(), "replacement");
    assert_eq!(tree.retained_bytes(), retained);
}
