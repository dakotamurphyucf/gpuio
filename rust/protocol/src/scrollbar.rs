//! Per-viewport scrollbar presentation. No handles, offsets or callbacks.
use crate::v1::{Color, Fill};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 8192;
pub const MAX_LABEL_BYTES: usize = 1024;
pub const MAX_DIMENSION: f64 = 16384.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, BinProtWrite)]
pub enum Axis {
    Horizontal,
    Vertical,
    #[default]
    Both,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, BinProtWrite)]
pub enum Mode {
    #[default]
    Scrolling,
    Hover,
    Always,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, BinProtWrite)]
pub enum Entrance {
    #[default]
    Fade,
    SlideAndFade,
}
#[derive(Clone, Copy, Debug, PartialEq, Default, BinProtWrite)]
pub struct Track {
    pub background: Option<i64>,
    pub border: Option<i64>,
    pub width: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Default, BinProtWrite)]
pub struct Thumb {
    pub background: Option<Fill>,
    pub width: Option<f64>,
    pub inset: Option<f64>,
    pub radius: Option<f64>,
    pub min_length: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Default, BinProtWrite)]
pub struct Appearance {
    pub track: Track,
    pub track_hover: Track,
    pub track_pressed: Track,
    pub thumb: Thumb,
    pub thumb_hover: Thumb,
    pub thumb_pressed: Thumb,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Motion {
    pub idle_ms: i64,
    pub enter_ms: i64,
    pub exit_ms: i64,
    pub expand_ms: i64,
    pub entrance: Entrance,
    pub thumb_hover_entrance: Entrance,
}
impl Default for Motion {
    fn default() -> Self {
        Self {
            idle_ms: 2000,
            enter_ms: 0,
            exit_ms: 0,
            expand_ms: 0,
            entrance: Entrance::Fade,
            thumb_hover_entrance: Entrance::Fade,
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub axis: Axis,
    pub mode: Mode,
    pub appearance: Appearance,
    pub motion: Motion,
}

pub fn dimension(value: f64) -> bool {
    value.is_finite() && (0. ..=MAX_DIMENSION).contains(&value)
}
fn rgba(value: i64) -> bool {
    (0..=0xffff_ffff).contains(&value)
}
fn color(value: &Color) -> bool {
    matches!(value, Color::Rgba(value) if rgba(*value))
}
fn gradient(angle: f64, from: &Color, start: f64, to: &Color, stop: f64) -> bool {
    angle.is_finite()
        && (0. ..=360.).contains(&angle)
        && start.is_finite()
        && stop.is_finite()
        && (0. ..=stop).contains(&start)
        && stop <= 1.
        && color(from)
        && color(to)
}
fn fill(value: &Fill) -> bool {
    match value {
        Fill::Solid(value) => color(value),
        Fill::LinearGradient(angle, from, start, to, stop) => {
            gradient(*angle, from, *start, to, *stop)
        }
        Fill::LinearGradientIn(space, angle, from, start, to, stop) => {
            (0..=1).contains(space) && gradient(*angle, from, *start, to, *stop)
        }
    }
}
impl Track {
    pub fn is_valid(&self) -> bool {
        self.width.is_none_or(dimension)
            && self.background.is_none_or(rgba)
            && self.border.is_none_or(rgba)
    }
}
impl Thumb {
    pub fn is_valid(&self) -> bool {
        [self.width, self.inset, self.radius, self.min_length]
            .into_iter()
            .all(|value| value.is_none_or(dimension))
            && self.background.as_ref().is_none_or(fill)
    }
}
impl Motion {
    pub fn is_valid(self) -> bool {
        [self.idle_ms, self.enter_ms, self.exit_ms, self.expand_ms]
            .into_iter()
            .all(|value| (0..=60_000).contains(&value))
    }
}
impl Config {
    /// Conservative admission units for the description plus two native ranges,
    /// focus handles, timing owners and bounded paint/input state; not measured RSS.
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.label.len() + 8192
    }
    pub fn is_valid(&self) -> bool {
        let a = &self.appearance;
        !self
            .label
            .trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
            .is_empty()
            && self.label.len() <= MAX_LABEL_BYTES
            && !self.label.contains('\0')
            && [&a.track, &a.track_hover, &a.track_pressed]
                .into_iter()
                .all(Track::is_valid)
            && [&a.thumb, &a.thumb_hover, &a.thumb_pressed]
                .into_iter()
                .all(Thumb::is_valid)
            && self.motion.is_valid()
    }
}
