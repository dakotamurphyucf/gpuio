//! Color-control value and interaction contracts. Codec/bridge admission is
//! separate; no native resources or GUI dependencies belong in this module.
use crate::{color_value::*, numeric};

pub const MAX_PALETTE_ENTRIES: usize = 256;
pub const MAX_PALETTE_LABEL_BYTES: usize = 256;
pub const MAX_LABEL_BYTES: usize = 4096;
pub const MAX_DRAFT_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Hue,
    Saturation,
    Lightness,
    Alpha,
}

impl Channel {
    /// Public channel entry uses degrees for hue, percentages for other channels.
    pub fn maximum(self) -> f64 {
        if self == Self::Hue { 360. } else { 100. }
    }
    pub fn read(self, value: Hsla) -> f64 {
        match self {
            Self::Hue => value.hue_degrees(),
            Self::Saturation => value.saturation() * 100.,
            Self::Lightness => value.lightness() * 100.,
            Self::Alpha => value.alpha() * 100.,
        }
    }
    pub fn set(self, value: Hsla, channel: f64) -> Option<Hsla> {
        if !channel.is_finite() || !(0. ..=self.maximum()).contains(&channel) {
            return None;
        }
        Hsla::new(
            if self == Self::Hue {
                channel
            } else {
                value.hue_degrees()
            },
            if self == Self::Saturation {
                channel / 100.
            } else {
                value.saturation()
            },
            if self == Self::Lightness {
                channel / 100.
            } else {
                value.lightness()
            },
            if self == Self::Alpha {
                channel / 100.
            } else {
                value.alpha()
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Hex,
    Channel(Channel),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteEntry {
    pub color: Rgba,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Labels {
    pub control: String,
    pub hue: String,
    pub saturation: String,
    pub lightness: String,
    pub alpha: String,
    pub hex: String,
    pub clear: String,
}

pub fn valid_label(text: &str, limit: usize) -> bool {
    text.len() <= limit
        && !text.contains('\0')
        && !text
            .trim_matches(|c: char| c.is_ascii_whitespace())
            .is_empty()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub labels: Labels,
    pub palette: Vec<PaletteEntry>,
    pub alpha_policy: AlphaPolicy,
    pub allow_empty: bool,
    pub disabled: bool,
    pub read_only: bool,
}

impl Config {
    pub fn is_valid(&self) -> bool {
        let labels = &self.labels;
        [
            &labels.control,
            &labels.hue,
            &labels.saturation,
            &labels.lightness,
            &labels.alpha,
            &labels.hex,
            &labels.clear,
        ]
        .iter()
        .all(|s| valid_label(s, MAX_LABEL_BYTES))
            && self.palette.len() <= MAX_PALETTE_ENTRIES
            && self
                .palette
                .iter()
                .all(|e| valid_label(&e.label, MAX_PALETTE_LABEL_BYTES))
    }
    pub fn allows(&self, value: Value) -> bool {
        match value {
            Value::Empty => self.allow_empty,
            Value::Color(color) => {
                self.alpha_policy == AlphaPolicy::AllowAlpha || color.alpha == 255
            }
        }
    }
    pub fn allows_channels(&self, value: Hsla) -> bool {
        self.alpha_policy == AlphaPolicy::AllowAlpha || value.alpha() == 1.
    }
    pub fn retained_bytes(&self) -> usize {
        let labels = &self.labels;
        std::mem::size_of::<Self>()
            + [
                &labels.control,
                &labels.hue,
                &labels.saturation,
                &labels.lightness,
                &labels.alpha,
                &labels.hex,
                &labels.clear,
            ]
            .iter()
            .map(|s| s.capacity())
            .sum::<usize>()
            + self.palette.capacity() * std::mem::size_of::<PaletteEntry>()
            + self
                .palette
                .iter()
                .map(|e| e.label.capacity())
                .sum::<usize>()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionKind {
    Drag(Channel),
    Text(Field),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interaction {
    pub id: i64,
    pub kind: InteractionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftStatus {
    Empty,
    Incomplete,
    Invalid,
    OutOfRange,
    Forbidden,
    Valid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub text: String,
    pub composing: bool,
    pub status: DraftStatus,
}

pub fn valid_draft(text: &str) -> bool {
    text.len() <= MAX_DRAFT_BYTES && !text.contains(['\0', '\n', '\r'])
}

/// Classification is distinct from native text storage. Invalid/composing text
/// must not replace the last valid native preview or commit a color.
pub fn classify(
    config: &Config,
    field: Field,
    text: &str,
    channels: Hsla,
) -> (DraftStatus, Option<Hsla>) {
    let (status, candidate) = match field {
        Field::Hex => match HexDraft::parse(text) {
            HexDraft::Empty => (DraftStatus::Empty, None),
            HexDraft::Incomplete => (DraftStatus::Incomplete, None),
            HexDraft::Invalid(_) => (DraftStatus::Invalid, None),
            HexDraft::Valid(color) => (DraftStatus::Valid, Some(color.to_hsla())),
        },
        Field::Channel(channel) => {
            let domain = numeric::Domain::new(0., channel.maximum(), 1.).unwrap();
            match numeric::Draft::parse(domain, text) {
                numeric::Draft::Empty => (DraftStatus::Empty, None),
                numeric::Draft::Incomplete => (DraftStatus::Incomplete, None),
                numeric::Draft::Invalid(_) => (DraftStatus::Invalid, None),
                numeric::Draft::OutOfRange(_) => (DraftStatus::OutOfRange, None),
                numeric::Draft::Valid(value) => (DraftStatus::Valid, channel.set(channels, value)),
            }
        }
    };
    let candidate = candidate.map(|next| {
        if field == Field::Hex && next.saturation() == 0. {
            Hsla::new(channels.hue_degrees(), 0., next.lightness(), next.alpha()).unwrap()
        } else {
            next
        }
    });
    match candidate {
        Some(candidate) if !config.allows_channels(candidate) => (DraftStatus::Forbidden, None),
        _ => (status, candidate),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub revision: i64,
    pub value: Value,
    pub committed: Value,
    pub channels: Hsla,
    pub interaction: Option<Interaction>,
    pub draft: Option<Draft>,
    pub value_allowed: bool,
    pub committed_allowed: bool,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.revision >= 0
            && match self.value {
                Value::Empty => true,
                Value::Color(v) => v == Rgba::of_hsla(self.channels),
            }
            && match (&self.interaction, &self.draft) {
                (None, None) => {
                    self.value == self.committed && self.value_allowed == self.committed_allowed
                }
                (
                    Some(Interaction {
                        id,
                        kind: InteractionKind::Drag(_),
                    }),
                    None,
                ) => *id > 0 && *id <= self.revision,
                (
                    Some(Interaction {
                        id,
                        kind: InteractionKind::Text(_),
                    }),
                    Some(draft),
                ) => *id > 0 && *id <= self.revision && valid_draft(&draft.text),
                _ => false,
            }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Pointer,
    Keyboard,
    Accessibility,
    Text,
    Palette,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelReason {
    Escape,
    ConfigurationChanged,
    Disabled,
    ReadOnly,
    Hidden,
    Modal,
    WindowInactive,
    Unmounted,
    Programmatic,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Observed(Snapshot),
    Started(Snapshot),
    Preview(Snapshot),
    Committed(Source, Snapshot),
    Cancelled(CancelReason, Snapshot),
}
impl Event {
    pub fn snapshot(&self) -> &Snapshot {
        match self {
            Self::Observed(s)
            | Self::Started(s)
            | Self::Preview(s)
            | Self::Committed(_, s)
            | Self::Cancelled(_, s) => s,
        }
    }
    pub fn is_valid(&self) -> bool {
        let s = self.snapshot();
        s.is_valid()
            && match self {
                Self::Observed(_) => true,
                Self::Started(_) => s.interaction.is_some() && s.value == s.committed,
                Self::Preview(_) => s.interaction.is_some(),
                Self::Committed(..) | Self::Cancelled(..) => {
                    s.interaction.is_none() && s.revision > 0
                }
            }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NotMounted,
    Closed,
    StaleColorInput,
    StaleRevision,
    StaleInteraction,
    Disabled,
    ReadOnly,
    FocusBlocked,
    Busy,
    InvalidValue,
    InvalidConfig,
    InvalidDraft,
    Composing,
    LimitExceeded,
    NativeFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Set {
        value: Value,
        if_revision: Option<i64>,
    },
    Reset {
        if_revision: Option<i64>,
    },
    Cancel,
    Focus(Field),
    ReadSnapshot,
}

impl Command {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Set { if_revision, .. } | Self::Reset { if_revision } => {
                if_revision.is_none_or(|r| r >= 0)
            }
            Self::Cancel | Self::Focus(_) | Self::ReadSnapshot => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    Applied(Snapshot),
    Failed(Error),
}

impl Response {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Applied(snapshot) => snapshot.is_valid(),
            Self::Failed(_) => true,
        }
    }
}
