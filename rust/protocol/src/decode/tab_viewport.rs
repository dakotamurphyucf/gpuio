use super::{DecodeError, Decoder};
use crate::tab_viewport::{Config, Reveal};
impl Decoder<'_> {
    pub(super) fn tab_viewport(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            reveal: self.option(|d| {
                Ok(Reveal {
                    serial: d.int()?,
                    target: d.bounded_text(256)?,
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
