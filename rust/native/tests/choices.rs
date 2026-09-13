use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

fn config() -> ChoiceConfig {
    ChoiceConfig {
        label: "Mode".into(),
        items: vec![
            ChoiceItem {
                id: "fast".into(),
                label: "Fast".into(),
                disabled: false,
            },
            ChoiceItem {
                id: "deep".into(),
                label: "Deep".into(),
                disabled: true,
            },
        ],
        selected: Some("deep".into()),
        disabled: false,
    }
}
#[test]
fn choice_configuration_rejects_duplicates_missing_selection_and_unbounded_text() {
    let valid = config();
    assert!(valid.is_valid());
    assert!(!valid.can_select("deep"));
    assert!(valid.can_select("fast"));
    let mut duplicate = valid.clone();
    duplicate.items.push(duplicate.items[0].clone());
    assert!(!duplicate.is_valid());
    let mut missing = valid.clone();
    missing.selected = Some("missing".into());
    assert!(!missing.is_valid());
    let mut oversized = valid.clone();
    oversized.items[0].id = "x".repeat(257);
    assert!(!oversized.is_valid());
    let mut nul = valid;
    nul.label = "bad\0label".into();
    assert!(!nul.is_valid());
}

#[test]
fn native_selection_checks_current_options_and_invalid_config_rolls_back() {
    for kind in [Kind::RadioGroup, Kind::Select] {
        let window = WindowId::from_parts(0, 1).unwrap();
        let node = NodeId::from_parts(0, 1).unwrap();
        let handler = HandlerId::from_parts(0, 1).unwrap();
        let tx = |base, operations| Transaction {
            window,
            base,
            revision: base + 1,
            operations,
        };
        let mut session = Session::default();
        session.hello(VERSION, CAPABILITIES).unwrap();
        session.open(1, window, "choices", 100., 100.).unwrap();
        session
            .apply(&tx(
                0,
                vec![
                    Op::Create(node, kind, "".into(), Some(handler)),
                    Op::SetChoice(node, config()),
                    Op::SetRoot(Some(node)),
                ],
            ))
            .unwrap();
        assert!(session.choose(window, node, handler, 1, "fast").is_some());
        assert!(session.choose(window, node, handler, 1, "deep").is_none());
        let bytes = session.retained_bytes();
        let mut invalid = config();
        invalid.selected = Some("missing".into());
        assert_eq!(
            session.apply(&tx(
                1,
                vec![
                    Op::SetText(node, "must roll back".into()),
                    Op::SetChoice(node, invalid)
                ]
            )),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(node)
                .unwrap()
                .text
                .as_ref(),
            ""
        );
        assert_eq!(session.retained_bytes(), bytes);
        let mut next = config();
        next.items[0].disabled = true;
        session
            .apply(&tx(1, vec![Op::SetChoice(node, next)]))
            .unwrap();
        assert!(session.choose(window, node, handler, 1, "fast").is_none());
        session.close(window).unwrap();
        assert_eq!(session.retained_bytes(), 0);
    }
}
