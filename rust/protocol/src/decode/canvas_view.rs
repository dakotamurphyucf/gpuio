use super::{DecodeError, Decoder};
use crate::{canvas::Point, canvas_view::*};
use std::io::Cursor;

impl Decoder<'_> {
    fn canvas_viewport(&mut self) -> Result<Viewport, DecodeError> {
        Ok(Viewport {
            origin: Point {
                x: self.float()?,
                y: self.float()?,
            },
            zoom: self.float()?,
        })
    }

    pub(super) fn canvas_view_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            source: self.option(|d| d.resource())?,
            label: self.bounded_text(1024)?,
            initial_viewport: self.canvas_viewport()?,
            minimum_zoom: self.float()?,
            maximum_zoom: self.float()?,
            selectable: self.boolean()?,
            draggable: self.boolean()?,
            pan_zoom: self.boolean()?,
            disabled: self.boolean()?,
            selection_color: self.int()?,
            command: self.option(|d| {
                Ok(Command {
                    sequence: d.int()?,
                    action: match d.tag()? {
                        0 => Action::Select(d.option(|d| d.int())?),
                        1 => Action::SetViewport(d.canvas_viewport()?),
                        2 => Action::ResetViewport,
                        3 => Action::ResetPositions,
                        _ => return Err(DecodeError::Malformed),
                    },
                })
            })?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_canvas_view_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.canvas_view_config()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(config)
}
