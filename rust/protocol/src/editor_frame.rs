use binprot::macros::BinProtWrite;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub clear_label: Option<String>,
    pub loading: bool,
    pub gap: f64,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.clear_label
            .as_ref()
            .is_none_or(|label| crate::v1::CommandConfig::valid_text(label, 4096))
            && self.gap.is_finite()
            && (0.0..=256.0).contains(&self.gap)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.clear_label.as_ref().map_or(0, String::len)
    }
}
