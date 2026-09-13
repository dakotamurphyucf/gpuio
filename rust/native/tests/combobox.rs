use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

#[test]
fn editable_choices_require_coherent_configuration_and_reserve_editor_memory() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let config = ChoiceConfig {
        label: "Model".into(),
        items: vec![ChoiceItem {
            id: "a".into(),
            label: "Alpha".into(),
            disabled: false,
        }],
        selected: None,
        disabled: false,
    };
    let editor = EditorConfig {
        label: "Model".into(),
        placeholder: "Search".into(),
        disabled: false,
        read_only: false,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    };
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "combo", 100., 100.).unwrap();
    let mut create = vec![
        Op::Create(node, Kind::Combobox, "query".into(), Some(handler)),
        Op::SetChoice(node, config.clone()),
        Op::SetEditor(node, editor.clone()),
        Op::SetRoot(Some(node)),
    ];
    assert_eq!(
        session.apply(&tx(0, create.clone())),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(session.retained_bytes(), 0);
    create.push(Op::SetComboboxFilter(node, ComboboxFilter::Substring));
    session.apply(&tx(0, create)).unwrap();
    assert!(session.retained_bytes() >= EDITOR_RESERVED_BYTES);
    let before = session.retained_bytes();
    for invalid in [
        EditorConfig {
            disabled: true,
            ..editor.clone()
        },
        EditorConfig {
            read_only: true,
            ..editor.clone()
        },
        EditorConfig {
            label: "Different".into(),
            ..editor.clone()
        },
        EditorConfig {
            submit_on_enter: true,
            ..editor.clone()
        },
        EditorConfig {
            max_rows: 2,
            ..editor.clone()
        },
    ] {
        assert_eq!(
            session.apply(&tx(1, vec![Op::SetEditor(node, invalid)])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), before);
        assert_eq!(session.tree(window).unwrap().revision(), 1);
    }
    assert_eq!(
        session.apply(&tx(1, vec![Op::SetText(node, "replacement".into())])),
        Err(ErrorCode::InvalidTree)
    );
    session
        .apply(&tx(
            1,
            vec![
                Op::SetEditor(
                    node,
                    EditorConfig {
                        disabled: true,
                        ..editor
                    },
                ),
                Op::SetChoice(
                    node,
                    ChoiceConfig {
                        disabled: true,
                        ..config
                    },
                ),
            ],
        ))
        .unwrap();
    assert!(session.choose(window, node, handler, 1, "a").is_none());
    session
        .apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
