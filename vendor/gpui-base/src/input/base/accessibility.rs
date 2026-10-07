//! Same-prepaint geometry for a native accessibility adapter. No platform tree,
//! callback, editor entity or application runtime is retained here.
use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{Bounds, Pixels, Point, ShapedLine, point, px, size};
use ropey::Rope;
use unicode_bidi::BidiInfo;

use super::layout::LastLayout;

/// A source scalar (CRLF is one cell), or an empty line's caret. Cluster members
/// may share bounds. Geometry is unclipped, in window-content logical pixels.
#[derive(Clone, Debug)]
pub struct BridgeTextCell {
    pub bytes: Range<usize>,
    pub bounds: Bounds<Pixels>,
    pub right_to_left: bool,
}

/// Immutable layout produced before the owning editor's accessibility children.
pub struct BridgeTextLayoutSnapshot {
    pub revision: i64,
    pub source_len: usize,
    pub scale_factor: f32,
    pub cells: Vec<BridgeTextCell>,
}

/// Data-only shared handle. Reading does not shape text, notify, focus or scroll.
#[derive(Clone, Default)]
pub struct BridgeTextLayout(Rc<RefCell<Option<Rc<BridgeTextLayoutSnapshot>>>>);

impl BridgeTextLayout {
    pub fn snapshot(&self) -> Option<Rc<BridgeTextLayoutSnapshot>> {
        self.0.borrow().clone()
    }

    pub(super) fn clear(&self) {
        self.0.borrow_mut().take();
    }

    pub(super) fn publish(
        &self,
        text: &Rope,
        layout: &LastLayout,
        origin: Point<Pixels>,
        scale_factor: f32,
    ) {
        let mut cells = Vec::new();
        let mut y = origin.y + layout.visible_top;
        for (line, &offset) in layout.lines.iter().zip(&layout.visible_line_byte_offsets) {
            let source = text
                .slice(offset..(offset + line.len()).min(text.len()))
                .to_string();
            // Resolve paragraph direction before splitting soft wraps; neutral
            // characters on continuation rows inherit the same paragraph.
            let bidi = BidiInfo::new(&source, None);
            let mut local_offset = 0;
            for (row, shaped) in line.wrapped_lines.iter().enumerate() {
                let x = origin.x
                    + layout.line_number_width
                    + if row == 0 { px(0.) } else { line.wrap_indent }
                    + layout.alignment_offset(shaped.width);
                let start = offset + local_offset;
                // An empty editor shapes its placeholder; it is not source text.
                if text.len() == 0 {
                    cells.push(BridgeTextCell {
                        bytes: 0..0,
                        bounds: Bounds::new(point(x, y), size(px(0.), layout.line_height)),
                        right_to_left: false,
                    });
                    break;
                }
                shaped_cells(
                    shaped,
                    start,
                    point(x, y),
                    layout.line_height,
                    &bidi.levels[local_offset..],
                    &mut cells,
                );
                local_offset += shaped.len;
                y += layout.line_height;
            }
            // Base excludes LF from shaped lines. Keep its source position on
            // the preceding visual row, and collapse CRLF to one native cell.
            let end = offset + local_offset;
            if end < text.len() && text.slice(end..end + 1).to_string() == "\n" {
                if let Some(last) = cells.last_mut() {
                    let x = if last.right_to_left {
                        last.bounds.left()
                    } else {
                        last.bounds.right()
                    };
                    if last.bytes.end == end
                        && last.bytes.start < end
                        && text.slice(last.bytes.clone()).to_string() == "\r"
                    {
                        last.bytes.end += 1;
                        last.bounds.origin.x = x;
                        last.bounds.size.width = px(0.);
                    } else {
                        let cell = BridgeTextCell {
                            bytes: end..end + 1,
                            bounds: Bounds::new(
                                point(x, y - layout.line_height),
                                size(px(0.), layout.line_height),
                            ),
                            right_to_left: last.right_to_left,
                        };
                        if last.bytes.is_empty() {
                            *last = cell;
                        } else {
                            cells.push(cell);
                        }
                    }
                }
            }
        }
        *self.0.borrow_mut() = Some(Rc::new(BridgeTextLayoutSnapshot {
            revision: layout.source_revision,
            source_len: text.len(),
            scale_factor,
            cells,
        }));
    }
}

// Build cluster extents once per shaped row. Sorting is O(g log g), then each
// scalar lookup is O(log g); avoid LineLayout::x_for_index's linear glyph scan
// for every scalar (quadratic for long unwrapped input).
fn shaped_cells(
    line: &ShapedLine,
    offset: usize,
    origin: Point<Pixels>,
    height: Pixels,
    levels: &[unicode_bidi::Level],
    output: &mut Vec<BridgeTextCell>,
) {
    let mut glyphs = line
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.index < line.len)
        .map(|glyph| (glyph.index, glyph.position.x))
        .collect::<Vec<_>>();
    glyphs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut clusters = Vec::<(usize, Range<Pixels>)>::with_capacity(glyphs.len());
    let mut next = 0;
    for &(index, x) in &glyphs {
        while next < glyphs.len() && glyphs[next].1 <= x {
            next += 1;
        }
        let end = glyphs.get(next).map_or(line.width, |(_, x)| *x).max(x);
        clusters.push((index, x..end));
    }
    clusters.sort_by_key(|(index, _)| *index);
    let mut merged = Vec::<(usize, Range<Pixels>)>::with_capacity(clusters.len());
    for (index, span) in clusters {
        if let Some((previous, bounds)) = merged.last_mut()
            && *previous == index
        {
            bounds.start = bounds.start.min(span.start);
            bounds.end = bounds.end.max(span.end);
        } else {
            merged.push((index, span));
        }
    }
    for (index, ch) in line.text.char_indices() {
        let cluster = merged.partition_point(|(start, _)| *start <= index);
        let span = cluster
            .checked_sub(1)
            .map(|index| merged[index].1.clone())
            .unwrap_or(px(0.)..px(0.));
        output.push(BridgeTextCell {
            bytes: offset + index..offset + index + ch.len_utf8(),
            bounds: Bounds::new(
                origin + point(span.start, px(0.)),
                size(span.end - span.start, height),
            ),
            right_to_left: levels.get(index).is_some_and(|level| level.is_rtl()),
        });
    }
    if line.text.is_empty() {
        output.push(BridgeTextCell {
            bytes: offset..offset,
            bounds: Bounds::new(origin, size(px(0.), height)),
            right_to_left: false,
        });
    }
}
