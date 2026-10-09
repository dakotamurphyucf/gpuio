use super::*;
use binprot::BinProtWrite;
use chart_data::{Contents, Layer, Point, Series};

fn bytes(count: usize) -> Vec<u8> {
    let data = Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Cartesian(vec![Layer::Line(Series {
            id: 1,
            name: "Rate".into(),
            points: (0..count)
                .map(|n| Point {
                    id: n as i64 + 1,
                    x: n as f64,
                    y: Some(n as f64),
                    label: String::new(),
                })
                .collect(),
        })]),
    };
    let mut bytes = vec![];
    data.binprot_write(&mut bytes).unwrap();
    bytes
}
fn stage(store: &mut Store, id: ResourceId, revision: i64, generation: i64, bytes: &[u8]) {
    store
        .begin(Update {
            id,
            base: revision - 1,
            revision,
            generation,
            bytes: bytes.len() as i64,
        })
        .unwrap();
    for (n, chunk) in bytes.chunks(MAX_CHUNK_BYTES).enumerate() {
        store
            .chunk(id, revision, n * MAX_CHUNK_BYTES, chunk)
            .unwrap();
    }
}
fn publish(store: &mut Store, id: ResourceId, revision: i64, generation: i64, bytes: &[u8]) {
    stage(store, id, revision, generation, bytes);
    let work = store.publish(id, revision).unwrap();
    store.complete(work.run()).unwrap();
}

#[test]
fn publication_is_atomic_and_readers_remain_charged_until_drop() {
    let mut store = Store::default();
    let id = store.create().unwrap();
    assert!(matches!(store.acquire(id), Err(Error::NotReady)));
    publish(&mut store, id, 1, 1, &bytes(2));
    let lease = store.acquire(id).unwrap();
    let first = lease.snapshot().unwrap();
    let first_charge = first._reservation.bytes;
    stage(&mut store, id, 2, 1, &bytes(3));
    let work = store.publish(id, 2).unwrap();
    // A real worker can decode without a GPUI/OCaml runtime or UI thread access.
    let completion = std::thread::spawn(move || work.run()).join().unwrap();
    assert_eq!(lease.snapshot().unwrap().revision, 1);
    assert_eq!((store.staged_count(), store.worker_count()), (1, 1));
    store.complete(completion).unwrap();
    assert_eq!(lease.snapshot().unwrap().revision, 2);
    assert_eq!((store.staged_count(), store.worker_count()), (0, 0));
    let second = lease.snapshot().unwrap();
    assert_eq!(
        store.reserved_bytes(),
        FIXED_CHARGE + first_charge + second._reservation.bytes
    );
    store.release(id).unwrap();
    assert!(lease.snapshot().is_none());
    drop(lease);
    drop(second);
    assert_eq!(store.reserved_bytes(), first_charge);
    drop(first);
    assert_eq!(store.reserved_bytes(), 0);
}

#[test]
fn dense_backgrounds_publish_atomically_and_retired_snapshots_stay_charged() {
    let count = chart_data::MAX_POINTS;
    let mut data = Data {
        version: 3,
        contents: Contents::Cartesian(vec![Layer::Bar(Series {
            id: 1,
            name: "Dense".into(),
            points: (0..count)
                .map(|i| Point {
                    id: (count - i) as i64,
                    x: i as f64,
                    y: Some(1.),
                    label: String::new(),
                })
                .collect(),
        })]),
        bar_baselines: vec![],
        bar_backgrounds: vec![],
    };
    let empty_charge = data_charge(&data);
    data.bar_backgrounds = (1..=count)
        .map(|id| chart_data::BarBackground {
            series: 1,
            datum: id as i64,
            brush: gpuio_protocol::chart_appearance::Brush::Solid(id as i64),
        })
        .collect();
    data.bar_baselines = (1..=count)
        .map(|id| chart_data::BarBaseline {
            series: 1,
            datum: id as i64,
            baseline: 0.5,
        })
        .collect();
    assert_eq!(
        data_charge(&data) - empty_charge,
        data.bar_backgrounds.capacity() * size_of::<chart_data::BarBackground>()
            + data.bar_baselines.capacity() * size_of::<chart_data::BarBaseline>()
    );
    let mut bytes = vec![];
    data.binprot_write(&mut bytes).unwrap();
    assert!(bytes.len() < chart_data::MAX_BYTES);
    let mut store = Store::default();
    let id = store.create().unwrap();
    publish(&mut store, id, 1, 1, &bytes);
    let lease = store.acquire(id).unwrap();
    let old = lease.snapshot().unwrap();
    assert_eq!(old.data().bar_backgrounds.len(), count);
    assert_eq!(old.data().bar_baselines.len(), count);
    let old_charge = old._reservation.bytes;
    data.bar_backgrounds.clear();
    data.bar_baselines.clear();
    bytes.clear();
    data.binprot_write(&mut bytes).unwrap();
    stage(&mut store, id, 2, 1, &bytes);
    let work = store.publish(id, 2).unwrap();
    assert_eq!(
        lease.snapshot().unwrap().data().bar_backgrounds.len(),
        count
    );
    store
        .complete(std::thread::spawn(move || work.run()).join().unwrap())
        .unwrap();
    assert!(lease.snapshot().unwrap().data().bar_backgrounds.is_empty());
    store.release(id).unwrap();
    drop(lease);
    assert_eq!(store.reserved_bytes(), old_charge);
    drop(old);
    assert_eq!(store.reserved_bytes(), 0);
}

