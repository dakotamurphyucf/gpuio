use gpuio_native::tree::Tree;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    input_validation::{Matching, Rule, Source},
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
fn rule(pattern: &str) -> Rule {
    Rule {
        regex: Source {
            pattern: pattern.into(),
            matching: Matching::WholeValue,
            case_sensitive: true,
        },
        allow_empty: true,
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
fn edit_filter_compilation_admission_and_removal_are_atomic_and_budgeted() {
    let mut tree = Tree::new(tx(0, vec![]).window);
    tree.apply(&tx(0, mount(Kind::Input))).unwrap();
    let original = tree.get(id()).unwrap().clone();
    let bytes = tree.retained_bytes();
    let change = tx(1, vec![Op::SetEditorValidation(id(), Some(rule("[0-9]*")))]);
    assert_eq!(
        tree.apply_with_budget(&change, bytes),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.get(id()).unwrap(), &original);
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), bytes);
    tree.apply(&change).unwrap();
    assert_eq!(tree.get(id()).unwrap().text, original.text);
    let charged = tree.retained_bytes();
    assert!(charged > bytes + 1024 * 1024);
    let accepted = tree.get(id()).unwrap().clone();
    for (pattern, error) in [
        ("[", ErrorCode::InvalidTree),
        ("a{100000000}", ErrorCode::LimitExceeded),
        ("a\0", ErrorCode::InvalidTree),
    ] {
        assert_eq!(
            tree.apply(&tx(
                2,
                vec![
                    Op::SetEditorValidation(id(), None),
                    Op::SetEditorValidation(id(), Some(rule(pattern)))
                ]
            )),
            Err(error)
        );
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.get(id()).unwrap(), &accepted);
        assert_eq!(tree.retained_bytes(), charged);
    }
    tree.apply(&tx(2, vec![Op::SetEditorValidation(id(), None)]))
        .unwrap();
    assert_eq!(tree.get(id()).unwrap(), &original);
    assert_eq!(tree.retained_bytes(), bytes);
    tree.apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(id())]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn edit_filter_is_not_admitted_on_multiline_or_presentation_nodes() {
    let mut tree = Tree::new(tx(0, vec![]).window);
    tree.apply(&tx(0, mount(Kind::Textarea))).unwrap();
    for rule in [None, Some(rule(".*"))] {
        assert_eq!(
            tree.apply(&tx(1, vec![Op::SetEditorValidation(id(), rule)])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.revision(), 1);
    }
    let mut tree = Tree::new(tx(0, vec![]).window);
    assert_eq!(
        tree.apply(&tx(
            0,
            vec![
                Op::Create(id(), Kind::Container, "".into(), None),
                Op::SetEditorValidation(id(), Some(rule(".*"))),
                Op::SetRoot(Some(id()))
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert!(tree.is_empty());
}
