//! Pure, bounded source reduction performed before geometry preparation. No
//! drawing, text shaping, callbacks or source mutation occurs in this module.
use crate::chart_cartesian::{Kind, Layers, Points, Projection};
use gpuio_protocol::{chart_data as data, chart_sampling as policy};
use std::{
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidData,
    InvalidPolicy,
    InvalidWidth,
    Cancelled,
}

/// Half-open contiguous source range, interpreted against the exact immutable
/// dataset revision held by the eventual render plan. It is not an ID interval:
/// stable IDs need not be ordered or consecutive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    start: usize,
    end: usize,
}
impl SourceSpan {
    pub fn start(self) -> usize {
        self.start
    }
    pub fn end(self) -> usize {
        self.end
    }
    pub fn len(self) -> usize {
        self.end - self.start
    }
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
    fn singleton(index: usize) -> Self {
        Self {
            start: index,
            end: index + 1,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinePoint {
    pub source: usize,
    pub starts_run: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    pub source: SourceSpan,
    pub x: f64,
    pub value: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candle {
    pub source: SourceSpan,
    pub x: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Series {
    Line(Vec<LinePoint>),
    Area(Vec<LinePoint>),
    Bar(Vec<Bar>),
}
impl Series {
    pub fn len(&self) -> usize {
        match self {
            Self::Line(points) | Self::Area(points) => points.len(),
            Self::Bar(bars) => bars.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn bytes(&self) -> usize {
        match self {
            Self::Line(points) | Self::Area(points) => points.capacity() * size_of::<LinePoint>(),
            Self::Bar(bars) => bars.capacity() * size_of::<Bar>(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Contents {
    Cartesian(Vec<Series>),
    Candlestick(Vec<Candle>),
    Unchanged,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Reduction {
    pub contents: Contents,
    pub source_values: usize,
    pub rendered_values: usize,
}
impl Reduction {
    pub fn retained_bytes(&self) -> usize {
        512 + match &self.contents {
            Contents::Cartesian(series) => {
                series.capacity() * size_of::<Series>()
                    + series.iter().map(Series::bytes).sum::<usize>()
            }
            Contents::Candlestick(candles) => candles.capacity() * size_of::<Candle>(),
            Contents::Unchanged => 0,
        }
    }
}
#[derive(Clone, Copy)]
struct Domain {
    min: f64,
    max: f64,
}
impl Domain {
    fn bucket(self, x: f64, count: usize) -> usize {
        if self.min == self.max {
            return 0;
        }
        (((x - self.min) / (self.max - self.min) * count as f64).floor() as usize).min(count - 1)
    }
}
fn cancelled(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn checkpoint(index: usize, cancel: &AtomicBool) -> Result<(), Error> {
    if index & 255 == 0 {
        cancelled(cancel)
    } else {
        Ok(())
    }
}
fn bucket_count(maximum: i64, width: f64) -> usize {
    (maximum as usize).min(width.ceil() as usize)
}
fn midpoint(first: f64, last: f64) -> f64 {
    first + (last - first) * 0.5
}

struct Envelope {
    first: usize,
    last: usize,
    low: usize,
    high: usize,
    bucket: usize,
}
fn flush_line(envelope: &mut Option<Envelope>, output: &mut Vec<LinePoint>, starts_run: &mut bool) {
    if let Some(e) = envelope.take() {
        let mut indices = [e.first, e.low, e.high, e.last];
        indices.sort_unstable();
        let mut previous = None;
        for source in indices {
            if previous != Some(source) {
                output.push(LinePoint {
                    source,
                    starts_run: *starts_run,
                });
                *starts_run = false;
                previous = Some(source);
            }
        }
    }
}
fn line(
    points: Points<'_>,
    policy: policy::Line,
    domain: Domain,
    width: f64,
    cancel: &AtomicBool,
) -> Result<Vec<LinePoint>, Error> {
    let buckets = match policy {
        policy::Line::Exact => None,
        policy::Line::Envelope(max) => Some(bucket_count(max, width)),
    };
    let mut output = Vec::with_capacity(points.len().min(buckets.map_or(points.len(), |n| n * 4)));
    let mut envelope: Option<Envelope> = None;
    let mut starts_run = true;
    for (index, point) in points.iter().enumerate() {
        checkpoint(index, cancel)?;
        let Some(y) = point.y else {
            flush_line(&mut envelope, &mut output, &mut starts_run);
            starts_run = true;
            continue;
        };
        let Some(buckets) = buckets else {
            output.push(LinePoint {
                source: index,
                starts_run,
            });
            starts_run = false;
            continue;
        };
        let bucket = domain.bucket(point.x, buckets);
        if envelope.as_ref().is_some_and(|e| e.bucket != bucket) {
            flush_line(&mut envelope, &mut output, &mut starts_run);
        }
        if let Some(e) = &mut envelope {
            e.last = index;
            if y < points
                .get(e.low)
                .unwrap()
                .y
                .expect("defined envelope point")
            {
                e.low = index;
            }
            if y > points
                .get(e.high)
                .unwrap()
                .y
                .expect("defined envelope point")
            {
                e.high = index;
            }
        } else {
            envelope = Some(Envelope {
                first: index,
                last: index,
                low: index,
                high: index,
                bucket,
            });
        }
    }
    flush_line(&mut envelope, &mut output, &mut starts_run);
    Ok(output)
}

/// Neumaier summation keeps small signed contributions across large opposing
/// values. Bounded source magnitudes/counts keep sum and correction finite.
#[derive(Default)]
struct Sum {
    total: f64,
    correction: f64,
}
impl Sum {
    fn add(&mut self, value: f64) {
        let next = self.total + value;
        self.correction += if self.total.abs() >= value.abs() {
            (self.total - next) + value
        } else {
            (value - next) + self.total
        };
        self.total = next;
    }
    fn value(&self) -> f64 {
        self.total + self.correction
    }
}
fn bars(
    points: Points<'_>,
    policy: policy::Bar,
    domain: Domain,
    width: f64,
    cancel: &AtomicBool,
) -> Result<Vec<Bar>, Error> {
    let (buckets, mean) = match policy {
        policy::Bar::Exact => (None, false),
        policy::Bar::Sum(max) => (Some(bucket_count(max, width)), false),
        policy::Bar::Mean(max) => (Some(bucket_count(max, width)), true),
    };
    let mut output = Vec::with_capacity(points.len().min(buckets.unwrap_or(points.len())));
    let mut start = 0;
    while start < points.len() {
        checkpoint(start, cancel)?;
        let mut end = start + 1;
        let mut sum = Sum::default();
        sum.add(points.get(start).unwrap().y.unwrap_or(0.));
        let mut present = usize::from(points.get(start).unwrap().y.is_some());
        if let Some(buckets) = buckets {
            let bucket = domain.bucket(points.get(start).unwrap().x, buckets);
            while end < points.len() && domain.bucket(points.get(end).unwrap().x, buckets) == bucket
            {
                checkpoint(end, cancel)?;
                sum.add(points.get(end).unwrap().y.unwrap_or(0.));
                present += usize::from(points.get(end).unwrap().y.is_some());
                end += 1;
            }
        }
        let value = sum.value();
        if present == 0 {
            start = end;
            continue;
        }
        output.push(Bar {
            source: SourceSpan { start, end },
            x: midpoint(points.get(start).unwrap().x, points.get(end - 1).unwrap().x),
            value: if mean { value / present as f64 } else { value },
        });
        start = end;
    }
    Ok(output)
}
fn candles(
    points: &[data::Candle],
    policy: policy::Candlestick,
    domain: Domain,
    width: f64,
    cancel: &AtomicBool,
) -> Result<Vec<Candle>, Error> {
    let buckets = match policy {
        policy::Candlestick::Exact => None,
        policy::Candlestick::Ohlc(max) => Some(bucket_count(max, width)),
    };
    let mut output = Vec::with_capacity(points.len().min(buckets.unwrap_or(points.len())));
    let mut start = 0;
    while start < points.len() {
        checkpoint(start, cancel)?;
        let first = &points[start];
        let mut candle = Candle {
            source: SourceSpan::singleton(start),
            x: first.x,
            open: first.open_,
            high: first.high,
            low: first.low,
            close: first.close,
        };
        let mut end = start + 1;
        if let Some(buckets) = buckets {
            let bucket = domain.bucket(first.x, buckets);
            while end < points.len() && domain.bucket(points[end].x, buckets) == bucket {
                checkpoint(end, cancel)?;
                candle.high = candle.high.max(points[end].high);
                candle.low = candle.low.min(points[end].low);
                candle.close = points[end].close;
                end += 1;
            }
        }
        candle.source.end = end;
        candle.x = midpoint(first.x, points[end - 1].x);
        output.push(candle);
        start = end;
    }
    Ok(output)
}

/// Width is in logical pixels, finite and within (0,32768]. Source validation is
/// repeated at this pure boundary; native workers can cache the completed result
/// by source revision, policy and plot width. At most the source count is retained.
pub fn prepare(
    data: &data::Data,
    policy: policy::Policy,
    width: f64,
    cancel: &AtomicBool,
) -> Result<Reduction, Error> {
    prepare_with_options(
        data,
        policy,
        width,
        &gpuio_protocol::chart_options::Options::default(),
        cancel,
    )
}

pub(crate) fn prepare_with_options(
    data: &data::Data,
    policy: policy::Policy,
    width: f64,
    options: &gpuio_protocol::chart_options::Options,
    cancel: &AtomicBool,
) -> Result<Reduction, Error> {
    cancelled(cancel)?;
    if !policy.is_valid() || !options.is_valid() {
        return Err(Error::InvalidPolicy);
    }
    if !width.is_finite() || width <= 0. || width > 32768. {
        return Err(Error::InvalidWidth);
    }
    let stats = data.validate().map_err(|_| Error::InvalidData)?;
    let contents =
        match &data.contents {
            data::Contents::Cartesian(_) | data::Contents::Categorical(..) => {
                let layers = Layers::of(data).expect("Cartesian source");
                let domain = match &data.contents {
                    data::Contents::Categorical(categories, _) => {
                        let projection = Projection::new(
                            categories.len(),
                            width,
                            options.cartesian.category_layout,
                            layers.iter().any(|l| l.kind == Kind::Bar),
                        );
                        let (min, max) = projection.sampling_domain(width);
                        Domain { min, max }
                    }
                    _ => layers
                        .iter()
                        .filter_map(|l| {
                            Some(Domain {
                                min: l.points.first()?.x,
                                max: l.points.last()?.x,
                            })
                        })
                        .reduce(|a, b| Domain {
                            min: a.min.min(b.min),
                            max: a.max.max(b.max),
                        })
                        .unwrap_or(Domain { min: 0., max: 1. }),
                };
                let mut output = Vec::with_capacity(layers.len());
                for layer in layers.iter() {
                    output.push(match layer.kind {
                        Kind::Line => {
                            Series::Line(line(layer.points, policy.line, domain, width, cancel)?)
                        }
                        Kind::Area => {
                            Series::Area(line(layer.points, policy.line, domain, width, cancel)?)
                        }
                        Kind::Bar => {
                            Series::Bar(bars(layer.points, policy.bars, domain, width, cancel)?)
                        }
                    });
                }
                Contents::Cartesian(output)
            }
            data::Contents::Candlestick(points) => {
                let domain = points.first().zip(points.last()).map_or(
                    Domain { min: 0., max: 1. },
                    |(a, b)| Domain { min: a.x, max: b.x },
                );
                Contents::Candlestick(candles(points, policy.candles, domain, width, cancel)?)
            }
            data::Contents::Pie(_) | data::Contents::Radar(_, _) | data::Contents::Sankey(_, _) => {
                Contents::Unchanged
            }
        };
    cancelled(cancel)?;
    let rendered_values = match &contents {
        Contents::Cartesian(series) => series.iter().map(Series::len).sum(),
        Contents::Candlestick(candles) => candles.len(),
        Contents::Unchanged => stats.values,
    };
    Ok(Reduction {
        contents,
        source_values: stats.values,
        rendered_values,
    })
}

#[cfg(test)]
mod tests;
