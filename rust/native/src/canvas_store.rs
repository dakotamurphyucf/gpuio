//! Main-thread scene registrations. Publication is atomic; snapshots and image
//! leases remain charged/readable until their final reader drops them.
use crate::asset_store;
use binprot::BinProtWrite;
use gpuio_protocol::{
    ResourceId,
    canvas_resource::{Error, MAX_CHUNK_BYTES, Update},
    canvas_scene::{MAX_BYTES, MAX_RESOURCES, ResourceData, Scene},
    decode_canvas_scene,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    mem::size_of,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub const MAX_SCENES: usize = 256;
pub const MAX_STAGING: usize = 4;
pub const MAX_RESERVED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_HISTORY_BYTES: usize = 4 * 1024 * 1024;
const FIXED_CHARGE: usize = 4096;

// Publication is synchronous on the native thread. Before the final quota
// reservation succeeds, decoding/validation and candidate-history construction
// have independently bounded temporary capacities. This is an accounting bound,
// not an allocator/RSS limit, and excludes the separately charged input buffer.
pub const MAX_PUBLISH_WORKSPACE_BYTES: usize = 64 * 1024 * 1024;
const PUBLISH_WORKSPACE_BOUND: usize = FIXED_CHARGE
    + gpuio_protocol::canvas_scene::MAX_ITEMS * size_of::<gpuio_protocol::canvas_scene::Item>()
    + MAX_RESOURCES * size_of::<gpuio_protocol::canvas_scene::Resource>()
    + gpuio_protocol::canvas_scene::MAX_ITEMS * gpuio_protocol::canvas_scene::MAX_CLIPS * size_of::<gpuio_protocol::canvas::Rect>()
    + gpuio_protocol::canvas_scene::MAX_INTERACTIVE_ITEMS * 256 * size_of::<gpuio_protocol::canvas::Point>()
    + gpuio_protocol::canvas_scene::MAX_PATH_COMMANDS * size_of::<gpuio_protocol::canvas::PathCommand>()
    + gpuio_protocol::canvas_scene::MAX_TEXT_BYTES
    // Admission maps/sets, history map with up to old+new resource IDs, image map.
    + (gpuio_protocol::canvas_scene::MAX_ITEMS + MAX_RESOURCES * 5) * 128
    // Cloned prior canonical history plus new serialized values (Vec growth).
    + MAX_HISTORY_BYTES + MAX_BYTES * 2 + MAX_RESOURCES * 8;
const _: () = assert!(PUBLISH_WORKSPACE_BOUND <= MAX_PUBLISH_WORKSPACE_BYTES);

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
    pub scene: Scene,
    pub encoded_bytes: usize,
    pub images: BTreeMap<ResourceId, asset_store::Lease>,
    _reservation: Reservation,
}
#[derive(Clone)]
pub struct Lease(Rc<RefCell<Option<Arc<Snapshot>>>>);
impl Lease {
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.0
            .borrow()
            .as_ref()
            .expect("only published scenes issue leases")
            .clone()
    }
}
#[derive(Clone)]
struct History {
    generation: i64,
    canonical: Vec<u8>,
}
struct Staging {
    update: Update,
    bytes: Vec<u8>,
    _reservation: Reservation,
}
struct Entry {
    lease: Lease,
    history: BTreeMap<i64, History>,
    _history_reservation: Reservation,
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

fn scene_charge(scene: &Scene) -> usize {
    use gpuio_protocol::canvas::{HitRegion, PathCommand, Point, Rect};
    use gpuio_protocol::canvas_scene::{Item, Resource};
    let mut bytes = FIXED_CHARGE
        + size_of::<Scene>()
        + scene.description.capacity()
        + scene.items.capacity() * size_of::<Item>()
        + scene.resources.capacity() * size_of::<Resource>();
    for resource in &scene.resources {
        bytes += match &resource.data {
            ResourceData::Path(path) => path.0.capacity() * size_of::<PathCommand>(),
            ResourceData::Text(text) => text.value.capacity() + text.font_family.capacity(),
            ResourceData::Image(_) => 128, // lease/map bookkeeping; encoded assets charge their own store
        };
    }
    for item in &scene.items {
        bytes += item.clips.capacity() * size_of::<Rect>();
        if let Some(interaction) = &item.interaction {
            bytes += interaction.label.capacity();
            if let HitRegion::Polygon(points) = &interaction.hit_region {
                bytes += points.capacity() * size_of::<Point>();
            }
        }
    }
    bytes
}

impl Store {
    pub fn reserved_bytes(&self) -> usize {
        self.reserved.load(Ordering::Relaxed)
    }
    pub fn staged_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.entry.as_ref().is_some_and(|e| e.staging.is_some()))
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
        if index >= MAX_SCENES {
            return Err(Error::ResourceLimit);
        }
        let reservation = Reservation::new(&self.reserved, FIXED_CHARGE)?;
        if index == self.slots.len() {
            self.slots.push(Slot::default());
        }
        let slot = &mut self.slots[index];
        slot.generation += 1;
        slot.entry = Some(Entry {
            lease: Lease(Rc::new(RefCell::new(None))),
            history: BTreeMap::new(),
            _history_reservation: reservation,
            staging: None,
        });
        Ok(
            ResourceId::from_parts(index as i64, i64::from(slot.generation))
                .expect("bounded canvas slot"),
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
            .expect("validated scene"))
    }
    pub fn acquire(&self, id: ResourceId) -> Result<Lease, Error> {
        let entry = self.entry(id)?;
        if entry.lease.0.borrow().is_none() {
            return Err(Error::NotReady);
        }
        Ok(entry.lease.clone())
    }
    pub fn begin(&mut self, update: Update) -> Result<(), Error> {
        let entry = self.entry(update.id)?;
        if entry.staging.is_some() {
            return Err(Error::Busy);
        }
        let previous = entry.lease.0.borrow();
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
        if count == 0 || count > MAX_BYTES {
            return Err(Error::InvalidRange);
        }
        if self.staged_count() >= MAX_STAGING {
            return Err(Error::ResourceLimit);
        }
        let reservation = Reservation::new(&self.reserved, FIXED_CHARGE + count)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        drop(previous);
        let id = update.id;
        self.entry_mut(id)?.staging = Some(Staging {
            update,
            bytes,
            _reservation: reservation,
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
            || offset != stage.bytes.len()
            || bytes.len() > stage.update.bytes as usize - stage.bytes.len()
        {
            return Err(Error::InvalidRange);
        }
        stage.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub fn publish(
        &mut self,
        id: ResourceId,
        revision: i64,
        assets: &asset_store::Store,
    ) -> Result<(), Error> {
        let entry = self.entry(id)?;
        let stage = entry.staging.as_ref().ok_or(Error::NotReady)?;
        if stage.update.revision != revision {
            return Err(Error::InvalidRevision);
        }
        if stage.bytes.len() != stage.update.bytes as usize {
            return Err(Error::Incomplete);
        }
        let scene = decode_canvas_scene(&stage.bytes).map_err(|error| match error {
            gpuio_protocol::DecodeError::LimitExceeded => Error::ResourceLimit,
            gpuio_protocol::DecodeError::Malformed => Error::InvalidScene,
        })?;
        let previous = entry.lease.0.borrow();
        let reset = previous
            .as_ref()
            .is_some_and(|s| s.generation != stage.update.generation);
        let mut history = if reset {
            BTreeMap::new()
        } else {
            entry.history.clone()
        };
        let mut images = BTreeMap::new();
        for resource in &scene.resources {
            let mut canonical = Vec::new();
            resource
                .data
                .binprot_write(&mut canonical)
                .map_err(|_| Error::NativeFailure)?;
            if let Some(old) = history.get(&resource.key.id)
                && (resource.key.generation < old.generation
                    || (resource.key.generation == old.generation && old.canonical != canonical))
            {
                return Err(Error::StaleResource);
            }
            history.insert(
                resource.key.id,
                History {
                    generation: resource.key.generation,
                    canonical,
                },
            );
            if let ResourceData::Image(asset) = resource.data {
                let lease = previous
                    .as_ref()
                    .and_then(|s| s.images.get(&asset))
                    .cloned()
                    .map_or_else(
                        || assets.acquire(asset).map_err(|_| Error::UnavailableImage),
                        Ok,
                    )?;
                images.insert(asset, lease);
            }
        }
        if history.len() > MAX_RESOURCES {
            return Err(Error::ResourceLimit);
        }
        let history_bytes = history
            .values()
            .map(|h| h.canonical.capacity())
            .sum::<usize>();
        if history_bytes > MAX_HISTORY_BYTES {
            return Err(Error::ResourceLimit);
        }
        let history_reservation = Reservation::new(
            &self.reserved,
            FIXED_CHARGE + history_bytes + history.len() * 128,
        )?;
        let reservation = Reservation::new(&self.reserved, scene_charge(&scene))?;
        let snapshot = Arc::new(Snapshot {
            revision,
            generation: stage.update.generation,
            encoded_bytes: stage.bytes.len(),
            scene,
            images,
            _reservation: reservation,
        });
        drop(previous);
        let entry = self.entry_mut(id)?;
        *entry.lease.0.borrow_mut() = Some(snapshot);
        entry.history = history;
        entry._history_reservation = history_reservation;
        entry.staging = None;
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
    use gpuio_protocol::{asset::Format, canvas::*, canvas_scene::*};
    fn scene() -> Scene {
        Scene {
            version: 1,
            description: "Canvas".into(),
            resources: vec![],
            items: vec![Item {
                id: 1,
                transform: Transform::IDENTITY,
                clips: vec![],
                drawing: Drawing::Shape(
                    Shape::Rectangle(Rect {
                        x: 0.,
                        y: 0.,
                        width: 10.,
                        height: 10.,
                    }),
                    Paint {
                        fill: Some(0xff0000ff),
                        stroke: None,
                    },
                ),
                interaction: None,
            }],
        }
    }
    fn bytes(scene: &Scene) -> Vec<u8> {
        let mut bytes = Vec::new();
        scene.binprot_write(&mut bytes).unwrap();
        bytes
    }
    fn stage(store: &mut Store, id: ResourceId, base: i64, generation: i64, scene: &Scene) {
        let bytes = bytes(scene);
        store
            .begin(Update {
                id,
                base,
                revision: base + 1,
                generation,
                bytes: bytes.len() as i64,
            })
            .unwrap();
        for (index, chunk) in bytes.chunks(MAX_CHUNK_BYTES).enumerate() {
            store
                .chunk(id, base + 1, index * MAX_CHUNK_BYTES, chunk)
                .unwrap();
        }
    }
    fn publish(
        store: &mut Store,
        id: ResourceId,
        base: i64,
        generation: i64,
        scene: &Scene,
        assets: &asset_store::Store,
    ) {
        stage(store, id, base, generation, scene);
        store.publish(id, base + 1, assets).unwrap();
    }
    #[test]
    fn publication_is_atomic_and_old_readers_remain_charged_after_release() {
        let mut store = Store::default();
        let assets = asset_store::Store::default();
        let id = store.create().unwrap();
        assert!(matches!(store.acquire(id), Err(Error::NotReady)));
        publish(&mut store, id, 0, 1, &scene(), &assets);
        let lease = store.acquire(id).unwrap();
        let old = lease.snapshot();
        let mut next = scene();
        next.description = "Changed".into();
        let encoded = bytes(&next);
        store
            .begin(Update {
                id,
                base: 1,
                revision: 2,
                generation: 1,
                bytes: encoded.len() as i64,
            })
            .unwrap();
        let split = encoded.len() / 2;
        store.chunk(id, 2, 0, &encoded[..split]).unwrap();
        assert_eq!(store.publish(id, 2, &assets), Err(Error::Incomplete));
        assert_eq!(lease.snapshot().scene.description, "Canvas");
        store.chunk(id, 2, split, &encoded[split..]).unwrap();
        store.publish(id, 2, &assets).unwrap();
        assert_eq!(lease.snapshot().scene.description, "Changed");
        assert_eq!(old.scene.description, "Canvas");
        store.release(id).unwrap();
        assert!(matches!(store.acquire(id), Err(Error::StaleHandle)));
        let charged = store.reserved_bytes();
        assert!(charged > 0);
        drop(old);
        assert!(store.reserved_bytes() < charged);
        assert_eq!(lease.snapshot().revision, 2);
        drop(lease);
        assert_eq!(store.reserved_bytes(), 0);
    }
    #[test]
    fn ordered_uploads_abort_stale_ids_and_close_cannot_mutate_a_reused_slot() {
        let mut store = Store::default();
        let id = store.create().unwrap();
        let data = bytes(&scene());
        let update = Update {
            id,
            base: 0,
            revision: 1,
            generation: 1,
            bytes: data.len() as i64,
        };
        store.begin(update.clone()).unwrap();
        assert_eq!(store.begin(update), Err(Error::Busy));
        assert_eq!(store.chunk(id, 2, 0, &data), Err(Error::InvalidRevision));
        assert_eq!(store.chunk(id, 1, 1, &data), Err(Error::InvalidRange));
        assert_eq!(store.chunk(id, 1, 0, &[]), Err(Error::InvalidRange));
        assert_eq!(store.abort(id, 2), Err(Error::InvalidRevision));
        store.abort(id, 1).unwrap();
        assert_eq!(store.staged_count(), 0);
        store.release(id).unwrap();
        let replacement = store.create().unwrap();
        assert_eq!(replacement.slot(), id.slot());
        assert_ne!(replacement.generation(), id.generation());
        assert_eq!(store.release(id), Err(Error::StaleHandle));
        store.close();
        assert_eq!(store.create(), Err(Error::Closed));
        assert_eq!(store.release(replacement), Err(Error::Closed));
        assert_eq!(store.reserved_bytes(), 0);
    }
    fn with_text(generation: i64, value: &str) -> Scene {
        let mut scene = scene();
        scene.resources.push(Resource {
            key: ResourceKey { id: 1, generation },
            data: ResourceData::Text(Text {
                value: value.into(),
                font_family: "system".into(),
                font_size: 14.,
                font_weight: 400,
            }),
        });
        scene
    }
    #[test]
    fn generation_history_survives_removal_and_reset_is_explicit() {
        let mut store = Store::default();
        let assets = asset_store::Store::default();
        let id = store.create().unwrap();
        publish(&mut store, id, 0, 1, &with_text(1, "one"), &assets);
        let lease = store.acquire(id).unwrap();
        stage(&mut store, id, 1, 1, &with_text(1, "changed"));
        assert_eq!(store.publish(id, 2, &assets), Err(Error::StaleResource));
        assert_eq!(lease.snapshot().revision, 1);
        store.abort(id, 2).unwrap();
        publish(&mut store, id, 1, 1, &with_text(2, "changed"), &assets);
        publish(&mut store, id, 2, 1, &scene(), &assets);
        stage(&mut store, id, 3, 1, &with_text(1, "one"));
        assert_eq!(store.publish(id, 4, &assets), Err(Error::StaleResource));
        store.abort(id, 4).unwrap();
        stage(&mut store, id, 3, 1, &with_text(2, "different"));
        assert_eq!(store.publish(id, 4, &assets), Err(Error::StaleResource));
        store.abort(id, 4).unwrap();
        publish(&mut store, id, 3, 2, &with_text(1, "reset"), &assets);
        assert_eq!(lease.snapshot().generation, 2);
        let before = store.reserved_bytes();
        assert_eq!(
            store.begin(Update {
                id,
                base: 3,
                revision: 5,
                generation: 2,
                bytes: 1
            }),
            Err(Error::InvalidRevision)
        );
        assert_eq!(
            store.begin(Update {
                id,
                base: 4,
                revision: 5,
                generation: 4,
                bytes: 1
            }),
            Err(Error::InvalidRevision)
        );
        assert_eq!(store.reserved_bytes(), before);
    }
    #[test]
    fn scene_image_leases_survive_asset_release_and_fail_new_stale_binds() {
        let mut assets = asset_store::Store::default();
        let asset = assets.begin(Format::Png, 3).unwrap();
        assets.append(asset, 0, b"png").unwrap();
        assets.finish(asset).unwrap();
        let mut scene = scene();
        scene.resources.push(Resource {
            key: ResourceKey {
                id: 1,
                generation: 1,
            },
            data: ResourceData::Image(asset),
        });
        let mut store = Store::default();
        let id = store.create().unwrap();
        publish(&mut store, id, 0, 1, &scene, &assets);
        let lease = store.acquire(id).unwrap();
        assets.release(asset).unwrap();
        assert_eq!(lease.snapshot().images[&asset].source().as_bytes(), b"png");
        scene.description = "Updated layout".into();
        publish(&mut store, id, 1, 1, &scene, &assets);
        let other = store.create().unwrap();
        stage(&mut store, other, 0, 1, &scene);
        assert_eq!(
            store.publish(other, 1, &assets),
            Err(Error::UnavailableImage)
        );
        assert!(matches!(store.acquire(other), Err(Error::NotReady)));
        store.release(other).unwrap();
        store.release(id).unwrap();
        assets.collect();
        assert_eq!(assets.stats().retired, 1);
        drop(lease);
        assets.collect();
        assert_eq!(assets.stats().reserved_bytes, 0);
        assert_eq!(store.reserved_bytes(), 0);
    }
    #[test]
    fn staging_and_history_limits_recover_after_abort_and_generation_reset() {
        let mut store = Store::default();
        let assets = asset_store::Store::default();
        let ids = (0..MAX_STAGING + 1)
            .map(|_| store.create().unwrap())
            .collect::<Vec<_>>();
        for &id in &ids[..MAX_STAGING] {
            stage(&mut store, id, 0, 1, &scene());
        }
        assert_eq!(
            store.begin(Update {
                id: ids[MAX_STAGING],
                base: 0,
                revision: 1,
                generation: 1,
                bytes: 10
            }),
            Err(Error::ResourceLimit)
        );
        for &id in &ids[..MAX_STAGING] {
            store.abort(id, 1).unwrap();
        }
        let id = ids[0];
        let mut scene = scene();
        let template = with_text(1, "x").resources.remove(0);
        scene.resources = (1..=MAX_RESOURCES)
            .map(|id| Resource {
                key: ResourceKey {
                    id: id as i64,
                    generation: 1,
                },
                ..template.clone()
            })
            .collect();
        publish(&mut store, id, 0, 1, &scene, &assets);
        scene.resources = vec![Resource {
            key: ResourceKey {
                id: MAX_RESOURCES as i64 + 1,
                generation: 1,
            },
            ..template
        }];
        stage(&mut store, id, 1, 1, &scene);
        assert_eq!(store.publish(id, 2, &assets), Err(Error::ResourceLimit));
        store.abort(id, 2).unwrap();
        publish(&mut store, id, 1, 2, &scene, &assets);
        for id in ids {
            store.release(id).unwrap();
        }
        assert_eq!(store.reserved_bytes(), 0);
    }
    #[test]
    fn malformed_publication_preserves_prior_scene_and_staging_is_explicitly_releasable() {
        let mut store = Store::default();
        let assets = asset_store::Store::default();
        let id = store.create().unwrap();
        publish(&mut store, id, 0, 1, &scene(), &assets);
        let lease = store.acquire(id).unwrap();
        let before = store.reserved_bytes();
        store
            .begin(Update {
                id,
                base: 1,
                revision: 2,
                generation: 1,
                bytes: 1,
            })
            .unwrap();
        store.chunk(id, 2, 0, &[255]).unwrap();
        assert_eq!(store.publish(id, 2, &assets), Err(Error::InvalidScene));
        assert_eq!(lease.snapshot().revision, 1);
        store.abort(id, 2).unwrap();
        assert_eq!(store.reserved_bytes(), before);
        store.close();
        assert_eq!(lease.snapshot().revision, 1);
        drop(lease);
        assert_eq!(store.reserved_bytes(), 0);
    }

    #[test]
    fn held_large_snapshots_enforce_global_quota_then_recover_without_reupload() {
        let mut store = Store::default();
        let assets = asset_store::Store::default();
        let id = store.create().unwrap();
        let mut scene = scene();
        let template = scene.items.remove(0);
        scene.items = (1..=MAX_ITEMS)
            .map(|id| Item {
                id: id as i64,
                ..template.clone()
            })
            .collect();
        publish(&mut store, id, 0, 1, &scene, &assets);
        let lease = store.acquire(id).unwrap();
        let mut retained = Vec::new();
        let mut revision = 1;
        let mut stopped = false;
        for _ in 0..128 {
            retained.push(lease.snapshot());
            stage(&mut store, id, revision, 1, &scene);
            match store.publish(id, revision + 1, &assets) {
                Ok(()) => revision += 1,
                Err(Error::ResourceLimit) => {
                    stopped = true;
                    break;
                }
                Err(error) => panic!("unexpected publication failure: {error:?}"),
            }
            assert!(store.reserved_bytes() <= MAX_RESERVED_BYTES);
        }
        assert!(stopped);
        assert_eq!(lease.snapshot().revision, revision);
        let peak = store.reserved_bytes();
        assert!(peak <= MAX_RESERVED_BYTES);
        println!(
            "canvas retained-snapshot pressure: revision {revision}, {} held readers, {peak} charged bytes",
            retained.len()
        );
        retained.clear();
        assert!(store.reserved_bytes() < peak);
        store.publish(id, revision + 1, &assets).unwrap();
        assert_eq!(lease.snapshot().revision, revision + 1);
        store.release(id).unwrap();
        drop(lease);
        assert_eq!(store.reserved_bytes(), 0);
    }

    #[test]
    fn removed_resource_history_bytes_are_bounded_and_reset_reclaims_them() {
        let mut store = Store::default();
        let assets = asset_store::Store::default();
        let id = store.create().unwrap();
        let mut scene = scene();
        let mut revision = 0;
        let mut limited = false;
        for group in 0..10 {
            scene.resources = (1..=60)
                .map(|offset| Resource {
                    key: ResourceKey {
                        id: group * 60 + offset,
                        generation: 1,
                    },
                    data: ResourceData::Text(Text {
                        value: "x".repeat(16_000),
                        font_family: "system".into(),
                        font_size: 14.,
                        font_weight: 400,
                    }),
                })
                .collect();
            stage(&mut store, id, revision, 1, &scene);
            match store.publish(id, revision + 1, &assets) {
                Ok(()) => revision += 1,
                Err(Error::ResourceLimit) => {
                    limited = true;
                    break;
                }
                Err(error) => panic!("unexpected history failure: {error:?}"),
            }
        }
        assert!(limited);
        assert!(revision > 0);
        assert_eq!(store.acquire(id).unwrap().snapshot().revision, revision);
        store.abort(id, revision + 1).unwrap();
        publish(&mut store, id, revision, 2, &scene, &assets);
        store.release(id).unwrap();
        assert_eq!(store.reserved_bytes(), 0);
    }

    #[test]
    fn session_negotiation_owns_canvas_requests_and_shutdown_closes_acquisition() {
        use crate::session::Session;
        use gpuio_protocol::{
            asset::Chunk,
            canvas_resource::{Request, Response},
            v1::{CAPABILITIES, VERSION},
        };
        let mut session = Session::default();
        assert_eq!(
            session.canvas_request(Request::Create),
            Response::Failed(Error::NotReady)
        );
        session.hello(VERSION, CAPABILITIES).unwrap();
        let Response::Created(id) = session.canvas_request(Request::Create) else {
            panic!("expected scene handle")
        };
        let encoded = bytes(&scene());
        assert_eq!(
            session.canvas_request(Request::Begin(Update {
                id,
                base: 0,
                revision: 1,
                generation: 1,
                bytes: encoded.len() as i64
            })),
            Response::Ack
        );
        assert_eq!(
            session.canvas_request(Request::Chunk(id, 1, -1, Chunk::new(vec![0]).unwrap())),
            Response::Failed(Error::InvalidRange)
        );
        assert_eq!(
            session.canvas_request(Request::Chunk(id, 1, 0, Chunk::new(encoded).unwrap())),
            Response::Ack
        );
        assert_eq!(
            session.canvas_request(Request::Publish(id, 1)),
            Response::Ack
        );
        let lease = session.canvas(id).unwrap();
        session.shutdown();
        assert_eq!(
            session.canvas_request(Request::Create),
            Response::Failed(Error::Closed)
        );
        assert!(matches!(session.canvas(id), Err(Error::Closed)));
        assert_eq!(lease.snapshot().scene.description, "Canvas");
    }
}
