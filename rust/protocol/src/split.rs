//! Native-owned two-pane geometry; reset generation changes geometry only.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub axis: Axis,
    pub initial_first: f64,
    pub minimum_first: f64,
    pub maximum_first: f64,
    pub minimum_second: f64,
    pub keyboard_step: f64,
    pub reset_generation: i64,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        !self.label.trim_ascii().is_empty()
            && self.label.len() <= 4096
            && !self.label.contains('\0')
            && [
                self.initial_first,
                self.minimum_first,
                self.maximum_first,
                self.minimum_second,
                self.keyboard_step,
            ]
            .iter()
            .all(|n| n.is_finite())
            && self.minimum_first >= 0.
            && self.minimum_first <= self.initial_first
            && self.initial_first <= self.maximum_first
            && self.maximum_first <= 16384.
            && (0.0..=16384.).contains(&self.minimum_second)
            && self.keyboard_step > 0.
            && self.keyboard_step <= 16384.
            && self.reset_generation >= 0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub first: f64,
    pub second: f64,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.first.is_finite() && self.second.is_finite() && self.first >= 0. && self.second >= 0.
    }
}
