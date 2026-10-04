use gpuio_native::tree::Tree;
use gpuio_protocol::{
    NodeId, WindowId,
    document::{Config as Document, Layout, Mode},
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
fn markdown_options_are_atomic_mode_scoped_and_reset_on_reuse() {
    use gpuio_protocol::document::{Frontmatter, MarkdownOptions};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let options = MarkdownOptions {
        frontmatter: Frontmatter::CodeBlock,
        mdx: true,
    };
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
                Op::SetDocumentMarkdownOptions(node, options),
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
            Op::SetDocumentMarkdownOptions(node, options),
            Op::SetDocument(node, document(Mode::Markdown)),
            Op::SetRoot(Some(node)),
        ],
    ))
    .unwrap();
    assert_eq!(tree.get(node).unwrap().document_markdown_options, options);
    for mode in [Mode::Html, Mode::Diff, Mode::Code("txt".into())] {
        assert!(
            tree.apply(&tx(1, vec![Op::SetDocument(node, document(mode.clone()))]))
                .is_err()
        );
        assert_eq!(tree.revision(), 1);
        assert_eq!(
            tree.get(node).unwrap().document.as_ref().unwrap().mode,
            Mode::Markdown
        );
    }
    tree.apply(&tx(
        1,
        vec![
            Op::SetDocument(node, document(Mode::Html)),
            Op::SetDocumentMarkdownOptions(node, Default::default()),
        ],
    ))
    .unwrap();
    assert_eq!(
        tree.get(node).unwrap().document_markdown_options,
        MarkdownOptions::default()
    );
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
    assert_eq!(
        tree.get(next).unwrap().document_markdown_options,
        MarkdownOptions::default()
    );
    assert!(
        tree.apply(&tx(4, vec![Op::SetDocumentMarkdownOptions(node, options)]))
            .is_err()
    );
}
