//! Worker-only style resolution. Stable IDs never become source index ranges.
use crate::{
    chart_cartesian::{Kind, Layers},
    chart_geometry as geometry,
    chart_paint::Error,
};
use gpuio_protocol::{
    chart_appearance::{self as appearance, Aggregates, Bar, BarFill, Brush, Corners},
    chart_data::{Contents, Data},
    chart_options::{Curve, Options, Orientation, Stacking},
    chart_style::Style,
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) struct Index<'a> {
    data: &'a Data,
    style: &'a Style,
    options: &'a Options,
    series: BTreeMap<i64, &'a appearance::Series>,
    points: BTreeMap<(i64, i64), &'a appearance::Datum>,
    cancel: &'a AtomicBool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Marker {
    pub visible: bool,
    pub radius: f64,
    pub fill: u32,
    pub stroke: u32,
    pub stroke_width: f64,
}

impl Marker {
    fn apply(&mut self, value: Option<appearance::Marker>) {
        if let Some(value) = value {
            if let Some(n) = value.visible {
                self.visible = n;
            }
            if let Some(n) = value.radius {
                self.radius = n;
            }
            if let Some(n) = value.fill {
                self.fill = n as u32;
            }
            if let Some(n) = value.stroke {
                self.stroke = n as u32;
            }
            if let Some(n) = value.stroke_width {
                self.stroke_width = n;
            }
        }
    }
}

fn apply_bar(base: Bar, value: Option<Bar>) -> Bar {
    match value {
        None => base,
        Some(value) => Bar {
            fill: value.fill.or(base.fill),
            corners: value.corners.or(base.corners),
        },
    }
}

pub(crate) fn solid(color: u32) -> Brush {
    Brush::Solid(i64::from(color))
}
pub(crate) fn gradient(angle: f64, from: u32, to: u32) -> Brush {
    Brush::Linear {
        oklab: false,
        angle,
        from: i64::from(from),
        start: 0.,
        to: i64::from(to),
        stop: 1.,
    }
}
pub(crate) fn native_brush(brush: Brush) -> gpui::Background {
    match brush {
        Brush::Solid(color) => gpui::rgba(color as u32).into(),
        Brush::PatternSlash(color, width, interval) => {
            gpui::pattern_slash(gpui::rgba(color as u32), width as f32, interval as f32)
        }
        Brush::Checkerboard(color, size) => {
            gpui::checkerboard(gpui::rgba(color as u32), size as f32)
        }
        Brush::Linear {
            oklab,
            angle,
            from,
            start,
            to,
            stop,
        } => gpui::linear_gradient(
            angle as f32,
            gpui::linear_color_stop(gpui::rgba(from as u32), start as f32),
            gpui::linear_color_stop(gpui::rgba(to as u32), stop as f32),
        )
        .color_space(if oklab {
            gpui::ColorSpace::Oklab
        } else {
            gpui::ColorSpace::Srgb
        }),
    }
}

