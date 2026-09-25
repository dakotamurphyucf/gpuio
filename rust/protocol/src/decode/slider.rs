use super::{DecodeError, Decoder};
use crate::slider::*;
use std::io::Cursor;
impl Decoder<'_> {
    pub(super) fn slider_value(&mut self) -> Result<Value, DecodeError> {
        let value = match self.tag()? {
            0 => Value::Single(self.float()?),
            1 => Value::Range {
                lower: self.float()?,
                upper: self.float()?,
            },
            _ => return Err(DecodeError::Malformed),
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn slider_thumb(&mut self) -> Result<Thumb, DecodeError> {
        match self.tag()? {
            0 => Ok(Thumb::Single),
            1 => Ok(Thumb::Lower),
            2 => Ok(Thumb::Upper),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn slider_snapshot(&mut self) -> Result<Snapshot, DecodeError> {
        let s = Snapshot {
            revision: self.int()?,
            value: self.slider_value()?,
            committed: self.slider_value()?,
            dragging: match self.tag()? {
                0 => None,
                1 => Some(self.slider_thumb()?),
                _ => return Err(DecodeError::Malformed),
            },
        };
        if s.is_valid() {
            Ok(s)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
impl Decoder<'_> {
    pub(super) fn slider_config(&mut self) -> Result<Config, DecodeError> {
        let c = Config {
            domain: self.numeric_domain()?,
            label: self.bounded_text(4096)?,
            lower_label: self.bounded_text(4096)?,
            upper_label: self.bounded_text(4096)?,
            axis: match self.tag()? {
                0 => Axis::Horizontal,
                1 => Axis::Vertical,
                _ => return Err(DecodeError::Malformed),
            },
            scale: match self.tag()? {
                0 => Scale::Linear,
                1 => Scale::Logarithmic,
                _ => return Err(DecodeError::Malformed),
            },
            disabled: self.boolean()?,
            read_only: self.boolean()?,
        };
        if c.is_valid() {
            Ok(c)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_slider_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let c = d.slider_config()?;
    if d.remaining() == 0 {
        Ok(c)
    } else {
        Err(DecodeError::Malformed)
    }
}
pub fn decode_slider_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    if bytes.len() > 64 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let event = match d.tag()? {
        0 => Event::Observed(d.slider_snapshot()?),
        1 => Event::DragStarted(d.slider_snapshot()?),
        2 => Event::Preview(d.slider_snapshot()?),
        3 => {
            let source = match d.tag()? {
                0 => Source::Pointer,
                1 => Source::Keyboard,
                2 => Source::Accessibility,
                _ => return Err(DecodeError::Malformed),
            };
            Event::Committed(source, d.slider_snapshot()?)
        }
        4 => {
            let reason = match d.tag()? {
                0 => CancelReason::Escape,
                1 => CancelReason::ConfigurationChanged,
                2 => CancelReason::Disabled,
                3 => CancelReason::ReadOnly,
                4 => CancelReason::Hidden,
                5 => CancelReason::Modal,
                6 => CancelReason::WindowInactive,
                7 => CancelReason::Unmounted,
                8 => CancelReason::Programmatic,
                9 => CancelReason::Interrupted,
                _ => return Err(DecodeError::Malformed),
            };
            Event::Cancelled(reason, d.slider_snapshot()?)
        }
        _ => return Err(DecodeError::Malformed),
    };
    if event.is_valid() && d.remaining() == 0 {
        Ok(event)
    } else {
        Err(DecodeError::Malformed)
    }
}
pub fn decode_slider_command(bytes: &[u8]) -> Result<Command, DecodeError> {
    if bytes.len() > 32 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let command = match d.tag()? {
        0 => Command::Replace {
            value: d.slider_value()?,
            if_revision: match d.tag()? {
                0 => None,
                1 => Some(d.int()?),
                _ => return Err(DecodeError::Malformed),
            },
        },
        1 => Command::CancelDrag,
        2 => Command::Focus(d.slider_thumb()?),
        3 => Command::ReadSnapshot,
        _ => return Err(DecodeError::Malformed),
    };
    if command.is_valid() && d.remaining() == 0 {
        Ok(command)
    } else {
        Err(DecodeError::Malformed)
    }
}
