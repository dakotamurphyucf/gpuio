use crate::v1::CommandConfig;
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const PALETTE_QUERY_BYTES: usize = 4096;
pub const PALETTE_HISTORY_BYTES: usize = 65536;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct PaletteConfig {
    pub label: String,
    pub placeholder: String,
    pub commands: Vec<String>,
    pub dismiss_on_outside_pointer: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum PaletteDismissal {
    Escape,
    OutsidePointer,
    Selected(String),
}
impl PaletteConfig {
    pub fn is_valid(&self) -> bool {
        let mut seen = BTreeSet::new();
        CommandConfig::valid_text(&self.label, 4096)
            && self.placeholder.len() <= 4096
            && !self.placeholder.contains('\0')
            && self.commands.len() <= 1024
            && self
                .commands
                .iter()
                .all(|id| CommandConfig::valid_text(id, 256) && seen.insert(id))
            && self.label.len()
                + self.placeholder.len()
                + self.commands.iter().map(String::len).sum::<usize>()
                <= 262144
    }
    pub fn permits(&self, id: &str) -> bool {
        self.commands.iter().any(|candidate| candidate == id)
    }
    pub fn allows(&self, reason: &PaletteDismissal) -> bool {
        match reason {
            PaletteDismissal::Escape => true,
            PaletteDismissal::OutsidePointer => self.dismiss_on_outside_pointer,
            PaletteDismissal::Selected(id) => self.permits(id),
        }
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.label.len()
            + self.placeholder.len()
            + self
                .commands
                .iter()
                .map(|id| std::mem::size_of::<String>() + id.len())
                .sum::<usize>()
            + PALETTE_QUERY_BYTES * 8
            + PALETTE_HISTORY_BYTES
    }
}
