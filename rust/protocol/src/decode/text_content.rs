use super::{DecodeError, Decoder};
use crate::text_content::{Content, MAX_CONFIG_BYTES, MAX_SPANS, MAX_TEXT_BYTES, Span};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn text_content(&mut self) -> Result<Content, DecodeError> {
        let text = self.bounded_text(MAX_TEXT_BYTES)?;
        let spans = self.list(MAX_SPANS, |decoder| {
            Ok(Span {
                start_byte: decoder.int()?,
                end_byte: decoder.int()?,
                foreground: decoder.int()?,
            })
        })?;
        let content = Content { text, spans };
        if content.is_valid() {
            Ok(content)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_text_content(bytes: &[u8]) -> Result<Content, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let content = decoder.text_content()?;
    if decoder.remaining() == 0 {
        Ok(content)
    } else {
        Err(DecodeError::Malformed)
    }
}
