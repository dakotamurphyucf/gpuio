//! Numeric paint/layout policy, independent of draft and stepping ownership.
use crate::v1::Style;
use binprot::macros::BinProtWrite;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub gap: f64,
    pub button_width: f64,
    pub button_min_height: f64,
    pub stacked_button_min_height: f64,
    pub editor_padding: f64,
    pub border_width: Option<f64>,
    pub frame_style: Vec<Style>,
    pub editor_style: Vec<Style>,
    pub decrement_style: Vec<Style>,
    pub increment_style: Vec<Style>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            gap: 4.,
            button_width: 24.,
            button_min_height: 20.,
            stacked_button_min_height: 16.,
            editor_padding: 0.,
            border_width: None,
            frame_style: vec![],
            editor_style: vec![],
            decrement_style: vec![],
            increment_style: vec![],
        }
    }
}
impl Config {
    pub fn geometry_is_valid(&self) -> bool {
        let bounded = |n: f64, min, max| n.is_finite() && (min..=max).contains(&n);
        [self.gap, self.editor_padding]
            .into_iter()
            .all(|n| bounded(n, 0., 256.))
            && [
                self.button_width,
                self.button_min_height,
                self.stacked_button_min_height,
            ]
            .into_iter()
            .all(|n| bounded(n, 1., 256.))
            && self.border_width.is_none_or(|n| bounded(n, 0., 64.))
    }
}
