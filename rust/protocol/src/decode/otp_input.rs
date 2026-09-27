use super::{DecodeError, Decoder};
use crate::otp_input::*;
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn otp_policy(&mut self) -> Result<Policy, DecodeError> {
        let length = self.int()?;
        let alphabet = match self.tag()? {
            0 => Alphabet::Digits,
            1 => Alphabet::AsciiAlphanumeric,
            _ => return Err(DecodeError::Malformed),
        };
        Policy::new(length, alphabet).ok_or(DecodeError::Malformed)
    }
    pub(super) fn otp_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            policy: self.otp_policy()?,
            label: self.bounded_text(MAX_INPUT_BYTES)?,
            masked: self.boolean()?,
            disabled: self.boolean()?,
            read_only: self.boolean()?,
            auto_focus: self.boolean()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn otp_selection(&mut self) -> Result<Selection, DecodeError> {
        let selection = Selection {
            anchor: self.int()?,
            head: self.int()?,
        };
        if selection.is_valid() {
            Ok(selection)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn otp_selection_policy(&mut self) -> Result<SelectionPolicy, DecodeError> {
        match self.tag()? {
            0 => Ok(SelectionPolicy::Start),
            1 => Ok(SelectionPolicy::End),
            2 => Ok(SelectionPolicy::Preserve),
            3 => Ok(SelectionPolicy::Select(self.otp_selection()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn otp_undo(&mut self) -> Result<UndoPolicy, DecodeError> {
        match self.tag()? {
            0 => Ok(UndoPolicy::Record),
            1 => Ok(UndoPolicy::Reset),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn otp_snapshot(&mut self) -> Result<Snapshot, DecodeError> {
        let snapshot = Snapshot {
            revision: self.int()?,
            policy: self.otp_policy()?,
            value: self.bounded_text(32)?,
            draft: self.bounded_text(MAX_INPUT_BYTES)?,
            selection: self.otp_selection()?,
            composition: self.option(Self::otp_selection)?,
            focused: self.boolean()?,
            can_undo: self.boolean()?,
            can_redo: self.boolean()?,
        };
        if snapshot.is_valid() {
            Ok(snapshot)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn otp_input_error(&mut self) -> Result<InputError, DecodeError> {
        Ok(match self.tag()? {
            0 => InputError::InvalidPolicy,
            1 => InputError::InputTooLarge,
            2 => InputError::InvalidUtf8,
            3 => {
                let offset = self.int()?;
                if !(0..MAX_INPUT_BYTES as i64).contains(&offset) {
                    return Err(DecodeError::Malformed);
                }
                InputError::UnexpectedCharacter {
                    byte_offset: offset as usize,
                }
            }
            4 => InputError::TooLong,
            5 => InputError::InvalidValue,
            6 => InputError::InvalidSelection,
            _ => return Err(DecodeError::Malformed),
        })
    }
    pub(super) fn otp_event(&mut self) -> Result<Event, DecodeError> {
        let event = match self.tag()? {
            0 => Event::Observed(self.otp_snapshot()?),
            1 => Event::Changed(self.otp_snapshot()?),
            2 => Event::Complete(self.otp_snapshot()?),
            3 => Event::Rejected(self.otp_input_error()?, self.otp_snapshot()?),
            _ => return Err(DecodeError::Malformed),
        };
        if event.is_valid() {
            Ok(event)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn otp_command(&mut self) -> Result<Command, DecodeError> {
        let command = match self.tag()? {
            0 => Command::Replace {
                value: self.bounded_text(32)?,
                selection: self.otp_selection_policy()?,
                undo: self.otp_undo()?,
                if_revision: self.option(Self::int)?,
            },
            1 => Command::Clear {
                undo: self.otp_undo()?,
                if_revision: self.option(Self::int)?,
            },
            2 => Command::Select(self.otp_selection()?),
            3 => Command::Focus,
            4 => Command::Undo,
            5 => Command::Redo,
            6 => Command::CancelComposition,
            7 => Command::ReadSnapshot,
            _ => return Err(DecodeError::Malformed),
        };
        if command.is_valid() {
            Ok(command)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn otp_error(&mut self) -> Result<Error, DecodeError> {
        Ok(match self.tag()? {
            0 => Error::NotMounted,
            1 => Error::Closed,
            2 => Error::StaleInput,
            3 => Error::StaleRevision,
            4 => Error::Composing,
            5 => Error::InvalidSelection,
            6 => Error::LimitExceeded,
            7 => Error::Busy,
            8 => Error::NativeFailure,
            9 => Error::InvalidValue,
            10 => Error::FocusBlocked,
            11 => Error::Disabled,
            12 => Error::ReadOnly,
            13 => Error::InvalidConfig,
            _ => return Err(DecodeError::Malformed),
        })
    }
    pub(super) fn otp_response(&mut self) -> Result<Response, DecodeError> {
        match self.tag()? {
            0 => Ok(Response::Applied(self.otp_snapshot()?)),
            1 => Ok(Response::Failed(self.otp_error()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
}
fn standalone<T>(
    bytes: &[u8],
    maximum: usize,
    read: impl FnOnce(&mut Decoder<'_>) -> Result<T, DecodeError>,
) -> Result<T, DecodeError> {
    if bytes.len() > maximum {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = read(&mut decoder)?;
    if decoder.remaining() == 0 {
        Ok(value)
    } else {
        Err(DecodeError::Malformed)
    }
}
pub fn decode_otp_input_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    standalone(bytes, MAX_CONFIG_BYTES, |d| d.otp_config())
}
pub fn decode_otp_input_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    standalone(bytes, MAX_EVENT_BYTES, |d| d.otp_event())
}
pub fn decode_otp_input_command(bytes: &[u8]) -> Result<Command, DecodeError> {
    standalone(bytes, MAX_COMMAND_BYTES, |d| d.otp_command())
}
pub fn decode_otp_input_response(bytes: &[u8]) -> Result<Response, DecodeError> {
    standalone(bytes, MAX_EVENT_BYTES, |d| d.otp_response())
}
