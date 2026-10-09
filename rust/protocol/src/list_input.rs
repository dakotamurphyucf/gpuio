//! Ordered list intents. No native selection model or application payload.
use crate::NodeId;
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Navigation {
    Previous,
    Next,
    First,
    Last,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Gesture {
    Replace,
    Toggle,
    Range { extend: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Confirmation {
    Primary,
    Secondary,
}

/// Interaction lifetime, not a cursor update counter. Query ownership, disabled
/// state and navigation policy changes require a newer generation. Native node
/// admission retains the generation watermark when this configuration is cleared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub generation: i64,
    pub cursor: Option<i64>,
    pub query: Option<NodeId>,
    pub selection_on_navigation: bool,
    pub disabled: bool,
    pub busy: bool,
}
impl Config {
    pub fn is_valid(self) -> bool {
        self.generation > 0 && self.cursor.is_none_or(|row| row > 0)
    }
    pub fn can_replace(self, old: Self) -> bool {
        self.generation >= old.generation
            && (self.generation > old.generation
                || (self.query == old.query
                    && self.disabled == old.disabled
                    && self.selection_on_navigation == old.selection_on_navigation))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Navigate(Navigation, Option<Gesture>),
    Select(i64, Gesture),
    Focus(i64),
    SelectActive(Gesture),
    Confirm(i64, Confirmation),
    ConfirmActive(Confirmation),
    Context(i64),
    ContextActive,
    SetSelected(i64, bool),
    Cancel,
}
impl Request {
    pub fn target(self) -> Option<i64> {
        match self {
            Self::Select(id, _)
            | Self::Focus(id)
            | Self::Confirm(id, _)
            | Self::Context(id)
            | Self::SetSelected(id, _) => Some(id),
            Self::Navigate(..)
            | Self::SelectActive(_)
            | Self::ConfirmActive(_)
            | Self::ContextActive
            | Self::Cancel => None,
        }
    }
    pub fn is_valid(self) -> bool {
        self.target().is_none_or(|id| id > 0)
    }
}
