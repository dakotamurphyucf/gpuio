//! Immutable shaped visual cells for plain selection runs. A cell includes whole
//! source graphemes, even when a font emits several glyphs for one cluster.
use super::{TextSelectionRun, point_in_selection_band};
use gpui::{Pixels, Point, TextAlign, TextLayout, WrappedLineLayout, point, px};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

struct Cell {
    source: Range<usize>,
    x: Range<Pixels>,
    rtl: bool,
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
    pub(super) fn text_bounds(&self, run: &TextSelectionRun) -> Vec<gpui::Bounds<Pixels>> {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                (row.width > px(0.)).then(|| {
                    gpui::Bounds::new(
                        point(
                            run.bounds.left() + row.inset(run),
                            run.bounds.top() + run.layout.line_height() * index,
                        ),
                        gpui::size(row.width, run.layout.line_height()),
                    )
                })
            })
            .collect()
    }

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
        let bidi = unicode_bidi::BidiInfo::new(text, None);
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
                            rtl: bidi.levels.get(start).is_some_and(|level| level.is_rtl()),
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
                    rtl: false,
                });
            }
            offset = end + 1;
        }
        for row in &mut rows {
            // Several fallback glyphs may paint one extended grapheme. Its caret
            // midpoint belongs to the entire visual cluster, not each glyph.
            let mut cells: Vec<Cell> = Vec::with_capacity(row.cells.len());
            for cell in row.cells.drain(..) {
                if let Some(previous) = cells.last_mut()
                    && previous.source == cell.source
                    && previous.rtl == cell.rtl
                    && previous.x.end >= cell.x.start
                {
                    previous.x.end = previous.x.end.max(cell.x.end);
                } else {
                    cells.push(cell);
                }
            }
            row.cells = cells;
        }
        Self { layouts, rows }
    }
    fn cell_at(&self, run: &TextSelectionRun, position: Point<Pixels>) -> Option<(&Cell, Pixels)> {
        let height = run.layout.line_height();
        if height <= px(0.) {
            return None;
        }
        let index = ((position.y - run.bounds.top()) / height).floor().max(0.) as usize;
        let row = self
            .rows
            .get(index.min(self.rows.len().saturating_sub(1)))?;
        let x = position.x - run.bounds.left() - row.inset(run);
        let cell = row.cells.iter().min_by(|a, b| {
            let distance = |cell: &Cell| (cell.x.start - x).max(x - cell.x.end).max(px(0.));
            distance(a)
                .partial_cmp(&distance(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        Some((cell, x))
    }
    pub(super) fn index_at(
        &self,
        run: &TextSelectionRun,
        position: Point<Pixels>,
        caret: bool,
    ) -> usize {
        let Some((cell, x)) = self.cell_at(run, position) else {
            return run.text.len();
        };
        if !caret {
            return cell.source.start;
        }
        let after_midpoint = x >= (cell.x.start + cell.x.end) / 2.;
        if after_midpoint != cell.rtl {
            cell.source.end
        } else {
            cell.source.start
        }
    }
    pub(super) fn position(&self, run: &TextSelectionRun, index: usize) -> Option<Point<Pixels>> {
        if index > run.text.len() {
            return None;
        }
        // Prefer the downstream source cluster at bidi and soft-wrap boundaries.
        let cells = || {
            self.rows
                .iter()
                .enumerate()
                .flat_map(|(row, value)| value.cells.iter().map(move |cell| (row, value, cell)))
        };
        let found = cells()
            .find(|(_, _, cell)| cell.source.contains(&index))
            .map(|value| (value, false))
            .or_else(|| {
                cells()
                    .find(|(_, _, cell)| cell.source.end == index)
                    .map(|value| (value, true))
            });
        let Some(((row_index, row, cell), trailing)) = found else {
            return (index == 0).then_some(run.bounds.origin);
        };
        let x = if trailing != cell.rtl {
            cell.x.end
        } else {
            cell.x.start
        };
        Some(point(
            run.bounds.left() + row.inset(run) + x,
            run.bounds.top() + run.layout.line_height() * row_index,
        ))
    }
    pub(super) fn range_points(
        &self,
        run: &TextSelectionRun,
        range: Range<usize>,
    ) -> Option<(Point<Pixels>, Point<Pixels>)> {
        let mut start: Option<Point<Pixels>> = None;
        let mut end: Option<Point<Pixels>> = None;
        for (index, row) in self.rows.iter().enumerate() {
            for cell in &row.cells {
                if cell.source.start >= range.end || cell.source.end <= range.start {
                    continue;
                }
                let y = run.bounds.top()
                    + run.layout.line_height() * index
                    + run.layout.line_height() / 2.;
                let left = point(run.bounds.left() + row.inset(run) + cell.x.start, y);
                let right = point(run.bounds.left() + row.inset(run) + cell.x.end, y);
                start.get_or_insert(left);
                end = Some(right);
            }
        }
        start.zip(end)
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
            let inset = row.inset(run);
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

impl Row {
    fn inset(&self, run: &TextSelectionRun) -> Pixels {
        match run.text_align {
            TextAlign::Left => px(0.),
            TextAlign::Center => (run.bounds.size.width - self.width) / 2.,
            TextAlign::Right => run.bounds.size.width - self.width,
        }
    }
}
