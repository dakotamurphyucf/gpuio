//! Native sheet geometry, modal input and retained entry on TestPlatform.
use super::popover_semantics_test::{apply, ax, button, draw, events, handler, id, with_view};
use super::*;
use gpuio_protocol::sheet_insets::Insets;
use std::time::Duration;
fn config(kind: OverlayKind) -> OverlayConfig {
    OverlayConfig {
        kind,
        label: "Inset drawer".into(),
        width: 220.,
        dismiss_on_escape: true,
        dismiss_on_outside_pointer: true,
    }
}
fn open(kind: OverlayKind, insets: Insets, animated: bool) -> Vec<Op> {
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
        Op::SetOverlay(id(2), Some(config(kind))),
        Op::SetSheetInsets(id(2), Some(insets)),
        Op::SetOverlayMotion(id(2), animated),
        Op::SetStyle(
            id(2),
            vec![
                Style::Background(Color::Rgba(0x1144ffff)),
                Style::Width(Length::Px(5000.)),
                Style::Height(Length::Px(5000.)),
                Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(3000.)),
                    Field::MarginTop(Length::Px(3000.)),
                ]),
            ],
        ),
    ];
    ops.extend(button(3, "Keep"));
    ops.extend([
        Op::Create(
            id(4),
            Kind::Input,
            "Retained draft".into(),
            Some(handler(4)),
        ),
        Op::SetEditor(
            id(4),
            EditorConfig {
                label: "Drawer draft".into(),
                placeholder: "".into(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::SetStyle(
            id(4),
            vec![
                Style::Width(Length::Px(160.)),
                Style::Height(Length::Px(32.)),
            ],
        ),
        Op::Splice(id(2), 0, 0, vec![id(3), id(4)]),
        Op::Splice(id(0), 1, 0, vec![id(2)]),
    ]);
    ops
}
fn bounds(cx: &mut gpui::VisualTestContext, label: &str) -> gpui::accesskit::Rect {
    let b = ax(cx, label).1.bounds().unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    gpui::accesskit::Rect::new(b.x0 / scale, b.y0 / scale, b.x1 / scale, b.y1 / scale)
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 0.03, "{a} != {b}");
}
fn close(owner: &gpui::Entity<View>, cx: &mut gpui::VisualTestContext) {
    apply(
        owner,
        cx,
        vec![
            Op::Splice(id(0), 1, 1, vec![]),
            Op::Remove(id(4)),
            Op::Remove(id(3)),
            Op::Remove(id(2)),
        ],
    );
}

#[test]
fn all_edges_reserve_space_but_keep_full_modal_blocking_and_retained_children() {
    for (kind, x, y, width, height) in [
        (OverlayKind::SheetLeft, 20., 34., 220., 354.),
        (OverlayKind::SheetRight, 372., 34., 220., 354.),
        (OverlayKind::SheetTop, 20., 34., 572., 220.),
        (OverlayKind::SheetBottom, 20., 168., 572., 220.),
    ] {
        with_view(|owner, cx, transport| {
            cx.simulate_resize(gpui::size(px(600.), px(400.)));
            cx.update(|w, cx| {
                let focus = owner.read_with(cx, |v, _| v.buttons[&id(1)].focus.clone());
                w.focus(&focus, cx);
            });
            apply(
                owner,
                cx,
                open(
                    kind,
                    Insets {
                        top: 34.,
                        right: 8.,
                        bottom: 12.,
                        left: 20.,
                    },
                    false,
                ),
            );
            let stable = ax(cx, "Inset drawer").0;
            let b = bounds(cx, "Inset drawer");
            near(b.x0, x);
            near(b.y0, y);
            near(b.width(), width);
            near(b.height(), height);
            assert!(ax(cx, "Inset drawer").1.is_modal());
            assert_eq!(cx.a11y_tree().unwrap().focus, ax(cx, "Keep").0);
            let retained = owner.read_with(cx, |v, cx| v.editors[&id(4)].focus_handle(cx));
            let button = owner.read_with(cx, |v, _| v.buttons[&id(3)].clone());
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
            cx.simulate_click(gpui::point(px(5.), px(5.)), gpui::Modifiers::default());
            draw(cx);
            let received = events(transport);
            assert!(!received.iter().any(|e| matches!(e, Event::Press(..))));
            assert!(received.iter().any(|e|matches!(e,Event::OverlayDismissed(_,n,_,_,Dismissal::OutsidePointer) if *n==id(2))));
            assert_eq!(ax(cx, "Inset drawer").0, stable); // Dismissal is still only a request.
            apply(owner, cx, vec![Op::SetSheetInsets(id(2), None)]);
            assert_eq!(ax(cx, "Inset drawer").0, stable);
            assert_eq!(
                owner.read_with(cx, |v, cx| v.editors[&id(4)].focus_handle(cx)),
                retained
            );
            assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&button, &v.buttons[&id(3)])));
            let b = bounds(cx, "Inset drawer");
            match kind {
                OverlayKind::SheetLeft | OverlayKind::SheetTop => {
                    near(b.x0, 0.);
                    near(b.y0, 0.);
                }
                OverlayKind::SheetRight => {
                    near(b.x1, 600.);
                    near(b.y0, 0.);
                }
                OverlayKind::SheetBottom => {
                    near(b.x0, 0.);
                    near(b.y1, 400.);
                }
                _ => unreachable!(),
            }
            let keep = bounds(cx, "Keep");
            events(transport);
            cx.simulate_click(
                gpui::point(
                    px(((keep.x0 + keep.x1) / 2.) as f32),
                    px(((keep.y0 + keep.y1) / 2.) as f32),
                ),
                gpui::Modifiers::default(),
            );
            draw(cx);
            assert!(
                events(transport)
                    .iter()
                    .any(|e| matches!(e,Event::Press(_,n,..) if *n==id(3)))
            );
            close(owner, cx);
            assert_eq!(cx.a11y_tree().unwrap().focus, ax(cx, "Open").0);
            draw(cx);
            cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        });
    }
}

