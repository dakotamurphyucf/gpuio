use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn format_metadata_is_atomic_kind_checked_and_generation_scoped() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut tree = Tree::new(window);
    let transaction = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    assert_eq!(
        tree.apply(&transaction(
            0,
            vec![
                Op::Create(node, Kind::Container, "".into(), None),
                Op::SetDocumentSelectionFormat(node, true),
                Op::SetRoot(Some(node)),
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert!(tree.is_empty());
    tree.apply(&transaction(
        0,
        vec![
            Op::Create(node, Kind::DocumentView, "".into(), None),
            Op::SetDocument(
                node,
                gpuio_protocol::document::Config {
                    source: None,
                    mode: gpuio_protocol::document::Mode::Markdown,
                    dark: false,
                    layout: gpuio_protocol::document::Layout::Flow,
                    label: "Document".into(),
                    path: None,
                    line_numbers: false,
                    initially_collapsed: false,
                    search: "".into(),
                    images: vec![],
                },
            ),
            Op::SetDocumentSelectionFormat(node, true),
            Op::SetRoot(Some(node)),
        ],
    ))
    .unwrap();
    assert!(tree.get(node).unwrap().document_selection_markdown);
    let stale = NodeId::from_parts(0, 2).unwrap();
    assert!(
        tree.apply(&transaction(
            1,
            vec![
                Op::SetDocumentSelectionFormat(node, false),
                Op::SetDocumentSelectionFormat(stale, true),
            ]
        ))
        .is_err()
    );
    assert_eq!(tree.revision(), 1);
    assert!(tree.get(node).unwrap().document_selection_markdown);
    tree.apply(&transaction(
        1,
        vec![Op::SetDocumentSelectionFormat(node, false)],
    ))
    .unwrap();
    assert!(!tree.get(node).unwrap().document_selection_markdown);
    let config = tree
        .get(node)
        .unwrap()
        .document
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
    tree.apply(&transaction(
        2,
        vec![
            Op::SetDocumentSelectionFormat(node, true),
            Op::SetRoot(None),
            Op::Remove(node),
            Op::Create(stale, Kind::DocumentView, "".into(), None),
            Op::SetDocument(stale, config),
            Op::SetRoot(Some(stale)),
        ],
    ))
    .unwrap();
    assert!(
        !tree.get(stale).unwrap().document_selection_markdown,
        "a recycled slot starts in plain mode"
    );
}
