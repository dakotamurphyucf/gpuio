//! Retained calendar policy and event sequencing, independent of GPUI painting.
//! The adapter checks live window/node leases and publishes returned events in order.
use gpuio_protocol::{calendar, calendar_input::*};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Allowed,
    Blocked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Activate(Date),
    Clear,
    MoveDays(i64),
    ShowMonth(Month),
    MoveMonths(i64),
    ChooseMonth(Month),
    ChooseYear(i64),
    Reveal(Date),
    SetPresentation(Presentation),
}
pub struct Outcome {
    pub events: Vec<Event>,
    pub response: Response,
}
pub struct State {
    config: Arc<Config>,
    selection: Selection,
    month: Month,
    focused_date: Date,
    presentation: Presentation,
    focused: bool,
    revision: i64,
}

impl State {
    /// Initial month is explicit. The cursor prefers a selection endpoint or
    /// supplied today inside that month, otherwise the month's first day.
    pub fn new(config: Arc<Config>, selection: Selection, month: Month) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        check_selection(&config, selection)?;
        Self::from_retained(config, selection, month)
    }
    /// Restore a tree-admitted seed whose constraints may have changed within
    /// the same atomic transaction before the native owner was constructed.
    pub(crate) fn from_retained(
        config: Arc<Config>,
        selection: Selection,
        month: Month,
    ) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if !selection.fits(config.mode) {
            return Err(Error::WrongMode);
        }
        let selected_date = match selection {
            Selection::Empty => None,
            Selection::Single(date) | Selection::RangeStart(date) => Some(date),
            Selection::Range(range) => Some(range.first()),
        };
        let focused_date = selected_date
            .filter(|&d| Month::from_date(d) == month)
            .or_else(|| config.today.filter(|&d| Month::from_date(d) == month))
            .unwrap_or_else(|| month.first_day());
        Ok(Self {
            config,
            selection,
            month,
            focused_date,
            presentation: Presentation::Days,
            focused: false,
            revision: 0,
        })
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            mode: self.config.mode,
            selection: self.selection,
            selection_allowed: self
                .config
                .constraints
                .allows_selection(self.selection, self.config.mode),
            month: self.month,
            focused_date: self.focused_date,
            presentation: self.presentation,
            focused: self.focused,
        }
    }
    fn reserve(&self, count: i64) -> Result<(), Error> {
        self.revision
            .checked_add(count)
            .map(|_| ())
            .ok_or(Error::LimitExceeded)
    }
    fn advance(&mut self) -> Snapshot {
        self.revision += 1; // Reserved before mutation or native focus side effects.
        let snapshot = self.snapshot();
        debug_assert!(snapshot.is_valid());
        snapshot
    }
    fn access(&self, access: Access, editing: bool) -> Result<(), Error> {
        if self.config.disabled {
            return Err(Error::Disabled);
        }
        if access == Access::Blocked {
            return Err(Error::FocusBlocked);
        }
        if editing && self.config.read_only {
            return Err(Error::ReadOnly);
        }
        Ok(())
    }
    fn show_month(&mut self, month: Month) {
        let day = self.focused_date.ymd().2.min(month.last_day().ymd().2);
        self.focused_date =
            Date::from_ymd(month.year(), month.month(), day).expect("clamped civil day");
        self.month = month;
    }
    pub fn configure(&mut self, config: Arc<Config>) -> Result<Vec<Event>, Error> {
        if !config.is_valid() || config.mode != self.config.mode {
            return Err(Error::InvalidConfig);
        }
        if config == self.config {
            return Ok(vec![]);
        }
        self.reserve(1)?;
        self.config = config;
        Ok(vec![Event::Observed(self.advance())])
    }
    /// Report actual platform focus, including cleanup after hiding/disabling.
    pub fn observe_focus(&mut self, focused: bool) -> Result<Vec<Event>, Error> {
        if self.focused == focused {
            return Ok(vec![]);
        }
        self.reserve(1)?;
        self.focused = focused;
        Ok(vec![Event::Changed(self.advance())])
    }
    pub fn native(&mut self, action: Action, access: Access) -> Result<Vec<Event>, Error> {
        self.access(
            access,
            matches!(action, Action::Activate(_) | Action::Clear),
        )?;
        self.reserve(if matches!(action, Action::Activate(_)) {
            2
        } else {
            1
        })?;
        let before = self.snapshot();
        match action {
            Action::Activate(date) => {
                match calendar::activate(
                    self.selection,
                    date,
                    self.config.mode,
                    &self.config.constraints,
                ) {
                    Ok(selection) => self.selection = selection,
                    Err(error) => return Ok(vec![Event::Rejected(error.into(), self.advance())]),
                }
                self.focused_date = date;
                self.month = Month::from_date(date);
                self.presentation = Presentation::Days;
            }
            Action::Clear => self.selection = Selection::Empty,
            Action::MoveDays(delta) => {
                let date = self.focused_date.shift(delta).ok_or(Error::InvalidValue)?;
                self.focused_date = date;
                self.month = Month::from_date(date);
            }
            Action::ShowMonth(month) => self.show_month(month),
            Action::ChooseMonth(month) => {
                self.show_month(month);
                self.presentation = Presentation::Days;
            }
            Action::ChooseYear(year) => {
                let month = Month::new(year, self.month.month()).ok_or(Error::InvalidValue)?;
                self.show_month(month);
                self.presentation = Presentation::Months;
            }
            Action::Reveal(date) => {
                self.focused_date = date;
                self.month = Month::from_date(date);
                self.presentation = Presentation::Days;
            }
            Action::MoveMonths(delta) => {
                let month = self.month.shift(delta).ok_or(Error::InvalidValue)?;
                self.show_month(month);
            }
            Action::SetPresentation(presentation) => self.presentation = presentation,
        }
        if self.snapshot() == before {
            return Ok(vec![]);
        }
        let complete = self.selection != before.selection && self.selection.is_complete();
        let mut events = vec![Event::Changed(self.advance())];
        if complete {
            events.push(Event::Selected(self.advance()));
        }
        Ok(events)
    }
    /// The Rust-only callback checks live native gates, focuses the handle and
    /// confirms success. It must not reenter this owner or call OCaml. A failed
    /// callback must leave platform focus unchanged. Only Focus/FocusDate call it.
    pub fn execute(
        &mut self,
        command: &Command,
        focus: impl FnOnce() -> Result<(), Error>,
    ) -> Outcome {
        match self.command(command, focus) {
            Ok(events) => Outcome {
                events,
                response: Response::Applied(self.snapshot()),
            },
            Err(error) => Outcome {
                events: vec![],
                response: Response::Failed(error),
            },
        }
    }
    fn command(
        &mut self,
        command: &Command,
        focus: impl FnOnce() -> Result<(), Error>,
    ) -> Result<Vec<Event>, Error> {
        if !command.is_valid() {
            return Err(Error::InvalidValue);
        }
        let guard = match command {
            Command::Replace { if_revision, .. } | Command::Clear { if_revision } => *if_revision,
            _ => None,
        };
        if guard.is_some_and(|revision| revision != self.revision) {
            return Err(Error::StaleRevision);
        }
        if matches!(command, Command::ReadSnapshot) {
            return Ok(vec![]);
        }
        self.reserve(1)?;
        let before = self.snapshot();
        match command {
            Command::Replace { selection, .. } => {
                check_selection(&self.config, *selection)?;
                self.selection = *selection;
            }
            Command::Clear { .. } => self.selection = Selection::Empty,
            Command::ShowMonth(month) => self.show_month(*month),
            Command::MoveMonths(delta) => {
                let month = self.month.shift(*delta).ok_or(Error::InvalidValue)?;
                self.show_month(month);
            }
            Command::FocusDate(date) => {
                if self.config.disabled {
                    return Err(Error::FocusBlocked);
                }
                focus()?;
                self.focused = true;
                self.focused_date = *date;
                self.month = Month::from_date(*date);
                self.presentation = Presentation::Days;
            }
            Command::Focus => {
                if self.config.disabled {
                    return Err(Error::FocusBlocked);
                }
                focus()?;
                self.focused = true;
            }
            Command::SetPresentation(presentation) => self.presentation = *presentation,
            Command::ReadSnapshot => unreachable!("read handled before reservation"),
        }
        if self.snapshot() == before {
            Ok(vec![])
        } else {
            Ok(vec![Event::Observed(self.advance())])
        }
    }
}

fn check_selection(config: &Config, selection: Selection) -> Result<(), Error> {
    if !selection.fits(config.mode) {
        return Err(Error::WrongMode);
    }
    if config.constraints.allows_selection(selection, config.mode) {
        return Ok(());
    }
    match selection {
        Selection::Range(range)
            if config.constraints.allows(range.first())
                && config.constraints.allows(range.last()) =>
        {
            Err(Error::DisabledInterior)
        }
        Selection::Empty
        | Selection::Single(_)
        | Selection::RangeStart(_)
        | Selection::Range(_) => Err(Error::DisabledDate),
    }
}

#[cfg(test)]
#[path = "calendar_state_test.rs"]
mod tests;
