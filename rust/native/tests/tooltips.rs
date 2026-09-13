use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
#[test]
fn tooltip_structure_validation_visibility_events_and_disposal_are_atomic() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let config = TooltipConfig {
        label: "Details".into(),
        width: 200.,
        open_state: TooltipOpenState::Managed(false),
        disabled: false,
        hoverable: true,
        show_delay_ns: 250_000_000,
        hide_delay_ns: 80_000_000,
        skip_delay_ns: 300_000_000,
    };
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "tooltip", 300., 200.).unwrap();
    let mut ops = vec![
        Op::Create(node(0), Kind::Tooltip, "".into(), Some(handler)),
        Op::SetTooltip(node(0), config.clone()),
        Op::SetRoot(Some(node(0))),
    ];
    assert_eq!(
        session.apply(&tx(0, ops.clone())),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(session.retained_bytes(), 0);
    ops.extend([
        Op::Create(node(1), Kind::Text, "Anchor".into(), None),
        Op::Create(node(2), Kind::Text, "Content".into(), None),
        Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
    ]);
    session.apply(&tx(0, ops)).unwrap();
    assert_eq!(
        session.tree(window).unwrap().tooltip_description(node(1)),
        Some("Details")
    );
    assert_eq!(
        session.tree(window).unwrap().tooltip_description(node(2)),
        None
    );
    let retained = session.retained_bytes();
    for invalid in [
        TooltipConfig {
            width: f64::NAN,
            ..config.clone()
        },
        TooltipConfig {
            show_delay_ns: -1,
            ..config.clone()
        },
        TooltipConfig {
            hide_delay_ns: 60_000_000_001,
            ..config.clone()
        },
        TooltipConfig {
            skip_delay_ns: i64::MAX,
            ..config.clone()
        },
    ] {
        assert_eq!(
            session.apply(&tx(1, vec![Op::SetTooltip(node(0), invalid)])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), retained);
    }
    assert_eq!(
        session.apply(&tx(1, vec![Op::Splice(node(0), 1, 1, vec![])])),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(
        session
            .tree(window)
            .unwrap()
            .get(node(0))
            .unwrap()
            .children
            .len(),
        2
    );
    assert!(
        session
            .tooltip_open_changed(window, node(0), handler, 1, true)
            .is_some()
    );
    assert!(
        session
            .tooltip_open_changed(window, node(0), handler, 2, true)
            .is_none()
    );
    assert!(
        session
            .tooltip_open_changed(
                window,
                node(0),
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                true
            )
            .is_none()
    );
    session
        .apply(&tx(
            1,
            vec![Op::SetTooltip(
                node(0),
                TooltipConfig {
                    disabled: true,
                    ..config
                },
            )],
        ))
        .unwrap();
    assert!(
        session
            .tooltip_open_changed(window, node(0), handler, 1, true)
            .is_none()
    );
    assert!(
        session
            .tooltip_open_changed(window, node(0), handler, 1, false)
            .is_some()
    );
    session
        .apply(&tx(
            2,
            vec![
                Op::Remove(node(2)),
                Op::Remove(node(1)),
                Op::Remove(node(0)),
                Op::SetRoot(None),
            ],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
    assert!(
        session
            .tooltip_open_changed(window, node(0), handler, 1, false)
            .is_none()
    );
}
