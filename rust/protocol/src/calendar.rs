//! Bounded civil-date values and selection rules. No clock, time zone or GPUI state.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub struct Date(i64);

fn leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
fn before_year(year: i64) -> i64 {
    let n = year - 1;
    n * 365 + n / 4 - n / 100 + n / 400
}
fn month_days(year: i64, month: i64) -> i64 {
    match month {
        2 => {
            if leap(year) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

impl Date {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(3_652_058);
    pub fn from_ordinal(ordinal: i64) -> Option<Self> {
        (0..=Self::MAX.0)
            .contains(&ordinal)
            .then_some(Self(ordinal))
    }
    pub fn ordinal(self) -> i64 {
        self.0
    }
    pub fn from_ymd(year: i64, month: i64, day: i64) -> Option<Self> {
        if !(1..=9999).contains(&year)
            || !(1..=12).contains(&month)
            || !(1..=month_days(year, month)).contains(&day)
        {
            return None;
        }
        let prior_months: i64 = (1..month).map(|m| month_days(year, m)).sum();
        Some(Self(before_year(year) + prior_months + day - 1))
    }
    /// At most fourteen year comparisons and twelve month steps.
    pub fn ymd(self) -> (i64, i64, i64) {
        let (mut low, mut high) = (1, 10000);
        while low + 1 < high {
            let middle = (low + high) / 2;
            if before_year(middle) <= self.0 {
                low = middle;
            } else {
                high = middle;
            }
        }
        let mut rest = self.0 - before_year(low);
        let mut month = 1;
        while rest >= month_days(low, month) {
            rest -= month_days(low, month);
            month += 1;
        }
        (low, month, rest + 1)
    }
    /// Sunday = 0, ..., Saturday = 6. 0001-01-01 was Monday.
    pub fn weekday(self) -> i64 {
        (self.0 + 1) % 7
    }
    pub fn shift(self, days: i64) -> Option<Self> {
        Self::from_ordinal(self.0.checked_add(days)?)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub struct Month(i64);
impl Month {
    pub fn new(year: i64, month: i64) -> Option<Self> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        Some(Self((year - 1) * 12 + month - 1))
    }
    pub fn from_index(index: i64) -> Option<Self> {
        (0..119988).contains(&index).then_some(Self(index))
    }
    pub fn index(self) -> i64 {
        self.0
    }
    pub fn from_date(date: Date) -> Self {
        let (year, month, _) = date.ymd();
        Self::new(year, month).expect("validated civil date")
    }
    pub fn year(self) -> i64 {
        self.0 / 12 + 1
    }
    pub fn month(self) -> i64 {
        self.0 % 12 + 1
    }
    pub fn first_day(self) -> Date {
        Date::from_ymd(self.year(), self.month(), 1).expect("validated civil month")
    }
    pub fn last_day(self) -> Date {
        Date::from_ymd(
            self.year(),
            self.month(),
            month_days(self.year(), self.month()),
        )
        .expect("validated civil month")
    }
    pub fn shift(self, months: i64) -> Option<Self> {
        Self::from_index(self.0.checked_add(months)?)
    }
    pub fn days(self, first_weekday: i64) -> Option<[Option<Date>; 42]> {
        if !(0..=6).contains(&first_weekday) {
            return None;
        }
        let first = self.first_day();
        let offset = (first.weekday() + 7 - first_weekday) % 7;
        Some(std::array::from_fn(|i| first.shift(i as i64 - offset)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Range {
    first: Date,
    last: Date,
}
impl Range {
    pub fn new(first: Date, last: Date) -> Option<Self> {
        (first <= last).then_some(Self { first, last })
    }
    pub fn first(self) -> Date {
        self.first
    }
    pub fn last(self) -> Date {
        self.last
    }
    pub fn contains(self, date: Date) -> bool {
        self.first <= date && date <= self.last
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Mode {
    Single,
    Range,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Selection {
    Empty,
    Single(Date),
    RangeStart(Date),
    Range(Range),
}
impl Selection {
    pub fn fits(self, mode: Mode) -> bool {
        matches!(
            (self, mode),
            (Self::Empty, _)
                | (Self::Single(_), Mode::Single)
                | (Self::RangeStart(_) | Self::Range(_), Mode::Range)
        )
    }
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Single(_) | Self::Range(_))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum RangePolicy {
    EveryDay,
    EndpointsOnly,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Constraints {
    min: Date,
    max: Date,
    disabled_dates: Vec<Date>,
    disabled_ranges: Vec<Range>,
    disabled_weekdays: Vec<i64>,
    range_policy: RangePolicy,
}
impl Constraints {
    pub fn new(
        min: Date,
        max: Date,
        mut dates: Vec<Date>,
        mut ranges: Vec<Range>,
        mut weekdays: Vec<i64>,
        range_policy: RangePolicy,
    ) -> Option<Self> {
        if min > max
            || dates.len() > 512
            || ranges.len() > 128
            || weekdays.len() > 7
            || weekdays.iter().any(|day| !(0..=6).contains(day))
        {
            return None;
        }
        dates.sort_unstable();
        dates.dedup();
        weekdays.sort_unstable();
        weekdays.dedup();
        ranges.sort_by_key(|range| range.first);
        let mut merged: Vec<Range> = Vec::with_capacity(ranges.len());
        for next in ranges {
            if let Some(previous) = merged.last_mut()
                && next.first.0 - previous.last.0 <= 1
            {
                previous.last = previous.last.max(next.last);
            } else {
                merged.push(next);
            }
        }
        Some(Self {
            min,
            max,
            disabled_dates: dates,
            disabled_ranges: merged,
            disabled_weekdays: weekdays,
            range_policy,
        })
    }
    pub fn unrestricted() -> Self {
        Self::new(
            Date::MIN,
            Date::MAX,
            vec![],
            vec![],
            vec![],
            RangePolicy::EveryDay,
        )
        .expect("constant valid constraints")
    }
    pub fn min(&self) -> Date {
        self.min
    }
    pub fn max(&self) -> Date {
        self.max
    }
    pub fn disabled_dates(&self) -> &[Date] {
        &self.disabled_dates
    }
    pub fn disabled_ranges(&self) -> &[Range] {
        &self.disabled_ranges
    }
    pub fn disabled_weekdays(&self) -> &[i64] {
        &self.disabled_weekdays
    }
    pub fn range_policy(&self) -> RangePolicy {
        self.range_policy
    }
    pub fn allows(&self, date: Date) -> bool {
        self.min <= date
            && date <= self.max
            && self.disabled_dates.binary_search(&date).is_err()
            && !self
                .disabled_ranges
                .iter()
                .any(|range| range.contains(date))
            && !self.disabled_weekdays.contains(&date.weekday())
    }
    pub fn allows_selection(&self, selection: Selection, mode: Mode) -> bool {
        if !selection.fits(mode) {
            return false;
        }
        match selection {
            Selection::Empty => true,
            Selection::Single(date) | Selection::RangeStart(date) => self.allows(date),
            Selection::Range(range) => {
                self.allows(range.first)
                    && self.allows(range.last)
                    && (self.range_policy == RangePolicy::EndpointsOnly
                        || (!self.disabled_dates.iter().any(|&date| range.contains(date))
                            && !self.disabled_ranges.iter().any(|disabled| {
                                disabled.first <= range.last && disabled.last >= range.first
                            })
                            && !self.disabled_weekdays.iter().any(|&weekday| {
                                (weekday + 7 - range.first.weekday()) % 7
                                    <= range.last.0 - range.first.0
                            })))
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    WrongMode,
    DisabledDate,
    DisabledInterior,
}
pub fn activate(
    selection: Selection,
    date: Date,
    mode: Mode,
    constraints: &Constraints,
) -> Result<Selection, SelectionError> {
    if !selection.fits(mode) {
        return Err(SelectionError::WrongMode);
    }
    if !constraints.allows(date) {
        return Err(SelectionError::DisabledDate);
    }
    let next = match (mode, selection) {
        (Mode::Single, _) => Selection::Single(date),
        (Mode::Range, Selection::RangeStart(first)) if date >= first => {
            Selection::Range(Range { first, last: date })
        }
        (Mode::Range, _) => Selection::RangeStart(date),
    };
    if constraints.allows_selection(next, mode) {
        Ok(next)
    } else if matches!(next, Selection::Range(range) if !constraints.allows(range.first)) {
        Err(SelectionError::DisabledDate)
    } else {
        Err(SelectionError::DisabledInterior)
    }
}
