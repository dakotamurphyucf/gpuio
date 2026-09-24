//! GPUI owns measurement and scroll state. This adapter restores stable logical
//! identity across collection edits before the next native layout.
use crate::list_index::{Anchor, Index};
use gpui::{FollowMode, ListAlignment, ListOffset, ListState, px};
use gpuio_protocol::list::{Config, Order, ScrollPolicy, ScrollRequest, ScrollTarget};

pub struct State {
    index: Index,
    order: Order,
    config: Config,
    handle: ListState,
    last_scroll: i64,
}

impl State {
    pub fn new(config: Config, order: Order) -> Result<Self, &'static str> {
        if !config.is_valid() {
            return Err("invalid list configuration");
        }
        let index = Index::new(&order)?;
        let handle = ListState::new(index.len(), ListAlignment::Top, px(config.overscan as f32))
            .with_uniform_item_height(px(config.estimated_height as f32));
        if config.scroll_policy == ScrollPolicy::FollowTailWhenAtEnd {
            handle.set_follow_mode(FollowMode::Tail);
        }
        Ok(Self {
            index,
            order,
            config,
            handle,
            last_scroll: 0,
        })
    }

    pub fn handle(&self) -> &ListState {
        &self.handle
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    fn anchor(&self) -> Option<Anchor> {
        let offset = self.handle.logical_scroll_top();
        self.index.id(offset.item_ix).map(|row| Anchor {
            row,
            offset: f32::from(offset.offset_in_item),
        })
    }

    fn restore(&self, anchor: Option<Anchor>) {
        if !self.handle.is_following_tail()
            && let Some(anchor) = anchor
            && let Some(item_ix) = self.index.position(anchor.row)
        {
            self.handle.scroll_to(ListOffset {
                item_ix,
                offset_in_item: px(anchor.offset),
            });
        }
    }

    pub fn replace_order(&mut self, order: Order) -> Result<bool, &'static str> {
        if order == self.order {
            return Ok(false);
        }
        if order.revision <= self.order.revision {
            return Err("list order revision must advance");
        }
        let next = Index::new(&order)?;
        let anchor = self
            .anchor()
            .and_then(|anchor| self.index.remap_anchor(&next, anchor));
        let span = self.index.splice_to(&next);
        self.handle.splice(span.removed, span.inserted.len());
        // Structural changes are O(n); streaming row invalidation below never
        // reapplies hints to the entire list.
        self.handle
            .clone()
            .with_uniform_item_height(px(self.config.estimated_height as f32));
        self.index = next;
        self.order = order;
        self.restore(anchor);
        Ok(true)
    }

    pub fn invalidate_rows(&self, rows: &[i64]) {
        for row in rows {
            if let Some(index) = self.index.position(*row) {
                self.handle.remeasure_items(index..index + 1);
            }
        }
    }

    /// A command is executed at most once, after its transaction's data updates.
    /// Jump-to-end resumes tail following when configured; offsets/reveal use
    /// native scrolling behavior and can pause following.
    pub fn scroll(&mut self, request: ScrollRequest) -> Result<bool, &'static str> {
        if request.serial < 1 {
            return Err("invalid list scroll serial");
        }
        if request.serial <= self.last_scroll {
            return Ok(false);
        }
        match request.target {
            ScrollTarget::Offset(row, offset) => {
                if !offset.is_finite() || !(0.0..=1_000_000.0).contains(&offset) {
                    return Err("invalid list scroll offset");
                }
                let item_ix = self
                    .index
                    .position(row)
                    .ok_or("list scroll row is absent")?;
                self.handle.scroll_to(ListOffset {
                    item_ix,
                    offset_in_item: px(offset as f32),
                });
            }
            ScrollTarget::Reveal(row) => {
                let item_ix = self
                    .index
                    .position(row)
                    .ok_or("list reveal row is absent")?;
                self.handle.scroll_to_reveal_item(item_ix);
            }
            ScrollTarget::End => {
                if self.config.scroll_policy == ScrollPolicy::FollowTailWhenAtEnd {
                    self.handle.set_follow_mode(FollowMode::Tail);
                } else {
                    self.handle.scroll_to_end();
                }
            }
        }
        self.last_scroll = request.serial;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::list::IdRun;

    fn config(policy: ScrollPolicy) -> Config {
        Config {
            estimated_height: 100.,
            overscan: 200.,
            max_active: 128,
            scroll_policy: policy,
            scrollbar: true,
        }
    }
    fn order(revision: i64, ids: &[i64]) -> Order {
        Order {
            revision,
            runs: ids
                .iter()
                .map(|id| IdRun {
                    first: *id,
                    count: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn gpui_anchor_survives_prepend_reorder_and_height_invalidation() {
        let mut state =
            State::new(config(ScrollPolicy::KeepPosition), order(1, &[10, 11, 12])).unwrap();
        state
            .scroll(ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(11, 37.5),
            })
            .unwrap();
        state.replace_order(order(2, &[8, 9, 10, 11, 12])).unwrap();
        assert_eq!(state.handle.logical_scroll_top().item_ix, 3);
        assert_eq!(state.handle.logical_scroll_top().offset_in_item, px(37.5));
        state.replace_order(order(3, &[12, 11, 10, 9, 8])).unwrap();
        assert_eq!(state.handle.logical_scroll_top().item_ix, 1);
        assert_eq!(state.handle.logical_scroll_top().offset_in_item, px(37.5));
        state.invalidate_rows(&[10, 11]);
        assert_eq!(state.handle.logical_scroll_top().offset_in_item, px(37.5));
        state.replace_order(order(4, &[12, 10])).unwrap();
        assert_eq!(state.handle.logical_scroll_top().item_ix, 1);
        assert_eq!(state.handle.logical_scroll_top().offset_in_item, px(0.));
    }

    #[test]
    fn native_tail_pause_resume_and_duplicate_command_rejection() {
        let mut state = State::new(
            config(ScrollPolicy::FollowTailWhenAtEnd),
            order(1, &[1, 2, 3]),
        )
        .unwrap();
        assert!(state.handle.is_following_tail());
        state
            .scroll(ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(1, 0.),
            })
            .unwrap();
        assert!(!state.handle.is_following_tail());
        state.replace_order(order(2, &[1, 2, 3, 4])).unwrap();
        assert!(!state.handle.is_following_tail());
        let jump = ScrollRequest {
            serial: 2,
            target: ScrollTarget::End,
        };
        assert!(state.scroll(jump).unwrap());
        assert!(state.handle.is_following_tail());
        assert!(!state.scroll(jump).unwrap());
        assert!(state.replace_order(order(1, &[9])).is_err());
        assert_eq!(state.index.len(), 4);
    }
}
