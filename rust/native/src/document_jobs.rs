//! Bounded, latest-snapshot document jobs. No GPUI callbacks occur in Work::run.
//! Mounted views own handles; dropping a handle cancels queued/running work.
use crate::{document_highlight as highlight, document_store::Snapshot};
use gpui_base::text::PreparedMarkdown;
use gpuio_protocol::document::Mode;
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

pub const MAX_WORKERS: usize = 2;
pub const MAX_VIEWS: usize = 128;
pub const MAX_RESERVED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    ResourceLimit,
    Cancelled,
    Parse,
    Highlight,
}

pub struct Charge {
    used: Arc<AtomicUsize>,
    bytes: usize,
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.used.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}
pub enum Prepared {
    Markdown {
        document: Box<PreparedMarkdown>,
        code: BTreeMap<(String, String), Vec<highlight::Run>>,
    },
    Code(Vec<highlight::Run>),
    Diff {
        document: crate::document_diff::Diff,
        runs: Vec<highlight::Run>,
    },
    Source(Error),
}
pub struct Ready {
    pub prepared: Prepared,
    pub charge: Charge,
    pub search: crate::document_search::Matches,
}
#[derive(Default, Clone, Copy, Debug)]
pub struct Measurements {
    pub queue_us: u128,
    pub parse_us: u128,
    pub highlight_us: u128,
    pub search_us: u128,
    pub source_bytes: usize,
}

#[derive(Clone)]
pub struct Request {
    pub observer: Option<gpui::WindowId>,
    pub snapshot: Arc<Snapshot>,
    pub mode: Mode,
    pub dark: bool,
    pub search: String,
}
impl Request {
    fn equal(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.snapshot, &other.snapshot)
            && self.mode == other.mode
            && self.dark == other.dark
            && self.search == other.search
    }
}
struct Entry {
    closed: bool,
    queued_at: Instant,
    serial: u64,
    request: Request,
    running: Option<u64>,
    completed: u64,
    cancel: Arc<AtomicBool>,
    ready: Option<Result<Ready, Error>>,
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
        if entry.request.equal(&request) {
            return Ok(false);
        }
        let serial = entry.serial.checked_add(1).ok_or(Error::ResourceLimit)?;
        entry.cancel.store(true, Ordering::Relaxed);
        entry.cancel = Arc::new(AtomicBool::new(false));
        entry.queued_at = Instant::now();
        entry.serial = serial;
        entry.request = request;
        entry.ready = None;
        Ok(true)
    }
    pub fn take_ready(&self) -> Option<Result<Ready, Error>> {
        self.0.borrow_mut().ready.take()
    }
    pub fn is_pending(&self) -> bool {
        let entry = self.0.borrow();
        entry.serial != entry.completed
    }
}

