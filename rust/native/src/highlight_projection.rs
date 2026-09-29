//! Immutable text-to-paint mapping for one highlight scope. Collection from the
//! retained tree and native document layout is owned by the mounted adapter.
use crate::{document_store::Snapshot, highlight_search};
use gpuio_protocol::{NodeId, highlight::Config};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};

pub const MAX_GROUPS: usize = 4096;
pub const MAX_RUNS: usize = 16384;
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PAINT_SPANS: usize = 32768;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RunKey {
    pub node: NodeId,
    pub fragment: u32,
}

#[derive(Clone)]
pub enum Source {
    Text(Arc<str>),
    /// Retains the document store's original reservation; never flatten for find.
    Document(Arc<Snapshot>),
    /// The exact installed native page, with offsets local to this slice.
    DocumentSlice(DocumentSlice),
}

/// Validated UTF-8 byte boundaries into an immutable document-store snapshot.
/// Keeping the snapshot also keeps its original store reservation alive.
#[derive(Clone)]
pub struct DocumentSlice {
    snapshot: Arc<Snapshot>,
    bytes: Range<usize>,
}
impl Source {
    pub fn document_slice(
        snapshot: Arc<Snapshot>,
        bytes: Range<usize>,
    ) -> Result<Self, RangeError> {
        if bytes.start > bytes.end || bytes.end > snapshot.text.len() {
            return Err(RangeError::OutOfBounds);
        }
        if !snapshot.text.is_char_boundary(bytes.start)
            || !snapshot.text.is_char_boundary(bytes.end)
        {
            return Err(RangeError::ScalarBoundary);
        }
        Ok(Self::DocumentSlice(DocumentSlice { snapshot, bytes }))
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Document(s) => s.text.len(),
            Self::DocumentSlice(s) => s.bytes.len(),
        }
    }
    fn same_source(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => Arc::ptr_eq(a, b) || a == b,
            // A newer document revision must fence paints even if its bytes
            // happen to be equal. Retain the original snapshot reservation.
            (Self::Document(a), Self::Document(b)) => Arc::ptr_eq(a, b),
            (Self::DocumentSlice(a), Self::DocumentSlice(b)) => {
                Arc::ptr_eq(&a.snapshot, &b.snapshot) && a.bytes == b.bytes
            }
            _ => false,
        }
    }
    fn is_boundary(&self, offset: usize) -> bool {
        match self {
            Self::Text(text) => text.is_char_boundary(offset),
            Self::Document(s) => s.text.is_char_boundary(offset),
            Self::DocumentSlice(s) => {
                offset <= s.bytes.len() && s.snapshot.text.is_char_boundary(s.bytes.start + offset)
            }
        }
    }
    fn chunks(&self) -> Chunks<'_> {
        match self {
            Self::Text(text) => Chunks::Text(Some(text)),
            Self::Document(s) => Chunks::Document(s.text.chunks()),
            Self::DocumentSlice(s) => {
                Chunks::Document(s.snapshot.text.slice(s.bytes.clone()).chunks())
            }
        }
    }
}
enum Chunks<'a> {
    Text(Option<&'a str>),
    Document(ropey::iter::Chunks<'a>),
}
impl<'a> Iterator for Chunks<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Text(text) => text.take(),
            Self::Document(chunks) => chunks.next(),
        }
    }
}

pub struct Run {
    pub key: RunKey,
    pub source: Source,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ordinary,
    NativeDocument,
}
pub struct Group {
    pub kind: Kind,
    pub runs: Vec<Run>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    GroupLimit,
    RunLimit,
    ByteLimit,
    DuplicateRun,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeError {
    OutOfBounds,
    ScalarBoundary,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    InvalidRange {
        spec_index: usize,
        range_index: usize,
        reason: RangeError,
    },
    Cancelled,
    WorkLimit,
}

struct PreparedRun {
    key: RunKey,
    source: Source,
    bytes: Range<usize>,
}
struct PreparedGroup {
    runs: Vec<PreparedRun>,
    ordinary: Option<Range<usize>>,
}
type Piece = (RunKey, Range<usize>);
impl PreparedGroup {
    fn pieces(&self, bytes: Range<usize>) -> impl Iterator<Item = Piece> + Clone + '_ {
        let start = self
            .runs
            .partition_point(|run| run.bytes.end <= bytes.start);
        self.runs[start..]
            .iter()
            .take_while(move |run| run.bytes.start < bytes.end)
            .map(move |run| {
                (
                    run.key,
                    bytes.start.max(run.bytes.start) - run.bytes.start
                        ..bytes.end.min(run.bytes.end) - run.bytes.start,
                )
            })
    }
    fn is_boundary(&self, offset: usize) -> bool {
        let index = self.runs.partition_point(|run| run.bytes.end < offset);
        self.runs
            .get(index)
            .is_some_and(|run| run.source.is_boundary(offset - run.bytes.start))
    }
}

