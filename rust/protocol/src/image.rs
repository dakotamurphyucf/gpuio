//! Image configuration and asynchronous state. Encoded bytes use asset registration.
use crate::{ResourceId, v1::CommandConfig};
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum ImageFit {
    Fill,
    Contain,
    Cover,
    ScaleDown,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ImageError {
    WrongApplication,
    Released,
    InvalidData,
    Unsupported,
    ResourceLimit,
    NativeFailure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ImageSource {
    Reference(ResourceId),
    Unavailable(ImageError),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct ImageConfig {
    pub source: ImageSource,
    pub fit: ImageFit,
    /// None explicitly declares a decorative image.
    pub label: Option<String>,
}
impl ImageConfig {
    pub fn is_valid(&self) -> bool {
        self.label
            .as_ref()
            .is_none_or(|label| CommandConfig::valid_text(label, 4096))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct ImageMetadata {
    pub width_px: i64,
    pub height_px: i64,
    pub frames: i64,
}
impl ImageMetadata {
    pub fn is_valid(self) -> bool {
        (1..=16384).contains(&self.width_px)
            && (1..=16384).contains(&self.height_px)
            && (1..=120).contains(&self.frames)
            && self.width_px * self.height_px * 4 * self.frames <= 64 * 1024 * 1024
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ImageState {
    Loading,
    Ready(ImageMetadata),
    Failed(ImageError),
}
impl ImageState {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Ready(metadata) => metadata.is_valid(),
            Self::Loading | Self::Failed(_) => true,
        }
    }
}
