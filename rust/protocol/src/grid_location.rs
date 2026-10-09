//! Bounded, atomic native grid placement. Endpoint order is part of the wire ABI.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Edge {
    Auto,
    Line(i64),
    Span(i64),
}

impl Edge {
    pub fn valid(self) -> bool {
        match self {
            Self::Auto => true,
            Self::Line(value) => value != 0 && (-1025..=1025).contains(&value),
            Self::Span(value) => (1..=1024).contains(&value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Axis {
    pub start: Edge,
    pub end: Edge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Location {
    pub column: Axis,
    pub row: Axis,
}

impl Location {
    pub fn valid(self) -> bool {
        [
            self.column.start,
            self.column.end,
            self.row.start,
            self.row.end,
        ]
        .into_iter()
        .all(Edge::valid)
    }
}
