//! Declarative assigned-size selection. This contains no layout callbacks.
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const MAX_RULES: usize = 32;
pub const MAX_BRANCHES: usize = 16;
pub const MAX_BRANCH_BYTES: usize = 128;
pub const MAX_CONFIG_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Range {
    pub minimum: f64,
    pub maximum: Option<f64>,
}
impl Range {
    pub const ALL: Self = Self {
        minimum: 0.,
        maximum: None,
    };
    pub fn is_valid(&self) -> bool {
        self.minimum.is_finite()
            && self.minimum >= 0.
            && self
                .maximum
                .is_none_or(|n| n.is_finite() && n > self.minimum)
    }
    pub fn contains(&self, value: f64) -> bool {
        value.is_finite() && value >= self.minimum && self.maximum.is_none_or(|n| value < n)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Predicate {
    pub width: Range,
    pub height: Range,
}
impl Predicate {
    pub fn is_valid(&self) -> bool {
        self.width.is_valid() && self.height.is_valid()
    }
    pub fn matches(&self, width: f64, height: f64) -> bool {
        self.width.contains(width) && self.height.contains(height)
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Rule {
    pub condition: Predicate,
    pub branch: i64,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub generation: i64,
    pub branches: Vec<String>,
    pub default: i64,
    pub rules: Vec<Rule>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        let valid_index = |index: i64| index >= 0 && index < self.branches.len() as i64;
        self.generation > 0
            && !self.branches.is_empty()
            && self.branches.len() <= MAX_BRANCHES
            && self.branches.iter().all(|name| {
                !name.is_empty() && name.len() <= MAX_BRANCH_BYTES && !name.contains('\0')
            })
            && self.branches.iter().collect::<BTreeSet<_>>().len() == self.branches.len()
            && valid_index(self.default)
            && self.rules.len() <= MAX_RULES
            && self
                .rules
                .iter()
                .all(|rule| rule.condition.is_valid() && valid_index(rule.branch))
    }
    /// The config must already have passed admission. Selection does not allocate.
    /// Invalid assigned sizes or branch indices return None, never index a child.
    pub fn select(&self, width: f64, height: f64) -> Option<usize> {
        if !width.is_finite() || !height.is_finite() || width < 0. || height < 0. {
            return None;
        }
        let index = self
            .rules
            .iter()
            .find(|r| r.condition.matches(width, height))
            .map_or(self.default, |rule| rule.branch);
        usize::try_from(index)
            .ok()
            .filter(|index| *index < self.branches.len())
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.branches.capacity() * std::mem::size_of::<String>()
            + self.branches.iter().map(String::capacity).sum::<usize>()
            + self.rules.capacity() * std::mem::size_of::<Rule>()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub generation: i64,
    pub sequence: i64,
    pub branch: i64,
    pub width: f64,
    pub height: f64,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.generation > 0
            && self.sequence > 0
            && self.branch >= 0
            && self.branch < MAX_BRANCHES as i64
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width >= 0.
            && self.height >= 0.
    }
}
