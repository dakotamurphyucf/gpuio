use super::{DecodeError, Decoder};
use crate::text_shimmer::{Appearance, Config, Direction, MAX_CONFIG_BYTES, Repeat, Spread};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn text_shimmer_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            duration_ms: self.int()?,
            spread: match self.tag()? {
                0 => Spread::Relative(self.float()?),
                1 => Spread::Pixels(self.float()?),
                _ => return Err(DecodeError::Malformed),
            },
            direction: match self.tag()? {
                0 => Direction::LeftToRight,
                1 => Direction::RightToLeft,
                _ => return Err(DecodeError::Malformed),
            },
            repeat: match self.tag()? {
                0 => Repeat::Once,
                1 => Repeat::Loop,
                _ => return Err(DecodeError::Malformed),
            },
            animated: self.boolean()?,
            highlight: self.option(Self::int)?,
            appearance: self.option(|decoder| {
                Ok(Appearance {
                    foreground: decoder.int()?,
                    background: decoder.int()?,
                    dark: decoder.boolean()?,
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

pub fn decode_text_shimmer_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.text_shimmer_config()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
