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
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Navigate(Navigation, Option<Selection>),
    Select(i64, Selection),
    Focus(i64),
    SetExpanded(i64, bool),
    Activate(i64),
    SelectActive(Selection),
    ActivateActive,
}
impl Request {
    pub fn target(&self) -> Option<i64> {
        match self {
            Self::Select(id, _)
            | Self::Focus(id)
            | Self::SetExpanded(id, _)
            | Self::Activate(id) => Some(*id),
            Self::Navigate(..) | Self::SelectActive(_) | Self::ActivateActive => None,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.target().is_none_or(|id| id > 0)
    }
}
