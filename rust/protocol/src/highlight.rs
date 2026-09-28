//! Subtree highlight configuration. No mounted capability is advertised yet.
//! Byte ranges index ordinary text; active indices/offsets count matches.
use binprot::macros::BinProtWrite;

pub const MAX_SPECS: usize = 16;
pub const MAX_RANGES: usize = 4096;
pub const MAX_QUERY_BYTES: usize = 4096;
pub const MAX_CONFIG_BYTES: usize = 262144;

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
