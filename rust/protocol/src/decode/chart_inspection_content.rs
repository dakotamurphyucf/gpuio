use super::{DecodeError, Decoder};
use crate::chart_inspection_content::{self as content, Container, Entry, Target};
use std::io::Cursor;

impl Decoder<'_> {
    fn chart_content_target(&mut self) -> Result<Target, DecodeError> {
        let target = match self.tag()? {
            0 => Target::Cartesian(self.int()?, self.int()?),
            1 => Target::Slice(self.int()?),
            2 => Target::Radar(self.int()?, self.int()?),
            3 => Target::Candlestick(self.int()?),
            4 => Target::Node(self.int()?),
            5 => Target::Edge(self.int()?),
            6 => Target::Aggregate {
                source: self.resource()?,
                data_revision: self.int()?,
                data_generation: self.int()?,
                selection: self.chart_selection()?,
            },
            _ => return Err(DecodeError::Malformed),
        };
        if target.is_valid() {
            Ok(target)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn chart_inspection_content(&mut self) -> Result<Vec<Entry>, DecodeError> {
        let entries = self.list(content::MAX_ENTRIES, |d| {
            Ok(Entry {
                target: d.option(|d| d.chart_content_target())?,
                container: match d.tag()? {
                    0 => Container::Card,
                    1 => Container::Overlay,
                    _ => return Err(DecodeError::Malformed),
                },
            })
        })?;
        if content::is_valid(&entries) {
            Ok(entries)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
/// Standalone metadata only; a later chart-view envelope supplies retained slots.
pub fn decode_chart_inspection_content(bytes: &[u8]) -> Result<Vec<Entry>, DecodeError> {
    if bytes.len() > content::MAX_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let entries = decoder.chart_inspection_content()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(entries)
}
