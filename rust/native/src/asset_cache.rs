//! UI-owned decoded cache with transferable work tickets. The host must execute
//! work off-thread and process evictions on every window atlas that used it.
use crate::{asset_decode, asset_store::Lease, asset_svg};
use gpuio_protocol::ResourceId;
use std::{
    collections::BTreeMap,
    rc::{Rc, Weak as LocalWeak},
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

type Key = (ResourceId, asset_svg::Request);

pub const MAX_ENTRIES: usize = 256;
pub const MAX_RETIRED: usize = 256;
pub const MAX_WORKERS: usize = 2;
pub const MAX_PENDING: usize = 32;
pub const MAX_PIXEL_BYTES: usize = 256 * 1024 * 1024;
const WORK_RESERVATION: usize = asset_decode::MAX_PIXEL_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    ResourceLimit,
    Decode(asset_decode::Error),
    WorkerFailed,
}

/// Keep this lease in the mounted node. The cache itself only keeps a weak
/// reference and cannot keep an unused encoded registration alive.
#[derive(Clone)]
pub struct Handle(Rc<Owner>);
struct Owner {
    cancelled: Arc<AtomicBool>,
    cache: Rc<()>,
    source: Lease,
    key: Key,
    ticket: u64,
}

impl Handle {
    pub fn source_format(&self) -> gpuio_protocol::asset::Format {
        self.0.source.source().format()
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[derive(Clone)]
pub enum State {
    Loading,
    Ready(Arc<gpui::RenderImage>),
    Failed(Error),
}
enum EntryState {
    Queued,
    Running,
    Ready(asset_decode::Decoded),
    Failed(Error),
}
struct Entry {
    owner: LocalWeak<Owner>,
    ticket: u64,
    touched: u64,
    state: EntryState,
}
struct Ticket {
    cancelled: Arc<AtomicBool>,
    abandoned: AtomicBool,
}
struct Guard(Arc<Ticket>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.abandoned.store(true, Ordering::Release);
    }
}

/// Holds an encoded lease and a reserved result slot. Dropping an unexecuted
/// job or its undelivered completion is detected by the next cache collection.
pub struct Work {
    ticket: u64,
    id: Key,
    source: Lease,
    guard: Guard,
}
pub struct Completion {
    ticket: u64,
    id: Key,
    result: Result<asset_decode::Decoded, Error>,
    _guard: Guard,
}
impl Work {
    /// No UI callbacks; runs on GPUI's background executor. SVG text can
    /// trigger native system-font discovery. Panics are contained. Cancellation suppresses both unnecessary and late output.
    pub fn run(self) -> Completion {
        let result = if self.guard.0.cancelled.load(Ordering::Acquire) {
            Err(Error::Closed)
        } else {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if self.source.source().format() == gpuio_protocol::asset::Format::Svg {
                    asset_svg::render(self.source.source(), self.id.1)
                } else {
                    asset_decode::raster(self.source.source())
                }
            }))
            .map_err(|_| Error::WorkerFailed)
            .and_then(|result| result.map_err(Error::Decode))
        };
        let result = if self.guard.0.cancelled.load(Ordering::Acquire) {
            Err(Error::Closed)
        } else {
            result
        };
        Completion {
            ticket: self.ticket,
            id: self.id,
            result,
            _guard: self.guard,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub entries: usize,
    pub pending: usize,
    pub running: usize,
    pub retired: usize,
    pub charged_pixels: usize,
    pub evictions: usize,
}
#[derive(Default)]
pub struct Cache {
    identity: Rc<()>,
    entries: BTreeMap<Key, Entry>,
    running: BTreeMap<u64, Arc<Ticket>>,
    retired: Vec<(Weak<gpui::RenderImage>, usize)>,
    evictions: Vec<Arc<gpui::RenderImage>>,
    next: u64,
    clock: u64,
    closed: bool,
}
impl Cache {
    fn touch(&mut self) -> u64 {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }
    fn collect(&mut self) {
        self.retired.retain(|(image, _)| image.strong_count() > 0);
        self.running
            .retain(|_, ticket| !ticket.abandoned.load(Ordering::Acquire));
        self.entries.retain(|_, entry| {
            let live = entry.owner.strong_count() > 0;
            match entry.state {
                EntryState::Running => {
                    if let Some(ticket) = self.running.get(&entry.ticket) {
                        if !live {
                            ticket.cancelled.store(true, Ordering::Release);
                        }
                    } else {
                        entry.state = EntryState::Failed(Error::WorkerFailed);
                    }
                    live
                }
                EntryState::Queued | EntryState::Failed(_) => live,
                EntryState::Ready(_) => true,
            }
        });
    }
    fn charged(&self) -> usize {
        self.running.len() * WORK_RESERVATION
            + self.retired.iter().map(|(_, bytes)| bytes).sum::<usize>()
            + self
                .entries
                .values()
                .map(|entry| match &entry.state {
                    EntryState::Ready(decoded) => decoded.pixel_bytes,
                    _ => 0,
                })
                .sum::<usize>()
    }
    fn evict_one(&mut self) -> Option<usize> {
        if self.retired.len() >= MAX_RETIRED {
            return None;
        }
        let oldest = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                entry.owner.strong_count() == 0 && matches!(entry.state, EntryState::Ready(_))
            })
            .min_by_key(|(_, entry)| entry.touched)
            .map(|(id, _)| *id);
        let id = oldest?;
        let EntryState::Ready(decoded) = self.entries.remove(&id).unwrap().state else {
            unreachable!()
        };
        self.retired
            .push((Arc::downgrade(&decoded.image), decoded.pixel_bytes));
        self.evictions.push(decoded.image);
        Some(decoded.pixel_bytes)
    }
    /// The source must be freshly acquired from this application's encoded
    /// registry. Retired IDs cannot gain new bindings via a warm decoded entry.
    pub fn request(&mut self, source: Lease) -> Result<Handle, Error> {
        self.request_variant(source, asset_svg::Request::default())
    }
    /// Resample an existing mounted SVG lease, including after encoded
    /// registration retirement. This must not be used to create a new binding.
    pub fn rerasterize(
        &mut self,
        handle: &Handle,
        request: asset_svg::Request,
    ) -> Result<Handle, Error> {
        if self.closed || !Rc::ptr_eq(&self.identity, &handle.0.cache) {
            return Err(Error::Closed);
        }
        if handle.source_format() != gpuio_protocol::asset::Format::Svg {
            return Err(Error::Decode(asset_decode::Error::Unsupported));
        }
        self.request_variant(handle.0.source.clone(), request)
    }
    fn request_variant(
        &mut self,
        source: Lease,
        request: asset_svg::Request,
    ) -> Result<Handle, Error> {
        self.collect();
        if self.closed {
            return Err(Error::Closed);
        }
        let touched = self.touch();
        let id = (source.id(), request);
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.touched = touched;
            if let Some(owner) = entry.owner.upgrade() {
                return Ok(Handle(owner));
            }
            // Only a warm Ready entry can survive collection without a user.
            let owner = Rc::new(Owner {
                cancelled: Arc::new(AtomicBool::new(false)),
                cache: self.identity.clone(),
                source,
                key: id,
                ticket: entry.ticket,
            });
            entry.owner = Rc::downgrade(&owner);
            return Ok(Handle(owner));
        }
        let pending = self
            .entries
            .values()
            .filter(|entry| matches!(entry.state, EntryState::Queued))
            .count();
        if pending >= MAX_PENDING || self.next == u64::MAX {
            return Err(Error::ResourceLimit);
        }
        if self.entries.len() >= MAX_ENTRIES && self.evict_one().is_none() {
            return Err(Error::ResourceLimit);
        }
        self.next += 1;
        let owner = Rc::new(Owner {
            cancelled: Arc::new(AtomicBool::new(false)),
            cache: self.identity.clone(),
            source,
            key: id,
            ticket: self.next,
        });
        self.entries.insert(
            id,
            Entry {
                owner: Rc::downgrade(&owner),
                ticket: self.next,
                touched,
                state: EntryState::Queued,
            },
        );
        Ok(Handle(owner))
    }
    pub fn state(&self, handle: &Handle) -> State {
        if self.closed || !Rc::ptr_eq(&self.identity, &handle.0.cache) {
            return State::Failed(Error::Closed);
        }
        match self.entries.get(&handle.0.key) {
            Some(entry) if entry.ticket == handle.0.ticket => match &entry.state {
                EntryState::Queued | EntryState::Running => State::Loading,
                EntryState::Ready(decoded) => State::Ready(decoded.image.clone()),
                EntryState::Failed(error) => State::Failed(*error),
            },
            Some(_) | None => State::Failed(Error::Closed),
        }
    }
    /// Reserves the maximum retained output before handing off work. At most
    /// two jobs/completions coexist. Decoder working allocations are additional
    /// and best-effort limited; this reservation covers retained pixels only.
    pub fn next_work(&mut self) -> Option<Work> {
        self.collect();
        if self.closed || self.running.len() >= MAX_WORKERS {
            return None;
        }
        let id = self
            .entries
            .iter()
            .filter(|(_, entry)| matches!(entry.state, EntryState::Queued))
            .min_by_key(|(_, entry)| entry.ticket)
            .map(|(id, _)| *id)?;
        if self.charged() > MAX_PIXEL_BYTES - WORK_RESERVATION {
            // Evictions stay charged until atlas cleanup/other readers release.
            let needed = self.charged() - (MAX_PIXEL_BYTES - WORK_RESERVATION);
            let mut retired = 0;
            while retired < needed {
                let Some(bytes) = self.evict_one() else {
                    break;
                };
                retired += bytes;
            }
            if self.running.is_empty() && self.evictions.is_empty() {
                self.entries.get_mut(&id).unwrap().state = EntryState::Failed(Error::ResourceLimit);
            }
            return None;
        }
        let entry = self.entries.get_mut(&id).unwrap();
        let owner = entry.owner.upgrade().expect("collected orphan queue");
        let ticket = Arc::new(Ticket {
            cancelled: owner.cancelled.clone(),
            abandoned: AtomicBool::new(false),
        });
        self.running.insert(entry.ticket, ticket.clone());
        entry.state = EntryState::Running;
        Some(Work {
            ticket: entry.ticket,
            id,
            source: owner.source.clone(),
            guard: Guard(ticket),
        })
    }
    /// Returns whether a live consumer's observable state changed. Disposed or
    /// replaced consumers never receive a late result under their new ticket.
    pub fn complete(&mut self, completion: Completion) -> bool {
        self.collect();
        let Some(ticket) = self.running.get(&completion.ticket) else {
            return false;
        };
        if !Arc::ptr_eq(ticket, &completion._guard.0) {
            return false;
        }
        self.running.remove(&completion.ticket);
        if self.closed {
            return false;
        }
        let Some(entry) = self.entries.get_mut(&completion.id) else {
            return false;
        };
        if entry.ticket != completion.ticket || entry.owner.strong_count() == 0 {
            return false;
        }
        entry.state = match completion.result {
            Ok(decoded) => EntryState::Ready(decoded),
            Err(error) => EntryState::Failed(error),
        };
        true
    }
    /// Drop every returned image from all atlases that used it, then release
    /// this vector. Existing paint/consumer Arcs remain charged until gone.
    pub fn take_evictions(&mut self) -> Vec<Arc<gpui::RenderImage>> {
        std::mem::take(&mut self.evictions)
    }
    pub fn stats(&mut self) -> Stats {
        self.collect();
        Stats {
            entries: self.entries.len(),
            pending: self
                .entries
                .values()
                .filter(|entry| matches!(entry.state, EntryState::Queued))
                .count(),
            running: self.running.len(),
            retired: self.retired.len(),
            charged_pixels: self.charged(),
            evictions: self.evictions.len(),
        }
    }
    /// Stops new work and cancels live jobs. The host must still drain running
    /// tasks and process returned atlas evictions before destroying windows.
    pub fn close(&mut self) {
        self.closed = true;
        for ticket in self.running.values() {
            ticket.cancelled.store(true, Ordering::Release);
        }
        // Active handles lose cache access, but their encoded leases remain
        // owned until their nodes are disposed. Keep decoded frames charged.
        for (_, entry) in std::mem::take(&mut self.entries) {
            if let EntryState::Ready(decoded) = entry.state {
                self.retired
                    .push((Arc::downgrade(&decoded.image), decoded.pixel_bytes));
                self.evictions.push(decoded.image);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_store::{MAX_CHUNK_BYTES, Store};
    use gpuio_protocol::asset::Format;

    fn source(store: &mut Store) -> Lease {
        let data = b"P6\n1 1\n255\n\xff\x07\x20";
        let id = store.begin(Format::Pnm, data.len()).unwrap();
        for (index, chunk) in data.chunks(MAX_CHUNK_BYTES).enumerate() {
            store.append(id, index * MAX_CHUNK_BYTES, chunk).unwrap();
        }
        store.finish(id).unwrap();
        store.acquire(id).unwrap()
    }
    fn ready(cache: &Cache, handle: &Handle) -> Arc<gpui::RenderImage> {
        match cache.state(handle) {
            State::Ready(image) => image,
            _ => panic!("not ready"),
        }
    }

    #[test]
    fn shared_decode_runs_off_thread_and_warm_pixels_do_not_retain_encoded_source() {
        let mut store = Store::default();
        let source = source(&mut store);
        let id = source.id();
        let mut cache = Cache::default();
        let first = cache.request(source.clone()).unwrap();
        let second = cache.request(source.clone()).unwrap();
        let work = cache.next_work().unwrap();
        assert!(cache.next_work().is_none());
        assert_eq!(cache.stats().charged_pixels, WORK_RESERVATION);
        let completion = std::thread::spawn(move || work.run()).join().unwrap();
        assert!(cache.complete(completion));
        assert!(Arc::ptr_eq(&ready(&cache, &first), &ready(&cache, &second)));
        assert_eq!(
            ready(&cache, &first).as_bytes(0).unwrap(),
            [32, 7, 255, 255]
        );
        assert_eq!(cache.stats().charged_pixels, 4);
        store.release(id).unwrap();
        assert!(
            store.acquire(id).is_err(),
            "warm pixels cannot resurrect retired acquisition"
        );
        drop((first, second, source));
        assert_eq!(store.stats().reserved_bytes, 0);
        assert_eq!(
            cache.stats().entries,
            1,
            "warm pixels are retained independently"
        );
    }

    #[test]
    fn pending_and_worker_bounds_and_abandoned_work_reclaim_reservations() {
        let mut store = Store::default();
        let mut cache = Cache::default();
        let mut owners = Vec::new();
        for _ in 0..MAX_PENDING {
            owners.push(cache.request(source(&mut store)).unwrap());
        }
        assert!(matches!(
            cache.request(source(&mut store)),
            Err(Error::ResourceLimit)
        ));
        let a = cache.next_work().unwrap();
        let b = cache.next_work().unwrap();
        assert!(cache.next_work().is_none());
        assert_eq!(cache.stats().running, 2);
        assert_eq!(cache.stats().charged_pixels, 2 * WORK_RESERVATION);
        drop(a);
        assert_eq!(cache.stats().running, 1);
        assert!(matches!(
            cache.state(&owners[0]),
            State::Failed(Error::WorkerFailed)
        ));
        let c = cache.next_work().unwrap();
        drop(b.run()); // Undelivered completion releases its reservation too.
        assert_eq!(cache.stats().running, 1);
        assert!(matches!(
            cache.state(&owners[1]),
            State::Failed(Error::WorkerFailed)
        ));
        drop((owners, c));
        let stats = cache.stats();
        assert_eq!(
            (stats.entries, stats.running, stats.charged_pixels),
            (0, 0, 0)
        );
    }

    #[test]
    fn disposed_replaced_and_shutdown_owners_reject_late_results() {
        let mut store = Store::default();
        let source = source(&mut store);
        let mut cache = Cache::default();
        let old = cache.request(source.clone()).unwrap();
        let work = cache.next_work().unwrap();
        let completion = work.run(); // Completed but not yet delivered.
        drop(old);
        let replacement = cache.request(source.clone()).unwrap();
        assert!(!cache.complete(completion));
        assert!(matches!(cache.state(&replacement), State::Loading));
        let work = cache.next_work().unwrap();
        assert!(cache.complete(work.run()));
        assert_eq!(ready(&cache, &replacement).frame_count(), 1);
        drop(replacement);
        let other = cache.request(self::source(&mut store)).unwrap();
        let work = cache.next_work().unwrap();
        cache.close();
        assert!(matches!(cache.state(&other), State::Failed(Error::Closed)));
        assert!(!cache.complete(work.run()));
        assert!(matches!(cache.request(source), Err(Error::Closed)));
        drop(cache.take_evictions());
        assert_eq!(cache.stats().charged_pixels, 0);
    }

    #[test]
    fn atlas_and_paint_references_remain_charged_after_cache_eviction() {
        let mut store = Store::default();
        let mut cache = Cache::default();
        let owner = cache.request(source(&mut store)).unwrap();
        let work = cache.next_work().unwrap();
        cache.complete(work.run());
        let paint = ready(&cache, &owner);
        drop(owner);
        assert_eq!(cache.evict_one(), Some(4));
        assert_eq!(cache.stats().entries, 0);
        assert_eq!(cache.stats().charged_pixels, 4);
        drop(cache.take_evictions());
        assert_eq!(
            cache.stats().charged_pixels,
            4,
            "a painted element still owns pixels"
        );
        drop(paint);
        assert_eq!(cache.stats().charged_pixels, 0);
        assert_eq!(cache.stats().retired, 0);
    }

    #[test]
    fn retained_and_reserved_pixels_share_one_budget_before_worker_dispatch() {
        let mut store = Store::default();
        let mut cache = Cache::default();
        let mut owners = Vec::new();
        // Exercise admission with charged maximum-size results without making
        // the unit test allocate 256 MiB. This test validates accounting;
        // asset_decode tests independently validate actual output sizing.
        for _ in 0..4 {
            let owner = cache.request(source(&mut store)).unwrap();
            let work = cache.next_work().unwrap();
            let mut completion = work.run();
            completion.result.as_mut().unwrap().pixel_bytes = WORK_RESERVATION;
            cache.complete(completion);
            owners.push(owner);
        }
        assert_eq!(cache.stats().charged_pixels, MAX_PIXEL_BYTES);
        let blocked = cache.request(source(&mut store)).unwrap();
        assert!(cache.next_work().is_none());
        assert!(matches!(
            cache.state(&blocked),
            State::Failed(Error::ResourceLimit)
        ));
        drop(owners.remove(0));
        drop(blocked);
        let retry = cache.request(source(&mut store)).unwrap();
        assert!(cache.next_work().is_none());
        assert_eq!(
            cache.stats().evictions,
            1,
            "only enough warm entries are retired"
        );
        drop(cache.take_evictions());
        let work = cache.next_work().unwrap();
        assert_eq!(cache.stats().charged_pixels, MAX_PIXEL_BYTES);
        assert!(cache.complete(work.run()));
        assert!(matches!(cache.state(&retry), State::Ready(_)));
    }
    #[test]
    fn cache_identity_rejects_foreign_handles_and_equal_numbered_work() {
        let mut store = Store::default();
        let source = source(&mut store);
        let mut a = Cache::default();
        let mut b = Cache::default();
        let owner_a = a.request(source.clone()).unwrap();
        let owner_b = b.request(source).unwrap();
        let work_a = a.next_work().unwrap();
        let work_b = b.next_work().unwrap();
        assert_eq!(work_a.ticket, work_b.ticket);
        assert!(!b.complete(work_a.run()));
        assert_eq!(
            b.stats().running,
            1,
            "foreign completion cannot retire our reservation"
        );
        assert!(matches!(b.state(&owner_a), State::Failed(Error::Closed)));
        assert!(b.complete(work_b.run()));
        assert!(matches!(b.state(&owner_b), State::Ready(_)));
        assert_eq!(
            a.stats().running,
            0,
            "discarded completion is collected by its owner"
        );
    }

    #[test]
    fn entry_and_undrained_eviction_metadata_have_finite_bounds() {
        let mut store = Store::default();
        let mut cache = Cache::default();
        for _ in 0..MAX_ENTRIES + MAX_RETIRED {
            let owner = cache.request(source(&mut store)).unwrap();
            let work = cache.next_work().unwrap();
            assert!(cache.complete(work.run()));
            drop(owner);
        }
        let stats = cache.stats();
        assert_eq!(stats.entries, MAX_ENTRIES);
        assert_eq!(stats.retired, MAX_RETIRED);
        assert_eq!(stats.evictions, MAX_RETIRED);
        assert!(matches!(
            cache.request(source(&mut store)),
            Err(Error::ResourceLimit)
        ));
        drop(cache.take_evictions());
        let next = cache.request(source(&mut store)).unwrap();
        let work = cache.next_work().unwrap();
        assert!(cache.complete(work.run()));
        cache.close();
        assert!(matches!(cache.state(&next), State::Failed(Error::Closed)));
        drop(cache.take_evictions());
        assert_eq!(cache.stats().charged_pixels, 0);
    }
    #[test]
    fn last_owner_drop_cancels_before_another_cache_turn() {
        let mut store = Store::default();
        let mut cache = Cache::default();
        let first = cache.request(source(&mut store)).unwrap();
        let second = first.clone();
        let work = cache.next_work().unwrap();
        drop(first);
        assert!(!work.guard.0.cancelled.load(Ordering::Acquire));
        drop(second);
        // No intervening cache collection: the final mounted owner signals
        // the transferable job directly, allowing it to skip decoding.
        let completion = std::thread::spawn(move || work.run()).join().unwrap();
        assert!(matches!(completion.result, Err(Error::Closed)));
        assert!(!cache.complete(completion));
        assert_eq!(cache.stats().charged_pixels, 0);
    }
    #[test]
    fn svg_variants_share_by_size_fit_density_and_tint_without_reacquiring_retired_sources() {
        let mut store = Store::default();
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="red"/></svg>"#;
        let id = store.begin(Format::Svg, bytes.len()).unwrap();
        store.append(id, 0, bytes).unwrap();
        store.finish(id).unwrap();
        let mut cache = Cache::default();
        let original = cache.request(store.acquire(id).unwrap()).unwrap();
        let work = cache.next_work().unwrap();
        assert!(cache.complete(std::thread::spawn(move || work.run()).join().unwrap()));
        assert_eq!(
            ready(&cache, &original).as_bytes(0).unwrap()[..4],
            [0, 0, 255, 255]
        );
        store.release(id).unwrap();
        assert!(store.acquire(id).is_err());
        let params = asset_svg::Request {
            size: asset_svg::Size::Exact(asset_svg::RasterSize::new(8, 8).unwrap()),
            tint: Some(0x00ff00ff),
            ..Default::default()
        };
        let green = cache.rerasterize(&original, params).unwrap();
        let shared = cache.rerasterize(&original, params).unwrap();
        assert!(Rc::ptr_eq(&green.0, &shared.0));
        let blue = cache
            .rerasterize(
                &original,
                asset_svg::Request {
                    tint: Some(0x0000ffff),
                    ..params
                },
            )
            .unwrap();
        let green_work = cache.next_work().unwrap();
        let blue_work = cache.next_work().unwrap();
        assert!(cache.next_work().is_none());
        assert!(cache.complete(blue_work.run()));
        assert!(matches!(cache.state(&green), State::Loading));
        assert_eq!(
            ready(&cache, &blue).as_bytes(0).unwrap()[..4],
            [255, 0, 0, 255]
        );
        assert!(cache.complete(green_work.run()));
        assert_eq!(
            ready(&cache, &green).as_bytes(0).unwrap()[..4],
            [0, 255, 0, 255]
        );
        assert_eq!(cache.stats().charged_pixels, 64 + 256 + 256);
        let mut foreign = Cache::default();
        assert!(matches!(
            foreign.rerasterize(&original, params),
            Err(Error::Closed)
        ));
        let density = cache
            .rerasterize(
                &original,
                asset_svg::Request {
                    density: asset_svg::Density::new(2.).unwrap(),
                    ..params
                },
            )
            .unwrap();
        let fit = cache
            .rerasterize(
                &original,
                asset_svg::Request {
                    fit: asset_svg::Fit::None,
                    ..params
                },
            )
            .unwrap();
        assert_eq!(cache.stats().pending, 2);
        drop((original, green, shared, blue, density, fit));
        assert_eq!(cache.stats().pending, 0);
        assert_eq!(
            store.stats().retired,
            0,
            "warm pixel variants do not retain encoded sources"
        );
    }
}
