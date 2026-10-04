//! Native tab scrolling and one-shot stable-ID reveal.
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Reveal {
    pub serial: i64,
    pub target: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub reveal: Option<Reveal>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.reveal.as_ref().is_none_or(|r| {
            r.serial > 0
                && !r.target.is_empty()
                && r.target.len() <= 256
                && !r.target.contains('\0')
        })
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.reveal.as_ref().map_or(0, |r| r.target.capacity())
    }
    /// Bounded native state, pending ID, scroll state and exact measured-box array.
    pub fn owner_reserved_bytes(items: usize) -> usize {
        4096 + 64 * items
    }
}
