//! Latest-request chart preparation. Weak mounted owners, exact worker tokens,
//! admitted workspace and retained plans; workers never await the UI.
use crate::{
    chart_paint::{self as paint, Layout, Prepared},
    chart_store::Snapshot,
};
use gpuio_protocol::chart_view::Config;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};

// Retained plans have an enforced capacity limit. Preparation also reserves a
// conservative allowance for reduction, command conversion and Lyon scratch.
// This is admission accounting, not an allocator/RSS bound on Lyon internals.
pub const MAX_RETAINED_BYTES: usize = 256 * 1024 * 1024;
pub const WORKSPACE_BYTES: usize = 192 * 1024 * 1024;
const FIXED_CHARGE: usize = 4096;
#[derive(Default)]
struct Budget(Arc<AtomicUsize>);
impl Budget {
    fn used_bytes(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
    fn reserve(&self, bytes: usize, limit: usize) -> Result<Charge, Error> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(bytes).filter(|n| *n <= limit)
            })
            .map_err(|_| Error::LimitExceeded)?;
        Ok(Charge {
            used: self.0.clone(),
            bytes,
        })
    }
}
struct Charge {
    used: Arc<AtomicUsize>,
    bytes: usize,
}
impl Charge {
    fn shrink(&mut self, bytes: usize) {
        assert!(bytes <= self.bytes);
        self.used.fetch_sub(self.bytes - bytes, Ordering::Relaxed);
        self.bytes = bytes;
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.used.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

pub const MAX_WORKERS: usize = 2;
pub const MAX_VIEWS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    LimitExceeded,
    Cancelled,
    Preparation(paint::Error),
}

#[derive(Clone)]
pub struct Request {
    pub observer: Option<gpui::WindowId>,
    pub snapshot: Arc<Snapshot>,
    pub config: Arc<Config>,
    pub layout: Layout,
    pub text: Option<crate::chart_label_metrics::Context>,
}
impl Request {
    fn equal(&self, other: &Self) -> bool {
        self.observer == other.observer
            && Arc::ptr_eq(&self.snapshot, &other.snapshot)
            && self.config.options == other.config.options
            && self.config.sampling == other.config.sampling
            && self.config.style == other.config.style
            && self.config.legend == other.config.legend
            && self.layout == other.layout
            && self.text == other.text
    }
}

pub struct Ready {
    pub snapshot: Arc<Snapshot>,
    pub config: Arc<Config>,
    pub layout: Layout,
    pub plan: Prepared,
    pub label_style: Option<crate::chart_label_metrics::LabelStyle>,
    _charge: Charge,
}

struct Entry {
    request: Request,
    serial: u64,
    completed: u64,
    running: bool,
    queued_at: Instant,
    cancel: Arc<AtomicBool>,
    ready: Option<Result<Ready, Error>>,
    closed: bool,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct Handle(Rc<RefCell<Entry>>);
impl Handle {
    pub fn update(&self, request: Request) -> Result<bool, Error> {
        let mut entry = self.0.borrow_mut();
        if entry.closed {
            return Err(Error::Closed);
        }
        if !request.config.is_valid() {
            return Err(Error::Preparation(paint::Error::InvalidInput));
        }
        if entry.request.equal(&request) {
            return Ok(false);
        }
        entry.serial = entry.serial.checked_add(1).ok_or(Error::LimitExceeded)?;
        entry.cancel.store(true, Ordering::Relaxed);
        entry.cancel = Arc::new(AtomicBool::new(false));
        entry.queued_at = Instant::now();
        entry.request = request;
        entry.ready = None;
        Ok(true)
    }
    pub fn take_ready(&self) -> Option<Result<Ready, Error>> {
        self.0.borrow_mut().ready.take()
    }
    pub fn is_pending(&self) -> bool {
        let entry = self.0.borrow();
        !entry.closed && entry.serial != entry.completed
    }
}

pub struct Work {
    id: u64,
    serial: u64,
    request: Request,
    cancel: Arc<AtomicBool>,
    output_charge: Result<Charge, Error>,
    workspace_charge: Charge,
    queued_at: Instant,
}
pub struct Completion {
    id: u64,
    serial: u64,
    pub observer: Option<gpui::WindowId>,
    cancel: Arc<AtomicBool>,
    _workspace: Charge,
    result: Result<Ready, Error>,
    pub queue_us: u128,
    pub prepare_us: u128,
}
impl Work {
    pub fn run(self) -> Completion {
        let queue_us = self.queued_at.elapsed().as_micros();
        let start = Instant::now();
        let result = self.output_charge.and_then(|mut charge| {
            let plan = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                paint::prepare_with_text(
                    &self.request.snapshot.data,
                    self.request.config.sampling,
                    &self.request.config.options,
                    &self.request.config.style,
                    self.request.layout,
                    self.request.text.as_ref(),
                    &self.cancel,
                )
            }))
            .unwrap_or(Err(paint::Error::NativeFailure))
            .map_err(|error| match error {
                paint::Error::Cancelled => Error::Cancelled,
                paint::Error::RenderLimit => Error::LimitExceeded,
                error => Error::Preparation(error),
            })?;
            if self.cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            charge.shrink(
                plan.retained_bytes() + FIXED_CHARGE + self.request.config.retained_bytes(),
            );
            Ok(Ready {
                snapshot: self.request.snapshot,
                config: self.request.config,
                layout: self.request.layout,
                plan,
                label_style: self.request.text.as_ref().map(|text| text.style.clone()),
                _charge: charge,
            })
        });
        Completion {
            id: self.id,
            serial: self.serial,
            observer: self.request.observer,
            result,
            cancel: self.cancel,
            _workspace: self.workspace_charge,
            queue_us,
            prepare_us: start.elapsed().as_micros(),
        }
    }
}