impl<'a> Index<'a> {
    pub fn new(
        data: &'a Data,
        style: &'a Style,
        options: &'a Options,
        cancel: &'a AtomicBool,
    ) -> Result<Self, Error> {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        if !style.appearance.is_valid() {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            data,
            style,
            options,
            cancel,
            series: style
                .appearance
                .series
                .iter()
                .map(|s| (s.series, s))
                .collect(),
            points: style
                .appearance
                .data
                .iter()
                .map(|d| ((d.series, d.datum), d))
                .collect(),
        })
    }
    fn series_at(&self, layer: usize) -> Option<&appearance::Series> {
        let id = match &self.data.contents {
            Contents::Radar(_, series) => series.get(layer)?.id,
            _ => Layers::of(self.data)?.get(layer)?.id,
        };
        self.series.get(&id).copied()
    }
    pub fn path(&self, layer: usize) -> appearance::Path {
        self.series_at(layer)
            .and_then(|s| s.path)
            .unwrap_or_default()
    }
    pub fn legend(&self, layer: usize, color: u32) -> u32 {
        self.series_at(layer)
            .and_then(|s| s.legend)
            .map_or(color, |c| c as u32)
    }
    pub fn curves(&self) -> Result<Vec<Curve>, Error> {
        let Some(layers) = Layers::of(self.data) else {
            return Ok(vec![]);
        };
        let mut result = Vec::with_capacity(layers.len());
        let mut stacked_area = None;
        for (i, layer) in layers.iter().enumerate() {
            if self.cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let curve = self.path(i).curve.unwrap_or(self.options.cartesian.curve);
            if layer.kind == Kind::Area && self.options.cartesian.stacking == Stacking::Stacked {
                if stacked_area.is_some_and(|previous| previous != curve) {
                    return Err(Error::InvalidConfiguration);
                }
                stacked_area = Some(curve);
            }
            result.push(curve);
        }
        Ok(result)
    }
    fn datum(&self, source: geometry::Source) -> Option<&appearance::Datum> {
        let (series, datum) = match source {
            geometry::Source::Cartesian { series, start, end } if end == start + 1 => {
                let layer = Layers::of(self.data)?.get(series)?;
                (layer.id, layer.points.get(start)?.id)
            }
            geometry::Source::Radar { series, axis } => {
                let Contents::Radar(axes, series_values) = &self.data.contents else {
                    return None;
                };
                (series_values.get(series)?.id, axes.get(axis)?.id)
            }
            _ => return None,
        };
        self.points.get(&(series, datum)).copied()
    }
    pub fn marker(&self, mark: &geometry::Mark, color: u32) -> Marker {
        let mut marker = Marker {
            visible: true,
            radius: self.style.point_radius,
            fill: color,
            stroke: color,
            stroke_width: 0.,
        };
        marker.apply(self.series_at(mark.layer).and_then(|s| s.marker));
        marker.apply(self.datum(mark.source).and_then(|d| d.marker));
        marker.visible &= matches!(mark.shape, geometry::Shape::Dot { visible: true, .. });
        marker.stroke_width = marker.stroke_width.min(marker.radius);
        marker
    }
    pub fn bar(&self, mark: &geometry::Mark, color: u32) -> Result<Bar, Error> {
        let horizontal = self.options.cartesian.orientation.is_horizontal();
        let brush = self.style.gradient_end.map_or(solid(color), |end| {
            let (from, to) = if self.options.cartesian.orientation.is_reversed() {
                (end as u32, color)
            } else {
                (color, end as u32)
            };
            gradient(if horizontal { 90. } else { 180. }, from, to)
        });
        let r = self.style.bar_radius;
        let base = apply_bar(
            Bar {
                fill: Some(BarFill::Background(brush)),
                corners: Some(Corners {
                    top_left: r,
                    top_right: r,
                    bottom_right: r,
                    bottom_left: r,
                }),
            },
            self.series_at(mark.layer).and_then(|s| s.bar),
        );
        if self.points.is_empty() {
            return Ok(base);
        }
        let geometry::Source::Cartesian { series, start, end } = mark.source else {
            return Err(Error::InvalidInput);
        };
        let layer = Layers::of(self.data)
            .and_then(|l| l.get(series))
            .ok_or(Error::InvalidInput)?;
        if start >= end || end > layer.points.len() {
            return Err(Error::InvalidInput);
        }
        let mut first = None;
        for index in start..end {
            if index & 255 == 0 && self.cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let point = layer.points.get(index).ok_or(Error::InvalidInput)?;
            if point.y.is_none() {
                continue;
            }
            let current = apply_bar(
                base,
                self.points.get(&(layer.id, point.id)).and_then(|d| d.bar),
            );
            if let Some(previous) = first {
                if self.style.appearance.aggregates == Aggregates::InheritSeries
                    || previous != current
                {
                    return Ok(base);
                }
            } else {
                first = Some(current);
            }
        }
        Ok(first.unwrap_or(base))
    }
    pub fn bar_values(&self, plan: &geometry::Plan, index: usize) -> Result<(f64, f64), Error> {
        match plan.summary(index) {
            Some(geometry::Summary::Stacked { lower, upper, .. }) => Ok((lower, upper)),
            Some(geometry::Summary::Bar(value)) => Ok((0., value)),
            _ => {
                let geometry::Source::Cartesian { series, start, end } = plan.marks[index].source
                else {
                    return Err(Error::InvalidInput);
                };
                if end != start + 1 {
                    return Err(Error::InvalidInput);
                }
                let point = Layers::of(self.data)
                    .and_then(|l| l.get(series))
                    .and_then(|l| l.points.get(start))
                    .ok_or(Error::InvalidInput)?;
                Ok((0., point.y.ok_or(Error::InvalidInput)?))
            }
        }
    }
}

fn direction(orientation: Orientation) -> f64 {
    match orientation {
        Orientation::Vertical => 0.,
        Orientation::Horizontal => 90.,
        Orientation::VerticalReversed => 180.,
        Orientation::HorizontalReversed => 270.,
    }
}
fn mix(from: i64, to: i64, fraction: f64) -> i64 {
    let mut color = 0;
    for shift in [0, 8, 16, 24] {
        let a = ((from >> shift) & 255) as f64;
        let b = ((to >> shift) & 255) as f64;
        color |= ((a + (b - a) * fraction).round() as i64) << shift;
    }
    color
}
/// Clip a ramp to the bar while preserving both endpoint plateaus. Mapping only
/// the two rectangle endpoint colors would incorrectly smear clipped ramps.
pub(crate) fn bar_brush(
    fill: BarFill,
    values: (f64, f64),
    domain: geometry::Domain,
    orientation: Orientation,
) -> Brush {
    let angle = direction(orientation);
    let (lo, from, hi, to) = match fill {
        BarFill::Background(brush) => return brush,
        BarFill::BaseToTip(from, to) => {
            return gradient(
                if values.1 < values.0 {
                    (angle + 180.) % 360.
                } else {
                    angle
                },
                from as u32,
                to as u32,
            );
        }
        BarFill::Domain(from, to) => (domain.min, from, domain.max, to),
        BarFill::Values(lo, from, hi, to) => (lo, from, hi, to),
    };
    if hi <= lo {
        return Brush::Solid(from);
    }
    let (start_value, end_value) = (values.0.min(values.1), values.0.max(values.1));
    if end_value <= lo {
        return Brush::Solid(from);
    }
    if start_value >= hi {
        return Brush::Solid(to);
    }
    if start_value == end_value {
        return Brush::Solid(mix(
            from,
            to,
            ((start_value - lo) / (hi - lo)).clamp(0., 1.),
        ));
    }
    let color_at = |v: f64| mix(from, to, ((v - lo) / (hi - lo)).clamp(0., 1.));
    Brush::Linear {
        oklab: false,
        angle,
        from: color_at(start_value.max(lo)),
        start: ((lo - start_value) / (end_value - start_value)).clamp(0., 1.),
        to: color_at(end_value.min(hi)),
        stop: ((hi - start_value) / (end_value - start_value)).clamp(0., 1.),
    }
}

#[cfg(test)]
mod tests;
