use super::{DecodeError, Decoder};
use crate::{
    image::ImageSource,
    spinner::{Config, MAX_CONFIG_BYTES},
};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn spinner_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            label: self.bounded_text(4096)?,
            animated: self.boolean()?,
            period_ms: self.int()?,
            easing: self.animation_easing()?,
            source: self.option(|decoder| {
                Ok(match decoder.tag()? {
                    0 => ImageSource::Reference(decoder.resource()?),
                    1 => ImageSource::Unavailable(decoder.image_error()?),
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_spinner_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.spinner_config()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
