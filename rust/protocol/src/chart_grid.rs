//! Grid presentation; positions use the corresponding data axis's projection.
use crate::chart_axis::{MAX_TICKS, TickPosition, color, within};
use binprot::macros::BinProtWrite;
use std::mem::size_of;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Grid {
    pub x: Option<Vec<TickPosition>>,
    pub y: Option<Vec<TickPosition>>,
    pub dashes: Vec<f64>,
    pub width: f64,
    pub color: Option<i64>,
}
impl Grid {
    pub fn is_valid(&self) -> bool {
        [&self.x, &self.y].into_iter().all(|ps| {
            ps.as_ref()
                .is_none_or(|ps| ps.len() <= MAX_TICKS && ps.iter().all(|p| p.is_valid()))
        }) && self.dashes.len() <= 16
            && self.dashes.iter().all(|n| within(*n, 0.5, 128.))
            && within(self.width, 0.5, 8.)
            && self.color.is_none_or(color)
    }
    pub fn heap_bytes(&self) -> usize {
        [&self.x, &self.y]
            .into_iter()
            .map(|p| {
                p.as_ref()
                    .map_or(0, |p| p.capacity() * size_of::<TickPosition>())
            })
            .sum::<usize>()
            + self.dashes.capacity() * size_of::<f64>()
    }
}
impl Default for Grid {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            dashes: vec![],
            width: 1.,
            color: None,
        }
    }
}
