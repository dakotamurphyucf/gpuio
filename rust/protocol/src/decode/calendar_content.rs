use super::{DecodeError, Decoder};
use crate::calendar_content::{Config, Item, MAX_DESCRIPTION_BYTES, MAX_ITEMS, Slot};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn calendar_content(&mut self) -> Result<Config, DecodeError> {
        let mut description_bytes = 0;
        let items = self.list(MAX_ITEMS, |d| {
            let slot = match d.tag()? {
                0 => Slot::Previous,
                1 => Slot::Next,
                2 => Slot::ChooseMonth,
                3 => Slot::ChooseYear,
                4 => Slot::Today,
                5 => Slot::Clear,
                6 => Slot::Day(d.int()?),
                7 => Slot::Month(d.int()?),
                8 => Slot::Year(d.int()?),
                9 => Slot::MonthHeading(d.int()?),
                10 => Slot::Weekday(d.int()?, d.int()?),
                _ => return Err(DecodeError::Malformed),
            };
            if !slot.is_valid() {
                return Err(DecodeError::Malformed);
            }
            let description = d.option(|d| {
                let text = d.bounded_text((MAX_DESCRIPTION_BYTES - description_bytes).min(1024))?;
                description_bytes += text.len();
                Ok(text)
            })?;
            Ok(Item { slot, description })
        })?;
        let config = Config { items };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_calendar_content(bytes: &[u8]) -> Result<Config, DecodeError> {
    // Descriptions plus bounded integer/variant/string envelopes for 1024 slots.
    if bytes.len() > 98304 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.calendar_content()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
