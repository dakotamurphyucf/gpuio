//! Scope-owned, latest-request highlight work. Pool/handles live on the native
//! thread; Work and Completion can cross threads without GPUI/OCaml callbacks.
use crate::{
    highlight_projection::{self as projection, Matches, Projection},
    highlight_search,
};
use gpuio_protocol::highlight::Config;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{
        Arc, Weak as SyncWeak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

pub const MAX_SCOPES: usize = 128;
pub const MAX_WORKERS: usize = 2;
pub const MAX_RESERVED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    AdmissionLimit,
    EpochExhausted,
    Match(projection::Error),
    WorkerFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    Unchanged,
    Presentation,
    Rematch,
}

/// Opaque identity within one pool, used by the native service to route wakeups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopeId(u64);

#[derive(Default)]
struct Quota {
    used: AtomicUsize,
    closed: AtomicBool,
}
struct Charge {
    quota: Arc<Quota>,
    bytes: AtomicUsize,
}
impl Charge {
    fn shrink_to(&self, bytes: usize) {
        let previous = self.bytes.fetch_min(bytes, Ordering::Relaxed);
        self.quota
            .used
            .fetch_sub(previous.saturating_sub(bytes), Ordering::Relaxed);
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.quota
            .used
            .fetch_sub(self.bytes.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}
struct Data {
    source: Arc<Projection>,
    config: Arc<Config>,
    base_units: usize,
    charge: Charge,
}

fn reservation(source: &Projection, config: &Config) -> (usize, usize) {
    let queries = config.0.iter().filter(|s| s.query.is_some()).count();
    let ranges = config.0.iter().map(|s| s.ranges.len()).sum::<usize>();
    let query_bytes = config
        .0
        .iter()
        .filter_map(|s| s.query.as_ref())
        .map(|q| q.text.len())
        .sum::<usize>();
    let largest_query = config
        .0
        .iter()
        .filter_map(|s| s.query.as_ref())
        .map(|q| q.text.len())
        .max()
        .unwrap_or(0);
    let possible_spans = (source.source_bytes() * queries + ranges * source.run_count())
        .min(projection::MAX_PAINT_SPANS);
    // Conservative logical admission units, not measured allocator RSS. Include
    // two config copies (matched/current presentation), metadata, worst bounded
    // output/map/vector capacity and query-sized origin/prefix scratch.
    let base = 8192
        + source.source_bytes()
        + source.run_count() * 256
        + source.group_count() * 128
        + config.0.len() * 256
        + ranges * 64
        + query_bytes * 2;
    let scratch = if queries == 0 {
        0
    } else {
        largest_query * 128
            + source
                .source_bytes()
                .min(highlight_search::MAX_STORED_MATCHES)
                * 64
    };
    (base, base + possible_spans * 512 + scratch)
}
fn admit(
    quota: &Arc<Quota>,
    source: Arc<Projection>,
    config: Arc<Config>,
) -> Result<Arc<Data>, Error> {
    if quota.closed.load(Ordering::Relaxed) {
        return Err(Error::Closed);
    }
    if !config.is_valid() {
        return Err(Error::Match(projection::Error::InvalidConfig));
    }
    let (base_units, bytes) = reservation(&source, &config);
    quota
        .used
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
            used.checked_add(bytes).filter(|n| *n <= MAX_RESERVED_BYTES)
        })
        .map_err(|_| Error::AdmissionLimit)?;
    Ok(Arc::new(Data {
        source,
        config,
        base_units,
        charge: Charge {
            quota: quota.clone(),
            bytes: AtomicUsize::new(bytes),
        },
    }))
}

