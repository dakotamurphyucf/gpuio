//! Bounded external result order. Command definitions remain registry-owned.
use crate::{palette_layout, v1::PaletteConfig};
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Results {
    pub commands: Vec<String>,
    pub layout: Option<palette_layout::Config>,
}
impl Results {
    pub fn is_valid(&self) -> bool {
        let palette = PaletteConfig {
            label: "Results".into(),
            placeholder: String::new(),
            commands: self.commands.clone(),
            dismiss_on_outside_pointer: false,
        };
        palette.is_valid()
            && self
                .layout
                .as_ref()
                .is_none_or(|layout| layout.fits(&palette, None))
    }
}
// External search reserves this worst-case retained payload/projection capacity
// at tree admission, before a command can install results outside a transaction.
// <=256 KiB text *3 +1024 entries*(size+256) +1024 commands*256 +ID/vector storage.
pub const RESERVATION_BYTES: usize = 2 * 1024 * 1024;
