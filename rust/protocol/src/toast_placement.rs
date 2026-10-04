//! Window-relative notification-stack placement; content and lifetime are independent.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    LeftCenter,
    RightCenter,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Placement {
    pub anchor: Anchor,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
impl Placement {
    pub fn is_valid(&self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .into_iter()
            .all(|n| n.is_finite() && (0.0..=16384.).contains(&n))
    }
}
