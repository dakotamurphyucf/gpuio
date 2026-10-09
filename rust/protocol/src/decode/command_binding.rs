use super::{DecodeError, Decoder};
use crate::command::NativeCommand;
use crate::command_binding::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn binding_native_action(&mut self) -> Result<NativeCommand, DecodeError> {
        Ok(match self.tag()? {
            0 => NativeCommand::Copy,
            1 => NativeCommand::Cut,
            2 => NativeCommand::Paste,
            3 => NativeCommand::SelectAll,
            4 => NativeCommand::Undo,
            5 => NativeCommand::Redo,
            _ => return Err(DecodeError::Malformed),
        })
    }
    pub(super) fn command_binding_config(&mut self) -> Result<Config, DecodeError> {
        let context = match self.tag()? {
            0 => Context::Focused,
            1 => Context::Here,
            2 => Context::Editor(self.window()?, self.node()?),
            3 => Context::NativeContext(self.bounded_text(1024)?),
            _ => return Err(DecodeError::Malformed),
        };
        let targets = self.list(MAX_TARGETS, |d| {
            Ok(match d.tag()? {
                0 => Target::Command(d.bounded_text(256)?),
                1 => Target::NativeAction(d.binding_native_action()?),
                _ => return Err(DecodeError::Malformed),
            })
        })?;
        let config = Config { context, targets };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn binding_disposition(&mut self) -> Result<Disposition, DecodeError> {
        Ok(match self.tag()? {
            0 => Disposition::Declared,
            1 => Disposition::Override,
            2 => Disposition::NativeFirst,
            3 => Disposition::Widget,
            4 => Disposition::Unavailable(match self.tag()? {
                0 => Suppression::Disabled,
                1 => Suppression::ScopeBlocked,
                2 => Suppression::NativeUnavailable,
                3 => Suppression::Composition,
                4 => Suppression::TextInput,
                5 => Suppression::NativeNavigation,
                6 => Suppression::Conflict(self.bounded_text(256)?),
                _ => return Err(DecodeError::Malformed),
            }),
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn binding_entry(&mut self) -> Result<Entry, DecodeError> {
        Ok(match self.tag()? {
            0 => Entry::MissingCommand,
            1 => Entry::Registry {
                enabled: self.boolean()?,
                candidates: self.list(4, |d| {
                    Ok(Candidate {
                        shortcut: d.shortcut()?,
                        disposition: d.binding_disposition()?,
                    })
                })?,
            },
            2 => Entry::NativeUnbound,
            3 => Entry::NativeBinding {
                strokes: self.list(MAX_STROKES, |d| {
                    Ok(Stroke {
                        key: d.bounded_text(256)?,
                        modifiers: d.int()?,
                    })
                })?,
                disposition: self.binding_disposition()?,
            },
            4 => Entry::NativeUnsupported(match self.tag()? {
                0 => Unsupported::SequenceTooLong,
                1 => Unsupported::InvalidStroke,
                _ => return Err(DecodeError::Malformed),
            }),
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn command_binding_observation(&mut self) -> Result<Observation, DecodeError> {
        let epoch = self.int()?;
        let state = match self.tag()? {
            0 => State::Ready(self.list(MAX_TARGETS, Self::binding_entry)?),
            1 => State::Suspended,
            2 => State::ContextGone,
            3 => State::InvalidContext,
            4 => State::EpochExhausted,
            5 => State::Capacity,
            _ => return Err(DecodeError::Malformed),
        };
        let observation = Observation { epoch, state };
        if observation.is_valid() {
            Ok(observation)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_command_binding_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.command_binding_config()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
pub fn decode_command_binding_observation(bytes: &[u8]) -> Result<Observation, DecodeError> {
    if bytes.len() > MAX_OBSERVATION_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let observation = decoder.command_binding_observation()?;
    if decoder.remaining() == 0 {
        Ok(observation)
    } else {
        Err(DecodeError::Malformed)
    }
}