pub struct Work {
    queued_at: Instant,
    id: u64,
    serial: u64,
    request: Request,
    cancel: Arc<AtomicBool>,
    charge: Charge,
}
pub struct Completion {
    pub observer: Option<gpui::WindowId>,
    id: u64,
    serial: u64,
    pub result: Result<Ready, Error>,
    pub measurements: Measurements,
}
impl Work {
    pub fn run(self) -> Completion {
        let mut measurements = Measurements {
            queue_us: self.queued_at.elapsed().as_micros(),
            source_bytes: self.request.snapshot.text.len(),
            ..Default::default()
        };
        let cancelled = || self.cancel.load(Ordering::Relaxed);
        let search_started = Instant::now();
        let search = crate::document_search::find(
            &self.request.snapshot.text,
            &self.request.search,
            cancelled,
        );
        measurements.search_us = search_started.elapsed().as_micros();
        let prepared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if cancelled() {
                return Err(Error::Cancelled);
            }
            let max = match self.request.mode {
                Mode::Markdown => 65536,
                _ => highlight::MAX_HIGHLIGHT_BYTES,
            };
            if self.request.snapshot.text.len() > max {
                return Err(Error::ResourceLimit);
            }
            let text = self.request.snapshot.text.to_string();
            let value = match &self.request.mode {
                Mode::Markdown => {
                    let start = Instant::now();
                    let document = PreparedMarkdown::parse(
                        &text,
                        crate::document_markdown::extensions(Default::default()),
                    )
                    .map_err(|_| Error::Parse)?;
                    measurements.parse_us = start.elapsed().as_micros();
                    let start = Instant::now();
                    let mut code = BTreeMap::new();
                    let mut runs = 0;
                    for block in document.code_blocks() {
                        if cancelled() {
                            return Err(Error::Cancelled);
                        }
                        let language = block
                            .lang()
                            .map_or_else(|| "txt".to_string(), |s| s.to_string());
                        let text = block.code().to_string();
                        let styles =
                            highlight::highlight(&text, &language, self.request.dark, cancelled)
                                .map_err(|_| Error::Highlight)?;
                        runs += styles.len();
                        if runs > highlight::MAX_RUNS {
                            return Err(Error::ResourceLimit);
                        }
                        code.insert((language, text), styles);
                    }
                    measurements.highlight_us = start.elapsed().as_micros();
                    Prepared::Markdown {
                        document: Box::new(document),
                        code,
                    }
                }
                Mode::Code(language) => {
                    let start = Instant::now();
                    let runs = highlight::highlight(&text, language, self.request.dark, cancelled)
                        .map_err(|error| match error {
                            highlight::Error::Cancelled => Error::Cancelled,
                            highlight::Error::Limit => Error::ResourceLimit,
                            highlight::Error::Grammar => Error::Highlight,
                        })?;
                    measurements.highlight_us = start.elapsed().as_micros();
                    Prepared::Code(runs)
                }
                Mode::Diff => {
                    let start = Instant::now();
                    let diff = crate::document_diff::parse(&text, cancelled)
                        .ok_or(Error::ResourceLimit)?;
                    measurements.parse_us = start.elapsed().as_micros();
                    Prepared::Diff {
                        runs: crate::document_diff::highlights(&diff, self.request.dark),
                        document: diff,
                    }
                }
            };
            if cancelled() {
                Err(Error::Cancelled)
            } else {
                Ok(value)
            }
        }))
        .unwrap_or(Err(Error::Parse));
        let prepared = match prepared {
            Err(Error::ResourceLimit | Error::Parse | Error::Highlight) => {
                Ok(Prepared::Source(prepared.err().unwrap()))
            }
            other => other,
        };
        Completion {
            observer: self.request.observer,
            id: self.id,
            serial: self.serial,
            result: prepared.and_then(|prepared| {
                Ok(Ready {
                    prepared,
                    charge: self.charge,
                    search: search.ok_or(Error::Cancelled)?,
                })
            }),
            measurements,
        }
    }
}

