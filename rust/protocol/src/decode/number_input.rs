use super::{DecodeError, Decoder};
use crate::number_input::*;
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn number_initial_draft(&mut self) -> Result<String, DecodeError> {
        let draft = self.bounded_text(MAX_DRAFT_BYTES)?;
        if valid_text(&draft) {
            Ok(draft)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    pub(super) fn number_value(&mut self) -> Result<Value, DecodeError> {
        let value = match self.tag()? {
            0 => Value::Empty,
            1 => Value::Number(self.float()?),
            _ => return Err(DecodeError::Malformed),
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn number_config(&mut self) -> Result<Config, DecodeError> {
        let value = Config {
            domain: self.numeric_domain()?,
            label: self.bounded_text(4096)?,
            placeholder: self.bounded_text(4096)?,
            increment_label: self.bounded_text(4096)?,
            decrement_label: self.bounded_text(4096)?,
            step_controls: match self.tag()? {
                0 => StepControls::Hidden,
                1 => StepControls::Sides,
                2 => StepControls::Stacked,
                _ => return Err(DecodeError::Malformed),
            },
            allow_empty: self.boolean()?,
            disabled: self.boolean()?,
            read_only: self.boolean()?,
            auto_focus: self.boolean()?,
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn number_selection(&mut self) -> Result<Selection, DecodeError> {
        let value = Selection {
            anchor: self.int()?,
            head: self.int()?,
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn number_selection_policy(&mut self) -> Result<SelectionPolicy, DecodeError> {
        match self.tag()? {
            0 => Ok(SelectionPolicy::Start),
            1 => Ok(SelectionPolicy::End),
            2 => Ok(SelectionPolicy::Preserve),
            3 => Ok(SelectionPolicy::Select(self.number_selection()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn number_undo(&mut self) -> Result<UndoPolicy, DecodeError> {
        match self.tag()? {
            0 => Ok(UndoPolicy::Record),
            1 => Ok(UndoPolicy::Reset),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn number_snapshot(&mut self) -> Result<Snapshot, DecodeError> {
        let s = Snapshot {
            revision: self.int()?,
            domain: self.numeric_domain()?,
            draft: self.bounded_text(4096)?,
            committed: self.number_value()?,
            selection: self.number_selection()?,
            composition: self.option(Self::number_selection)?,
            focused: self.boolean()?,
        };
        if s.is_valid() {
            Ok(s)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn number_rejection(&mut self) -> Result<Rejection, DecodeError> {
        match self.tag()? {
            0 => Ok(Rejection::EmptyRequired),
            1 => Ok(Rejection::Incomplete),
            2 => Ok(Rejection::Syntax),
            3 => Ok(Rejection::NonFinite),
            4 => Ok(Rejection::Composing),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn number_source(&mut self) -> Result<Source, DecodeError> {
        match self.tag()? {
            0 => Ok(Source::Keyboard),
            1 => Ok(Source::Stepper),
            2 => Ok(Source::Accessibility),
            3 => Ok(Source::Programmatic),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn number_direction(&mut self) -> Result<crate::numeric::Direction, DecodeError> {
        match self.tag()? {
            0 => Ok(crate::numeric::Direction::Increase),
            1 => Ok(crate::numeric::Direction::Decrease),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn number_event(&mut self) -> Result<Event, DecodeError> {
        let event = match self.tag()? {
            0 => Event::Observed(self.number_snapshot()?),
            1 => Event::Changed(self.number_snapshot()?),
            2 => {
                let source = match self.tag()? {
                    0 => Source::Keyboard,
                    1 => Source::Stepper,
                    2 => Source::Accessibility,
                    3 => Source::Programmatic,
                    _ => return Err(DecodeError::Malformed),
                };
                Event::Committed(source, self.number_snapshot()?)
            }
            3 => Event::Rejected(self.number_rejection()?, self.number_snapshot()?),
            4 => {
                let reason = match self.tag()? {
                    0 => CancelReason::Escape,
                    1 => CancelReason::Programmatic,
                    _ => return Err(DecodeError::Malformed),
                };
                Event::Cancelled(reason, self.number_snapshot()?)
            }
            5 => Event::StepRequested(StepRequest {
                id: self.int()?,
                direction: self.number_direction()?,
                source: self.number_source()?,
                snapshot: self.number_snapshot()?,
            }),
            _ => return Err(DecodeError::Malformed),
        };
        if event.is_valid() {
            Ok(event)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn number_command(&mut self) -> Result<Command, DecodeError> {
        let command = match self.tag()? {
            0 => Command::ReplaceDraft {
                text: self.bounded_text(4096)?,
                selection: self.number_selection_policy()?,
                undo: self.number_undo()?,
                if_revision: self.option(Self::int)?,
            },
            1 => Command::ReplaceValue {
                value: self.number_value()?,
                selection: self.number_selection_policy()?,
                undo: self.number_undo()?,
                if_revision: self.option(Self::int)?,
            },
            2 => Command::Select(self.number_selection()?),
            3 => Command::Focus,
            4 => Command::Undo,
            5 => Command::Redo,
            6 => Command::Commit,
            7 => Command::Cancel,
            8 => Command::Step(match self.tag()? {
                0 => crate::numeric::Direction::Increase,
                1 => crate::numeric::Direction::Decrease,
                _ => return Err(DecodeError::Malformed),
            }),
            9 => Command::ReadSnapshot,
            10 => Command::ResolveStep {
                request_id: self.int()?,
                revision: self.int()?,
                value: self.option(Self::number_value)?,
            },
            _ => return Err(DecodeError::Malformed),
        };
        if command.is_valid() {
            Ok(command)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn number_error(&mut self) -> Result<Error, DecodeError> {
        match self.tag()? {
            0 => Ok(Error::NotMounted),
            1 => Ok(Error::Closed),
            2 => Ok(Error::StaleInput),
            3 => Ok(Error::StaleRevision),
            4 => Ok(Error::Composing),
            5 => Ok(Error::InvalidSelection),
            6 => Ok(Error::LimitExceeded),
            7 => Ok(Error::Busy),
            8 => Ok(Error::NativeFailure),
            9 => Ok(Error::InvalidText),
            10 => Ok(Error::InvalidValue),
            11 => Ok(Error::FocusBlocked),
            12 => Ok(Error::Disabled),
            13 => Ok(Error::ReadOnly),
            14 => Ok(Error::InvalidConfig),
            15 => Ok(Error::Rejected(self.number_rejection()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn number_response(&mut self) -> Result<Response, DecodeError> {
        match self.tag()? {
            0 => Ok(Response::Applied(self.number_snapshot()?)),
            1 => Ok(Response::Failed(self.number_error()?)),
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
pub fn decode_number_input_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    standalone(bytes, MAX_CONFIG_BYTES, |d| d.number_config())
}
pub fn decode_number_input_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    standalone(bytes, MAX_EVENT_BYTES, |d| d.number_event())
}
pub fn decode_number_input_command(bytes: &[u8]) -> Result<Command, DecodeError> {
    standalone(bytes, MAX_COMMAND_BYTES, |d| d.number_command())
}
pub fn decode_number_input_response(bytes: &[u8]) -> Result<Response, DecodeError> {
    standalone(bytes, MAX_EVENT_BYTES, |d| d.number_response())
}
