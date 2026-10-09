use super::*;
use crate::checkable::{Position, TabOrder};

pub fn decode_tab_order(bytes: &[u8]) -> Result<TabOrder, DecodeError> {
    decode_drag_data(bytes, |decoder| decoder.tab_order())
}
pub fn decode_radio_position(bytes: &[u8]) -> Result<Position, DecodeError> {
    decode_drag_data(bytes, |decoder| decoder.radio_position())
}
impl Decoder<'_> {
    pub(super) fn tab_order(&mut self) -> Result<TabOrder, DecodeError> {
        let value = TabOrder {
            tab_stop: self.boolean()?,
            index: self.int()?,
        };
        if !value.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(value)
    }
    pub(super) fn radio_position(&mut self) -> Result<Position, DecodeError> {
        let value = Position {
            index: self.int()?,
            count: self.int()?,
        };
        if !value.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(value)
    }
}
