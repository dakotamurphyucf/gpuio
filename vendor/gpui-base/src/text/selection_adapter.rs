use std::{cell::RefCell, ops::RangeInclusive, rc::Rc};

use crate::{
    TextSelectionContentKey, TextSelectionCoverage, TextSelectionEndpoint, TextSelectionEvent,
    TextSelectionHandle, TextSelectionRegistration, TextSelectionSnapshot,
};
use gpui::{App, Bounds, EntityId, Hitbox, Pixels, Point, TextLayout, WeakEntity, Window};

use super::TextViewState;
use super::rendered_text::RenderedFragment;

// Nonvirtual participants have no block restriction, even if they have a
// logical text endpoint. In particular this must not mean "only block zero".
const NO_BLOCK: u64 = u64::MAX;

#[derive(Clone)]
struct TextEndpointRun {
    layout: TextLayout,
    bounds: Bounds<Pixels>,
    fragment: Option<RenderedFragment>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CachedBlockEndpoint {
    endpoint: TextSelectionEndpoint,
    block_ix: Option<usize>,
}

#[derive(Default)]
struct VirtualBlockSelection {
    anchor: Option<CachedBlockEndpoint>,
    cursor: Option<CachedBlockEndpoint>,
    coverage: TextSelectionCoverage,
}

impl VirtualBlockSelection {
    fn update(&mut self, snapshot: Option<TextSelectionSnapshot>, entity_id: EntityId) {
        let Some(snapshot) = snapshot else {
            *self = Self::default();
            return;
        };

        self.coverage = snapshot.coverage();

        Self::update_endpoint(&mut self.anchor, snapshot.anchor(), entity_id);
        Self::update_endpoint(&mut self.cursor, snapshot.cursor(), entity_id);
    }

    fn update_endpoint(
        cached: &mut Option<CachedBlockEndpoint>,
        endpoint: TextSelectionEndpoint,
        entity_id: EntityId,
    ) {
        if cached.is_some_and(|cached| cached.endpoint == endpoint) {
            return;
        }
        let block_ix = (endpoint.entity_id() == Some(entity_id))
            .then(|| {
                endpoint
                    .content_key()
                    .filter(|key| key.value() != NO_BLOCK)
                    .map(|key| key.value() as usize)
            })
            .flatten();
        *cached = Some(CachedBlockEndpoint { endpoint, block_ix });
    }

