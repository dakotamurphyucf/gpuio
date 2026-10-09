use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, sheet_insets::Insets, v1::*};
#[test]
fn sheet_insets_require_sheet_ownership_and_validate_atomically() {
    let id = |n| NodeId::from_parts(n, 1).unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Geometry", 600., 500.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let overlay = OverlayConfig {
        kind: OverlayKind::SheetRight,
        label: "Point".into(),
        width: 200.,
        dismiss_on_escape: true,
        dismiss_on_outside_pointer: false,
    };
    let insets = Insets {
        top: 34.,
        right: 8.,
        bottom: 12.,
        left: 20.,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(id(0), Kind::Container, "".into(), None),
                Op::Create(id(1), Kind::Text, "Anchor".into(), None),
                Op::Create(
                    id(2),
                    Kind::FocusScope,
                    "".into(),
                    Some(HandlerId::from_parts(2, 1).unwrap()),
                ),
                Op::SetFocusScope(
                    id(2),
                    FocusScopeConfig {
                        trap: true,
                        auto_focus: true,
                        restore_focus: true,
                    },
                ),
                Op::SetOverlay(id(2), Some(overlay.clone())),
                Op::SetSheetInsets(id(2), Some(insets)),
                Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    for operations in [
        vec![Op::SetSheetInsets(id(0), Some(insets))],
        vec![Op::SetSheetInsets(
            id(2),
            Some(Insets { top: -1., ..insets }),
        )],
        vec![Op::SetSheetInsets(
            id(2),
            Some(Insets {
                top: f64::NAN,
                ..insets
            }),
        )],
        vec![Op::SetOverlay(id(2), None)],
        vec![
            Op::SetFocusScope(
                id(2),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                id(2),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    ..overlay.clone()
                }),
            ),
        ],
    ] {
        assert_eq!(
            session.apply(&tx(1, operations)),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.tree(window).unwrap().revision(), 1);
        assert_eq!(session.retained_bytes(), bytes);
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(id(2))
                .unwrap()
                .sheet_insets,
            Some(insets)
        );
    }
    session
        .apply(&tx(
            1,
            vec![Op::SetOverlay(id(2), None), Op::SetSheetInsets(id(2), None)],
        ))
        .unwrap();
    session
        .apply(&tx(
            2,
            vec![
                Op::SetRoot(None),
                Op::Remove(id(2)),
                Op::Remove(id(1)),
                Op::Remove(id(0)),
            ],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
