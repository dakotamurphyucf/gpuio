use super::{DecodeError, Decoder};
use crate::link::{Config, MAX_CONFIG_BYTES, MAX_LABEL_BYTES};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn link_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            label: self.bounded_text(MAX_LABEL_BYTES)?,
            disabled: self.boolean()?,
            tab_stop: self.boolean()?,
            tab_index: self.int()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_link_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.link_config()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
