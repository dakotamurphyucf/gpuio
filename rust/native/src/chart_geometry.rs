//! Retained logical-pixel plotting geometry. Prepare on a bounded worker, then
//! paint/hit-test the plan against the exact source snapshot used to build it.
//! This module has no GPUI window or OCaml callbacks.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelKind {
    X,
    Y,
    Radial,
    Flow,
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
        let category = self.inset + self.x.unit(x) * (self.width - 2. * self.inset);
        let value = self.value(y);
        if self.horizontal {
            Point::new(value, category)
        } else {
            Point::new(category, self.height - value)
        }
    }
    fn category(self, x: f64) -> f64 {
        self.inset + self.x.unit(x) * (self.width - 2. * self.inset)
    }
    fn rect(self, x: f64, value: f64, offset: f64, width: f64) -> Rect {
        let c = self.category(x) + offset;
        let a = self.value(0.);
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
    fn axes(self, plan: &mut Plan, axes: options::Axes) {
        plan.x_domain = Some(self.x);
        plan.y_domain = Some(self.y);
        for v in self.x.ticks(axes.ticks) {
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
                    text: format_number(v, axes.x_format),
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
    layers: &[data::Layer],
    reduced: &[reduce::Series],
    options: &options::Options,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let x = Domain::from(
        layers
            .iter()
            .flat_map(|l| l.series().points.iter().map(|p| p.x)),
        false,
    );
    let zero = layers
        .iter()
        .any(|l| matches!(l, data::Layer::Area(_) | data::Layer::Bar(_)));
    // Include original extrema even when their points were reduced, plus bar
    // aggregates whose sums can exceed the individual source values.
    let source_y = layers
        .iter()
        .filter(|layer| !matches!(layer, data::Layer::Bar(_)))
        .flat_map(|l| l.series().points.iter().filter_map(|p| p.y));
    let aggregate_y = reduced
        .iter()
        .flat_map(|s| match s {
            reduce::Series::Bar(b) => b.as_slice(),
            _ => &[],
        })
        .map(|b| b.value);
    let y = Domain::from(source_y.chain(aggregate_y), zero);
    let bars = reduced
        .iter()
        .filter(|s| matches!(s, reduce::Series::Bar(_)))
        .count();
    let positions = reduced
        .iter()
        .flat_map(|s| match s {
            reduce::Series::Bar(b) => b.as_slice(),
            _ => &[],
        })
        .map(|b| b.x)
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
    };
    c.axes(plan, options.axes);
    let bar_width = slot * options.cartesian.bar_width / (bars.max(1) as f64);
    let mut bar_index = 0;
    for (series, (layer, reduction)) in layers.iter().zip(reduced).enumerate() {
        check(cancel)?;
        match reduction {
            reduce::Series::Bar(values) => {
                let offset = (bar_index as f64 - (bars - 1) as f64 / 2.) * bar_width;
                for (i, b) in values.iter().enumerate() {
                    checkpoint(i, cancel)?;
                    if b.source.len() > 1 {
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
                        shape: Shape::Bar(c.rect(b.x, b.value, offset, bar_width)),
                    });
                }
                bar_index += 1;
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
                        let datum = &layer.series().points[p.source];
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
                            fill.push(Command::Line(
                                c.point(layer.series().points[run.last().unwrap().source].x, 0.),
                            ));
                            fill.push(Command::Line(
                                c.point(layer.series().points[run[0].source].x, 0.),
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
fn pie(plan: &mut Plan, slices: &[data::Slice], options: options::Pie) {
    // Dividing first avoids both aggregate overflow and precision loss from
    // converting raw tiny/huge values to f32 in a graphics API.
    let maximum = slices.iter().map(|s| s.value).fold(0., f64::max);
    if maximum == 0. {
        return;
    }
    let total = slices.iter().map(|s| s.value / maximum).sum::<f64>();
    let radius = plan.width.min(plan.height) / 2.;
    let center = Point::new(plan.width / 2., plan.height / 2.);
    let mut angle = -PI / 2.;
    for (i, slice) in slices.iter().enumerate() {
        let sweep = (slice.value / maximum) / total * TAU;
        let gap = options.pad_angle.min(sweep * 0.5);
        if sweep > 0. {
            plan.marks.push(Mark {
                layer: i,
                source: Source::Slice(i),
                shape: Shape::Wedge {
                    center,
                    inner: radius * options.inner_radius,
                    outer: radius,
                    start: angle + gap / 2.,
                    end: angle + sweep - gap / 2.,
                },
            });
            if options.labels {
                plan.labels.push(Label {
                    position: Point::polar(
                        center,
                        radius * (options.inner_radius + (1. - options.inner_radius) * 0.65),
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
) {
    if axes.is_empty() {
        return;
    }
    let center = Point::new(plan.width / 2., plan.height / 2.);
    let radius = plan.width.min(plan.height) / 2.;
    let angle = |i: usize| i as f64 / axes.len() as f64 * TAU - PI / 2.;
    for (i, axis) in axes.iter().enumerate() {
        let endpoint = Point::polar(center, radius, angle(i));
        plan.grid.push((center, endpoint));
        if options.labels {
            plan.labels.push(Label {
                position: endpoint,
                text: axis.label.clone(),
                kind: LabelKind::Radial,
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
            let position =
                Point::polar(center, radius * (values[&axis.id] / axis.maximum), angle(i));
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
    };
    c.axes(plan, options.axes);
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
) -> Result<(), Error> {
    use gpuio_plot::sankey::{Sankey, SankeyAlign, SankeyLink, SankeyValueScale};
    if nodes.is_empty() {
        return Ok(());
    }
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
    // Fit the actual columns, not total node count. Keep room for positive
    // node heights so crowded padding cannot erase all visible flows.
    let mut columns = vec![0_usize; graph.layer_count()];
    for node in &graph.nodes {
        columns[node.layer] += 1;
    }
    let largest_column = columns.iter().copied().max().unwrap_or(1);
    let graph = layout
        .node_width(
            options
                .node_width
                .min(plan.width / (2 * columns.len() - 1) as f64) as f32,
        )
        .node_padding(
            options
                .node_padding
                .min(plan.height / (2 * largest_column) as f64) as f32,
        )
        .layout_from(graph);
    for link in &graph.links {
        if link.value == 0. {
            continue;
        }
        let a = &graph.nodes[link.source];
        let b = &graph.nodes[link.target];
        plan.marks.push(Mark {
            layer: link.source,
            source: Source::Edge(link.index),
            shape: Shape::Ribbon {
                start_top: Point::new(a.x1 as f64, (link.y0 - link.source_width / 2.) as f64),
                start_bottom: Point::new(a.x1 as f64, (link.y0 + link.source_width / 2.) as f64),
                end_top: Point::new(b.x0 as f64, (link.y1 - link.target_width / 2.) as f64),
                end_bottom: Point::new(b.x0 as f64, (link.y1 + link.target_width / 2.) as f64),
            },
        });
    }
    for node in &graph.nodes {
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
            plan.labels.push(Label {
                position: Point::new(bounds.left, (bounds.top + bounds.bottom) / 2.),
                text: nodes[node.index].label.clone(),
                kind: LabelKind::Flow,
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
    check(cancel)?;
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
    let reduction = reduce::prepare(
        data,
        policy,
        if options.cartesian.orientation.is_horizontal()
            && matches!(data.contents, data::Contents::Cartesian(_))
        {
            height
        } else {
            width
        },
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
        (data::Contents::Cartesian(layers), reduce::Contents::Cartesian(series)) => {
            cartesian(&mut plan, layers, series, options, cancel)?
        }
        (data::Contents::Candlestick(source), reduce::Contents::Candlestick(values)) => {
            candles(&mut plan, source, values, options)
        }
        (data::Contents::Pie(slices), _) => pie(&mut plan, slices, options.pie),
        (data::Contents::Radar(axes, series), _) => radar(&mut plan, axes, series, options.radar),
        (data::Contents::Sankey(nodes, edges), _) => {
            sankey(&mut plan, nodes, edges, options.sankey)?
        }
        _ => return Err(Error::InvalidInput),
    }
    match &data.contents {
        data::Contents::Cartesian(layers) => series_identifiers(&mut plan, layers.len(), cancel)?,
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
