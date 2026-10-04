use super::{DecodeError, Decoder};
use crate::text_area_layout::{Config, WrappingIndent};

impl Decoder<'_> {
    pub(super) fn text_area_layout(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            soft_wrap: self.boolean()?,
            wrapping_indent: match self.tag()? {
                0 => WrappingIndent::FlushLeft,
                1 => WrappingIndent::MatchFirstLine,
                _ => return Err(DecodeError::Malformed),
            },
            show_whitespace: self.boolean()?,
            cursor_margin_lines: self.option(Self::int)?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
