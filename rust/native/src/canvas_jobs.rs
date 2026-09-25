//! Latest-request native mesh jobs. UI-owned handles and a weak pool keep one
//! desired snapshot per mounted canvas; background work never awaits the UI.
use crate::{
    canvas_plan::{self as plan, Budget, Plan, Quality},
    canvas_store::Snapshot,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

pub const MAX_WORKERS: usize = 2;
pub const MAX_VIEWS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    LimitExceeded,
    Cancelled,
    Preparation(plan::Error),
}

#[derive(Clone)]
pub struct Request {
    pub observer: Option<gpui::WindowId>,
    pub snapshot: Arc<Snapshot>,
    pub quality: Quality,
}
impl Request {
    fn equal(&self, other: &Self) -> bool {
        self.observer == other.observer
            && Arc::ptr_eq(&self.snapshot, &other.snapshot)
            && self.quality == other.quality
    }
}

pub struct Ready {
    pub snapshot: Arc<Snapshot>,
    pub quality: Quality,
    pub plan: Plan,
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
    budget: Budget,
    queued_at: Instant,
}
pub struct Completion {
    id: u64,
    serial: u64,
    pub observer: Option<gpui::WindowId>,
    result: Result<Ready, Error>,
    pub queue_us: u128,
    pub prepare_us: u128,
}
impl Work {
    pub fn run(self) -> Completion {
        let queue_us = self.queued_at.elapsed().as_micros();
        let start = Instant::now();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            plan::prepare(
                &self.request.snapshot.scene,
                self.request.quality,
                &self.budget,
                &self.cancel,
            )
        }))
        .unwrap_or(Err(plan::Error::Tessellation))
        .map_err(|error| match error {
            plan::Error::Cancelled => Error::Cancelled,
            plan::Error::LimitExceeded => Error::LimitExceeded,
            error => Error::Preparation(error),
        })
        .and_then(|plan| {
            if self.cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            Ok(Ready {
                snapshot: self.request.snapshot,
                quality: self.request.quality,
                plan,
            })
        });
        Completion {
            id: self.id,
            serial: self.serial,
            observer: self.request.observer,
            result,
            queue_us,
            prepare_us: start.elapsed().as_micros(),
        }
    }
}

