//! Bounded binding inspection data. Transport tags and mounted observers are not
//! integrated yet; this module alone does not advertise a live capability.
use crate::{
    NodeId, WindowId,
    command::{CommandConfig, NativeCommand, Shortcut, ShortcutPriority},
};
use binprot::macros::BinProtWrite;

pub const MAX_TARGETS: usize = 64;
pub const MAX_OBSERVERS: usize = 64;
pub const MAX_CONFIG_BYTES: usize = 32768;
pub const MAX_OBSERVATION_BYTES: usize = 262144;
pub const MAX_STROKES: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Context {
    Focused,
    Here,
    Editor(WindowId, NodeId),
    NativeContext(String),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Target {
    Command(String),
    NativeAction(NativeCommand),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub context: Context,
    pub targets: Vec<Target>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        (match &self.context {
            Context::NativeContext(text) => CommandConfig::valid_text(text, 1024),
            _ => true,
        }) && !self.targets.is_empty()
            && self.targets.len() <= MAX_TARGETS
            && self.targets.iter().enumerate().all(|(i, target)| {
                !self.targets[..i].contains(target)
                    && match (&self.context, target) {
                        (Context::Here, Target::NativeAction(_))
                        | (Context::NativeContext(_), Target::Command(_)) => false,
                        (_, Target::Command(id)) => CommandConfig::valid_text(id, 256),
                        (_, Target::NativeAction(_)) => true,
                    }
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Suppression {
    Disabled,
    ScopeBlocked,
    NativeUnavailable,
    Composition,
    TextInput,
    NativeNavigation,
    Conflict(String),
}
impl Suppression {
    fn is_valid(&self) -> bool {
        match self {
            Self::Conflict(id) => CommandConfig::valid_text(id, 256),
            _ => true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Disposition {
    Declared,
    Override,
    NativeFirst,
    Widget,
    Unavailable(Suppression),
}
impl Disposition {
    fn is_valid(&self) -> bool {
        match self {
            Self::Unavailable(reason) => reason.is_valid(),
            _ => true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Candidate {
    pub shortcut: Shortcut,
    pub disposition: Disposition,
}
impl Candidate {
    fn is_valid(&self) -> bool {
        self.shortcut.is_valid() && self.disposition.is_valid()
    }
    fn valid_for(&self, focused: bool, enabled: bool, command: &str) -> bool {
        if !focused {
            return self.disposition == Disposition::Declared;
        }
        match &self.disposition {
            Disposition::Declared | Disposition::Widget => false,
            Disposition::Override => {
                enabled && self.shortcut.priority == ShortcutPriority::Override
            }
            Disposition::NativeFirst => {
                enabled && self.shortcut.priority == ShortcutPriority::NativeFirst
            }
            Disposition::Unavailable(Suppression::Disabled) => !enabled,
            Disposition::Unavailable(Suppression::Conflict(id)) => enabled && id != command,
            Disposition::Unavailable(_) => enabled,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Stroke {
    pub key: String,
    /// Control=1, Alt=2, Shift=4, platform=8, Function=16. Display data only.
    pub modifiers: i64,
}
impl Stroke {
    pub fn is_valid(&self) -> bool {
        CommandConfig::valid_text(&self.key, 256)
            && self.key.chars().all(|ch| !ch.is_control())
            && (0..=31).contains(&self.modifiers)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Unsupported {
    SequenceTooLong,
    InvalidStroke,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Entry {
    MissingCommand,
    Registry {
        enabled: bool,
        candidates: Vec<Candidate>,
    },
    NativeUnbound,
    NativeBinding {
        strokes: Vec<Stroke>,
        disposition: Disposition,
    },
    NativeUnsupported(Unsupported),
}
impl Entry {
    fn is_valid(&self) -> bool {
        match self {
            Self::MissingCommand | Self::NativeUnbound | Self::NativeUnsupported(_) => true,
            Self::Registry { candidates, .. } => {
                candidates.len() <= 4 && candidates.iter().all(Candidate::is_valid)
            }
            Self::NativeBinding {
                strokes,
                disposition,
            } => {
                !strokes.is_empty()
                    && strokes.len() <= MAX_STROKES
                    && strokes.iter().all(Stroke::is_valid)
                    && disposition.is_valid()
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum State {
    Ready(Vec<Entry>),
    Suspended,
    ContextGone,
    InvalidContext,
    EpochExhausted,
    /// Sampling exceeded the native per-window work budget; never a missing binding.
    Capacity,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Observation {
    pub epoch: i64,
    pub state: State,
}
impl Observation {
    pub fn is_valid(&self) -> bool {
        self.epoch > 0
            && match &self.state {
                State::Ready(entries) => {
                    entries.len() <= MAX_TARGETS && entries.iter().all(Entry::is_valid)
                }
                _ => true,
            }
    }
    pub fn valid_for(&self, config: &Config) -> bool {
        if !self.is_valid() || !config.is_valid() {
            return false;
        }
        let focused = config.context == Context::Focused;
        match &self.state {
            State::Suspended | State::EpochExhausted | State::Capacity => true,
            State::ContextGone => matches!(config.context, Context::Editor(..)),
            State::InvalidContext => matches!(config.context, Context::NativeContext(_)),
            State::Ready(entries) => {
                entries.len() == config.targets.len()
                    && entries.iter().zip(&config.targets).all(|(entry, target)| {
                        match (target, entry) {
                            (Target::Command(_), Entry::MissingCommand)
                            | (
                                Target::NativeAction(_),
                                Entry::NativeUnbound | Entry::NativeUnsupported(_),
                            ) => true,
                            (
                                Target::Command(id),
                                Entry::Registry {
                                    enabled,
                                    candidates,
                                },
                            ) => candidates
                                .iter()
                                .all(|candidate| candidate.valid_for(focused, *enabled, id)),
                            (Target::NativeAction(_), Entry::NativeBinding { disposition, .. }) => {
                                if focused {
                                    matches!(
                                        disposition,
                                        Disposition::Widget | Disposition::Unavailable(_)
                                    )
                                } else {
                                    *disposition == Disposition::Declared
                                }
                            }
                            _ => false,
                        }
                    })
            }
        }
    }
}
