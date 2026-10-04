//! Bounded metadata for native calendar slots. Actual passive content uses the
//! retained child tree; these values never carry render closures or callbacks.
use binprot::macros::BinProtWrite;

pub const MAX_ITEMS: usize = 1024;
pub const MAX_DESCRIPTION_BYTES: usize = 65536;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Slot {
    Previous,
    Next,
    ChooseMonth,
    ChooseYear,
    Today,
    Clear,
    Day(i64),
    Month(i64),
    Year(i64),
    MonthHeading(i64),
    Weekday(i64, i64),
}
impl Slot {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Previous
            | Self::Next
            | Self::ChooseMonth
            | Self::ChooseYear
            | Self::Today
            | Self::Clear => true,
            Self::Day(date) => (0..=3652058).contains(&date),
            Self::Month(month) => (1..=12).contains(&month),
            Self::Year(year) => (1..=9999).contains(&year),
            Self::MonthHeading(month) => (0..119988).contains(&month),
            Self::Weekday(month, weekday) => {
                (0..119988).contains(&month) && (0..=6).contains(&weekday)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Item {
    pub slot: Slot,
    pub description: Option<String>,
}
impl Item {
    pub fn is_valid(&self) -> bool {
        self.slot.is_valid()
            && self.description.as_ref().is_none_or(|text| {
                !text.is_empty()
                    && text.len() <= 1024
                    && !text.bytes().all(|b| b == b' ')
                    && !text.bytes().any(|b| b < 32 || b == 127)
            })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub items: Vec<Item>,
}
impl Config {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.items.len() * std::mem::size_of::<Item>()
            + self
                .items
                .iter()
                .map(|item| item.description.as_ref().map_or(0, String::len))
                .sum::<usize>()
    }

    pub fn is_valid(&self) -> bool {
        if self.items.len() > MAX_ITEMS {
            return false;
        }
        if !self
            .items
            .windows(2)
            .all(|pair| pair[0].slot < pair[1].slot)
        {
            return false;
        }
        let mut bytes = 0;
        self.items.iter().all(|item| {
            bytes += item.description.as_ref().map_or(0, String::len);
            bytes <= MAX_DESCRIPTION_BYTES && item.is_valid()
        })
    }
}
