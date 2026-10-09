//! Bounded, resolved chart presentation. Attachment to chart style is forthcoming.
use crate::chart_axis::{color, within};
use crate::chart_options::Curve;
use binprot::macros::BinProtWrite;
use std::{collections::BTreeSet, mem::size_of};

pub const MAX_SERIES: usize = 128;
pub const MAX_DATA: usize = 1024;
pub const MAX_BYTES: usize = 192 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Brush {
    Solid(i64),
    Linear {
        oklab: bool,
        angle: f64,
        from: i64,
        start: f64,
        to: i64,
        stop: f64,
    },
    PatternSlash(i64, f64, f64),
    Checkerboard(i64, f64),
}
impl Brush {
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::Solid(c) => color(c),
            Self::PatternSlash(c, width, interval) => {
                color(c) && within(width, 0.5, 64.) && within(interval, 0.5, 64.)
            }
            Self::Checkerboard(c, size) => color(c) && within(size, 0.5, 64.),
            Self::Linear {
                oklab: _,
                angle,
                from,
                start,
                to,
                stop,
            } => {
                within(angle, 0., 360.)
                    && color(from)
                    && color(to)
                    && within(start, 0., 1.)
                    && within(stop, 0., 1.)
                    && start <= stop
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Corners {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_right: f64,
    pub bottom_left: f64,
}
impl Corners {
    pub fn is_valid(&self) -> bool {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
        .into_iter()
        .all(|n| within(n, 0., 32.))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum BarFill {
    Background(Brush),
    BaseToTip(i64, i64),
    Domain(i64, i64),
    Values(f64, i64, f64, i64),
}
impl BarFill {
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::Background(b) => b.is_valid(),
            Self::BaseToTip(a, b) | Self::Domain(a, b) => color(a) && color(b),
            Self::Values(lo, a, hi, b) => {
                within(lo, -1e100, 1e100)
                    && within(hi, -1e100, 1e100)
                    && lo < hi
                    && color(a)
                    && color(b)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Stroke {
    pub visible: bool,
    pub width: Option<f64>,
    pub brush: Brush,
}
impl Stroke {
    pub fn is_valid(&self) -> bool {
        self.width.is_none_or(|n| within(n, 0.5, 8.)) && self.brush.is_valid()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, BinProtWrite)]
pub struct Path {
    pub stroke: Option<Stroke>,
    pub fill: Option<Brush>,
    pub curve: Option<Curve>,
}
impl Path {
    pub fn is_valid(&self) -> bool {
        self.stroke.is_none_or(|s| s.is_valid()) && self.fill.is_none_or(|b| b.is_valid())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, BinProtWrite)]
pub struct Marker {
    pub visible: Option<bool>,
    pub radius: Option<f64>,
    pub fill: Option<i64>,
    pub stroke: Option<i64>,
    pub stroke_width: Option<f64>,
}
impl Marker {
    pub fn is_valid(&self) -> bool {
        self.radius.is_none_or(|n| within(n, 1., 24.))
            && self.fill.is_none_or(color)
            && self.stroke.is_none_or(color)
            && self.stroke_width.is_none_or(|n| within(n, 0., 8.))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, BinProtWrite)]
pub struct Bar {
    pub fill: Option<BarFill>,
    pub corners: Option<Corners>,
}
impl Bar {
    pub fn is_valid(&self) -> bool {
        self.fill.is_none_or(|f| f.is_valid()) && self.corners.is_none_or(|c| c.is_valid())
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Series {
    pub series: i64,
    pub path: Option<Path>,
    pub marker: Option<Marker>,
    pub bar: Option<Bar>,
    pub legend: Option<i64>,
    pub area_baseline: Option<f64>,
}
impl Series {
    pub fn is_valid(&self) -> bool {
        self.series > 0
            && self.path.is_none_or(|p| p.is_valid())
            && self.marker.is_none_or(|m| m.is_valid())
            && self.bar.is_none_or(|b| b.is_valid())
            && self.legend.is_none_or(color)
            && self.area_baseline.is_none_or(|n| within(n, -1e100, 1e100))
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Datum {
    pub series: i64,
    pub datum: i64,
    pub marker: Option<Marker>,
    pub bar: Option<Bar>,
}
impl Datum {
    pub fn is_valid(&self) -> bool {
        self.series > 0
            && self.datum > 0
            && self.marker.is_none_or(|m| m.is_valid())
            && self.bar.is_none_or(|b| b.is_valid())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub enum Aggregates {
    #[default]
    InheritSeries,
    Uniform,
}

#[derive(Clone, Debug, Default, PartialEq, BinProtWrite)]
pub struct Appearance {
    pub series: Vec<Series>,
    pub data: Vec<Datum>,
    pub aggregates: Aggregates,
}
impl Appearance {
    pub fn is_valid(&self) -> bool {
        self.series.len() <= MAX_SERIES
            && self.data.len() <= MAX_DATA
            && self.series.iter().all(Series::is_valid)
            && self.data.iter().all(Datum::is_valid)
            && self
                .series
                .iter()
                .map(|s| s.series)
                .collect::<BTreeSet<_>>()
                .len()
                == self.series.len()
            && self
                .data
                .iter()
                .map(|d| (d.series, d.datum))
                .collect::<BTreeSet<_>>()
                .len()
                == self.data.len()
    }
    pub fn heap_bytes(&self) -> usize {
        self.series.capacity() * size_of::<Series>() + self.data.capacity() * size_of::<Datum>()
    }
}
