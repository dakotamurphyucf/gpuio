//! Encoded asset registration, independent of decoding and rendering.
//! A retired registration cannot issue new leases; existing leases keep its
//! bytes charged until the last reader (view or worker) drops them.
use gpuio_protocol::{
    ResourceId,
    asset::{Format, MAX_ENCODED_BYTES, Source},
};
use std::sync::{Arc, Weak};

pub const MAX_ASSETS: usize = 1024;
pub const MAX_UPLOADS: usize = 8;
pub const MAX_RESERVED_BYTES: usize = 64 * 1024 * 1024;
pub use gpuio_protocol::asset::{Error, MAX_CHUNK_BYTES};

#[derive(Clone, Debug)]
pub struct Lease {
    id: ResourceId,
    source: Arc<Source>,
}
impl Lease {
    pub fn id(&self) -> ResourceId {
        self.id
    }
    pub fn source(&self) -> &Source {
        &self.source
    }
}

struct Upload {
    format: Format,
    expected: usize,
    data: Vec<u8>,
}
enum Entry {
    Upload(Upload),
    Available(Arc<Source>),
    Retired(Weak<Source>),
}
#[derive(Default)]
struct Slot {
    generation: u32,
    reserved: usize,
    entry: Option<Entry>,
}
#[derive(Clone, Copy)]
struct Limits {
    assets: usize,
    uploads: usize,
    bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            assets: MAX_ASSETS,
            uploads: MAX_UPLOADS,
            bytes: MAX_RESERVED_BYTES,
        }
    }
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub uploading: usize,
    pub available: usize,
    pub retired: usize,
    pub reserved_bytes: usize,
}
#[derive(Default)]
pub struct Store {
    slots: Vec<Slot>,
    limits: Limits,
    closed: bool,
}
impl Store {
    /// Reserve the entire declared encoded length before accepting any bytes.
    /// Native-issued IDs include a generation; failed starts expose no handle.
    pub fn begin(&mut self, format: Format, length: usize) -> Result<ResourceId, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if length == 0 || length > MAX_ENCODED_BYTES {
            return Err(Error::InvalidSize);
        }
        let stats = self.stats();
        if stats.uploading >= self.limits.uploads
            || length > self.limits.bytes.saturating_sub(stats.reserved_bytes)
        {
            return Err(Error::ResourceLimit);
        }
        let index = self
            .slots
            .iter()
            .position(|slot| slot.entry.is_none() && slot.generation < u32::MAX)
            .unwrap_or(self.slots.len());
        if index >= self.limits.assets {
            return Err(Error::ResourceLimit);
        }
        let mut data = Vec::new();
        data.try_reserve_exact(length)
            .map_err(|_| Error::ResourceLimit)?;
        if index == self.slots.len() {
            self.slots.push(Slot::default());
        }
        let slot = &mut self.slots[index];
        slot.generation += 1; // Exhausted generations are never selected above.
        slot.reserved = length;
        slot.entry = Some(Entry::Upload(Upload {
            format,
            expected: length,
            data,
        }));
        Ok(ResourceId::from_parts(index as i64, slot.generation as i64)
            .expect("bounded native asset ID"))
    }

    fn slot(&self, id: ResourceId) -> Result<&Slot, Error> {
        self.slots
            .get(id.slot())
            .filter(|slot| slot.generation == id.generation())
            .ok_or(Error::StaleHandle)
    }

    /// Ordered, nonempty chunks only. A malformed chunk aborts the matching
    /// upload; stale IDs never mutate a new generation in the same slot.
    pub fn append(&mut self, id: ResourceId, offset: usize, bytes: &[u8]) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let Some(Entry::Upload(upload)) = &self.slot(id)?.entry else {
            return Err(Error::NotUploading);
        };
        if bytes.is_empty()
            || bytes.len() > MAX_CHUNK_BYTES
            || offset != upload.data.len()
            || bytes.len() > upload.expected - upload.data.len()
        {
            self.release(id)?;
            return Err(Error::InvalidChunk);
        }
        let Some(Entry::Upload(upload)) = &mut self.slots[id.slot()].entry else {
            unreachable!()
        };
        upload.data.extend_from_slice(bytes);
        Ok(())
    }

    /// Publish complete encoded data. This does not validate or decode pixels.
    /// Finishing a prefix aborts it; callers must start a new generation.
    pub fn finish(&mut self, id: ResourceId) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let Some(Entry::Upload(upload)) = &self.slot(id)?.entry else {
            return Err(Error::NotUploading);
        };
        if upload.data.len() != upload.expected {
            self.release(id)?;
            return Err(Error::Incomplete);
        }
        let slot = &mut self.slots[id.slot()];
        let Some(Entry::Upload(upload)) = slot.entry.take() else {
            unreachable!()
        };
        let source = Source::new(upload.format, upload.data).expect("validated encoded length");
        slot.entry = Some(Entry::Available(Arc::new(source)));
        Ok(())
    }

    pub fn acquire(&self, id: ResourceId) -> Result<Lease, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let Some(Entry::Available(source)) = &self.slot(id)?.entry else {
            return Err(Error::StaleHandle);
        };
        Ok(Lease {
            id,
            source: source.clone(),
        })
    }

    /// Abort staging or retire a registration. Repeated cleanup of the same
    /// generation succeeds; cleanup of an older generation cannot affect reuse.
    /// Existing leases remain readable and charged, but no new acquisition is allowed.
    pub fn release(&mut self, id: ResourceId) -> Result<(), Error> {
        self.slot(id)?;
        let slot = &mut self.slots[id.slot()];
        slot.entry = match slot.entry.take() {
            Some(Entry::Available(source)) => Some(Entry::Retired(Arc::downgrade(&source))),
            Some(Entry::Retired(source)) => Some(Entry::Retired(source)),
            Some(Entry::Upload(_)) | None => {
                slot.reserved = 0;
                None
            }
        };
        self.collect();
        Ok(())
    }

    pub fn collect(&mut self) {
        for slot in &mut self.slots {
            if matches!(&slot.entry, Some(Entry::Retired(source)) if source.strong_count() == 0) {
                slot.entry = None;
                slot.reserved = 0;
            }
        }
    }

    /// Terminal: cancel staging and retire available registrations. Mounted
    /// readers are disposed by their owners; their memory remains accounted here.
    pub fn close(&mut self) {
        self.closed = true;
        for slot in &mut self.slots {
            slot.entry = match slot.entry.take() {
                Some(Entry::Available(source)) => Some(Entry::Retired(Arc::downgrade(&source))),
                Some(Entry::Retired(source)) => Some(Entry::Retired(source)),
                Some(Entry::Upload(_)) | None => {
                    slot.reserved = 0;
                    None
                }
            };
        }
        self.collect();
    }

    pub fn stats(&mut self) -> Stats {
        self.collect();
        let mut stats = Stats::default();
        for slot in &self.slots {
            stats.reserved_bytes += slot.reserved;
            match &slot.entry {
                Some(Entry::Upload(_)) => stats.uploading += 1,
                Some(Entry::Available(_)) => stats.available += 1,
                Some(Entry::Retired(_)) => stats.retired += 1,
                None => {}
            }
        }
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limited(assets: usize, uploads: usize, bytes: usize) -> Store {
        Store {
            limits: Limits {
                assets,
                uploads,
                bytes,
            },
            ..Store::default()
        }
    }
    fn ready(store: &mut Store, data: &[u8]) -> ResourceId {
        let id = store.begin(Format::Png, data.len()).unwrap();
        store.append(id, 0, data).unwrap();
        store.finish(id).unwrap();
        id
    }

    #[test]
    fn assets_larger_than_an_envelope_assemble_without_publishing_a_prefix() {
        let mut store = Store::default();
        let bytes: Vec<_> = (0..2 * 1024 * 1024 + 13).map(|i| (i % 256) as u8).collect();
        let id = store.begin(Format::Gif, bytes.len()).unwrap();
        for (index, chunk) in bytes.chunks(MAX_CHUNK_BYTES).enumerate() {
            assert!(matches!(store.acquire(id), Err(Error::StaleHandle)));
            store.append(id, index * MAX_CHUNK_BYTES, chunk).unwrap();
        }
        assert_eq!(store.stats().reserved_bytes, bytes.len());
        store.finish(id).unwrap();
        let lease = store.acquire(id).unwrap();
        assert_eq!(lease.id(), id);
        assert_eq!(lease.source().format(), Format::Gif);
        assert_eq!(lease.source().as_bytes(), bytes);
        assert_eq!(store.finish(id), Err(Error::NotUploading));
    }

    #[test]
    fn malformed_uploads_reclaim_reservations_and_stale_cleanup_cannot_touch_reuse() {
        let mut store = limited(1, 1, MAX_CHUNK_BYTES + 2);
        for (offset, chunk) in [
            (1, vec![1]),
            (0, vec![]),
            (0, vec![1; 5]),
            (0, vec![1; MAX_CHUNK_BYTES + 1]),
        ] {
            let id = store.begin(Format::Png, 4).unwrap();
            assert_eq!(store.append(id, offset, &chunk), Err(Error::InvalidChunk));
            assert_eq!(store.stats(), Stats::default());
            assert!(store.acquire(id).is_err());
        }
        let old = store.begin(Format::Png, 4).unwrap();
        store.append(old, 0, &[0, 255]).unwrap();
        assert_eq!(store.finish(old), Err(Error::Incomplete));
        let current = ready(&mut store, &[3, 4]);
        assert_eq!(old.slot(), current.slot());
        assert!(old.generation() < current.generation());
        assert_eq!(store.append(old, 2, &[1, 2]), Err(Error::StaleHandle));
        assert_eq!(store.finish(old), Err(Error::StaleHandle));
        assert_eq!(store.release(old), Err(Error::StaleHandle));
        assert_eq!(store.acquire(current).unwrap().source().as_bytes(), &[3, 4]);
        store.release(current).unwrap();
        store.release(current).unwrap();
        assert_eq!(store.stats(), Stats::default());
    }

    #[test]
    fn live_retired_leases_still_count_toward_the_budget() {
        let mut store = limited(2, 1, 8);
        let id = store.begin(Format::Svg, 8).unwrap();
        assert_eq!(store.begin(Format::Png, 1), Err(Error::ResourceLimit));
        store.append(id, 0, b"12345678").unwrap();
        store.finish(id).unwrap();
        let lease = store.acquire(id).unwrap();
        let worker = lease.clone();
        store.release(id).unwrap();
        assert!(store.acquire(id).is_err());
        drop(lease);
        assert_eq!(
            store.stats(),
            Stats {
                retired: 1,
                reserved_bytes: 8,
                ..Stats::default()
            }
        );
        assert_eq!(store.begin(Format::Png, 1), Err(Error::ResourceLimit));
        assert_eq!(worker.source().as_bytes(), b"12345678");
        drop(worker);
        let next = store.begin(Format::Png, 8).unwrap();
        assert_eq!(next.slot(), id.slot());
        assert_eq!(next.generation(), id.generation() + 1);
        store.release(next).unwrap();
        assert_eq!(store.stats(), Stats::default());
    }

    #[test]
    fn shutdown_retires_readers_and_cancels_staging_without_new_acquisitions() {
        let mut store = limited(2, 2, 8);
        let id = ready(&mut store, &[1, 2]);
        let lease = store.acquire(id).unwrap();
        let pending = store.begin(Format::Png, 4).unwrap();
        store.append(pending, 0, &[3]).unwrap();
        store.close();
        store.close();
        assert_eq!(
            store.stats(),
            Stats {
                retired: 1,
                reserved_bytes: 2,
                ..Stats::default()
            }
        );
        assert_eq!(store.begin(Format::Png, 1), Err(Error::Closed));
        assert_eq!(store.append(pending, 1, &[4]), Err(Error::Closed));
        assert_eq!(store.finish(pending), Err(Error::Closed));
        assert!(matches!(store.acquire(id), Err(Error::Closed)));
        assert_eq!(lease.source().as_bytes(), &[1, 2]);
        drop(lease);
        store.release(id).unwrap();
        assert_eq!(store.stats(), Stats::default());
    }

    #[test]
    fn entry_upload_and_generation_limits_do_not_alias_or_overreserve() {
        let mut store = limited(2, 1, 8);
        assert_eq!(store.begin(Format::Png, 0), Err(Error::InvalidSize));
        assert_eq!(
            store.begin(Format::Png, MAX_ENCODED_BYTES + 1),
            Err(Error::InvalidSize)
        );
        let first = store.begin(Format::Png, 1).unwrap();
        assert_eq!(store.begin(Format::Png, 1), Err(Error::ResourceLimit));
        store.append(first, 0, &[0]).unwrap();
        store.finish(first).unwrap();
        ready(&mut store, &[1]);
        assert_eq!(store.begin(Format::Png, 1), Err(Error::ResourceLimit));
        assert_eq!(store.stats().reserved_bytes, 2);
        store.release(first).unwrap();
        store.slots[first.slot()].generation = u32::MAX;
        assert_eq!(store.begin(Format::Png, 1), Err(Error::ResourceLimit));
        assert_eq!(store.stats().reserved_bytes, 1);
    }
}
