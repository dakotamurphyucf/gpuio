use gpuio_native::motion::{Error, State, Wake};
use gpuio_protocol::animation::*;
use std::{sync::Arc, time::Duration};
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn config(generation: i64, target: f64) -> Config {
    Config {
        generation,
        targets: vec![Target {
            property: Property::Width,
            value: target,
        }],
        initial: Some(vec![Target {
            property: Property::Width,
            value: 0.,
        }]),
        duration_ms: 1000,
        delay_ms: 0,
        easing: Easing::Linear,
        repeat: Repeat::Once,
    }
}
fn value(state: &mut State, time: u64) -> (f64, Wake, Option<Endpoint>) {
    let sample = state.sample(ms(time));
    (
        sample.values.get(Property::Width).unwrap(),
        sample.wake,
        state.painted(sample),
    )
}
#[test]
fn paint_owns_endpoints_and_delay_uses_one_deadline() {
    let mut cfg = config(1, 100.);
    cfg.delay_ms = 200;
    let mut state = State::new(Arc::new(cfg), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 0), (0., Wake::At(ms(200)), None));
    assert_eq!(value(&mut state, 199), (0., Wake::At(ms(200)), None));
    assert_eq!(value(&mut state, 200), (0., Wake::Frame, None));
    assert_eq!(value(&mut state, 700), (50., Wake::Frame, None));
    let unpainted = state.sample(ms(1200));
    assert_eq!(unpainted.wake, Wake::Idle);
    let changed = state
        .retarget(Arc::new(config(2, 0.)), ms(1200))
        .unwrap()
        .unwrap();
    assert_eq!(
        changed,
        Endpoint {
            generation: 1,
            outcome: Outcome::Cancelled(CancelReason::Replaced)
        }
    );
    assert_eq!(
        state.painted(unpainted),
        None,
        "old sampled frame cannot finish replacement"
    );
    assert_eq!(
        value(&mut state, 1200).0,
        50.,
        "retarget begins at last painted value"
    );
    assert_eq!(value(&mut state, 1700).0, 25.);
    assert_eq!(
        value(&mut state, 2200),
        (
            0.,
            Wake::Idle,
            Some(Endpoint {
                generation: 2,
                outcome: Outcome::Finished
            })
        )
    );
    assert_eq!(value(&mut state, 5000), (0., Wake::Idle, None));
    assert_eq!(state.cancel(CancelReason::Removed), None);
}
#[test]
fn immediate_placement_zero_duration_and_invalid_retargets() {
    let mut cfg = config(1, 80.);
    cfg.initial = None;
    cfg.delay_ms = 500;
    let mut state = State::new(Arc::new(cfg.clone()), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 0).0, 80.);
    assert_eq!(
        state.retarget(Arc::new(cfg.clone()), ms(100)).unwrap(),
        None
    );
    cfg.targets[0].value = 90.;
    assert_eq!(
        state.retarget(Arc::new(cfg.clone()), ms(100)),
        Err(Error::StaleGeneration)
    );
    cfg.generation = 2;
    cfg.duration_ms = 0;
    state.retarget(Arc::new(cfg), ms(100)).unwrap();
    assert_eq!(value(&mut state, 599), (80., Wake::At(ms(600)), None));
    assert_eq!(value(&mut state, 600).0, 90.);
    let mut bad = config(3, f64::NAN);
    assert_eq!(
        state.retarget(Arc::new(bad.clone()), ms(700)),
        Err(Error::InvalidConfig)
    );
    bad.targets[0].value = -1.;
    assert!(!bad.is_valid());
    assert_eq!(value(&mut state, 800).0, 90.);
}
#[test]
fn hidden_runs_pause_and_reduced_motion_settles_once() {
    let mut state = State::new(Arc::new(config(1, 100.)), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 250).0, 25.);
    state.set_visible(false, ms(250));
    assert_eq!(value(&mut state, 10000), (25., Wake::Idle, None));
    state.set_visible(true, ms(10000));
    assert_eq!(value(&mut state, 10000).0, 25.);
    assert_eq!(value(&mut state, 10250).0, 50.);
    state.set_reduced_motion(true, ms(10250));
    assert_eq!(
        value(&mut state, 10250),
        (
            100.,
            Wake::Idle,
            Some(Endpoint {
                generation: 1,
                outcome: Outcome::Finished
            })
        )
    );
    state.set_reduced_motion(false, ms(11000));
    assert_eq!(value(&mut state, 11000), (100., Wake::Idle, None));
}
#[test]
fn repeated_ranges_survive_interruption_and_reduced_motion_pause() {
    let mut cfg = config(1, 100.);
    cfg.repeat = Repeat::Alternate;
    let mut state = State::new(Arc::new(cfg.clone()), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 250).0, 25.);
    assert_eq!(value(&mut state, 1000).0, 100.);
    assert_eq!(value(&mut state, 1250).0, 75.);
    cfg.generation = 2;
    cfg.targets[0].value = 200.;
    state.retarget(Arc::new(cfg), ms(1250)).unwrap();
    assert_eq!(value(&mut state, 1250).0, 75.);
    assert_eq!(value(&mut state, 1750).0, 137.5);
    assert_eq!(value(&mut state, 2250).0, 200.);
    assert_eq!(
        value(&mut state, 2750).0,
        100.,
        "subsequent cycles use declared initial range"
    );
    state.set_reduced_motion(true, ms(2750));
    assert_eq!(value(&mut state, 10000), (0., Wake::Idle, None));
    state.set_reduced_motion(false, ms(10000));
    assert_eq!(value(&mut state, 10000).0, 100.);
    assert_eq!(
        state.cancel(CancelReason::WindowClosed),
        Some(Endpoint {
            generation: 2,
            outcome: Outcome::Cancelled(CancelReason::WindowClosed)
        })
    );
    assert_eq!(state.cancel(CancelReason::Removed), None);
    assert_eq!(value(&mut state, 11000).1, Wake::Idle);
}
#[test]
fn loop_uses_integer_modulo_at_long_uptime_and_clock_never_reverses() {
    let mut cfg = config(1, 100.);
    cfg.repeat = Repeat::Loop;
    let mut state = State::new(Arc::new(cfg), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 1_000_000_000_000_250).0, 25.);
    assert_eq!(value(&mut state, 0).0, 25.);
    assert_eq!(value(&mut state, 1_000_000_000_001_000).0, 0.);
}
#[test]
fn stepped_motion_holds_during_delay_then_jumps_and_completes_after_paint() {
    let mut cfg = config(8, 120.);
    cfg.delay_ms = 200;
    cfg.easing = Easing::Steps(4, StepPosition::JumpStart);
    let mut state = State::new(Arc::new(cfg), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 199), (0., Wake::At(ms(200)), None));
    assert_eq!(value(&mut state, 200), (30., Wake::Frame, None));
    assert_eq!(value(&mut state, 449), (30., Wake::Frame, None));
    assert_eq!(value(&mut state, 450), (60., Wake::Frame, None));
    assert_eq!(value(&mut state, 950), (120., Wake::Frame, None));
    // Reaching the target on an early step is not terminal completion.
    let sample = state.sample(ms(1200));
    assert_eq!(sample.wake, Wake::Idle);
    assert_eq!(
        state.painted(sample),
        Some(Endpoint {
            generation: 8,
            outcome: Outcome::Finished,
        })
    );
    assert_eq!(value(&mut state, 1300), (120., Wake::Idle, None));
}

