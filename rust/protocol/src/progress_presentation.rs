//! Extended presentation configuration, independent of native rendering.
use crate::{animation::Easing, progress::ProgressConfig};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Shape {
    Linear,
    Circle,
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Transition {
    Immediate,
    Tween { duration_ms: i64, easing: Easing },
}
impl Transition {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Immediate => true,
            Self::Tween {
                duration_ms,
                easing,
            } => (1..=60_000).contains(&duration_ms) && easing.is_valid(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub progress: ProgressConfig,
    pub shape: Shape,
    pub transition: Transition,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.progress.is_valid() && self.transition.is_valid()
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.progress.label.capacity()
    }
}
