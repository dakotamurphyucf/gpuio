use super::{DecodeError, Decoder};
use crate::{
    document_profile::{Config, Instance},
    extension::{MAX_PROPERTIES, Schema},
};
impl Decoder<'_> {
    pub(super) fn document_profile(&mut self) -> Result<Config, DecodeError> {
        let value = Config {
            epoch: self.int()?,
            instance: self.option(|decoder| {
                Ok(Instance {
                    schema: Schema {
                        name: decoder.bounded_text(128)?,
                        version: decoder.int()?,
                        fingerprint: decoder.bounded_text(64)?,
                    },
                    generation: decoder.int()?,
                    properties: decoder.extension_payload(MAX_PROPERTIES)?,
                })
            })?,
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
