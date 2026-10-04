use gpuio_native::tree::Tree;
use gpuio_protocol::{
    NodeId, WindowId,
    document::{Config as Document, Layout, Mode},
    document_style::Config,
    v1::*,
};
fn document(mode: Mode) -> Document {
    Document {
        source: None,
        mode,
        dark: false,
        layout: Layout::Flow,
        label: "Reader".into(),
        path: None,
        line_numbers: false,
        initially_collapsed: false,
        search: String::new(),
        images: vec![],
    }
}
#[test]
fn text_style_admission_is_atomic_and_clears_on_recycling() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mut tree = Tree::new(window);
    assert!(
        tree.apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Container, String::new(), None),
                Op::SetDocumentTextStyle(node, Some(Config::default())),
                Op::SetRoot(Some(node))
            ]
        ))
        .is_err()
    );
    assert!(tree.is_empty());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(node, Kind::DocumentView, String::new(), None),
            Op::SetDocument(node, document(Mode::Html)),
            Op::SetDocumentTextStyle(node, Some(Config::default())),
            Op::SetRoot(Some(node)),
        ],
    ))
    .unwrap();
    for style in [
        Config {
            paragraph_gap_rem: Some(65.),
            ..Default::default()
        },
        Config {
            table_cell: vec![Style::Fields(vec![Field::Opacity(2.)])],
            ..Default::default()
        },
        Config {
            table: vec![Style::Fields(vec![Field::Foreground(Color::Token(1))])],
            ..Default::default()
        },
    ] {
        assert!(
            tree.apply(&tx(1, vec![Op::SetDocumentTextStyle(node, Some(style))]))
                .is_err()
        );
        assert_eq!(tree.revision(), 1);
        assert_eq!(
            tree.get(node).unwrap().document_text_style.as_deref(),
            Some(&Config::default())
        );
    }
    assert!(
        tree.apply(&tx(1, vec![Op::SetDocument(node, document(Mode::Diff))]))
            .is_err()
    );
    tree.apply(&tx(
        1,
        vec![
            Op::SetDocument(node, document(Mode::Diff)),
            Op::SetDocumentTextStyle(node, None),
        ],
    ))
    .unwrap();
    assert!(tree.get(node).unwrap().document_text_style.is_none());
    tree.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    let next = NodeId::from_parts(0, 2).unwrap();
    tree.apply(&tx(
        3,
        vec![
            Op::Create(next, Kind::DocumentView, String::new(), None),
            Op::SetDocument(next, document(Mode::Markdown)),
            Op::SetRoot(Some(next)),
        ],
    ))
    .unwrap();
    assert!(tree.get(next).unwrap().document_text_style.is_none());
    assert!(
        tree.apply(&tx(
            4,
            vec![Op::SetDocumentTextStyle(node, Some(Config::default()))]
        ))
        .is_err()
    );
}
