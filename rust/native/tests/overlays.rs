use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[test]
fn overlay_validation_dismissal_policy_and_lifetime_are_atomic() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "overlay", 300., 200.).unwrap();
    let config = OverlayConfig {
        kind: OverlayKind::Dialog,
        label: "Settings".into(),
        width: 220.,
        dismiss_on_escape: true,
        dismiss_on_outside_pointer: false,
    };
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
                Op::Create(node, Kind::FocusScope, "".into(), Some(handler)),
                Op::SetFocusScope(
                    node,
                    FocusScopeConfig {
                        trap: true,
                        auto_focus: true,
                        restore_focus: true,
                    },
                ),
                Op::SetOverlay(node, Some(config.clone())),
                Op::SetRoot(Some(node)),
            ],
        ))
        .unwrap();
    let retained = session.retained_bytes();
    for invalid in [
        OverlayConfig {
            width: f64::NAN,
            ..config.clone()
        },
        OverlayConfig {
            label: "".into(),
            ..config.clone()
        },
        OverlayConfig {
            width: 16385.,
            ..config.clone()
        },
        OverlayConfig {
            kind: OverlayKind::Popover,
            ..config.clone()
        },
    ] {
        assert_eq!(
            session.apply(&tx(1, vec![Op::SetOverlay(node, Some(invalid))])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), retained);
        assert_eq!(session.tree(window).unwrap().revision(), 1);
    }
    assert_eq!(
        session.apply(&tx(
            1,
            vec![Op::SetFocusScope(
                node,
                FocusScopeConfig {
                    trap: false,
                    auto_focus: true,
                    restore_focus: true
                }
            )]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert!(
        session
            .dismiss(window, node, handler, 1, Dismissal::Escape)
            .is_some()
    );
    assert!(
        session
            .dismiss(window, node, handler, 1, Dismissal::OutsidePointer)
            .is_none()
    );
    assert!(
        session
            .dismiss(window, node, handler, 2, Dismissal::Escape)
            .is_none()
    );
    assert!(
        session
            .dismiss(
                window,
                node,
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                Dismissal::Escape
            )
            .is_none()
    );
    for offset in [f64::NAN, f64::INFINITY, -16385., 16385.] {
        assert_eq!(
            session.apply(&tx(
                1,
                vec![Op::SetPlacement(
                    node,
                    Some(Placement {
                        offset,
                        ..Default::default()
                    })
                )]
            )),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), retained);
    }
    session
        .apply(&tx(
            1,
            vec![Op::SetPlacement(node, Some(Placement::default()))],
        ))
        .unwrap();
    assert!(session.retained_bytes() > retained);
    assert_eq!(
        session.apply(&tx(2, vec![Op::SetOverlay(node, None)])),
        Err(ErrorCode::InvalidTree)
    );
    session
        .apply(&tx(
            2,
            vec![
                Op::SetPlacement(node, None),
                Op::SetOverlay(node, None),
                Op::Bind(node, None),
            ],
        ))
        .unwrap();
    assert!(session.retained_bytes() < retained);
    assert!(
        session
            .dismiss(window, node, handler, 1, Dismissal::Escape)
            .is_none()
    );
    session
        .apply(&tx(3, vec![Op::Remove(node), Op::SetRoot(None)]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
