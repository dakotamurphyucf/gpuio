//! Bounded advanced motion configuration; legacy animation wire data stays intact.
use crate::animation::{
    self as legacy, Easing, MAX_TIME_MS, Repeat, Spring, Target, valid_targets,
};
use binprot::macros::BinProtWrite;

pub const MAX_STAGES: usize = 32;
pub const MAX_GROUP_BYTES: usize = 128;
pub const MAX_CONFIG_BYTES: usize = 16_384;
pub const MAX_GROUPS: usize = 128;
pub const MAX_MEMBERS: usize = 1024;
pub const TERMINAL_INDEX: i64 = MAX_STAGES as i64 + 1;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Timing {
    Tween(i64, Easing),
    Spring(Spring),
}
impl Timing {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Tween(duration, easing) => {
                (0..=MAX_TIME_MS).contains(duration) && easing.is_valid()
            }
            Self::Spring(parameters) => parameters.is_valid(),
        }
    }
    pub fn maximum_duration_ms(&self) -> i64 {
        match self {
            Self::Tween(duration, _) => *duration,
            Self::Spring(parameters) => parameters.max_duration_ms,
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Stage {
    pub targets: Vec<Target>,
    pub timing: Timing,
    pub delay_ms: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Clock {
    Independent,
    Application,
    Group(String),
}
impl Clock {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Group(name) => {
                !name.is_empty() && name.len() <= MAX_GROUP_BYTES && !name.contains('\0')
            }
            _ => true,
        }
    }
    pub fn is_shared(&self) -> bool {
        !matches!(self, Self::Independent)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Playback {
    Running,
    Paused,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Program {
    pub initial: Option<Vec<Target>>,
    pub stages: Vec<Stage>,
    pub delay_ms: i64,
    pub repeat: Repeat,
    pub clock: Clock,
}
fn same_properties(a: &[Target], b: &[Target]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.property == b.property)
}
impl Program {
    pub fn is_valid(&self) -> bool {
        let Some(first) = self.stages.first() else {
            return false;
        };
        if self.stages.len() > MAX_STAGES
            || !(-MAX_TIME_MS..=MAX_TIME_MS).contains(&self.delay_ms)
            || !self.clock.is_valid()
            || !self.stages.iter().all(|stage| {
                valid_targets(&stage.targets)
                    && same_properties(&first.targets, &stage.targets)
                    && stage.timing.is_valid()
                    && (0..=MAX_TIME_MS).contains(&stage.delay_ms)
            })
            || self.initial.as_ref().is_some_and(|initial| {
                !valid_targets(initial) || !same_properties(initial, &first.targets)
            })
            || ((self.stages.len() > 1 || self.repeat != Repeat::Once) && self.initial.is_none())
        {
            return false;
        }
        // Each validated summand is <= one day and there are at most 32 stages.
        let period: i64 = self
            .stages
            .iter()
            .map(|s| s.delay_ms + s.timing.maximum_duration_ms())
            .sum();
        binprot::BinProtSize::binprot_size(self) + 19 <= MAX_CONFIG_BYTES
            && period <= MAX_TIME_MS
            && (self.repeat == Repeat::Once || period > 0)
            && (!self.clock.is_shared()
                || (self.repeat != Repeat::Once
                    && self.delay_ms == 0
                    && self
                        .stages
                        .iter()
                        .all(|s| matches!(s.timing, Timing::Tween(..)))))
    }
    /// Common representation for the established API. Its delay is before the
    /// first cycle, whereas advanced per-stage delays are part of each cycle.
    pub fn from_legacy(config: &legacy::Config) -> Option<Self> {
        config.is_valid().then(|| Self {
            initial: config.initial.clone(),
            stages: vec![Stage {
                targets: config.targets.clone(),
                timing: Timing::Tween(config.duration_ms, config.easing.clone()),
                delay_ms: 0,
            }],
            delay_ms: config.delay_ms,
            repeat: config.repeat,
            clock: Clock::Independent,
        })
    }
    /// Heap capacity owned by this program, excluding its inline struct.
    pub fn heap_bytes(&self) -> usize {
        self.stages.capacity() * std::mem::size_of::<Stage>()
            + self
                .stages
                .iter()
                .map(|s| {
                    s.targets.capacity() * std::mem::size_of::<Target>()
                        + match &s.timing {
                            Timing::Tween(_, easing) => easing.heap_bytes(),
                            Timing::Spring(_) => 0,
                        }
                })
                .sum::<usize>()
            + self
                .initial
                .as_ref()
                .map_or(0, |v| v.capacity() * std::mem::size_of::<Target>())
            + match &self.clock {
                Clock::Group(name) => name.capacity(),
                _ => 0,
            }
    }
    /// Same boundaries and direction, independent of numeric ranges and easing.
    pub fn same_clock_schedule(&self, other: &Self) -> bool {
        self.repeat == other.repeat
            && self.delay_ms == other.delay_ms
            && self.stages.len() == other.stages.len()
            && self.stages.iter().zip(&other.stages).all(|(a, b)| {
                a.delay_ms == b.delay_ms
                    && match (&a.timing, &b.timing) {
                        (Timing::Tween(a, _), Timing::Tween(b, _)) => a == b,
                        _ => false,
                    }
            })
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    /// Configuration generation, not changed by native frames.
    pub generation: i64,
    pub program: Program,
    pub playback: Playback,
    pub restart: i64,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.generation > 0 && self.restart >= 0 && self.program.is_valid()
    }
    /// Playback-only snapshots update admission generation but preserve the run.
    pub fn same_run(&self, other: &Self) -> bool {
        self.restart == other.restart && self.program == other.program
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.program.heap_bytes()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum StageResult {
    Played,
    ReducedMotion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum CancelReason {
    Replaced,
    Removed,
    WindowClosed,
    Requested,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Observation {
    StageCompleted(i64, StageResult),
    Finished,
    Cancelled(CancelReason),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Signal {
    pub generation: i64,
    pub index: i64,
    pub observation: Observation,
}
impl Signal {
    pub fn valid_batch(signals: &[Self]) -> bool {
        !signals.is_empty()
            && signals.len() <= MAX_STAGES + 1
            && signals
                .iter()
                .all(|s| s.is_valid() && s.generation == signals[0].generation)
            && signals.windows(2).all(|w| w[0].index < w[1].index)
    }
    pub fn is_valid(self) -> bool {
        self.generation > 0
            && match self.observation {
                Observation::StageCompleted(stage, _) => {
                    (0..MAX_STAGES as i64).contains(&stage) && self.index == stage + 1
                }
                Observation::Finished | Observation::Cancelled(_) => self.index == TERMINAL_INDEX,
            }
    }
}
