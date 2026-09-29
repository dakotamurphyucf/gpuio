//! Pointer typography measured from shaped glyph cells rather than caret indices.
use super::*;
use std::ops::Range;

async fn gesture(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    start: gpui::Point<gpui::Pixels>,
    end: gpui::Point<gpui::Pixels>,
) {
    move_mouse(cx, window, start, false);
    mouse(cx, window, start, true);
    move_mouse(cx, window, end, true);
    mouse(cx, window, end, false);
    frame(cx, window).await;
}
fn band(
    layout: &gpui::TextLayout,
    bytes: Range<usize>,
    align: i64,
) -> (gpui::Point<gpui::Pixels>, gpui::Point<gpui::Pixels>) {
    let lines = layout.line_layouts();
    assert_eq!(lines.len(), 1);
    let wrapped = &lines[0];
    assert!(
        wrapped.wrap_boundaries.is_empty(),
        "single-row glyph fixture"
    );
    let line = &wrapped.unwrapped_layout;
    let mut glyphs = line
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .collect::<Vec<_>>();
    glyphs.sort_by(|a, b| a.position.x.partial_cmp(&b.position.x).unwrap());
    let mut left = line.width;
    let mut right = px(0.);
    for (i, glyph) in glyphs.iter().enumerate() {
        if bytes.contains(&glyph.index) {
            left = left.min(glyph.position.x);
            let end = glyphs[i + 1..]
                .iter()
                .find(|next| next.position.x > glyph.position.x)
                .map_or(line.width, |next| next.position.x);
            right = right.max(end);
        }
    }
    assert!(
        right - left > px(2.),
        "fixture must have visible selected glyphs"
    );
    let bounds = layout.bounds();
    let inset = match align {
        1 => (bounds.size.width - line.width) / 2.,
        2 => bounds.size.width - line.width,
        _ => px(0.),
    };
    if align != 0 {
        assert!(
            inset > px(1.),
            "alignment fixture must have a real inset: bounds={bounds:?}, width={:?}",
            line.width
        );
    }
    let y = bounds.top() + layout.line_height() / 2.;
    (
        gpui::point(bounds.left() + inset + left + px(0.5), y),
        gpui::point(bounds.left() + inset + right - px(0.5), y),
    )
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    for (source, expected) in [
        ("A 👨‍👩‍👧‍👦 Z", "👨‍👩‍👧‍👦"),
        ("A e\u{301} Z", "e\u{301}"),
        ("A 🇺🇸 Z", "🇺🇸"),
        ("A אבג Z", "אבג"),
        ("A אבג Z", "ב"),
        ("A مرحبا Z", "مرحبا"),
        ("A مرحبا Z", "ح"),
    ] {
        for align in [0, 1, 2] {
            apply(
                cx,
                window,
                vec![
                    Op::SetText(id(2), source.into()),
                    Op::SetStyle(
                        id(2),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(320.)),
                            Field::TextAlign(align),
                        ])],
                    ),
                ],
            );
            frame(cx, window).await;
            let layout = window
                .update(cx, |v, _, _| v.selections[&id(2)].borrow().layout())
                .unwrap();
            let start = source.find(expected).unwrap();
            let (left, right) = band(&layout, start..start + expected.len(), align);
            for (a, b) in [(left, right), (right, left)] {
                gesture(cx, window, a, b).await;
                assert_eq!(
                    copy(cx, window),
                    expected,
                    "glyph-cell drag source={source:?}, align={align}"
                );
            }
        }
    }
    for source in [
        "alpha β gamma delta epsilon ζ",
        "alpha β\nmiddle 😀\nomega ζ",
        "alpha β\r\nmiddle 😀\r\nomega ζ",
    ] {
        apply(
            cx,
            window,
            vec![
                Op::SetText(id(2), source.into()),
                Op::SetStyle(
                    id(2),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(100.)),
                        Field::TextAlign(0),
                    ])],
                ),
            ],
        );
        frame(cx, window).await;
        let a = point_at(cx, window, id(2), 0);
        let b = point_at(cx, window, id(2), source.len());
        assert!(a.y < b.y, "fixture actually wraps or has hard newlines");
        gesture(cx, window, a, b).await;
        assert_eq!(copy(cx, window), source, "complete multi-row selection");
        gesture(cx, window, b, a).await;
        assert_eq!(
            copy(cx, window),
            source,
            "reverse complete multi-row selection"
        );
    }
    let source = "alpha β\r\nmiddle 😀\r\nomega ζ";
    let newline = source.find("\r\n").unwrap();
    let a = point_at(cx, window, id(2), newline);
    let b = point_at(cx, window, id(2), newline + 2);
    gesture(cx, window, a, b).await;
    assert_eq!(copy(cx, window), "\r\n", "CRLF is one selected grapheme");
    eprintln!(
        "GPUIO_SELECTION_TYPOGRAPHY_OK: joined emoji, combining mark, flag, Hebrew/Arabic visual cells, three alignments, both directions and soft/hard wraps"
    );
}
