use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[test]
fn entry_is_modal_only_and_transactional() {
    let node = NodeId::from_parts(1, 1).unwrap();
    let root = NodeId::from_parts(0, 1).unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Modal", 600., 500.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let config = OverlayConfig {
        kind: OverlayKind::Dialog,
        label: "Modal".into(),
        width: 200.,
        dismiss_on_escape: true,
        dismiss_on_outside_pointer: false,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(root, Kind::Container, "".into(), None),
                Op::Create(
                    node,
                    Kind::FocusScope,
                    "".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetFocusScope(
                    node,
                    FocusScopeConfig {
                        trap: true,
                        auto_focus: true,
                        restore_focus: true,
                    },
                ),
                Op::SetOverlay(node, Some(config.clone())),
                Op::SetOverlayMotion(node, true),
                Op::Splice(root, 0, 0, vec![node]),
                Op::SetRoot(Some(root)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    for ops in [
        vec![Op::SetOverlayMotion(root, true)],
        vec![Op::SetOverlay(node, None)],
        vec![Op::SetOverlay(
            node,
            Some(OverlayConfig {
                kind: OverlayKind::Popover,
                ..config.clone()
            }),
        )],
    ] {
        assert_eq!(session.apply(&tx(1, ops)), Err(ErrorCode::InvalidTree));
        assert_eq!(session.tree(window).unwrap().revision(), 1);
        assert_eq!(session.retained_bytes(), bytes);
        assert!(
            session
                .tree(window)
                .unwrap()
                .get(node)
                .unwrap()
                .overlay_motion
        );
    }
    session
        .apply(&tx(1, vec![Op::SetOverlayMotion(node, false)]))
        .unwrap();
    session
        .apply(&tx(
            2,
            vec![
                Op::SetOverlay(node, None),
                Op::SetOverlayMotion(node, false),
            ],
        ))
        .unwrap();
    session
        .apply(&tx(
            3,
            vec![Op::SetRoot(None), Op::Remove(node), Op::Remove(root)],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
