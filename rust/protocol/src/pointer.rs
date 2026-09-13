use crate::v1::CommandConfig;
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum PointerCancel {
    Escape,
    Hidden,
    Blocked,
    Disabled,
    Reconfigured,
    CaptureLost,
    WindowInactive,
    Removed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum PointerPhase {
    Started,
    Moved,
    Released,
    Cancelled(PointerCancel),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub struct PointerModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
    pub function: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct PointerConfig {
    pub label: String,
    pub button: PointerButton,
    pub disabled: bool,
    pub prevent_default: bool,
    pub stop_propagation: bool,
}
impl PointerConfig {
    pub fn is_valid(&self) -> bool {
        CommandConfig::valid_text(&self.label, 4096)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.label.len() + 512
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct PointerSample {
    pub gesture: i64,
    pub phase: PointerPhase,
    pub button: PointerButton,
    pub window_x: f64,
    pub window_y: f64,
    pub local_x: f64,
    pub local_y: f64,
    pub modifiers: PointerModifiers,
}
impl PointerSample {
    pub fn is_valid(&self) -> bool {
        self.gesture > 0
            && [self.window_x, self.window_y, self.local_x, self.local_y]
                .iter()
                .all(|value| value.is_finite())
    }
}
