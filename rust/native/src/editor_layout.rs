use gpui::{Context, Window};
use gpui_base::input::{TextareaState, WrappingIndent};
use gpuio_protocol::text_area_layout::{Config, WrappingIndent as WireIndent};

/// Apply only changed layout properties. Reapplying soft-wrap during a label or
/// read-only update would reset horizontal scroll despite no layout-mode change.
pub(super) fn configure(
    state: &mut TextareaState,
    previous: Option<Config>,
    next: Option<Config>,
    window: &mut Window,
    cx: &mut Context<TextareaState>,
) {
    let previous = previous.unwrap_or_default();
    let next = next.unwrap_or_default();
    if previous.wrapping_indent != next.wrapping_indent {
        state.set_wrapping_indent(
            match next.wrapping_indent {
                WireIndent::FlushLeft => WrappingIndent::None,
                WireIndent::MatchFirstLine => WrappingIndent::Same,
            },
            window,
            cx,
        );
    }
    if previous.soft_wrap != next.soft_wrap {
        state.set_soft_wrap(next.soft_wrap, window, cx);
    }
    if previous.show_whitespace != next.show_whitespace {
        state.set_show_whitespaces(next.show_whitespace, window, cx);
    }
    if previous.cursor_margin_lines != next.cursor_margin_lines {
        state.set_cursor_surrounding_lines(
            next.cursor_margin_lines.map(|n| n as usize),
            window,
            cx,
        );
    }
}
