use super::{DecodeError, Decoder};
use crate::{
    control_appearance::{Config, LabelPosition, MAX_CONFIG_BYTES, MAX_DECLARATIONS},
    v1::Style,
};
use std::io::Cursor;

impl Decoder<'_> {
    fn control_part_styles(&mut self, remaining: &mut usize) -> Result<Vec<Style>, DecodeError> {
        self.list(MAX_DECLARATIONS, |d| {
            let style = d.style()?;
            let count = match &style {
                Style::Fields(fields) => fields.len(),
                Style::State(state, fields) if [4, 5, 6].contains(state) => fields.len(),
                _ => return Err(DecodeError::Malformed),
            };
            *remaining = remaining
                .checked_sub(count)
                .ok_or(DecodeError::LimitExceeded)?;
            Ok(style)
        })
    }

    pub(super) fn control_appearance(&mut self) -> Result<Config, DecodeError> {
        let mut remaining = MAX_DECLARATIONS;
        let config = Config {
            size: self.float()?,
            switch_width: self.float()?,
            gap: self.float()?,
            label_position: match self.tag()? {
                0 => LabelPosition::Before,
                1 => LabelPosition::After,
                _ => return Err(DecodeError::Malformed),
            },
            indicator_style: self.control_part_styles(&mut remaining)?,
            mark_style: self.control_part_styles(&mut remaining)?,
        };
        if config.valid_geometry() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_control_appearance(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.control_appearance()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
