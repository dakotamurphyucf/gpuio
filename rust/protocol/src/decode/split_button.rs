use super::*;
use crate::split_button::{Config, MAX_DECLARATIONS, Parts};

pub fn decode_split_button_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    decode_drag_data(bytes, |decoder| decoder.split_button_config())
}

impl Decoder<'_> {
    fn split_button_styles(&mut self, remaining: &mut usize) -> Result<Vec<Style>, DecodeError> {
        self.list(MAX_DECLARATIONS, |d| {
            let style = d.style()?;
            let Style::Fields(fields) = &style else {
                return Err(DecodeError::Malformed);
            };
            *remaining = remaining
                .checked_sub(fields.len())
                .ok_or(DecodeError::LimitExceeded)?;
            Ok(style)
        })
    }

    pub(super) fn split_button_config(&mut self) -> Result<Config, DecodeError> {
        let mut remaining = MAX_DECLARATIONS;
        let config = Config {
            parts: match self.tag()? {
                0 => Parts::Primary,
                1 => Parts::Menu,
                2 => Parts::Split,
                _ => return Err(DecodeError::Malformed),
            },
            surface: self.split_button_styles(&mut remaining)?,
            menu_open: self.split_button_styles(&mut remaining)?,
        };
        if !config.has_valid_shape() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
}
