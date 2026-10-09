//! Native notification reflow and finite entry/exit policy.
use crate::animation::Spring;
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub spring: Spring,
    pub enter_ms: i64,
    pub exit_ms: i64,
    pub offset: f64,
}
impl Config {
    pub fn is_valid(self) -> bool {
        self.spring.is_valid()
            && (0..=60_000).contains(&self.enter_ms)
            && (0..=60_000).contains(&self.exit_ms)
            && self.offset.is_finite()
            && (0.0..=16384.).contains(&self.offset)
    }
    /// Conservative quota units for measured descriptors, four trajectories,
    /// phase state, accepted token and finite tasks; child payloads are separate.
    pub fn retained_bytes(self, submitted: usize) -> usize {
        std::mem::size_of::<Self>() + 1024 + submitted * 4096
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            spring: crate::tab_motion::Config::default().spring,
            enter_ms: 400,
            exit_ms: 200,
            offset: 96.,
        }
    }
}
