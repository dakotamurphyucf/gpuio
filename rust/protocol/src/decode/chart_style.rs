use super::{DecodeError, Decoder};
use crate::chart_style::{Key, MAX_COLOR_DOMAIN, MAX_STYLE_BYTES, Ordinal, Style};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn chart_style(&mut self) -> Result<Style, DecodeError> {
        let version = self.int()?;
        if version != 0 {
            return Err(DecodeError::Malformed);
        }
        let style = Style {
            version,
            palette: self.list(32, |d| d.int())?,
            axis_color: self.int()?,
            grid_color: self.int()?,
            label_color: self.int()?,
            selection_color: self.int()?,
            gradient_end: self.option(|d| d.int())?,
            stroke_width: self.float()?,
            point_radius: self.float()?,
            bar_radius: self.float()?,
            area_opacity: self.float()?,
            ordinal: self.option(|d| {
                Ok(Ordinal {
                    domain: d.list(MAX_COLOR_DOMAIN, |d| {
                        Ok(match d.tag()? {
                            0 => Key::Series(d.int()?),
                            1 => Key::Slice(d.int()?),
                            2 => Key::Node(d.int()?),
                            3 => Key::Rising,
                            4 => Key::Falling,
                            _ => return Err(DecodeError::Malformed),
                        })
                    })?,
                    range: d.list(32, |d| d.int())?,
                    unknown: d.option(|d| d.int())?,
                })
            })?,
        };
        if style.is_valid() {
            Ok(style)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_chart_style(bytes: &[u8]) -> Result<Style, DecodeError> {
    if bytes.len() > MAX_STYLE_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let value = d.chart_style()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
