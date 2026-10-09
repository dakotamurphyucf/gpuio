use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub revision: i64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        let coordinate = |n: f64| n.is_finite() && (-1e9..=1e9).contains(&n);
        let dimension = |n: f64| n.is_finite() && (0.0..=1e9).contains(&n);
        self.revision >= 0
            && coordinate(self.x)
            && coordinate(self.y)
            && dimension(self.width)
            && dimension(self.height)
            && self.height > 0.
    }
}
