//! Mandatory epoch-2 button-owner policy and content interpretation.
use crate::checkable::TabOrder;
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Focus {
    Focusable(TabOrder),
    Preserve,
}
impl Default for Focus {
    fn default() -> Self {
        Self::Focusable(TabOrder::default())
    }
}
impl Focus {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Focusable(order) => order.is_valid(),
            Self::Preserve => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub struct Policy {
    pub loading: bool,
    pub focus: Focus,
}
impl Policy {
    pub fn is_valid(self) -> bool {
        self.focus.is_valid()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub enum Content {
    #[default]
    IconSlots,
    Rich,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub policy: Policy,
    pub content: Content,
}
impl Config {
    pub fn is_valid(self) -> bool {
        self.policy.is_valid()
    }
}
