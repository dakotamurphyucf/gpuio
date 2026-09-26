use super::{DecodeError, Decoder};
use crate::carousel::{Axis, Config, Direction, MAX_ITEMS, Request};
use std::io::Cursor;
impl Decoder<'_> {
    pub(super) fn carousel_config(&mut self) -> Result<Config, DecodeError> {
        let revision = self.int()?;
        let count = self.count(MAX_ITEMS)?;
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            ids.push(self.bounded_text(256)?);
        }
        let config = Config {
            revision,
            ids,
            selected: self.option(|d| d.int())?,
            looping: self.boolean()?,
            disabled: self.boolean()?,
            axis: match self.tag()? {
                0 => Axis::Horizontal,
                1 => Axis::Vertical,
                _ => return Err(DecodeError::Malformed),
            },
            auto_advance_ms: self.option(|d| d.int())?,
            direction: match self.tag()? {
                0 => Direction::Direct,
                1 => Direction::Previous,
                2 => Direction::Next,
                _ => return Err(DecodeError::Malformed),
            },
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn carousel_request(&mut self) -> Result<Request, DecodeError> {
        let request = match self.tag()? {
            0 => Request::Previous,
            1 => Request::Next,
            2 => Request::First,
            3 => Request::Last,
            4 => Request::Select(self.bounded_text(256)?),
            5 => Request::AutoNext {
                revision: self.int()?,
                from: self.bounded_text(256)?,
                target: self.bounded_text(256)?,
            },
            _ => return Err(DecodeError::Malformed),
        };
        if request.is_valid() {
            Ok(request)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_carousel_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_ITEMS * (256 + 3) + 32 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.carousel_config()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(config)
}
pub fn decode_carousel_request(bytes: &[u8]) -> Result<Request, DecodeError> {
    if bytes.len() > 530 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let request = decoder.carousel_request()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(request)
}
