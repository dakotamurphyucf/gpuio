//! Bounded application-reserved space inside a sheet's window content viewport.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, Default, PartialEq, BinProtWrite)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
impl Insets {
    pub fn is_valid(&self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .into_iter()
            .all(|n| n.is_finite() && (0. ..=16384.).contains(&n))
    }
}
