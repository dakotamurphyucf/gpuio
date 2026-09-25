//! Canonical display snapshots and atomic suffix uploads. Main-thread resource
//! leases and immutable background snapshots have separate lifetimes. Quota
//! charges survive release while a view/parser still holds an old snapshot.
use gpuio_protocol::{
    ResourceId,
    document::{Error, MAX_BYTES, MAX_CHUNK_BYTES, Status, Update},
};
use ropey::Rope;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const MAX_DOCUMENTS: usize = 1024;
const MAX_STAGING: usize = 8;
const MAX_RESERVED_BYTES: usize = 64 * 1024 * 1024;
// Also bounds retained empty snapshots and leases, not just their text bytes.
const SNAPSHOT_CHARGE: usize = 4096;

struct Reservation {
    used: Arc<AtomicUsize>,
    bytes: usize,
}
impl Reservation {
    fn new(used: &Arc<AtomicUsize>, bytes: usize) -> Result<Self, Error> {
        used.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current
                .checked_add(bytes)
                .filter(|next| *next <= MAX_RESERVED_BYTES)
        })
        .map_err(|_| Error::ResourceLimit)?;
        Ok(Self {
            used: used.clone(),
            bytes,
        })
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.used.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

pub struct Snapshot {
    pub revision: i64,
    pub generation: i64,
    pub status: Status,
    pub text: Rope,
    /// Prefix known unchanged from the immediately preceding revision.
    pub unchanged_bytes: usize,
    _reservation: Reservation,
}

#[derive(Clone)]
pub struct Lease(Rc<RefCell<Arc<Snapshot>>>);
impl Lease {
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.0.borrow().clone()
    }
}

struct Staging {
    update: Update,
    suffix: Vec<u8>,
    reservation: Reservation,
}
struct Entry {
    lease: Lease,
    staging: Option<Staging>,
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
    closed: bool,
}

