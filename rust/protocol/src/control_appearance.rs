//! Checkable indicator geometry and part styles. No selected value or callbacks.
use crate::v1::Style;
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 16 * 1024;
pub const MAX_DECLARATIONS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum LabelPosition {
    Before,
    After,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub size: f64,
    pub switch_width: f64,
    pub gap: f64,
    pub label_position: LabelPosition,
    pub indicator_style: Vec<Style>,
    pub mark_style: Vec<Style>,
}

impl Config {
    pub fn valid_geometry(&self) -> bool {
        self.size.is_finite()
            && (8. ..=128.).contains(&self.size)
            && self.switch_width.is_finite()
            && (self.size..=256.).contains(&self.switch_width)
            && self.gap.is_finite()
            && (0. ..=128.).contains(&self.gap)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: 18.,
            switch_width: 30.,
            gap: 8.,
            label_position: LabelPosition::After,
            indicator_style: vec![],
            mark_style: vec![],
        }
    }
}