pub struct Projection {
    groups: Vec<PreparedGroup>,
    ordinary: Vec<usize>,
    ordinary_len: usize,
    source_bytes: usize,
    run_count: usize,
}
impl Projection {
    /// Inputs are in stable structural/source order, never frame visitation order.
    /// The collector excludes nested declarations and editable widget contents.
    pub fn new(groups: Vec<Group>) -> Result<Self, ProjectionError> {
        if groups.len() > MAX_GROUPS {
            return Err(ProjectionError::GroupLimit);
        }
        let mut out = Self {
            groups: Vec::new(),
            ordinary: Vec::new(),
            ordinary_len: 0,
            source_bytes: 0,
            run_count: 0,
        };
        let mut keys = BTreeSet::new();
        let mut visited_runs = 0;
        for group in groups {
            if group.runs.len() > MAX_RUNS - visited_runs {
                return Err(ProjectionError::RunLimit);
            }
            visited_runs += group.runs.len();
            let mut runs = Vec::new();
            let mut len = 0;
            for run in group.runs {
                if !keys.insert(run.key) {
                    return Err(ProjectionError::DuplicateRun);
                }
                let bytes = run.source.len();
                if bytes > MAX_SOURCE_BYTES - out.source_bytes {
                    return Err(ProjectionError::ByteLimit);
                }
                out.source_bytes += bytes;
                if bytes == 0 {
                    continue;
                }
                runs.push(PreparedRun {
                    key: run.key,
                    source: run.source,
                    bytes: len..len + bytes,
                });
                len += bytes;
                out.run_count += 1;
            }
            if runs.is_empty() {
                continue;
            }
            let ordinary = match group.kind {
                Kind::NativeDocument => None,
                Kind::Ordinary => {
                    if !out.ordinary.is_empty() {
                        out.ordinary_len += 1;
                    }
                    let range = out.ordinary_len..out.ordinary_len + len;
                    out.ordinary_len += len;
                    out.ordinary.push(out.groups.len());
                    Some(range)
                }
            };
            out.groups.push(PreparedGroup { runs, ordinary });
        }
        Ok(out)
    }

    pub fn source_bytes(&self) -> usize {
        self.source_bytes
    }

    /// Fence a native fragment's painter against its currently installed source.
    pub(crate) fn has_source(&self, key: RunKey, source: &Source) -> bool {
        self.groups
            .iter()
            .flat_map(|group| &group.runs)
            .any(|run| run.key == key && run.source.same_source(source))
    }

    /// Compare ordered source identities and run keys, excluding presentation.
    /// A mounted cache may keep its existing Arc when this returns true, so
    /// unrelated tree/style changes do not cancel matching or advance its epoch.
    pub fn same_source(&self, other: &Self) -> bool {
        self.source_bytes == other.source_bytes
            && self.ordinary_len == other.ordinary_len
            && self.run_count == other.run_count
            && self.groups.len() == other.groups.len()
            && self.groups.iter().zip(&other.groups).all(|(a, b)| {
                a.ordinary == b.ordinary
                    && a.runs.len() == b.runs.len()
                    && a.runs.iter().zip(&b.runs).all(|(a, b)| {
                        a.key == b.key && a.bytes == b.bytes && a.source.same_source(&b.source)
                    })
            })
    }
    pub fn ordinary_bytes(&self) -> usize {
        self.ordinary_len
    }
    pub fn run_count(&self) -> usize {
        self.run_count
    }
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    fn is_boundary(&self, offset: usize) -> bool {
        if offset == self.ordinary_len {
            return true;
        }
        let index = self
            .ordinary
            .partition_point(|g| self.groups[*g].ordinary.as_ref().unwrap().end < offset);
        let Some(group) = self.ordinary.get(index).map(|g| &self.groups[*g]) else {
            return false;
        };
        let start = group.ordinary.as_ref().unwrap().start;
        offset < start || group.is_boundary(offset - start)
    }

