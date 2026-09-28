use super::{DecodeError, Decoder};
use crate::highlight::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn highlight_config(&mut self) -> Result<Config, DecodeError> {
        let count = self.count(MAX_SPECS)?;
        let mut specs = Vec::with_capacity(count);
        let mut remaining_ranges = MAX_RANGES;
        for _ in 0..count {
            let query = self.option(|d| {
                Ok(Query {
                    text: d.bounded_text(MAX_QUERY_BYTES)?,
                    case_sensitive: d.boolean()?,
                    whole_word: d.boolean()?,
                })
            })?;
            let count = self.count(remaining_ranges)?;
            remaining_ranges -= count;
            let mut ranges = Vec::with_capacity(count);
            for _ in 0..count {
                ranges.push(Range {
                    start_byte: self.int()?,
                    end_byte: self.int()?,
                });
            }
            specs.push(Spec {
                query,
                ranges,
                appearance: Appearance {
                    color: self.int()?,
                    active_color: self.int()?,
                    radius: self.float()?,
                },
                active_index: self.option(Self::int)?,
                match_index_offset: self.int()?,
            });
        }
        let config = Config(specs);
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_highlight_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.highlight_config()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