impl Store {
    pub fn reserved_bytes(&self) -> usize {
        self.reserved.load(Ordering::Relaxed)
    }
    pub fn staged_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.entry.as_ref().is_some_and(|e| e.staging.is_some()))
            .count()
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
        if index >= MAX_DOCUMENTS {
            return Err(Error::ResourceLimit);
        }
        let snapshot = Arc::new(Snapshot {
            revision: 0,
            generation: 1,
            status: Status::Streaming,
            text: Rope::new(),
            unchanged_bytes: 0,
            _reservation: Reservation::new(&self.reserved, SNAPSHOT_CHARGE)?,
        });
        if index == self.slots.len() {
            self.slots.push(Slot::default());
        }
        let slot = &mut self.slots[index];
        slot.generation += 1;
        slot.entry = Some(Entry {
            lease: Lease(Rc::new(RefCell::new(snapshot))),
            staging: None,
        });
        Ok(ResourceId::from_parts(index as i64, slot.generation as i64)
            .expect("bounded document slot"))
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
            .expect("validated document"))
    }
    pub fn acquire(&self, id: ResourceId) -> Result<Lease, Error> {
        Ok(self.entry(id)?.lease.clone())
    }
    pub fn begin(&mut self, update: Update) -> Result<(), Error> {
        let entry = self.entry(update.id)?;
        if entry.staging.is_some() {
            return Err(Error::Busy);
        }
        let old = entry.lease.snapshot();
        let reset = old.generation.checked_add(1) == Some(update.generation);
        if update.base != old.revision
            || old.revision.checked_add(1) != Some(update.revision)
            || !(update.generation == old.generation || reset)
            || (reset && update.from_byte != 0)
            || (!reset && old.status != Status::Streaming && old.status != update.status)
        {
            return Err(Error::InvalidRevision);
        }
        let from = usize::try_from(update.from_byte).map_err(|_| Error::InvalidRange)?;
        let suffix = usize::try_from(update.suffix_bytes).map_err(|_| Error::InvalidRange)?;
        if from > old.text.len()
            || suffix > MAX_BYTES
            || from > MAX_BYTES - suffix
            || !old.text.is_char_boundary(from)
        {
            return Err(Error::InvalidRange);
        }
        if self.staged_count() >= MAX_STAGING {
            return Err(Error::ResourceLimit);
        }
        // Charge both the future source and staged bytes. This conservative
        // admission bound is not an RSS estimate for Rope or parser layouts.
        let reservation = Reservation::new(&self.reserved, SNAPSHOT_CHARGE + from + suffix * 2)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(suffix)
            .map_err(|_| Error::ResourceLimit)?;
        let id = update.id;
        self.entry_mut(id)?.staging = Some(Staging {
            update,
            suffix: bytes,
            reservation,
        });
        Ok(())
    }
    pub fn chunk(
        &mut self,
        id: ResourceId,
        revision: i64,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let stage = self
            .entry_mut(id)?
            .staging
            .as_mut()
            .ok_or(Error::NotReady)?;
        if stage.update.revision != revision {
            return Err(Error::InvalidRevision);
        }
        if bytes.is_empty()
            || bytes.len() > MAX_CHUNK_BYTES
            || offset != stage.suffix.len()
            || bytes.len() > stage.update.suffix_bytes as usize - stage.suffix.len()
        {
            return Err(Error::InvalidRange);
        }
        // Byte chunks may split Unicode; validate only the assembled suffix.
        stage.suffix.extend_from_slice(bytes);
        Ok(())
    }
    pub fn publish(&mut self, id: ResourceId, revision: i64) -> Result<(), Error> {
        let entry = self.entry_mut(id)?;
        let stage = entry.staging.as_ref().ok_or(Error::NotReady)?;
        if stage.update.revision != revision {
            return Err(Error::InvalidRevision);
        }
        if stage.suffix.len() != stage.update.suffix_bytes as usize {
            return Err(Error::Incomplete);
        }
        let suffix = std::str::from_utf8(&stage.suffix).map_err(|_| Error::InvalidUtf8)?;
        let mut text = entry.lease.snapshot().text.clone();
        let from = stage.update.from_byte as usize;
        text.remove(from..);
        text.insert(from, suffix);
        let stage = entry.staging.take().expect("validated stage");
        *entry.lease.0.borrow_mut() = Arc::new(Snapshot {
            revision,
            generation: stage.update.generation,
            status: stage.update.status,
            text,
            unchanged_bytes: from,
            _reservation: stage.reservation,
        });
        Ok(())
    }
    pub fn abort(&mut self, id: ResourceId, revision: i64) -> Result<(), Error> {
        let entry = self.entry_mut(id)?;
        match &entry.staging {
            Some(stage) if stage.update.revision == revision => {
                entry.staging = None;
                Ok(())
            }
            Some(_) => Err(Error::InvalidRevision),
            None => Err(Error::NotReady),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    fn update(id: ResourceId, base: i64, from: i64, bytes: i64) -> Update {
        Update {
            id,
            base,
            revision: base + 1,
            generation: 1,
            from_byte: from,
            suffix_bytes: bytes,
            status: Status::Streaming,
        }
    }
    #[test]
    fn unicode_staging_is_atomic_and_old_snapshots_survive() {
        let mut store = Store::default();
        let id = store.create().unwrap();
        let lease = store.acquire(id).unwrap();
        store.begin(update(id, 0, 0, 4)).unwrap();
        store.chunk(id, 1, 0, &[0xf0, 0x9f]).unwrap();
        assert_eq!(store.publish(id, 1), Err(Error::Incomplete));
        assert_eq!(lease.snapshot().text.len(), 0);
        store.chunk(id, 1, 2, &[0xa6, 0x80]).unwrap();
        store.publish(id, 1).unwrap();
        let old = lease.snapshot();
        assert_eq!(old.text.to_string(), "🦀");
        store.begin(update(id, 1, 4, 1)).unwrap();
        store.chunk(id, 2, 0, b"!").unwrap();
        store.publish(id, 2).unwrap();
        assert_eq!(lease.snapshot().text.to_string(), "🦀!");
        assert_eq!(old.text.to_string(), "🦀");
        store.release(id).unwrap();
        assert!(store.acquire(id).is_err());
        assert!(store.reserved_bytes() > 0);
        drop(old);
        drop(lease);
        assert_eq!(store.reserved_bytes(), 0);
    }
    #[test]
    fn stale_generation_and_bad_utf8_never_publish() {
        let mut store = Store::default();
        let id = store.create().unwrap();
        let lease = store.acquire(id).unwrap();
        store.begin(update(id, 0, 0, 1)).unwrap();
        store.chunk(id, 1, 0, &[0xff]).unwrap();
        assert_eq!(store.publish(id, 1), Err(Error::InvalidUtf8));
        assert_eq!(lease.snapshot().revision, 0);
        assert_eq!(store.abort(id, 2), Err(Error::InvalidRevision));
        store.abort(id, 1).unwrap();
        assert_eq!(store.reserved_bytes(), SNAPSHOT_CHARGE);
        store.release(id).unwrap();
        let replacement = store.create().unwrap();
        assert_ne!(id, replacement);
        assert_eq!(store.release(id), Err(Error::StaleHandle));
        assert!(store.acquire(replacement).is_ok());
    }
    #[test]
    fn pending_work_and_retained_snapshots_are_charged() {
        let mut store = Store::default();
        let ids: Vec<_> = (0..9).map(|_| store.create().unwrap()).collect();
        for id in &ids[..8] {
            store.begin(update(*id, 0, 0, 1)).unwrap();
        }
        assert_eq!(
            store.begin(update(ids[8], 0, 0, 1)),
            Err(Error::ResourceLimit)
        );
        store.close();
        assert_eq!(store.reserved_bytes(), 0);
        assert_eq!(store.create(), Err(Error::Closed));
    }
}
