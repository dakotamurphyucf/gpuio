use super::*;
use gpuio_protocol::{ResourceId, animation::Easing, image::ImageSource};

fn config() -> Arc<Config> {
    Arc::new(Config {
        label: "Working".into(),
        animated: true,
        period_ms: 1000,
        easing: Easing::Linear,
        source: None,
    })
}
fn setup() -> (Owner, Rc<Clock>) {
    let clock = Rc::new(Clock::default());
    clock.set_test_time(Duration::ZERO);
    (Owner::new(config(), clock.clone()).unwrap(), clock)
}
fn at(clock: &Clock, ms: u64) {
    clock.set_test_time(Duration::from_millis(ms));
}
fn phase(owner: &Owner) -> f32 {
    owner.0.borrow_mut().sample(false)
}
fn paint(owner: &Owner) -> bool {
    owner.0.borrow_mut().painted(true, false)
}

#[test]
fn monotonic_elapsed_pauses_hidden_intervals_and_preserves_large_time_precision() {
    let (owner, clock) = setup();
    assert_eq!(phase(&owner), 0.);
    assert!(paint(&owner));
    at(&clock, 250);
    assert_eq!(phase(&owner), 0.25);
    at(&clock, 200);
    assert_eq!(phase(&owner), 0.25);
    at(&clock, 1250);
    assert_eq!(phase(&owner), 0.25);
    owner.suspend();
    at(&clock, 5000);
    assert_eq!(phase(&owner), 0.25);
    paint(&owner);
    at(&clock, 5250);
    assert_eq!(phase(&owner), 0.5);
    at(&clock, 9_000_000_000_000_500);
    assert_eq!(phase(&owner), 0.75);
}

#[test]
fn labels_preserve_phase_but_all_source_and_timing_changes_restart_atomically() {
    let (owner, clock) = setup();
    paint(&owner);
    at(&clock, 250);
    let stale = owner.driver();
    let old = Arc::downgrade(&owner.0.borrow().config);
    let mut cosmetic = (*config()).clone();
    cosmetic.label = "Another label".into();
    owner.update(Arc::new(cosmetic)).unwrap();
    assert!(
        old.upgrade().is_none(),
        "share current tree config allocation"
    );
    assert!(stale.sample(false).is_none());
    assert_eq!(phase(&owner), 0.25);
    for case in 0..4 {
        let mut next = (*owner.0.borrow().config).clone();
        match case {
            0 => next.period_ms = 2000,
            1 => next.easing = Easing::EaseIn,
            2 => {
                next.source = Some(ImageSource::Reference(
                    ResourceId::from_parts(7, 1).unwrap(),
                ))
            }
            3 => next.animated = false,
            _ => unreachable!(),
        }
        owner.update(Arc::new(next)).unwrap();
        assert_eq!(phase(&owner), 0.);
        paint(&owner);
        at(&clock, 500 + case * 250);
        phase(&owner);
    }
    let current = owner.0.borrow().config.clone();
    let driver = owner.driver();
    let mut invalid = (*current).clone();
    invalid.period_ms = 0;
    assert_eq!(owner.update(Arc::new(invalid)), Err(InvalidConfig));
    assert!(Arc::ptr_eq(&current, &owner.0.borrow().config));
    assert_eq!(driver.sample(false), Some(0.));
    assert!(
        Owner::new(
            Arc::new(Config {
                label: "".into(),
                ..(*current).clone()
            }),
            clock
        )
        .is_err()
    );
}

#[test]
fn static_and_inert_or_reduced_paint_zero_and_do_not_replenish_wakes() {
    let (owner, clock) = setup();
    paint(&owner);
    at(&clock, 250);
    assert_eq!(owner.0.borrow_mut().sample(true), 0.);
    assert!(!owner.0.borrow_mut().painted(true, true));
    assert!(!owner.0.borrow_mut().delivered(true));
    at(&clock, 10000);
    assert_eq!(
        phase(&owner),
        0.25,
        "resume retained phase, not hidden wall time"
    );
    assert!(paint(&owner));
    let mut static_config = (*config()).clone();
    static_config.animated = false;
    owner.update(Arc::new(static_config)).unwrap();
    assert_eq!(phase(&owner), 0.);
    assert!(!paint(&owner));
    assert!(!owner.0.borrow_mut().delivered(false));
    owner.update(config()).unwrap();
    assert_eq!(phase(&owner), 0.);
    assert!(paint(&owner));
}

