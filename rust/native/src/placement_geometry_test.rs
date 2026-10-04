//! Deferred geometry and input on TestPlatform; no physical OS acceptance.
use super::popover_semantics_test::{apply, ax, button, draw, events, handler, id, with_view};
use super::*;
use gpuio_protocol::placement_geometry::{Config, Corner, Point};

fn geometry(corner: Corner, x: f64, y: f64, margin: f64) -> Option<Config> {
    Some(Config {
        viewport_margin: margin,
        point: Some(Point { corner, x, y }),
    })
}
fn bounds(cx: &mut gpui::VisualTestContext, label: &str) -> gpui::accesskit::Rect {
    let b = ax(cx, label).1.bounds().unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    gpui::accesskit::Rect::new(b.x0 / scale, b.y0 / scale, b.x1 / scale, b.y1 / scale)
}
fn open(kind: OverlayKind) -> Vec<Op> {
    let mut ops = vec![
        Op::SetPopover(id(0), kind == OverlayKind::Popover),
        Op::Create(id(2), Kind::FocusScope, "".into(), Some(handler(2))),
        Op::SetFocusScope(
            id(2),
            FocusScopeConfig {
                trap: kind.is_modal(),
                auto_focus: true,
                restore_focus: true,
            },
        ),
        Op::SetOverlay(
            id(2),
            Some(OverlayConfig {
                kind,
                label: "Placed panel".into(),
                width: 240.,
                dismiss_on_escape: true,
                dismiss_on_outside_pointer: true,
            }),
        ),
        Op::SetStyle(
            id(2),
            vec![
                Style::Height(Length::Px(100.)),
                Style::Background(Color::Rgba(0x1144ffff)),
            ],
        ),
    ];
    ops.extend(button(3, "Inside"));
    ops.extend([
        Op::Splice(id(2), 0, 0, vec![id(3)]),
        Op::Splice(id(0), 1, 0, vec![id(2)]),
    ]);
    ops
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.02, "{actual} != {expected}");
}

#[test]
fn corner_changes_move_paint_hit_and_ax_without_replacing_focus_or_owners() {
    with_view(|owner, cx, transport| {
        let mut ops = open(OverlayKind::Popover);
        ops.push(Op::SetPlacementGeometry(
            id(2),
            geometry(Corner::TopLeft, 500., 400., 8.),
        ));
        apply(owner, cx, ops);
        let panel_id = ax(cx, "Placed panel").0;
        let inside = owner.read_with(cx, |v, _| v.buttons[&id(3)].clone());
        for (corner, x, y) in [
            (Corner::TopLeft, 500., 400.),
            (Corner::TopRight, 260., 400.),
            (Corner::BottomLeft, 500., 300.),
            (Corner::BottomRight, 260., 300.),
        ] {
            apply(
                owner,
                cx,
                vec![Op::SetPlacementGeometry(
                    id(2),
                    geometry(corner, 500., 400., 8.),
                )],
            );
            let b = bounds(cx, "Placed panel");
            near(b.x0, x);
            near(b.y0, y);
            near(b.width(), 240.);
            near(b.height(), 100.);
            assert_eq!(ax(cx, "Placed panel").0, panel_id);
            assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&inside, &v.buttons[&id(3)])));
            cx.update(|w, _| assert!(inside.focus.is_focused(w)));
            cx.update(|w, _| {
                let color: gpui::Hsla = rgba(0x1144ffff).into();
                let q = w
                    .painted_quads()
                    .into_iter()
                    .find(|q| q.background.as_solid() == Some(color))
                    .unwrap();
                near(f64::from(q.bounds.origin.x.0 / w.scale_factor()), x);
                near(f64::from(q.bounds.origin.y.0 / w.scale_factor()), y);
            });
            events(transport);
            let button = bounds(cx, "Inside");
            cx.simulate_click(
                gpui::point(
                    px(((button.x0 + button.x1) / 2.) as f32),
                    px(((button.y0 + button.y1) / 2.) as f32),
                ),
                gpui::Modifiers::default(),
            );
            draw(cx);
            assert!(matches!(events(transport).as_slice(),[Event::Press(_,n,..)] if *n==id(3)));
        }
        events(transport);
        cx.simulate_click(gpui::point(px(800.), px(700.)), gpui::Modifiers::default());
        draw(cx);
        assert!(
            matches!(events(transport).as_slice(),[Event::OverlayDismissed(_,n,_,_,Dismissal::OutsidePointer)] if *n==id(2))
        );
        apply(owner, cx, vec![Op::SetPlacementGeometry(id(2), None)]);
        let b = bounds(cx, "Placed panel");
        near(b.x0, 8.);
        near(b.y0, 30.);
        assert_eq!(ax(cx, "Placed panel").0, panel_id);
        // Outside-pointer focus is respected; focus inside before accepted close.
        cx.update(|w, cx| w.focus(&inside.focus, cx));
        draw(cx);
        apply(
            owner,
            cx,
            vec![
                Op::Splice(id(0), 1, 1, vec![]),
                Op::Remove(id(3)),
                Op::Remove(id(2)),
            ],
        );
        assert_eq!(cx.a11y_tree().unwrap().focus, ax(cx, "Open").0);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
    });
}

