//! Checked optional layered notification layout. Does not change toast identity.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Layering {
    pub peek: f64,
    pub gap: f64,
    pub width_step: f64,
    pub visible: i64,
}
impl Layering {
    pub fn is_valid(self) -> bool {
        [self.peek, self.gap]
            .into_iter()
            .all(|n| n.is_finite() && (0.0..=16384.).contains(&n))
            && self.width_step.is_finite()
            && (0.0..=0.1).contains(&self.width_step)
            && (1..=8).contains(&self.visible)
    }
    /// Bounded native stack/scroll bookkeeping, plus one measured descriptor
    /// and child wrapper per submitted card. Child-owned payloads are separate.
    pub fn retained_bytes(self, submitted: usize) -> usize {
        std::mem::size_of::<Self>() + 512 + submitted * 1024
    }
}
