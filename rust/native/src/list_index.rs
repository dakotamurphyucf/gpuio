//! O(logical row count) metadata, separate from the bounded mounted row set.
//! IDs survive reordering; the renderer never borrows OCaml keys or values.
use gpuio_protocol::list::Order;
use std::{collections::HashMap, ops::Range};

#[derive(PartialEq, Eq)]
pub struct Index {
    revision: i64,
    ids: Vec<i64>,
    positions: HashMap<i64, usize>,
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListIndex")
            .field("revision", &self.revision)
            .field("logical_rows", &self.ids.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub row: i64,
    pub offset: f32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Splice {
    pub removed: Range<usize>,
    pub inserted: Range<usize>,
}

impl Index {
    pub fn new(order: &Order) -> Result<Self, &'static str> {
        if !order.is_valid() {
            return Err("invalid logical list order");
        }
        let count = order.runs.iter().map(|run| run.count as usize).sum();
        let mut ids = Vec::with_capacity(count);
        for run in &order.runs {
            ids.extend(run.first..=run.first + (run.count - 1));
        }
        let positions = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        Ok(Self {
            revision: order.revision,
            ids,
            positions,
        })
    }

    pub fn revision(&self) -> i64 {
        self.revision
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn id(&self, index: usize) -> Option<i64> {
        self.ids.get(index).copied()
    }

    pub fn position(&self, id: i64) -> Option<usize> {
        self.positions.get(&id).copied()
    }

    /// Preserve unaffected prefix/suffix measurement slots. Reordering a middle
    /// interval invalidates that interval, independently of restoring its anchor.
    pub fn splice_to(&self, next: &Self) -> Splice {
        let prefix = self
            .ids
            .iter()
            .zip(&next.ids)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = self.ids[prefix..]
            .iter()
            .rev()
            .zip(next.ids[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        Splice {
            removed: prefix..self.len() - suffix,
            inserted: prefix..next.len() - suffix,
        }
    }

    /// Keep the exact pixel offset when the anchored key survives. When it is
    /// deleted, prefer the next surviving old neighbor, then the previous one,
    /// then the new first row, all at offset zero. An empty result has no anchor.
    pub fn remap_anchor(&self, next: &Self, anchor: Anchor) -> Option<Anchor> {
        if next.position(anchor.row).is_some() {
            return Some(anchor);
        }
        let surviving = self.position(anchor.row).and_then(|index| {
            self.ids[index + 1..]
                .iter()
                .chain(self.ids[..index].iter().rev())
                .find(|id| next.position(**id).is_some())
                .copied()
        });
        surviving
            .or_else(|| next.id(0))
            .map(|row| Anchor { row, offset: 0. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::list::IdRun;

    fn index(ids: &[i64]) -> Index {
        Index::new(&Order {
            revision: 1,
            runs: ids
                .iter()
                .map(|id| IdRun {
                    first: *id,
                    count: 1,
                })
                .collect(),
        })
        .unwrap()
    }

    #[test]
    fn prepend_reorder_and_deleted_anchor_have_stable_identity() {
        let old = index(&[10, 11, 12]);
        let prepended = index(&[8, 9, 10, 11, 12]);
        assert_eq!(
            old.splice_to(&prepended),
            Splice {
                removed: 0..0,
                inserted: 0..2
            }
        );
        let anchor = Anchor {
            row: 11,
            offset: 37.5,
        };
        assert_eq!(old.remap_anchor(&prepended, anchor), Some(anchor));
        let reordered = index(&[12, 11, 10]);
        assert_eq!(old.remap_anchor(&reordered, anchor), Some(anchor));
        let removed = index(&[10, 12]);
        assert_eq!(
            old.remap_anchor(&removed, anchor),
            Some(Anchor {
                row: 12,
                offset: 0.
            })
        );
        assert_eq!(
            old.remap_anchor(&index(&[10]), anchor),
            Some(Anchor {
                row: 10,
                offset: 0.
            })
        );
        assert_eq!(old.remap_anchor(&index(&[]), anchor), None);
        assert_eq!(
            old.remap_anchor(&index(&[99]), anchor),
            Some(Anchor {
                row: 99,
                offset: 0.
            })
        );
    }

    #[test]
    fn middle_change_preserves_prefix_and_suffix_and_large_runs_expand_once() {
        let old = index(&[1, 2, 3, 4]);
        let next = index(&[1, 2, 8, 9, 4]);
        assert_eq!(
            old.splice_to(&next),
            Splice {
                removed: 2..3,
                inserted: 2..4
            }
        );
        assert_eq!(
            old.splice_to(&old),
            Splice {
                removed: 4..4,
                inserted: 4..4
            }
        );
        let large = Index::new(&Order {
            revision: 7,
            runs: vec![IdRun {
                first: 1,
                count: 100_000,
            }],
        })
        .unwrap();
        assert_eq!(large.len(), 100_000);
        assert_eq!(large.revision(), 7);
        assert_eq!(large.position(50_000), Some(49_999));
        assert_eq!(large.id(99_999), Some(100_000));
        assert_eq!(large.id(100_000), None);
    }
}