    fn explicit_pieces(&self, bytes: Range<usize>) -> impl Iterator<Item = Piece> + Clone + '_ {
        let start = self
            .ordinary
            .partition_point(|g| self.groups[*g].ordinary.as_ref().unwrap().end <= bytes.start);
        self.ordinary[start..]
            .iter()
            .take_while(move |g| self.groups[**g].ordinary.as_ref().unwrap().start < bytes.end)
            .flat_map(move |g| {
                let group = &self.groups[*g];
                let ordinary = group.ordinary.as_ref().unwrap();
                group.pieces(
                    bytes.start.max(ordinary.start) - ordinary.start
                        ..bytes.end.min(ordinary.end) - ordinary.start,
                )
            })
    }

    fn checked_range(
        &self,
        range: gpuio_protocol::highlight::Range,
    ) -> Result<Range<usize>, RangeError> {
        let start = usize::try_from(range.start_byte).map_err(|_| RangeError::OutOfBounds)?;
        let end = usize::try_from(range.end_byte).map_err(|_| RangeError::OutOfBounds)?;
        if start >= end || end > self.ordinary_len {
            return Err(RangeError::OutOfBounds);
        }
        if !self.is_boundary(start) || !self.is_boundary(end) {
            return Err(RangeError::ScalarBoundary);
        }
        Ok(start..end)
    }

    pub fn find(&self, config: &Config, cancelled: impl Fn() -> bool) -> Result<Matches, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if cancelled() {
            return Err(Error::Cancelled);
        }
        for (spec_index, spec) in config.0.iter().enumerate() {
            for (range_index, range) in spec.ranges.iter().enumerate() {
                if cancelled() {
                    return Err(Error::Cancelled);
                }
                self.checked_range(*range)
                    .map_err(|reason| Error::InvalidRange {
                        spec_index,
                        range_index,
                        reason,
                    })?;
            }
        }
        let mut out = Matches {
            counts: vec![Count::default(); config.0.len()],
            spans: BTreeMap::new(),
        };
        let mut storage = Storage {
            matches: 0,
            spans: 0,
            full: false,
        };
        let mut budget = highlight_search::Budget::default();
        for (spec_index, spec) in config.0.iter().enumerate() {
            if let Some(query) = &spec.query {
                let matcher =
                    highlight_search::Matcher::new(query).map_err(|_| Error::InvalidConfig)?;
                for group in &self.groups {
                    let found = matcher
                        .find(
                            group.runs.iter().flat_map(|r| r.source.chunks()),
                            &mut budget,
                            &cancelled,
                        )
                        .map_err(|e| match e {
                            highlight_search::Error::InvalidQuery => Error::InvalidConfig,
                            highlight_search::Error::Cancelled => Error::Cancelled,
                            highlight_search::Error::WorkLimit => Error::WorkLimit,
                        })?;
                    let base = out.counts[spec_index].total;
                    for (ordinal, range) in found.ranges.iter().enumerate() {
                        storage.store(
                            &mut out,
                            spec_index,
                            base + ordinal as i64,
                            group.pieces(range.clone()),
                        );
                    }
                    out.counts[spec_index].total += found.total as i64;
                    if !found.has_all_ranges() {
                        storage.full = true;
                    }
                    if storage.full {
                        budget.stop_storing();
                    }
                }
            }
            for range in &spec.ranges {
                if cancelled() {
                    return Err(Error::Cancelled);
                }
                let range = self
                    .checked_range(*range)
                    .expect("prevalidated immutable projection");
                let pieces = self.explicit_pieces(range);
                if pieces.clone().next().is_none() {
                    continue;
                }
                let ordinal = out.counts[spec_index].total;
                storage.store(&mut out, spec_index, ordinal, pieces);
                out.counts[spec_index].total += 1;
                if storage.full {
                    budget.stop_storing();
                }
            }
        }
        if cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(out)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    pub total: i64,
    pub stored: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub spec_index: usize,
    pub ordinal: i64,
    pub bytes: Range<usize>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Matches {
    pub counts: Vec<Count>,
    pub spans: BTreeMap<RunKey, Vec<Span>>,
}
struct Storage {
    matches: usize,
    spans: usize,
    full: bool,
}
impl Storage {
    fn store(
        &mut self,
        out: &mut Matches,
        spec_index: usize,
        ordinal: i64,
        pieces: impl Iterator<Item = Piece> + Clone,
    ) {
        if self.full {
            return;
        }
        let remaining = MAX_PAINT_SPANS - self.spans;
        let count = pieces.clone().take(remaining + 1).count();
        if self.matches == highlight_search::MAX_STORED_MATCHES || count > remaining {
            self.full = true;
            return;
        }
        debug_assert!(count > 0);
        self.matches += 1;
        self.spans += count;
        out.counts[spec_index].stored += 1;
        for (key, bytes) in pieces {
            out.spans.entry(key).or_default().push(Span {
                spec_index,
                ordinal,
                bytes,
            });
        }
    }
}
