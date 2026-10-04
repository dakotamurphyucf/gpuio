//! Declarative custom-spinner configuration. The adapter owns source acquisition.
use crate::{
    animation::Easing,
    image::{ImageConfig, ImageFit, ImageSource},
    loading,
};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 8192;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub animated: bool,
    pub period_ms: i64,
    pub easing: Easing,
    pub source: Option<ImageSource>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        crate::v1::CommandConfig::valid_text(&self.label, 4096)
            && (100..=60_000).contains(&self.period_ms)
            && self.easing.is_valid()
    }
    pub fn loading(&self) -> loading::Config {
        loading::Config {
            kind: loading::Kind::Spinner,
            label: self.label.clone(),
            animated: self.animated,
            period_ms: self.period_ms,
        }
    }
    pub fn image(&self) -> Option<ImageConfig> {
        self.source.map(|source| ImageConfig {
            source,
            fit: ImageFit::Contain,
            label: None,
        })
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.label.capacity()
    }
}
