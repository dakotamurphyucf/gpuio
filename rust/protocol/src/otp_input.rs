//! Segmented OTP configuration and observation/command contracts.
//! These types do not register a retained node or transport envelope by themselves.
pub use crate::otp::{Alphabet, InputError, MAX_INPUT_BYTES, Policy};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 4200;
pub const MAX_EVENT_BYTES: usize = 4352;
pub const MAX_COMMAND_BYTES: usize = 128;

pub fn valid_draft(text: &str) -> bool {
    text.len() <= MAX_INPUT_BYTES && !text.contains(['\0', '\r', '\n'])
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub policy: Policy,
    pub label: String,
    pub masked: bool,
    pub disabled: bool,
    pub read_only: bool,
    pub auto_focus: bool,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        valid_draft(&self.label)
            && !self
                .label
                .trim_matches([' ', '\t', '\r', '\n', '\u{b}', '\u{c}'])
                .is_empty()
    }
    pub fn retained_bytes(&self) -> usize {
        96 + self.label.len()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Selection {
    pub anchor: i64,
    pub head: i64,
}
impl Selection {
    pub fn is_valid(self) -> bool {
        (0..=4096).contains(&self.anchor) && (0..=4096).contains(&self.head)
    }
    pub fn within(self, text: &str) -> bool {
        self.is_valid()
            && text.is_char_boundary(self.anchor as usize)
            && text.is_char_boundary(self.head as usize)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum SelectionPolicy {
    Start,
    End,
    Preserve,
    Select(Selection),
}
impl SelectionPolicy {
    pub fn within(self, text: &str) -> bool {
        match self {
            Self::Select(s) => s.within(text),
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum UndoPolicy {
    Record,
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Snapshot {
    pub revision: i64,
    pub policy: Policy,
    pub value: String,
    pub draft: String,
    pub selection: Selection,
    pub composition: Option<Selection>,
    pub focused: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.revision >= 0
            && self.policy.canonical(&self.value)
            && valid_draft(&self.draft)
            && self.selection.within(&self.draft)
            && match self.composition {
                None => self.value == self.draft,
                Some(marked) => marked.anchor < marked.head && marked.within(&self.draft),
            }
    }
    pub fn is_complete(&self) -> bool {
        self.composition.is_none() && self.value.len() == self.policy.length()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Event {
    Observed(Snapshot),
    Changed(Snapshot),
    Complete(Snapshot),
    Rejected(InputError, Snapshot),
}
impl Event {
    pub fn snapshot(&self) -> &Snapshot {
        match self {
            Self::Observed(s) | Self::Changed(s) | Self::Complete(s) | Self::Rejected(_, s) => s,
        }
    }
    pub fn is_valid(&self) -> bool {
        let s = self.snapshot();
        s.is_valid()
            && match self {
                Self::Observed(_) => true,
                Self::Changed(_) => s.revision > 0,
                Self::Complete(_) => s.revision > 0 && s.is_complete(),
                Self::Rejected(reason, _) => {
                    s.revision > 0 && reason.is_valid() && s.composition.is_none()
                }
            }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Command {
    Replace {
        value: String,
        selection: SelectionPolicy,
        undo: UndoPolicy,
        if_revision: Option<i64>,
    },
    Clear {
        undo: UndoPolicy,
        if_revision: Option<i64>,
    },
    Select(Selection),
    Focus,
    Undo,
    Redo,
    CancelComposition,
    ReadSnapshot,
}
impl Command {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Replace {
                value,
                selection,
                if_revision,
                ..
            } => {
                value.len() <= 32
                    && value.bytes().all(|ch| ch.is_ascii_alphanumeric())
                    && selection.within(value)
                    && if_revision.is_none_or(|r| r >= 0)
            }
            Self::Clear { if_revision, .. } => if_revision.is_none_or(|r| r >= 0),
            Self::Select(s) => s.is_valid(),
            Self::Focus
            | Self::Undo
            | Self::Redo
            | Self::CancelComposition
            | Self::ReadSnapshot => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    NotMounted,
    Closed,
    StaleInput,
    StaleRevision,
    Composing,
    InvalidSelection,
    LimitExceeded,
    Busy,
    NativeFailure,
    InvalidValue,
    FocusBlocked,
    Disabled,
    ReadOnly,
    InvalidConfig,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Response {
    Applied(Snapshot),
    Failed(Error),
}
impl Response {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Applied(s) => s.is_valid(),
            Self::Failed(_) => true,
        }
    }
}
