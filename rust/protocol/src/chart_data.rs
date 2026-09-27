//! Owned chart data schema. Decode/validate before native registration.
use binprot::macros::BinProtWrite;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_POINTS: usize = 100_000;
pub const MAX_SERIES: usize = 32;
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Point {
    pub id: i64,
    pub x: f64,
    pub y: Option<f64>,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Series {
    pub id: i64,
    pub name: String,
    pub points: Vec<Point>,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Slice {
    pub id: i64,
    pub label: String,
    pub value: f64,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct RadarAxis {
    pub id: i64,
    pub label: String,
    pub maximum: f64,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct RadarSeries {
    pub id: i64,
    pub name: String,
    pub values: Vec<(i64, f64)>,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Candle {
    pub id: i64,
    pub x: f64,
    pub label: String,
    pub open_: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Node {
    pub id: i64,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Edge {
    pub id: i64,
    pub source: i64,
    pub target: i64,
    pub value: f64,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Layer {
    Line(Series),
    Area(Series),
    Bar(Series),
}
impl Layer {
    pub fn series(&self) -> &Series {
        match self {
            Self::Line(s) | Self::Area(s) | Self::Bar(s) => s,
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Contents {
    Cartesian(Vec<Layer>),
    Pie(Vec<Slice>),
    Radar(Vec<RadarAxis>, Vec<RadarSeries>),
    Candlestick(Vec<Candle>),
    Sankey(Vec<Node>, Vec<Edge>),
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Data {
    pub version: i64,
    pub contents: Contents,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError {
    InvalidData,
    LimitExceeded,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub values: usize,
    pub text_bytes: usize,
}
fn require(valid: bool) -> Result<(), ValidationError> {
    if valid {
        Ok(())
    } else {
        Err(ValidationError::InvalidData)
    }
}
fn limit(valid: bool) -> Result<(), ValidationError> {
    if valid {
        Ok(())
    } else {
        Err(ValidationError::LimitExceeded)
    }
}
fn number(value: f64) -> bool {
    value.is_finite() && value.abs() <= 1e100
}
fn nonnegative(value: f64) -> bool {
    number(value) && value >= 0.
}
fn unique(ids: impl Iterator<Item = i64>) -> bool {
    let mut seen = BTreeSet::new();
    ids.into_iter().all(|id| id > 0 && seen.insert(id))
}
fn increasing(values: impl Iterator<Item = f64>) -> bool {
    let mut previous = None;
    values.into_iter().all(|value| {
        let valid = number(value) && previous.is_none_or(|old| old < value);
        previous = Some(value);
        valid
    })
}
impl Stats {
    fn text(&mut self, text: &str, maximum: usize, nonblank: bool) -> Result<(), ValidationError> {
        limit(text.len() <= maximum)?;
        require(
            !text.bytes().any(|byte| b"\0\r\n".contains(&byte))
                && (!nonblank || text.bytes().any(|byte| !b" \t\r\n\x0b\x0c".contains(&byte))),
        )?;
        self.text_bytes += text.len();
        limit(self.text_bytes <= MAX_TEXT_BYTES)
    }
}
impl Data {
    pub fn validate(&self) -> Result<Stats, ValidationError> {
        require(self.version == 1)?;
        let mut stats = Stats::default();
        match &self.contents {
            Contents::Cartesian(layers) => {
                limit(layers.len() <= MAX_SERIES)?;
                require(unique(layers.iter().map(|layer| layer.series().id)))?;
                for layer in layers {
                    let series = layer.series();
                    stats.values += series.points.len();
                    limit(stats.values <= MAX_POINTS)?;
                    stats.text(&series.name, 128, true)?;
                    require(
                        unique(series.points.iter().map(|point| point.id))
                            && increasing(series.points.iter().map(|point| point.x)),
                    )?;
                    for point in &series.points {
                        require(
                            point.y.is_none_or(number)
                                && (!matches!(layer, Layer::Bar(_)) || point.y.is_some()),
                        )?;
                        stats.text(&point.label, 256, false)?;
                    }
                }
            }
            Contents::Pie(slices) => {
                limit(slices.len() <= 256)?;
                require(unique(slices.iter().map(|slice| slice.id)))?;
                stats.values = slices.len();
                for slice in slices {
                    require(nonnegative(slice.value))?;
                    stats.text(&slice.label, 256, true)?;
                }
            }
            Contents::Radar(axes, series) => {
                limit(axes.len() <= 64 && series.len() <= MAX_SERIES)?;
                require((axes.is_empty() && series.is_empty()) || axes.len() >= 3)?;
                require(
                    unique(axes.iter().map(|axis| axis.id))
                        && unique(series.iter().map(|series| series.id)),
                )?;
                let mut maxima = BTreeMap::new();
                for axis in axes {
                    require(number(axis.maximum) && axis.maximum > 0.)?;
                    stats.text(&axis.label, 256, true)?;
                    maxima.insert(axis.id, axis.maximum);
                }
                stats.values = axes.len() * series.len();
                for series in series {
                    stats.text(&series.name, 128, true)?;
                    require(
                        series.values.len() == axes.len()
                            && unique(series.values.iter().map(|(id, _)| *id)),
                    )?;
                    for (id, value) in &series.values {
                        require(
                            nonnegative(*value)
                                && maxima.get(id).is_some_and(|maximum| value <= maximum),
                        )?;
                    }
                }
            }
            Contents::Candlestick(candles) => {
                limit(candles.len() <= MAX_POINTS)?;
                require(
                    unique(candles.iter().map(|candle| candle.id))
                        && increasing(candles.iter().map(|candle| candle.x)),
                )?;
                stats.values = candles.len();
                for candle in candles {
                    require(
                        [candle.open_, candle.high, candle.low, candle.close]
                            .into_iter()
                            .all(number)
                            && candle.low <= candle.open_
                            && candle.open_ <= candle.high
                            && candle.low <= candle.close
                            && candle.close <= candle.high,
                    )?;
                    stats.text(&candle.label, 256, false)?;
                }
            }
            Contents::Sankey(nodes, edges) => {
                limit(nodes.len() <= 256 && edges.len() <= 2048)?;
                require(
                    unique(nodes.iter().map(|node| node.id))
                        && unique(edges.iter().map(|edge| edge.id)),
                )?;
                for node in nodes {
                    stats.text(&node.label, 256, true)?;
                }
                stats.values = edges.len();
                let mut degree = nodes
                    .iter()
                    .map(|node| (node.id, 0usize))
                    .collect::<BTreeMap<_, _>>();
                let mut outgoing = BTreeMap::<i64, Vec<i64>>::new();
                for edge in edges {
                    require(
                        edge.source != edge.target
                            && degree.contains_key(&edge.source)
                            && degree.contains_key(&edge.target)
                            && nonnegative(edge.value),
                    )?;
                    *degree.get_mut(&edge.target).unwrap() += 1;
                    outgoing.entry(edge.source).or_default().push(edge.target);
                }
                let mut ready = degree
                    .iter()
                    .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
                    .collect::<VecDeque<_>>();
                let mut visited = 0;
                while let Some(source) = ready.pop_front() {
                    visited += 1;
                    for target in outgoing.get(&source).into_iter().flatten() {
                        let count = degree.get_mut(target).unwrap();
                        *count -= 1;
                        if *count == 0 {
                            ready.push_back(*target);
                        }
                    }
                }
                require(visited == nodes.len())?;
            }
        }
        Ok(stats)
    }
}
