use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, editor_frame::Config, v1::*};
fn id(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base,
        revision: base + 1,
        operations,
    }
}
fn frame() -> Config {
    Config {
        clear_label: Some("Clear".into()),
        loading: false,
        gap: 6.,
    }
}
fn mount(kind: Kind) -> Vec<Op> {
    let mut ops = vec![
        Op::Create(
            id(0),
            kind,
            "seed".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetEditor(
            id(0),
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
        Op::SetEditorFrame(id(0), Some(frame())),
    ];
    for i in 1..=4 {
        ops.push(Op::Create(id(i), Kind::Container, "".into(), None));
    }
    ops.extend([
        Op::Splice(id(0), 0, 0, (1..=4).map(id).collect()),
        Op::SetRoot(Some(id(0))),
    ]);
    ops
}
#[test]
fn frame_shape_and_descendant_only_changes_are_atomic() {
    let initial = tx(0, mount(Kind::Input));
    let mut tree = Tree::new(initial.window);
    tree.apply(&initial).unwrap();
    let retained = tree.retained_bytes();
    let invalid = vec![
        vec![Op::SetEditorFrame(id(0), None)],
        vec![Op::SetEditorFrame(
            id(0),
            Some(Config {
                loading: true,
                ..frame()
            }),
        )],
        vec![Op::SetEditorFrame(id(1), Some(frame()))],
        vec![Op::SetText(id(1), "ignored wrapper text".into())],
        vec![Op::SetStyle(id(1), vec![Style::Width(Length::Px(10.))])],
        vec![Op::Bind(id(1), Some(HandlerId::from_parts(1, 1).unwrap()))],
        vec![
            Op::Create(id(5), Kind::Text, "not spinner".into(), None),
            Op::Splice(id(2), 0, 0, vec![id(5)]),
        ],
        vec![
            Op::Create(
                id(5),
                Kind::Button,
                "Reveal".into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::Splice(id(3), 0, 0, vec![id(5)]),
        ],
    ];
    for ops in invalid {
        assert_eq!(tree.apply(&tx(1, ops)), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), retained);
    }
    let stamp = tree.get(id(0)).unwrap().editor_frame_activation_revision;
    tree.apply(&tx(1, vec![Op::SetEditorFrame(id(0), Some(frame()))]))
        .unwrap();
    assert_eq!(
        tree.get(id(0)).unwrap().editor_frame_activation_revision,
        stamp
    );
    tree.apply(&tx(
        2,
        vec![Op::SetEditorFrame(
            id(0),
            Some(Config { gap: 8., ..frame() }),
        )],
    ))
    .unwrap();
    assert_eq!(tree.get(id(0)).unwrap().editor_frame_activation_revision, 3);
    let mut remove = vec![
        Op::SetEditorFrame(id(0), None),
        Op::Splice(id(0), 0, 4, vec![]),
    ];
    remove.extend((1..=4).map(|i| Op::Remove(id(i))));
    tree.apply(&tx(3, remove)).unwrap();
    assert!(tree.get(id(0)).unwrap().editor_frame.is_none());
    assert_eq!(tree.get(id(0)).unwrap().text.as_ref(), "seed");
    assert!(tree.retained_bytes() < retained);
}
#[test]
fn frame_clear_is_single_line_but_adornments_also_support_textarea() {
    let mut tree = Tree::new(tx(0, vec![]).window);
    assert_eq!(
        tree.apply(&tx(0, mount(Kind::Textarea))),
        Err(ErrorCode::InvalidTree)
    );
    assert!(tree.is_empty());
    let mut ops = mount(Kind::Textarea);
    ops.push(Op::SetEditorFrame(
        id(0),
        Some(Config {
            clear_label: None,
            ..frame()
        }),
    ));
    tree.apply(&tx(0, ops)).unwrap();
}
