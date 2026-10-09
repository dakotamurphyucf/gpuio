use super::{DecodeError, Decoder};
use crate::highlight::*;
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn highlight_config(&mut self) -> Result<Config, DecodeError> {
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

    fn highlight_observation(&mut self) -> Result<Observation, DecodeError> {
        let epoch = self.int()?;
        let state = match self.tag()? {
            0 => State::Pending,
            1 => {
                let count = self.count(MAX_SPECS)?;
                let mut counts = Vec::with_capacity(count);
                for _ in 0..count {
                    counts.push(Count {
                        total: self.int()?,
                        stored: self.int()?,
                    });
                }
                State::Ready(counts)
            }
            2 => State::InvalidRange(InvalidRange {
                spec_index: self.int()?,
                range_index: self.int()?,
                reason: match self.tag()? {
                    0 => RangeError::OutOfBounds,
                    1 => RangeError::ScalarBoundary,
                    _ => return Err(DecodeError::Malformed),
                },
            }),
            3 => State::Capacity(match self.tag()? {
                0 => Limit::Source,
                1 => Limit::Work,
                2 => Limit::Admission,
                _ => return Err(DecodeError::Malformed),
            }),
            4 => State::Failed(match self.tag()? {
                0 => Failure::SourceUnavailable,
                1 => Failure::WorkerFailed,
                2 => Failure::EpochExhausted,
                _ => return Err(DecodeError::Malformed),
            }),
            _ => return Err(DecodeError::Malformed),
        };
        let observation = Observation { epoch, state };
        if observation.is_valid() {
            Ok(observation)
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

pub fn decode_highlight_observation(bytes: &[u8]) -> Result<Observation, DecodeError> {
    if bytes.len() > MAX_OBSERVATION_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let observation = decoder.highlight_observation()?;
    if decoder.remaining() == 0 {
        Ok(observation)
    } else {
        Err(DecodeError::Malformed)
    }
}