#[test]
fn point_clamping_uses_live_viewport_and_margin_without_a_transaction() {
    with_view(|owner, cx, _| {
        let mut ops = open(OverlayKind::Popover);
        ops.push(Op::SetPlacementGeometry(
            id(2),
            geometry(Corner::TopLeft, 1e6, 1e6, 24.),
        ));
        apply(owner, cx, ops);
        let (width, height) = cx.update(|w, _| {
            (
                f64::from(w.viewport_size().width),
                f64::from(w.viewport_size().height),
            )
        });
        let b = bounds(cx, "Placed panel");
        near(b.x1, width - 24.);
        near(b.y1, height - 24.);
        cx.simulate_resize(gpui::size(px(600.), px(400.)));
        draw(cx);
        draw(cx);
        let b = bounds(cx, "Placed panel");
        near(b.x0, 336.);
        near(b.y0, 276.);
        apply(
            owner,
            cx,
            vec![Op::SetPlacementGeometry(
                id(2),
                geometry(Corner::TopLeft, 300., 380., 8.),
            )],
        );
        let b = bounds(cx, "Placed panel");
        near(b.x0, 300.);
        near(b.y0, 292.); // Clamp, never flip to 280.
    });
}

#[test]
fn dialog_absolute_insets_compose_with_modal_focus_and_backdrop() {
    with_view(|owner, cx, transport| {
        let mut ops = open(OverlayKind::Dialog);
        ops.push(Op::SetStyle(
            id(2),
            vec![
                Style::Height(Length::Px(100.)),
                Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(70.)),
                    Field::Top(Length::Px(90.)),
                ]),
            ],
        ));
        apply(owner, cx, ops);
        let b = bounds(cx, "Placed panel");
        near(b.x0, 70.);
        near(b.y0, 90.);
        assert!(ax(cx, "Placed panel").1.is_modal());
        assert_eq!(cx.a11y_tree().unwrap().focus, ax(cx, "Inside").0);
        events(transport);
        cx.simulate_click(gpui::point(px(50.), px(15.)), gpui::Modifiers::default());
        draw(cx);
        assert!(
            !events(transport)
                .iter()
                .any(|e| matches!(e, Event::Press(..)))
        );
    });
}

#[test]
fn tooltip_and_hover_card_use_point_geometry_and_retain_their_native_state() {
    for kind in [Kind::Tooltip, Kind::HoverCard] {
        with_view(|owner, cx, _| {
            let mut ops = vec![
                Op::Create(id(2), kind, "".into(), Some(handler(2))),
                Op::SetTooltip(
                    id(2),
                    TooltipConfig {
                        label: "Placed help".into(),
                        width: 200.,
                        open_state: TooltipOpenState::Controlled(true),
                        disabled: false,
                        hoverable: true,
                        show_delay_ns: 0,
                        hide_delay_ns: 0,
                        skip_delay_ns: 0,
                    },
                ),
                Op::SetPlacementGeometry(id(2), geometry(Corner::TopRight, 600., 200., 8.)),
            ];
            ops.extend(button(3, "Help anchor"));
            ops.extend(button(4, "Help action"));
            ops.extend([
                Op::Splice(id(2), 0, 0, vec![id(3), id(4)]),
                Op::Splice(id(0), 1, 0, vec![id(2)]),
            ]);
            apply(owner, cx, ops);
            let stable = ax(cx, "Placed help").0;
            let action = owner.read_with(cx, |v, _| v.buttons[&id(4)].clone());
            let b = bounds(cx, "Placed help");
            near(b.x1, 600.);
            near(b.y0, 200.);
            apply(
                owner,
                cx,
                vec![Op::SetPlacementGeometry(
                    id(2),
                    geometry(Corner::BottomRight, 700., 500., 8.),
                )],
            );
            let b = bounds(cx, "Placed help");
            near(b.x1, 700.);
            near(b.y1, 500.);
            assert_eq!(ax(cx, "Placed help").0, stable);
            assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&action, &v.buttons[&id(4)])));
            apply(owner, cx, vec![Op::SetPlacementGeometry(id(2), None)]);
            let b = bounds(cx, "Placed help");
            assert!(b.y0 < 200.);
        });
    }
}