pub struct Ready {
    pub matches: Matches,
    data: Arc<Data>,
}
impl Ready {
    pub fn source(&self) -> &Arc<Projection> {
        &self.data.source
    }
}
#[derive(Clone)]
pub enum Status {
    Pending,
    Ready(Arc<Ready>),
    Failed(Error),
}
struct Entry {
    epoch: i64,
    data: Option<Arc<Data>>,
    presentation: Option<Arc<Config>>,
    status: Status,
    cancel: Arc<AtomicBool>,
    running: Option<u64>,
    closed: bool,
}
impl Entry {
    fn fail(&mut self, error: Error) {
        self.cancel.store(true, Ordering::Relaxed);
        self.data = None;
        self.presentation = None;
        self.status = Status::Failed(error);
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct Handle {
    scope: ScopeId,
    entry: Rc<RefCell<Entry>>,
    quota: Arc<Quota>,
}
impl Handle {
    /// Native window teardown can cancel this entry even while another native
    /// owner still holds a handle. Retired paint readers keep their own charge.
    pub fn close(&self) {
        let mut entry = self.entry.borrow_mut();
        entry.closed = true;
        entry.fail(Error::Closed);
    }
    pub fn scope_id(&self) -> ScopeId {
        self.scope
    }
    pub fn epoch(&self) -> i64 {
        self.entry.borrow().epoch
    }
    pub fn status(&self) -> Status {
        self.entry.borrow().status.clone()
    }
    pub fn config(&self) -> Option<Arc<Config>> {
        self.entry.borrow().presentation.clone()
    }

    /// Invalid/admission-failed updates also retire old paint and cancel work.
    /// The caller retains its desired input and can explicitly retry admission.
    pub fn update(&self, source: Arc<Projection>, config: Arc<Config>) -> Result<Update, Error> {
        let mut entry = self.entry.borrow_mut();
        if entry.closed || self.quota.closed.load(Ordering::Relaxed) {
            return Err(Error::Closed);
        }
        if let Some(current) = &entry.data
            && Arc::ptr_eq(&source, &current.source)
            && entry
                .presentation
                .as_ref()
                .is_some_and(|p| Arc::ptr_eq(p, &config))
        {
            // Both immutable identities already passed admission/validation.
            return Ok(Update::Unchanged);
        }
        if config.is_valid()
            && let Some(current) = &entry.data
            && Arc::ptr_eq(&source, &current.source)
            && config.same_matchers(&current.config)
        {
            if entry.presentation.as_deref() == Some(config.as_ref()) {
                return Ok(Update::Unchanged);
            }
            entry.presentation = Some(config);
            return Ok(Update::Presentation);
        }
        let Some(epoch) = entry.epoch.checked_add(1) else {
            entry.fail(Error::EpochExhausted);
            return Err(Error::EpochExhausted);
        };
        entry.epoch = epoch;
        entry.cancel.store(true, Ordering::Relaxed);
        entry.cancel = Arc::new(AtomicBool::new(false));
        // Release entry-owned old data before admission. Paint/worker readers
        // keep their own charge until they actually release those references.
        entry.data = None;
        entry.status = Status::Pending;
        entry.presentation = None;
        match admit(&self.quota, source, config.clone()) {
            Ok(data) => {
                entry.data = Some(data);
                entry.presentation = Some(config);
                Ok(Update::Rematch)
            }
            Err(error) => {
                entry.fail(error);
                Err(error)
            }
        }
    }
}

struct Running {
    scope: u64,
    epoch: i64,
    cancel: Arc<AtomicBool>,
    live: SyncWeak<()>,
}
#[must_use = "dispatch with Work::run; dropping an undispatched job becomes WorkerFailed"]
pub struct Work {
    task: u64,
    scope: u64,
    epoch: i64,
    data: Arc<Data>,
    cancel: Arc<AtomicBool>,
    live: Arc<()>,
    quota: Arc<Quota>,
}
pub struct Completion {
    task: u64,
    scope: u64,
    epoch: i64,
    result: Result<Arc<Ready>, Error>,
    _live: Arc<()>,
    quota: Arc<Quota>,
}
impl Completion {
    pub fn scope_id(&self) -> ScopeId {
        ScopeId(self.scope)
    }
}
impl Work {
    pub fn run(self) -> Completion {
        let cancelled =
            || self.cancel.load(Ordering::Relaxed) || self.quota.closed.load(Ordering::Relaxed);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.data.source.find(&self.data.config, cancelled)
        }))
        .map_err(|_| Error::WorkerFailed)
        .and_then(|result| result.map_err(Error::Match));
        // Scratch and unused output reservations retire on the worker thread.
        // Input and actual output stay charged through the last paint reader.
        let spans = result
            .as_ref()
            .map_or(0, |m| m.spans.values().map(Vec::len).sum::<usize>());
        self.data
            .charge
            .shrink_to(self.data.base_units + spans * 512);
        let result = result.map(|matches| {
            Arc::new(Ready {
                matches,
                data: self.data,
            })
        });
        Completion {
            task: self.task,
            scope: self.scope,
            epoch: self.epoch,
            result,
            _live: self.live,
            quota: self.quota,
        }
    }
}

#[derive(Default)]
pub struct Pool {
    quota: Arc<Quota>,
    entries: BTreeMap<u64, Weak<RefCell<Entry>>>,
    running: BTreeMap<u64, Running>,
    next_scope: u64,
    next_task: u64,
    cursor: u64,
}
impl Pool {
    pub fn request(
        &mut self,
        source: Arc<Projection>,
        config: Arc<Config>,
    ) -> Result<Handle, Error> {
        if self.quota.closed.load(Ordering::Relaxed) {
            return Err(Error::Closed);
        }
        self.entries.retain(|_, e| e.strong_count() > 0);
        if self.entries.len() == MAX_SCOPES {
            return Err(Error::AdmissionLimit);
        }
        let scope = self
            .next_scope
            .checked_add(1)
            .ok_or(Error::AdmissionLimit)?;
        let data = admit(&self.quota, source, config.clone())?;
        let entry = Rc::new(RefCell::new(Entry {
            epoch: 1,
            data: Some(data),
            presentation: Some(config),
            status: Status::Pending,
            cancel: Arc::new(AtomicBool::new(false)),
            running: None,
            closed: false,
        }));
        self.entries.insert(scope, Rc::downgrade(&entry));
        self.next_scope = scope;
        Ok(Handle {
            scope: ScopeId(scope),
            entry,
            quota: self.quota.clone(),
        })
    }

