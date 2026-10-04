use gpuio_native::toast_reflow::*;
use gpuio_protocol::{NodeId, tab_motion::Config};
use std::time::Duration;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn target(i: i64, y: f64) -> Target {
    Target {
        id: NodeId::from_parts(i, 1).unwrap(),
        rect: Rect {
            x: 20.,
            y,
            width: 200.,
            height: 60.,
        },
    }
}
#[test]
fn interrupted_reflow_keeps_painted_position_and_velocity_and_ignores_speculative_layout() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    state.configure(&[target(0, 0.)], spring, ms(0)).unwrap();
    let initial = state.sample(ms(0));
    assert!(!initial.needs_frame());
    state.painted(&initial);
    state.configure(&[target(0, 200.)], spring, ms(0)).unwrap();
    let painted = state.sample(ms(80));
    assert!(painted.needs_frame());
    state.painted(&painted);
    let old_velocity_probe = state.sample(ms(80) + Duration::from_micros(1));
    let speculative = state.sample(ms(180));
    state.configure(&[target(0, 50.)], spring, ms(80)).unwrap();
    assert!(!state.painted(&speculative));
    let restart = state.sample(ms(80));
    assert_eq!(restart.items(), painted.items());
    let continued = state.sample(ms(80) + Duration::from_micros(1));
    assert!((continued.items()[0].rect.y - old_velocity_probe.items()[0].rect.y).abs() < 0.00001);
    let done = state.sample(ms(3000));
    assert!(!done.needs_frame());
    assert_eq!(done.items(), vec![target(0, 50.)]);
    assert!(state.painted(&done));
    assert!(!state.painted(&done));
    assert!(!state.sample(ms(4000)).needs_frame());
}
#[test]
fn reorder_removal_generation_and_suspend_retire_old_travel_without_replay() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    state
        .configure(&[target(0, 0.), target(1, 100.)], spring, ms(0))
        .unwrap();
    state.painted(&state.sample(ms(0)));
    state
        .configure(&[target(1, 0.), target(0, 100.)], spring, ms(1))
        .unwrap();
    let frame = state.sample(ms(40));
    assert_eq!(frame.items()[0].id, target(1, 0.).id);
    state.painted(&frame);
    let stale = state.sample(ms(50));
    state.suspend();
    assert!(!state.painted(&stale));
    assert!(!state.sample(ms(60)).needs_frame());
    state.configure(&[target(1, 90.)], spring, ms(70)).unwrap();
    assert_eq!(state.retained_items(), 1);
    assert_eq!(state.sample(ms(70)).items(), vec![target(1, 90.)]);
    state.painted(&state.sample(ms(70)));
    let mut replacement = target(1, 50.);
    replacement.id = NodeId::from_parts(1, 2).unwrap();
    state.configure(&[replacement], spring, ms(80)).unwrap();
    assert_eq!(state.retained_items(), 1);
    assert!(!state.sample(ms(80)).needs_frame());
    assert_eq!(state.sample(ms(80)).items(), vec![replacement]);
    let old = state.sample(ms(90));
    state.configure(&[], spring, ms(100)).unwrap();
    assert_eq!(state.retained_items(), 0);
    assert!(!state.painted(&old));
    assert!(state.sample(ms(100)).items().is_empty());
}
#[test]
fn invalid_updates_are_atomic_and_large_valid_coordinates_snap_exactly() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    state.configure(&[target(0, 0.)], spring, ms(0)).unwrap();
    state.painted(&state.sample(ms(0)));
    for targets in [
        vec![target(0, 0.), target(0, 2.)],
        vec![target(0, f64::NAN)],
        vec![target(0, 33_000_001.)],
        (0..33).map(|i| target(i, 0.)).collect(),
    ] {
        assert!(state.configure(&targets, spring, ms(1)).is_err());
        assert_eq!(state.sample(ms(1)).items(), vec![target(0, 0.)]);
    }
    let mut invalid = Config::default().spring;
    invalid.mass = 0.;
    assert!(
        state
            .configure(&[target(0, 1.)], Some(invalid), ms(1))
            .is_err()
    );
    state
        .configure(&[target(0, 2_000_000.)], spring, ms(2))
        .unwrap();
    let outside = state.sample(ms(2));
    assert!(!outside.needs_frame());
    assert_eq!(outside.items(), vec![target(0, 2_000_000.)]);
    state.painted(&outside);
    state.configure(&[target(0, 30.)], spring, ms(3)).unwrap();
    assert!(!state.sample(ms(3)).needs_frame());
    assert_eq!(state.sample(ms(3)).items(), vec![target(0, 30.)]);
}

#[test]
fn streamed_frame_targets_keep_moving_and_bottom_edges_do_not_drift() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    let mut value = target(0, 440.);
    state.configure_frame(&[value], spring, ms(0)).unwrap();
    state.painted(&state.sample(ms(0)));
    let mut prior_height = value.rect.height;
    for tick in 1..=120 {
        // Every single frame adds content; restarting at "now" would freeze it.
        value.rect.height += 2.;
        value.rect.y = 500. - value.rect.height;
        state
            .configure_frame(&[value], spring, ms(tick * 16))
            .unwrap();
        let frame = state.sample(ms(tick * 16));
        let r = frame.items()[0].rect;
        assert!(r.height > prior_height, "stream stalled at frame {tick}");
        assert!(
            (r.y + r.height - 500.).abs() < 1e-8,
            "bottom edge drift at {tick}: {r:?}"
        );
        prior_height = r.height;
        assert!(state.painted(&frame));
    }
    let done = state.sample(ms(4000));
    assert_eq!(done.items(), &[value]);
    assert!(!done.needs_frame());
}