#[test]
fn piecewise_cubic_motion_has_exact_quarter_points_and_painted_completion() {
    let mut cfg = config(7, 160.);
    cfg.easing = Easing::EaseInOutCubic;
    let mut state = State::new(Arc::new(cfg), ms(0), false).unwrap();
    for (time, expected) in [(0, 0.), (250, 10.), (500, 80.), (750, 150.)] {
        assert_eq!(value(&mut state, time), (expected, Wake::Frame, None));
    }
    let sample = state.sample(ms(1000));
    assert_eq!(sample.values.get(Property::Width), Some(160.));
    assert_eq!(sample.wake, Wake::Idle);
    assert_eq!(
        state.painted(sample),
        Some(Endpoint {
            generation: 7,
            outcome: Outcome::Finished,
        })
    );
    assert_eq!(value(&mut state, 1100), (160., Wake::Idle, None));
}

#[test]
fn bezier_inverts_x_and_clamps_property_overshoot() {
    assert!((Easing::CubicBezier(0., 0., 0., 1.).sample(0.125) - 0.5).abs() < 1e-10);
    for easing in [
        Easing::Linear,
        Easing::Ease,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::EaseInOutCubic,
    ] {
        assert_eq!(easing.sample(0.), 0.);
        assert_eq!(easing.sample(1.), 1.);
        let mut previous = 0.;
        for n in 0..=100 {
            let value = easing.sample(n as f64 / 100.);
            assert!(value >= previous);
            previous = value;
        }
    }
    let mut cfg = config(1, 1.);
    cfg.targets[0].property = Property::Opacity;
    cfg.initial.as_mut().unwrap()[0].property = Property::Opacity;
    cfg.easing = Easing::CubicBezier(0., 10., 1., 10.);
    let mut state = State::new(Arc::new(cfg), ms(0), false).unwrap();
    assert_eq!(
        state.sample(ms(500)).values.get(Property::Opacity),
        Some(1.)
    );
}
#[test]
fn configuration_bounds_and_canonical_property_sets() {
    let base = config(1, 100.);
    assert!(base.is_valid());
    for invalid in [f64::NAN, f64::INFINITY, -1., 1_000_001.] {
        let mut cfg = base.clone();
        cfg.targets[0].value = invalid;
        assert!(!cfg.is_valid());
    }
    let mut cfg = base.clone();
    cfg.targets.push(cfg.targets[0]);
    assert!(!cfg.is_valid());
    let mut cfg = base.clone();
    cfg.initial.as_mut().unwrap()[0].property = Property::Height;
    assert!(!cfg.is_valid());
    let mut cfg = base.clone();
    cfg.repeat = Repeat::Loop;
    cfg.duration_ms = 0;
    assert!(!cfg.is_valid());
    let mut cfg = base.clone();
    cfg.delay_ms = MAX_TIME_MS + 1;
    assert!(!cfg.is_valid());
    let mut cfg = base;
    cfg.easing = Easing::CubicBezier(-0.1, 0., 1., 1.);
    assert!(!cfg.is_valid());
}

