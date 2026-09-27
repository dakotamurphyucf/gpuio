//! Keyed scroll preservation around atomic retained-source updates.
use super::*;
use crate::Size;
use gpui::{DeferredScrollToItem, point};

struct RowAnchor {
    key: RowKey,
    index: usize,
    within: Pixels,
}
struct ColumnAnchor {
    key: SharedString,
    index: usize,
    within: Pixels,
}
struct Anchors {
    row: Option<RowAnchor>,
    column: Option<ColumnAnchor>,
    pending_row: Option<(Option<RowKey>, DeferredScrollToItem)>,
}

impl<D: TableDelegate> TableState<D> {
    /// Explicitly reapply retained column descriptions, even when equal to the
    /// previous source schema. This lets application commands reject optimistic
    /// widths/sort indicators without fabricating a schema change. Retires active
    /// column gestures and preserves keyed selection and viewport anchors.
    pub fn reset_columns(&mut self, cx: &mut Context<Self>) {
        let anchors = self.capture_anchors(cx);
        self.invalidate_columns();
        self.prepare_col_groups(cx);
        self.refresh(cx);
        self.restore_anchors(anchors, cx);
    }

    /// Replace retained source/schema atomically while preserving the top visible
    /// row and first scrolling column by key and intra-item pixel offset.
    /// The closure must only update retained Rust data; no I/O or OCaml callback.
    ///
    /// An absent anchor falls back to the old numeric position, clamped by native
    /// layout. Empty sources reset scroll. A pending row command follows its key
    /// through reorder and overrides the painted anchor; removal cancels it.
    /// Issue new explicit scroll commands after this update so they take priority.
    ///
    /// Unchanged column descriptions preserve optimistic widths/sort state and
    /// active resize/reorder gestures. A schema replacement retires those gestures
    /// and uses supplied widths. Content-only delivery can notify directly without
    /// calling this operation. Stable keys must identify membership lifetimes.
    pub fn update_source<R>(
        &mut self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut D) -> R,
    ) -> R {
        self.update_source_with_size(self.options.size, cx, update)
    }

    /// Preserve row identity and intra-row offset when fixed row height changes.
    pub fn update_source_with_size<R>(
        &mut self,
        size: Size,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut D) -> R,
    ) -> R {
        let anchors = self.capture_anchors(cx);
        let result = update(&mut self.delegate);
        self.options.size = size;
        self.refresh(cx);
        self.restore_anchors(anchors, cx);
        result
    }

    fn capture_anchors(&self, cx: &App) -> Anchors {
        let height = self.options.size.table_row_height();
        let scroll = self.vertical_scroll_handle.0.borrow();
        let top = (-scroll.base_handle.offset().y).max(px(0.));
        let index = (top / height).floor() as usize;
        let row = self.delegate.row_key(index, cx).map(|key| RowAnchor {
            key,
            index,
            within: top - height * index as f32,
        });
        let pending_row = scroll
            .deferred_scroll_to_item
            .map(|request| (self.delegate.row_key(request.item_index, cx), request));
        let mut within = (-self.horizontal_scroll_handle.offset().x).max(px(0.));
        let column = self
            .col_groups
            .iter()
            .skip(self.fixed_left_cols_count())
            .enumerate()
            .find_map(|(index, column)| {
                if within < column.width {
                    Some(ColumnAnchor {
                        key: column.column.key.clone(),
                        index,
                        within,
                    })
                } else {
                    within -= column.width;
                    None
                }
            });
        Anchors {
            row,
            column,
            pending_row,
        }
    }

    fn restore_anchors(&mut self, anchors: Anchors, cx: &App) {
        let count = self.delegate.rows_count(cx);
        let height = self.options.size.table_row_height();
        let top = anchors
            .row
            .map(|anchor| {
                let index = self
                    .resolve_row(anchor.key, cx)
                    .unwrap_or_else(|| anchor.index.min(count.saturating_sub(1)));
                height * index as f32 + anchor.within.min(height)
            })
            .unwrap_or(px(0.));
        let mut scroll = self.vertical_scroll_handle.0.borrow_mut();
        let x = scroll.base_handle.offset().x;
        scroll
            .base_handle
            .set_offset(point(x, if count == 0 { px(0.) } else { -top }));
        if let Some((key, request)) = anchors.pending_row {
            // A removed target cancels the command; it must not target a neighbor.
            scroll.deferred_scroll_to_item =
                key.and_then(|key| self.resolve_row(key, cx))
                    .map(|index| DeferredScrollToItem {
                        item_index: index,
                        ..request
                    });
        }
        drop(scroll);

        let columns = &self.col_groups[self.fixed_left_cols_count()..];
        let left = anchors
            .column
            .and_then(|anchor| {
                if columns.is_empty() {
                    return None;
                }
                let index = columns
                    .iter()
                    .position(|column| column.column.key == anchor.key)
                    .unwrap_or_else(|| anchor.index.min(columns.len() - 1));
                let prefix: Pixels = columns.iter().take(index).map(|column| column.width).sum();
                Some(prefix + anchor.within.min(columns[index].width))
            })
            .unwrap_or(px(0.));
        self.horizontal_scroll_handle
            .set_offset(point(-left, px(0.)));
    }
}
