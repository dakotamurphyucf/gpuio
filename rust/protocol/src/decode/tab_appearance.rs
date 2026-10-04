use super::{DecodeError, Decoder};
use crate::{
    tab_appearance::{Config, MAX_DECLARATIONS, MAX_ITEMS, Variant},
    v1::Style,
};
impl Decoder<'_> {
    fn tab_styles(&mut self, remaining: &mut usize) -> Result<Vec<Style>, DecodeError> {
        self.list(MAX_DECLARATIONS, |d| {
            let style = d.style()?;
            let fields = match &style {
                Style::Fields(fields) => fields,
                Style::State(1..=3 | 6..=7, fields) => fields,
                _ => return Err(DecodeError::Malformed),
            };
            *remaining = remaining
                .checked_sub(fields.len().max(1))
                .ok_or(DecodeError::LimitExceeded)?;
            Ok(style)
        })
    }
    pub(super) fn tab_appearance(&mut self) -> Result<Config, DecodeError> {
        let mut remaining = MAX_DECLARATIONS;
        let config = Config {
            variant: match self.tag()? {
                0 => Variant::Tab,
                1 => Variant::Outline,
                2 => Variant::Pill,
                3 => Variant::Segmented,
                4 => Variant::Underline,
                _ => return Err(DecodeError::Malformed),
            },
            height: self.float()?,
            gap: self.float()?,
            padding: self.float()?,
            tab_style: self.tab_styles(&mut remaining)?,
            item_styles: self.list(MAX_ITEMS, |d| {
                Ok((d.bounded_text(256)?, d.tab_styles(&mut remaining)?))
            })?,
        };
        if config.valid_geometry_and_ids() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
