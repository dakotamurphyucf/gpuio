//! Shared mounted window budget: exact boundary, whole-text fallback, stable
//! phase admission and recovery. Runs after lifecycle's slots 4 through 24.
use super::*;
use crate::text_shimmer_budget::{MAX_CANDIDATES, MAX_FRAME_GLYPHS};

const ROOT: i64 = 25;
const FIRST: i64 = 26;
fn node(index: usize) -> NodeId {
    id(FIRST + index as i64)
}
fn cell(index: usize) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(20. + (index % 8) as f64 * 40.)),
        Field::Top(Length::Px(10. + (index / 8) as f64 * 26.)),
        Field::Width(Length::Px(38.)),
        Field::Height(Length::Px(24.)),
        Field::FontSize(12.),
        Field::LineHeight(Length::Px(20.)),
        Field::Foreground(Color::Rgba(0x000000ff)),
    ])]
}
fn usage(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> (usize, usize) {
    handle
        .update(cx, |v, _, _| v.text_shimmer_budget.borrow().usage())
        .unwrap()
}
fn advance(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    clock: &Clock,
    now: &mut u64,
    ms: u64,
) {
    *now += ms;
    clock.set_time(*now);
    draw(cx, handle);
}
fn last_cell(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> image::RgbaImage {
    let scale = handle.update(cx, |_, w, _| w.scale_factor()).unwrap();
    image::imageops::crop_imm(
        &image(cx, handle),
        (20. * scale) as u32,
        (218. * scale) as u32,
        (38. * scale) as u32,
        (24. * scale) as u32,
    )
    .to_image()
}

pub(super) fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    clock: &Clock,
    now: &mut u64,
) {
    assert_eq!(
        MAX_CANDIDATES, 64,
        "fixture geometry fits 65 visible labels"
    );
    let mut ops = vec![
        Op::SetStyle(id(3), vec![Style::Fields(vec![Field::Display(3)])]),
        Op::Create(id(ROOT), Kind::Container, String::new(), None),
        Op::SetStyle(id(ROOT), root(vec![])),
    ];
    for index in 0..=MAX_CANDIDATES {
        ops.extend([
            Op::Create(node(index), Kind::Text, "Busy".into(), None),
            Op::SetStyle(node(index), cell(index)),
            Op::SetTextShimmer(node(index), Some(config())),
        ]);
    }
    ops.extend([
        Op::Splice(id(ROOT), 0, 0, (0..=MAX_CANDIDATES).map(node).collect()),
        Op::Splice(id(0), 1, 0, vec![id(ROOT)]),
    ]);
    apply(cx, handle, ops);
    draw(cx, handle);
    assert_eq!(
        usage(cx, handle),
        (64, 256),
        "phase zero reserves whole texts too"
    );
    let first = probe(cx, handle, node(0));
    let last = probe(cx, handle, node(64));
    assert!(first.snapshot().unwrap().running);
    idle(cx, handle, &[&last]);
    let base = last_cell(cx, handle);
    assert!(
        base.pixels()
            .any(|p| p[0] < 100 && p[1] < 100 && p[2] < 100),
        "fallback text is actually visible"
    );
    advance(cx, handle, clock, now, 500);
    assert_eq!(usage(cx, handle), (64, 256));
    assert_eq!(first.snapshot().unwrap().phase, 0.5);
    assert_eq!(last.snapshot().unwrap().phase, 0.);
    assert_eq!(
        last_cell(cx, handle),
        base,
        "over-budget text remains completely ordinary"
    );
    assert!(
        colors(&image(cx, handle))[0] > 20,
        "admitted labels animate actual glyphs"
    );
    // Removing an early effect frees a candidate, without replacing any node.
    apply(cx, handle, vec![Op::SetTextShimmer(node(0), None)]);
    draw(cx, handle);
    assert!(first.snapshot().is_none());
    assert!(last.snapshot().unwrap().running);
    assert_eq!(last.snapshot().unwrap().phase, 0.);
    advance(cx, handle, clock, now, 500);
    assert_eq!(last.snapshot().unwrap().phase, 0.5);
    assert_ne!(last_cell(cx, handle), base);
    cx.update(|cx| cx.set_reduce_motion(true));
    draw(cx, handle);
    assert_eq!(
        usage(cx, handle),
        (0, 0),
        "reduced motion performs no overlay preflight"
    );
    idle(cx, handle, &[&last]);
    cx.update(|cx| cx.set_reduce_motion(false));
    draw(cx, handle);
    assert_eq!(
        usage(cx, handle),
        (64, 256),
        "next eligible frame has a fresh budget"
    );

    let once = Config {
        repeat: Repeat::Once,
        ..config()
    };
    apply(
        cx,
        handle,
        (1..=MAX_CANDIDATES)
            .map(|i| Op::SetTextShimmer(node(i), Some(once)))
            .collect(),
    );
    draw(cx, handle);
    advance(cx, handle, clock, now, 1000);
    assert_eq!(
        usage(cx, handle),
        (0, 0),
        "completed one-shot effects consume no overlay budget"
    );
    idle(cx, handle, &[&last]);

    // Five individually valid maximum-glyph strings exceed one frame's glyph
    // allowance. Overlay admission remains all-or-static, not a painted prefix.
    let mut ops = Vec::new();
    for index in 0..=MAX_CANDIDATES {
        ops.push(Op::SetTextShimmer(
            node(index),
            if index < 5 { Some(config()) } else { None },
        ));
        if index < 5 {
            let mut style = cell(index);
            style.push(Style::Fields(vec![
                Field::Left(Length::Px(20.)),
                Field::Top(Length::Px(10.)),
                Field::Width(Length::Px(300.)),
                Field::WhiteSpace(1),
            ]));
            ops.extend([
                Op::SetStyle(node(index), style),
                Op::SetText(node(index), "W".repeat(4096)),
            ]);
        }
    }
    apply(cx, handle, ops);
    draw(cx, handle);
    assert_eq!(usage(cx, handle), (5, MAX_FRAME_GLYPHS));
    let rejected = probe(cx, handle, node(4));
    let admitted = probe(cx, handle, node(1));
    idle(cx, handle, &[&rejected]);
    advance(cx, handle, clock, now, 500);
    assert_eq!(usage(cx, handle), (5, MAX_FRAME_GLYPHS));
    assert_eq!(admitted.snapshot().unwrap().phase, 0.5);
    assert_eq!(rejected.snapshot().unwrap().phase, 0.);
    apply(cx, handle, vec![Op::SetTextShimmer(node(0), None)]);
    draw(cx, handle);
    assert_eq!(usage(cx, handle), (4, MAX_FRAME_GLYPHS));
    assert!(rejected.snapshot().unwrap().running);
    advance(cx, handle, clock, now, 250);
    assert_eq!(rejected.snapshot().unwrap().phase, 0.25);

    let mut ops = vec![Op::Splice(id(0), 1, 1, vec![])];
    ops.extend((0..=MAX_CANDIDATES).rev().map(|i| Op::Remove(node(i))));
    ops.push(Op::Remove(id(ROOT)));
    ops.push(Op::SetStyle(id(3), root(vec![])));
    let first_node = NodeId::from_parts(1, 2).unwrap();
    ops.push(Op::SetText(first_node, "Fresh after frame budget".into()));
    apply(cx, handle, ops);
    draw(cx, handle);
    assert!(admitted.snapshot().is_none() && rejected.snapshot().is_none());
    assert_eq!(probe(cx, handle, first_node).snapshot().unwrap().phase, 0.);
    eprintln!(
        "GPUIO_NATIVE_TEXT_SHIMMER_BUDGET_OK: 64 candidate boundary, 16384 glyph boundary, phase-independent whole-text fallback, native pause/recovery, reduced motion, frame reset and cleanup"
    );
}