    fn block_range(&self, entity_id: EntityId, last: usize) -> Option<RangeInclusive<usize>> {
        let anchor = self.anchor?;
        let cursor = self.cursor?;
        match self.coverage {
            TextSelectionCoverage::Full => Some(0..=last),
            TextSelectionCoverage::FromStart => Some(0..=anchor.block_ix.or(cursor.block_ix)?),
            TextSelectionCoverage::ToEnd => Some(anchor.block_ix.or(cursor.block_ix)?..=last),
            TextSelectionCoverage::Bounded => {
                if anchor.endpoint.entity_id() != Some(entity_id)
                    || cursor.endpoint.entity_id() != Some(entity_id)
                {
                    return None;
                }
                let (anchor, cursor) = (anchor.block_ix?, cursor.block_ix?);
                Some(anchor.min(cursor)..=anchor.max(cursor))
            }
        }
    }
}

/// TextView's renderer-specific bridge to base-owned window selection.
#[derive(Clone)]
pub(super) struct TextViewSelectionAdapter {
    selection: TextSelectionHandle,
    text_bounds: Vec<Bounds<Pixels>>,
    endpoint_runs: Vec<TextEndpointRun>,
    layout_revision: Option<usize>,
}

impl TextViewSelectionAdapter {
    pub(super) fn new(view: WeakEntity<TextViewState>, cx: &mut App) -> Self {
        let selection = TextSelectionHandle::new("", cx);
        let selection_id = selection.entity_id();
        let virtual_blocks = Rc::new(RefCell::new(VirtualBlockSelection::default()));

        let view_for_events = view.clone();
        let blocks_for_events = virtual_blocks.clone();
        selection
            .subscribe(
                move |event, cx| match event {
                    TextSelectionEvent::SelectionChanged(snapshot) => {
                        let Some(view) = view_for_events.upgrade() else {
                            return;
                        };
                        let snapshot = *snapshot;
                        view.update(cx, |state, cx| {
                            state.retire_rendered_selection();
                            state.preserve_inline_selection = false;
                            blocks_for_events
                                .borrow_mut()
                                .update(snapshot, selection_id);
                            state.is_selecting =
                                snapshot.is_some_and(|snapshot| snapshot.is_selecting());
                            state.adopt_rendered_pointer_selection(snapshot);
                            cx.notify();
                        });
                    }
                    TextSelectionEvent::AutoScroll(delta) => {
                        let Some(view) = view_for_events.upgrade() else {
                            return;
                        };
                        let delta = *delta;
                        view.update(cx, |state, cx| {
                            if state.scrollable {
                                state.set_auto_scroll(delta, cx);
                            } else if delta.is_none() {
                                state.stop_auto_scroll();
                            }
                        });
                    }
                    TextSelectionEvent::Cleared => {}
                },
                cx,
            )
            .detach();

        let view_for_clear = view.clone();
        let blocks_for_clear = virtual_blocks.clone();
        selection.clear_with(
            move |cx| {
                blocks_for_clear.replace(VirtualBlockSelection::default());
                // Window teardown can deliver this after the document retires.
                // That is an ordinary no-op, not an error requiring a backtrace
                // through the mixed OCaml/Rust application stack.
                let Some(view) = view_for_clear.upgrade() else {
                    return;
                };
                view.update(cx, |state, cx| {
                    state.reset_selection();
                    cx.notify();
                });
            },
            cx,
        );

        let view_for_copy = view.clone();
        let blocks_for_copy = virtual_blocks.clone();
        selection.copy_with(
            move |cx| {
                let Some(view) = view_for_copy.upgrade() else {
                    return String::new();
                };
                let state = view.read(cx);
                // A restyle can disable selection before the frame-end sweep
                // removes this participant's previous registration.
                if !state.is_selectable() {
                    return String::new();
                }
                let last = state.parsed_content.document.blocks.len().saturating_sub(1);
                let blocks = blocks_for_copy.borrow().block_range(selection_id, last);
                let text = state.selected_text_in(blocks);
                if state.effective_format() == super::SelectionFormat::Source {
                    // Source selection owns its whitespace, including the exact
                    // source returned by select-all. Never normalize those bytes.
                    text
                } else {
                    // Normalize rendered paragraph separators locally, without
                    // trimming neighboring participants in the window result.
                    text.trim().to_string()
                }
            },
            cx,
        );

        let view_for_content_key = view.clone();
        selection.resolve_content_key_with(
            move |point, cx| {
                let view = view_for_content_key.upgrade()?;
                let view = view.read(cx);
                let block = view.block_ix_at(point.y);
                let window_point = point + view.bounds().origin + view.scroll_offset();
                let position =
                    view.selection_adapter
                        .endpoint_at(window_point)
                        .filter(|position| {
                            view.rendered_text()
                                .is_some_and(|text| text.captured_position(*position).is_some())
                        });
                if block.is_none() && position.is_none() {
                    return None;
                }
                let key =
                    TextSelectionContentKey::new(block.map_or(NO_BLOCK, |block| block as u64));
                Some(position.map_or(key, |position| key.with_position(position)))
            },
            cx,
        );

        selection.focus_with(
            move |window, cx| {
                let Some(view) = view.upgrade() else {
                    return;
                };
                let focus_handle = view.read(cx).focus_handle.clone();
                view.update(cx, |state, _| state.is_selecting = true);
                focus_handle.focus(window, cx);
            },
            cx,
        );

        Self {
            selection,
            text_bounds: Vec::new(),
            endpoint_runs: Vec::new(),
            layout_revision: None,
        }
    }

