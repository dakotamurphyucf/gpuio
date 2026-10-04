use super::{DecodeError, Decoder};
use crate::list_input::Config;

impl Decoder<'_> {
    pub(super) fn list_input_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            generation: self.int()?,
            cursor: self.option(|d| d.int())?,
            query: self.option(|d| d.node())?,
            selection_on_navigation: self.boolean()?,
            disabled: self.boolean()?,
            busy: self.boolean()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
