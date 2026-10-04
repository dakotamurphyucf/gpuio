use super::{DecodeError, Decoder};
use crate::{
    document_style::*,
    v1::{MAX_STYLE_FIELDS, Style},
};
use std::io::Cursor;
impl Decoder<'_> {
    fn document_part_styles(&mut self, remaining: &mut usize) -> Result<Vec<Style>, DecodeError> {
        self.list(MAX_STYLE_FIELDS, |d| {
            let style = d.style()?;
            let Style::Fields(fields) = &style else {
                return Err(DecodeError::Malformed);
            };
            *remaining = remaining
                .checked_sub(fields.len())
                .ok_or(DecodeError::LimitExceeded)?;
            if !fields.iter().all(allowed_field) {
                return Err(DecodeError::Malformed);
            }
            Ok(style)
        })
    }
    pub(super) fn document_style(&mut self) -> Result<Config, DecodeError> {
        let start = self.0.position();
        let mut remaining = MAX_DECLARATIONS;
        let config = Config {
            colors: self.list(6, |d| {
                let part = match d.tag()? {
                    0 => Part::Foreground,
                    1 => Part::MutedForeground,
                    2 => Part::Link,
                    3 => Part::Selection,
                    4 => Part::CodeBackground,
                    5 => Part::Border,
                    _ => return Err(DecodeError::Malformed),
                };
                Ok((part, d.color()?))
            })?,
            paragraph_gap_rem: self.option(Self::float)?,
            heading_base_font_size: self.option(Self::float)?,
            heading_sizes: self.option(|d| {
                Ok(HeadingSizes {
                    h1: d.float()?,
                    h2: d.float()?,
                    h3: d.float()?,
                    h4: d.float()?,
                    h5: d.float()?,
                    h6: d.float()?,
                })
            })?,
            inline_code: InlineCode {
                foreground: self.option(Self::color)?,
                background: self.option(Self::color)?,
                font_weight: self.option(Self::int)?,
                italic: self.option(Self::boolean)?,
                underline: self.option(|d| {
                    Ok(Underline {
                        color: d.option(Self::color)?,
                        thickness: d.float()?,
                        wavy: d.boolean()?,
                    })
                })?,
                strikethrough: self.option(|d| {
                    Ok(Strikethrough {
                        color: d.option(Self::color)?,
                        thickness: d.float()?,
                    })
                })?,
                fade_out: self.option(Self::float)?,
            },
            code_block: self.document_part_styles(&mut remaining)?,
            table: self.document_part_styles(&mut remaining)?,
            table_head: self.document_part_styles(&mut remaining)?,
            table_cell: self.document_part_styles(&mut remaining)?,
        };
        if self.0.position() - start > MAX_CONFIG_BYTES as u64 {
            return Err(DecodeError::LimitExceeded);
        }
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
}
pub fn decode_document_style(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.document_style()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
