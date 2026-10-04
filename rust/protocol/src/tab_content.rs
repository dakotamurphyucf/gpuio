//! Structured tab parts; selection remains in ChoiceConfig.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Label {
    Default,
    Custom,
    Hidden,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub max_width: Option<f64>,
    pub labels: Vec<Label>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.labels.len() <= 4096
            && self
                .max_width
                .is_none_or(|v| v.is_finite() && (1. ..=1e6).contains(&v))
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.labels.capacity() * std::mem::size_of::<Label>()
    }
}
