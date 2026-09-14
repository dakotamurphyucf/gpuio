//! Bounded declarative motion data. Native frames never call an OCaml easing function.
use binprot::macros::BinProtWrite;

pub const PROPERTY_COUNT: usize = 11;
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
}
impl Property {
    pub fn clamp(self, value: f64) -> f64 {
        match self {
            Self::Opacity => value.clamp(0., 1.),
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
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Easing {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f64, f64, f64, f64),
}
impl Easing {
    pub fn is_valid(self) -> bool {
        match self {
            Self::CubicBezier(x1, y1, x2, y2) => {
                [x1, x2]
                    .iter()
                    .all(|x| x.is_finite() && (0. ..=1.).contains(x))
                    && [y1, y2].iter().all(|y| y.is_finite())
            }
            _ => true,
        }
    }
    pub fn sample(self, progress: f64) -> f64 {
        if progress <= 0. {
            return 0.;
        }
        if progress >= 1. {
            return 1.;
        }
        let (x1, y1, x2, y2) = match self {
            Self::Linear => return progress,
            Self::Ease => (0.25, 0.1, 0.25, 1.),
            Self::EaseIn => (0.42, 0., 1., 1.),
            Self::EaseOut => (0., 0., 0.58, 1.),
            Self::EaseInOut => (0.42, 0., 0.58, 1.),
            Self::CubicBezier(x1, y1, x2, y2) => (x1, y1, x2, y2),
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Repeat {
    Once,
    Loop,
    Alternate,
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
        let valid = |targets: &[Target]| {
            !targets.is_empty()
                && targets.len() <= PROPERTY_COUNT
                && targets.iter().all(|t| t.property.accepts(t.value))
                && targets.windows(2).all(|w| w[0].property < w[1].property)
        };
        self.generation > 0
            && valid(&self.targets)
            && self.initial.as_ref().is_none_or(|initial| {
                valid(initial)
                    && initial.len() == self.targets.len()
                    && initial
                        .iter()
                        .zip(&self.targets)
                        .all(|(a, b)| a.property == b.property)
            })
            && (0..=MAX_TIME_MS).contains(&self.duration_ms)
            && (0..=MAX_TIME_MS).contains(&self.delay_ms)
            && self.easing.is_valid()
            && (self.repeat == Repeat::Once || (self.initial.is_some() && self.duration_ms > 0))
    }
}
