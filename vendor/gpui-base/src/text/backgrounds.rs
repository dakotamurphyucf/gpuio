//! Prepared decoration data; no matching or application callbacks during paint.
use std::{ops::Range, rc::Rc, sync::Arc};

use gpui::{Bounds, TextAlign, TextLayout, Window, point, px, quad, size};

use crate::input::{RangeBackground, RangeBackgroundError, RangeBackgrounds};

use super::{DisplayedFragment, DisplayedText, ReorderedTextGeometry};

/// A fixed array of paint layers for one immutable parsed text snapshot. The
/// range providers retain their preparation leases through each painted frame.
pub struct TextBackgrounds {
    pub(super) source: Arc<DisplayedText>,
    layers: Vec<Option<Rc<dyn RangeBackgrounds>>>,
}
impl TextBackgrounds {
    pub fn new(
        source: Arc<DisplayedText>,
        layers: Vec<Option<Rc<dyn RangeBackgrounds>>>,
    ) -> Result<Self, RangeBackgroundError> {
        if layers.len() != source.fragments().len() {
            return Err(RangeBackgroundError::InvalidRange);
        }
        let mut count = 0usize;
        for (fragment, layer) in source.fragments().iter().zip(&layers) {
            if let Some(layer) = layer {
                count += layer.ranges().len();
                if count > 32768 {
                    return Err(RangeBackgroundError::Limit);
                }
                for wash in layer.ranges() {
                    if wash.bytes.start >= wash.bytes.end
                        || wash.bytes.end > fragment.text().len()
                        || !fragment.text().is_char_boundary(wash.bytes.start)
                        || !fragment.text().is_char_boundary(wash.bytes.end)
                    {
                        return Err(RangeBackgroundError::InvalidRange);
                    }
                    if !wash.radius.as_f32().is_finite()
                        || wash.radius < px(0.)
                        || wash.radius > px(64.)
                    {
                        return Err(RangeBackgroundError::InvalidRadius);
                    }
                }
            }
        }
        Ok(Self { source, layers })
    }

    pub(super) fn layer(&self, fragment: &DisplayedFragment) -> Option<Rc<dyn RangeBackgrounds>> {
        let current = self.source.fragments().get(fragment.id() as usize)?;
        // Identical bytes in a different prepared AST are still a new source.
        if !Arc::ptr_eq(current.text(), fragment.text()) {
            return None;
        }
        self.layers.get(fragment.id() as usize)?.clone()
    }
}

/// Source ranges are local to the unsplit prepared fragment. InlineFlow calls
/// this with its actual shaped subrange, preserving Unicode byte boundaries.
pub(super) fn paint(
    ranges: &[RangeBackground],
    source: Range<usize>,
    layout: &TextLayout,
    align: TextAlign,
    window: &mut Window,
) {
    let bounds = layout.bounds();
    let height = layout.line_height();
    let mask = window.content_mask().bounds;
    let lines = layout.line_layouts();
    // Construct at most once per shaped line in this paint pass, not per match.
    let geometries = lines
        .iter()
        .map(|line| ReorderedTextGeometry::new(&line.unwrapped_layout))
        .collect::<Vec<_>>();
    for wash in ranges {
        let start = wash.bytes.start.max(source.start);
        let end = wash.bytes.end.min(source.end);
        if start >= end {
            continue;
        }
        let start = start - source.start;
        let end = end - source.start;
        let mut offset = 0;
        let mut row = 0;
        for (line, geometry) in lines.iter().zip(&geometries) {
            let local_start = start.saturating_sub(offset);
            let local_end = end.saturating_sub(offset).min(line.len());
            if local_start < local_end {
                let boundary = |row: usize| {
                    if row == 0 {
                        (0, px(0.))
                    } else {
                        line.wrap_boundaries.get(row - 1).map_or(
                            (line.len(), line.unwrapped_layout.width),
                            |b| {
                                let glyph =
                                    &line.unwrapped_layout.runs[b.run_ix].glyphs[b.glyph_ix];
                                (glyph.index, glyph.position.x)
                            },
                        )
                    }
                };
                for wrapped in 0..=line.wrap_boundaries.len() {
                    let y = bounds.top() + height * (row + wrapped);
                    if y >= mask.bottom() {
                        break;
                    }
                    if y + height <= mask.top() {
                        continue;
                    }
                    let (from, x_from) = boundary(wrapped);
                    let (to, x_to) = boundary(wrapped + 1);
                    let a = local_start.max(from);
                    let b = local_end.min(to);
                    if geometry.is_none() && a >= b {
                        continue;
                    }
                    let inset = match align {
                        TextAlign::Left => px(0.),
                        TextAlign::Center => (bounds.size.width - (x_to - x_from)) / 2.,
                        TextAlign::Right => bounds.size.width - (x_to - x_from),
                    };
                    if let Some(geometry) = geometry {
                        geometry.spans(local_start..local_end, x_from..x_to, |span| {
                            window.paint_quad(quad(
                                Bounds::new(
                                    point(bounds.left() + inset + span.start - x_from, y),
                                    size(span.end - span.start, height),
                                ),
                                wash.radius,
                                wash.color,
                                px(0.),
                                gpui::transparent_black(),
                                Default::default(),
                            ));
                        });
                        continue;
                    }
                    let a = line.unwrapped_layout.x_for_index(a) - x_from;
                    let b = line.unwrapped_layout.x_for_index(b) - x_from;
                    if a == b {
                        continue;
                    }
                    window.paint_quad(quad(
                        Bounds::new(
                            point(bounds.left() + inset + a.min(b), y),
                            size((b - a).abs(), height),
                        ),
                        wash.radius,
                        wash.color,
                        px(0.),
                        gpui::transparent_black(),
                        Default::default(),
                    ));
                }
            }
            row += line.wrap_boundaries.len() + 1;
            offset += line.len() + 1;
            if offset >= end {
                break;
            }
        }
    }
}
