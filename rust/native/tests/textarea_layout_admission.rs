use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, text_area_layout::Config, v1::*};

fn id() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base,
        revision: base + 1,
        operations,
    }
}
fn mount(kind: Kind) -> Vec<Op> {
    vec![
        Op::Create(
            id(),
            kind,
            "incompatible seed".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetEditor(
            id(),
            EditorConfig {
                label: "Draft".into(),
                placeholder: "".into(),
                read_only: false,
                disabled: false,
                submit_on_enter: true,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::SetRoot(Some(id())),
    ]
}

#[test]
fn textarea_layout_admission_is_atomic_budgeted_and_preserves_seed() {
    let mut tree = Tree::new(tx(0, vec![]).window);
    tree.apply(&tx(0, mount(Kind::Textarea))).unwrap();
    let original = tree.get(id()).unwrap().clone();
    let bytes = tree.retained_bytes();
    let layout = Config {
        soft_wrap: false,
        cursor_margin_lines: Some(256),
        ..Config::default()
    };
    let change = tx(1, vec![Op::SetTextAreaLayout(id(), Some(layout))]);
    assert_eq!(
        tree.apply_with_budget(&change, bytes),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.get(id()).unwrap(), &original);
    assert_eq!(tree.revision(), 1);
    tree.apply(&change).unwrap();
    assert_eq!(tree.get(id()).unwrap().textarea_layout, Some(layout));
    assert_eq!(tree.get(id()).unwrap().text, original.text);
    let accepted = tree.get(id()).unwrap().clone();
    let charged = tree.retained_bytes();
    assert!(charged > bytes);
    for margin in [-1, 257, i64::MAX] {
        assert_eq!(
            tree.apply(&tx(
                2,
                vec![
                    Op::SetTextAreaLayout(id(), None),
                    Op::SetTextAreaLayout(
                        id(),
                        Some(Config {
                            cursor_margin_lines: Some(margin),
                            ..layout
                        })
                    ),
                ]
            )),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.get(id()).unwrap(), &accepted);
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.retained_bytes(), charged);
    }
    tree.apply(&tx(2, vec![Op::SetTextAreaLayout(id(), None)]))
        .unwrap();
    assert_eq!(tree.get(id()).unwrap(), &original);
    assert_eq!(tree.retained_bytes(), bytes);
    tree.apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(id())]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn textarea_layout_rejects_single_line_and_presentation_nodes() {
    for kind in [Kind::Input, Kind::Container] {
        for config in [None, Some(Config::default())] {
            let mut tree = Tree::new(tx(0, vec![]).window);
            assert_eq!(
                tree.apply(&tx(
                    0,
                    vec![
                        Op::Create(id(), kind, "".into(), None),
                        Op::SetTextAreaLayout(id(), config),
                        Op::SetRoot(Some(id())),
                    ]
                )),
                Err(ErrorCode::InvalidTree)
            );
            assert!(tree.is_empty());
        }
    }
}
