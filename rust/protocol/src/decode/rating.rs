use super::{DecodeError, Decoder};
use crate::rating::Config;
impl Decoder<'_> {
    pub(super) fn rating_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            label: self.bounded_text(4096)?,
            value: self.int()?,
            maximum: self.int()?,
            star_size: self.float()?,
            disabled: self.boolean()?,
            read_only: self.boolean()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
