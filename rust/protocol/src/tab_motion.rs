//! Native tab indicator policy; no frame callbacks or mutable handles cross FFI.
use crate::animation::Spring;
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub spring: Spring,
    pub color_duration_ms: i64,
}
impl Config {
    pub fn is_valid(self) -> bool {
        self.spring.is_valid() && (0..=60_000).contains(&self.color_duration_ms)
    }
    /// Fixed native owner, coordinate snapshots and finite spring trajectories.
    pub const OWNER_RESERVED_BYTES: usize = 4096;
}
impl Default for Config {
    fn default() -> Self {
        Self {
            spring: Spring {
                stiffness: 400.,
                damping: 40.,
                mass: 1.,
                epsilon: 0.01,
                max_duration_ms: 2000,
            },
            color_duration_ms: 200,
        }
    }
}
