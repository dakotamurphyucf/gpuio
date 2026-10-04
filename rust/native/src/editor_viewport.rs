use gpui::{Context, point, px};
use gpui_base::input::{InputBaseState, InputModeKind};
use gpuio_protocol::{
    editor_viewport::{Offset, Snapshot},
    v1::{EditorCommand, EditorError, EditorResult},
};

pub(super) fn command<M: InputModeKind>(
    state: &mut InputBaseState<M>,
    command: &EditorCommand,
    cx: &mut Context<InputBaseState<M>>,
) -> EditorResult {
    match command {
        EditorCommand::ReadRangeBounds(expected, selection) => {
            if *expected < 0 {
                return EditorResult::Failed(EditorError::NativeFailure);
            }
            if *expected != state.bridge_revision() {
                return EditorResult::Failed(EditorError::StaleRevision);
            }
            let (Ok(anchor), Ok(head)) = (
                usize::try_from(selection.anchor),
                usize::try_from(selection.head),
            ) else {
                return EditorResult::Failed(EditorError::InvalidSelection);
            };
            let text = state.text();
            if anchor > text.len()
                || head > text.len()
                || !text.is_char_boundary(anchor)
                || !text.is_char_boundary(head)
            {
                return EditorResult::Failed(EditorError::InvalidSelection);
            }
            let Some(bounds) = state.range_to_bounds(&(anchor.min(head)..anchor.max(head))) else {
                return EditorResult::RangeBounds(None);
            };
            let result = gpuio_protocol::editor_geometry::Snapshot {
                revision: *expected,
                x: f64::from(f32::from(bounds.origin.x)),
                y: f64::from(f32::from(bounds.origin.y)),
                width: f64::from(f32::from(bounds.size.width)),
                height: f64::from(f32::from(bounds.size.height)),
            };
            if result.is_valid() {
                EditorResult::RangeBounds(Some(result))
            } else {
                EditorResult::Failed(EditorError::NativeFailure)
            }
        }
        EditorCommand::ReadViewport => {
            let Some(line_height) = state.line_height() else {
                return EditorResult::Viewport(None);
            };
            let Some(lines) = state.visible_row_range() else {
                return EditorResult::Viewport(None);
            };
            let bounds = state.input_bounds();
            let Some(offset) = state.layout_scroll_offset() else {
                return EditorResult::Viewport(None);
            };
            let result = Snapshot {
                offset: Offset {
                    x: f64::from(f32::from(-offset.x)),
                    y: f64::from(f32::from(-offset.y)),
                },
                width: f64::from(f32::from(bounds.size.width)),
                height: f64::from(f32::from(bounds.size.height)),
                line_height: f64::from(f32::from(line_height)),
                first_buffer_line: lines.start as i64,
                buffer_line_limit: lines.end as i64,
            };
            if result.is_valid() {
                EditorResult::Viewport(Some(result))
            } else {
                EditorResult::Failed(EditorError::NativeFailure)
            }
        }
        EditorCommand::ScrollViewport(offset) => {
            if !offset.is_valid() {
                return EditorResult::Failed(EditorError::NativeFailure);
            }
            state.set_scroll_offset(point(px(-offset.x as f32), px(-offset.y as f32)), cx);
            EditorResult::ViewportScrollAccepted
        }
        _ => EditorResult::Failed(EditorError::NativeFailure),
    }
}
