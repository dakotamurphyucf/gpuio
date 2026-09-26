//! Application selection and native presentation policy; no native route payloads.
use binprot::macros::BinProtWrite;

pub const MAX_PAGES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Motion {
    Immediate,
    Slide,
    Fade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub selected: Option<i64>,
    pub retain: bool,
    pub motion: Motion,
    pub duration_ms: i64,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.selected
            .is_none_or(|index| (0..MAX_PAGES as i64).contains(&index))
            && (0..=10_000).contains(&self.duration_ms)
    }

    pub fn valid_children(&self, count: usize) -> bool {
        self.is_valid()
            && count <= MAX_PAGES
            && match self.selected {
                None => count == 0,
                Some(index) => index < count as i64,
            }
    }
}