    pub(super) fn update_layout_revision(&mut self, revision: usize, is_selecting: bool) -> bool {
        let changed = self
            .layout_revision
            .is_some_and(|previous| previous != revision);
        if !changed || !is_selecting {
            self.layout_revision = Some(revision);
        }
        changed && !is_selecting
    }

    pub(super) fn begin_frame(&mut self) {
        self.text_bounds.clear();
        self.endpoint_runs.clear();
    }

    pub(super) fn register_inline(&mut self, bounds: Vec<Bounds<Pixels>>) {
        self.text_bounds.extend(bounds);
    }

    pub(super) fn register_text_endpoint(
        &mut self,
        layout: TextLayout,
        bounds: Bounds<Pixels>,
        fragment: Option<RenderedFragment>,
    ) {
        self.endpoint_runs.push(TextEndpointRun {
            layout,
            bounds,
            fragment,
        });
    }

    fn endpoint_at(&self, point: Point<Pixels>) -> Option<crate::TextSelectionContentPosition> {
        // No extrapolation across a custom object or paragraph gap. Unmapped
        // painted runs are barriers, not permission to borrow adjacent text.
        let run = self
            .endpoint_runs
            .iter()
            .find(|run| run.bounds.contains(&point))?;
        let fragment = run.fragment.as_ref()?;
        let byte = run
            .layout
            .index_for_position(point)
            .unwrap_or_else(|byte| byte);
        fragment.position(byte)
    }

    /// Only actual captured endpoints, never inferred from selected strings or
    /// the minimum/maximum currently painted fragment ranges.
    pub(super) fn captured_rendered_selection(
        &self,
        text: &super::RenderedText,
        cx: &App,
    ) -> Option<super::RenderedSelection> {
        self.rendered_selection_from_snapshot(text, self.selection.snapshot(cx))
    }

    pub(super) fn rendered_selection_from_snapshot(
        &self,
        text: &super::RenderedText,
        snapshot: Option<TextSelectionSnapshot>,
    ) -> Option<super::RenderedSelection> {
        let snapshot = snapshot?;
        if snapshot.coverage() != TextSelectionCoverage::Bounded
            || snapshot.anchor().entity_id() != Some(self.selection.entity_id())
            || snapshot.cursor().entity_id() != Some(self.selection.entity_id())
        {
            return None;
        }
        let anchor = text.captured_position(snapshot.anchor().content_key()?.position()?)?;
        let head = text.captured_position(snapshot.cursor().content_key()?.position()?)?;
        text.selection(&anchor, &head).ok()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn register(
        &self,
        hitbox: Hitbox,
        bounds: Bounds<Pixels>,
        scroll_offset: Point<Pixels>,
        document_order: u64,
        self_scroll: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.selection.register(
            TextSelectionRegistration::new(hitbox, bounds)
                .with_scroll_offset(scroll_offset)
                .with_document_order(document_order)
                .with_text_bounds(self.text_bounds.clone())
                .with_self_scroll(self_scroll),
            window,
            cx,
        );
    }

    pub(super) fn selection_points(&self, cx: &App) -> Option<(Point<Pixels>, Point<Pixels>)> {
        let points = self.selection.snapshot(cx)?.window_points()?;
        Some((points.anchor(), points.cursor()))
    }

    pub(super) fn set_local_selection(&self, active: bool, cx: &mut App) {
        self.selection.set_local_selection(active, cx);
    }

    pub(super) fn is_part_of_window_selection(&self, cx: &App) -> bool {
        let id = self.selection.entity_id();
        self.selection.snapshot(cx).is_some_and(|snapshot| {
            snapshot.anchor().entity_id() == Some(id) || snapshot.cursor().entity_id() == Some(id)
        })
    }

    pub(super) fn has_selection_snapshot(&self, cx: &App) -> bool {
        self.selection.snapshot(cx).is_some()
    }

    #[cfg(test)]
    pub(super) fn text_bounds(&self) -> Vec<Bounds<Pixels>> {
        self.text_bounds.clone()
    }
}
