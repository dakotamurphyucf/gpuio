//! Bounded fallback text paint without a second accessibility child.
use gpui::{App, Bounds, Pixels, Window, canvas, point, prelude::*, px};
use std::sync::Arc;

pub(super) fn paint(text: &str, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
        return;
    }
    let style = window.text_style();
    let font_size = style
        .font_size
        .to_pixels(window.rem_size())
        .min(bounds.size.height * 0.5);
    if font_size <= px(0.) {
        return;
    }
    let run = style.to_run(text.len());
    let mut line = window.text_system().shape_line(
        text.to_owned().into(),
        font_size,
        std::slice::from_ref(&run),
        None,
    );
    let ratio = (bounds.size.width * 0.75 / line.width.max(px(0.001)))
        .min(bounds.size.height * 0.65 / (line.ascent + line.descent).max(px(0.001)))
        .min(1.);
    if ratio < 1. {
        line = window.text_system().shape_line(
            text.to_owned().into(),
            font_size * ratio,
            &[run],
            None,
        );
    }
    let height = line.ascent + line.descent;
    let origin = bounds.center() - point(line.width / 2., height / 2.);
    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        if let Err(error) = line.paint(origin, height, gpui::TextAlign::Left, None, window, cx) {
            eprintln!("GPUIO avatar fallback paint failed: {error}");
        }
    });
}
pub(super) fn fallback(text: Arc<str>) -> impl gpui::IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| paint(&text, bounds, window, cx),
    )
    .size_full()
}