    pub(crate) fn reap_abandoned(&mut self) {
        let dead: Vec<_> = self
            .running
            .iter()
            .filter(|(_, r)| r.live.strong_count() == 0)
            .map(|(id, _)| *id)
            .collect();
        for task in dead {
            let running = self.running.remove(&task).unwrap();
            if let Some(entry) = self.entries.get(&running.scope).and_then(Weak::upgrade) {
                let mut entry = entry.borrow_mut();
                if entry.running == Some(task) {
                    entry.running = None;
                    if !entry.closed && entry.epoch == running.epoch {
                        entry.fail(Error::WorkerFailed);
                    }
                }
            }
        }
    }

    pub fn next_work(&mut self) -> Option<Work> {
        self.reap_abandoned();
        if self.quota.closed.load(Ordering::Relaxed) || self.running.len() >= MAX_WORKERS {
            return None;
        }
        self.entries.retain(|_, e| e.strong_count() > 0);
        let ids: Vec<_> = self.entries.keys().copied().collect();
        for scope in ids
            .iter()
            .filter(|id| **id > self.cursor)
            .chain(ids.iter().filter(|id| **id <= self.cursor))
        {
            let Some(entry) = self.entries[scope].upgrade() else {
                continue;
            };
            let mut entry = entry.borrow_mut();
            if entry.closed || entry.running.is_some() || !matches!(entry.status, Status::Pending) {
                continue;
            }
            let Some(data) = entry.data.clone() else {
                continue;
            };
            let Some(task) = self.next_task.checked_add(1) else {
                entry.fail(Error::AdmissionLimit);
                continue;
            };
            self.next_task = task;
            let live = Arc::new(());
            self.running.insert(
                task,
                Running {
                    scope: *scope,
                    epoch: entry.epoch,
                    cancel: entry.cancel.clone(),
                    live: Arc::downgrade(&live),
                },
            );
            entry.running = Some(task);
            self.cursor = *scope;
            return Some(Work {
                task,
                scope: *scope,
                epoch: entry.epoch,
                data,
                cancel: entry.cancel.clone(),
                live,
                quota: self.quota.clone(),
            });
        }
        None
    }

    /// True only for a current completion published to a live scope. Foreign,
    /// removed, closed and stale completions cannot mutate current ready state.
    pub fn complete(&mut self, completion: Completion) -> bool {
        if !Arc::ptr_eq(&self.quota, &completion.quota) {
            return false;
        }
        let Some(running) = self.running.get(&completion.task) else {
            return false;
        };
        if running.scope != completion.scope || running.epoch != completion.epoch {
            return false;
        }
        self.running.remove(&completion.task);
        let Some(entry) = self.entries.get(&completion.scope).and_then(Weak::upgrade) else {
            return false;
        };
        let mut entry = entry.borrow_mut();
        if entry.running != Some(completion.task) {
            return false;
        }
        entry.running = None;
        if entry.closed
            || self.quota.closed.load(Ordering::Relaxed)
            || entry.epoch != completion.epoch
            || !matches!(entry.status, Status::Pending)
        {
            return false;
        }
        entry.status = match completion.result {
            Ok(ready) => Status::Ready(ready),
            Err(error) => Status::Failed(error),
        };
        true
    }

    pub fn reserved_bytes(&self) -> usize {
        self.quota.used.load(Ordering::Relaxed)
    }
    pub fn running_count(&self) -> usize {
        self.running.len()
    }
    pub fn close(&mut self) {
        self.quota.closed.store(true, Ordering::Relaxed);
        for work in self.running.values() {
            work.cancel.store(true, Ordering::Relaxed);
        }
        for entry in self.entries.values().filter_map(Weak::upgrade) {
            let mut entry = entry.borrow_mut();
            entry.closed = true;
            entry.fail(Error::Closed);
        }
        self.entries.clear();
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_epoch_cannot_accept_an_already_completed_result() {
        let mut pool = Pool::default();
        let config = Arc::new(Config(vec![]));
        let handle = pool
            .request(Arc::new(Projection::new(vec![]).unwrap()), config.clone())
            .unwrap();
        handle.entry.borrow_mut().epoch = i64::MAX;
        let late = pool.next_work().unwrap().run();
        assert_eq!(
            handle.update(Arc::new(Projection::new(vec![]).unwrap()), config),
            Err(Error::EpochExhausted)
        );
        assert!(!pool.complete(late));
        assert!(matches!(
            handle.status(),
            Status::Failed(Error::EpochExhausted)
        ));
        assert_eq!(pool.reserved_bytes(), 0);
    }

    #[test]
    fn dropping_pool_cancels_a_worker_even_while_a_handle_survives() {
        let mut pool = Pool::default();
        let handle = pool
            .request(
                Arc::new(Projection::new(vec![]).unwrap()),
                Arc::new(Config(vec![])),
            )
            .unwrap();
        let work = pool.next_work().unwrap();
        drop(pool);
        assert!(matches!(handle.status(), Status::Failed(Error::Closed)));
        let completion = std::thread::spawn(move || work.run()).join().unwrap();
        assert!(matches!(
            completion.result,
            Err(Error::Match(projection::Error::Cancelled))
        ));
        assert_eq!(handle.quota.used.load(Ordering::Relaxed), 0);
    }
}