#[test]
fn resize_compresses_insets_and_extent_without_bridge_updates() {
    with_view(|owner, cx, transport| {
        cx.simulate_resize(gpui::size(px(600.), px(400.)));
        apply(
            owner,
            cx,
            open(
                OverlayKind::SheetRight,
                Insets {
                    top: 100.,
                    right: 200.,
                    bottom: 100.,
                    left: 100.,
                },
                false,
            ),
        );
        cx.simulate_resize(gpui::size(px(100.), px(80.)));
        draw(cx);
        draw(cx);
        let b = bounds(cx, "Inset drawer");
        near(b.x0, 33.);
        near(b.y0, 39.5);
        near(b.width(), 1.);
        near(b.height(), 1.);
        events(transport);
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert!(
            events(transport).iter().any(
                |e| matches!(e,Event::OverlayDismissed(_,n,_,_,Dismissal::Escape) if *n==id(2))
            ),
            "tiny sheet must remain keyboard-dismissable"
        );
        cx.simulate_resize(gpui::size(px(800.), px(600.)));
        draw(cx);
        draw(cx);
        let b = bounds(cx, "Inset drawer");
        near(b.x0, 380.);
        near(b.y0, 100.);
        near(b.width(), 220.);
        near(b.height(), 400.);
        let retained = owner.read_with(cx, |v, cx| v.editors[&id(4)].focus_handle(cx));
        apply(
            owner,
            cx,
            vec![Op::SetOverlay(
                id(2),
                Some(config(OverlayKind::SheetBottom)),
            )],
        );
        let b = bounds(cx, "Inset drawer");
        near(b.x0, 100.);
        near(b.y0, 280.);
        near(b.width(), 500.);
        near(b.height(), 220.);
        assert_eq!(
            owner.read_with(cx, |v, cx| v.editors[&id(4)].focus_handle(cx)),
            retained
        );
        close(owner, cx);
    });
}

#[test]
fn inset_updates_preserve_entry_clock_and_settled_autofocus() {
    with_view(|owner, cx, _| {
        cx.simulate_resize(gpui::size(px(600.), px(400.)));
        cx.update(|_, cx| cx.set_reduce_motion(false));
        apply(
            owner,
            cx,
            open(
                OverlayKind::SheetTop,
                Insets {
                    top: 34.,
                    ..Insets::default()
                },
                true,
            ),
        );
        cx.executor().advance_clock(Duration::from_millis(75));
        draw(cx);
        let stable = ax(cx, "Inset drawer").0;
        let before = bounds(cx, "Inset drawer");
        near(before.y0, -16.);
        apply(
            owner,
            cx,
            vec![Op::SetSheetInsets(
                id(2),
                Some(Insets {
                    top: 60.,
                    ..Insets::default()
                }),
            )],
        );
        let after = bounds(cx, "Inset drawer");
        near(after.y0, 10.);
        assert_eq!(ax(cx, "Inset drawer").0, stable);
        cx.executor().advance_clock(Duration::from_millis(75));
        draw(cx);
        draw(cx);
        near(bounds(cx, "Inset drawer").y0, 60.);
        assert_eq!(cx.a11y_tree().unwrap().focus, ax(cx, "Keep").0);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        close(owner, cx);
    });
}

#[test]
fn overlapping_hover_and_focus_decorations_cannot_expand_the_sheet_box() {
    with_view(|owner, cx, _| {
        cx.simulate_resize(gpui::size(px(600.), px(400.)));
        apply(
            owner,
            cx,
            open(
                OverlayKind::SheetRight,
                Insets {
                    top: 34.,
                    right: 8.,
                    bottom: 12.,
                    left: 20.,
                },
                false,
            ),
        );
        let stable = ax(cx, "Inset drawer").0;
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                id(2),
                vec![
                    Style::Fields(vec![Field::PaddingRight(Length::Percent(80.))]),
                    Style::State(
                        1,
                        vec![
                            Field::PaddingTop(Length::Percent(100.)),
                            Field::BorderLeftWidth(500.),
                        ],
                    ),
                    Style::State(
                        2,
                        vec![
                            Field::PaddingRight(Length::Px(1000.)),
                            Field::BorderBottomWidth(2000.),
                            Field::Background(Fill::Solid(Color::Rgba(0x2255aaff))),
                        ],
                    ),
                ],
            )],
        );
        cx.update(|w, cx| {
            let focus = owner.read_with(cx, |v, _| v.focus.borrow().handle(id(2)).unwrap().clone());
            w.focus(&focus, cx);
        });
        cx.simulate_event(gpui::MouseMoveEvent {
            position: gpui::point(px(450.), px(200.)),
            pressed_button: None,
            modifiers: gpui::Modifiers::default(),
        });
        draw(cx);
        draw(cx);
        let b = bounds(cx, "Inset drawer");
        near(b.x0, 372.);
        near(b.y0, 34.);
        near(b.width(), 220.);
        near(b.height(), 354.);
        assert_eq!(ax(cx, "Inset drawer").0, stable);
        cx.update(|w, _| {
            let color: gpui::Hsla = rgba(0x2255aaff).into();
            assert!(
                w.painted_quads()
                    .into_iter()
                    .any(|q| q.background.as_solid() == Some(color)),
                "actual hover state must paint"
            );
        });
        apply(owner, cx, vec![Op::SetStyle(id(2), vec![])]);
        let b = bounds(cx, "Inset drawer");
        near(b.width(), 220.);
        near(b.height(), 354.);
        close(owner, cx);
    });
}
