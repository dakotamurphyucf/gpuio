//! Text-glyph shimmer configuration, distinct from rectangular loading placeholders.
//! Standalone codec foundation; not yet a negotiated mounted operation.
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Spread {
    Relative(f64),
    Pixels(f64),
}

impl Spread {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Relative(value) => value.is_finite() && (0.05..=1.).contains(&value),
            Self::Pixels(value) => value.is_finite() && (1. ..=1_000_000.).contains(&value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Repeat {
    Once,
    Loop,
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub duration_ms: i64,
    pub spread: Spread,
    pub direction: Direction,
    pub repeat: Repeat,
    pub animated: bool,
    pub highlight: Option<i64>,
}

impl Config {
    pub fn is_valid(&self) -> bool {
        (1..=60_000).contains(&self.duration_ms)
            && self.spread.is_valid()
            && self
                .highlight
                .is_none_or(|c| (0..=0xffff_ffff).contains(&c))
    }

    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}
