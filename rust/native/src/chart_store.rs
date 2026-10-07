//! UI-owned registrations with bounded, off-thread decoding. A completion may
//! publish only into the exact live upload that issued it. Reader snapshots stay
//! charged after replacement/release until their final reader drops them.
use gpuio_protocol::{
    ResourceId,
    chart_data::{self, Data},
    chart_resource::{Error, MAX_CHUNK_BYTES, Update},
    decode_chart_data,
};
use std::{
    cell::RefCell,
    mem::size_of,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

pub const MAX_CHARTS: usize = 256;
pub const MAX_STAGING: usize = 4;
pub const MAX_WORKERS: usize = 2;
pub const MAX_RESERVED_BYTES: usize = 256 * 1024 * 1024;
/// Per admitted decode: owned records/strings plus temporary validation sets.
/// This is conservative accounting, not a bound on allocator metadata or RSS.
pub const DECODE_WORKSPACE_BYTES: usize = 64 * 1024 * 1024;
const FIXED_CHARGE: usize = 4096;
const _: () = assert!(
    FIXED_CHARGE
        + chart_data::MAX_TEXT_BYTES
        + chart_data::MAX_POINTS
            * (size_of::<chart_data::Candle>() + size_of::<chart_data::Category>() + 256)
        + chart_data::MAX_POINTS * size_of::<chart_data::BarBackground>()
        + 2048 * 256
        < DECODE_WORKSPACE_BYTES
);

struct Reservation {
    used: Arc<AtomicUsize>,
    bytes: usize,
}
impl Reservation {
    fn new(used: &Arc<AtomicUsize>, bytes: usize) -> Result<Self, Error> {
        used.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current
                .checked_add(bytes)
                .filter(|n| *n <= MAX_RESERVED_BYTES)
        })
        .map_err(|_| Error::ResourceLimit)?;
        Ok(Self {
            used: used.clone(),
            bytes,
        })
    }
    fn shrink(&mut self, bytes: usize) {
        assert!(bytes <= self.bytes);
        self.used.fetch_sub(self.bytes - bytes, Ordering::Relaxed);
        self.bytes = bytes;
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.used.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

struct Permit(Arc<AtomicUsize>);
impl Permit {
    fn take(count: &Arc<AtomicUsize>, limit: usize) -> Result<Self, Error> {
        count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (n < limit).then_some(n + 1)
            })
            .map_err(|_| Error::Busy)?;
        Ok(Self(count.clone()))
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct Snapshot {
    pub(crate) revision: i64,
    pub(crate) generation: i64,
    pub(crate) data: Data,
    pub(crate) encoded_bytes: usize,
    _reservation: Reservation,
}
impl Snapshot {
    pub fn revision(&self) -> i64 {
        self.revision
    }
    pub fn generation(&self) -> i64 {
        self.generation
    }
    pub fn data(&self) -> &Data {
        &self.data
    }
    pub fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
}
/// Live reader slot. Release/close retires it; already acquired immutable
/// snapshots remain valid for in-flight work but cannot revive the resource.
#[derive(Clone)]
pub struct Lease {
    current: Rc<RefCell<Option<Arc<Snapshot>>>>,
    _reservation: Rc<Reservation>,
}
impl Lease {
    pub fn snapshot(&self) -> Option<Arc<Snapshot>> {
        self.current.borrow().clone()
    }
}
struct Upload {
    update: Update,
    bytes: Vec<u8>,
    _reservation: Reservation,
    _permit: Permit,
}
enum Stage {
    Upload(Upload),
    Decoding {
        revision: i64,
        cancel: Arc<AtomicBool>,
    },
}
impl Stage {
    fn revision(&self) -> i64 {
        match self {
            Self::Upload(u) => u.update.revision,
            Self::Decoding { revision, .. } => *revision,
        }
    }
    fn cancel(&self) {
        if let Self::Decoding { cancel, .. } = self {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}
struct Entry {
    lease: Lease,
    stage: Option<Stage>,
}
impl Drop for Entry {
    fn drop(&mut self) {
        if let Some(stage) = &self.stage {
            stage.cancel();
        }
        self.lease.current.borrow_mut().take();
    }
}
#[derive(Default)]
struct Slot {
    generation: u32,
    entry: Option<Entry>,
}
#[derive(Default)]
pub struct Store {
    slots: Vec<Slot>,
    reserved: Arc<AtomicUsize>,
    staging: Arc<AtomicUsize>,
    workers: Arc<AtomicUsize>,
    closed: bool,
}

/// Sendable job, issued only after both worker and memory admission. Cancellation
/// is checked before and after bounded decoding. The host must return Completion
/// (or abort the upload if it deliberately drops a job without running it).
pub struct Work {
    upload: Upload,
    cancel: Arc<AtomicBool>,
    workspace: Reservation,
    worker: Permit,
}
pub struct Completion {
    id: ResourceId,
    revision: i64,
    cancel: Arc<AtomicBool>,
    result: Result<Arc<Snapshot>, Error>,
    _staging: Permit,
    _worker: Permit,
}
impl Completion {
    pub fn id(&self) -> ResourceId {
        self.id
    }
    pub fn revision(&self) -> i64 {
        self.revision
    }
}

fn data_charge(data: &Data) -> usize {
    use chart_data::*;
    let dynamic = match &data.contents {
        Contents::Cartesian(layers) => {
            layers.capacity() * size_of::<Layer>()
                + layers
                    .iter()
                    .map(|l| {
                        let s = l.series();
                        s.name.capacity()
                            + s.points.capacity() * size_of::<Point>()
                            + s.points.iter().map(|p| p.label.capacity()).sum::<usize>()
                    })
                    .sum::<usize>()
        }
        Contents::Categorical(categories, layers) => {
            categories.capacity() * size_of::<Category>()
                + categories.iter().map(|c| c.label.capacity()).sum::<usize>()
                + layers.capacity() * size_of::<CategoricalLayer>()
                + layers
                    .iter()
                    .map(|l| {
                        let s = l.series();
                        s.name.capacity()
                            + s.points.capacity() * size_of::<CategoricalPoint>()
                            + s.points.iter().map(|p| p.label.capacity()).sum::<usize>()
                    })
                    .sum::<usize>()
        }
        Contents::Pie(slices) => {
            slices.capacity() * size_of::<Slice>()
                + slices.iter().map(|s| s.label.capacity()).sum::<usize>()
        }
        Contents::Radar(axes, series) => {
            axes.capacity() * size_of::<RadarAxis>()
                + axes.iter().map(|a| a.label.capacity()).sum::<usize>()
                + series.capacity() * size_of::<RadarSeries>()
                + series
                    .iter()
                    .map(|s| s.name.capacity() + s.values.capacity() * size_of::<(i64, f64)>())
                    .sum::<usize>()
        }
        Contents::Candlestick(candles) => {
            candles.capacity() * size_of::<Candle>()
                + candles.iter().map(|c| c.label.capacity()).sum::<usize>()
        }
        Contents::Sankey(nodes, edges) => {
            nodes.capacity() * size_of::<Node>()
                + nodes.iter().map(|n| n.label.capacity()).sum::<usize>()
                + edges.capacity() * size_of::<Edge>()
        }
    };
    FIXED_CHARGE
        + size_of::<Data>()
        + dynamic
        + data.bar_backgrounds.capacity() * size_of::<chart_data::BarBackground>()
}
impl Work {
    pub fn run(mut self) -> Completion {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let data = decode_chart_data(&self.upload.bytes).map_err(|e| match e {
                gpuio_protocol::DecodeError::Malformed => Error::InvalidData,
                gpuio_protocol::DecodeError::LimitExceeded => Error::ResourceLimit,
            })?;
            if self.cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let charge = data_charge(&data);
            if charge > self.workspace.bytes {
                return Err(Error::ResourceLimit);
            }
            self.workspace.shrink(charge);
            Ok(data)
        }))
        .unwrap_or(Err(Error::NativeFailure));
        let result = result.map(|data| {
            Arc::new(Snapshot {
                revision: self.upload.update.revision,
                generation: self.upload.update.generation,
                encoded_bytes: self.upload.bytes.len(),
                data,
                _reservation: self.workspace,
            })
        });
        Completion {
            id: self.upload.update.id,
            revision: self.upload.update.revision,
            cancel: self.cancel,
            result,
            _staging: self.upload._permit,
            _worker: self.worker,
        }
    }
}

