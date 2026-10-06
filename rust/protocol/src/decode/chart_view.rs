use super::{DecodeError, Decoder};
use crate::chart_view::Config;
use std::io::Cursor;
impl Decoder<'_> {
    pub(super) fn chart_view_config(&mut self) -> Result<Config, DecodeError> {
        let version = self.int()?;
        if version != -1 {
            return Err(DecodeError::Malformed);
        }
        let value = Config {
            version,
            source: self.option(|d| d.resource())?,
            label: self.bounded_text(1024)?,
            options: self.chart_options()?,
            sampling: self.chart_sampling()?,
            style: self.chart_style()?,
            radar_labels: self.list(64, |d| d.int())?,
            legend: self.boolean()?,
            disabled: self.boolean()?,
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_chart_view_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len()
        > crate::chart_style::MAX_STYLE_BYTES + crate::chart_options::MAX_OPTIONS_BYTES + 2 * 1024
    {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let value = d.chart_view_config()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