#[test]
fn policy_changes_invalidate_prepared_but_unpainted_frames() {
    let mut state = State::new(Arc::new(config(1, 100.)), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 100).0, 10.);
    let stale = state.sample(ms(1000));
    state.set_visible(false, ms(1000));
    state.set_visible(true, ms(2000));
    assert_eq!(state.painted(stale), None);
    let endpoint = state.retarget(Arc::new(config(2, 50.)), ms(2000)).unwrap();
    assert_eq!(
        endpoint.unwrap().outcome,
        Outcome::Cancelled(CancelReason::Replaced)
    );
    assert_eq!(value(&mut state, 2000).0, 10.);
    let stale = state.sample(ms(2200));
    state.set_reduced_motion(true, ms(2200));
    assert_eq!(state.painted(stale), None);
    assert_eq!(value(&mut state, 2200).0, 50.);
}

#[test]
fn extreme_finite_bezier_controls_still_produce_bounded_geometry() {
    let mut cfg = config(1, 1_000_000.);
    cfg.easing = Easing::CubicBezier(0., f64::MAX, 1., f64::MAX);
    assert!(cfg.is_valid());
    let mut state = State::new(Arc::new(cfg), ms(0), false).unwrap();
    let (value, _, _) = value(&mut state, 500);
    assert_eq!(value, 1_000_000.);
    assert!(
        Easing::CubicBezier(0., f64::MAX, 1., -f64::MAX)
            .sample(0.5)
            .is_finite()
    );
}

#[test]
fn linear_stop_motion_holds_jumps_clamps_and_confirms_completion() {
    let mut cfg = config(9, 100.);
    cfg.delay_ms = 200;
    cfg.easing = Easing::LinearStops(
        LinearStops::new(vec![(0., -0.25), (0.5, -0.25), (0.5, 1.25), (1., 1.25)]).unwrap(),
    );
    let mut state = State::new(Arc::new(cfg), ms(0), false).unwrap();
    assert_eq!(value(&mut state, 199), (0., Wake::At(ms(200)), None));
    assert_eq!(value(&mut state, 200), (0., Wake::Frame, None));
    assert_eq!(value(&mut state, 699).0, 0.);
    assert_eq!(value(&mut state, 700).0, 125.);
    assert_eq!(value(&mut state, 1199).0, 125.);
    let sample = state.sample(ms(1200));
    assert_eq!(sample.values.get(Property::Width), Some(100.));
    assert_eq!(
        state.painted(sample),
        Some(Endpoint {
            generation: 9,
            outcome: Outcome::Finished
        })
    );
    assert_eq!(value(&mut state, 1300), (100., Wake::Idle, None));
}
