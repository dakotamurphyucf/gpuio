use super::{DecodeError, Decoder};
use crate::{
    table::MAX_COLUMNS,
    table_header::{MAX_TARGET_BYTES, Target},
};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn table_header_target(&mut self) -> Result<Target, DecodeError> {
        let target = match self.tag()? {
            0 => Target::Column(self.bounded_text(256)?),
            1 => {
                let level = self.int()?;
                let count = self.count(MAX_COLUMNS)?;
                let columns = (0..count)
                    .map(|_| self.bounded_text(256))
                    .collect::<Result<Vec<_>, _>>()?;
                Target::Group { level, columns }
            }
            _ => return Err(DecodeError::Malformed),
        };
        if target.is_valid() {
            Ok(target)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_table_header_target(bytes: &[u8]) -> Result<Target, DecodeError> {
    if bytes.len() > MAX_TARGET_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let target = decoder.table_header_target()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(target)
}
