use super::{DecodeError, Decoder};
use crate::avatar::Config;
use crate::image::{ImageFit, ImageSource};
impl Decoder<'_> {
    pub(super) fn avatar_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            source: self.option(|d| {
                Ok(match d.tag()? {
                    0 => ImageSource::Reference(d.resource()?),
                    1 => ImageSource::Unavailable(d.image_error()?),
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
            fit: match self.tag()? {
                0 => ImageFit::Fill,
                1 => ImageFit::Contain,
                2 => ImageFit::Cover,
                3 => ImageFit::ScaleDown,
                4 => ImageFit::None,
                _ => return Err(DecodeError::Malformed),
            },
            label: self.option(|d| d.bounded_text(4096))?,
            fallback: self.bounded_text(128)?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
