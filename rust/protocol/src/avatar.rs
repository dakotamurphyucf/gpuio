use crate::image::{ImageConfig, ImageFit, ImageSource};
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub source: Option<ImageSource>,
    pub fit: ImageFit,
    pub label: Option<String>,
    pub fallback: String,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        crate::v1::CommandConfig::valid_text(&self.fallback, 128)
            && !self.fallback.bytes().any(|b| b.is_ascii_control())
            && self
                .label
                .as_deref()
                .is_none_or(|s| crate::v1::CommandConfig::valid_text(s, 4096))
    }
    pub fn image(&self) -> Option<ImageConfig> {
        self.source.map(|source| ImageConfig {
            source,
            fit: self.fit,
            label: self.label.clone(),
        })
    }
    pub fn matches_image(&self, image: Option<&ImageConfig>) -> bool {
        match (self.source, image) {
            (None, None) => true,
            (Some(source), Some(image)) => {
                source == image.source && self.fit == image.fit && self.label == image.label
            }
            (None, Some(_)) | (Some(_), None) => false,
        }
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.fallback.capacity()
            + self.label.as_ref().map_or(0, String::capacity)
    }
}
