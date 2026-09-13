use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

#[test]
fn choice_appearance_is_atomic_validated_and_charged_to_retained_memory() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "appearance", 400., 300.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Select, "".into(), Some(handler)),
                Op::SetChoice(
                    node,
                    ChoiceConfig {
                        label: "Mode".into(),
                        items: vec![ChoiceItem {
                            id: "deep".into(),
                            label: "Deep".into(),
                            disabled: false,
                        }],
                        selected: None,
                        disabled: false,
                    },
                ),
                Op::SetRoot(Some(node)),
            ],
        ))
        .unwrap();
    let baseline = session.retained_bytes();
    let valid = ChoiceAppearance {
        empty_label: "Keine Optionen".into(),
        option_style: vec![Style::Fields(vec![Field::FontFamily("x".repeat(256))])],
        ..Default::default()
    };
    session
        .apply(&tx(1, vec![Op::SetChoiceAppearance(node, valid.clone())]))
        .unwrap();
    let bytes = session.retained_bytes();
    assert!(
        bytes > baseline + 256,
        "nested style strings and records are retained payload"
    );
    let invalid = vec![
        ChoiceAppearance {
            option_style: vec![Style::Fields(vec![Field::FontFamily("x".repeat(257))])],
            ..valid.clone()
        },
        ChoiceAppearance {
            row_height: f64::NAN,
            ..valid.clone()
        },
        ChoiceAppearance {
            row_height: 0.,
            ..valid.clone()
        },
        ChoiceAppearance {
            max_visible_rows: 65,
            ..valid.clone()
        },
        ChoiceAppearance {
            empty_label: "bad\0label".into(),
            ..valid.clone()
        },
        ChoiceAppearance {
            empty_label: "x".repeat(1025),
            ..valid.clone()
        },
        ChoiceAppearance {
            popup_style: vec![Style::Fields(vec![Field::Width(Length::Px(40.))])],
            ..valid.clone()
        },
        ChoiceAppearance {
            option_style: vec![Style::Fields(vec![Field::PointerEvents(false)])],
            ..valid.clone()
        },
        ChoiceAppearance {
            empty_style: vec![Style::State(2, vec![Field::Opacity(0.5)])],
            ..valid.clone()
        },
        ChoiceAppearance {
            option_style: vec![Style::Fields(vec![Field::Opacity(0.5); 128])],
            popup_style: vec![Style::Fields(vec![Field::Opacity(0.5)])],
            ..valid.clone()
        },
    ];
    for appearance in invalid {
        assert!(
            session
                .apply(&tx(
                    2,
                    vec![
                        Op::SetText(node, "must roll back".into()),
                        Op::SetChoiceAppearance(node, appearance)
                    ]
                ))
                .is_err()
        );
        assert_eq!(session.retained_bytes(), bytes);
        let tree = session.tree(window).unwrap();
        let retained = tree.get(node).unwrap();
        assert_eq!(retained.text.as_ref(), "");
        assert_eq!(retained.choice_appearance.as_deref(), Some(&valid));
    }
    assert!(
        session.choose(window, node, handler, 1, "deep").is_some(),
        "appearance retains semantic handler"
    );
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
