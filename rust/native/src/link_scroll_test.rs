//! Offscreen native links share measured focus/reveal with compound controls.
use super::*;
use gpuio_protocol::{numeric::Domain, slider as s};

fn outer() -> NodeId {
    NodeId::from_parts(10, 2).unwrap()
}
fn box_style(width: f64, height: f64) -> Vec<Field> {
    vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ]
}
fn position(left: f64, top: f64, width: f64, height: f64) -> Vec<Style> {
    let mut fields = box_style(width, height);
    fields.extend([
        Field::Position(1),
        Field::Left(Length::Px(left)),
        Field::Top(Length::Px(top)),
    ]);
    vec![Style::Fields(fields)]
}
async fn frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    // The first frame measures; focus reveal requests one subsequent layout.
    super::super::editor_test::frame(cx, handle).await;
    super::super::editor_test::frame(cx, handle).await;
}
fn visible(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, node: NodeId) {
    handle
        .update(cx, |view, _, _| {
            let target = view.probes.borrow()[&node].bounds;
            for scroll in [outer(), id(0)] {
                let viewport = view.scrolls[&scroll].mask.get().unwrap();
                assert!(
                    target.left() >= viewport.left() - px(0.5)
                        && target.right() <= viewport.right() + px(0.5)
                        && target.top() >= viewport.top() - px(0.5)
                        && target.bottom() <= viewport.bottom() + px(0.5),
                    "target {node:?}: {target:?} outside {scroll:?}: {viewport:?}"
                );
            }
        })
        .unwrap();
}
fn offsets(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> [gpui::Point<gpui::Pixels>; 2] {
    handle
        .update(cx, |view, _, _| {
            [
                view.scrolls[&outer()].handle.offset(),
                view.scrolls[&id(0)].handle.offset(),
            ]
        })
        .unwrap()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut inner = box_style(320., 180.);
    inner.extend([
        Field::OverflowX(3),
        Field::OverflowY(3),
        Field::BorderTopWidth(4.),
        Field::BorderRightWidth(4.),
        Field::BorderBottomWidth(4.),
        Field::BorderLeftWidth(4.),
        Field::BorderColor(Color::Rgba(0x557799ff)),
        Field::MarginLeft(Length::Px(80.)),
        Field::MarginTop(Length::Px(120.)),
    ]);
    let mut viewport = box_style(220., 140.);
    viewport.extend([Field::OverflowX(3), Field::OverflowY(3)]);
    let mut operations = vec![
        Op::Create(outer(), Kind::Container, String::new(), None),
        Op::SetStyle(outer(), vec![Style::Fields(viewport)]),
        Op::SetStyle(id(0), vec![Style::Fields(inner)]),
        Op::Splice(outer(), 0, 0, vec![id(0)]),
        Op::Create(
            id(11),
            Kind::Slider,
            String::new(),
            Some(gpuio_protocol::HandlerId::from_parts(11, 1).unwrap()),
        ),
        Op::SetSlider(
            id(11),
            s::Config {
                domain: Domain::new(0., 100., 1.).unwrap(),
                label: "Linked range".into(),
                lower_label: "Lower limit".into(),
                upper_label: "Upper limit".into(),
                axis: s::Axis::Horizontal,
                scale: s::Scale::Linear,
                disabled: false,
                read_only: false,
            },
            s::Value::Range {
                lower: 20.,
                upper: 80.,
            },
        ),
        Op::SetStyle(id(11), position(360., 640., 140., 40.)),
        Op::Splice(id(0), 5, 0, vec![id(11)]),
        Op::Create(id(12), Kind::Container, String::new(), None),
        Op::Create(
            id(13),
            Kind::Button,
            "Outside trap".into(),
            Some(gpuio_protocol::HandlerId::from_parts(13, 1).unwrap()),
        ),
        Op::SetStyle(id(13), position(10., 230., 180., 40.)),
        Op::Splice(id(12), 0, 0, vec![outer(), id(13)]),
        Op::SetRoot(Some(id(12))),
    ];
    for (slot, index, left, top) in [
        (1, 10, 0., 0.),
        (2, 30, 360., 320.),
        (3, 20, 360., 400.),
        (4, 20, 0., 480.),
    ] {
        operations.push(Op::SetLink(id(slot), config(slot, index)));
        operations.push(Op::SetStyle(id(slot), position(left, top, 120., 50.)));
    }
    operations.push(Op::SetStyle(id(5), position(360., 560., 120., 40.)));
    apply(cx, handle, operations);
    frame(cx, handle).await;
    focus(cx, handle, 1);
    frame(cx, handle).await;
    visible(cx, handle, id(1));
    let initial = offsets(cx, handle);
    key(cx, handle, "tab");
    focused(cx, handle, 3);
    frame(cx, handle).await;
    visible(cx, handle, id(3));
    assert_ne!(
        offsets(cx, handle),
        initial,
        "offscreen target must move native scroll owners"
    );
    for slot in [4, 2] {
        key(cx, handle, "tab");
        focused(cx, handle, slot);
        frame(cx, handle).await;
        visible(cx, handle, id(slot));
    }
    for slot in [4, 3, 1] {
        key(cx, handle, "shift-tab");
        focused(cx, handle, slot);
        frame(cx, handle).await;
        visible(cx, handle, id(slot));
    }
    // Scrolling the focused node away is allowed and must not cause idle snapback.
    handle
        .update(cx, |view, window, _| {
            view.scrolls[&id(0)]
                .handle
                .set_offset(gpui::point(px(-100.), px(-300.)));
            window.refresh();
        })
        .unwrap();
    frame(cx, handle).await;
    let manual = offsets(cx, handle);
    frame(cx, handle).await;
    assert_eq!(offsets(cx, handle), manual);
    focused(cx, handle, 1);
    #[cfg(target_os = "macos")]
    {
        accessible(cx, handle, 1, AxAction::Focus);
        frame(cx, handle).await;
        visible(cx, handle, id(1));
    }
    assert!(
        presses(transport).is_empty(),
        "scroll/reveal never activates"
    );
    focus(cx, handle, 13);
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(14), Kind::FocusScope, String::new(), None),
            Op::SetFocusScope(
                id(14),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Splice(id(12), 0, 1, vec![id(14)]),
            Op::Splice(id(14), 0, 0, vec![outer()]),
        ],
    );
    frame(cx, handle).await;
    focused(cx, handle, 5);
    visible(cx, handle, id(5));
    for part in [0, 1] {
        key(cx, handle, "tab");
        frame(cx, handle).await;
        handle
            .update(cx, |view, window, _| {
                assert!(
                    view.sliders[&id(11)].borrow().focus[part]
                        .1
                        .is_focused(window),
                    "expected slider part {part}"
                );
            })
            .unwrap();
    }
    key(cx, handle, "tab");
    focused(cx, handle, 1);
    frame(cx, handle).await;
    visible(cx, handle, id(1));
    apply(
        cx,
        handle,
        vec![Op::SetLink(
            id(1),
            Config {
                tab_stop: false,
                ..config(1, 10)
            },
        )],
    );
    frame(cx, handle).await;
    focused(cx, handle, 1);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    focused(cx, handle, 3);
    focus(cx, handle, 1);
    frame(cx, handle).await;
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(view.sliders[&id(11)].borrow().focus[1].1.is_focused(window))
        })
        .unwrap();
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(view.sliders[&id(11)].borrow().focus[0].1.is_focused(window))
        })
        .unwrap();
    key(cx, handle, "shift-tab");
    focused(cx, handle, 5);
    frame(cx, handle).await;
    visible(cx, handle, id(5));
    key(cx, handle, "shift-tab");
    focused(cx, handle, 2);
    frame(cx, handle).await;
    visible(cx, handle, id(2));
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(14), 0, 1, vec![]),
            Op::Splice(id(12), 0, 1, vec![outer()]),
            Op::Remove(id(14)),
        ],
    );
    frame(cx, handle).await;
    focused(cx, handle, 13);
    // A fixed clip cannot reveal its excluded descendants. It must not turn
    // them into invisible stops, including when every tab index is zero.
    let mut clipped = box_style(320., 180.);
    clipped.extend([
        Field::OverflowX(2),
        Field::OverflowY(2),
        Field::MarginLeft(Length::Px(80.)),
        Field::MarginTop(Length::Px(120.)),
    ]);
    let mut update = vec![Op::SetStyle(id(0), vec![Style::Fields(clipped)])];
    update.extend((1..=4).map(|slot| Op::SetLink(id(slot), config(slot, 0))));
    apply(cx, handle, update);
    frame(cx, handle).await;
    focus(cx, handle, 13);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    focused(cx, handle, 1);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    focused(cx, handle, 13);
    #[cfg(target_os = "macos")]
    {
        accessible(cx, handle, 3, AxAction::Focus);
        frame(cx, handle).await;
        focused(cx, handle, 13);
    }
    // Index-zero fallback still needs the non-stop's anchor when other native
    // handles have been excluded by a fixed clip. Here order is 1, 2, outside.
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(2), position(130., 0., 120., 50.)),
            Op::SetLink(
                id(2),
                Config {
                    tab_stop: false,
                    ..config(2, 0)
                },
            ),
        ],
    );
    frame(cx, handle).await;
    focus(cx, handle, 2);
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    focused(cx, handle, 13);
    focus(cx, handle, 2);
    frame(cx, handle).await;
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    focused(cx, handle, 1);
    // Vertical scrolling cannot expose horizontal overflow. Negative content
    // beyond the clamped origin must not be made reachable by an outer scroller.
    let mut vertical = box_style(320., 180.);
    vertical.extend([
        Field::OverflowY(3),
        Field::MarginLeft(Length::Px(80.)),
        Field::MarginTop(Length::Px(120.)),
    ]);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(0), vec![Style::Fields(vertical)]),
            Op::SetLink(id(2), config(2, 0)),
            Op::SetStyle(id(2), position(0., 320., 420., 240.)),
            Op::SetStyle(id(4), position(-200., 0., 120., 50.)),
        ],
    );
    frame(cx, handle).await;
    focus(cx, handle, 13);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    focused(cx, handle, 1);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    focused(cx, handle, 2);
    handle
        .update(cx, |view, _, _| {
            let inner = &view.scrolls[&id(0)].handle;
            assert_eq!(
                inner.offset(),
                gpui::point(px(0.), px(-320.)),
                "oversized targets align their leading edge without cross-axis movement"
            );
            let target = view.probes.borrow()[&id(2)].bounds;
            let outer = view.scrolls[&outer()].handle.bounds();
            assert!((target.top() - outer.top()).abs() < px(0.5));
            assert!((target.left() - outer.left()).abs() < px(0.5));
        })
        .unwrap();
    key(cx, handle, "tab");
    frame(cx, handle).await;
    focused(cx, handle, 13);
    // Observe idle without requesting test frames; focus reveal must settle.
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    let settled = handle.update(cx, |view, _, _| view.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    assert_eq!(
        handle.update(cx, |view, _, _| view.render_count).unwrap(),
        settled,
        "focus reveal must not keep an idle window redrawing"
    );
    // Deferred modal paint has its own hierarchy, even when its retained node
    // belongs to scrolled content. Focusing it cannot move the underlying page.
    let modal = NodeId::from_parts(14, 2).unwrap();
    let before_modal = offsets(cx, handle);
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                modal,
                Kind::FocusScope,
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(14, 2).unwrap()),
            ),
            Op::SetFocusScope(
                modal,
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                modal,
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Floating reveal test".into(),
                    width: 240.,
                    dismiss_on_escape: false,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::Create(
                id(15),
                Kind::Button,
                "Floating action".into(),
                Some(gpuio_protocol::HandlerId::from_parts(15, 1).unwrap()),
            ),
            Op::SetStyle(id(15), vec![Style::Fields(box_style(120., 40.))]),
            Op::Splice(modal, 0, 0, vec![id(15)]),
            Op::Splice(id(0), 6, 0, vec![modal]),
        ],
    );
    frame(cx, handle).await;
    focused(cx, handle, 15);
    assert_eq!(
        offsets(cx, handle),
        before_modal,
        "floating focus cannot scroll anchor ancestors"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(0), 6, 1, vec![]),
            Op::Remove(id(15)),
            Op::Remove(modal),
        ],
    );
    frame(cx, handle).await;
    focused(cx, handle, 13);
    let mut remove = vec![Op::SetRoot(Some(id(0))), Op::Splice(id(0), 5, 1, vec![])];
    remove.extend([
        Op::Remove(id(13)),
        Op::Remove(id(12)),
        Op::Remove(id(11)),
        Op::Remove(outer()),
        Op::SetStyle(id(0), vec![]),
    ]);
    apply(cx, handle, remove);
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| assert!(view.scrolls.is_empty()))
        .unwrap();
    assert!(presses(transport).is_empty());
    eprintln!(
        "GPUIO_LINK_SCROLL_OK: nested XY offscreen Tab/Shift-Tab reveal, no unchanged-focus snapback, explicit AX reveal, mixed range-thumb order, modal restoration, fixed/cross-axis clip exclusion, oversized leading-edge reveal, idle, floating focus isolation and scroll-owner disposal"
    );
}
