use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, control_appearance::Config, v1::*};

#[test]
fn appearance_updates_and_resets_preserve_handlers_and_rollback_invalid_parts() {
    for kind in [Kind::Checkbox, Kind::Switch, Kind::RadioGroup] {
        let window = WindowId::from_parts(0, 1).unwrap();
        let node = NodeId::from_parts(0, 1).unwrap();
        let handler = HandlerId::from_parts(0, 1).unwrap();
        let mut session = Session::default();
        session.hello(VERSION, CAPABILITIES).unwrap();
        session.open(1, window, "Control", 200., 200.).unwrap();
        let tx = |base, operations| Transaction {
            window,
            base,
            revision: base + 1,
            operations,
        };
        let value = match kind {
            Kind::Checkbox => Op::SetControl(node, Control::Checkbox(CheckState::Checked, false)),
            Kind::Switch => Op::SetControl(node, Control::Switch(true, false)),
            Kind::RadioGroup => Op::SetChoice(
                node,
                ChoiceConfig {
                    label: "Control".into(),
                    items: vec![ChoiceItem {
                        id: "one".into(),
                        label: "One".into(),
                        disabled: false,
                    }],
                    selected: Some("one".into()),
                    disabled: false,
                },
            ),
            _ => unreachable!(),
        };
        session
            .apply(&tx(
                0,
                vec![
                    Op::Create(node, kind, "Control".into(), Some(handler)),
                    value,
                    Op::SetRoot(Some(node)),
                ],
            ))
            .unwrap();
        let baseline = session.retained_bytes();
        let appearance = Config {
            size: 32.,
            switch_width: 64.,
            indicator_style: vec![Style::State(
                4,
                vec![Field::Foreground(Color::Rgba(0xff0000ff))],
            )],
            ..Config::default()
        };
        session
            .apply(&tx(
                1,
                vec![Op::SetControlAppearance(node, Some(appearance.clone()))],
            ))
            .unwrap();
        assert!(session.retained_bytes() > baseline);
        let bytes = session.retained_bytes();
        for invalid in [
            Config {
                size: 0.,
                ..appearance.clone()
            },
            Config {
                mark_style: vec![Style::Fields(vec![Field::Inert(true)])],
                ..appearance.clone()
            },
            Config {
                indicator_style: vec![Style::State(2, vec![])],
                ..appearance.clone()
            },
        ] {
            assert!(
                session
                    .apply(&tx(
                        2,
                        vec![
                            Op::SetText(node, "must rollback".into()),
                            Op::SetControlAppearance(node, Some(invalid))
                        ]
                    ))
                    .is_err()
            );
            assert_eq!(session.tree(window).unwrap().revision(), 2);
            assert_eq!(
                session
                    .tree(window)
                    .unwrap()
                    .get(node)
                    .unwrap()
                    .text
                    .as_ref(),
                "Control"
            );
            assert_eq!(session.retained_bytes(), bytes);
        }
        assert_eq!(
            session.tree(window).unwrap().get(node).unwrap().handler,
            Some(handler)
        );
        if kind == Kind::RadioGroup {
            assert!(session.choose(window, node, handler, 1, "one").is_some());
        } else {
            assert!(session.press(window, node, handler, 1).is_some());
        }
        session
            .apply(&tx(2, vec![Op::SetControlAppearance(node, None)]))
            .unwrap();
        assert_eq!(session.retained_bytes(), baseline);
        session
            .apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(node)]))
            .unwrap();
        assert_eq!(session.retained_bytes(), 0);
    }
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Wrong kind", 100., 100.).unwrap();
    for appearance in [None, Some(Config::default())] {
        assert!(
            session
                .apply(&Transaction {
                    window,
                    base: 0,
                    revision: 1,
                    operations: vec![
                        Op::Create(node, Kind::Text, "Wrong".into(), None),
                        Op::SetControlAppearance(node, appearance),
                        Op::SetRoot(Some(node))
                    ]
                })
                .is_err()
        );
        assert_eq!(session.retained_bytes(), 0);
    }
}