#[derive(Default)]
pub struct Pool {
    entries: BTreeMap<u64, Weak<RefCell<Entry>>>,
    running: BTreeMap<u64, Arc<AtomicBool>>,
    next: u64,
    cursor: u64,
    notifications: Vec<gpui::WindowId>,
    reserved: Arc<AtomicUsize>,
    closed: bool,
    pub totals: Measurements,
    pub peak_workers: usize,
    pub discarded: usize,
    pub completed: usize,
    pub peak_reserved_bytes: usize,
}
impl Pool {
    pub fn request(&mut self, request: Request) -> Result<Handle, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        if self.entries.len() >= MAX_VIEWS {
            return Err(Error::ResourceLimit);
        }
        self.next = self.next.checked_add(1).ok_or(Error::ResourceLimit)?;
        let entry = Rc::new(RefCell::new(Entry {
            closed: false,
            queued_at: Instant::now(),
            serial: 1,
            request,
            running: None,
            completed: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            ready: None,
        }));
        self.entries.insert(self.next, Rc::downgrade(&entry));
        Ok(Handle(entry))
    }
    pub fn next_work(&mut self) -> Option<Work> {
        if self.closed || self.running.len() >= MAX_WORKERS {
            return None;
        }
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        let ids: Vec<_> = self.entries.keys().copied().collect();
        for id in ids
            .iter()
            .filter(|id| **id > self.cursor)
            .chain(ids.iter().filter(|id| **id <= self.cursor))
        {
            let entry = self.entries[id].upgrade()?;
            let mut entry = entry.borrow_mut();
            if entry.running.is_some() || entry.serial == entry.completed {
                continue;
            }
            // Conservative work/cache units, distinct from measured allocator RSS.
            let bytes = 4096
                + entry
                    .request
                    .snapshot
                    .text
                    .len()
                    .min(highlight::MAX_HIGHLIGHT_BYTES)
                    * 32;
            let reserved = self.reserved.load(Ordering::Relaxed);
            if bytes > MAX_RESERVED_BYTES - reserved {
                entry.completed = entry.serial;
                entry.ready = Some(Err(Error::ResourceLimit));
                if let Some(window) = entry.request.observer {
                    self.notifications.push(window);
                }
                continue;
            }
            self.reserved.fetch_add(bytes, Ordering::Relaxed);
            self.peak_reserved_bytes = self.peak_reserved_bytes.max(reserved + bytes);
            let charge = Charge {
                used: self.reserved.clone(),
                bytes,
            };
            entry.running = Some(entry.serial);
            self.running.insert(*id, entry.cancel.clone());
            self.peak_workers = self.peak_workers.max(self.running.len());
            let work = Work {
                queued_at: entry.queued_at,
                id: *id,
                serial: entry.serial,
                request: entry.request.clone(),
                cancel: entry.cancel.clone(),
                charge,
            };
            self.cursor = *id;
            return Some(work);
        }
        None
    }
    pub fn complete(&mut self, completion: Completion) {
        self.running.remove(&completion.id);
        self.completed += 1;
        self.totals.queue_us = self
            .totals
            .queue_us
            .saturating_add(completion.measurements.queue_us);
        self.totals.parse_us = self
            .totals
            .parse_us
            .saturating_add(completion.measurements.parse_us);
        self.totals.highlight_us = self
            .totals
            .highlight_us
            .saturating_add(completion.measurements.highlight_us);
        self.totals.search_us = self
            .totals
            .search_us
            .saturating_add(completion.measurements.search_us);
        self.totals.source_bytes = self
            .totals
            .source_bytes
            .saturating_add(completion.measurements.source_bytes);
        let Some(entry) = self.entries.get(&completion.id).and_then(Weak::upgrade) else {
            self.discarded += 1;
            return;
        };
        let mut entry = entry.borrow_mut();
        entry.running = None;
        if self.closed || entry.serial != completion.serial {
            self.discarded += 1;
            return;
        }
        entry.completed = completion.serial;
        entry.ready = Some(completion.result);
    }
    pub fn take_notifications(&mut self) -> Vec<gpui::WindowId> {
        std::mem::take(&mut self.notifications)
    }
    pub fn reserved_bytes(&self) -> usize {
        self.reserved.load(Ordering::Relaxed)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_store::Store;
    use gpuio_protocol::document::{Status, Update};

    fn request(text: &str) -> Request {
        let mut store = Store::default();
        let id = store.create().unwrap();
        store
            .begin(Update {
                id,
                base: 0,
                revision: 1,
                generation: 1,
                from_byte: 0,
                suffix_bytes: text.len() as i64,
                status: Status::Complete,
            })
            .unwrap();
        for (i, bytes) in text.as_bytes().chunks(262144).enumerate() {
            store.chunk(id, 1, i * 262144, bytes).unwrap();
        }
        store.publish(id, 1).unwrap();
        Request {
            observer: None,
            snapshot: store.acquire(id).unwrap().snapshot(),
            mode: Mode::Markdown,
            dark: true,
            search: String::new(),
        }
    }

    #[test]
    fn measurement_report() {
        let mut pool = Pool::default();
        let specifications = [
            ("**Streaming** λ paragraph.\n\n".repeat(100), Mode::Markdown),
            (
                "let answer = 42 (* λ *)\n".repeat(1000),
                Mode::Code("ml".into()),
            ),
            (
                "--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n".repeat(100),
                Mode::Diff,
            ),
            (format!("{}needle", "x".repeat(1024 * 1024)), Mode::Markdown),
        ];
        let handles: Vec<_> = specifications
            .iter()
            .map(|(text, mode)| {
                let mut value = request(text);
                value.mode = mode.clone();
                value.search = "needle".into();
                pool.request(value).unwrap()
            })
            .collect();
        loop {
            let work: Vec<_> = (0..MAX_WORKERS).filter_map(|_| pool.next_work()).collect();
            if work.is_empty() {
                break;
            }
            for work in work {
                pool.complete(work.run());
            }
        }
        assert_eq!(pool.peak_workers, 2);
        for handle in &handles {
            assert!(handle.take_ready().unwrap().is_ok());
        }
        assert_eq!(pool.reserved_bytes(), 0);
        eprintln!(
            "GPUIO_DOCUMENT_WORKER_METRICS {:?}; completed={}; peak_workers={}; peak_reserved_bytes={}",
            pool.totals, pool.completed, pool.peak_workers, pool.peak_reserved_bytes
        );
    }

    #[test]
    fn concurrency_and_stale_completion_are_bounded() {
        let mut pool = Pool::default();
        let a = pool.request(request("old")).unwrap();
        let b = pool.request(request("second")).unwrap();
        let c = pool.request(request("third")).unwrap();
        let first = pool.next_work().unwrap();
        let second = pool.next_work().unwrap();
        assert!(pool.next_work().is_none());
        for i in 0..1000 {
            a.update(request(&format!("latest {i}"))).unwrap();
        }
        pool.complete(first.run());
        assert!(a.take_ready().is_none());
        assert_eq!(pool.discarded, 1);
        pool.complete(second.run());
        assert!(b.take_ready().unwrap().is_ok());
        while let Some(work) = pool.next_work() {
            pool.complete(work.run());
        }
        match a.take_ready().unwrap().unwrap().prepared {
            Prepared::Markdown { document, .. } => {
                assert_eq!(document.source().as_ref(), "latest 999")
            }
            _ => panic!("wrong prepared kind"),
        }
        assert!(c.take_ready().unwrap().is_ok());
        assert_eq!(pool.reserved_bytes(), 0);
        assert_eq!(pool.completed, 4);
    }

    #[test]
    fn dropped_views_cancel_workers_and_close_rejects_updates() {
        let mut pool = Pool::default();
        let handle = pool.request(request("hello")).unwrap();
        let work = pool.next_work().unwrap();
        drop(handle);
        let completion = work.run();
        assert!(matches!(completion.result, Err(Error::Cancelled)));
        pool.complete(completion);
        assert_eq!(pool.reserved_bytes(), 0);
        let handle = pool.request(request("next")).unwrap();
        let work = pool.next_work().unwrap();
        pool.close();
        assert_eq!(handle.update(request("closed")), Err(Error::Closed));
        pool.complete(work.run());
        assert_eq!(pool.reserved_bytes(), 0);
    }

    #[test]
    fn per_document_and_aggregate_admission_keep_plain_fallback_explicit() {
        let mut pool = Pool::default();
        let large = pool.request(request(&"x".repeat(65537))).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        assert!(matches!(
            large.take_ready(),
            Some(Ok(Ready {
                prepared: Prepared::Source(Error::ResourceLimit),
                ..
            }))
        ));
        assert_eq!(pool.reserved_bytes(), 0);
        let text = format!("{}\n", "x".repeat(15000)).repeat(16);
        let request = Request {
            mode: Mode::Code("txt".into()),
            ..request(&text)
        };
        let handles: Vec<_> = (0..12)
            .map(|_| pool.request(request.clone()).unwrap())
            .collect();
        let mut retained = Vec::new();
        for handle in &handles {
            while handle.is_pending() {
                if let Some(work) = pool.next_work() {
                    pool.complete(work.run());
                } else {
                    break;
                }
            }
            if let Some(Ok(ready)) = handle.take_ready() {
                retained.push(ready);
            }
        }
        assert!(pool.reserved_bytes() <= MAX_RESERVED_BYTES);
        assert!(!retained.is_empty() && retained.len() < handles.len());
        drop(retained);
        drop(handles);
        drop(large);
        pool.close();
        assert_eq!(pool.reserved_bytes(), 0);
    }
}
