use super::*;
use crate::button::{Config, Content, Focus, Policy};

pub fn decode_button_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    decode_drag_data(bytes, |decoder| decoder.button_config())
}
impl Decoder<'_> {
    pub(super) fn button_config(&mut self) -> Result<Config, DecodeError> {
        let loading = self.boolean()?;
        let focus = match self.tag()? {
            0 => Focus::Focusable(self.tab_order()?),
            1 => Focus::Preserve,
            _ => return Err(DecodeError::Malformed),
        };
        let content = match self.tag()? {
            0 => Content::IconSlots,
            1 => Content::Rich,
            _ => return Err(DecodeError::Malformed),
        };
        Ok(Config {
            policy: Policy { loading, focus },
            content,
        })
    }
}
