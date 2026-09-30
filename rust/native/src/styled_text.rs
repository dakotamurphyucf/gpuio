//! A single layout shared by ordinary and selectable foreground-styled text.
use gpui::{HighlightStyle, SharedString, StyledText, rgba};
use gpuio_protocol::text_content::Span;

pub(crate) fn element(text: SharedString, spans: &[Span]) -> StyledText {
    // The tree admits only sorted, nonoverlapping scalar-boundary ranges. Keep
    // gaps inherited from the element's current text style, including its state.
    StyledText::new(text).with_highlights(spans.iter().map(|span| {
        (
            span.start_byte as usize..span.end_byte as usize,
            HighlightStyle {
                color: Some(rgba(span.foreground as u32).into()),
                ..Default::default()
            },
        )
    }))
}
