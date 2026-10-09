//! Actual deferred surface paint, hit testing and focus on TestPlatform.
use super::popover_semantics_test::{apply, ax, button, draw, events, handler, id, with_view};
use super::*;
use gpui::Entity;
use std::time::Duration;
fn open(kind: OverlayKind) -> Vec<Op> {
    let mut ops = vec![
        Op::Create(id(2), Kind::FocusScope, "".into(), Some(handler(2))),
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
                kind,
                label: "Moving modal".into(),
                width: 220.,
                dismiss_on_escape: true,
                dismiss_on_outside_pointer: kind != OverlayKind::AlertDialog,
            }),
        ),
        Op::SetOverlayMotion(id(2), true),
        Op::SetStyle(
            id(2),
            vec![
                Style::Height(Length::Px(320.)),
                Style::Fields(vec![Field::PaddingTop(Length::Px(120.))]),
                Style::Background(Color::Rgba(0x1144ffff)),
            ],
        ),
    ];
    ops.extend(button(3, "Keep"));
    ops.extend(button(4, "Confirm"));
    ops.extend([
        Op::Splice(id(2), 0, 0, vec![id(3), id(4)]),
        Op::Splice(id(0), 1, 0, vec![id(2)]),
    ]);
    ops
}
fn tick(cx: &mut gpui::VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    draw(cx);
}
fn logical_bounds(cx: &mut gpui::VisualTestContext, label: &str) -> gpui::accesskit::Rect {
    let bounds = ax(cx, label).1.bounds().unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    gpui::accesskit::Rect::new(
        bounds.x0 / scale,
        bounds.y0 / scale,
        bounds.x1 / scale,
        bounds.y1 / scale,
    )
}
fn panel(cx: &mut gpui::VisualTestContext) -> gpui::accesskit::Rect {
    logical_bounds(cx, "Moving modal")
}
fn close(owner: &Entity<View>, cx: &mut gpui::VisualTestContext) {
    apply(
        owner,
        cx,
        vec![
            Op::Splice(id(0), 1, 1, vec![]),
            Op::Remove(id(3)),
            Op::Remove(id(4)),
            Op::Remove(id(2)),
        ],
    );
}
#[test]
fn modal_entry_moves_the_surface_and_preserves_input_and_identity() {
    for kind in [
        OverlayKind::Dialog,
        OverlayKind::AlertDialog,
        OverlayKind::SheetLeft,
        OverlayKind::SheetRight,
        OverlayKind::SheetTop,
        OverlayKind::SheetBottom,
    ] {
        with_view(|owner, cx, transport| {
            cx.update(|_, cx| cx.set_reduce_motion(false));
            let original = ax(cx, "Open").0;
            cx.update(|w, cx| owner.update(cx, |v, cx| w.focus(&v.buttons[&id(1)].focus, cx)));
            apply(owner, cx, open(kind));
            let modal_id = ax(cx, "Moving modal").0;

            let first = panel(cx);
            // Full viewport blocking starts before any visible dialog opacity.
            events(transport);
            cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
                action: gpui::accesskit::Action::Click,
                target_node: original,
                target_tree: gpui::accesskit::TreeId::ROOT,
                data: None,
            });
            if matches!(kind, OverlayKind::Dialog | OverlayKind::AlertDialog) {
                let background =
                    owner.read_with(cx, |v, _| v.probes.borrow()[&id(1)].bounds.center());
                cx.simulate_click(background, gpui::Modifiers::default());
                draw(cx);
            }
            assert!(
                !events(transport)
                    .iter()
                    .any(|e| matches!(e, Event::Press(_, n, ..) if *n == id(1)))
            );
            let is_dialog = matches!(kind, OverlayKind::Dialog | OverlayKind::AlertDialog);
            tick(cx, if is_dialog { 125 } else { 75 });
            let mid = panel(cx);
            assert_ne!(mid, first, "surface must actually move: {kind:?}");
            assert!(ax(cx, "Moving modal").1.is_modal());
            assert_eq!(ax(cx, "Moving modal").0, modal_id);
            cx.update(|w, cx| {
                owner.read_with(cx, |v, _| {
                    assert!(
                        v.focus.borrow().handle(id(2)).unwrap().is_focused(w),
                        "scope holds entry focus {kind:?}"
                    );
                })
            });
            // Current geometry, not the final layout, owns native pointer hits.
            events(transport);
            let keep = logical_bounds(cx, "Keep");
            cx.simulate_click(
                gpui::point(
                    px(((keep.x0 + keep.x1) / 2.) as f32),
                    px(((keep.y0 + keep.y1) / 2.) as f32),
                ),
                gpui::Modifiers::default(),
            );
            draw(cx);
            let received = events(transport);
            assert!(
                received
                    .iter()
                    .any(|e| matches!(e,Event::Press(_,n,..) if *n==id(3))),
                "moving hitbox {kind:?}"
            );
            cx.simulate_keystrokes("tab");
            draw(cx);
            assert_eq!(cx.a11y_tree().unwrap().focus, ax(cx, "Confirm").0);
            // A paint update at the half-way point must not restart entry.
            apply(
                owner,
                cx,
                vec![Op::SetOverlayBackdrop(id(2), Some(0x22334480))],
            );
            assert_eq!(panel(cx), mid);
            cx.update(|w, _| {
                let expected: gpui::Hsla = rgba(0x1144ffff).into();
                let q = w
                    .painted_quads()
                    .into_iter()
                    .find(|q| {
                        q.background
                            .as_solid()
                            .is_some_and(|c| c.alpha(1.) == expected)
                    })
                    .expect("actual panel paint");
                assert!(
                    (q.background.as_solid().unwrap().a - if is_dialog { 0.5 } else { 1. }).abs()
                        < 0.01
                );
                assert!((q.bounds.origin.x.0 / w.scale_factor() - mid.x0 as f32).abs() < 0.01);
                assert!((q.bounds.origin.y.0 / w.scale_factor() - mid.y0 as f32).abs() < 0.01);
            });
            tick(cx, if is_dialog { 125 } else { 75 });
            let final_bounds = panel(cx);
            match kind {
                OverlayKind::SheetLeft => assert!((final_bounds.x0 - mid.x0 - 50.).abs() < 0.01),
                OverlayKind::SheetRight => assert!((mid.x0 - final_bounds.x0 - 50.).abs() < 0.01),
                OverlayKind::SheetTop => assert!((final_bounds.y0 - mid.y0 - 50.).abs() < 0.01),
                OverlayKind::SheetBottom => assert!((mid.y0 - final_bounds.y0 - 50.).abs() < 0.01),
                _ => assert!((final_bounds.y0 - mid.y0 - 8.).abs() < 0.01),
            }
            draw(cx);
            cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0, "settled entry is idle"));
            close(owner, cx);
            assert_eq!(cx.a11y_tree().unwrap().focus, original);
            assert!(
                !cx.a11y_tree()
                    .unwrap()
                    .nodes
                    .iter()
                    .any(|(_, n)| n.label() == Some("Moving modal"))
            );
        });
    }
}
#[test]
fn reduced_motion_disable_hide_and_early_close_retire_frames() {
    with_view(|owner, cx, _| {
        cx.update(|_, cx| cx.set_reduce_motion(false));
        apply(owner, cx, open(OverlayKind::Dialog));
        tick(cx, 50);
        let original = ax(cx, "Moving modal").0;
        cx.update(|w, cx| {
            cx.set_reduce_motion(true);
            w.refresh();
        });
        draw(cx);
        let settled = panel(cx);
        cx.update(|w, cx| {
            cx.set_reduce_motion(false);
            w.refresh();
        });
        draw(cx);
        assert_eq!(panel(cx), settled);
        assert_eq!(ax(cx, "Moving modal").0, original);
        apply(owner, cx, vec![Op::SetOverlayMotion(id(2), false)]);
        apply(owner, cx, vec![Op::SetOverlayMotion(id(2), true)]);
        assert_eq!(panel(cx), settled);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        // Hidden surface retires its frame state; showing again gets a new entry.
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                id(2),
                vec![Style::Fields(vec![Field::Display(3)])],
            )],
        );
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                id(2),
                vec![
                    Style::Height(Length::Px(320.)),
                    Style::Fields(vec![Field::PaddingTop(Length::Px(120.))]),
                    Style::Background(Color::Rgba(0x1144ffff)),
                ],
            )],
        );
        let reopened = panel(cx);
        assert_eq!(reopened.width(), settled.width());
        assert_eq!(reopened.height(), settled.height());
        assert!((settled.y0 - reopened.y0 - 16.).abs() < 0.01);
        close(owner, cx); // before entry finishes
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        assert!(owner.read_with(cx, |v, _| !v.buttons.contains_key(&id(3))));
    });
    with_view(|owner, cx, _| {
        cx.update(|_, cx| cx.set_reduce_motion(false));
        apply(owner, cx, open(OverlayKind::Dialog));
        tick(cx, 50);
        apply(owner, cx, vec![Op::SetOverlayMotion(id(2), false)]);
        let immediate = panel(cx);
        apply(owner, cx, vec![Op::SetOverlayMotion(id(2), true)]);
        assert_eq!(panel(cx), immediate);
        close(owner, cx);
    });
}

#[test]
fn top_sheet_focus_waits_for_unclipped_controls_then_resolves_without_input() {
    with_view(|owner, cx, _| {
        cx.update(|_, cx| cx.set_reduce_motion(false));
        let mut ops = open(OverlayKind::SheetTop);
        ops.push(Op::SetStyle(id(2), vec![]));
        apply(owner, cx, ops);
        tick(cx, 75);
        cx.update(|w, cx| {
            owner.read_with(cx, |v, _| {
                assert!(v.focus.borrow().handle(id(2)).unwrap().is_focused(w));
            })
        });
        tick(cx, 75);
        draw(cx);
        assert_eq!(cx.a11y_tree().unwrap().focus, ax(cx, "Keep").0);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        close(owner, cx);
    });
}
