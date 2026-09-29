//! Immutable shaped visual cells for plain selection runs. A cell includes whole
//! source graphemes, even when a font emits several glyphs for one cluster.
use super::{TextSelectionRun, point_in_selection_band};
use gpui::{Pixels, Point, TextAlign, TextLayout, WrappedLineLayout, point, px};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

struct Cell {
    source: Range<usize>,
    x: Range<Pixels>,
}
struct Row {
    width: Pixels,
    cells: Vec<Cell>,
}
pub(super) struct Geometry {
    layouts: Vec<Arc<WrappedLineLayout>>,
    rows: Vec<Row>,
}
impl Geometry {
    pub(super) fn matches(&self, layout: &TextLayout) -> bool {
        let lines = layout.line_layouts();
        self.layouts.len() == lines.len()
            && self
                .layouts
                .iter()
                .zip(&lines)
                .all(|(a, b)| Arc::ptr_eq(a, b))
    }
    pub(super) fn new(text: &str, layout: &TextLayout) -> Self {
        let layouts: Vec<_> = layout.line_layouts().into_iter().collect();
        let boundaries: Vec<_> = text
            .grapheme_indices(true)
            .map(|(offset, _)| offset)
            .chain([text.len()])
            .collect();
        let mut rows = Vec::new();
        let mut offset = 0;
        for wrapped in &layouts {
            let line = &wrapped.unwrapped_layout;
            let mut glyphs: Vec<_> = line
                .runs
                .iter()
                .flat_map(|run| &run.glyphs)
                .filter(|glyph| glyph.index < line.len)
                .collect();
            let mut indices: Vec<_> = glyphs
                .iter()
                .map(|glyph| glyph.index)
                .chain([line.len])
                .collect();
            indices.sort_unstable();
            indices.dedup();
            glyphs.sort_by(|a, b| {
                a.position
                    .x
                    .partial_cmp(&b.position.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let edges: Vec<_> = std::iter::once(px(0.))
                .chain(
                    wrapped
                        .wrap_boundaries
                        .iter()
                        .map(|edge| line.runs[edge.run_ix].glyphs[edge.glyph_ix].position.x),
                )
                .chain([line.width])
                .collect();
            let first_row = rows.len();
            rows.extend(edges.windows(2).map(|edge| Row {
                width: edge[1] - edge[0],
                cells: Vec::new(),
            }));
            let mut next = 0;
            for glyph in &glyphs {
                let x = glyph.position.x;
                while next < glyphs.len() && glyphs[next].position.x <= x {
                    next += 1;
                }
                let end_x = glyphs
                    .get(next)
                    .map_or(line.width, |glyph| glyph.position.x);
                if end_x <= x {
                    continue;
                }
                let end = offset + indices[indices.partition_point(|index| *index <= glyph.index)];
                let start = offset + glyph.index;
                let start = boundaries[boundaries
                    .partition_point(|edge| *edge <= start)
                    .saturating_sub(1)];
                let end = boundaries[boundaries.partition_point(|edge| *edge < end)];
                let first = edges.partition_point(|edge| *edge <= x).saturating_sub(1);
                for row in first..edges.len() - 1 {
                    if edges[row] >= end_x {
                        break;
                    }
                    let left = x.max(edges[row]);
                    let right = end_x.min(edges[row + 1]);
                    if left < right {
                        rows[first_row + row].cells.push(Cell {
                            source: start..end,
                            x: left - edges[row]..right - edges[row],
                        });
                    }
                }
            }
            let end = offset + line.len;
            if text.as_bytes().get(end) == Some(&b'\n') {
                let row = rows.last_mut().expect("a shaped line has a visual row");
                row.cells.push(Cell {
                    source: boundaries[boundaries
                        .partition_point(|edge| *edge <= end)
                        .saturating_sub(1)]
                        ..boundaries[boundaries.partition_point(|edge| *edge < end + 1)],
                    x: row.width..row.width,
                });
            }
            offset = end + 1;
        }
        Self { layouts, rows }
    }
    pub(super) fn project(
        &self,
        run: &TextSelectionRun,
        start: Point<Pixels>,
        end: Point<Pixels>,
    ) -> Option<Range<usize>> {
        let height = run.layout.line_height();
        let mut selected: Option<Range<usize>> = None;
        for (index, row) in self.rows.iter().enumerate() {
            let y = run.bounds.top() + height * index;
            if y + height <= start.y.min(end.y) || y > start.y.max(end.y) {
                continue;
            }
            let inset = match run.text_align {
                TextAlign::Left => px(0.),
                TextAlign::Center => (run.bounds.size.width - row.width) / 2.,
                TextAlign::Right => run.bounds.size.width - row.width,
            };
            for cell in &row.cells {
                if point_in_selection_band(
                    point(run.bounds.left() + inset + cell.x.start, y),
                    cell.x.end - cell.x.start,
                    start,
                    end,
                    height,
                ) {
                    match &mut selected {
                        Some(range) => {
                            range.start = range.start.min(cell.source.start);
                            range.end = range.end.max(cell.source.end);
                        }
                        None => selected = Some(cell.source.clone()),
                    }
                }
            }
        }
        selected
    }
}