#[test]
fn abort_and_retry_same_revision_cannot_accept_old_completion() {
    let mut store = Store::default();
    let id = store.create().unwrap();
    publish(&mut store, id, 1, 1, &bytes(1));
    stage(&mut store, id, 2, 1, &bytes(2));
    let old = store.publish(id, 2).unwrap().run();
    store.abort(id, 2).unwrap();
    stage(&mut store, id, 2, 2, &bytes(3));
    let new = store.publish(id, 2).unwrap();
    assert_eq!(store.complete(old), Err(Error::Cancelled));
    store.complete(new.run()).unwrap();
    let snapshot = store.acquire(id).unwrap().snapshot().unwrap();
    assert_eq!((snapshot.revision, snapshot.generation), (2, 2));
    assert_eq!(snapshot.data.validate().unwrap().values, 3);
    drop(snapshot);
    store.close();
    assert_eq!(store.reserved_bytes(), 0);
}

#[test]
fn stale_foreign_and_closed_completions_never_revive_resources() {
    let mut store = Store::default();
    let old_id = store.create().unwrap();
    stage(&mut store, old_id, 1, 1, &bytes(1));
    let old = store.publish(old_id, 1).unwrap();
    store.release(old_id).unwrap();
    let id = store.create().unwrap();
    assert_eq!(old_id.slot(), id.slot());
    assert_ne!(old_id, id);
    assert_eq!(store.complete(old.run()), Err(Error::StaleHandle));
    stage(&mut store, id, 1, 1, &bytes(1));
    let live = store.publish(id, 1).unwrap();

    // Even identical slot/generation/revision in a different store is fenced.
    let mut foreign = Store::default();
    let f0 = foreign.create().unwrap();
    foreign.release(f0).unwrap();
    let fid = foreign.create().unwrap();
    assert_eq!(id, fid);
    stage(&mut foreign, fid, 1, 1, &bytes(1));
    let job = foreign.publish(fid, 1).unwrap();
    assert_eq!(store.complete(job.run()), Err(Error::Cancelled));
    foreign.abort(fid, 1).unwrap();
    foreign.close();
    assert_eq!(foreign.reserved_bytes(), 0);
    store.close();
    assert_eq!(store.complete(live.run()), Err(Error::Closed));
    assert_eq!(store.reserved_bytes(), 0);
    assert_eq!(store.create(), Err(Error::Closed));
}

#[test]
fn cancelled_jobs_keep_worker_and_staging_permits_until_reaped() {
    let mut store = Store::default();
    let id = store.create().unwrap();
    let mut jobs = vec![];
    for _ in 0..MAX_WORKERS {
        stage(&mut store, id, 1, 1, &bytes(1));
        jobs.push(store.publish(id, 1).unwrap());
        store.abort(id, 1).unwrap();
    }
    stage(&mut store, id, 1, 1, &bytes(1));
    assert!(matches!(store.publish(id, 1), Err(Error::Busy)));
    assert_eq!((store.staged_count(), store.worker_count()), (3, 2));
    for job in jobs {
        let completion = job.run();
        assert!(matches!(completion.result, Err(Error::Cancelled)));
        assert_eq!(store.complete(completion), Err(Error::Cancelled));
    }
    let work = store.publish(id, 1).unwrap();
    store.complete(work.run()).unwrap();
    assert_eq!((store.staged_count(), store.worker_count()), (0, 0));
    store.close();
    assert_eq!(store.reserved_bytes(), 0);
}

#[test]
fn malformed_data_and_failed_admission_preserve_last_publication() {
    let mut store = Store::default();
    let id = store.create().unwrap();
    publish(&mut store, id, 1, 1, &bytes(1));
    let lease = store.acquire(id).unwrap();
    stage(&mut store, id, 2, 1, &[255]);
    let work = store.publish(id, 2).unwrap();
    assert_eq!(store.complete(work.run()), Err(Error::InvalidData));
    assert_eq!(store.abort(id, 2), Ok(()));
    assert_eq!(store.abort(id, 1), Err(Error::InvalidRevision));
    assert_eq!(lease.snapshot().unwrap().revision, 1);
    assert_eq!(store.staged_count(), 0);
    stage(&mut store, id, 2, 1, &bytes(2));
    let used = store.reserved_bytes();
    let external_reader = Reservation::new(&store.reserved, MAX_RESERVED_BYTES - used).unwrap();
    assert!(matches!(store.publish(id, 2), Err(Error::ResourceLimit)));
    assert_eq!(store.worker_count(), 0);
    assert_eq!(store.staged_count(), 1);
    assert_eq!(lease.snapshot().unwrap().revision, 1);
    drop(external_reader);
    let work = store.publish(id, 2).unwrap();
    store.complete(work.run()).unwrap();
    assert_eq!(lease.snapshot().unwrap().revision, 2);
    store.close();
    assert!(lease.snapshot().is_none());
    drop(lease);
    assert_eq!(store.reserved_bytes(), 0);
}