#[derive(Default)]
pub struct Pool {
    entries: BTreeMap<u64, Weak<RefCell<Entry>>>,
    running: BTreeMap<u64, Arc<AtomicBool>>,
    output_budget: Budget,
    workspace_budget: Budget,
    next: u64,
    cursor: u64,
    closed: bool,
    pub completed: usize,
    pub discarded: usize,
    pub peak_workers: usize,
}
impl Pool {
    #[cfg(feature = "native-canvas-tests")]
    pub(crate) fn hold_remaining_budget_for_test(&self) -> Box<dyn std::any::Any> {
        Box::new(
            self.output_budget
                .reserve(
                    MAX_RETAINED_BYTES - self.output_budget.used_bytes(),
                    MAX_RETAINED_BYTES,
                )
                .unwrap(),
        )
    }

    pub fn request(&mut self, request: Request) -> Result<Handle, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if !request.config.is_valid() {
            return Err(Error::Preparation(paint::Error::InvalidInput));
        }
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        if self.entries.len() == MAX_VIEWS {
            return Err(Error::LimitExceeded);
        }
        self.next = self.next.checked_add(1).ok_or(Error::LimitExceeded)?;
        let entry = Rc::new(RefCell::new(Entry {
            request,
            serial: 1,
            completed: 0,
            running: false,
            queued_at: Instant::now(),
            cancel: Arc::new(AtomicBool::new(false)),
            ready: None,
            closed: false,
        }));
        self.entries.insert(self.next, Rc::downgrade(&entry));
        Ok(Handle(entry))
    }
    pub fn next_work(&mut self) -> Option<Work> {
        if self.closed || self.running.len() == MAX_WORKERS {
            return None;
        }
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        let ids: Vec<_> = self.entries.keys().copied().collect();
        for id in ids
            .iter()
            .filter(|id| **id > self.cursor)
            .chain(ids.iter().filter(|id| **id <= self.cursor))
        {
            let Some(entry) = self.entries[id].upgrade() else {
                continue;
            };
            let mut entry = entry.borrow_mut();
            if entry.closed || entry.running || entry.serial == entry.completed {
                continue;
            }
            // Reserve preparation allowance before a worker can allocate. A failed
            // retained-plan admission still completes through the bounded worker
            // channel, so the mounted observer receives a typed failure.
            let workspace_charge = self
                .workspace_budget
                .reserve(WORKSPACE_BYTES, MAX_WORKERS * WORKSPACE_BYTES)
                .ok()?;
            let output_charge = self.output_budget.reserve(
                paint::MAX_BYTES + FIXED_CHARGE + entry.request.config.retained_bytes(),
                MAX_RETAINED_BYTES,
            );
            entry.running = true;
            self.running.insert(*id, entry.cancel.clone());
            self.peak_workers = self.peak_workers.max(self.running.len());
            self.cursor = *id;
            return Some(Work {
                id: *id,
                serial: entry.serial,
                request: entry.request.clone(),
                cancel: entry.cancel.clone(),
                output_charge,
                workspace_charge,
                queued_at: entry.queued_at,
            });
        }
        None
    }
    /// Completion returns only an observer whose current request was accepted.
    /// Obsolete completions are dropped, including their mesh/snapshot leases.
    pub fn complete(&mut self, completion: Completion) -> Option<gpui::WindowId> {
        // Numeric IDs are private to each pool. Only the exact issued token may
        // release a worker slot, including cancelled or superseded work.
        if !self
            .running
            .get(&completion.id)
            .is_some_and(|token| Arc::ptr_eq(token, &completion.cancel))
        {
            return None;
        }
        self.running.remove(&completion.id);
        self.completed = self.completed.saturating_add(1);
        let Some(entry) = self.entries.get(&completion.id).and_then(Weak::upgrade) else {
            self.discarded = self.discarded.saturating_add(1);
            return None;
        };
        let mut entry = entry.borrow_mut();
        entry.running = false;
        if self.closed || entry.closed || entry.serial != completion.serial {
            self.discarded = self.discarded.saturating_add(1);
            return None;
        }
        entry.completed = completion.serial;
        entry.ready = Some(completion.result);
        completion.observer
    }
    pub fn reserved_bytes(&self) -> usize {
        self.output_budget.used_bytes()
    }
    pub fn workspace_bytes(&self) -> usize {
        self.workspace_budget.used_bytes()
    }
    pub fn close(&mut self) {
        self.closed = true;
        for cancel in self.running.values() {
            cancel.store(true, Ordering::Relaxed);
        }
        for entry in self.entries.values().filter_map(Weak::upgrade) {
            let mut entry = entry.borrow_mut();
            entry.closed = true;
            entry.cancel.store(true, Ordering::Relaxed);
            entry.ready = None;
            entry.completed = entry.serial;
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
    use crate::chart_store;
    use binprot::BinProtWrite;
    use gpuio_protocol::{
        chart_data::{Contents, Data, Slice},
        chart_resource::Update,
    };
    fn request() -> Request {
        let data = Data {
            version: 3,
            bar_baselines: vec![],
            bar_backgrounds: vec![],
            contents: Contents::Pie(vec![Slice {
                id: 1,
                label: "One".into(),
                value: 1.,
            }]),
        };
        let mut bytes = vec![];
        data.binprot_write(&mut bytes).unwrap();
        let mut store = chart_store::Store::default();
        let id = store.create().unwrap();
        store
            .begin(Update {
                id,
                base: 0,
                revision: 1,
                generation: 1,
                bytes: bytes.len() as i64,
            })
            .unwrap();
        store.chunk(id, 1, 0, &bytes).unwrap();
        let work = store.publish(id, 1).unwrap();
        store.complete(work.run()).unwrap();
        Request {
            observer: None,
            snapshot: store.acquire(id).unwrap().snapshot().unwrap(),
            config: Arc::new(Config {
                version: -2,
                radar_labels: vec![],
                inspection_content: vec![],
                source: Some(id),
                label: "Chart".into(),
                legend: true,
                disabled: false,
                options: Default::default(),
                sampling: Default::default(),
                style: Default::default(),
            }),
            layout: Layout::new(200., 150., 1.).unwrap(),
            text: None,
        }
    }
    #[test]
    fn superseded_worker_stays_charged_and_only_latest_request_completes() {
        let mut pool = Pool::default();
        let request = request();
        let handle = pool.request(request.clone()).unwrap();
        let work = pool.next_work().unwrap();
        assert_eq!(pool.workspace_bytes(), WORKSPACE_BYTES);
        for i in 1..=1000 {
            let mut next = request.clone();
            next.layout = Layout::new(200. + i as f64, 150., 1.).unwrap();
            handle.update(next).unwrap();
        }
        assert!(pool.next_work().is_none());
        let completion = std::thread::spawn(move || work.run()).join().unwrap();
        assert!(matches!(completion.result, Err(Error::Cancelled)));
        assert_eq!(pool.workspace_bytes(), WORKSPACE_BYTES);
        pool.complete(completion);
        assert_eq!(pool.workspace_bytes(), 0);
        assert!(handle.take_ready().is_none());
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().ok().unwrap();
        assert_eq!(ready.layout, Layout::new(1200., 150., 1.).unwrap());
        let retained = pool.reserved_bytes();
        assert!(retained > 0);
        drop(handle);
        assert_eq!(pool.reserved_bytes(), retained);
        drop(ready);
        assert_eq!(pool.reserved_bytes(), 0);
        assert_eq!(pool.peak_workers, 1);
    }
    #[test]
    fn foreign_completion_cannot_release_another_pools_slot() {
        let mut first = Pool::default();
        let mut second = Pool::default();
        let first_handle = first.request(request()).unwrap();
        let _second_handle = second.request(request()).unwrap();
        let first_work = first.next_work().unwrap();
        let foreign = second.next_work().unwrap().run();
        first.complete(foreign);
        assert_eq!(first.running.len(), 1);
        assert_eq!(first.workspace_bytes(), WORKSPACE_BYTES);
        first.complete(first_work.run());
        assert!(first_handle.take_ready().unwrap().is_ok());
        assert!(first.running.is_empty());
    }
    #[test]
    fn owner_disposal_and_close_cancel_but_hold_admitted_work_until_reaped() {
        let mut pool = Pool::default();
        let mut handles = vec![];
        for _ in 0..MAX_VIEWS {
            handles.push(pool.request(request()).unwrap());
        }
        assert!(matches!(pool.request(request()), Err(Error::LimitExceeded)));
        let first = pool.next_work().unwrap();
        let second = pool.next_work().unwrap();
        assert!(pool.next_work().is_none());
        assert_eq!(pool.workspace_bytes(), 2 * WORKSPACE_BYTES);
        handles.clear();
        pool.close();
        assert!(pool.next_work().is_none());
        assert_eq!(pool.workspace_bytes(), 2 * WORKSPACE_BYTES);
        pool.complete(first.run());
        assert_eq!(pool.workspace_bytes(), WORKSPACE_BYTES);
        pool.complete(second.run());
        assert_eq!(pool.workspace_bytes(), 0);
        assert_eq!(pool.reserved_bytes(), 0);
        assert!(matches!(pool.request(request()), Err(Error::Closed)));
    }
    #[test]
    fn ordinal_config_charge_survives_owner_close_until_ready_reader_release() {
        use gpuio_protocol::chart_style::{Key, MAX_COLOR_DOMAIN, Ordinal};
        let mut pool = Pool::default();
        let mut request = request();
        Arc::make_mut(&mut request.config).style.ordinal = Some(Ordinal {
            domain: (1..=MAX_COLOR_DOMAIN as i64).map(Key::Slice).collect(),
            range: vec![0x2dd4bfff; 32],
            unknown: None,
        });
        let config_bytes = request.config.retained_bytes();
        assert!(config_bytes > FIXED_CHARGE);
        let handle = pool.request(request).unwrap();
        let work = pool.next_work().unwrap();
        assert_eq!(
            pool.reserved_bytes(),
            paint::MAX_BYTES + FIXED_CHARGE + config_bytes
        );
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().ok().unwrap();
        let retained = ready.plan.retained_bytes() + FIXED_CHARGE + config_bytes;
        assert_eq!(pool.reserved_bytes(), retained);
        drop(handle);
        pool.close();
        assert_eq!(pool.reserved_bytes(), retained);
        assert_eq!(pool.workspace_bytes(), 0);
        drop(ready);
        assert_eq!(pool.reserved_bytes(), 0);
    }
    #[test]
    fn retained_limit_reports_failure_and_recovers_after_reader_release() {
        let mut pool = Pool::default();
        let hold = pool
            .output_budget
            .reserve(MAX_RETAINED_BYTES, MAX_RETAINED_BYTES)
            .unwrap();
        let mut request = request();
        let handle = pool.request(request.clone()).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        assert!(matches!(
            handle.take_ready(),
            Some(Err(Error::LimitExceeded))
        ));
        drop(hold);
        request.layout = Layout::new(201., 150., 1.).unwrap();
        handle.update(request).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().ok().unwrap();
        pool.close();
        assert!(pool.reserved_bytes() > 0);
        drop(ready);
        assert_eq!(pool.reserved_bytes(), 0);
    }
}
