//! Native numeric editor contracts. Draft text is not a committed numeric value.
use crate::numeric::{Direction, Domain, Draft, DraftError};
use binprot::macros::BinProtWrite;

pub const MAX_DRAFT_BYTES: usize = 4096;
pub const MAX_CONFIG_BYTES: usize = 16_480;
pub const MAX_EVENT_BYTES: usize = 4300;
pub const MAX_COMMAND_BYTES: usize = 4200;
pub fn valid_text(text: &str) -> bool {
    text.len() <= MAX_DRAFT_BYTES && !text.contains(['\0', '\n', '\r'])
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Value {
    Empty,
    Number(f64),
}
impl Value {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Empty => true,
            Self::Number(v) => v.is_finite(),
        }
    }
    pub fn normalized(self, domain: Domain) -> Option<Self> {
        match self {
            Self::Empty => Some(self),
            Self::Number(v) => domain.normalize(v).map(Self::Number),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum StepControls {
    Hidden,
    Sides,
    Stacked,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub domain: Domain,
    pub label: String,
    pub placeholder: String,
    pub increment_label: String,
    pub decrement_label: String,
    pub step_controls: StepControls,
    pub allow_empty: bool,
    pub disabled: bool,
    pub read_only: bool,
    pub auto_focus: bool,
}
impl Config {
    pub fn retained_bytes(&self) -> usize {
        128 + self.label.len()
            + self.placeholder.len()
            + self.increment_label.len()
            + self.decrement_label.len()
    }
    pub fn is_valid(&self) -> bool {
        let label = |s: &str| {
            valid_text(s)
                && !s
                    .trim_matches(|c: char| {
                        matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}')
                    })
                    .is_empty()
        };
        label(&self.label)
            && valid_text(&self.placeholder)
            && label(&self.increment_label)
            && label(&self.decrement_label)
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
    pub fn is_valid(self) -> bool {
        match self {
            Self::Select(s) => s.is_valid(),
            _ => true,
        }
    }
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
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub revision: i64,
    pub domain: Domain,
    pub draft: String,
    pub committed: Value,
    pub selection: Selection,
    pub composition: Option<Selection>,
    pub focused: bool,
}
impl Snapshot {
    pub fn classification(&self) -> Draft {
        Draft::parse(self.domain, &self.draft)
    }
    pub fn is_valid(&self) -> bool {
        self.revision >= 0
            && valid_text(&self.draft)
            && self.selection.within(&self.draft)
            && self
                .composition
                .is_none_or(|s| s.anchor <= s.head && s.within(&self.draft))
            && self.committed.normalized(self.domain) == Some(self.committed)
    }
    pub fn is_settled(&self) -> bool {
        self.composition.is_none()
            && match (self.committed, self.classification()) {
                (Value::Empty, Draft::Empty) => true,
                (Value::Number(value), Draft::Valid(draft)) => value == draft,
                _ => false,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Source {
    Keyboard,
    Stepper,
    Accessibility,
    Programmatic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Rejection {
    EmptyRequired,
    Incomplete,
    Syntax,
    NonFinite,
    Composing,
}
impl Rejection {
    pub fn matches(self, s: &Snapshot) -> bool {
        match (self, s.classification()) {
            (Self::Composing, _) => s.composition.is_some(),
            (Self::EmptyRequired, Draft::Empty)
            | (Self::Incomplete, Draft::Incomplete)
            | (Self::Syntax, Draft::Invalid(DraftError::Syntax))
            | (Self::NonFinite, Draft::Invalid(DraftError::NonFinite)) => s.composition.is_none(),
            _ => false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum CancelReason {
    Escape,
    Programmatic,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Event {
    Observed(Snapshot),
    Changed(Snapshot),
    Committed(Source, Snapshot),
    Rejected(Rejection, Snapshot),
    Cancelled(CancelReason, Snapshot),
}
impl Event {
    pub fn snapshot(&self) -> &Snapshot {
        match self {
            Self::Observed(s)
            | Self::Changed(s)
            | Self::Committed(_, s)
            | Self::Rejected(_, s)
            | Self::Cancelled(_, s) => s,
        }
    }
    pub fn is_valid(&self) -> bool {
        let s = self.snapshot();
        s.is_valid()
            && match self {
                Self::Observed(_) => true,
                Self::Changed(_) => s.revision > 0,
                Self::Committed(..) | Self::Cancelled(..) => s.revision > 0 && s.is_settled(),
                Self::Rejected(reason, _) => s.revision > 0 && reason.matches(s),
            }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Command {
    ReplaceDraft {
        text: String,
        selection: SelectionPolicy,
        undo: UndoPolicy,
        if_revision: Option<i64>,
    },
    ReplaceValue {
        value: Value,
        selection: SelectionPolicy,
        undo: UndoPolicy,
        if_revision: Option<i64>,
    },
    Select(Selection),
    Focus,
    Undo,
    Redo,
    Commit,
    Cancel,
    Step(Direction),
    ReadSnapshot,
}
impl Command {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::ReplaceDraft {
                text,
                selection,
                if_revision,
                ..
            } => valid_text(text) && selection.within(text) && if_revision.is_none_or(|r| r >= 0),
            Self::ReplaceValue {
                value,
                selection,
                if_revision,
                ..
            } => value.is_valid() && selection.is_valid() && if_revision.is_none_or(|r| r >= 0),
            Self::Select(s) => s.is_valid(),
            Self::Focus
            | Self::Undo
            | Self::Redo
            | Self::Commit
            | Self::Cancel
            | Self::Step(_)
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
    InvalidText,
    InvalidValue,
    FocusBlocked,
    Disabled,
    ReadOnly,
    InvalidConfig,
    Rejected(Rejection),
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
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
