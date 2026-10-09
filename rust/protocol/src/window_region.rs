//! Declarative native window gestures; no callback crosses the bridge.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Region {
    TitleBar,
    Exclude,
    Resize(Edge),
}
