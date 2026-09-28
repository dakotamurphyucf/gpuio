use gpuio_native::{
    highlight_jobs::*,
    highlight_projection::{self as projection, Group, Kind, Projection, Run, RunKey, Source},
};
use gpuio_protocol::{
    NodeId,
    highlight::{Appearance, Config, Query, Spec},
};
use std::sync::Arc;

fn source(text: &str) -> Arc<Projection> {
    Arc::new(
        Projection::new(vec![Group {
            kind: Kind::Ordinary,
            runs: vec![Run {
                key: RunKey {
                    node: NodeId::from_parts(0, 1).unwrap(),
                    fragment: 0,
                },
                source: Source::Text(text.into()),
            }],
        }])
        .unwrap(),
    )
}
fn config(query: &str) -> Arc<Config> {
    Arc::new(Config(vec![Spec {
        query: Some(Query {
            text: query.into(),
            case_sensitive: false,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: Appearance {
            color: 1,
            active_color: 2,
            radius: 2.,
        },
        active_index: None,
        match_index_offset: 0,
    }]))
}
fn ready(handle: &Handle) -> Arc<Ready> {
    match handle.status() {
        Status::Ready(ready) => ready,
        _ => panic!("expected ready"),
    }
}
fn finish(pool: &mut Pool) {
    let work = pool.next_work().expect("queued work");
    assert!(pool.complete(work.run()));
}

#[test]
fn real_worker_completion_and_cosmetic_changes_share_ready_results() {
    let mut pool = Pool::default();
    let p = source("a a");
    let c = config("a");
    let handle = pool.request(p.clone(), c.clone()).unwrap();
    let work = pool.next_work().unwrap();
    let completion = std::thread::spawn(move || work.run()).join().unwrap();
    assert!(pool.complete(completion));
    let initial = ready(&handle);
    assert_eq!(
        initial.matches.counts[0],
        projection::Count {
            total: 2,
            stored: 2
        }
    );
    assert!(Arc::ptr_eq(initial.source(), &p));
    assert_eq!(handle.update(p.clone(), c.clone()), Ok(Update::Unchanged));
    let mut cosmetic = (*c).clone();
    cosmetic.0[0].active_index = Some(21);
    cosmetic.0[0].match_index_offset = 20;
    cosmetic.0[0].appearance.radius = 8.;
    assert_eq!(
        handle.update(p, Arc::new(cosmetic)),
        Ok(Update::Presentation)
    );
    assert_eq!(handle.epoch(), 1);
    assert!(Arc::ptr_eq(&initial, &ready(&handle)));
    assert!(handle.config().unwrap().0[0].is_active(1));
    assert!(pool.next_work().is_none());
}

#[test]
fn late_success_cannot_publish_after_matcher_or_source_changes() {
    let mut pool = Pool::default();
    let p = source("a b");
    let handle = pool.request(p.clone(), config("a")).unwrap();
    let old = pool.next_work().unwrap().run(); // already finished, not delivered
    assert_eq!(handle.update(p.clone(), config("b")), Ok(Update::Rematch));
    assert!(matches!(handle.status(), Status::Pending));
    assert_eq!(handle.epoch(), 2);
    assert!(pool.next_work().is_none()); // one job per scope until old delivery
    assert!(!pool.complete(old));
    finish(&mut pool);
    let old_ready = ready(&handle);
    assert_eq!(
        old_ready.matches.spans.values().next().unwrap()[0].bytes,
        2..3
    );
    assert_eq!(
        handle.update(source("bbb"), config("b")),
        Ok(Update::Rematch)
    );
    assert!(matches!(handle.status(), Status::Pending));
    finish(&mut pool);
    assert_eq!(ready(&handle).matches.counts[0].total, 3);
    assert_eq!(old_ready.matches.counts[0].total, 1);
}

#[test]
fn dropping_scope_cancels_undispatched_work_and_releases_every_charge() {
    let mut pool = Pool::default();
    let handle = pool.request(source("a"), config("a")).unwrap();
    let work = pool.next_work().unwrap();
    assert!(pool.reserved_bytes() > 0);
    drop(handle);
    assert!(pool.reserved_bytes() > 0); // worker owns the retired request
    let completion = std::thread::spawn(move || work.run()).join().unwrap();
    assert!(!pool.complete(completion));
    assert_eq!(pool.running_count(), 0);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn retired_paint_holds_its_charge_through_updates_and_close() {
    let mut pool = Pool::default();
    let handle = pool.request(source("a"), config("a")).unwrap();
    finish(&mut pool);
    let held = ready(&handle);
    let first = pool.reserved_bytes();
    handle.update(source("b"), config("b")).unwrap();
    assert!(pool.reserved_bytes() > first);
    pool.close();
    assert!(matches!(handle.status(), Status::Failed(Error::Closed)));
    assert_eq!(pool.reserved_bytes(), first);
    drop(held);
    assert_eq!(pool.reserved_bytes(), 0);
    assert_eq!(handle.update(source("a"), config("a")), Err(Error::Closed));
    assert!(matches!(
        pool.request(source("a"), config("a")),
        Err(Error::Closed)
    ));
}

#[test]
fn memory_admission_failure_retires_previous_ready_and_can_be_retried() {
    let mut pool = Pool::default();
    let big = source(&"a".repeat(projection::MAX_SOURCE_BYTES));
    let blocker = pool.request(big.clone(), config("a")).unwrap();
    let small = pool.request(source("a"), config("a")).unwrap();
    // Dispatch the blocker without running its large scan; complete small work.
    let blocked_work = pool.next_work().unwrap();
    finish(&mut pool);
    assert_eq!(ready(&small).matches.counts[0].total, 1);
    assert_eq!(
        small.update(big.clone(), config("a")),
        Err(Error::AdmissionLimit)
    );
    assert!(matches!(
        small.status(),
        Status::Failed(Error::AdmissionLimit)
    ));
    assert!(small.config().is_none());
    assert!(pool.reserved_bytes() <= MAX_RESERVED_BYTES);
    drop(blocker);
    drop(blocked_work);
    assert!(pool.next_work().is_none()); // reap the abandoned retired job
    assert_eq!(pool.reserved_bytes(), 0);
    assert_eq!(small.update(big, config("a")), Ok(Update::Rematch));
    drop(small);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn worker_limit_fair_dispatch_and_abandoned_ticket_recovery() {
    let mut pool = Pool::default();
    let p = source("a");
    let a = pool.request(p.clone(), config("a")).unwrap();
    let b = pool.request(p.clone(), config("a")).unwrap();
    let c = pool.request(p.clone(), config("a")).unwrap();
    let wa = pool.next_work().unwrap();
    let wb = pool.next_work().unwrap();
    assert_eq!(pool.running_count(), MAX_WORKERS);
    assert!(pool.next_work().is_none());
    a.update(p, config("b")).unwrap();
    assert!(!pool.complete(wa.run()));
    let wc = pool.next_work().unwrap(); // c precedes the updated a
    assert!(pool.complete(wc.run()));
    assert_eq!(ready(&c).matches.counts[0].total, 1);
    assert!(matches!(a.status(), Status::Pending));
    drop(wb);
    let replacement = pool.next_work().unwrap();
    assert!(matches!(b.status(), Status::Failed(Error::WorkerFailed)));
    assert!(pool.complete(replacement.run()));
    assert_eq!(ready(&a).matches.counts[0].total, 0);
}

#[test]
fn foreign_completion_cannot_consume_another_pools_task_slot() {
    let mut a = Pool::default();
    let mut b = Pool::default();
    let ha = a.request(source("a"), config("a")).unwrap();
    let hb = b.request(source("b"), config("b")).unwrap();
    let wa = a.next_work().unwrap();
    let wrong = b.next_work().unwrap().run();
    assert!(!a.complete(wrong));
    assert_eq!(a.running_count(), 1);
    assert!(a.complete(wa.run()));
    assert_eq!(ready(&ha).matches.counts[0].total, 1);
    assert!(b.next_work().is_none()); // its lost completion is reaped explicitly
    assert!(matches!(hb.status(), Status::Failed(Error::WorkerFailed)));
}

#[test]
fn source_range_errors_and_scope_count_limits_are_explicit() {
    let mut pool = Pool::default();
    let p = source("λ");
    let mut cfg = (*config("λ")).clone();
    cfg.0[0].ranges.push(gpuio_protocol::highlight::Range {
        start_byte: 1,
        end_byte: 2,
    });
    let handle = pool.request(p.clone(), Arc::new(cfg)).unwrap();
    finish(&mut pool);
    assert!(matches!(
        handle.status(),
        Status::Failed(Error::Match(projection::Error::InvalidRange {
            spec_index: 0,
            range_index: 0,
            reason: projection::RangeError::ScalarBoundary
        }))
    ));
    let mut handles = vec![handle];
    for _ in 1..MAX_SCOPES {
        handles.push(pool.request(p.clone(), Arc::new(Config(vec![]))).unwrap());
    }
    assert!(matches!(
        pool.request(p.clone(), config("λ")),
        Err(Error::AdmissionLimit)
    ));
    handles.pop();
    handles.push(pool.request(p, config("λ")).unwrap());
    pool.close();
    assert_eq!(pool.reserved_bytes(), 0);
    assert!(pool.next_work().is_none());
}

#[test]
fn completed_sparse_results_release_unused_output_and_scratch_reservations() {
    let mut pool = Pool::default();
    let handle = pool
        .request(source(&"a".repeat(100000)), config("absent"))
        .unwrap();
    let admitted = pool.reserved_bytes();
    let work = pool.next_work().unwrap();
    let completion = std::thread::spawn(move || work.run()).join().unwrap();
    // Credit is released even before the native thread delivers the completion.
    assert!(pool.reserved_bytes() < admitted / 2);
    assert!(pool.complete(completion));
    assert_eq!(ready(&handle).matches.counts[0].total, 0);
    drop(handle);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn close_with_two_completed_but_undelivered_jobs_never_revives_scopes() {
    let mut pool = Pool::default();
    let a = pool.request(source("a"), config("a")).unwrap();
    let b = pool.request(source("b"), config("b")).unwrap();
    let wa = pool.next_work().unwrap();
    let wb = pool.next_work().unwrap();
    let ca = wa.run();
    let cb = wb.run();
    pool.close();
    assert!(pool.reserved_bytes() > 0);
    assert!(!pool.complete(ca));
    assert!(!pool.complete(cb));
    assert_eq!(pool.reserved_bytes(), 0);
    assert_eq!(pool.running_count(), 0);
    assert!(matches!(a.status(), Status::Failed(Error::Closed)));
    assert!(matches!(b.status(), Status::Failed(Error::Closed)));
}

#[test]
fn invalid_update_cancels_queued_work_and_advances_its_epoch() {
    let mut pool = Pool::default();
    let p = source("a");
    let handle = pool.request(p.clone(), config("a")).unwrap();
    let work = pool.next_work().unwrap();
    let mut bad = (*config("a")).clone();
    bad.0[0].match_index_offset = -1;
    assert_eq!(
        handle.update(p, Arc::new(bad)),
        Err(Error::Match(projection::Error::InvalidConfig))
    );
    assert_eq!(handle.epoch(), 2);
    assert!(!pool.complete(work.run()));
    assert!(matches!(
        handle.status(),
        Status::Failed(Error::Match(projection::Error::InvalidConfig))
    ));
    assert_eq!(pool.reserved_bytes(), 0);
}
