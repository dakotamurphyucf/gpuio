use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    placement_geometry::{Config, Corner, Point},
    v1::*,
};
#[test]
fn geometry_is_bounded_targeted_and_final_tree_atomic() {
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
        kind: OverlayKind::Popover,
        label: "Point".into(),
        width: 200.,
        dismiss_on_escape: true,
        dismiss_on_outside_pointer: false,
    };
    let geometry = Config {
        viewport_margin: 16.,
        point: Some(Point {
            corner: Corner::BottomRight,
            x: 250.,
            y: 200.,
        }),
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
                        trap: false,
                        auto_focus: true,
                        restore_focus: true,
                    },
                ),
                Op::SetOverlay(id(2), Some(overlay.clone())),
                Op::SetPlacementGeometry(id(2), Some(geometry)),
                Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    for operations in [
        vec![Op::SetPlacementGeometry(id(0), Some(geometry))],
        vec![Op::SetPlacementGeometry(
            id(2),
            Some(Config {
                viewport_margin: -1.,
                ..geometry
            }),
        )],
        vec![Op::SetPlacementGeometry(
            id(2),
            Some(Config {
                viewport_margin: f64::NAN,
                ..geometry
            }),
        )],
        vec![Op::SetPlacementGeometry(
            id(2),
            Some(Config {
                point: Some(Point {
                    x: 1_000_001.,
                    ..geometry.point.unwrap()
                }),
                ..geometry
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
                .placement_geometry,
            Some(geometry)
        );
    }
    session
        .apply(&tx(
            1,
            vec![
                Op::SetOverlay(id(2), None),
                Op::SetPlacementGeometry(id(2), None),
            ],
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
