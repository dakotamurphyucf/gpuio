//! Application-owned carousel selection and revisioned native requests.
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const MAX_ITEMS: usize = 128;
pub const MIN_INTERVAL_MS: i64 = 1_000;
pub const MAX_INTERVAL_MS: i64 = 3_600_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Direction {
    Direct,
    Previous,
    Next,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub revision: i64,
    pub ids: Vec<String>,
    pub selected: Option<i64>,
    pub looping: bool,
    pub disabled: bool,
    pub axis: Axis,
    pub auto_advance_ms: Option<i64>,
    pub direction: Direction,
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256 && !id.contains('\0')
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.revision >= 0
            && self.ids.len() <= MAX_ITEMS
            && self.ids.iter().all(|id| valid_id(id))
            && self.ids.iter().collect::<BTreeSet<_>>().len() == self.ids.len()
            && self
                .auto_advance_ms
                .is_none_or(|ms| (MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&ms))
            && match self.selected {
                None => self.ids.is_empty(),
                Some(index) => index >= 0 && (index as usize) < self.ids.len(),
            }
    }
    /// A relative step always resolves against this accepted snapshot. No implicit
    /// selection mutation occurs in native code, including automatic advancement.
    pub fn target(&self, request: &Request) -> Option<usize> {
        if self.disabled {
            return None;
        }
        let index = usize::try_from(self.selected?).ok()?;
        if index >= self.ids.len() {
            return None;
        }
        let next = || {
            if index + 1 < self.ids.len() {
                Some(index + 1)
            } else if self.looping {
                Some(0)
            } else {
                None
            }
        };
        let result = match request {
            Request::Previous => index
                .checked_sub(1)
                .or_else(|| self.looping.then_some(self.ids.len() - 1)),
            Request::Next => next(),
            Request::First => Some(0),
            Request::Last => Some(self.ids.len() - 1),
            Request::Select(id) => self.ids.iter().position(|candidate| candidate == id),
            Request::AutoNext {
                revision,
                from,
                target,
            } => {
                if *revision != self.revision
                    || self.auto_advance_ms.is_none()
                    || from != &self.ids[index]
                {
                    return None;
                }
                next().filter(|next| &self.ids[*next] == target)
            }
        };
        result.filter(|target| *target != index)
    }
    pub fn automatic_request(&self) -> Option<Request> {
        self.auto_advance_ms?;
        let target = self.target(&Request::Next)?;
        Some(Request::AutoNext {
            revision: self.revision,
            from: self.ids[self.selected? as usize].clone(),
            target: self.ids[target].clone(),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Previous,
    Next,
    First,
    Last,
    Select(String),
    AutoNext {
        revision: i64,
        from: String,
        target: String,
    },
}
impl Request {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Previous | Self::Next | Self::First | Self::Last => true,
            Self::Select(id) => valid_id(id),
            Self::AutoNext {
                revision,
                from,
                target,
            } => *revision >= 0 && valid_id(from) && valid_id(target) && from != target,
        }
    }
}
