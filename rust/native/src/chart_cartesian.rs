//! Borrowed numeric/categorical access for preparation. Category ranks are
//! internal coordinates only; identity and labels stay in the original dataset.
use gpuio_protocol::chart_data as data;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Line,
    Area,
    Bar,
}
#[derive(Clone, Copy)]
pub(crate) enum Layers<'a> {
    Numeric(&'a [data::Layer]),
    Categorical(&'a [data::CategoricalLayer]),
}
impl<'a> Layers<'a> {
    pub fn of(data: &'a data::Data) -> Option<Self> {
        match &data.contents {
            data::Contents::Cartesian(l) => Some(Self::Numeric(l)),
            data::Contents::Categorical(_, l) => Some(Self::Categorical(l)),
            _ => None,
        }
    }
    pub fn len(self) -> usize {
        match self {
            Self::Numeric(l) => l.len(),
            Self::Categorical(l) => l.len(),
        }
    }
    pub fn get(self, index: usize) -> Option<Layer<'a>> {
        Some(match self {
            Self::Numeric(l) => {
                let l = l.get(index)?;
                let s = l.series();
                Layer {
                    kind: match l {
                        data::Layer::Line(_) => Kind::Line,
                        data::Layer::Area(_) => Kind::Area,
                        data::Layer::Bar(_) => Kind::Bar,
                    },
                    id: s.id,
                    name: &s.name,
                    points: Points::Numeric(&s.points),
                }
            }
            Self::Categorical(l) => {
                let l = l.get(index)?;
                let s = l.series();
                Layer {
                    kind: match l {
                        data::CategoricalLayer::Line(_) => Kind::Line,
                        data::CategoricalLayer::Area(_) => Kind::Area,
                        data::CategoricalLayer::Bar(_) => Kind::Bar,
                    },
                    id: s.id,
                    name: &s.name,
                    points: Points::Categorical(&s.points),
                }
            }
        })
    }
    pub fn iter(self) -> impl Iterator<Item = Layer<'a>> + Clone {
        (0..self.len()).map(move |i| self.get(i).expect("bounded layer index"))
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Layer<'a> {
    pub kind: Kind,
    pub id: i64,
    pub name: &'a str,
    pub points: Points<'a>,
}
#[derive(Clone, Copy)]
pub(crate) enum Points<'a> {
    Numeric(&'a [data::Point]),
    Categorical(&'a [data::CategoricalPoint]),
}
#[derive(Clone, Copy)]
pub(crate) struct Point<'a> {
    pub id: i64,
    pub x: f64,
    pub y: Option<f64>,
    pub label: &'a str,
}
impl<'a> Points<'a> {
    pub fn len(self) -> usize {
        match self {
            Self::Numeric(p) => p.len(),
            Self::Categorical(p) => p.len(),
        }
    }
    pub fn get(self, index: usize) -> Option<Point<'a>> {
        Some(match self {
            Self::Numeric(p) => {
                let p = p.get(index)?;
                Point {
                    id: p.id,
                    x: p.x,
                    y: p.y,
                    label: &p.label,
                }
            }
            Self::Categorical(p) => {
                let p = p.get(index)?;
                Point {
                    id: p.id,
                    x: index as f64,
                    y: p.value,
                    label: &p.label,
                }
            }
        })
    }
    pub fn first(self) -> Option<Point<'a>> {
        self.get(0)
    }
    pub fn last(self) -> Option<Point<'a>> {
        self.get(self.len().checked_sub(1)?)
    }
    pub fn iter(self) -> impl Iterator<Item = Point<'a>> + Clone {
        (0..self.len()).map(move |i| self.get(i).expect("bounded point index"))
    }
}

/// Uniform domain geometry; supports fractional ranks for explicit aggregates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Projection {
    start: f64,
    step: f64,
    pub band_width: f64,
}
impl Projection {
    pub fn new(
        count: usize,
        extent: f64,
        layout: gpuio_protocol::chart_options::CategoryLayout,
        bars: bool,
    ) -> Self {
        use gpuio_protocol::chart_options::CategoryLayout;
        let layout = match layout {
            CategoryLayout::Auto if bars => CategoryLayout::Band {
                inner: 0.2,
                outer: 0.1,
            },
            CategoryLayout::Auto => CategoryLayout::Point(0.),
            other => other,
        };
        let n = count as f64;
        let (step, width) = match layout {
            CategoryLayout::Point(padding) => {
                let padding = if bars { padding.max(0.5) } else { padding };
                let step = extent / (n - 1. + 2. * padding).max(1.);
                (step, step)
            }
            CategoryLayout::Band { inner, outer } => {
                let step = extent / (n - inner + 2. * outer).max(1.);
                (step, step * (1. - inner))
            }
            CategoryLayout::Auto => unreachable!(),
        };
        Self {
            start: (extent - step * (n - 1.).max(0.)) / 2.,
            step,
            band_width: width,
        }
    }
    pub fn center(self, rank: f64) -> f64 {
        self.start + rank * self.step
    }
    pub fn sampling_domain(self, extent: f64) -> (f64, f64) {
        (-self.start / self.step, (extent - self.start) / self.step)
    }
    pub fn interval_width(self, first: usize, last: usize) -> f64 {
        (last - first) as f64 * self.step + self.band_width
    }
}

#[cfg(test)]
mod tests;