impl Store {
    pub fn reserved_bytes(&self) -> usize {
        self.reserved.load(Ordering::Relaxed)
    }
    /// Includes cancelled work until its completion is applied or dropped.
    pub fn staged_count(&self) -> usize {
        self.staging.load(Ordering::Relaxed)
    }
    pub fn worker_count(&self) -> usize {
        self.workers.load(Ordering::Relaxed)
    }
    pub fn create(&mut self) -> Result<ResourceId, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let index = self
            .slots
            .iter()
            .position(|s| s.entry.is_none() && s.generation < u32::MAX)
            .unwrap_or(self.slots.len());
        if index >= MAX_CHARTS {
            return Err(Error::ResourceLimit);
        }
        let reservation = Reservation::new(&self.reserved, FIXED_CHARGE)?;
        if index == self.slots.len() {
            self.slots.push(Slot::default());
        }
        let slot = &mut self.slots[index];
        slot.generation += 1;
        slot.entry = Some(Entry {
            lease: Lease {
                current: Rc::new(RefCell::new(None)),
                _reservation: Rc::new(reservation),
            },
            stage: None,
        });
        Ok(
            ResourceId::from_parts(index as i64, i64::from(slot.generation))
                .expect("bounded chart slot"),
        )
    }
    fn entry(&self, id: ResourceId) -> Result<&Entry, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        self.slots
            .get(id.slot())
            .filter(|s| s.generation == id.generation())
            .and_then(|s| s.entry.as_ref())
            .ok_or(Error::StaleHandle)
    }
    fn entry_mut(&mut self, id: ResourceId) -> Result<&mut Entry, Error> {
        self.entry(id)?;
        Ok(self.slots[id.slot()]
            .entry
            .as_mut()
            .expect("validated chart"))
    }
    pub fn acquire(&self, id: ResourceId) -> Result<Lease, Error> {
        let entry = self.entry(id)?;
        if entry.lease.current.borrow().is_none() {
            return Err(Error::NotReady);
        }
        Ok(entry.lease.clone())
    }
    pub fn begin(&mut self, update: Update) -> Result<(), Error> {
        let entry = self.entry(update.id)?;
        if entry.stage.is_some() {
            return Err(Error::Busy);
        }
        let previous = entry.lease.current.borrow();
        let revision = previous.as_ref().map_or(0, |s| s.revision);
        let generation = previous.as_ref().map_or(1, |s| s.generation);
        if update.base != revision
            || revision.checked_add(1) != Some(update.revision)
            || !(update.generation == generation
                || (revision > 0 && generation.checked_add(1) == Some(update.generation)))
        {
            return Err(Error::InvalidRevision);
        }
        let count = usize::try_from(update.bytes).map_err(|_| Error::InvalidRange)?;
        if count == 0 || count > chart_data::MAX_BYTES {
            return Err(Error::InvalidRange);
        }
        let permit = Permit::take(&self.staging, MAX_STAGING)?;
        let mut reservation = Reservation::new(&self.reserved, FIXED_CHARGE + count)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        // try_reserve_exact may return a larger capacity; account for it too.
        if bytes.capacity() > count {
            reservation = Reservation::new(&self.reserved, FIXED_CHARGE + bytes.capacity())?;
        }
        drop(previous);
        let id = update.id;
        self.entry_mut(id)?.stage = Some(Stage::Upload(Upload {
            update,
            bytes,
            _reservation: reservation,
            _permit: permit,
        }));
        Ok(())
    }
    pub fn chunk(
        &mut self,
        id: ResourceId,
        revision: i64,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let stage = self.entry_mut(id)?.stage.as_mut().ok_or(Error::NotReady)?;
        if stage.revision() != revision {
            return Err(Error::InvalidRevision);
        }
        let Stage::Upload(upload) = stage else {
            return Err(Error::Busy);
        };
        if bytes.is_empty()
            || bytes.len() > MAX_CHUNK_BYTES
            || offset != upload.bytes.len()
            || bytes.len() > upload.update.bytes as usize - upload.bytes.len()
        {
            return Err(Error::InvalidRange);
        }
        upload.bytes.extend_from_slice(bytes);
        Ok(())
    }
    /// Admission failure leaves the upload intact for retry or Abort. Success
    /// transfers its memory/staging permits to the job and reserves a worker.
    pub fn publish(&mut self, id: ResourceId, revision: i64) -> Result<Work, Error> {
        let stage = self.entry(id)?.stage.as_ref().ok_or(Error::NotReady)?;
        if stage.revision() != revision {
            return Err(Error::InvalidRevision);
        }
        let Stage::Upload(upload) = stage else {
            return Err(Error::Busy);
        };
        if upload.bytes.len() != upload.update.bytes as usize {
            return Err(Error::Incomplete);
        }
        let worker = Permit::take(&self.workers, MAX_WORKERS)?;
        let workspace = Reservation::new(&self.reserved, DECODE_WORKSPACE_BYTES)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let stage = self.entry_mut(id)?.stage.replace(Stage::Decoding {
            revision,
            cancel: cancel.clone(),
        });
        let Some(Stage::Upload(upload)) = stage else {
            unreachable!("validated upload")
        };
        Ok(Work {
            upload,
            cancel,
            workspace,
            worker,
        })
    }
    /// Failure consumes this upload while leaving the prior publication intact.
    /// A cancelled/foreign completion never clears a replacement upload, even if
    /// the caller retries the same revision and reuses the same slot.
    pub fn complete(&mut self, completion: Completion) -> Result<(), Error> {
        let entry = self.entry_mut(completion.id)?;
        let Some(Stage::Decoding { cancel, .. }) = &entry.stage else {
            return Err(Error::Cancelled);
        };
        if !Arc::ptr_eq(cancel, &completion.cancel) {
            return Err(Error::Cancelled);
        }
        entry.stage = None;
        if completion.cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let snapshot = completion.result?;
        *entry.lease.current.borrow_mut() = Some(snapshot);
        Ok(())
    }
    pub fn abort(&mut self, id: ResourceId, revision: i64) -> Result<(), Error> {
        let entry = self.entry_mut(id)?;
        let Some(stage) = entry.stage.as_ref() else {
            // A failed decode already consumed its stage; cleanup of that next
            // revision is idempotent. Never acknowledge aborting a publication.
            let published = entry
                .lease
                .current
                .borrow()
                .as_ref()
                .map_or(0, |s| s.revision);
            return if published.checked_add(1) == Some(revision) {
                Ok(())
            } else {
                Err(Error::InvalidRevision)
            };
        };
        if stage.revision() != revision {
            return Err(Error::InvalidRevision);
        }
        stage.cancel();
        entry.stage = None;
        Ok(())
    }
    pub fn release(&mut self, id: ResourceId) -> Result<(), Error> {
        self.entry(id)?;
        self.slots[id.slot()].entry = None;
        Ok(())
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.slots.clear();
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
