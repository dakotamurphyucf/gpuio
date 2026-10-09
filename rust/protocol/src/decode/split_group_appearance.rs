use super::{DecodeError, Decoder};
use crate::{
    split_group_appearance::{Config, MAX_DECLARATIONS, MAX_ITEMS},
    v1::Style,
};
impl Decoder<'_> {
    fn split_group_styles(&mut self, remaining: &mut usize) -> Result<Vec<Style>, DecodeError> {
        self.list(MAX_DECLARATIONS, |d| {
            let style = d.style()?;
            let fields = match &style {
                Style::Fields(fields) => fields,
                Style::State(1..=3 | 6, fields) => fields,
                _ => return Err(DecodeError::Malformed),
            };
            *remaining = remaining
                .checked_sub(fields.len().max(1))
                .ok_or(DecodeError::LimitExceeded)?;
            Ok(style)
        })
    }
    pub(super) fn split_group_appearance(&mut self) -> Result<Config, DecodeError> {
        let mut remaining = MAX_DECLARATIONS;
        let config = Config {
            thickness: self.float()?,
            hit_extent: self.float()?,
            handle_style: self.split_group_styles(&mut remaining)?,
            item_styles: self.list(MAX_ITEMS, |d| {
                Ok((d.bounded_text(256)?, d.split_group_styles(&mut remaining)?))
            })?,
        };
        if config.valid_geometry_and_ids() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_split_group_appearance(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > 131072 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(std::io::Cursor::new(bytes));
    let result = d.split_group_appearance()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(result)
}
