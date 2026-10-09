use super::{DecodeError, Decoder};
use crate::input_format::{Config, MAX_PATTERN_BYTES, Number};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn input_format_config(&mut self) -> Result<Config, DecodeError> {
        let value = match self.tag()? {
            0 => Config::Pattern(self.bounded_text(MAX_PATTERN_BYTES)?),
            1 => Config::Number(Number {
                separator: self.option(|decoder| decoder.bounded_text(4))?,
                fraction_digits: self.option(Self::int)?,
            }),
            _ => return Err(DecodeError::Malformed),
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_input_format(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > 2048 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = decoder.input_format_config()?;
    if decoder.remaining() == 0 {
        Ok(value)
    } else {
        Err(DecodeError::Malformed)
    }
}
