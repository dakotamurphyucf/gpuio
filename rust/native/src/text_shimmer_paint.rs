//! Bounded glyph overlay; shaping, selection and accessible source stay owned by
//! the supplied StyledText. Animation phase comes from the native owner, not an
//! OCaml timer. This adapter itself never schedules frames or retains an owner.
//!
//! Glyph traversal/masks adapted from gpui-kit84f57fd component/shimmer.rs.
//! Copyright 2024-2026 Longbridge. Apache-2.0; see docs/catalog/sources/gpui-kit-LICENSE.
use gpui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hsla, InspectorElementId,
    IntoElement, LayoutId, Pixels, StyledText, TextAlign, Window, point, px, rgba, size,
};
use gpuio_protocol::text_shimmer::{Config, Direction, Spread};

pub const LAYERS: usize = 12;
pub const MAX_GLYPHS: usize = 4096;
pub const MAX_LINES: usize = 256;
pub const MAX_TEXT_BYTES: usize = 16_384;

#[derive(Clone, Copy)]
pub struct Appearance {
    pub foreground: Hsla,
    pub background: Hsla,
    pub dark: bool,
}

#[derive(Clone, Copy)]
pub struct Sample {
    /// Normalized elapsed sweep time before applying physical direction.
    pub phase: f32,
    pub reduced_motion: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Report {
    #[default]
    Inactive,
    OutsideBand,
    Painted {
        eligible_glyphs: usize,
        calls: usize,
    },
    Capacity,
}

/// Construct after validation. Paint rechecks the configuration defensively and
/// falls back to the original text for static/reduced/invalid/over-budget cases.
pub fn element(text: StyledText, config: Config, sample: Sample, appearance: Appearance) -> Text {
    Text {
        text,
        config,
        sample,
        appearance,
        #[cfg(feature = "native-image-tests")]
        probe: None,
    }
}

pub struct Text {
    text: StyledText,
    config: Config,
    sample: Sample,
    appearance: Appearance,
    #[cfg(feature = "native-image-tests")]
    probe: Option<std::rc::Rc<std::cell::Cell<Report>>>,
}

fn band(
    bounds: Bounds<Pixels>,
    spread: Spread,
    phase: f32,
    layer: usize,
) -> Option<ContentMask<Pixels>> {
    let width = bounds.size.width.as_f32();
    if !width.is_finite() || width <= 0. || bounds.size.height <= px(0.) || layer >= LAYERS {
        return None;
    }
    let half = match spread {
        Spread::Relative(fraction) => width as f64 * fraction,
        Spread::Pixels(pixels) => pixels,
    };
    // f64 avoids overflowing a division by near-zero widths; no per-glyph work
    // is needed when the whole band lies outside the layout.
    let width = width as f64;
    let padding = half + width * 0.05;
    let center = phase as f64 * (width + 2. * padding) - padding;
    let radius = half * (1. - layer as f64 / LAYERS as f64);
    let left = (center - radius).max(0.);
    let right = (center + radius).min(width);
    (right > left).then(|| ContentMask {
        bounds: Bounds::new(
            point(bounds.left() + px(left as f32), bounds.top()),
            size(px((right - left) as f32), bounds.size.height),
        ),
    })
}

fn alignment(align: TextAlign, available: Pixels, width: Pixels) -> Pixels {
    match align {
        TextAlign::Left => px(0.),
        TextAlign::Center => (available - width) / 2.,
        TextAlign::Right => available - width,
    }
}

impl Text {
    fn overlay(&self, window: &mut Window, cx: &mut App) -> Report {
        if !self.config.is_valid()
            || !self.config.animated
            || self.sample.reduced_motion
            || !self.sample.phase.is_finite()
            || !(0. ..=1.).contains(&self.sample.phase)
        {
            return Report::Inactive;
        }
        let layout = self.text.layout();
        let bounds = layout.bounds();
        let clip = bounds.intersect(&window.content_mask().bounds);
        if clip.size.width <= px(0.) || clip.size.height <= px(0.) {
            return Report::Inactive;
        }
        if layout.len() > MAX_TEXT_BYTES {
            return Report::Capacity;
        }
        let text_style = window.text_style();
        let highlight = self.config.highlight.map_or_else(
            || {
                crate::text_shimmer_color::highlight(
                    text_style.color,
                    if self.appearance.dark {
                        self.appearance.foreground
                    } else {
                        self.appearance.background
                    },
                )
            },
            |color| rgba(color as u32).into(),
        );
        if highlight.is_transparent() {
            return Report::Inactive;
        }
        let lines = layout.line_layouts();
        if lines.len() > MAX_LINES {
            return Report::Capacity;
        }
        let mut glyph_count = 0usize;
        let mut eligible_glyphs = 0;
        for line in &lines {
            for run in &line.unwrapped_layout.runs {
                glyph_count = glyph_count.saturating_add(run.glyphs.len());
                if glyph_count > MAX_GLYPHS {
                    return Report::Capacity;
                }
                eligible_glyphs += run.glyphs.iter().filter(|glyph| !glyph.is_emoji).count();
            }
        }
        if eligible_glyphs == 0 {
            return Report::Inactive;
        }
        let phase = match self.config.direction {
            Direction::LeftToRight => self.sample.phase,
            Direction::RightToLeft => 1. - self.sample.phase,
        };
        let masks = std::array::from_fn::<_, LAYERS, _>(|layer| {
            band(bounds, self.config.spread, phase, layer).and_then(|mask| {
                let bounds = mask.bounds.intersect(&clip);
                (bounds.size.width > px(0.) && bounds.size.height > px(0.))
                    .then_some(ContentMask { bounds })
            })
        });
        if masks.iter().all(Option::is_none) {
            return Report::OutsideBand;
        }
        let peak: f32 = if self.appearance.dark { 0.6 } else { 0.75 };
        let color = highlight.opacity(1. - (1. - peak).powf(1. / LAYERS as f32));
        let line_height = layout.line_height();
        let mut calls = 0;
        window.paint_layer(bounds, |window| {
            let mut top = bounds.top();
            for wrapped in &lines {
                let line = &wrapped.unwrapped_layout;
                let baseline = (line_height - line.ascent - line.descent) / 2. + line.ascent;
                let mut wraps = wrapped.wrap_boundaries.iter().peekable();
                let mut row_start = px(0.);
                let mut row_end = wraps.peek().map_or(line.width, |edge| {
                    line.runs[edge.run_ix].glyphs[edge.glyph_ix].position.x
                });
                let mut row_top = top;
                for (run_index, run) in line.runs.iter().enumerate() {
                    let glyph_size = cx
                        .text_system()
                        .bounding_box(run.font_id, line.font_size)
                        .size;
                    for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
                        if wraps.peek().is_some_and(|edge| {
                            edge.run_ix == run_index && edge.glyph_ix == glyph_index
                        }) {
                            wraps.next();
                            row_start = glyph.position.x;
                            row_end = wraps.peek().map_or(line.width, |edge| {
                                line.runs[edge.run_ix].glyphs[edge.glyph_ix].position.x
                            });
                            row_top += line_height;
                        }
                        if glyph.is_emoji {
                            continue;
                        }
                        // Visual positions, not source-byte order, control placement.
                        let origin = point(
                            bounds.left()
                                + alignment(
                                    text_style.text_align,
                                    bounds.size.width,
                                    row_end - row_start,
                                )
                                + glyph.position.x
                                - row_start,
                            row_top,
                        );
                        let glyph_bounds = Bounds::new(origin, glyph_size);
                        for mask in masks.iter().flatten() {
                            if glyph_bounds.intersects(&mask.bounds) {
                                window.with_content_mask(Some(*mask), |window| {
                                    if window
                                        .paint_glyph(
                                            origin + point(px(0.), baseline + glyph.position.y),
                                            run.font_id,
                                            glyph.id,
                                            line.font_size,
                                            color,
                                        )
                                        .is_ok()
                                    {
                                        calls += 1;
                                    }
                                });
                            }
                        }
                    }
                }
                top += wrapped.size(line_height).height;
            }
        });
        Report::Painted {
            eligible_glyphs,
            calls,
        }
    }
}

