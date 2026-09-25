use super::{DecodeError, Decoder};
use crate::container_query::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn container_range(&mut self) -> Result<Range, DecodeError> {
        Ok(Range {
            minimum: self.float()?,
            maximum: self.option(|d| d.float())?,
        })
    }
    pub(super) fn container_query_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            generation: self.int()?,
            branches: self.list(MAX_BRANCHES, |d| d.bounded_text(MAX_BRANCH_BYTES))?,
            default: self.int()?,
            rules: self.list(MAX_RULES, |d| {
                Ok(Rule {
                    condition: Predicate {
                        width: d.container_range()?,
                        height: d.container_range()?,
                    },
                    branch: d.int()?,
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
pub fn decode_container_query(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.container_query_config()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(config)
}
