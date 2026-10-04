use super::{DecodeError, Decoder};
use crate::input_validation::{MAX_SOURCE_BYTES, Matching, Rule, Source};
use std::io::Cursor;

pub fn decode_input_validation_source(bytes: &[u8]) -> Result<Source, DecodeError> {
    if bytes.len() > MAX_SOURCE_BYTES + 16 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let source = decoder.input_validation_source()?;
    if decoder.remaining() != 0 {
        Err(DecodeError::Malformed)
    } else {
        Ok(source)
    }
}

impl Decoder<'_> {
    fn input_validation_source(&mut self) -> Result<Source, DecodeError> {
        let source = Source {
            pattern: self.bounded_text(MAX_SOURCE_BYTES)?,
            matching: match self.tag()? {
                0 => Matching::WholeValue,
                1 => Matching::Substring,
                _ => return Err(DecodeError::Malformed),
            },
            case_sensitive: self.boolean()?,
        };
        if !source.is_valid() {
            Err(DecodeError::Malformed)
        } else {
            Ok(source)
        }
    }

    pub(super) fn input_validation_rule(&mut self) -> Result<Rule, DecodeError> {
        Ok(Rule {
            regex: self.input_validation_source()?,
            allow_empty: self.boolean()?,
        })
    }
}