#[test]
fn validates_revisions_ranges_upload_limits_and_exhausted_generations() {
    let mut store = Store::default();
    let id = store.create().unwrap();
    let valid = Update {
        id,
        base: 0,
        revision: 1,
        generation: 1,
        bytes: 3,
    };
    for invalid in [
        Update {
            base: 1,
            ..valid.clone()
        },
        Update {
            revision: 0,
            ..valid.clone()
        },
        Update {
            generation: 0,
            ..valid.clone()
        },
        Update {
            generation: 2,
            ..valid.clone()
        },
    ] {
        assert_eq!(store.begin(invalid), Err(Error::InvalidRevision));
    }
    for n in [-1, 0, chart_data::MAX_BYTES as i64 + 1] {
        assert_eq!(
            store.begin(Update {
                bytes: n,
                ..valid.clone()
            }),
            Err(Error::InvalidRange)
        );
    }
    store.begin(valid.clone()).unwrap();
    assert_eq!(store.begin(valid), Err(Error::Busy));
    assert!(matches!(store.publish(id, 1), Err(Error::Incomplete)));
    assert_eq!(store.chunk(id, 2, 0, &[1]), Err(Error::InvalidRevision));
    assert_eq!(store.chunk(id, 1, 0, &[]), Err(Error::InvalidRange));
    assert_eq!(store.chunk(id, 1, 1, &[1]), Err(Error::InvalidRange));
    assert_eq!(
        store.chunk(id, 1, 0, &[1, 2, 3, 4]),
        Err(Error::InvalidRange)
    );
    store.chunk(id, 1, 0, &[1]).unwrap();
    assert_eq!(store.chunk(id, 1, 0, &[1]), Err(Error::InvalidRange));
    assert_eq!(store.abort(id, 2), Err(Error::InvalidRevision));
    for _ in 1..MAX_STAGING {
        let id = store.create().unwrap();
        stage(&mut store, id, 1, 1, &bytes(1));
    }
    let extra = store.create().unwrap();
    assert_eq!(
        store.begin(Update {
            id: extra,
            base: 0,
            revision: 1,
            generation: 1,
            bytes: 3
        }),
        Err(Error::Busy)
    );
    store.abort(id, 1).unwrap();
    store.release(id).unwrap();
    store.slots[id.slot()].generation = u32::MAX;
    assert_ne!(store.create().unwrap().slot(), id.slot());
    while store.slots.len() < MAX_CHARTS {
        store.create().unwrap();
    }
    assert_eq!(store.create(), Err(Error::ResourceLimit));
    store.close();
    assert_eq!(store.reserved_bytes(), 0);
}

#[test]
fn full_dataset_decodes_off_thread_and_drop_cancels_remaining_work() {
    let mut store = Store::default();
    let id = store.create().unwrap();
    let bytes = bytes(chart_data::MAX_POINTS);
    stage(&mut store, id, 1, 1, &bytes);
    let work = store.publish(id, 1).unwrap();
    let completion = std::thread::spawn(move || work.run()).join().unwrap();
    store.complete(completion).unwrap();
    let lease = store.acquire(id).unwrap();
    let snapshot = lease.snapshot().unwrap();
    assert_eq!(snapshot.data.validate().unwrap().values, 100_000);
    assert!(snapshot._reservation.bytes < DECODE_WORKSPACE_BYTES);
    assert_eq!(snapshot.encoded_bytes, bytes.len());
    stage(&mut store, id, 2, 1, &bytes);
    let pending = store.publish(id, 2).unwrap();
    let reserved = store.reserved.clone();
    drop(store);
    assert!(lease.snapshot().is_none());
    drop(lease);
    let completion = pending.run();
    assert!(matches!(completion.result, Err(Error::Cancelled)));
    drop(completion);
    assert_eq!(
        reserved.load(Ordering::Relaxed),
        snapshot._reservation.bytes
    );
    drop(snapshot);
    assert_eq!(reserved.load(Ordering::Relaxed), 0);
}

#[test]
fn retired_empty_reader_handles_remain_charged() {
    let mut store = Store::default();
    let mut readers = vec![];
    for _ in 0..1024 {
        let id = store.create().unwrap();
        publish(&mut store, id, 1, 1, &bytes(0));
        readers.push(store.acquire(id).unwrap());
        store.release(id).unwrap();
    }
    assert_eq!(store.slots.len(), 1);
    assert_eq!(store.reserved_bytes(), readers.len() * FIXED_CHARGE);
    assert!(readers.iter().all(|r| r.snapshot().is_none()));
    store.close();
    assert_eq!(store.reserved_bytes(), readers.len() * FIXED_CHARGE);
    drop(readers);
    assert_eq!(store.reserved_bytes(), 0);
}
