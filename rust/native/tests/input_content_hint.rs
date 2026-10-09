use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, input_content_hint::Hint, v1::*};
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
#[test]
fn password_hint_admission_is_atomic_with_privacy_and_kind() {
    let mut tree = Tree::new(tx(0, vec![]).window);
    tree.apply(&tx(
        0,
        vec![
            Op::Create(
                id(),
                Kind::Input,
                "seed".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetEditor(
                id(),
                EditorConfig {
                    label: "Draft".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetRoot(Some(id())),
        ],
    ))
    .unwrap();
    for hint in [Hint::Password, Hint::NewPassword] {
        assert_eq!(
            tree.apply(&tx(1, vec![Op::SetEditorContentHint(id(), Some(hint))])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.get(id()).unwrap().editor_content_hint, None);
    }
    tree.apply(&tx(
        1,
        vec![
            Op::SetEditorPrivacy(id(), EditorPrivacy::PasswordHidden),
            Op::SetEditorContentHint(id(), Some(Hint::NewPassword)),
        ],
    ))
    .unwrap();
    assert_eq!(
        tree.apply(&tx(
            2,
            vec![Op::SetEditorPrivacy(id(), EditorPrivacy::Plain)]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(
        tree.get(id()).unwrap().editor_privacy,
        EditorPrivacy::PasswordHidden
    );
    tree.apply(&tx(
        2,
        vec![
            Op::SetEditorPrivacy(id(), EditorPrivacy::Plain),
            Op::SetEditorContentHint(id(), None),
        ],
    ))
    .unwrap();
    assert_eq!(tree.get(id()).unwrap().text.as_ref(), "seed");
    let mut tree = Tree::new(tx(0, vec![]).window);
    assert_eq!(
        tree.apply(&tx(
            0,
            vec![
                Op::Create(id(), Kind::Container, "".into(), None),
                Op::SetEditorContentHint(id(), Some(Hint::EmailAddress)),
                Op::SetRoot(Some(id()))
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert!(tree.is_empty());
}
