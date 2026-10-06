//! Bounded preformatted axis presentation. Parent chart style owns versioning.
use binprot::macros::BinProtWrite;
use std::mem::size_of;

pub const MAX_TICKS: usize = 64;
pub(crate) fn within(n: f64, lo: f64, hi: f64) -> bool {
    n.is_finite() && (lo..=hi).contains(&n)
}
pub(crate) fn color(n: i64) -> bool {
    (0..=0xffff_ffff).contains(&n)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum LabelSide {
    Auto,
    Before,
    After,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum LabelAlign {
    Auto,
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum TickPosition {
    Value(f64),
    Category(i64),
    Fraction(f64),
}
impl TickPosition {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Value(n) => within(n, -1e100, 1e100),
            Self::Category(n) => n > 0,
            Self::Fraction(n) => within(n, 0., 1.),
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Tick {
    pub position: TickPosition,
    pub text: String,
    pub color: Option<i64>,
    pub font_size: Option<f64>,
    pub align: LabelAlign,
}
impl Tick {
    pub fn is_valid(&self) -> bool {
        self.position.is_valid()
            && self.text.len() <= 256
            && !self.text.bytes().any(|b| b < 32 || b == 127)
            && self.color.is_none_or(color)
            && self.font_size.is_none_or(|n| within(n, 8., 32.))
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Axis {
    pub line: bool,
    pub labels: bool,
    pub position: Option<f64>,
    pub ticks: Option<Vec<Tick>>,
    pub tick_count: Option<i64>,
    pub label_side: LabelSide,
    pub label_align: LabelAlign,
    pub label_gap: Option<f64>,
    pub label_width: Option<f64>,
    pub font_size: f64,
    pub line_width: f64,
    pub line_color: Option<i64>,
    pub label_color: Option<i64>,
}
impl Axis {
    pub fn is_valid(&self) -> bool {
        self.position.is_none_or(|n| within(n, 0., 1.))
            && self
                .ticks
                .as_ref()
                .is_none_or(|ts| ts.len() <= MAX_TICKS && ts.iter().all(Tick::is_valid))
            && self.tick_count.is_none_or(|n| (2..=64).contains(&n))
            && self.label_gap.is_none_or(|n| within(n, 0., 64.))
            && self.label_width.is_none_or(|n| within(n, 8., 256.))
            && within(self.font_size, 8., 32.)
            && within(self.line_width, 0.5, 8.)
            && self.line_color.is_none_or(color)
            && self.label_color.is_none_or(color)
    }
    pub fn heap_bytes(&self) -> usize {
        self.ticks.as_ref().map_or(0, |ts| {
            ts.capacity() * size_of::<Tick>() + ts.iter().map(|t| t.text.capacity()).sum::<usize>()
        })
    }
}
impl Default for Axis {
    fn default() -> Self {
        Self {
            line: true,
            labels: true,
            position: None,
            ticks: None,
            tick_count: None,
            label_side: LabelSide::Auto,
            label_align: LabelAlign::Auto,
            label_gap: None,
            label_width: None,
            font_size: 11.,
            line_width: 1.,
            line_color: None,
            label_color: None,
        }
    }
}
