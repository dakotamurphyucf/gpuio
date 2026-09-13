use crate::v1::CommandConfig;
use binprot::macros::BinProtWrite;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct ProgressConfig {
    pub label: String,
    pub fraction: Option<f64>,
}
impl ProgressConfig {
    pub fn is_valid(&self) -> bool {
        CommandConfig::valid_text(&self.label, 4096)
            && self
                .fraction
                .is_none_or(|value| value.is_finite() && (0.0..=1.0).contains(&value))
    }
}
