//! Real retained modal, paint and input routing on TestPlatform; no OS window.
use super::popover_semantics_test::{apply, ax, button, draw, events, handler, id, with_view};
use super::*;
#[test]
fn backdrop_paint_updates_keep_modal_focus_and_block_background_actions() {
    for kind in [
        OverlayKind::Dialog,
        OverlayKind::AlertDialog,
        OverlayKind::SheetLeft,
        OverlayKind::SheetRight,
        OverlayKind::SheetTop,
        OverlayKind::SheetBottom,
    ] {
        with_view(|owner, cx, transport| {
            let original = ax(cx, "Open").0;
            let point = owner.read_with(cx, |v, _| v.probes.borrow()[&id(1)].bounds.center());
            cx.simulate_click(point, gpui::Modifiers::default());
            draw(cx);
            events(transport);
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
                        label: "Modal".into(),
                        width: 220.,
                        dismiss_on_escape: true,
                        dismiss_on_outside_pointer: false,
                    }),
                ),
                Op::SetStyle(
                    id(2),
                    vec![
                        Style::Height(Length::Px(160.)),
                        Style::Background(Color::Rgba(0xffffffff)),
                    ],
                ),
            ];
            ops.extend(button(3, "Keep"));
            ops.extend(button(4, "Confirm"));
            ops.extend([
                Op::Splice(id(2), 0, 0, vec![id(3), id(4)]),
                Op::Splice(id(0), 1, 0, vec![id(2)]),
            ]);
            apply(owner, cx, ops);
            let focused = ax(cx, "Keep").0;
            assert_eq!(cx.a11y_tree().unwrap().focus, focused);
            for color in [Some(0x11223380), Some(0), None] {
                apply(owner, cx, vec![Op::SetOverlayBackdrop(id(2), color)]);
                assert_eq!(cx.a11y_tree().unwrap().focus, focused);
                assert!(ax(cx, "Modal").1.is_modal());
                cx.update(|w, _| {
                    let expected: gpui::Background =
                        rgba(color.unwrap_or(0x00000080) as u32).into();
                    let viewport = w.viewport_size();
                    let painted = w.painted_quads().iter().any(|q| {
                        q.background == expected
                            && q.bounds.size.width.0 == f32::from(viewport.width) * w.scale_factor()
                            && q.bounds.size.height.0
                                == f32::from(viewport.height) * w.scale_factor()
                    });
                    if color != Some(0) {
                        assert!(painted, "viewport backdrop {kind:?} {color:?}");
                    } else {
                        assert!(
                            !w.painted_quads()
                                .iter()
                                .any(|q| !q.background.is_transparent()
                                    && q.bounds.size.width.0
                                        == f32::from(viewport.width) * w.scale_factor()
                                    && q.bounds.size.height.0
                                        == f32::from(viewport.height) * w.scale_factor()),
                            "transparent modal must not paint a viewport fill"
                        );
                    }
                });
                events(transport);
                // An old accessible target cannot activate through a transparent modal.
                cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
                    action: gpui::accesskit::Action::Click,
                    target_node: original,
                    target_tree: gpui::accesskit::TreeId::ROOT,
                    data: None,
                });
                cx.simulate_click(point, gpui::Modifiers::default());
                draw(cx);
                assert!(
                    !events(transport)
                        .iter()
                        .any(|e| matches!(e,Event::Press(_,node,..) if *node==id(1)))
                );
                for _ in 0..3 {
                    cx.simulate_keystrokes("tab");
                    draw(cx);
                    assert_ne!(cx.a11y_tree().unwrap().focus, original);
                }
                cx.simulate_keystrokes("shift-tab");
                draw(cx);
                // Restore a known child focus before the next appearance-only update.
                cx.update(|w, cx| owner.update(cx, |v, cx| w.focus(&v.buttons[&id(3)].focus, cx)));
                draw(cx);
            }
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
            assert_eq!(cx.a11y_tree().unwrap().focus, original);
        });
    }
}
