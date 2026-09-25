//! Small, noninteractive native loading presentations.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Kind {
    Skeleton,
    Shimmer,
    Spinner,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub kind: Kind,
    pub label: String,
    pub animated: bool,
    pub period_ms: i64,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        crate::v1::CommandConfig::valid_text(&self.label, 4096)
            && (100..=60_000).contains(&self.period_ms)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.label.capacity()
    }
}
