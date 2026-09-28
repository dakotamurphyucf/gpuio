use super::{DecodeError, Decoder};
use crate::chart_selection::{Aggregation, Selection, Span};
use std::io::Cursor;
impl Decoder<'_> {
    fn chart_span(&mut self) -> Result<Span, DecodeError> {
        Ok(Span {
            start_index: self.int()?,
            length: self.int()?,
            first: self.int()?,
            last: self.int()?,
        })
    }
    pub(super) fn chart_selection(&mut self) -> Result<Selection, DecodeError> {
        let selection = match self.tag()? {
            0 => Selection::Cartesian {
                series: self.int()?,
                span: self.chart_span()?,
                aggregation: match self.tag()? {
                    0 => Aggregation::Exact,
                    1 => Aggregation::Sum,
                    2 => Aggregation::Mean,
                    _ => return Err(DecodeError::Malformed),
                },
            },
            1 => Selection::Slice(self.int()?),
            2 => Selection::Radar {
                series: self.int()?,
                axis: self.int()?,
            },
            3 => Selection::Candlestick {
                span: self.chart_span()?,
                aggregated: self.boolean()?,
            },
            4 => Selection::Node(self.int()?),
            5 => Selection::Edge(self.int()?),
            _ => return Err(DecodeError::Malformed),
        };
        if selection.is_valid() {
            Ok(selection)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
/// Semantic targets contain only tags and at most five signed 64-bit integers.
pub fn decode_chart_selection(bytes: &[u8]) -> Result<Selection, DecodeError> {
    if bytes.len() > 64 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let selection = decoder.chart_selection()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(selection)
}
