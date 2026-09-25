use crate::numeric::{Direction, Domain};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 12_416;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Scale {
    Linear,
    Logarithmic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Thumb {
    Single,
    Lower,
    Upper,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Value {
    Single(f64),
    Range { lower: f64, upper: f64 },
}
impl Value {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Single(v) => v.is_finite(),
            Self::Range { lower, upper } => {
                lower.is_finite() && upper.is_finite() && lower <= upper
            }
        }
    }
    pub fn supports(self, thumb: Thumb) -> bool {
        matches!(
            (self, thumb),
            (Self::Single(_), Thumb::Single) | (Self::Range { .. }, Thumb::Lower | Thumb::Upper)
        )
    }
    pub fn same_mode(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Single(_), Self::Single(_)) | (Self::Range { .. }, Self::Range { .. })
        )
    }
    pub fn normalized(self, domain: Domain) -> Option<Self> {
        if !self.is_valid() {
            return None;
        }
        Some(match self {
            Self::Single(v) => Self::Single(domain.normalize(v)?),
            Self::Range { lower, upper } => Self::Range {
                lower: domain.normalize(lower)?,
                upper: domain.normalize(upper)?,
            },
        })
    }
    pub fn at(self, thumb: Thumb) -> Option<f64> {
        match (self, thumb) {
            (Self::Single(v), Thumb::Single)
            | (Self::Range { lower: v, .. }, Thumb::Lower)
            | (Self::Range { upper: v, .. }, Thumb::Upper) => Some(v),
            _ => None,
        }
    }
    pub fn set(self, thumb: Thumb, value: f64, domain: Domain) -> Option<Self> {
        if !self.is_valid() {
            return None;
        }
        let value = domain.normalize(value)?;
        Some(match (self, thumb) {
            (Self::Single(_), Thumb::Single) => Self::Single(value),
            (Self::Range { upper, .. }, Thumb::Lower) => Self::Range {
                lower: value.min(upper),
                upper,
            },
            (Self::Range { lower, .. }, Thumb::Upper) => Self::Range {
                lower,
                upper: value.max(lower),
            },
            _ => return None,
        })
    }
    pub fn advance(self, thumb: Thumb, domain: Domain, direction: Direction) -> Option<Self> {
        self.set(thumb, domain.advance(self.at(thumb)?, direction)?, domain)
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub domain: Domain,
    pub label: String,
    pub lower_label: String,
    pub upper_label: String,
    pub axis: Axis,
    pub scale: Scale,
    pub disabled: bool,
    pub read_only: bool,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        let text = |s: &str| {
            s.len() <= 4096
                && !s.contains('\0')
                && !s
                    .trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}'))
                    .is_empty()
        };
        text(&self.label)
            && text(&self.lower_label)
            && text(&self.upper_label)
            && (self.scale == Scale::Linear || self.domain.min() > 0.)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.label.capacity()
            + self.lower_label.capacity()
            + self.upper_label.capacity()
    }
    pub fn editable(&self) -> bool {
        !self.disabled && !self.read_only
    }
    pub fn from_fraction(&self, fraction: f64) -> Option<f64> {
        if !self.is_valid() || !fraction.is_finite() {
            return None;
        }
        let (min, max) = (self.domain.min(), self.domain.max());
        if fraction <= 0. || min == max {
            return Some(min);
        }
        if fraction >= 1. {
            return Some(max);
        }
        let value = match self.scale {
            Scale::Linear => min + (max - min) * fraction,
            Scale::Logarithmic => {
                let ratio = (max - min) / min;
                if ratio.is_finite() {
                    min * (ratio.ln_1p() * fraction).exp()
                } else {
                    (min.ln() + (max.ln() - min.ln()) * fraction).exp()
                }
            }
        };
        self.domain.normalize(value.clamp(min, max))
    }
    pub fn fraction(&self, value: f64) -> Option<f64> {
        if !self.is_valid() || !value.is_finite() {
            return None;
        }
        let (min, max) = (self.domain.min(), self.domain.max());
        if min == max || value <= min {
            return Some(0.);
        }
        if value >= max {
            return Some(1.);
        }
        let fraction = match self.scale {
            Scale::Linear => (value - min) / (max - min),
            Scale::Logarithmic => {
                let ratio = (max - min) / min;
                if ratio.is_finite() {
                    ((value - min) / min).ln_1p() / ratio.ln_1p()
                } else {
                    (value.ln() - min.ln()) / (max.ln() - min.ln())
                }
            }
        };
        Some(fraction.clamp(0., 1.))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub revision: i64,
    pub value: Value,
    pub committed: Value,
    pub dragging: Option<Thumb>,
}
impl Snapshot {
    pub fn is_valid(self) -> bool {
        self.revision >= 0
            && self.value.is_valid()
            && self.committed.is_valid()
            && self.value.same_mode(self.committed)
            && match (self.dragging, self.value, self.committed) {
                (None, value, committed) => value == committed,
                (Some(Thumb::Single), Value::Single(_), Value::Single(_)) => self.revision > 0,
                (
                    Some(Thumb::Lower),
                    Value::Range { upper, .. },
                    Value::Range {
                        upper: previous, ..
                    },
                )
                | (
                    Some(Thumb::Upper),
                    Value::Range { lower: upper, .. },
                    Value::Range {
                        lower: previous, ..
                    },
                ) => self.revision > 0 && upper == previous,
                _ => false,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Source {
    Pointer,
    Keyboard,
    Accessibility,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
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
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Event {
    Observed(Snapshot),
    DragStarted(Snapshot),
    Preview(Snapshot),
    Committed(Source, Snapshot),
    Cancelled(CancelReason, Snapshot),
}
impl Event {
    pub fn snapshot(self) -> Snapshot {
        match self {
            Self::Observed(s)
            | Self::DragStarted(s)
            | Self::Preview(s)
            | Self::Committed(_, s)
            | Self::Cancelled(_, s) => s,
        }
    }
    pub fn is_valid(self) -> bool {
        let s = self.snapshot();
        s.is_valid()
            && match self {
                Self::Observed(_) => true,
                Self::DragStarted(_) => s.dragging.is_some() && s.value == s.committed,
                Self::Preview(_) => s.dragging.is_some(),
                Self::Committed(..) | Self::Cancelled(..) => s.dragging.is_none() && s.revision > 0,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Command {
    Replace {
        value: Value,
        if_revision: Option<i64>,
    },
    CancelDrag,
    Focus(Thumb),
    ReadSnapshot,
}
impl Command {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Replace { value, if_revision } => {
                value.is_valid() && if_revision.is_none_or(|r| r >= 0)
            }
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    NotMounted,
    Closed,
    StaleSlider,
    StaleRevision,
    WrongMode,
    WrongThumb,
    Disabled,
    FocusBlocked,
    Busy,
    InvalidValue,
    InvalidConfig,
    LimitExceeded,
    ReadOnly,
}
