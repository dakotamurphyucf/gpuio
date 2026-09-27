//! Ordered native tree intents. Targets are monotonic managed-list row IDs.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Navigation {
    Previous,
    Next,
    First,
    Last,
    Parent,
    Child,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Selection {
    Replace,
    Toggle,
    Range { extend: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Placement {
    Before,
    After,
    Inside,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Navigate(Navigation, Option<Selection>),
    Select(i64, Selection),
    Focus(i64),
    SetExpanded(i64, bool),
    Activate(i64),
    SelectActive(Selection),
    ActivateActive,
    Typeahead {
        text: String,
        reset: bool,
        cycle: bool,
    },
    SetSelected(i64, bool),
    Move {
        source: i64,
        destination: i64,
        placement: Placement,
    },
}
impl Request {
    pub fn target(&self) -> Option<i64> {
        match self {
            Self::Select(id, _)
            | Self::Focus(id)
            | Self::SetExpanded(id, _)
            | Self::SetSelected(id, _)
            | Self::Activate(id) => Some(*id),
            Self::Move { destination, .. } => Some(*destination),
            Self::Navigate(..)
            | Self::SelectActive(_)
            | Self::ActivateActive
            | Self::Typeahead { .. } => None,
        }
    }
    pub fn targets(&self) -> impl Iterator<Item = i64> {
        let source = match self {
            Self::Move { source, .. } => Some(*source),
            _ => None,
        };
        self.target().into_iter().chain(source)
    }
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Typeahead { text, .. } => valid_typeahead_text(text),
            Self::Move {
                source,
                destination,
                ..
            } => *source > 0 && *destination > 0 && source != destination,
            _ => self.target().is_none_or(|id| id > 0),
        }
    }
}

pub fn valid_typeahead_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= 256 && !text.chars().any(char::is_control)
}
