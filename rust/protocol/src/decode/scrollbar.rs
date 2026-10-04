use super::{DecodeError, Decoder};
use crate::scrollbar::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn scrollbar_entrance(&mut self) -> Result<Entrance, DecodeError> {
        match self.tag()? {
            0 => Ok(Entrance::Fade),
            1 => Ok(Entrance::SlideAndFade),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn scrollbar_track(&mut self) -> Result<Track, DecodeError> {
        Ok(Track {
            background: self.option(Self::int)?,
            border: self.option(Self::int)?,
            width: self.option(Self::float)?,
        })
    }
    fn scrollbar_thumb(&mut self) -> Result<Thumb, DecodeError> {
        Ok(Thumb {
            background: self.option(Self::fill)?,
            width: self.option(Self::float)?,
            inset: self.option(Self::float)?,
            radius: self.option(Self::float)?,
            min_length: self.option(Self::float)?,
        })
    }
    pub(super) fn scrollbar_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            label: self.bounded_text(MAX_LABEL_BYTES)?,
            axis: match self.tag()? {
                0 => Axis::Horizontal,
                1 => Axis::Vertical,
                2 => Axis::Both,
                _ => return Err(DecodeError::Malformed),
            },
            mode: match self.tag()? {
                0 => Mode::Scrolling,
                1 => Mode::Hover,
                2 => Mode::Always,
                _ => return Err(DecodeError::Malformed),
            },
            appearance: Appearance {
                track: self.scrollbar_track()?,
                track_hover: self.scrollbar_track()?,
                track_pressed: self.scrollbar_track()?,
                thumb: self.scrollbar_thumb()?,
                thumb_hover: self.scrollbar_thumb()?,
                thumb_pressed: self.scrollbar_thumb()?,
            },
            motion: Motion {
                idle_ms: self.int()?,
                enter_ms: self.int()?,
                exit_ms: self.int()?,
                expand_ms: self.int()?,
                entrance: self.scrollbar_entrance()?,
                thumb_hover_entrance: self.scrollbar_entrance()?,
            },
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_scrollbar_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.scrollbar_config()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
