//! Retained inspection child metadata. Arbitrary View contents travel through
//! ordinary tree operations. Metadata carries source IDs, never callbacks or
//! native pointers, and grants no ownership of a source.
use crate::{
    ResourceId,
    chart_selection::{Aggregation, Selection},
};
use binprot::macros::BinProtWrite;
use std::{collections::BTreeSet, mem::size_of};

pub const MAX_ENTRIES: usize = 128;
pub const MAX_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Target {
    Cartesian(i64, i64),
    Slice(i64),
    Radar(i64, i64),
    Candlestick(i64),
    Node(i64),
    Edge(i64),
    Aggregate {
        source: ResourceId,
        data_revision: i64,
        data_generation: i64,
        selection: Selection,
    },
}
fn is_aggregate(selection: Selection) -> bool {
    matches!(
        selection,
        Selection::Cartesian {
            aggregation: Aggregation::Sum | Aggregation::Mean,
            ..
        } | Selection::Candlestick {
            aggregated: true,
            ..
        }
    )
}
impl Target {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Cartesian(series, datum) | Self::Radar(series, datum) => series > 0 && datum > 0,
            Self::Slice(id) | Self::Candlestick(id) | Self::Node(id) | Self::Edge(id) => id > 0,
            Self::Aggregate {
                source: _,
                data_revision,
                data_generation,
                selection,
            } => {
                data_revision > 0
                    && data_generation > 0
                    && selection.is_valid()
                    && is_aggregate(selection)
            }
        }
    }
    /// Exact marks use stable IDs. Aggregates retain the full immutable
    /// publication: equal endpoints/count cannot establish interior membership.
    pub fn from_selection(
        selection: Selection,
        source: ResourceId,
        data_revision: i64,
        data_generation: i64,
    ) -> Option<Self> {
        if data_revision <= 0 || data_generation <= 0 || !selection.is_valid() {
            return None;
        }
        Some(match selection {
            Selection::Cartesian {
                series,
                span,
                aggregation: Aggregation::Exact,
            } => Self::Cartesian(series, span.first),
            Selection::Candlestick {
                span,
                aggregated: false,
            } => Self::Candlestick(span.first),
            Selection::Slice(id) => Self::Slice(id),
            Selection::Radar { series, axis } => Self::Radar(series, axis),
            Selection::Node(id) => Self::Node(id),
            Selection::Edge(id) => Self::Edge(id),
            Selection::Cartesian { .. } | Selection::Candlestick { .. } => Self::Aggregate {
                source,
                data_revision,
                data_generation,
                selection,
            },
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Container {
    Card,
    Overlay,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Entry {
    pub target: Option<Target>,
    pub container: Container,
}

pub fn is_valid(entries: &[Entry]) -> bool {
    entries.len() <= MAX_ENTRIES
        && entries
            .iter()
            .all(|e| e.target.is_none_or(Target::is_valid))
        && entries
            .iter()
            .filter_map(|e| e.target)
            .collect::<BTreeSet<_>>()
            .len()
            == entries.iter().filter(|e| e.target.is_some()).count()
}
pub fn heap_bytes(entries: &Vec<Entry>) -> usize {
    entries.capacity() * size_of::<Entry>()
}
