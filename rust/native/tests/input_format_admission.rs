use gpuio_native::tree::Tree;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    input_format::{Config, Number},
    v1::*,
};

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
    let mut ops = vec![
        Op::Create(
            id(),
            kind,
            "existing incompatible draft".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetEditor(
            id(),
            EditorConfig {
                label: "Draft".into(),
                placeholder: "Original placeholder".into(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::SetRoot(Some(id())),
    ];
    if kind == Kind::Combobox {
        ops.extend([
            Op::SetChoice(
                id(),
                ChoiceConfig {
                    label: "Draft".into(),
                    items: vec![],
                    selected: None,
                    disabled: false,
                },
            ),
            Op::SetComboboxFilter(id(), ComboboxFilter::Substring),
        ]);
    }
    ops
}

#[test]
fn format_changes_preserve_seed_and_charge_and_release_payload_atomically() {
    let mut tree = Tree::new(tx(0, vec![]).window);
    tree.apply(&tx(0, mount(Kind::Input))).unwrap();
    let original = tree.get(id()).unwrap().clone();
    let retained = tree.retained_bytes();
    let config = Config::Pattern("99-99".into());
    let change = tx(1, vec![Op::SetEditorFormat(id(), Some(config.clone()))]);
    assert_eq!(
        tree.apply_with_budget(&change, retained),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.get(id()).unwrap(), &original);
    assert_eq!(tree.retained_bytes(), retained);
    tree.apply(&change).unwrap();
    let formatted_retained = tree.retained_bytes();
    assert!(formatted_retained >= retained + config.retained_bytes());
    assert_eq!(tree.get(id()).unwrap().text, original.text);
    for invalid in [
        Config::Pattern("".into()),
        Config::Pattern("9".repeat(257)),
        Config::Number(Number {
            separator: Some("🙂🙂".into()),
            fraction_digits: None,
        }),
        Config::Number(Number {
            separator: None,
            fraction_digits: Some(-1),
        }),
    ] {
        let before = tree.get(id()).unwrap().clone();
        assert_eq!(
            tree.apply(&tx(
                2,
                vec![
                    Op::SetEditorFormat(id(), None),
                    Op::SetEditorFormat(id(), Some(invalid)),
                ]
            )),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.get(id()).unwrap(), &before);
        assert_eq!(tree.retained_bytes(), formatted_retained);
    }
    tree.apply(&tx(2, vec![Op::SetEditorFormat(id(), None)]))
        .unwrap();
    assert_eq!(tree.get(id()).unwrap(), &original);
    assert_eq!(tree.retained_bytes(), retained);
    tree.apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(id())]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn format_policy_is_only_for_ordinary_single_line_inputs() {
    for kind in [Kind::Textarea, Kind::Combobox] {
        let mut tree = Tree::new(tx(0, vec![]).window);
        tree.apply(&tx(0, mount(kind))).unwrap();
        let retained = tree.retained_bytes();
        for config in [None, Some(Config::Pattern("99".into()))] {
            assert_eq!(
                tree.apply(&tx(1, vec![Op::SetEditorFormat(id(), config)])),
                Err(ErrorCode::InvalidTree)
            );
            assert_eq!(tree.revision(), 1);
            assert_eq!(tree.retained_bytes(), retained);
        }
    }
    let mut tree = Tree::new(tx(0, vec![]).window);
    assert_eq!(
        tree.apply(&tx(
            0,
            vec![
                Op::Create(id(), Kind::Container, "".into(), None),
                Op::SetEditorFormat(id(), Some(Config::Pattern("99".into()))),
                Op::SetRoot(Some(id())),
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert!(tree.is_empty());
}
