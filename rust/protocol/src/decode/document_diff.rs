use super::{DecodeError, Decoder};
use crate::document_diff::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn diff_key(&mut self) -> Result<FileKey, DecodeError> {
        match self.tag()? {
            0 => Ok(FileKey::Path(self.bounded_text(MAX_PATH_BYTES)?)),
            1 => Ok(FileKey::Unnamed),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn document_diff_config(&mut self) -> Result<Config, DecodeError> {
        let start = self.0.position();
        let ownership = self.tag()?;
        if ownership > 1 {
            return Err(DecodeError::Malformed);
        }
        let count = self.count(MAX_KEYS)?;
        let mut keys = Vec::with_capacity(count);
        for _ in 0..count {
            if self.0.position() - start > MAX_CONFIG_BYTES as u64 {
                return Err(DecodeError::LimitExceeded);
            }
            keys.push(self.diff_key()?);
        }
        let collapse = if ownership == 0 {
            Collapse::Managed(keys)
        } else {
            Collapse::Controlled(keys)
        };
        let line_limit = match self.tag()? {
            0 => LineLimit::Managed {
                initial: self.option(Self::int)?,
                step: self.int()?,
            },
            1 => LineLimit::Controlled(self.option(Self::int)?),
            _ => return Err(DecodeError::Malformed),
        };
        let config = Config {
            collapse,
            line_limit,
            word_diff: self.boolean()?,
        };
        if self.0.position() - start > MAX_CONFIG_BYTES as u64 {
            return Err(DecodeError::LimitExceeded);
        }
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn diff_file(&mut self) -> Result<File, DecodeError> {
        Ok(File {
            index: self.int()?,
            key: self.diff_key()?,
            before_path: self.option(|d| d.bounded_text(MAX_PATH_BYTES))?,
            after_path: self.option(|d| d.bounded_text(MAX_PATH_BYTES))?,
        })
    }
    fn document_diff_event(&mut self) -> Result<Event, DecodeError> {
        let config_epoch = self.int()?;
        let source_revision = self.int()?;
        let source_generation = self.int()?;
        let observation = match self.tag()? {
            0 => Observation::ToggleFile {
                file: self.diff_file()?,
                collapsed: self.boolean()?,
                applied: self.boolean()?,
            },
            1 => Observation::ShowMore {
                visible: self.int()?,
                hidden: self.int()?,
                applied_limit: self.option(Self::int)?,
            },
            2 => Observation::Line(Line {
                file: self.diff_file()?,
                before: self.option(Self::int)?,
                after: self.option(Self::int)?,
                start_byte: self.int()?,
                end_byte: self.int()?,
                text: self.bounded_text(16384)?,
            }),
            _ => return Err(DecodeError::Malformed),
        };
        let event = Event {
            config_epoch,
            source_revision,
            source_generation,
            observation,
        };
        if event.is_valid() {
            Ok(event)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_document_diff_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = decoder.document_diff_config()?;
    if decoder.remaining() == 0 {
        Ok(value)
    } else {
        Err(DecodeError::Malformed)
    }
}
pub fn decode_document_diff_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    if bytes.len() > MAX_EVENT_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = decoder.document_diff_event()?;
    if decoder.remaining() == 0 {
        Ok(value)
    } else {
        Err(DecodeError::Malformed)
    }
}
