//! Locale/configuration contracts for the retained calendar; no native owner yet.
pub use crate::calendar::{Constraints, Date, Mode, Month, Range, RangePolicy, Selection};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 24576;
pub const MAX_LABEL_BYTES: usize = 128;
pub const MAX_EVENT_BYTES: usize = 128;
pub const MAX_COMMAND_BYTES: usize = 64;

fn valid_label(text: &str, maximum: usize) -> bool {
    text.len() <= maximum
        && !text.bytes().any(|b| b.is_ascii_control())
        && !text.trim_matches(' ').is_empty()
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Labels {
    pub months: Vec<String>,
    pub weekdays: Vec<String>,
    pub short_weekdays: Vec<String>,
    pub previous: String,
    pub next: String,
    pub choose_month: String,
    pub choose_year: String,
    pub today: String,
    pub clear: String,
}
impl Labels {
    pub fn english() -> Self {
        fn strings(values: &[&str]) -> Vec<String> {
            values.iter().map(|s| (*s).into()).collect()
        }
        Self {
            months: strings(&[
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ]),
            weekdays: strings(&[
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
            ]),
            short_weekdays: strings(&["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]),
            previous: "Previous".into(),
            next: "Next".into(),
            choose_month: "Choose month".into(),
            choose_year: "Choose year".into(),
            today: "Today".into(),
            clear: "Clear".into(),
        }
    }
    pub fn is_valid(&self) -> bool {
        let list = |values: &[String], count| {
            values.len() == count && values.iter().all(|s| valid_label(s, MAX_LABEL_BYTES))
        };
        list(&self.months, 12)
            && list(&self.weekdays, 7)
            && list(&self.short_weekdays, 7)
            && [
                &self.previous,
                &self.next,
                &self.choose_month,
                &self.choose_year,
                &self.today,
                &self.clear,
            ]
            .iter()
            .all(|s| valid_label(s, MAX_LABEL_BYTES))
    }
    pub fn retained_bytes(&self) -> usize {
        self.months
            .iter()
            .chain(&self.weekdays)
            .chain(&self.short_weekdays)
            .chain([
                &self.previous,
                &self.next,
                &self.choose_month,
                &self.choose_year,
                &self.today,
                &self.clear,
            ])
            .map(|s| 32 + s.len())
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub mode: Mode,
    pub constraints: Constraints,
    pub first_weekday: i64,
    pub labels: Labels,
    pub today: Option<Date>,
    pub label: String,
    pub disabled: bool,
    pub read_only: bool,
    pub auto_focus: bool,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        (0..=6).contains(&self.first_weekday)
            && self.labels.is_valid()
            && valid_label(&self.label, 4096)
    }
    pub fn retained_bytes(&self) -> usize {
        256 + self.labels.retained_bytes()
            + self.label.len()
            + 8 * self.constraints.disabled_dates().len()
            + 16 * self.constraints.disabled_ranges().len()
            + 8 * self.constraints.disabled_weekdays().len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Presentation {
    Days,
    Months,
    Years,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Snapshot {
    pub revision: i64,
    pub mode: Mode,
    pub selection: Selection,
    pub selection_allowed: bool,
    pub month: Month,
    pub focused_date: Date,
    pub presentation: Presentation,
    pub focused: bool,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.revision >= 0
            && self.selection.fits(self.mode)
            && (self.selection != Selection::Empty || self.selection_allowed)
            && Month::from_date(self.focused_date) == self.month
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum SelectionError {
    WrongMode,
    UnsupportedDate,
    DisabledDate,
    DisabledInterior,
}
impl From<crate::calendar::SelectionError> for SelectionError {
    fn from(error: crate::calendar::SelectionError) -> Self {
        match error {
            crate::calendar::SelectionError::WrongMode => Self::WrongMode,
            crate::calendar::SelectionError::DisabledDate => Self::DisabledDate,
            crate::calendar::SelectionError::DisabledInterior => Self::DisabledInterior,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Event {
    Observed(Snapshot),
    Changed(Snapshot),
    Selected(Snapshot),
    Rejected(SelectionError, Snapshot),
}
impl Event {
    pub fn snapshot(&self) -> &Snapshot {
        match self {
            Self::Observed(s) | Self::Changed(s) | Self::Selected(s) | Self::Rejected(_, s) => s,
        }
    }
    pub fn is_valid(&self) -> bool {
        let s = self.snapshot();
        s.is_valid()
            && match self {
                Self::Observed(_) => true,
                Self::Changed(_) | Self::Rejected(_, _) => s.revision > 0,
                Self::Selected(_) => {
                    s.revision > 0 && s.selection_allowed && s.selection.is_complete()
                }
            }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Command {
    Replace {
        selection: Selection,
        if_revision: Option<i64>,
    },
    Clear {
        if_revision: Option<i64>,
    },
    ShowMonth(Month),
    MoveMonths(i64),
    FocusDate(Date),
    Focus,
    SetPresentation(Presentation),
    ReadSnapshot,
}
impl Command {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Replace { if_revision, .. } | Self::Clear { if_revision } => {
                if_revision.is_none_or(|r| r >= 0)
            }
            Self::MoveMonths(delta) => (-119987..=119987).contains(delta),
            Self::ShowMonth(_)
            | Self::FocusDate(_)
            | Self::Focus
            | Self::SetPresentation(_)
            | Self::ReadSnapshot => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    NotMounted,
    Closed,
    StaleInput,
    StaleRevision,
    LimitExceeded,
    Busy,
    NativeFailure,
    InvalidConfig,
    WrongMode,
    DisabledDate,
    DisabledInterior,
    FocusBlocked,
    Disabled,
    ReadOnly,
    InvalidValue,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Response {
    Applied(Snapshot),
    Failed(Error),
}
impl Response {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Applied(s) => s.is_valid(),
            Self::Failed(_) => true,
        }
    }
}
