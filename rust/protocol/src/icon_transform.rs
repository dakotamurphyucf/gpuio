//! Bounded visual SRT metadata; layout and input geometry stay with the icon owner.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Transform {
    pub scale_x: f64,
    pub scale_y: f64,
    pub rotation_degrees: f64,
    pub translate_x: f64,
    pub translate_y: f64,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            scale_x: 1.,
            scale_y: 1.,
            rotation_degrees: 0.,
            translate_x: 0.,
            translate_y: 0.,
        }
    }
}
impl Transform {
    pub fn is_valid(self) -> bool {
        [
            (self.scale_x, 64.),
            (self.scale_y, 64.),
            (self.rotation_degrees, 360.),
            (self.translate_x, 16384.),
            (self.translate_y, 16384.),
        ]
        .into_iter()
        .all(|(value, limit)| value.is_finite() && value.abs() <= limit)
    }
}
