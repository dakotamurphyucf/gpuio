//! Slider paint/geometry policy, independent of editing state.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Fill {
    Selected,
    Remaining,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Appearance {
    pub fill: Fill,
    pub track_thickness: f64,
    pub track_radius: f64,
    pub thumb_size: f64,
    pub target_size: f64,
    pub ring_width: f64,
    pub track_color: Option<i64>,
    pub fill_color: Option<i64>,
    pub thumb_color: Option<i64>,
    pub ring_color: Option<i64>,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            fill: Fill::Selected,
            track_thickness: 4.,
            track_radius: 2.,
            thumb_size: 12.,
            target_size: 20.,
            ring_width: 2.,
            track_color: None,
            fill_color: None,
            thumb_color: None,
            ring_color: None,
        }
    }
}
impl Appearance {
    pub fn is_valid(&self) -> bool {
        let dimension = |n: f64| n.is_finite() && (1. ..=256.).contains(&n);
        [self.track_thickness, self.thumb_size, self.target_size]
            .into_iter()
            .all(dimension)
            && self.track_thickness <= self.target_size
            && self.thumb_size <= self.target_size
            && self.track_radius.is_finite()
            && (0. ..=256.).contains(&self.track_radius)
            && self.ring_width.is_finite()
            && (0. ..=self.target_size / 2.).contains(&self.ring_width)
            && [
                self.track_color,
                self.fill_color,
                self.thumb_color,
                self.ring_color,
            ]
            .into_iter()
            .all(|n| n.is_none_or(|n| (0..=0xffff_ffff).contains(&n)))
    }
}
