//! Atomic block ownership around arbitrary native layout and controls.
use super::{TextViewMultiClickKind, inline::point_in_text_selection, state::LineSpan};
use crate::GlobalState;
use gpui::{
    AnyElement, App, Bounds, CursorStyle, Element, ElementId, GlobalElementId, Hitbox,
    HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent, Pixels,
    Role, SharedString, Window,
};
use std::sync::{Arc, Mutex};

/// Selection ownership follows the declared presentation; glyph-owned Text
/// must never accidentally acquire a whole-object Copy override.
#[derive(Default)]
pub(super) enum BlockSelection {
    #[default]
    Text,
    Object(bool),
}
impl BlockSelection {
    pub(super) fn is_selected(&self) -> bool {
        matches!(self, Self::Object(true))
    }
    pub(super) fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub(super) fn clear(&mut self) {
        if let Self::Object(selected) = self {
            *selected = false;
        }
    }
}

pub(super) struct BlockObject {
    id: usize,
    copy_text: SharedString,
    label: SharedString,
    selected: Arc<Mutex<BlockSelection>>,
    content: AnyElement,
}
impl BlockObject {
    pub(super) fn new(
        id: usize,
        copy_text: SharedString,
        label: SharedString,
        selected: Arc<Mutex<BlockSelection>>,
        content: AnyElement,
    ) -> Self {
        Self {
            id,
            copy_text,
            label,
            selected,
            content,
        }
    }
}
impl IntoElement for BlockObject {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for BlockObject {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;
    fn id(&self) -> Option<ElementId> {
        Some(("atomic-block", self.id).into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn a11y_role(&self) -> Option<Role> {
        Some(Role::GenericContainer)
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        node.set_role(Role::GenericContainer);
        if !self.label.is_empty() {
            node.set_label(self.label.as_ref());
        }
        // Keep native child controls and their semantics; no hidden source tree.
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.content.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.content.prepaint(window, cx);
        if let Some(view) = GlobalState::global(cx).text_view_state() {
            let state = view.read(cx);
            if state.max_lines.is_some()
                && let Ok(mut spans) = state.line_spans.lock()
            {
                spans.push(LineSpan {
                    top: bounds.top(),
                    bottom: bounds.bottom(),
                    line_height: bounds.size.height,
                });
            }
        }
        hitbox
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        let view = GlobalState::global(cx).text_view_state().cloned();
        let selectable = view.as_ref().is_some_and(|v| v.read(cx).is_selectable());
        let selected = view.as_ref().is_some_and(|view| {
            let state = view.read(cx);
            if !selectable {
                return false;
            }
            if state.preserve_inline_selection && !state.is_all_selected() {
                return self.selected.lock().is_ok_and(|s| s.is_selected());
            }
            state.is_all_selected()
                || state
                    .multi_click_selection()
                    .is_some_and(|s| bounds.contains(&s.pos))
                || state.selection_points(cx).is_some_and(|(start, end)| {
                    point_in_text_selection(
                        bounds.origin,
                        bounds.size.width,
                        start,
                        end,
                        bounds.size.height,
                    )
                })
        });
        if let Ok(mut state) = self.selected.lock() {
            *state = BlockSelection::Object(selected);
        }
        let selection_color =
            selected.then(|| view.as_ref().unwrap().read(cx).text_view_style.selection());
        if selectable {
            let view = view.unwrap();
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
            let visible = bounds.intersect(&window.content_mask().bounds);
            if visible.size.width > Pixels::ZERO && visible.size.height > Pixels::ZERO {
                view.update(cx, |state, _| {
                    state.selection_adapter.register_inline(vec![visible]);
                    let fragment = state
                        .rendered_text()
                        .and_then(|t| t.object_fragment(&self.selected));
                    state
                        .selection_adapter
                        .register_object_endpoint(bounds, fragment);
                });
            }
            let revision = view.read(cx).rendered_text_revision();
            let hitbox = hitbox.clone();
            let selected_state = self.selected.clone();
            let copy_text = self.copy_text.clone();
            let current_view = window.current_view();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if !phase.bubble()
                    || !hitbox.is_hovered(window)
                    || event.button != MouseButton::Left
                    || GlobalState::is_text_selection_suppressed(cx)
                {
                    return;
                }
                if !view.read(cx).accepts_selection_frame(revision) {
                    GlobalState::suppress_text_selection(cx);
                    return;
                }
                if !(2..=3).contains(&event.click_count) {
                    return;
                }
                GlobalState::suppress_text_selection(cx);
                if let Ok(mut selected) = selected_state.lock() {
                    *selected = BlockSelection::Object(true);
                }
                view.update(cx, |s, cx| {
                    s.set_multi_click_selection(
                        event.position,
                        TextViewMultiClickKind::Word,
                        copy_text.to_string(),
                        cx,
                    )
                });
                cx.notify(current_view);
            });
        }
        self.content.paint(window, cx);
        if let Some(color) = selection_color {
            window.paint_quad(gpui::fill(bounds, color));
        }
    }
}
