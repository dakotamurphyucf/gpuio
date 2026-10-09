//! Optional geometry for shared popup surfaces; old side records are unchanged.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Point {
    pub corner: Corner,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub viewport_margin: f64,
    pub point: Option<Point>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.viewport_margin.is_finite()
            && (0. ..=16384.).contains(&self.viewport_margin)
            && self.point.is_none_or(|p| {
                [p.x, p.y]
                    .into_iter()
                    .all(|v| v.is_finite() && (-1_000_000. ..=1_000_000.).contains(&v))
            })
    }
    pub fn submenu(self) -> Self {
        Self {
            point: None,
            ..self
        }
    }
}
