use super::{DecodeError, Decoder};
use crate::chart_inspection::*;
impl Decoder<'_> {
    pub(super) fn chart_inspection(&mut self) -> Result<Inspection, DecodeError> {
        let value = Inspection {
            card: Card {
                visible: self.boolean()?,
                title: self.boolean()?,
                values: self.boolean()?,
                placement: match self.tag()? {
                    0 => Placement::Corner,
                    1 => Placement::Anchor,
                    2 => Placement::Cursor,
                    _ => return Err(DecodeError::Malformed),
                },
                width: self.float()?,
                gap: self.float()?,
                padding: self.float()?,
                radius: self.float()?,
                font_size: self.float()?,
                line_height: self.float()?,
                border_width: self.float()?,
                text_color: self.option(|d| d.int())?,
                background: self.option(|d| d.int())?,
                border_color: self.option(|d| d.int())?,
            },
            crosshair: Crosshair {
                axis: match self.tag()? {
                    0 => Axis::Off,
                    1 => Axis::Vertical,
                    2 => Axis::Horizontal,
                    3 => Axis::Both,
                    _ => return Err(DecodeError::Malformed),
                },
                pattern: match self.tag()? {
                    0 => Pattern::Dashed,
                    1 => Pattern::Solid,
                    _ => return Err(DecodeError::Malformed),
                },
                thickness: self.float()?,
                color: self.option(|d| d.int())?,
            },
            marker: Marker {
                visible: self.boolean()?,
                status: self.boolean()?,
                size: self.float()?,
                stroke_width: self.float()?,
                fill: self.option(|d| d.int())?,
                stroke: self.option(|d| d.int())?,
            },
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
