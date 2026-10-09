//! Bounded declarative motion data. Native frames never call an OCaml easing function.
use binprot::macros::BinProtWrite;
use std::sync::Arc;

pub const MAX_LINEAR_STOPS: usize = 256;

pub const PROPERTY_COUNT: usize = 12;
// Absolute opacity and its factor are mutually exclusive.
pub const MAX_TARGETS: usize = PROPERTY_COUNT - 1;
pub const MAX_TIME_MS: i64 = 86_400_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
#[repr(usize)]
pub enum Property {
    Width,
    Height,
    Top,
    Right,
    Bottom,
    Left,
    Opacity,
    TopLeftRadius,
    TopRightRadius,
    BottomLeftRadius,
    BottomRightRadius,
    OpacityFactor,
}
impl Property {
    pub fn clamp(self, value: f64) -> f64 {
        match self {
            Self::Opacity | Self::OpacityFactor => value.clamp(0., 1.),
            Self::Top | Self::Right | Self::Bottom | Self::Left => {
                value.clamp(-1_000_000., 1_000_000.)
            }
            _ => value.clamp(0., 1_000_000.),
        }
    }
    pub fn accepts(self, value: f64) -> bool {
        value.is_finite() && self.clamp(value) == value
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Target {
    pub property: Property,
    pub value: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum StepPosition {
    JumpStart,
    JumpEnd,
    JumpNone,
    JumpBoth,
}

/// Immutable resolved stops. Cloning a retained easing never copies its points.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearStops(Arc<[(f64, f64)]>);
impl LinearStops {
    pub fn new(stops: Vec<(f64, f64)>) -> Option<Self> {
        ((2..=MAX_LINEAR_STOPS).contains(&stops.len())
            && stops
                .iter()
                .all(|(x, y)| x.is_finite() && (0. ..=1.).contains(x) && y.is_finite())
            && stops.windows(2).all(|pair| pair[0].0 <= pair[1].0))
        .then(|| Self(stops.into()))
    }
    fn sample(&self, progress: f64) -> f64 {
        let progress = progress.clamp(0., 1.);
        let upper = self.0.partition_point(|(input, _)| *input <= progress);
        if upper == 0 {
            return self.0[0].1;
        }
        if upper == self.0.len() {
            return self.0[upper - 1].1;
        }
        let (x0, y0) = self.0[upper - 1];
        let (x1, y1) = self.0[upper];
        let fraction = (progress - x0) / (x1 - x0);
        // Weighted interpolation avoids overflowing y1-y0 for finite extremes.
        y0 * (1. - fraction) + y1 * fraction
    }
    pub fn heap_bytes(&self) -> usize {
        std::mem::size_of_val(self.0.as_ref()) + 2 * std::mem::size_of::<usize>()
    }
}
impl binprot::BinProtWrite for LinearStops {
    fn binprot_write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.as_ref().binprot_write(writer)
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Easing {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f64, f64, f64, f64),
    EaseInOutCubic,
    Steps(i64, StepPosition),
    LinearStops(LinearStops),
}
impl Easing {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Steps(count, position) => {
                (1..=i64::from(u32::MAX)).contains(count)
                    && (*position != StepPosition::JumpNone || *count >= 2)
            }
            Self::CubicBezier(x1, y1, x2, y2) => {
                [*x1, *x2]
                    .iter()
                    .all(|x| x.is_finite() && (0. ..=1.).contains(x))
                    && [y1, y2].iter().all(|y| y.is_finite())
            }
            _ => true,
        }
    }
    pub fn heap_bytes(&self) -> usize {
        match self {
            Self::LinearStops(stops) => stops.heap_bytes(),
            _ => 0,
        }
    }
    pub fn sample(&self, progress: f64) -> f64 {
        if let Self::LinearStops(stops) = self {
            return stops.sample(progress);
        }
        // Stepped curves may jump at zero: do not apply the continuous-curve
        // endpoint shortcut before evaluating their position policy.
        if let Self::Steps(count, position) = self {
            let count = *count as f64;
            let (jumps, offset) = match position {
                StepPosition::JumpStart => (count, 1.),
                StepPosition::JumpEnd => (count, 0.),
                StepPosition::JumpNone => (count - 1., 0.),
                StepPosition::JumpBoth => (count + 1., 1.),
            };
            return ((progress.clamp(0., 1.) * count).floor() + offset).clamp(0., jumps) / jumps;
        }
        if progress <= 0. {
            return 0.;
        }
        if progress >= 1. {
            return 1.;
        }
        let (x1, y1, x2, y2) = match self {
            Self::Steps(..) | Self::LinearStops(..) => {
                unreachable!("discontinuous easing evaluated above")
            }
            Self::Linear => return progress,
            Self::EaseInOutCubic => {
                return if progress <= 0.5 {
                    4. * progress.powi(3)
                } else {
                    1. - 4. * (1. - progress).powi(3)
                };
            }
            Self::Ease => (0.25, 0.1, 0.25, 1.),
            Self::EaseIn => (0.42, 0., 1., 1.),
            Self::EaseOut => (0., 0., 0.58, 1.),
            Self::EaseInOut => (0.42, 0., 0.58, 1.),
            Self::CubicBezier(x1, y1, x2, y2) => (*x1, *y1, *x2, *y2),
        };
        let curve = |t: f64, a: f64, b: f64| {
            3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t.powi(2) * b + t.powi(3)
        };
        // Invert x, including zero derivatives at either end. Bisection avoids
        // Newton division failures for valid curves such as x(t) = t^3.
        let (mut low, mut high) = (0., 1.);
        for _ in 0..40 {
            let t = (low + high) / 2.;
            if curve(t, x1, x2) < progress {
                low = t;
            } else {
                high = t;
            }
        }
        curve((low + high) / 2., y1, y2)
    }
}
/// Validated finite-duration physical motion; no callback crosses the bridge.
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Spring {
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
    pub epsilon: f64,
    pub max_duration_ms: i64,
}
impl Spring {
    pub fn is_valid(self) -> bool {
        let bounded =
            |value: f64, low: f64, high: f64| value.is_finite() && (low..=high).contains(&value);
        bounded(self.stiffness, 0.01, 10_000.)
            && bounded(self.damping, 0., 1_000.)
            && bounded(self.mass, 0.01, 1_000.)
            && bounded(self.epsilon, 0.0001, 1.)
            && (1..=60_000).contains(&self.max_duration_ms)
    }
}
/// Unsigned 64-bit count encoded as two independently validated 32-bit limbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct IterationCount {
    pub high: i64,
    pub low: i64,
}
impl IterationCount {
    pub fn new(value: u64) -> Self {
        Self {
            high: (value >> 32) as i64,
            low: (value & 0xffff_ffff) as i64,
        }
    }
    pub fn is_valid(self) -> bool {
        (0..=u32::MAX as i64).contains(&self.high) && (0..=u32::MAX as i64).contains(&self.low)
    }
    pub fn value(self) -> u64 {
        ((self.high as u64) << 32) | self.low as u64
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Direction {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}
impl Direction {
    pub fn reverses(self, iteration: u128) -> bool {
        match self {
            Self::Normal => false,
            Self::Reverse => true,
            Self::Alternate => iteration % 2 == 1,
            Self::AlternateReverse => iteration.is_multiple_of(2),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Repeat {
    Once,
    Loop,
    Alternate,
    Finite(IterationCount, Direction),
    Infinite(Direction),
}
impl Repeat {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Finite(count, _) => count.is_valid(),
            _ => true,
        }
    }
    pub fn is_infinite(self) -> bool {
        matches!(self, Self::Loop | Self::Alternate | Self::Infinite(_))
    }
    pub fn is_explicit(self) -> bool {
        matches!(self, Self::Finite(..) | Self::Infinite(_))
    }
    pub fn policy(self) -> Option<(Option<u64>, Direction)> {
        match self {
            Self::Finite(count, direction) => Some((Some(count.value()), direction)),
            Self::Infinite(direction) => Some((None, direction)),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Preference {
    System,
    Reduce,
    Full,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum CancelReason {
    Replaced,
    Removed,
    WindowClosed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Outcome {
    Finished,
    Cancelled(CancelReason),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Endpoint {
    pub generation: i64,
    pub outcome: Outcome,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub generation: i64,
    pub targets: Vec<Target>,
    pub initial: Option<Vec<Target>>,
    pub duration_ms: i64,
    pub delay_ms: i64,
    pub easing: Easing,
    pub repeat: Repeat,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.generation > 0
            && valid_targets(&self.targets)
            && self.initial.as_ref().is_none_or(|initial| {
                valid_targets(initial)
                    && initial.len() == self.targets.len()
                    && initial
                        .iter()
                        .zip(&self.targets)
                        .all(|(a, b)| a.property == b.property)
            })
            && (0..=MAX_TIME_MS).contains(&self.duration_ms)
            && (-MAX_TIME_MS..=MAX_TIME_MS).contains(&self.delay_ms)
            && self.easing.is_valid()
            && self.repeat.is_valid()
            && (self.repeat == Repeat::Once || self.initial.is_some())
            && (!self.repeat.is_infinite() || self.duration_ms > 0)
    }
}

/// Canonically ordered, bounded targets. Absolute and multiplicative opacity
/// are alternatives so a target never depends on evaluation order.
pub(crate) fn valid_targets(targets: &[Target]) -> bool {
    !targets.is_empty()
        && targets.len() <= MAX_TARGETS
        && targets.iter().all(|t| t.property.accepts(t.value))
        && targets.windows(2).all(|w| w[0].property < w[1].property)
        && !(targets.iter().any(|t| t.property == Property::Opacity)
            && targets
                .iter()
                .any(|t| t.property == Property::OpacityFactor))
}
