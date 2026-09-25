use super::{DecodeError, Decoder};
use crate::numeric::Domain;
use std::io::Cursor;

impl Decoder<'_> {
    fn numeric_domain(&mut self) -> Result<Domain, DecodeError> {
        Domain::new(self.float()?, self.float()?, self.float()?).ok_or(DecodeError::Malformed)
    }
}

pub fn decode_numeric_domain(bytes: &[u8]) -> Result<Domain, DecodeError> {
    if bytes.len() > 24 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let result = decoder.numeric_domain()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(result)
}
