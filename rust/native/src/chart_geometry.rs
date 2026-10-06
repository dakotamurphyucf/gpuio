//! Retained logical-pixel plotting geometry. Prepare on a bounded worker, then
//! paint/hit-test the plan against the exact source snapshot used to build it.
//! This module has no GPUI window or OCaml callbacks.
use crate::chart_cartesian::{Kind, Layers, Projection};
use crate::chart_reduce::{self as reduce, SourceSpan};
use gpuio_protocol::{chart_data as data, chart_options as options, chart_sampling::Policy};
use std::{
    collections::BTreeMap,
    f64::consts::{PI, TAU},
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_PLAN_BYTES: usize = 64 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Cancelled,
    RenderLimit,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    fn polar(center: Self, radius: f64, angle: f64) -> Self {
        Self::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Move(Point),
    Line(Point),
    Cubic(Point, Point, Point),
    Close,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub layer: usize,
    pub fill: bool,
    pub commands: Vec<Command>,
}
/// Index-based provenance is private to a plan's immutable source revision.
/// Aggregated spans are never treated as ranges of stable IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Cartesian {
        series: usize,
        start: usize,
        end: usize,
    },
    Slice(usize),
    Radar {
        series: usize,
        axis: usize,
    },
    Candle(SourceSpan),
    Node(usize),
    Edge(usize),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Dot {
        center: Point,
        visible: bool,
    },
    Bar(Rect),
    Candle {
        center: f64,
        left: f64,
        right: f64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    },
    Wedge {
        center: Point,
        inner: f64,
        outer: f64,
        start: f64,
        end: f64,
    },
    Node(Rect),
    Ribbon {
        start_top: Point,
        start_bottom: Point,
        end_top: Point,
        end_bottom: Point,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub source: Source,
    pub layer: usize,
    pub shape: Shape,
}
/// Worker-measured label block in logical pixels, including its painted backing.
/// The slice passed to preparation uses the exact source snapshot's node order;
/// None hides that node's label without reserving space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowLabelMetrics {
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlowAlign {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowLabelPlacement {
    pub align: FlowAlign,
    pub width: f64,
    pub block_height: f64,
    pub above: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LabelKind {
    X,
    Y,
    Radial,
    /// Stable source-axis identity, independent of caption text or display order.
    /// Custom retained label content must join against this ID, not label indices.
    RadarAxis(i64),
    Flow {
        placement: Option<FlowLabelPlacement>,
        align_right: bool,
        node_index: usize,
    },
    FlowLine {
        placement: Option<FlowLabelPlacement>,
        align_right: bool,
        font_size: f64,
        color: Option<u32>,
        block_height: f64,
        offset: f64,
    },
    Series(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub position: Point,
    pub text: String,
    pub kind: LabelKind,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain {
    pub min: f64,
    pub max: f64,
}
impl Domain {
    fn from(values: impl Iterator<Item = f64>, zero: bool) -> Self {
        values
            .fold(None, |domain: Option<Self>, v| {
                Some(match domain {
                    None => Self {
                        min: if zero { v.min(0.) } else { v },
                        max: if zero { v.max(0.) } else { v },
                    },
                    Some(d) => Self {
                        min: d.min.min(v),
                        max: d.max.max(v),
                    },
                })
            })
            .unwrap_or(Self { min: 0., max: 1. })
    }
    pub fn unit(self, value: f64) -> f64 {
        if self.min == self.max {
            0.5
        } else {
            (value - self.min) / (self.max - self.min)
        }
    }
    fn ticks(self, count: i64) -> Vec<f64> {
        if self.min == self.max {
            return vec![self.min];
        }
        let mut result = Vec::new();
        for i in 0..count {
            let value = if i == count - 1 {
                self.max
            } else {
                self.min + (self.max - self.min) * (i as f64 / (count - 1) as f64)
            };
            if result.last().is_none_or(|last| *last != value) {
                result.push(value);
            }
        }
        result
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Summary {
    Bar(f64),
    Stacked {
        value: f64,
        lower: f64,
        upper: f64,
    },
    Candle {
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    },
}
#[derive(Debug)]
pub struct Plan {
    pub width: f64,
    pub height: f64,
    pub paths: Vec<Path>,
    pub marks: Vec<Mark>,
    pub summaries: Vec<(usize, Summary)>,
    pub labels: Vec<Label>,
    pub grid: Vec<(Point, Point)>,
    pub x_domain: Option<Domain>,
    pub y_domain: Option<Domain>,
    pub source_values: usize,
    pub rendered_values: usize,
}
impl Plan {
    pub fn summary(&self, mark: usize) -> Option<Summary> {
        self.summaries
            .binary_search_by_key(&mark, |(i, _)| *i)
            .ok()
            .map(|i| self.summaries[i].1)
    }
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + self.paths.capacity() * size_of::<Path>()
            + self
                .paths
                .iter()
                .map(|p| p.commands.capacity() * size_of::<Command>())
                .sum::<usize>()
            + self.marks.capacity() * size_of::<Mark>()
            + self.summaries.capacity() * size_of::<(usize, Summary)>()
            + self.labels.capacity() * size_of::<Label>()
            + self.labels.iter().map(|l| l.text.capacity()).sum::<usize>()
            + self.grid.capacity() * size_of::<(Point, Point)>()
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn checkpoint(i: usize, cancel: &AtomicBool) -> Result<(), Error> {
    if i & 255 == 0 { check(cancel) } else { Ok(()) }
}
/// Pure native formatter. Nonfinite input is unavailable; callers cannot request
/// unbounded precision. Fixed output remains bounded by the source value domain.
pub fn format_number(value: f64, format: options::NumberFormat) -> String {
    if !value.is_finite() || value.abs() > 1e110 || !format.is_valid() {
        return "—".into();
    }
    let value = if value == 0. { 0. } else { value };
    match format {
        options::NumberFormat::Fixed(n) => format!("{value:.precision$}", precision = n as usize),
        options::NumberFormat::Scientific(n) => {
            format!("{value:.precision$e}", precision = n as usize)
        }
        options::NumberFormat::Percent(n) => {
            format!("{:.precision$}%", value * 100., precision = n as usize)
        }
        options::NumberFormat::Compact => {
            let abs = value.abs();
            if abs >= 1e15 || (abs > 0. && abs < 0.001) {
                return format!("{value:.2e}");
            }
            let (scale, suffix) = if abs >= 1e12 {
                (1e12, "T")
            } else if abs >= 1e9 {
                (1e9, "B")
            } else if abs >= 1e6 {
                (1e6, "M")
            } else if abs >= 1e3 {
                (1e3, "K")
            } else {
                (1., "")
            };
            let text = format!("{:.2}", value / scale);
            format!(
                "{}{suffix}",
                text.trim_end_matches('0').trim_end_matches('.')
            )
        }
    }
}
#[derive(Clone, Copy)]
struct Coordinates {
    x: Domain,
    y: Domain,
    width: f64,
    height: f64,
    inset: f64,
    horizontal: bool,
    reversed: bool,
    categorical: Option<Projection>,
}
impl Coordinates {
    fn value(self, value: f64) -> f64 {
        let fraction = self.y.unit(value);
        (if self.reversed {
            1. - fraction
        } else {
            fraction
        }) * self.height
    }
    fn point(self, x: f64, y: f64) -> Point {
        let category = self.category(x);
        let value = self.value(y);
        if self.horizontal {
            Point::new(value, category)
        } else {
            Point::new(category, self.height - value)
        }
    }
    fn category(self, x: f64) -> f64 {
        self.categorical.map_or_else(
            || self.inset + self.x.unit(x) * (self.width - 2. * self.inset),
            |p| p.center(x),
        )
    }
    fn rect(self, x: f64, value: f64, offset: f64, width: f64) -> Rect {
        self.rect_between(x, 0., value, offset, width)
    }
    fn rect_between(self, x: f64, lower: f64, value: f64, offset: f64, width: f64) -> Rect {
        let c = self.category(x) + offset;
        let a = self.value(lower);
        let b = self.value(value);
        if self.horizontal {
            Rect {
                left: a.min(b),
                right: a.max(b),
                top: c - width / 2.,
                bottom: c + width / 2.,
            }
        } else {
            Rect {
                left: c - width / 2.,
                right: c + width / 2.,
                top: self.height - a.max(b),
                bottom: self.height - a.min(b),
            }
        }
    }
    fn axes(self, plan: &mut Plan, axes: options::Axes, categories: Option<&[data::Category]>) {
        plan.x_domain = categories.is_none().then_some(self.x);
        plan.y_domain = Some(self.y);
        let ticks = match categories {
            None => self
                .x
                .ticks(axes.ticks)
                .into_iter()
                .map(|v| (v, format_number(v, axes.x_format)))
                .collect::<Vec<_>>(),
            Some(categories) => {
                let count = categories.len().min(axes.ticks as usize);
                (0..count)
                    .map(|i| {
                        let index = if count <= 1 {
                            0
                        } else {
                            i * (categories.len() - 1) / (count - 1)
                        };
                        (index as f64, categories[index].label.clone())
                    })
                    .collect()
            }
        };
        for (v, text) in ticks {
            let c = self.category(v);
            let (start, end) = if self.horizontal {
                (Point::new(0., c), Point::new(self.height, c))
            } else {
                (Point::new(c, 0.), Point::new(c, self.height))
            };
            if axes.grid {
                plan.grid.push((start, end));
            }
            if axes.x {
                plan.labels.push(Label {
                    position: if self.horizontal { start } else { end },
                    text,
                    kind: LabelKind::X,
                });
            }
        }
        for v in self.y.ticks(axes.ticks) {
            let c = self.value(v);
            let (start, end) = if self.horizontal {
                (Point::new(c, 0.), Point::new(c, self.width))
            } else {
                (
                    Point::new(0., self.height - c),
                    Point::new(self.width, self.height - c),
                )
            };
            if axes.grid {
                plan.grid.push((start, end));
            }
            if axes.y {
                plan.labels.push(Label {
                    position: if self.horizontal { end } else { start },
                    text: format_number(v, axes.y_format),
                    kind: LabelKind::Y,
                });
            }
        }
    }
}
fn minimum_spacing(mut positions: Vec<f64>, domain: Domain) -> f64 {
    positions.sort_unstable_by(f64::total_cmp);
    positions.dedup();
    positions
        .windows(2)
        .map(|p| domain.unit(p[1]) - domain.unit(p[0]))
        .filter(|v| *v > 0.)
        .fold(1., f64::min)
}
fn curve(points: &[Point], style: options::Curve, horizontal: bool) -> Vec<Command> {
    let mut out = Vec::with_capacity(points.len() * 2 + 3);
    if points.is_empty() {
        return out;
    }
    out.push(Command::Move(points[0]));
    for i in 1..points.len() {
        let a = points[i - 1];
        let b = points[i];
        match style {
            options::Curve::Linear => out.push(Command::Line(b)),
            options::Curve::StepAfter => {
                out.push(Command::Line(if horizontal {
                    Point::new(a.x, b.y)
                } else {
                    Point::new(b.x, a.y)
                }));
                out.push(Command::Line(b));
            }
            options::Curve::Natural => {
                // Uniform Catmull–Rom interpolation, matching the catalog's
                // Natural option. Endpoints are repeated; gaps never enter here.
                let before = points[i.saturating_sub(2)];
                let after = points[(i + 1).min(points.len() - 1)];
                out.push(Command::Cubic(
                    Point::new(a.x + (b.x - before.x) / 6., a.y + (b.y - before.y) / 6.),
                    Point::new(b.x - (after.x - a.x) / 6., b.y - (after.y - a.y) / 6.),
                    b,
                ));
            }
        }
    }
    out
}
fn cartesian(
    plan: &mut Plan,
    layers: Layers<'_>,
    categories: Option<&[data::Category]>,
    reduced: &[reduce::Series],
    options: &options::Options,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let x = Domain::from(
        layers.iter().flat_map(|l| l.points.iter().map(|p| p.x)),
        false,
    );
    let zero = layers
        .iter()
        .any(|l| matches!(l.kind, Kind::Area | Kind::Bar));
    // Include original extrema even when their points were reduced, plus bar
    // aggregates whose sums can exceed the individual source values.
    let stacked = options.cartesian.stacking == options::Stacking::Stacked;
    let source_y = layers
        .iter()
        .filter(|layer| layer.kind == Kind::Line || (!stacked && layer.kind == Kind::Area))
        .flat_map(|l| l.points.iter().filter_map(|p| p.y));
    let aggregate_y = reduced
        .iter()
        .flat_map(|s| s.bars())
        .flat_map(|(b, bounds)| bounds.map_or([0., b.value], |s| [s.lower, s.upper]));
    let area_y = reduced
        .iter()
        .flat_map(|s| match s {
            reduce::Series::StackedArea(points) => points.as_slice(),
            _ => &[],
        })
        .flat_map(|p| [p.bounds.lower, p.bounds.upper]);
    let y = Domain::from(source_y.chain(aggregate_y).chain(area_y), zero);
    let bars = reduced
        .iter()
        .filter(|s| matches!(s, reduce::Series::Bar(_) | reduce::Series::StackedBar(_)))
        .count();
    let groups = if stacked { 1 } else { bars.max(1) };
    let positions = reduced
        .iter()
        .flat_map(|s| s.bars())
        .map(|(b, _)| b.x)
        .collect();
    let spacing = minimum_spacing(positions, x);
    let horizontal = options.cartesian.orientation.is_horizontal();
    let (width, height) = if horizontal {
        (plan.height, plan.width)
    } else {
        (plan.width, plan.height)
    };
    let slot = width * spacing / (1. + spacing);
    let c = Coordinates {
        x,
        y,
        width,
        height,
        inset: if bars > 0 { slot / 2. } else { 0. },
        horizontal,
        reversed: options.cartesian.orientation.is_reversed(),
        categorical: categories
            .map(|c| Projection::new(c.len(), width, options.cartesian.category_layout, bars > 0)),
    };
    c.axes(plan, options.axes, categories);
    let bar_width = slot * options.cartesian.bar_width / (groups as f64);
    let mut bar_index = 0;
    for (series, (layer, reduction)) in layers.iter().zip(reduced).enumerate() {
        check(cancel)?;
        match reduction {
            reduce::Series::Bar(_) | reduce::Series::StackedBar(_) => {
                for (i, (b, bounds)) in reduction.bars().enumerate() {
                    let bar_width = c.categorical.map_or(bar_width, |p| {
                        p.interval_width(b.source.start(), b.source.end() - 1)
                            * options.cartesian.bar_width
                            / groups as f64
                    });
                    let offset = if stacked {
                        0.
                    } else {
                        (bar_index as f64 - (bars - 1) as f64 / 2.) * bar_width
                    };
                    checkpoint(i, cancel)?;
                    if let Some(bounds) = bounds {
                        plan.summaries.push((
                            plan.marks.len(),
                            Summary::Stacked {
                                value: b.value,
                                lower: bounds.lower,
                                upper: bounds.upper,
                            },
                        ));
                    } else if b.source.len() > 1 {
                        plan.summaries
                            .push((plan.marks.len(), Summary::Bar(b.value)));
                    }
                    plan.marks.push(Mark {
                        layer: series,
                        source: Source::Cartesian {
                            series,
                            start: b.source.start(),
                            end: b.source.end(),
                        },
                        shape: Shape::Bar(bounds.map_or_else(
                            || c.rect(b.x, b.value, offset, bar_width),
                            |s| c.rect_between(b.x, s.lower, s.upper, offset, bar_width),
                        )),
                    });
                }
                bar_index += 1;
            }
            reduce::Series::StackedArea(points) => {
                stacking::area(
                    plan,
                    series,
                    layer.points,
                    points,
                    c,
                    options.cartesian,
                    cancel,
                )?;
            }
            reduce::Series::Line(points) | reduce::Series::Area(points) => {
                let area = matches!(reduction, reduce::Series::Area(_));
                let mut start = 0;
                while start < points.len() {
                    check(cancel)?;
                    let mut end = start + 1;
                    while end < points.len() && !points[end].starts_run {
                        end += 1;
                    }
                    let run = &points[start..end];
                    let mut coordinates = Vec::with_capacity(run.len());
                    for (i, p) in run.iter().enumerate() {
                        checkpoint(i, cancel)?;
                        let datum = layer.points.get(p.source).expect("reduced source index");
                        let center = c.point(datum.x, datum.y.expect("reduced defined point"));
                        coordinates.push(center);
                        plan.marks.push(Mark {
                            layer: series,
                            source: Source::Cartesian {
                                series,
                                start: p.source,
                                end: p.source + 1,
                            },
                            shape: Shape::Dot {
                                center,
                                visible: options.cartesian.dots || run.len() == 1,
                            },
                        });
                    }
                    if run.len() > 1 {
                        let commands = curve(&coordinates, options.cartesian.curve, horizontal);
                        if area {
                            let mut fill = commands.clone();
                            fill.push(Command::Line(c.point(
                                layer.points.get(run.last().unwrap().source).unwrap().x,
                                0.,
                            )));
                            fill.push(Command::Line(
                                c.point(layer.points.get(run[0].source).unwrap().x, 0.),
                            ));
                            fill.push(Command::Close);
                            plan.paths.push(Path {
                                layer: series,
                                fill: true,
                                commands: fill,
                            });
                        }
                        plan.paths.push(Path {
                            layer: series,
                            fill: false,
                            commands,
                        });
                    }
                    start = end;
                }
            }
        }
    }
    Ok(())
}
fn pie(plan: &mut Plan, slices: &[data::Slice], options: &options::Pie) {
    // Dividing first avoids both aggregate overflow and precision loss from
    // converting raw tiny/huge values to f32 in a graphics API.
    let maximum = slices.iter().map(|s| s.value).fold(0., f64::max);
    if maximum == 0. {
        return;
    }
    let total = slices.iter().map(|s| s.value / maximum).sum::<f64>();
    let radius = match options.radius {
        options::PieRadius::Fit => plan.width.min(plan.height) / 2.,
        options::PieRadius::Pixels(radius) => radius,
    };
    let radii: std::collections::BTreeMap<_, _> = options
        .slice_radii
        .iter()
        .map(|r| (r.slice, (r.inner, r.outer)))
        .collect();
    let center = Point::new(plan.width / 2., plan.height / 2.);
    let mut angle = -PI / 2.;
    for (i, slice) in slices.iter().enumerate() {
        let sweep = (slice.value / maximum) / total * TAU;
        let gap = options.pad_angle.min(sweep * 0.5);
        let (inner, outer) = radii
            .get(&slice.id)
            .copied()
            .unwrap_or((radius * options.inner_radius, radius));
        if sweep > 0. && outer > inner {
            plan.marks.push(Mark {
                layer: i,
                source: Source::Slice(i),
                shape: Shape::Wedge {
                    center,
                    inner,
                    outer,
                    start: angle + gap / 2.,
                    end: angle + sweep - gap / 2.,
                },
            });
            if options.labels {
                plan.labels.push(Label {
                    position: Point::polar(
                        center,
                        inner + (outer - inner) * 0.65,
                        angle + sweep / 2.,
                    ),
                    text: slice.label.clone(),
                    kind: LabelKind::Radial,
                });
            }
        }
        angle += sweep;
    }
}
fn radar(
    plan: &mut Plan,
    axes: &[data::RadarAxis],
    series: &[data::RadarSeries],
    options: options::Radar,
) -> Result<(), Error> {
    if axes.is_empty() {
        return Ok(());
    }
    let center = Point::new(plan.width / 2., plan.height / 2.);
    let radius = match options.radius {
        options::RadarRadius::Fit => plan.width.min(plan.height) / 2.,
        options::RadarRadius::Pixels(radius) => radius,
    };
    let shared_maximum = match options.scale {
        options::RadarScale::PerAxis => None,
        options::RadarScale::Maximum(maximum) => Some(maximum),
        options::RadarScale::DataMax => {
            let maximum = series
                .iter()
                .flat_map(|s| &s.values)
                .map(|(_, value)| *value)
                .fold(0_f64, f64::max);
            Some(if maximum > 0. { maximum } else { 1. })
        }
    };
    let angle = |i: usize| i as f64 / axes.len() as f64 * TAU - PI / 2.;
    for (i, axis) in axes.iter().enumerate() {
        let endpoint = Point::polar(center, radius, angle(i));
        plan.grid.push((center, endpoint));
        if options.labels {
            plan.labels.push(Label {
                position: Point::polar(center, radius + options.label_gap, angle(i)),
                text: axis.label.clone(),
                kind: LabelKind::RadarAxis(axis.id),
            });
        }
        for level in 1..=options.levels {
            let r = radius * level as f64 / options.levels as f64;
            plan.grid.push((
                Point::polar(center, r, angle(i)),
                Point::polar(center, r, angle((i + 1) % axes.len())),
            ));
        }
    }
    for (index, s) in series.iter().enumerate() {
        let values: BTreeMap<_, _> = s.values.iter().copied().collect();
        let mut points = Vec::with_capacity(axes.len() + 1);
        for (i, axis) in axes.iter().enumerate() {
            let maximum = shared_maximum.unwrap_or(axis.maximum);
            let position = Point::polar(center, radius * (values[&axis.id] / maximum), angle(i));
            // Reject excessive extrapolation before f32 mesh conversion; never
            // clamp data or vertices into a different polygon.
            let limit = gpuio_protocol::canvas::COORDINATE_LIMIT;
            if !position.x.is_finite()
                || !position.y.is_finite()
                || position.x.abs() > limit
                || position.y.abs() > limit
            {
                return Err(Error::RenderLimit);
            }
            points.push(position);
            plan.marks.push(Mark {
                layer: index,
                source: Source::Radar {
                    series: index,
                    axis: i,
                },
                shape: Shape::Dot {
                    center: position,
                    visible: options.dots,
                },
            });
        }
        points.push(points[0]);
        let mut commands = curve(&points, options::Curve::Linear, false);
        commands.push(Command::Close);
        plan.paths.push(Path {
            layer: index,
            fill: true,
            commands: commands.clone(),
        });
        plan.paths.push(Path {
            layer: index,
            fill: false,
            commands,
        });
    }
    Ok(())
}
fn candles(
    plan: &mut Plan,
    source: &[data::Candle],
    values: &[reduce::Candle],
    options: &options::Options,
) {
    let x = Domain::from(source.iter().map(|v| v.x), false);
    let y = Domain::from(values.iter().flat_map(|v| [v.low, v.high]), false);
    let spacing = minimum_spacing(values.iter().map(|v| v.x).collect(), x);
    let slot = plan.width * spacing / (1. + spacing);
    let c = Coordinates {
        x,
        y,
        width: plan.width,
        height: plan.height,
        inset: slot / 2.,
        horizontal: false,
        reversed: false,
        categorical: None,
    };
    c.axes(plan, options.axes, None);
    for candle in values {
        let center = c.category(candle.x);
        let half = slot * options.candlestick.body_width / 2.;
        let y = |v| c.height - c.y.unit(v) * c.height;
        if candle.source.len() > 1 {
            plan.summaries.push((
                plan.marks.len(),
                Summary::Candle {
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                },
            ));
        }
        plan.marks.push(Mark {
            layer: 0,
            source: Source::Candle(candle.source),
            shape: Shape::Candle {
                center,
                left: center - half,
                right: center + half,
                open: y(candle.open),
                high: y(candle.high),
                low: y(candle.low),
                close: y(candle.close),
            },
        });
    }
}
fn sankey(
    plan: &mut Plan,
    nodes: &[data::Node],
    edges: &[data::Edge],
    options: options::Sankey,
    labels: Option<&[Option<FlowLabelMetrics>]>,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    use gpuio_plot::sankey::{Sankey, SankeyAlign, SankeyLink, SankeyValueScale};
    check(cancel)?;
    if let Some(labels) = labels
        && (labels.len() != nodes.len()
            || labels.iter().flatten().any(|m| {
                !m.width.is_finite() || m.width < 0. || !m.height.is_finite() || m.height <= 0.
            }))
    {
        return Err(Error::InvalidInput);
    }
    if nodes.is_empty() {
        return Ok(());
    }
    let labels = labels.filter(|_| options.labels);
    let indices: BTreeMap<_, _> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    let maximum = edges.iter().map(|e| e.value).fold(0., f64::max);
    let links: Vec<_> = edges
        .iter()
        .map(|e| {
            SankeyLink::new(
                indices[&e.source],
                indices[&e.target],
                if maximum > 0. { e.value / maximum } else { 0. },
            )
        })
        .collect();
    let align = match options.alignment {
        options::Alignment::Left => SankeyAlign::Left,
        options::Alignment::Right => SankeyAlign::Right,
        options::Alignment::Center => SankeyAlign::Center,
        options::Alignment::Justify => SankeyAlign::Justify,
    };
    let scale = match options.scale {
        options::FlowScale::Linear => SankeyValueScale::Linear,
        options::FlowScale::Sqrt => SankeyValueScale::Sqrt,
    };
    let layout = Sankey::new()
        .size(plan.width as f32, plan.height as f32)
        .node_width(options.node_width.min(plan.width) as f32)
        .node_align(align)
        .value_scale(scale)
        .iterations(options.iterations as usize);
    let graph = layout
        .topology(nodes.len(), &links)
        .map_err(|_| Error::InvalidInput)?;
    check(cancel)?;
    let (mut left, mut right, mut top, mut bottom) = (0_f64, 0_f64, 0_f64, 0_f64);
    if let Some(labels) = labels {
        for node in &graph.nodes {
            check(cancel)?;
            let Some(metric) = labels[node.index] else {
                continue;
            };
            bottom = 4.;
            if node.layer == 0 {
                left = left.max(metric.width + options.label_gap);
            } else if node.layer + 1 == graph.layer_count() {
                right = right.max(metric.width + options.label_gap);
            } else {
                top = top.max(metric.height + options.label_gap);
            }
        }
        left = left.min(plan.width * 0.2);
        right = right.min(plan.width * 0.2);
        let cap = plan.height * 0.6;
        if top + bottom > cap {
            let factor = cap / (top + bottom);
            top *= factor;
            bottom *= factor;
        }
    }
    let available_width = plan.width - left - right;
    let available_height = plan.height - top - bottom;
    // Fit the actual columns, not total node count. Keep room for positive
    // node heights so crowded padding cannot erase all visible flows.
    let mut columns = vec![0_usize; graph.layer_count()];
    for node in &graph.nodes {
        columns[node.layer] += 1;
    }
    let largest_column = columns.iter().copied().max().unwrap_or(1);
    let graph = layout
        .extent(
            left as f32,
            top as f32,
            (plan.width - right) as f32,
            (plan.height - bottom) as f32,
        )
        .node_width(
            options
                .node_width
                .min(available_width / (2 * columns.len() - 1) as f64) as f32,
        )
        .node_padding(
            options
                .node_padding
                .min(available_height / (2 * largest_column) as f64) as f32,
        )
        .layout_from(graph);
    for link in &graph.links {
        check(cancel)?;
        if link.value == 0. {
            continue;
        }
        let a = &graph.nodes[link.source];
        let b = &graph.nodes[link.target];
        // Widen paint and hit geometry together. Keep raw values/provenance
        // unchanged and clip endpoint spans inside small plotting rectangles.
        let source_half = f64::from(link.source_width).max(options.min_link_width) / 2.;
        let target_half = f64::from(link.target_width).max(options.min_link_width) / 2.;
        let span = |center: f64, half: f64| {
            (
                (center - half).max(top),
                (center + half).min(plan.height - bottom),
            )
        };
        let (source_top, source_bottom) = span(f64::from(link.y0), source_half);
        let (target_top, target_bottom) = span(f64::from(link.y1), target_half);
        plan.marks.push(Mark {
            layer: link.source,
            source: Source::Edge(link.index),
            shape: Shape::Ribbon {
                start_top: Point::new(a.x1 as f64, source_top),
                start_bottom: Point::new(a.x1 as f64, source_bottom),
                end_top: Point::new(b.x0 as f64, target_top),
                end_bottom: Point::new(b.x0 as f64, target_bottom),
            },
        });
    }
    for node in &graph.nodes {
        check(cancel)?;
        let bounds = Rect {
            left: node.x0 as f64,
            right: node.x1 as f64,
            top: node.y0 as f64,
            bottom: node.y1 as f64,
        };
        plan.marks.push(Mark {
            layer: node.index,
            source: Source::Node(node.index),
            shape: Shape::Node(bounds),
        });
        if options.labels {
            let align_right = (bounds.left + bounds.right) / 2. > plan.width / 2.;
            let (position, placement) = if let Some(labels) = labels {
                let Some(metric) = labels[node.index] else {
                    continue;
                };
                let middle = (bounds.left + bounds.right) / 2.;
                let (x, y, align, width, above) = if node.layer == 0 {
                    (
                        bounds.left - options.label_gap,
                        (bounds.top + bounds.bottom) / 2.,
                        FlowAlign::Right,
                        (left - options.label_gap).max(0.),
                        false,
                    )
                } else if node.layer + 1 == graph.layer_count() {
                    (
                        bounds.right + options.label_gap,
                        (bounds.top + bounds.bottom) / 2.,
                        FlowAlign::Left,
                        (right - options.label_gap).max(0.),
                        false,
                    )
                } else {
                    (
                        middle,
                        bounds.top - options.label_gap,
                        FlowAlign::Center,
                        2. * middle.min(plan.width - middle).max(0.),
                        true,
                    )
                };
                (
                    Point::new(x, y),
                    Some(FlowLabelPlacement {
                        align,
                        width,
                        block_height: metric.height,
                        above,
                    }),
                )
            } else {
                (
                    Point::new(
                        if align_right {
                            bounds.left - options.label_gap
                        } else {
                            bounds.right + options.label_gap
                        },
                        (bounds.top + bounds.bottom) / 2.,
                    ),
                    None,
                )
            };
            plan.labels.push(Label {
                position,
                text: nodes[node.index].label.clone(),
                kind: LabelKind::Flow {
                    placement,
                    align_right,
                    node_index: node.index,
                },
            });
        }
    }
    Ok(())
}
/// Attach repeated identifiers to actual representatives. Only a bounded number
/// become native text elements, even for exact 100,000-point plans. Numbers refer
/// to the same publication's legend order; source IDs remain semantic identity.
fn series_identifiers(plan: &mut Plan, count: usize, cancel: &AtomicBool) -> Result<(), Error> {
    if count > 32 {
        return Err(Error::InvalidInput);
    }
    if count < 2 {
        return Ok(());
    }
    let mut counts = [0_usize; 32];
    for (index, mark) in plan.marks.iter().enumerate() {
        if index % 256 == 0 {
            check(cancel)?;
        }
        if mark.layer < count {
            counts[mark.layer] += 1;
        }
    }
    let mut seen = [0_usize; 32];
    let mut previous = [None; 32];
    for (index, mark) in plan.marks.iter().enumerate() {
        if index % 256 == 0 {
            check(cancel)?;
        }
        let layer = mark.layer;
        if layer >= count {
            continue;
        }
        let position = match mark.shape {
            Shape::Dot { center, .. } => center,
            Shape::Bar(rect) => {
                Point::new((rect.left + rect.right) / 2., (rect.top + rect.bottom) / 2.)
            }
            _ => continue,
        };
        let ordinal = seen[layer];
        seen[layer] += 1;
        let count = counts[layer];
        if [count / 10, count / 2, count * 9 / 10].contains(&ordinal)
            && previous[layer] != Some(position)
        {
            plan.labels.push(Label {
                position,
                text: (layer + 1).to_string(),
                kind: LabelKind::Series(layer),
            });
            previous[layer] = Some(position);
        }
    }
    Ok(())
}
/// Size describes the interior plotting rectangle, excluding labels/legend.
/// Callers must reserve bounded worker/plan storage before entry. The returned
/// plan must be retained alongside its exact source snapshot; it has no lifetime
/// authority to resurrect a released resource. Geometry admission never drops
/// source runs to meet a byte limit.
pub fn prepare(
    data: &data::Data,
    policy: Policy,
    options: &options::Options,
    width: f64,
    height: f64,
    cancel: &AtomicBool,
) -> Result<Plan, Error> {
    prepare_with_flow_labels(data, policy, options, (width, height), None, cancel)
}
/// Measured outside-label geometry. The caller must supply exact native block
/// metrics for this snapshot; preparation never estimates text widths. None
/// preserves the existing inside layout. Only Sankey data accepts metrics.
/// Public option/worker integration is a separate layer above this pure engine.
pub fn prepare_with_flow_labels(
    data: &data::Data,
    policy: Policy,
    options: &options::Options,
    size: (f64, f64),
    labels: Option<&[Option<FlowLabelMetrics>]>,
    cancel: &AtomicBool,
) -> Result<Plan, Error> {
    let (width, height) = size;
    check(cancel)?;
    if labels.is_some() && !matches!(data.contents, data::Contents::Sankey(..)) {
        return Err(Error::InvalidInput);
    }
    if !options.is_valid()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.
        || height <= 0.
        || width > 32768.
        || height > 32768.
    {
        return Err(Error::InvalidInput);
    }
    let reduction = reduce::prepare_with_options(
        data,
        policy,
        if options.cartesian.orientation.is_horizontal() && Layers::of(data).is_some() {
            height
        } else {
            width
        },
        options,
        cancel,
    )
    .map_err(|e| {
        if e == reduce::Error::Cancelled {
            Error::Cancelled
        } else {
            Error::InvalidInput
        }
    })?;
    let mut plan = Plan {
        width,
        height,
        paths: vec![],
        marks: Vec::with_capacity(reduction.rendered_values),
        summaries: vec![],
        labels: vec![],
        grid: vec![],
        x_domain: None,
        y_domain: None,
        source_values: reduction.source_values,
        rendered_values: reduction.rendered_values,
    };
    match (&data.contents, &reduction.contents) {
        (
            data::Contents::Cartesian(_) | data::Contents::Categorical(..),
            reduce::Contents::Cartesian(series),
        ) => cartesian(
            &mut plan,
            Layers::of(data).unwrap(),
            match &data.contents {
                data::Contents::Categorical(c, _) => Some(c.as_slice()),
                _ => None,
            },
            series,
            options,
            cancel,
        )?,
        (data::Contents::Candlestick(source), reduce::Contents::Candlestick(values)) => {
            candles(&mut plan, source, values, options)
        }
        (data::Contents::Pie(slices), _) => pie(&mut plan, slices, &options.pie),
        (data::Contents::Radar(axes, series), _) => radar(&mut plan, axes, series, options.radar)?,
        (data::Contents::Sankey(nodes, edges), _) => {
            sankey(&mut plan, nodes, edges, options.sankey, labels, cancel)?
        }
        _ => return Err(Error::InvalidInput),
    }
    match &data.contents {
        data::Contents::Cartesian(_) | data::Contents::Categorical(..) => {
            series_identifiers(&mut plan, Layers::of(data).unwrap().len(), cancel)?
        }
        data::Contents::Radar(_, series) => series_identifiers(&mut plan, series.len(), cancel)?,
        _ => (),
    }
    check(cancel)?;
    if plan.retained_bytes() > MAX_PLAN_BYTES {
        return Err(Error::RenderLimit);
    }
    Ok(plan)
}

#[cfg(test)]
mod tests;

mod stacking;

#[cfg(test)]
mod flow_labels_tests;

#[cfg(test)]
mod pie_radii_tests;
