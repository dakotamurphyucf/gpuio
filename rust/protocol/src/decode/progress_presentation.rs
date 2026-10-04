use super::{DecodeError, Decoder};
use crate::{
    progress::ProgressConfig,
    progress_presentation::{Config, MAX_CONFIG_BYTES, Shape, Transition},
};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn progress_presentation(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            progress: ProgressConfig {
                label: self.bounded_text(4096)?,
                fraction: self.option(Self::float)?,
            },
            shape: match self.tag()? {
                0 => Shape::Linear,
                1 => Shape::Circle,
                _ => return Err(DecodeError::Malformed),
            },
            transition: match self.tag()? {
                0 => Transition::Immediate,
                1 => Transition::Tween {
                    duration_ms: self.int()?,
                    easing: self.animation_easing()?,
                },
                _ => return Err(DecodeError::Malformed),
            },
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_progress_presentation(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.progress_presentation()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
