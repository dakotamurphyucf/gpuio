//! Logical list metadata is separate from native node ownership. Consecutive
//! IDs encode a large initial history without a widget or key string per row.
use crate::NodeId;
use binprot::macros::BinProtWrite;

pub const MAX_LOGICAL_ROWS: usize = 1_000_000;
pub const MAX_ID_RUNS: usize = 100_000;
pub const MAX_ACTIVE_ROWS: usize = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct IdRun {
    pub first: i64,
    pub count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Order {
    pub revision: i64,
    pub runs: Vec<IdRun>,
}
impl Order {
    /// Validate before expansion; neither huge counts nor overlapping IDs can
    /// make the decoder allocate an unbounded per-row index.
    pub fn is_valid(&self) -> bool {
        if self.revision < 1 || self.runs.len() > MAX_ID_RUNS {
            return false;
        }
        let mut count = 0_i64;
        let mut intervals = Vec::with_capacity(self.runs.len());
        for run in &self.runs {
            if run.first < 1 || run.count < 1 || run.count > MAX_LOGICAL_ROWS as i64 - count {
                return false;
            }
            let Some(last) = run.first.checked_add(run.count - 1) else {
                return false;
            };
            count += run.count;
            intervals.push((run.first, last));
        }
        intervals.sort_unstable();
        intervals.windows(2).all(|pair| pair[0].1 < pair[1].0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ScrollPolicy {
    KeepPosition,
    FollowTailWhenAtEnd,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub estimated_height: f64,
    pub overscan: f64,
    pub max_active: i64,
    pub scroll_policy: ScrollPolicy,
    pub scrollbar: bool,
    pub managed: bool,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.estimated_height.is_finite()
            && (1.0..=1_000_000.0).contains(&self.estimated_height)
            && self.overscan.is_finite()
            && (0.0..=1_000_000.0).contains(&self.overscan)
            && (1..=MAX_ACTIVE_ROWS as i64).contains(&self.max_active)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Row {
    pub id: i64,
    pub node: NodeId,
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum ScrollTarget {
    Offset(i64, f64),
    Reveal(i64),
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct ScrollRequest {
    pub serial: i64,
    pub target: ScrollTarget,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Viewport {
    pub order_revision: i64,
    pub visible_first: i64,
    pub visible_last: i64,
    pub requested: Vec<i64>,
    pub pinned: Vec<i64>,
    pub anchor: Option<(i64, f64)>,
    pub following_tail: bool,
    pub at_start: bool,
    pub at_end: bool,
    pub budget_exhausted: bool,
}
impl Viewport {
    pub fn is_valid(&self) -> bool {
        use std::collections::HashSet;
        let valid_ids = |ids: &[i64]| {
            ids.len() <= MAX_ACTIVE_ROWS
                && ids.iter().all(|id| *id > 0)
                && ids.iter().collect::<HashSet<_>>().len() == ids.len()
        };
        self.order_revision > 0
            && self.visible_first >= 0
            && self.visible_last >= self.visible_first
            && self.visible_last <= MAX_LOGICAL_ROWS as i64
            && valid_ids(&self.requested)
            && valid_ids(&self.pinned)
            && self
                .requested
                .iter()
                .chain(&self.pinned)
                .collect::<HashSet<_>>()
                .len()
                <= MAX_ACTIVE_ROWS
            && self.anchor.is_none_or(|(id, offset)| {
                id > 0 && offset.is_finite() && (0.0..=1_000_000.0).contains(&offset)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binprot::BinProtWrite;

    #[test]
    fn large_history_is_compact_and_invalid_runs_do_not_expand() {
        let order = Order {
            revision: 1,
            runs: vec![IdRun {
                first: 1,
                count: 100_000,
            }],
        };
        assert!(order.is_valid());
        let mut bytes = Vec::new();
        order.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes.len(), 8);
        assert_eq!(bytes, [0x01, 0x01, 0x01, 0xfd, 0xa0, 0x86, 0x01, 0x00]);
        for runs in [
            vec![IdRun {
                first: i64::MAX,
                count: 2,
            }],
            vec![IdRun {
                first: 1,
                count: i64::MAX,
            }],
            vec![IdRun { first: 0, count: 1 }],
            vec![IdRun { first: 1, count: 0 }],
            vec![IdRun { first: 1, count: 3 }, IdRun { first: 3, count: 2 }],
        ] {
            assert!(!Order { revision: 1, runs }.is_valid());
        }
        assert!(
            Order {
                revision: 1,
                runs: vec![
                    IdRun {
                        first: 100,
                        count: 3
                    },
                    IdRun { first: 1, count: 3 }
                ],
            }
            .is_valid()
        );
    }
}

/// A native retention veto of an otherwise valid, unapplied transaction.
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Retained {
    pub node: crate::NodeId,
    pub rows: Vec<i64>,
}
