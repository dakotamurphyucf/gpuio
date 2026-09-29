//! Visual cells for shaped lines whose source indices are reordered (for example
//! bidirectional text). Logical endpoints alone cannot describe their paint.
use std::ops::Range;

use gpui::{LineLayout, Pixels, px};

struct Cell {
    source: Range<usize>,
    x: Range<Pixels>,
}

/// Immutable geometry tied to one shaped line. Build once per layout, then reuse
/// for its ranges. Coordinates are unwrapped line coordinates, not screen pixels.
/// A range touching part of a shaped cluster decorates that cluster's whole cell.
pub struct ReorderedTextGeometry {
    // Sorted by logical start/end, so finding a match does not scan earlier glyphs.
    cells: Vec<Cell>,
}

impl ReorderedTextGeometry {
    /// Monotonic layouts need no additional per-glyph geometry.
    pub fn new(line: &LineLayout) -> Option<Self> {
        let mut previous = 0;
        let mut reordered = false;
        for glyph in line.runs.iter().flat_map(|run| &run.glyphs) {
            reordered |= glyph.index < previous;
            previous = glyph.index;
        }
        if !reordered {
            return None;
        }
        let mut positions = line
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .filter(|glyph| glyph.index < line.len)
            .map(|glyph| (glyph.index, glyph.position.x))
            .collect::<Vec<_>>();
        let mut indices = positions
            .iter()
            .map(|(index, _)| *index)
            .collect::<Vec<_>>();
        indices.push(line.len);
        indices.sort_unstable();
        indices.dedup();
        positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut cells = Vec::with_capacity(positions.len());
        let mut next = 0;
        for (index, x) in &positions {
            while next < positions.len() && positions[next].1 <= *x {
                next += 1;
            }
            let end_x = positions.get(next).map_or(line.width, |(_, x)| *x);
            let end_index = indices[indices.partition_point(|value| value <= index)];
            if end_x > *x && end_index > *index {
                cells.push(Cell {
                    source: *index..end_index,
                    x: *x..end_x,
                });
            }
        }
        cells.sort_by_key(|cell| cell.source.start);
        Some(Self { cells })
    }

    /// Emit disjoint visual spans intersected with one visual row's coordinates.
    /// No shaping, matching, callbacks or source mutation occurs here.
    pub fn spans(
        &self,
        source: Range<usize>,
        visible_x: Range<Pixels>,
        mut emit: impl FnMut(Range<Pixels>),
    ) {
        if source.start >= source.end || visible_x.start >= visible_x.end {
            return;
        }
        let first = self
            .cells
            .partition_point(|cell| cell.source.end <= source.start);
        let mut spans = self.cells[first..]
            .iter()
            .take_while(|cell| cell.source.start < source.end)
            .filter_map(|cell| {
                let start = cell.x.start.max(visible_x.start);
                let end = cell.x.end.min(visible_x.end);
                (start < end).then_some(start..end)
            })
            .collect::<Vec<_>>();
        spans.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut merged: Option<Range<Pixels>> = None;
        for span in spans {
            if let Some(previous) = &mut merged {
                if span.start <= previous.end + px(0.001) {
                    previous.end = previous.end.max(span.end);
                } else {
                    emit(previous.clone());
                    *previous = span;
                }
            } else {
                merged = Some(span);
            }
        }
        if let Some(span) = merged {
            emit(span);
        }
    }
}
