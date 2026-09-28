//! Subtree highlight configuration. No mounted capability is advertised yet.
//! Byte ranges index ordinary text; active indices/offsets count matches.
use binprot::macros::BinProtWrite;

pub const MAX_SPECS: usize = 16;
pub const MAX_RANGES: usize = 4096;
pub const MAX_QUERY_BYTES: usize = 4096;
pub const MAX_CONFIG_BYTES: usize = 262144;
pub const MAX_OBSERVATION_BYTES: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Query {
    pub text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
}
impl Query {
    pub fn is_valid(&self) -> bool {
        !self.text.is_empty() && self.text.len() <= MAX_QUERY_BYTES && !self.text.contains('\0')
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Range {
    pub start_byte: i64,
    pub end_byte: i64,
}
impl Range {
    pub fn is_valid(&self) -> bool {
        self.start_byte >= 0 && self.end_byte > self.start_byte
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Appearance {
    pub color: i64,
    pub active_color: i64,
    pub radius: f64,
}
impl Appearance {
    pub fn is_valid(&self) -> bool {
        (0..=0xffff_ffff).contains(&self.color)
            && (0..=0xffff_ffff).contains(&self.active_color)
            && self.radius.is_finite()
            && (0.0..=64.0).contains(&self.radius)
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Spec {
    pub query: Option<Query>,
    pub ranges: Vec<Range>,
    pub appearance: Appearance,
    pub active_index: Option<i64>,
    pub match_index_offset: i64,
}
impl Spec {
    pub fn is_valid(&self) -> bool {
        (self.query.is_some() || !self.ranges.is_empty())
            && self.query.as_ref().is_none_or(Query::is_valid)
            && self.ranges.len() <= MAX_RANGES
            && self.ranges.iter().all(Range::is_valid)
            && self.appearance.is_valid()
            && self.active_index.is_none_or(|n| n >= 0)
            && self.match_index_offset >= 0
    }

    /// Cosmetic changes reuse prepared groups and match records.
    pub fn same_matcher(&self, other: &Self) -> bool {
        self.query == other.query && self.ranges == other.ranges
    }

    pub fn is_active(&self, local_index: i64) -> bool {
        local_index >= 0
            && self.active_index.is_some_and(|active| {
                active >= self.match_index_offset && active - self.match_index_offset == local_index
            })
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config(pub Vec<Spec>);
impl Config {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.0.len() * std::mem::size_of::<Spec>()
            + self
                .0
                .iter()
                .map(|s| {
                    s.ranges.len() * std::mem::size_of::<Range>()
                        + s.query.as_ref().map_or(0, |q| q.text.len())
                })
                .sum::<usize>()
    }
    pub fn is_valid(&self) -> bool {
        // These bounds also imply the 256-KiB encoded size bound: 16 queries,
        // 4096 pairs of two 9-byte integers, and bounded scalar metadata.
        self.0.len() <= MAX_SPECS
            && self.0.iter().all(Spec::is_valid)
            && self.0.iter().map(|s| s.ranges.len()).sum::<usize>() <= MAX_RANGES
    }

    pub fn same_matchers(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(&other.0).all(|(a, b)| a.same_matcher(b))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Count {
    pub total: i64,
    pub stored: i64,
}
impl Count {
    pub fn is_valid(&self) -> bool {
        self.total >= 0 && (0..=16384).contains(&self.stored) && self.stored <= self.total
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum RangeError {
    OutOfBounds,
    ScalarBoundary,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct InvalidRange {
    pub spec_index: i64,
    pub range_index: i64,
    pub reason: RangeError,
}
impl InvalidRange {
    pub fn is_valid(&self) -> bool {
        (0..16).contains(&self.spec_index) && (0..4096).contains(&self.range_index)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Limit {
    Source,
    Work,
    Admission,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Failure {
    SourceUnavailable,
    WorkerFailed,
    EpochExhausted,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum State {
    Pending,
    Ready(Vec<Count>),
    InvalidRange(InvalidRange),
    Capacity(Limit),
    Failed(Failure),
}
impl State {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Pending | Self::Capacity(_) | Self::Failed(_) => true,
            Self::InvalidRange(r) => r.is_valid(),
            Self::Ready(counts) => {
                counts.len() <= MAX_SPECS
                    && counts.iter().all(Count::is_valid)
                    && counts.iter().map(|c| c.stored).sum::<i64>() <= 16384
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Observation {
    pub epoch: i64,
    pub state: State,
}
impl Observation {
    pub fn is_valid(&self) -> bool {
        self.epoch > 0 && self.state.is_valid()
    }
    pub fn valid_for(&self, config: &Config) -> bool {
        self.is_valid()
            && match &self.state {
                State::Pending | State::Capacity(_) | State::Failed(_) => true,
                State::Ready(counts) => counts.len() == config.0.len(),
                State::InvalidRange(r) => config
                    .0
                    .get(r.spec_index as usize)
                    .is_some_and(|s| (r.range_index as usize) < s.ranges.len()),
            }
    }
    pub fn payload_bytes(&self) -> usize {
        match &self.state {
            State::Ready(counts) => counts.len() * std::mem::size_of::<Count>(),
            _ => 0,
        }
    }
}
