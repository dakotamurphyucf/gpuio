use super::{DecodeError, Decoder};
use crate::{
    carousel::MAX_ITEMS,
    carousel_track::{Config, Layout, Loop, Proposal, Request, Stops},
};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn carousel_track_motion(
        &mut self,
    ) -> Result<crate::carousel_track::Motion, DecodeError> {
        let motion = crate::carousel_track::Motion {
            duration_ms: self.int()?,
            easing: self.animation_easing()?,
        };
        if motion.is_valid() {
            Ok(motion)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    pub(super) fn carousel_track_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            carousel: self.carousel_config()?,
            lineage: self.int()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn carousel_track_layout(&mut self) -> Result<Layout, DecodeError> {
        let layout = Layout {
            lineage: self.int()?,
            epoch: self.int()?,
            stops: self.option(|decoder| {
                let count = decoder.count(MAX_ITEMS)?;
                let mut canonical = Vec::with_capacity(count);
                for _ in 0..count {
                    canonical.push(decoder.int()?);
                }
                let looping = match decoder.tag()? {
                    0 => Loop::Finite,
                    1 => Loop::Jump,
                    2 => Loop::Continuous,
                    _ => return Err(DecodeError::Malformed),
                };
                Ok(Stops { canonical, looping })
            })?,
        };
        if layout.is_valid() {
            Ok(layout)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn carousel_track_request(&mut self) -> Result<Request, DecodeError> {
        let request = match self.tag()? {
            0 => Request::Previous,
            1 => Request::Next,
            2 => Request::First,
            3 => Request::Last,
            4 => Request::Select(self.bounded_text(256)?),
            5 => Request::Layout(self.carousel_track_layout()?),
            6 => Request::AutoNext(Proposal {
                revision: self.int()?,
                geometry_epoch: self.int()?,
                from: self.bounded_text(256)?,
                target: self.bounded_text(256)?,
            }),
            _ => return Err(DecodeError::Malformed),
        };
        if request.is_valid() {
            Ok(request)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_carousel_track_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_ITEMS * (256 + 3) + 48 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = decoder.carousel_track_config()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
pub fn decode_carousel_track_request(bytes: &[u8]) -> Result<Request, DecodeError> {
    // Bounds checked before allocating individual lists/strings as well.
    if bytes.len() > MAX_ITEMS * 9 + 32 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = decoder.carousel_track_request()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
