//! Retained GPUI measurement/scroll state for the picker's mixed-height rows.
//!
//! Update before layout. Pass a cloned handle and immutable projection into the
//! native row renderer; never borrow this state from GPUI's list layout callback.
use crate::choice_picker_rows::{Cursor, Key, Projection};
use gpui::{ListAlignment, ListOffset, ListState, px};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidGeometry;

pub struct State {
    projection: Arc<Projection>,
    handle: ListState,
    estimated_height: f32,
}

impl State {
    /// Positive finite row estimate <=1,000,000 logical pixels; finite overscan
    /// in 0..4096. Actual row heights remain GPUI measurements, not fixed geometry.
    pub fn new(
        projection: Arc<Projection>,
        estimated_height: f32,
        overscan: f32,
    ) -> Result<Self, InvalidGeometry> {
        if !estimated_height.is_finite()
            || !(1.0..=1_000_000.0).contains(&estimated_height)
            || !overscan.is_finite()
            || !(0.0..=4096.0).contains(&overscan)
        {
            return Err(InvalidGeometry);
        }
        let handle = ListState::new(projection.len(), ListAlignment::Top, px(overscan))
            .with_uniform_item_height(px(estimated_height));
        Ok(Self {
            projection,
            handle,
            estimated_height,
        })
    }
    pub fn projection(&self) -> &Arc<Projection> {
        &self.projection
    }
    pub fn handle(&self) -> &ListState {
        &self.handle
    }

    fn remap_anchor(&self, next: &Projection) -> ListOffset {
        let offset = self.handle.logical_scroll_top();
        let position = |old| {
            self.projection
                .row(old)
                .and_then(|row| next.position(row.key()))
        };
        if let Some(item_ix) = position(offset.item_ix) {
            return ListOffset {
                item_ix,
                offset_in_item: offset.offset_in_item,
            };
        }
        let successor =
            (offset.item_ix.saturating_add(1)..self.projection.len()).find_map(position);
        let predecessor = || {
            (0..offset.item_ix.min(self.projection.len()))
                .rev()
                .find_map(position)
        };
        ListOffset {
            item_ix: successor.or_else(predecessor).unwrap_or(0),
            offset_in_item: px(0.),
        }
    }

    /// Preserve the visible logical key and its intra-row offset. If it was
    /// removed, prefer its old next surviving row, then its previous survivor.
    /// Content-only updates remeasure rows rather than resetting scroll input.
    pub fn replace(&mut self, next: Arc<Projection>) {
        if Arc::ptr_eq(&next, &self.projection) {
            return;
        }
        let anchor = self.remap_anchor(&next);
        let old_len = self.projection.len();
        let new_len = next.len();
        let same_key =
            |a, b| self.projection.row(a).map(|row| row.key()) == next.row(b).map(|row| row.key());
        let mut prefix = 0;
        while prefix < old_len.min(new_len) && same_key(prefix, prefix) {
            prefix += 1;
        }
        let mut suffix = 0;
        while suffix < old_len.min(new_len) - prefix
            && same_key(old_len - suffix - 1, new_len - suffix - 1)
        {
            suffix += 1;
        }
        let structural = prefix != old_len || prefix != new_len;
        if structural {
            self.handle
                .splice(prefix..old_len - suffix, new_len - prefix - suffix);
            // Existing measurements become size hints; newly inserted rows gain
            // an estimate. Do this only for structure changes, never each frame.
            self.handle
                .clone()
                .with_uniform_item_height(px(self.estimated_height));
        } else {
            for row in 0..new_len {
                if self.projection.row(row) != next.row(row)
                    || self.projection.is_selected(row) != next.is_selected(row)
                    || self.projection.is_enabled(row) != next.is_enabled(row)
                {
                    self.handle.remeasure_items(row..row + 1);
                }
            }
        }
        self.projection = next;
        if structural {
            self.handle.scroll_to(anchor);
        }
    }

    /// Rich subtree/style updates may change size without changing catalog data.
    /// Missing keys are harmless; run this before list layout, not within it.
    pub fn invalidate(&self, key: Key<'_>) {
        if let Some(row) = self.projection.position(key) {
            self.handle.remeasure_items(row..row + 1);
        }
    }

    /// Call on an explicit keyboard/highlight change, not every render: revealing
    /// an unchanged active option during wheel scrolling would fight the user.
    pub fn reveal(&self, cursor: &Cursor) {
        if let Some(row) = cursor.active_row(&self.projection) {
            self.handle.scroll_to_reveal_item(row);
        }
    }
}

#[cfg(test)]
#[path = "choice_picker_list_test.rs"]
mod tests;
