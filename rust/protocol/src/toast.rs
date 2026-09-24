use crate::v1::CommandConfig;
use binprot::macros::BinProtWrite;
pub const MAX_TOAST_TIMEOUT_NS: i64 = 86_400_000_000_000;
pub const MAX_TOASTS: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ToastPoliteness {
    Polite,
    Assertive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ToastCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct ToastConfig {
    pub label: String,
    pub close_label: String,
    pub timeout_ns: Option<i64>,
    pub politeness: ToastPoliteness,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct ToastStackConfig {
    pub label: String,
    pub corner: ToastCorner,
    pub width: f64,
    pub max_visible: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ToastDismissal {
    Timeout,
    CloseButton,
    Escape,
    Overflow,
}
impl ToastConfig {
    pub fn is_valid(&self) -> bool {
        CommandConfig::valid_text(&self.label, 4096)
            && CommandConfig::valid_text(&self.close_label, 256)
            && self
                .timeout_ns
                .is_none_or(|ns| (1..=MAX_TOAST_TIMEOUT_NS).contains(&ns))
    }
    pub fn allows(&self, reason: ToastDismissal) -> bool {
        reason != ToastDismissal::Timeout || self.timeout_ns.is_some()
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.label.len() + self.close_label.len() + 1024
    }
}
impl ToastStackConfig {
    pub fn is_valid(&self) -> bool {
        CommandConfig::valid_text(&self.label, 4096)
            && self.width.is_finite()
            && self.width > 0.
            && self.width <= 1_000_000.
            && (1..=8).contains(&self.max_visible)
    }
}
