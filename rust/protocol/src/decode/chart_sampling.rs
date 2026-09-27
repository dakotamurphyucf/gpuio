use super::{DecodeError, Decoder};
use crate::chart_sampling::{Bar, Candlestick, Line, Policy};
use std::io::Cursor;

pub fn decode_chart_sampling(bytes: &[u8]) -> Result<Policy, DecodeError> {
    if bytes.len() > 64 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let policy = Policy {
        version: d.int()?,
        line: match d.tag()? {
            0 => Line::Exact,
            1 => Line::Envelope(d.int()?),
            _ => return Err(DecodeError::Malformed),
        },
        bars: match d.tag()? {
            0 => Bar::Exact,
            1 => Bar::Sum(d.int()?),
            2 => Bar::Mean(d.int()?),
            _ => return Err(DecodeError::Malformed),
        },
        candles: match d.tag()? {
            0 => Candlestick::Exact,
            1 => Candlestick::Ohlc(d.int()?),
            _ => return Err(DecodeError::Malformed),
        },
    };
    if d.remaining() != 0 || !policy.is_valid() {
        return Err(DecodeError::Malformed);
    }
    Ok(policy)
}
