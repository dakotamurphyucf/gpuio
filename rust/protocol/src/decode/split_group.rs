use super::{DecodeError, Decoder};
use crate::split::Axis;
use crate::split_group::{Config, MAX_PANELS, Panel, ResizeRequest, Snapshot, Source};
use std::io::Cursor;
impl Decoder<'_> {
    pub(super) fn split_group(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            label: self.bounded_text(4096)?,
            axis: match self.tag()? {
                0 => Axis::Horizontal,
                1 => Axis::Vertical,
                _ => return Err(DecodeError::Malformed),
            },
            keyboard_step: self.float()?,
            reset_generation: self.int()?,
            resize: self.option(|d| {
                Ok(ResizeRequest {
                    id: d.bounded_text(256)?,
                    size: d.float()?,
                    serial: d.int()?,
                })
            })?,
            panels: self.list(MAX_PANELS, |d| {
                Ok(Panel {
                    id: d.bounded_text(256)?,
                    label: d.bounded_text(1024)?,
                    initial_size: d.option(Self::float)?,
                    minimum_size: d.float()?,
                    maximum_size: d.float()?,
                    visible: d.boolean()?,
                })
            })?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn split_group_snapshot(&mut self) -> Result<Snapshot, DecodeError> {
        let source = match self.tag()? {
            0 => Source::Pointer,
            1 => Source::Keyboard,
            2 => Source::Accessibility,
            3 => Source::Request(self.int()?),
            _ => return Err(DecodeError::Malformed),
        };
        let snapshot = Snapshot {
            source,
            sizes: self.list(MAX_PANELS, |d| Ok((d.bounded_text(256)?, d.float()?)))?,
        };
        if snapshot.is_valid() {
            Ok(snapshot)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_split_group_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > 131072 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let result = d.split_group()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(result)
}
pub fn decode_split_group_snapshot(bytes: &[u8]) -> Result<Snapshot, DecodeError> {
    if bytes.len() > 32768 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let result = d.split_group_snapshot()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(result)
}