#[test]
fn width_measurement_is_not_restarted_by_frame_height_changes_or_committed_twice() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    let mut value = target(0, 0.);
    state.configure_frame(&[value], spring, ms(0)).unwrap();
    state.painted(&state.sample(ms(0)));
    let mut widths = vec![];
    for tick in 1..=150 {
        let now = ms(tick * 16);
        let preview = state
            .measure_widths(&[(value.id, 100.)], spring, now)
            .unwrap();
        let measured_width = preview.items()[0].rect.width;
        // Layout streams/changing wrapping independently of the width spring.
        value.rect.width = 100.;
        value.rect.height += 1.;
        value.rect.y -= 1.;
        state.configure_frame(&[value], spring, now).unwrap();
        let painted = state.sample(now);
        assert_eq!(painted.items()[0].rect.width, measured_width);
        assert!(
            !state.painted(&preview),
            "only final layout owns the paint epoch"
        );
        assert!(state.painted(&painted));
        widths.push(measured_width);
    }
    assert!(widths.windows(2).all(|w| w[0] >= w[1]));
    assert_eq!(
        widths.last(),
        Some(&100.),
        "unchanged width reaches its own finite endpoint despite changing heights"
    );
}

#[test]
fn hidden_layers_release_motion_history_and_do_not_replay_when_revealed() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    let first = target(0, 0.);
    let second = target(1, 100.);
    state.configure(&[first, second], spring, ms(0)).unwrap();
    state.painted(&state.sample(ms(0)));
    state
        .configure_frame(&[target(0, 50.), target(1, 150.)], spring, ms(16))
        .unwrap();
    let frame = state.sample(ms(16));
    assert!(frame.needs_frame_for(&[first.id]));
    assert!(!state.painted_subset(&frame, &[target(2, 0.).id]));
    assert!(state.painted_subset(&frame, &[first.id]));
    state
        .configure_frame(&[target(0, 50.), target(1, 300.)], spring, ms(32))
        .unwrap();
    let revealed = state.sample(ms(32));
    assert_eq!(revealed.items()[1].rect.y, 300.);
    assert!(!revealed.needs_frame_for(&[second.id]));
    assert!(revealed.needs_frame_for(&[first.id]));
}

#[test]
fn settling_offscreen_positions_preserves_width_and_rejects_the_previous_preview() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    let first = target(0, 100.);
    state.configure(&[first], spring, ms(0)).unwrap();
    state.painted(&state.sample(ms(0)));
    let mut next = target(0, 50.);
    next.rect.width = 100.;
    next.rect.x = 400.;
    state.configure_frame(&[next], spring, ms(16)).unwrap();
    let preview = state.sample(ms(16));
    state.settle_positions(&[first.id]).unwrap();
    assert!(!state.painted(&preview));
    let snapped = state.sample(ms(16));
    assert_eq!(snapped.items()[0].rect.x, 400.);
    assert_eq!(snapped.items()[0].rect.y, 50.);
    assert_eq!(snapped.items()[0].rect.width, preview.items()[0].rect.width);
    assert!(snapped.needs_frame());
    assert!(state.painted(&snapped));
}

#[test]
fn a_positive_width_target_remains_measurable_when_an_underdamped_spring_hits_zero() {
    let mut state = State::default();
    let mut spring = Config::default().spring;
    spring.damping = 0.;
    let mut first = target(0, 0.);
    first.rect.width = 100.;
    state.configure(&[first], Some(spring), ms(0)).unwrap();
    state.painted(&state.sample(ms(0)));
    let width = state
        .measure_widths(&[(first.id, 1.)], Some(spring), ms(100))
        .unwrap();
    assert_eq!(width.items()[0].rect.width, 1.);
    assert!(!width.needs_frame());
}

#[test]
fn accepted_removal_releases_geometry_before_another_frame_and_keeps_survivor_velocity() {
    let mut state = State::default();
    let spring = Some(Config::default().spring);
    state
        .configure(&[target(0, 0.), target(1, 100.)], spring, ms(0))
        .unwrap();
    state.painted(&state.sample(ms(0)));
    state
        .configure_frame(&[target(0, 50.), target(1, 150.)], spring, ms(16))
        .unwrap();
    let old = state.sample(ms(16));
    state.retain(&[target(1, 0.).id]).unwrap();
    assert_eq!(state.retained_items(), 1);
    assert!(!state.painted(&old));
    let next = state.sample(ms(16));
    assert_eq!(next.items()[0], old.items()[1]);
    assert!(next.needs_frame());
    state.retain(&[]).unwrap();
    assert_eq!(state.retained_items(), 0);
    assert!(state.sample(ms(16)).items().is_empty());
}
