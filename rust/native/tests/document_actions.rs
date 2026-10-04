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
fn document_actions_require_rich_mode_handler_and_monotonic_epochs() {
    use gpuio_protocol::document_actions::{Action, Config};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = gpuio_protocol::HandlerId::from_parts(0, 1).unwrap();
    let config = Config {
        epoch: 1,
        observe: true,
        copy_code: false,
        copy_table: true,
        code: vec![Action {
            id: "run".into(),
            label: "Run".into(),
            enabled: true,
        }],
        table: vec![],
    };
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mut tree = Tree::new(window);
    for (kind, mode, handler) in [
        (Kind::Container, Mode::Markdown, Some(handler)),
        (Kind::DocumentView, Mode::Markdown, None),
        (Kind::DocumentView, Mode::Code("txt".into()), Some(handler)),
    ] {
        assert!(
            tree.apply(&tx(
                0,
                vec![
                    Op::Create(node, kind, String::new(), handler),
                    Op::SetDocumentActions(node, config.clone()),
                    Op::SetDocument(node, document(mode)),
                    Op::SetRoot(Some(node))
                ]
            ))
            .is_err()
        );
        assert!(tree.is_empty());
    }
    tree.apply(&tx(
        0,
        vec![
            Op::Create(node, Kind::DocumentView, String::new(), Some(handler)),
            Op::SetDocumentActions(node, config.clone()),
            Op::SetDocument(node, document(Mode::Markdown)),
            Op::SetRoot(Some(node)),
        ],
    ))
    .unwrap();
    let retained = tree.get(node).unwrap().document_actions.clone().unwrap();
    assert!(
        tree.apply(&tx(1, vec![Op::SetDocumentActions(node, config.clone())]))
            .is_err()
    );
    assert!(
        tree.apply(&tx(1, vec![Op::SetDocument(node, document(Mode::Diff))]))
            .is_err()
    );
    assert!(tree.apply(&tx(1, vec![Op::Bind(node, None)])).is_err());
    assert_eq!(tree.revision(), 1);
    assert_eq!(
        tree.get(node).unwrap().document_actions.as_ref().unwrap(),
        &retained
    );
    let clear = Config {
        epoch: 2,
        observe: false,
        copy_code: true,
        copy_table: true,
        code: vec![],
        table: vec![],
    };
    tree.apply(&tx(
        1,
        vec![
            Op::SetDocument(node, document(Mode::Diff)),
            Op::SetDocumentActions(node, clear),
            Op::Bind(node, None),
        ],
    ))
    .unwrap();
    assert!(
        tree.apply(&tx(
            2,
            vec![
                Op::SetDocumentActions(node, config.clone()),
                Op::SetDocument(node, document(Mode::Markdown)),
                Op::Bind(node, Some(handler))
            ]
        ))
        .is_err()
    );
    tree.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    let next = NodeId::from_parts(0, 2).unwrap();
    tree.apply(&tx(
        3,
        vec![
            Op::Create(next, Kind::DocumentView, String::new(), Some(handler)),
            Op::SetDocumentActions(next, config.clone()),
            Op::SetDocument(next, document(Mode::Html)),
            Op::SetRoot(Some(next)),
        ],
    ))
    .unwrap();
    assert!(
        tree.apply(&tx(4, vec![Op::SetDocumentActions(node, config)]))
            .is_err()
    );
}