#[test]
fn visible_layout_counts_but_omitted_paint_and_delivered_wakes_cannot_replay_it() {
    let (owner, clock) = setup();
    paint(&owner);
    at(&clock, 200);
    owner.prepare_frame();
    let driver = owner.driver();
    at(&clock, 250);
    assert_eq!(driver.sample(false), Some(0.25));
    paint(&owner);
    owner.finish_frame();
    at(&clock, 300);
    owner.prepare_frame();
    at(&clock, 700);
    assert!(owner.0.borrow_mut().delivered(false));
    owner.finish_frame();
    assert!(!owner.0.borrow_mut().delivered(false));
    at(&clock, 10000);
    assert_eq!(owner.driver().sample(false), Some(0.3));
    paint(&owner);
    owner.finish_frame();
    at(&clock, 10100);
    assert_eq!(phase(&owner), 0.4);
}

#[test]
fn weak_drivers_and_one_pending_wake_bound_rapid_updates_and_owner_teardown() {
    let (owner, _) = setup();
    let driver = owner.driver();
    assert!(paint(&owner));
    for slot in 0..1000 {
        owner
            .update(Arc::new(Config {
                source: Some(ImageSource::Reference(
                    ResourceId::from_parts(slot, 1).unwrap(),
                )),
                ..(*config()).clone()
            }))
            .unwrap();
        owner.driver().sample(false).unwrap();
        assert!(!paint(&owner), "reuse the already pending wake");
    }
    assert!(driver.sample(false).is_none());
    assert_eq!(Rc::strong_count(&owner.0), 1);
    let config = Arc::downgrade(&owner.0.borrow().config);
    let current = owner.driver();
    drop(owner);
    assert!(config.upgrade().is_none());
    assert!(current.sample(false).is_none());
    current.suspend();
}

#[test]
fn overshooting_curves_reduce_to_finite_turns_before_f32_conversion() {
    let (owner, clock) = setup();
    for (y1, y2) in [
        (-2., 3.),
        (f64::MAX, f64::MAX),
        (-f64::MAX, f64::MAX),
        (-f64::MAX, -f64::MAX),
    ] {
        owner
            .update(Arc::new(Config {
                easing: Easing::CubicBezier(0.25, y1, 0.75, y2),
                ..(*config()).clone()
            }))
            .unwrap();
        at(&clock, 0);
        // Fresh clocks avoid a backwards-time reset obscuring the whole cycle.
        let test_clock = Rc::new(Clock::default());
        test_clock.set_test_time(Duration::ZERO);
        let sample_owner = Owner::new(owner.0.borrow().config.clone(), test_clock.clone()).unwrap();
        paint(&sample_owner);
        for ms in 0..1001 {
            at(&test_clock, ms);
            let turns = phase(&sample_owner);
            assert!(
                turns.is_finite() && (0. ..1.).contains(&turns),
                "{ms}: {turns}"
            );
        }
    }
}

#[test]
fn delivery_during_preparation_preserves_visible_time_and_discards_omitted_time() {
    let (owner, clock) = setup();
    paint(&owner);
    at(&clock, 100);
    owner.prepare_frame();
    at(&clock, 200);
    owner.0.borrow_mut().delivered(false);
    at(&clock, 250);
    assert_eq!(phase(&owner), 0.25, "delivery lost visible prepared time");
    paint(&owner);
    owner.finish_frame();

    at(&clock, 300);
    owner.prepare_frame();
    at(&clock, 400);
    assert!(owner.0.borrow_mut().delivered(false));
    at(&clock, 700);
    owner.finish_frame();
    assert!(!owner.0.borrow_mut().delivered(false));
    at(&clock, 10000);
    assert_eq!(phase(&owner), 0.3, "omitted layout interval replayed");
    paint(&owner);
    at(&clock, 10100);
    owner.prepare_frame();
    assert!(!owner.0.borrow_mut().delivered(true));
    assert_eq!(owner.0.borrow_mut().sample(true), 0.);
    assert!(!owner.0.borrow_mut().painted(true, true));
    assert!(!owner.0.borrow().running);
}
