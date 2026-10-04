use gpuio_native::tree::Tree;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    document::{Config as Document, Layout, Mode},
    document_preview::Config,
    v1::*,
};
fn document(mode: Mode) -> Document {
    Document {
        source: None,
        mode,
        dark: false,
        layout: Layout::Flow,
        label: "Preview".into(),
        path: None,
        line_numbers: false,
        initially_collapsed: false,
        search: "".into(),
        images: vec![],
    }
}
#[test]
fn preview_admission_checks_final_mode_monotone_epochs_and_atomic_rollback() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mut tree = Tree::new(window);
    let limit = Config {
        epoch: 1,
        max_lines: Some(2),
        observe: true,
    };
    assert!(
        tree.apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Container, "".into(), Some(handler)),
                Op::SetDocumentPreview(node, limit.clone()),
                Op::SetRoot(Some(node))
            ]
        ))
        .is_err()
    );
    assert!(tree.is_empty());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(node, Kind::DocumentView, "".into(), Some(handler)),
            Op::SetDocument(node, document(Mode::Markdown)),
            Op::SetDocumentPreview(node, limit.clone()),
            Op::SetRoot(Some(node)),
        ],
    ))
    .unwrap();
    assert!(
        tree.apply(&tx(1, vec![Op::SetDocumentPreview(node, limit.clone())]))
            .is_err()
    );
    assert_eq!(tree.revision(), 1);
    tree.apply(&tx(
        1,
        vec![
            Op::SetDocument(node, document(Mode::Code("txt".into()))),
            Op::SetDocumentPreview(
                node,
                Config {
                    epoch: 2,
                    max_lines: None,
                    observe: false,
                },
            ),
        ],
    ))
    .unwrap();
    assert!(
        tree.apply(&tx(
            2,
            vec![Op::SetDocumentPreview(
                node,
                Config {
                    epoch: 3,
                    ..limit.clone()
                }
            )]
        ))
        .is_err()
    );
    assert_eq!(
        tree.get(node)
            .unwrap()
            .document_preview
            .as_ref()
            .unwrap()
            .epoch,
        2
    );
    let next = NodeId::from_parts(0, 2).unwrap();
    assert!(
        tree.apply(&tx(
            2,
            vec![
                Op::SetDocumentPreview(
                    node,
                    Config {
                        epoch: 3,
                        max_lines: None,
                        observe: false
                    }
                ),
                Op::SetDocumentPreview(next, limit.clone())
            ]
        ))
        .is_err()
    );
    assert_eq!(
        tree.get(node)
            .unwrap()
            .document_preview
            .as_ref()
            .unwrap()
            .epoch,
        2
    );
    tree.apply(&tx(
        2,
        vec![
            Op::SetRoot(None),
            Op::Remove(node),
            Op::Create(next, Kind::DocumentView, "".into(), None),
            Op::SetDocument(next, document(Mode::Markdown)),
            Op::SetRoot(Some(next)),
        ],
    ))
    .unwrap();
    assert!(tree.get(next).unwrap().document_preview.is_none());
}
