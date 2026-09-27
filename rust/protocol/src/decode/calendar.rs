use super::{DecodeError, Decoder};
use crate::calendar_input::*;
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn calendar_month(&mut self) -> Result<Month, DecodeError> {
        Month::from_index(self.int()?).ok_or(DecodeError::Malformed)
    }
    fn calendar_mode(&mut self) -> Result<Mode, DecodeError> {
        match self.tag()? {
            0 => Ok(Mode::Single),
            1 => Ok(Mode::Range),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn calendar_presentation(&mut self) -> Result<Presentation, DecodeError> {
        match self.tag()? {
            0 => Ok(Presentation::Days),
            1 => Ok(Presentation::Months),
            2 => Ok(Presentation::Years),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn calendar_date(&mut self) -> Result<Date, DecodeError> {
        Date::from_ordinal(self.int()?).ok_or(DecodeError::Malformed)
    }
    fn calendar_range(&mut self) -> Result<Range, DecodeError> {
        Range::new(self.calendar_date()?, self.calendar_date()?).ok_or(DecodeError::Malformed)
    }
    pub(super) fn calendar_selection(&mut self) -> Result<Selection, DecodeError> {
        Ok(match self.tag()? {
            0 => Selection::Empty,
            1 => Selection::Single(self.calendar_date()?),
            2 => Selection::RangeStart(self.calendar_date()?),
            3 => Selection::Range(self.calendar_range()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn calendar_constraints(&mut self) -> Result<Constraints, DecodeError> {
        let min = self.calendar_date()?;
        let max = self.calendar_date()?;
        let count = self.count(512)?;
        let dates = (0..count)
            .map(|_| self.calendar_date())
            .collect::<Result<Vec<_>, _>>()?;
        let count = self.count(128)?;
        let ranges = (0..count)
            .map(|_| self.calendar_range())
            .collect::<Result<Vec<_>, _>>()?;
        let count = self.count(7)?;
        let weekdays = (0..count)
            .map(|_| self.int())
            .collect::<Result<Vec<_>, _>>()?;
        let policy = match self.tag()? {
            0 => RangePolicy::EveryDay,
            1 => RangePolicy::EndpointsOnly,
            _ => return Err(DecodeError::Malformed),
        };
        Constraints::new(min, max, dates, ranges, weekdays, policy).ok_or(DecodeError::Malformed)
    }
    fn calendar_labels(&mut self) -> Result<Labels, DecodeError> {
        fn list(d: &mut Decoder<'_>, expected: usize) -> Result<Vec<String>, DecodeError> {
            let count = d.count(expected)?;
            if count != expected {
                return Err(DecodeError::Malformed);
            }
            (0..count)
                .map(|_| d.bounded_text(MAX_LABEL_BYTES))
                .collect()
        }
        let labels = Labels {
            months: list(self, 12)?,
            weekdays: list(self, 7)?,
            short_weekdays: list(self, 7)?,
            previous: self.bounded_text(MAX_LABEL_BYTES)?,
            next: self.bounded_text(MAX_LABEL_BYTES)?,
            choose_month: self.bounded_text(MAX_LABEL_BYTES)?,
            choose_year: self.bounded_text(MAX_LABEL_BYTES)?,
            today: self.bounded_text(MAX_LABEL_BYTES)?,
            clear: self.bounded_text(MAX_LABEL_BYTES)?,
        };
        if labels.is_valid() {
            Ok(labels)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn calendar_config(&mut self) -> Result<Config, DecodeError> {
        let mode = self.calendar_mode()?;
        let config = Config {
            mode,
            constraints: self.calendar_constraints()?,
            first_weekday: self.int()?,
            labels: self.calendar_labels()?,
            today: self.option(Self::calendar_date)?,
            label: self.bounded_text(4096)?,
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
    fn calendar_snapshot(&mut self) -> Result<Snapshot, DecodeError> {
        let snapshot = Snapshot {
            revision: self.int()?,
            mode: self.calendar_mode()?,
            selection: self.calendar_selection()?,
            selection_allowed: self.boolean()?,
            month: self.calendar_month()?,
            focused_date: self.calendar_date()?,
            presentation: self.calendar_presentation()?,
            focused: self.boolean()?,
        };
        if snapshot.is_valid() {
            Ok(snapshot)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn calendar_event(&mut self) -> Result<Event, DecodeError> {
        let event = match self.tag()? {
            0 => Event::Observed(self.calendar_snapshot()?),
            1 => Event::Changed(self.calendar_snapshot()?),
            2 => Event::Selected(self.calendar_snapshot()?),
            3 => {
                let error = match self.tag()? {
                    0 => SelectionError::WrongMode,
                    1 => SelectionError::UnsupportedDate,
                    2 => SelectionError::DisabledDate,
                    3 => SelectionError::DisabledInterior,
                    _ => return Err(DecodeError::Malformed),
                };
                Event::Rejected(error, self.calendar_snapshot()?)
            }
            _ => return Err(DecodeError::Malformed),
        };
        if event.is_valid() {
            Ok(event)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn calendar_command(&mut self) -> Result<Command, DecodeError> {
        let command = match self.tag()? {
            0 => Command::Replace {
                selection: self.calendar_selection()?,
                if_revision: self.option(Self::int)?,
            },
            1 => Command::Clear {
                if_revision: self.option(Self::int)?,
            },
            2 => Command::ShowMonth(self.calendar_month()?),
            3 => Command::MoveMonths(self.int()?),
            4 => Command::FocusDate(self.calendar_date()?),
            5 => Command::Focus,
            6 => Command::SetPresentation(self.calendar_presentation()?),
            7 => Command::ReadSnapshot,
            _ => return Err(DecodeError::Malformed),
        };
        if command.is_valid() {
            Ok(command)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn calendar_response(&mut self) -> Result<Response, DecodeError> {
        Ok(match self.tag()? {
            0 => Response::Applied(self.calendar_snapshot()?),
            1 => Response::Failed(match self.tag()? {
                0 => Error::NotMounted,
                1 => Error::Closed,
                2 => Error::StaleInput,
                3 => Error::StaleRevision,
                4 => Error::LimitExceeded,
                5 => Error::Busy,
                6 => Error::NativeFailure,
                7 => Error::InvalidConfig,
                8 => Error::WrongMode,
                9 => Error::DisabledDate,
                10 => Error::DisabledInterior,
                11 => Error::FocusBlocked,
                12 => Error::Disabled,
                13 => Error::ReadOnly,
                14 => Error::InvalidValue,
                _ => return Err(DecodeError::Malformed),
            }),
            _ => return Err(DecodeError::Malformed),
        })
    }
}

fn standalone<T>(
    bytes: &[u8],
    maximum: usize,
    parse: impl FnOnce(&mut Decoder<'_>) -> Result<T, DecodeError>,
) -> Result<T, DecodeError> {
    if bytes.len() > maximum {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let value = parse(&mut d)?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
pub fn decode_calendar_selection(bytes: &[u8]) -> Result<Selection, DecodeError> {
    standalone(bytes, 19, |d| d.calendar_selection())
}
pub fn decode_calendar_constraints(bytes: &[u8]) -> Result<Constraints, DecodeError> {
    standalone(bytes, 7100, |d| d.calendar_constraints())
}
pub fn decode_calendar_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    standalone(bytes, MAX_CONFIG_BYTES, |d| d.calendar_config())
}
pub fn decode_calendar_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    standalone(bytes, MAX_EVENT_BYTES, |d| d.calendar_event())
}
pub fn decode_calendar_command(bytes: &[u8]) -> Result<Command, DecodeError> {
    standalone(bytes, MAX_COMMAND_BYTES, |d| d.calendar_command())
}
pub fn decode_calendar_response(bytes: &[u8]) -> Result<Response, DecodeError> {
    standalone(bytes, MAX_EVENT_BYTES, |d| d.calendar_response())
}
