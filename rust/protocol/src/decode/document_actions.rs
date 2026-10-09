use super::{DecodeError, Decoder};
use crate::document_actions::{Action, Config, MAX_ACTIONS};
impl Decoder<'_> {
    fn document_action_list(&mut self) -> Result<Vec<Action>, DecodeError> {
        let count = self.count(MAX_ACTIONS)?;
        (0..count)
            .map(|_| {
                Ok(Action {
                    id: self.bounded_text(64)?,
                    label: self.bounded_text(256)?,
                    enabled: self.boolean()?,
                })
            })
            .collect()
    }
    pub(super) fn document_actions(&mut self) -> Result<Config, DecodeError> {
        let value = Config {
            epoch: self.int()?,
            observe: self.boolean()?,
            copy_code: self.boolean()?,
            copy_table: self.boolean()?,
            code: self.document_action_list()?,
            table: self.document_action_list()?,
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
