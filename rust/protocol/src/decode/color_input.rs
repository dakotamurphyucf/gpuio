use super::{DecodeError, Decoder};
use crate::{color_input::*, color_value::*};
use std::io::Cursor;

impl Decoder<'_> {
    fn color_rgba(&mut self) -> Result<Rgba, DecodeError> {
        Rgba::from_packed(self.int()?).ok_or(DecodeError::Malformed)
    }
    pub(super) fn color_value(&mut self) -> Result<Value, DecodeError> {
        match self.tag()? {
            0 => Ok(Value::Empty),
            1 => Ok(Value::Color(self.color_rgba()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn color_hsla(&mut self) -> Result<Hsla, DecodeError> {
        Hsla::new(self.float()?, self.float()?, self.float()?, self.float()?)
            .ok_or(DecodeError::Malformed)
    }
    fn color_channel(&mut self) -> Result<Channel, DecodeError> {
        match self.tag()? {
            0 => Ok(Channel::Hue),
            1 => Ok(Channel::Saturation),
            2 => Ok(Channel::Lightness),
            3 => Ok(Channel::Alpha),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn color_field(&mut self) -> Result<Field, DecodeError> {
        match self.tag()? {
            0 => Ok(Field::Hex),
            1 => Ok(Field::Channel(self.color_channel()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn color_config(&mut self) -> Result<Config, DecodeError> {
        let labels = Labels {
            control: self.bounded_text(MAX_LABEL_BYTES)?,
            hue: self.bounded_text(MAX_LABEL_BYTES)?,
            saturation: self.bounded_text(MAX_LABEL_BYTES)?,
            lightness: self.bounded_text(MAX_LABEL_BYTES)?,
            alpha: self.bounded_text(MAX_LABEL_BYTES)?,
            hex: self.bounded_text(MAX_LABEL_BYTES)?,
            clear: self.bounded_text(MAX_LABEL_BYTES)?,
        };
        let count = self.count(MAX_PALETTE_ENTRIES)?;
        let mut palette = Vec::with_capacity(count);
        for _ in 0..count {
            palette.push(PaletteEntry {
                color: self.color_rgba()?,
                label: self.bounded_text(MAX_PALETTE_LABEL_BYTES)?,
            });
        }
        let config = Config {
            labels,
            palette,
            alpha_policy: match self.tag()? {
                0 => AlphaPolicy::AllowAlpha,
                1 => AlphaPolicy::OpaqueOnly,
                _ => return Err(DecodeError::Malformed),
            },
            allow_empty: self.boolean()?,
            disabled: self.boolean()?,
            read_only: self.boolean()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn color_snapshot(&mut self) -> Result<Snapshot, DecodeError> {
        let revision = self.int()?;
        let value = self.color_value()?;
        let committed = self.color_value()?;
        let channels = self.color_hsla()?;
        let interaction = match self.tag()? {
            0 => None,
            1 => Some(Interaction {
                id: self.int()?,
                kind: match self.tag()? {
                    0 => InteractionKind::Drag(self.color_channel()?),
                    1 => InteractionKind::Text(self.color_field()?),
                    _ => return Err(DecodeError::Malformed),
                },
            }),
            _ => return Err(DecodeError::Malformed),
        };
        let draft = match self.tag()? {
            0 => None,
            1 => Some(Draft {
                text: self.bounded_text(MAX_DRAFT_BYTES)?,
                composing: self.boolean()?,
                status: match self.tag()? {
                    0 => DraftStatus::Empty,
                    1 => DraftStatus::Incomplete,
                    2 => DraftStatus::Invalid,
                    3 => DraftStatus::OutOfRange,
                    4 => DraftStatus::Forbidden,
                    5 => DraftStatus::Valid,
                    _ => return Err(DecodeError::Malformed),
                },
            }),
            _ => return Err(DecodeError::Malformed),
        };
        let snapshot = Snapshot {
            revision,
            value,
            committed,
            channels,
            interaction,
            draft,
            value_allowed: self.boolean()?,
            committed_allowed: self.boolean()?,
        };
        if snapshot.is_valid() {
            Ok(snapshot)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn color_source(&mut self) -> Result<Source, DecodeError> {
        match self.tag()? {
            0 => Ok(Source::Pointer),
            1 => Ok(Source::Keyboard),
            2 => Ok(Source::Accessibility),
            3 => Ok(Source::Text),
            4 => Ok(Source::Palette),
            5 => Ok(Source::Clear),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn color_cancel_reason(&mut self) -> Result<CancelReason, DecodeError> {
        match self.tag()? {
            0 => Ok(CancelReason::Escape),
            1 => Ok(CancelReason::ConfigurationChanged),
            2 => Ok(CancelReason::Disabled),
            3 => Ok(CancelReason::ReadOnly),
            4 => Ok(CancelReason::Hidden),
            5 => Ok(CancelReason::Modal),
            6 => Ok(CancelReason::WindowInactive),
            7 => Ok(CancelReason::Unmounted),
            8 => Ok(CancelReason::Programmatic),
            9 => Ok(CancelReason::Interrupted),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn color_event(&mut self) -> Result<Event, DecodeError> {
        let event = match self.tag()? {
            0 => Event::Observed(self.color_snapshot()?),
            1 => Event::Started(self.color_snapshot()?),
            2 => Event::Preview(self.color_snapshot()?),
            3 => Event::Committed(self.color_source()?, self.color_snapshot()?),
            4 => Event::Cancelled(self.color_cancel_reason()?, self.color_snapshot()?),
            _ => return Err(DecodeError::Malformed),
        };
        if event.is_valid() {
            Ok(event)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn color_revision_guard(&mut self) -> Result<Option<i64>, DecodeError> {
        match self.tag()? {
            0 => Ok(None),
            1 => Ok(Some(self.int()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn color_command(&mut self) -> Result<Command, DecodeError> {
        let command = match self.tag()? {
            0 => Command::Set {
                value: self.color_value()?,
                if_revision: self.color_revision_guard()?,
            },
            1 => Command::Reset {
                if_revision: self.color_revision_guard()?,
            },
            2 => Command::Cancel,
            3 => Command::Focus(self.color_field()?),
            4 => Command::ReadSnapshot,
            _ => return Err(DecodeError::Malformed),
        };
        if command.is_valid() {
            Ok(command)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn color_error(&mut self) -> Result<Error, DecodeError> {
        match self.tag()? {
            0 => Ok(Error::NotMounted),
            1 => Ok(Error::Closed),
            2 => Ok(Error::StaleColorInput),
            3 => Ok(Error::StaleRevision),
            4 => Ok(Error::StaleInteraction),
            5 => Ok(Error::Disabled),
            6 => Ok(Error::ReadOnly),
            7 => Ok(Error::FocusBlocked),
            8 => Ok(Error::Busy),
            9 => Ok(Error::InvalidValue),
            10 => Ok(Error::InvalidConfig),
            11 => Ok(Error::InvalidDraft),
            12 => Ok(Error::Composing),
            13 => Ok(Error::LimitExceeded),
            14 => Ok(Error::NativeFailure),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn color_response(&mut self) -> Result<Response, DecodeError> {
        match self.tag()? {
            0 => Ok(Response::Applied(self.color_snapshot()?)),
            1 => Ok(Response::Failed(self.color_error()?)),
            _ => Err(DecodeError::Malformed),
        }
    }
}

fn bounded<T>(
    bytes: &[u8],
    limit: usize,
    read: impl FnOnce(&mut Decoder<'_>) -> Result<T, DecodeError>,
) -> Result<T, DecodeError> {
    if bytes.len() > limit {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let value = read(&mut d)?;
    if d.remaining() == 0 {
        Ok(value)
    } else {
        Err(DecodeError::Malformed)
    }
}
pub fn decode_color_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    bounded(bytes, MAX_CONFIG_BYTES, |d| d.color_config())
}
pub fn decode_color_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    bounded(bytes, MAX_EVENT_BYTES, |d| d.color_event())
}
pub fn decode_color_command(bytes: &[u8]) -> Result<Command, DecodeError> {
    bounded(bytes, MAX_COMMAND_BYTES, |d| d.color_command())
}
pub fn decode_color_response(bytes: &[u8]) -> Result<Response, DecodeError> {
    bounded(bytes, MAX_EVENT_BYTES, |d| d.color_response())
}
