//! Internal native table adapter. Delegates read retained Rust descriptions;
//! events are forwarded asynchronously by the host. See UPSTREAM.md.
mod appearance;
mod sizing;
pub mod table;

pub use appearance::{Appearance, Colors, SelectionAppearance};
pub use gpui_base::{ElementExt, StyledExt, VirtualListScrollHandle, h_flex, v_flex};
pub(crate) use sizing::StyleSized;
pub use sizing::{Sizable, Size};

pub(crate) mod actions {
    pub use gpui_base::actions::*;
}
pub(crate) mod scroll {
    pub use gpui_base::{ScrollableMask, Scrollbar};
}
pub(crate) mod virtual_list {
    pub use gpui_base::virtual_list;
}
pub(crate) use gpui_base::measurement_enabled as measure_enable;

/// Register only this adapter's key context. Does not change global themes,
/// install an asset source, or initialize unrelated styled controls.
pub fn init(cx: &mut gpui::App) {
    table::init(cx);
}
