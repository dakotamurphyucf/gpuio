//! Logical calendar panes, independent of selection/cursor revisions and pixels.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Display {
    Days {
        first_month: i64,
        months: i64,
        first_weekday: i64,
    },
    Months {
        year: i64,
    },
    Years {
        first: i64,
        last: i64,
    },
}
impl Display {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Days {
                first_month,
                months,
                first_weekday,
            } => {
                (1..=12).contains(&months)
                    && (0..=119_988 - months).contains(&first_month)
                    && (0..=6).contains(&first_weekday)
            }
            Self::Months { year } => (1..=9999).contains(&year),
            Self::Years { first, last } => {
                (1..=9999).contains(&first)
                    && (first - 1) % 20 == 0
                    && last == (first + 19).min(9999)
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Observation {
    pub sequence: i64,
    pub display: Display,
}
impl Observation {
    pub fn is_valid(self) -> bool {
        self.sequence >= 0 && self.display.is_valid()
    }
}
