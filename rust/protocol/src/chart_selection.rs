//! Fixed-size semantic targets; source spans name one immutable publication.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Span {
    pub start_index: i64,
    pub length: i64,
    pub first: i64,
    pub last: i64,
}
impl Span {
    pub fn is_valid(self) -> bool {
        (0..100_000).contains(&self.start_index)
            && self.length > 0
            && self.length <= 100_000 - self.start_index
            && self.first > 0
            && self.last > 0
            && (self.length == 1) == (self.first == self.last)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Aggregation {
    Exact,
    Sum,
    Mean,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Selection {
    Cartesian {
        series: i64,
        span: Span,
        aggregation: Aggregation,
    },
    Slice(i64),
    Radar {
        series: i64,
        axis: i64,
    },
    Candlestick {
        span: Span,
        aggregated: bool,
    },
    Node(i64),
    Edge(i64),
}
impl Selection {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Cartesian {
                series,
                span,
                aggregation,
            } => {
                series > 0
                    && span.is_valid()
                    && (aggregation != Aggregation::Exact || span.length == 1)
            }
            Self::Slice(id) | Self::Node(id) | Self::Edge(id) => id > 0,
            Self::Radar { series, axis } => series > 0 && axis > 0,
            Self::Candlestick { span, aggregated } => {
                span.is_valid() && (aggregated || span.length == 1)
            }
        }
    }
}