#[derive(Default)]
pub struct Pool {
    entries: BTreeMap<u64, Weak<RefCell<Entry>>>,
    running: BTreeMap<u64, Arc<AtomicBool>>,
    budget: Budget,
    next: u64,
    cursor: u64,
    closed: bool,
    pub completed: usize,
    pub discarded: usize,
    pub peak_workers: usize,
}
impl Pool {
    pub fn request(&mut self, request: Request) -> Result<Handle, Error> {
        if self.closed {
            return Err(Error::Closed);
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
            entry.running = true;
            self.running.insert(*id, entry.cancel.clone());
            self.peak_workers = self.peak_workers.max(self.running.len());
            self.cursor = *id;
            return Some(Work {
                id: *id,
                serial: entry.serial,
                request: entry.request.clone(),
                cancel: entry.cancel.clone(),
                budget: self.budget.clone(),
                queued_at: entry.queued_at,
            });
        }
        None
    }
    /// Completion returns only an observer whose current request was accepted.
    /// Obsolete completions are dropped, including their mesh/snapshot leases.
    pub fn complete(&mut self, completion: Completion) -> Option<gpui::WindowId> {
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
        self.budget.used_bytes()
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
    use crate::{asset_store, canvas_store};
    use binprot::BinProtWrite;
    use gpuio_protocol::{canvas::*, canvas_resource::Update, canvas_scene::*};
    fn request() -> Request {
        let scene = Scene {
            version: 1,
            description: "Jobs".into(),
            resources: vec![],
            items: vec![Item {
                id: 1,
                transform: Transform::IDENTITY,
                clips: vec![],
                interaction: None,
                drawing: Drawing::Shape(
                    Shape::Rectangle(Rect {
                        x: 0.,
                        y: 0.,
                        width: 10.,
                        height: 10.,
                    }),
                    Paint {
                        fill: Some(0),
                        stroke: None,
                    },
                ),
            }],
        };
        let mut bytes = vec![];
        scene.binprot_write(&mut bytes).unwrap();
        let mut store = canvas_store::Store::default();
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
        store
            .publish(id, 1, &asset_store::Store::default())
            .unwrap();
        Request {
            observer: None,
            snapshot: store.acquire(id).unwrap().snapshot(),
            quality: Quality::new(1., 2.).unwrap(),
        }
    }
    fn ready(handle: &Handle) -> Ready {
        handle.take_ready().unwrap().ok().unwrap()
    }
    #[test]
    fn obsolete_jobs_cancel_and_only_latest_request_is_prepared() {
        let mut pool = Pool::default();
        let request = request();
        let handle = pool.request(request.clone()).unwrap();
        let first = pool.next_work().unwrap();
        for index in 1..=1000 {
            let mut next = request.clone();
            next.quality = Quality::new((index % 4 + 1) as f64, 2.).unwrap();
            handle.update(next).unwrap();
        }
        assert!(pool.next_work().is_none());
        let result = first.run();
        assert!(matches!(result.result, Err(Error::Cancelled)));
        pool.complete(result);
        assert!(handle.take_ready().is_none());
        assert_eq!(pool.discarded, 1);
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = ready(&handle);
        assert_eq!(ready.quality, request.quality);
        assert_eq!(ready.plan.draw_vertices, 6);
        assert!(!handle.is_pending());
        assert_eq!(handle.update(request), Ok(false));
        assert!(pool.next_work().is_none());
        drop(ready);
        drop(handle);
        assert_eq!(pool.reserved_bytes(), 0);
    }
    #[test]
    fn capacity_is_bounded_and_scheduling_is_round_robin() {
        let mut pool = Pool::default();
        let request = request();
        let handles: Vec<_> = (0..MAX_VIEWS)
            .map(|_| pool.request(request.clone()).unwrap())
            .collect();
        assert!(matches!(
            pool.request(request.clone()),
            Err(Error::LimitExceeded)
        ));
        let first = pool.next_work().unwrap();
        let second = pool.next_work().unwrap();
        assert_eq!((first.id, second.id), (1, 2));
        assert!(pool.next_work().is_none());
        pool.complete(first.run());
        let third = pool.next_work().unwrap();
        assert_eq!(third.id, 3);
        pool.complete(second.run());
        pool.complete(third.run());
        drop(handles);
        while let Some(work) = pool.next_work() {
            pool.complete(work.run());
        }
        assert_eq!(pool.peak_workers, 2);
        assert_eq!(pool.reserved_bytes(), 0);
        assert!(pool.request(request).is_ok());
    }
    #[test]
    fn drop_and_close_fence_running_and_completed_work() {
        let mut pool = Pool::default();
        let request = request();
        let handle = pool.request(request.clone()).unwrap();
        let work = pool.next_work().unwrap();
        let completion = work.run();
        assert!(pool.reserved_bytes() > 0);
        drop(handle);
        pool.complete(completion);
        assert_eq!(pool.reserved_bytes(), 0);
        let handle = pool.request(request.clone()).unwrap();
        let work = pool.next_work().unwrap();
        pool.close();
        assert!(matches!(handle.update(request.clone()), Err(Error::Closed)));
        let completion = work.run();
        assert!(matches!(completion.result, Err(Error::Cancelled)));
        pool.complete(completion);
        assert!(handle.take_ready().is_none());
        assert!(!handle.is_pending());
        assert!(pool.next_work().is_none());
        assert_eq!(pool.reserved_bytes(), 0);
        assert!(matches!(pool.request(request), Err(Error::Closed)));
    }
    #[test]
    fn work_can_run_off_thread_without_ui_or_runtime_access() {
        let mut pool = Pool::default();
        let handle = pool.request(request()).unwrap();
        let work = pool.next_work().unwrap();
        let completion = std::thread::spawn(move || work.run()).join().unwrap();
        pool.complete(completion);
        let ready = ready(&handle);
        assert_eq!(ready.plan.unique_meshes, 1);
        pool.close();
        assert!(pool.reserved_bytes() > 0);
        drop(ready);
        assert_eq!(pool.reserved_bytes(), 0);
    }
}
