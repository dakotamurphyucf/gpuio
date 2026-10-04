use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn config(kind: OverlayKind) -> OverlayConfig {
    OverlayConfig {
        kind,
        label: "Preview".into(),
        width: 220.,
        dismiss_on_escape: true,
        dismiss_on_outside_pointer: true,
    }
}
fn apply(s: &mut Session, operations: Vec<Op>) -> Result<(), ErrorCode> {
    let base = s.tree(window()).unwrap().revision();
    s.apply(&Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    })
    .map(|_| ())
}
#[test]
fn popover_marker_validates_final_shape_and_child_only_changes_atomically() {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "Popover", 400., 300.).unwrap();
    apply(
        &mut s,
        vec![
            Op::Create(node(0), Kind::Container, "".into(), None),
            Op::Create(
                node(1),
                Kind::Button,
                "Open".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    )
    .unwrap();
    let baseline = s.retained_bytes();
    apply(&mut s, vec![Op::SetPopover(node(0), true)]).unwrap();
    assert_eq!(baseline, s.retained_bytes());
    assert_eq!(
        s.tree(window())
            .unwrap()
            .popover_for_trigger(node(1))
            .unwrap()
            .id,
        node(0)
    );
    for ops in [
        vec![Op::SetPopover(node(1), true)],
        vec![Op::Splice(node(0), 0, 1, vec![])],
        vec![
            Op::Create(node(2), Kind::Text, "not a popup".into(), None),
            Op::Splice(node(0), 1, 0, vec![node(2)]),
        ],
    ] {
        let revision = s.tree(window()).unwrap().revision();
        assert_eq!(apply(&mut s, ops), Err(ErrorCode::InvalidTree));
        assert_eq!(revision, s.tree(window()).unwrap().revision());
        assert_eq!(baseline, s.retained_bytes());
    }
    apply(
        &mut s,
        vec![
            Op::Create(
                node(2),
                Kind::FocusScope,
                "".into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::SetFocusScope(
                node(2),
                FocusScopeConfig {
                    trap: false,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(node(2), Some(config(OverlayKind::Popover))),
            Op::Splice(node(0), 1, 0, vec![node(2)]),
        ],
    )
    .unwrap();
    let before = s.retained_bytes();
    for ops in [
        vec![Op::SetOverlay(node(2), None)],
        vec![
            Op::SetFocusScope(
                node(2),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(node(2), Some(config(OverlayKind::Dialog))),
        ],
    ] {
        let revision = s.tree(window()).unwrap().revision();
        assert_eq!(apply(&mut s, ops), Err(ErrorCode::InvalidTree));
        assert_eq!(revision, s.tree(window()).unwrap().revision());
        assert_eq!(before, s.retained_bytes());
    }
    apply(
        &mut s,
        vec![Op::Splice(node(0), 1, 1, vec![]), Op::Remove(node(2))],
    )
    .unwrap();
    apply(&mut s, vec![Op::SetPopover(node(0), false)]).unwrap();
    assert!(
        s.tree(window())
            .unwrap()
            .popover_for_trigger(node(1))
            .is_none()
    );
    assert_eq!(baseline, s.retained_bytes());
}
