//! Read-only display adapter: consume cached runs; never parse from GPUI layout.
use crate::document_highlight::Run;
use gpui::{Context, HighlightStyle, SharedString, Window};
use gpui_base::input::{
    EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, Rope,
};
use std::{ops::Range, sync::Arc};
pub struct Highlights(pub Arc<Vec<Run>>, pub Vec<FoldRange>);
pub fn style(run: &Run) -> HighlightStyle {
    let [r, g, b] = run.foreground;
    HighlightStyle {
        color: Some(gpui::rgb((r as u32) << 16 | (g as u32) << 8 | b as u32).into()),
        background_color: run
            .background
            .map(|[r, g, b]| gpui::rgb((r as u32) << 16 | (g as u32) << 8 | b as u32).into()),
        font_weight: run.bold.then_some(gpui::FontWeight::BOLD),
        font_style: run.italic.then_some(gpui::FontStyle::Italic),
        underline: run.underline.then_some(gpui::UnderlineStyle {
            thickness: gpui::px(1.),
            color: None,
            wavy: false,
        }),
        ..Default::default()
    }
}
impl InputHighlighter for Highlights {
    fn language(&self) -> SharedString {
        "gpuio-prepared".into()
    }
    fn update(
        &mut self,
        _: Option<InputEdit>,
        _: &Rope,
        _: bool,
        _: &mut Window,
        _: &mut Context<EditorState>,
    ) {
    }
    fn styles(
        &self,
        range: &Range<usize>,
        _: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut out = Vec::new();
        let mut cursor = range.start;
        let start = self.0.partition_point(|run| run.bytes.end <= range.start);
        for run in &self.0[start..] {
            if run.bytes.start >= range.end {
                break;
            }
            let bytes = run.bytes.start.max(range.start)..run.bytes.end.min(range.end);
            if bytes.start > cursor {
                out.push((cursor..bytes.start, HighlightStyle::default()));
            }
            cursor = bytes.end;
            out.push((bytes, style(run)));
        }
        if cursor < range.end {
            out.push((cursor..range.end, HighlightStyle::default()));
        }
        out
    }
    fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
        self.1.clone()
    }
}
