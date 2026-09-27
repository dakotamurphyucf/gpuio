use super::{DecodeError, Decoder};
use crate::chart_sampling::{Bar, Candlestick, Line, Policy};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn chart_sampling(&mut self) -> Result<Policy, DecodeError> {
        let policy = Policy {
            version: self.int()?,
            line: match self.tag()? {
                0 => Line::Exact,
                1 => Line::Envelope(self.int()?),
                _ => return Err(DecodeError::Malformed),
            },
            bars: match self.tag()? {
                0 => Bar::Exact,
                1 => Bar::Sum(self.int()?),
                2 => Bar::Mean(self.int()?),
                _ => return Err(DecodeError::Malformed),
            },
            candles: match self.tag()? {
                0 => Candlestick::Exact,
                1 => Candlestick::Ohlc(self.int()?),
                _ => return Err(DecodeError::Malformed),
            },
        };
        if policy.is_valid() {
            Ok(policy)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_chart_sampling(bytes: &[u8]) -> Result<Policy, DecodeError> {
    if bytes.len() > 64 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let policy = d.chart_sampling()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(policy)
}
