use super::{DecodeError, Decoder};
use crate::chart_resource::{Error, MAX_CHUNK_BYTES, Request, Response, Update};
use std::io::Cursor;

/// Standalone bounded resource frame; application correlation is added by the host.
pub fn decode_chart_request(bytes: &[u8]) -> Result<Request, DecodeError> {
    if bytes.len() > MAX_CHUNK_BYTES + 128 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let value = d.chart_request()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}

pub fn decode_chart_response(bytes: &[u8]) -> Result<Response, DecodeError> {
    let mut d = Decoder(Cursor::new(bytes));
    let value = match d.tag()? {
        0 => Response::Created(d.resource()?),
        1 => Response::Ack,
        2 => Response::Failed(match d.tag()? {
            0 => Error::Closed,
            1 => Error::ResourceLimit,
            2 => Error::StaleHandle,
            3 => Error::InvalidRevision,
            4 => Error::InvalidRange,
            5 => Error::Incomplete,
            6 => Error::Busy,
            7 => Error::NotReady,
            8 => Error::InvalidData,
            9 => Error::Cancelled,
            10 => Error::NativeFailure,
            _ => return Err(DecodeError::Malformed),
        }),
        _ => return Err(DecodeError::Malformed),
    };
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}

impl Decoder<'_> {
    pub(super) fn chart_request(&mut self) -> Result<Request, DecodeError> {
        Ok(match self.tag()? {
            0 => Request::Create,
            1 => Request::Begin(Update {
                id: self.resource()?,
                base: self.int()?,
                revision: self.int()?,
                generation: self.int()?,
                bytes: self.int()?,
            }),
            2 => {
                let id = self.resource()?;
                let revision = self.int()?;
                let offset = self.int()?;
                let bytes = self.extension_payload(MAX_CHUNK_BYTES)?;
                Request::Chunk(
                    id,
                    revision,
                    offset,
                    crate::asset::Chunk::new(bytes.0).map_err(|_| DecodeError::LimitExceeded)?,
                )
            }
            3 => Request::Publish(self.resource()?, self.int()?),
            4 => Request::Abort(self.resource()?, self.int()?),
            5 => Request::Release(self.resource()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
}
