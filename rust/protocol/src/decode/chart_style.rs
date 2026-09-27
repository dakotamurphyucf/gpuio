use super::{DecodeError, Decoder};
use crate::chart_style::Style;
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn chart_style(&mut self) -> Result<Style, DecodeError> {
        let style = Style {
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
        };
        if style.is_valid() {
            Ok(style)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_chart_style(bytes: &[u8]) -> Result<Style, DecodeError> {
    if bytes.len() > 512 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let value = d.chart_style()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
