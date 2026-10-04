//! Validated value foundation for checkable composition/navigation.
//! No retained-tree operation or host capability advertises these values yet.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct TabOrder {
    pub tab_stop: bool,
    pub index: i64,
}
impl TabOrder {
    pub fn is_valid(self) -> bool {
        (-1_000_000..=1_000_000).contains(&self.index)
    }
}
impl Default for TabOrder {
    fn default() -> Self {
        Self {
            tab_stop: true,
            index: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Position {
    pub index: i64,
    pub count: i64,
}
impl Position {
    pub fn is_valid(self) -> bool {
        self.index >= 0 && self.index < self.count && self.count <= 100_000
    }
}
