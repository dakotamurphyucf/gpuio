use super::{DecodeError, Decoder};
use crate::tab_content::{Config, Label};
impl Decoder<'_> {
    pub(super) fn tab_content(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            max_width: self.option(Self::float)?,
            labels: self.list(4096, |d| match d.tag()? {
                0 => Ok(Label::Default),
                1 => Ok(Label::Custom),
                2 => Ok(Label::Hidden),
                _ => Err(DecodeError::Malformed),
            })?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
