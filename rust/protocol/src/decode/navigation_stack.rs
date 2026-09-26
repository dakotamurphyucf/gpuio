use super::{DecodeError, Decoder};
use crate::navigation_stack::{Config, Motion};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn navigation_stack_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            selected: self.option(|d| d.int())?,
            retain: self.boolean()?,
            motion: match self.tag()? {
                0 => Motion::Immediate,
                1 => Motion::Slide,
                2 => Motion::Fade,
                _ => return Err(DecodeError::Malformed),
            },
            duration_ms: self.int()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_navigation_stack(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > 32 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.navigation_stack_config()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(config)
}