impl IntoElement for Text {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Text {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.text.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text
            .prepaint(id, inspector, bounds, layout, window, cx);
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut (),
        prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
        let report = self.overlay(window, cx);
        #[cfg(feature = "native-image-tests")]
        if let Some(probe) = &self.probe {
            probe.set(report);
        }
        let _ = report;
    }
}

#[cfg(feature = "native-image-tests")]
#[path = "text_shimmer_paint_test.rs"]
pub(crate) mod native_test;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_extents_scale_and_leave_no_residue_at_sweep_ends() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(100.), px(32.)));
        for spread in [
            Spread::Relative(0.05),
            Spread::Relative(1.),
            Spread::Pixels(1.),
            Spread::Pixels(1_000_000.),
        ] {
            for layer in 0..LAYERS {
                assert!(band(bounds, spread, 0., layer).is_none());
                assert!(band(bounds, spread, 1., layer).is_none());
                let middle = band(bounds, spread, 0.5, layer).unwrap().bounds;
                assert!(middle.left() >= bounds.left() && middle.right() <= bounds.right());
                assert_eq!(middle.center().x, bounds.center().x);
            }
        }
        let a = band(bounds, Spread::Pixels(10.), 0.5, 0).unwrap().bounds;
        let wide = Bounds::new(bounds.origin, size(px(200.), bounds.size.height));
        assert_eq!(a.size.width, px(20.));
        assert_eq!(
            band(wide, Spread::Pixels(10.), 0.5, 0)
                .unwrap()
                .bounds
                .size
                .width,
            a.size.width
        );
        assert_eq!(
            band(wide, Spread::Relative(0.1), 0.5, 0)
                .unwrap()
                .bounds
                .size
                .width,
            px(40.)
        );
        assert!(
            band(bounds, Spread::Relative(0.3), 0.3, 0)
                .unwrap()
                .bounds
                .left()
                < band(bounds, Spread::Relative(0.3), 0.7, 0)
                    .unwrap()
                    .bounds
                    .left()
        );
        assert!(band(bounds, Spread::Relative(0.3), 0.5, LAYERS).is_none());
        assert!(
            band(
                Bounds::new(bounds.origin, size(px(0.), px(32.))),
                Spread::Pixels(10.),
                0.5,
                0
            )
            .is_none()
        );
    }
}
