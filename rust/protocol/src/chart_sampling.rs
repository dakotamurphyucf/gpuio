//! Explicit family-specific reduction; original datasets remain immutable.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Line {
    Exact,
    Envelope(i64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Bar {
    Exact,
    Sum(i64),
    Mean(i64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Candlestick {
    Exact,
    Ohlc(i64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Policy {
    pub version: i64,
    pub line: Line,
    pub bars: Bar,
    pub candles: Candlestick,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            version: 1,
            line: Line::Envelope(1024),
            bars: Bar::Exact,
            candles: Candlestick::Exact,
        }
    }
}
impl Policy {
    pub fn is_valid(self) -> bool {
        let valid = |n| (1..=8192).contains(&n);
        self.version == 1
            && match self.line {
                Line::Exact => true,
                Line::Envelope(n) => valid(n),
            }
            && match self.bars {
                Bar::Exact => true,
                Bar::Sum(n) | Bar::Mean(n) => valid(n),
            }
            && match self.candles {
                Candlestick::Exact => true,
                Candlestick::Ohlc(n) => valid(n),
            }
    }
}
