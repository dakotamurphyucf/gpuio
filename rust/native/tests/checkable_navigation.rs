use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    checkable::{Position, TabOrder},
    v1::*,
};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}

#[test]
fn standalone_radio_retains_control_but_fences_checked_disabled_and_retired_actions() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Radios", 200., 100.).unwrap();
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(0), Kind::Container, "".into(), None),
                Op::Create(node(1), Kind::Radio, "Choose".into(), Some(handler)),
                Op::SetControl(
                    node(1),
                    Control::Radio(false, Some(Position { index: 1, count: 2 }), false),
                ),
                Op::SetTabOrder(
                    node(1),
                    Some(TabOrder {
                        tab_stop: false,
                        index: -2,
                    }),
                ),
                Op::Create(node(2), Kind::Text, "Rich caption".into(), None),
                Op::Splice(node(1), 0, 0, vec![node(2)]),
                Op::Splice(node(0), 0, 0, vec![node(1)]),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    assert!(session.press(window, node(1), handler, 1).is_some());
    let retained = session.retained_bytes();
    for invalid in [
        Op::SetControl(
            node(1),
            Control::Radio(false, Some(Position { index: 2, count: 2 }), false),
        ),
        Op::SetTabOrder(
            node(1),
            Some(TabOrder {
                tab_stop: true,
                index: i64::MAX,
            }),
        ),
        Op::SetTabOrder(node(0), Some(TabOrder::default())),
        Op::Bind(node(2), Some(handler)),
    ] {
        assert!(
            session
                .apply(&tx(
                    1,
                    vec![Op::SetText(node(2), "Must roll back".into()), invalid]
                ))
                .is_err()
        );
        assert_eq!(session.retained_bytes(), retained);
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(node(2))
                .unwrap()
                .text
                .as_ref(),
            "Rich caption"
        );
    }
    session
        .apply(&tx(
            1,
            vec![
                Op::SetControl(node(1), Control::Radio(true, None, false)),
                Op::Bind(node(1), None),
            ],
        ))
        .unwrap();
    assert!(session.press(window, node(1), handler, 1).is_none());
    session
        .apply(&tx(
            2,
            vec![Op::SetControl(node(1), Control::Radio(false, None, true))],
        ))
        .unwrap();
    assert!(session.press(window, node(1), handler, 1).is_none());
    session
        .apply(&tx(
            3,
            vec![
                Op::SetControl(node(1), Control::Radio(false, None, false)),
                Op::Bind(node(1), Some(handler)),
                Op::SetTabOrder(node(1), None),
            ],
        ))
        .unwrap();
    assert!(session.press(window, node(1), handler, 4).is_some());
    assert_eq!(
        session
            .tree(window)
            .unwrap()
            .get(node(1))
            .unwrap()
            .tab_order,
        None
    );
    session.close(window).unwrap();
    assert!(session.press(window, node(1), handler, 4).is_none());
    assert_eq!(session.retained_bytes(), 0);
}
