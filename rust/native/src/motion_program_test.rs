use super::*;
use crate::motion_clock::Clock;
use gpuio_protocol::{
    animation::{Easing, Property, Spring, Target},
    animation_program::{Clock as Selection, Stage, Timing},
};
fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}
fn targets(value: f64) -> Vec<Target> {
    vec![Target {
        property: Property::Left,
        value,
    }]
}
fn config(generation: i64, ends: &[f64], duration: i64) -> Config {
    Config {
        generation,
        program: Program {
            initial: Some(targets(0.)),
            stages: ends
                .iter()
                .map(|value| Stage {
                    targets: targets(*value),
                    timing: Timing::Tween(duration, Easing::Linear),
                    delay_ms: 0,
                })
                .collect(),
            delay_ms: 0,
            repeat: Repeat::Once,
            clock: Selection::Independent,
        },
        playback: Playback::Running,
        restart: 0,
    }
}
fn make_state(config: Config) -> State {
    State::new(Arc::new(config), ms(0), false).unwrap()
}
fn sample(state: &mut State, now: u64) -> Sample {
    state.sample(ms(now), None).unwrap()
}
fn value(sample: &Sample) -> f64 {
    sample.frame.values.get(Property::Left).unwrap()
}
fn paint(state: &mut State, now: u64) -> (f64, Wake, Vec<Signal>) {
    let sample = sample(state, now);
    (value(&sample), sample.wake, state.painted(sample))
}
#[test]
fn stages_and_terminal_are_ordered_bounded_and_paint_confirmed() {
    let mut state = make_state(config(1, &[100., 200., 300.], 100));
    let skipped = sample(&mut state, 250);
    assert_eq!(value(&skipped), 250.);
    assert_eq!(state.completed, 0, "sampling is not delivery");
    let duplicate = skipped.clone();
    let events = state.painted(skipped);
    assert_eq!(
        events.iter().map(|e| e.index).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(events.iter().all(|e| matches!(
        e.observation,
        Observation::StageCompleted(_, StageResult::Played)
    )));
    assert!(state.painted(duplicate).is_empty());
    let (_, wake, events) = paint(&mut state, 300);
    assert_eq!(wake, Wake::Idle);
    assert_eq!(
        events.iter().map(|e| e.index).collect::<Vec<_>>(),
        vec![3, 33]
    );
    assert!(state.is_finished());
    assert_eq!(paint(&mut state, 10_000), (300., Wake::Idle, vec![]));
    assert!(state.cancel(CancelReason::Removed).is_none());
    let mut many = make_state(config(1, &[0.; 32], 0));
    let events = paint(&mut many, 0).2;
    assert_eq!(events.len(), 33);
    assert!(events.iter().all(|e| e.is_valid()));
}
#[test]
fn pause_hidden_and_resume_preserve_run_and_exclude_overlapping_suspension() {
    let base = config(1, &[1000.], 1000);
    let mut state = make_state(base.clone());
    assert_eq!(paint(&mut state, 100).0, 100.);
    let prepared = sample(&mut state, 100);
    let mut paused = base.clone();
    paused.generation = 2;
    paused.playback = Playback::Paused;
    assert!(
        state
            .update(Arc::new(paused.clone()), ms(100))
            .unwrap()
            .is_none()
    );
    assert!(!state.accepts_sample(&prepared));
    assert_eq!(state.run_generation(), 1);
    assert_eq!(paint(&mut state, 150), (100., Wake::Idle, vec![]));
    state.set_visible(false, ms(150));
    paused.generation = 3;
    paused.playback = Playback::Running;
    state.update(Arc::new(paused), ms(200)).unwrap();
    assert_eq!(sample(&mut state, 250).wake, Wake::Idle);
    state.set_visible(true, ms(300));
    assert_eq!(paint(&mut state, 400).0, 200.);
    assert_eq!(state.run_generation(), 1);
}
#[test]
fn reduced_motion_skips_only_unplayed_stages_and_explicit_pause_holds() {
    let base = config(1, &[100., 200., 300.], 100);
    let mut state = make_state(base.clone());
    assert_eq!(paint(&mut state, 100).2.len(), 1);
    let stale = sample(&mut state, 150);
    state.set_reduced(true, ms(150));
    assert!(!state.accepts_sample(&stale));
    let mut paused = base.clone();
    paused.generation = 2;
    paused.playback = Playback::Paused;
    state.update(Arc::new(paused.clone()), ms(150)).unwrap();
    assert_eq!(paint(&mut state, 200), (100., Wake::Idle, vec![]));
    paused.generation = 3;
    paused.playback = Playback::Running;
    state.update(Arc::new(paused), ms(200)).unwrap();
    let (position, wake, events) = paint(&mut state, 200);
    assert_eq!(position, 300.);
    assert_eq!(wake, Wake::Idle);
    assert_eq!(
        events.iter().map(|e| e.observation).collect::<Vec<_>>(),
        vec![
            Observation::StageCompleted(1, StageResult::ReducedMotion),
            Observation::StageCompleted(2, StageResult::ReducedMotion),
            Observation::Finished
        ]
    );
    state.set_reduced(false, ms(300));
    assert_eq!(paint(&mut state, 400), (300., Wake::Idle, vec![]));
}
#[test]
fn cancelled_runs_need_restart_and_fresh_programs_retarget_from_paint() {
    let base = config(1, &[1000.], 1000);
    let mut state = make_state(base.clone());
    paint(&mut state, 100);
    let mut cancelled = base.clone();
    cancelled.generation = 2;
    cancelled.playback = Playback::Cancelled;
    assert_eq!(
        state
            .update(Arc::new(cancelled.clone()), ms(100))
            .unwrap()
            .unwrap()
            .observation,
        Observation::Cancelled(CancelReason::Requested)
    );
    cancelled.generation = 3;
    cancelled.playback = Playback::Running;
    assert!(
        state
            .update(Arc::new(cancelled.clone()), ms(200))
            .unwrap()
            .is_none()
    );
    assert_eq!(paint(&mut state, 250), (100., Wake::Idle, vec![]));
    cancelled.generation = 4;
    cancelled.restart = 1;
    assert!(
        state
            .update(Arc::new(cancelled.clone()), ms(300))
            .unwrap()
            .is_none()
    );
    assert_eq!(paint(&mut state, 300).0, 0.);
    assert_eq!(paint(&mut state, 350).0, 50.);
    let mut backwards = cancelled.clone();
    backwards.generation = 5;
    backwards.restart = 0;
    assert_eq!(
        state.update(Arc::new(backwards), ms(350)),
        Err(Error::StaleRestart)
    );
    let different = config(5, &[2000.], 1000);
    assert_eq!(
        state
            .update(Arc::new(different), ms(350))
            .unwrap()
            .unwrap()
            .generation,
        4
    );
    assert_eq!(paint(&mut state, 350).0, 50.);
    let mut dormant = base;
    dormant.playback = Playback::Cancelled;
    let mut dormant = make_state(dormant);
    assert_eq!(paint(&mut dormant, 10), (0., Wake::Idle, vec![]));
}
#[test]
fn spring_retarget_preserves_last_painted_velocity_and_rejects_stale_previews() {
    let mut base = config(1, &[100.], 1000);
    base.program.stages[0].timing = Timing::Spring(Spring {
        stiffness: 100.,
        damping: 4.,
        mass: 1.,
        epsilon: 0.001,
        max_duration_ms: 10_000,
    });
    let mut state = make_state(base.clone());
    let at100 = sample(&mut state, 100);
    let painted = at100.frame;
    state.painted(at100);
    let discarded = sample(&mut state, 500);
    base.generation = 2;
    base.program.stages[0].targets = targets(-100.);
    state.update(Arc::new(base), ms(500)).unwrap();
    assert!(!state.accepts_sample(&discarded));
    assert_eq!(sample(&mut state, 500).frame, painted);
    assert!(value(&sample(&mut state, 501)) > painted.values.get(Property::Left).unwrap());
}
#[test]
fn repeat_phase_is_native_bounded_and_integer_precise_at_long_uptime() {
    let mut base = config(1, &[100., 200.], 100);
    base.program.repeat = Repeat::Alternate;
    let mut state = make_state(base);
    assert_eq!(paint(&mut state, 50).0, 50.);
    assert_eq!(paint(&mut state, 250).0, 150.);
    assert_eq!(paint(&mut state, 350).0, 50.);
    assert_eq!(paint(&mut state, 450).0, 50.);
    let (position, wake, events) = paint(&mut state, 1_000_000_000_000_050);
    assert_eq!(position, 50.);
    assert_eq!(wake, Wake::Frame);
    assert!(events.is_empty());
    let mut constant = config(1, &[0.], 100);
    constant.program.repeat = Repeat::Loop;
    let mut constant = make_state(constant);
    assert_eq!(paint(&mut constant, 0), (0., Wake::Idle, vec![]));
    assert_eq!(paint(&mut constant, 100), (0., Wake::Idle, vec![]));
    let mut zero = config(1, &[0.00001], 100);
    zero.program.repeat = Repeat::Loop;
    zero.program.stages[0].timing = Timing::Spring(Spring {
        stiffness: 100.,
        damping: 10.,
        mass: 1.,
        epsilon: 0.001,
        max_duration_ms: 10_000,
    });
    let mut zero = make_state(zero);
    assert_eq!(
        sample(&mut zero, 100).wake,
        Wake::Idle,
        "sub-tolerance physical periods cannot divide by zero or redraw"
    );
}
#[test]
fn shared_members_join_current_phase_and_clock_changes_fence_prepared_paint() {
    let mut base = config(1, &[1000.], 1000);
    base.program.repeat = Repeat::Loop;
    base.program.clock = Selection::Application;
    let mut clock = Clock::new(ms(0));
    let mut state = State::new(Arc::new(base), ms(500), false).unwrap();
    assert!(matches!(
        state.sample(ms(500), None),
        Err(Error::MissingClock)
    ));
    let prepared = state.sample(ms(500), Some(clock.sample(ms(500)))).unwrap();
    assert_eq!(value(&prepared), 500.);
    state.painted(prepared.clone());
    clock.set_paused(true, ms(600));
    assert!(!state.accepts_sample(&prepared));
    assert!(matches!(
        state.sample(ms(600), prepared.clock.clone()),
        Err(Error::StaleClock)
    ));
    let paused = state
        .sample(ms(1000), Some(clock.sample(ms(1000))))
        .unwrap();
    assert_eq!(value(&paused), 600.);
    assert_eq!(paused.wake, Wake::Idle);
    state.painted(paused);
    clock.set_paused(false, ms(1100));
    let running = state
        .sample(ms(1250), Some(clock.sample(ms(1250))))
        .unwrap();
    assert_eq!(value(&running), 750.);
    assert_eq!(running.wake, Wake::Frame);
    state.painted(running);
    state.set_visible(false, ms(1250));
    assert_eq!(sample(&mut state, 1400).wake, Wake::Idle);
    state.set_visible(true, ms(1400));
    let shown = state
        .sample(ms(1400), Some(clock.sample(ms(1400))))
        .unwrap();
    assert_eq!(value(&shown), 900.);
    clock.set_reduced(true, ms(1400));
    state.set_reduced(true, ms(1400));
    assert_eq!(paint(&mut state, 1500), (0., Wake::Idle, vec![]));
    clock.set_reduced(false, ms(1600));
    state.set_reduced(false, ms(1600));
    assert_eq!(
        value(
            &state
                .sample(ms(1600), Some(clock.sample(ms(1600))))
                .unwrap()
        ),
        900.
    );
}
#[test]
fn invalid_replacements_are_atomic_and_samples_do_not_retain_compiled_work() {
    let base = config(1, &[1000.], 1000);
    let mut state = make_state(base.clone());
    paint(&mut state, 100);
    let pending = sample(&mut state, 200);
    let mut stale = base.clone();
    stale.program.stages[0].targets = targets(2000.);
    assert_eq!(
        state.update(Arc::new(stale), ms(200)),
        Err(Error::StaleGeneration)
    );
    assert!(state.accepts_sample(&pending));
    let mut invalid = base;
    invalid.generation = 2;
    invalid.program.stages.clear();
    assert_eq!(
        state.update(Arc::new(invalid), ms(200)),
        Err(Error::InvalidConfig)
    );
    assert!(state.accepts_sample(&pending));
    let weak = Arc::downgrade(&state.tracks.first);
    assert!(state.retained_bytes() > 0);
    drop(state);
    assert!(weak.upgrade().is_none());
    assert_eq!(value(&pending), 200.);
    for _ in 0..256 {
        let state = make_state(config(1, &[100.; 32], 100));
        assert!(state.retained_bytes() < 128 * 1024);
        let weak = Arc::downgrade(&state.tracks.first);
        drop(state);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn wrong_clock_sources_and_disposed_groups_cannot_commit_frames() {
    use crate::motion_clock::Registry;
    use gpuio_protocol::{NodeId, WindowId};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut cfg = config(1, &[100.], 100);
    cfg.program.repeat = Repeat::Loop;
    cfg.program.clock = Selection::Group("pulse".into());
    let mut registry = Registry::new(ms(0));
    registry
        .replace_window(window, &[(node, Arc::new(cfg.program.clone()))], ms(0))
        .unwrap();
    let mut state = make_state(cfg);
    let mut foreign = Clock::new(ms(0));
    assert!(matches!(
        state.sample(ms(10), Some(foreign.sample(ms(10)))),
        Err(Error::WrongClock)
    ));
    let prepared = state
        .sample(ms(10), registry.sample(window, node, ms(10)))
        .unwrap();
    registry.close_window(window, ms(20));
    assert!(!state.accepts_sample(&prepared));
    assert!(state.painted(prepared).is_empty());
}

#[test]
fn maximum_spring_program_compilation_has_measured_bounded_retention() {
    use gpuio_protocol::animation::PROPERTY_COUNT;
    let properties = [
        Property::Width,
        Property::Height,
        Property::Top,
        Property::Right,
        Property::Bottom,
        Property::Left,
        Property::Opacity,
        Property::TopLeftRadius,
        Property::TopRightRadius,
        Property::BottomLeftRadius,
        Property::BottomRightRadius,
    ];
    // Absolute and multiplicative opacity are mutually exclusive.
    assert_eq!(properties.len(), PROPERTY_COUNT - 1);
    let values = |step: usize| {
        properties
            .iter()
            .map(|property| Target {
                property: *property,
                value: if *property == Property::Opacity {
                    if step.is_multiple_of(2) { 0.2 } else { 0.8 }
                } else {
                    step as f64 * 10.
                },
            })
            .collect()
    };
    let mut cfg = config(1, &[100.], 100);
    cfg.program.initial = Some(values(0));
    cfg.program.repeat = Repeat::Alternate;
    cfg.program.stages = (1..=32)
        .map(|step| Stage {
            targets: values(step),
            timing: Timing::Spring(Spring {
                stiffness: 100.,
                damping: 10.,
                mass: 1.,
                epsilon: 0.001,
                max_duration_ms: 10_000,
            }),
            delay_ms: 0,
        })
        .collect();
    let started = std::time::Instant::now();
    let mut largest = 0;
    for _ in 0..16 {
        let mut state = make_state(cfg.clone());
        paint(&mut state, 50);
        let mut changed = cfg.clone();
        changed.generation = 2;
        changed.program.stages[0].targets[0].value += 1.;
        state.update(Arc::new(changed), ms(50)).unwrap();
        largest = largest.max(state.retained_bytes());
        assert!(state.retained_bytes() < 256 * 1024);
        let old = Arc::downgrade(&state.tracks.first);
        drop(state);
        assert!(old.upgrade().is_none());
    }
    eprintln!(
        "GPUIO_ANIMATION_PROGRAM_COMPILE: 16 create/retarget/dispose cycles, 32 spring stages x 11 properties, maximum {largest} accounted bytes, elapsed {:?}",
        started.elapsed()
    );
}

#[test]
fn alternate_preserves_each_reversed_intervals_duration() {
    let mut cfg = config(1, &[100., 300.], 100);
    cfg.program.stages[1].timing = Timing::Tween(200, Easing::Linear);
    cfg.program.repeat = Repeat::Alternate;
    let mut state = make_state(cfg);
    for (time, expected) in [
        (50, 50.),
        (250, 250.),
        (350, 250.),
        (500, 100.),
        (550, 50.),
        (650, 50.),
    ] {
        let (position, _, events) = paint(&mut state, time);
        assert!(
            (position - expected).abs() < 1e-9,
            "time {time}: {position} != {expected}"
        );
        assert!(events.is_empty());
    }
}

#[test]
fn initial_delay_is_once_and_stage_delay_repeats_without_frame_polling() {
    let mut cfg = config(1, &[100.], 100);
    cfg.program.delay_ms = 50;
    cfg.program.stages[0].delay_ms = 10;
    cfg.program.repeat = Repeat::Loop;
    let mut state = make_state(cfg);
    assert_eq!(paint(&mut state, 0), (0., Wake::At(ms(60)), vec![]));
    assert_eq!(paint(&mut state, 110).0, 50.);
    assert_eq!(paint(&mut state, 160), (0., Wake::At(ms(170)), vec![]));
    assert_eq!(paint(&mut state, 210).0, 40.);
}

#[test]
fn out_of_order_prepared_paints_cannot_rewind_a_later_retarget() {
    let mut state = make_state(config(1, &[1000.], 1000));
    let old = sample(&mut state, 100);
    let newer = sample(&mut state, 200);
    state.painted(newer);
    assert!(!state.accepts_sample(&old));
    assert!(state.painted(old).is_empty());
    state
        .update(Arc::new(config(2, &[0.], 1000)), ms(200))
        .unwrap();
    assert_eq!(value(&sample(&mut state, 200)), 200.);
}

#[test]
fn later_paints_preserve_a_live_deadline_but_visibility_invalidates_it() {
    let mut cfg = config(1, &[100.], 100);
    cfg.program.delay_ms = 100;
    let mut state = make_state(cfg);
    let waiting = sample(&mut state, 0);
    assert_eq!(waiting.wake, Wake::At(ms(100)));
    paint(&mut state, 50);
    assert!(!state.accepts_sample(&waiting));
    assert!(state.accepts_wake(&waiting));
    state.set_visible(false, ms(75));
    assert!(!state.accepts_wake(&waiting));
    state.set_visible(true, ms(100));
    assert!(!state.accepts_wake(&waiting));
    let current = sample(&mut state, 100);
    assert_eq!(current.wake, Wake::At(ms(125)));
    assert!(state.accepts_wake(&current));
}

#[test]
fn negative_delay_skips_initial_stages_but_delivery_requires_paint() {
    let mut cfg = config(1, &[100., 200., 300.], 100);
    cfg.program.delay_ms = -250;
    let mut state = make_state(cfg.clone());
    let advanced = sample(&mut state, 0);
    assert_eq!(value(&advanced), 250.);
    assert_eq!(state.completed, 0);
    assert_eq!(advanced.wake, Wake::Frame);
    let events = state.painted(advanced);
    assert_eq!(events.iter().map(|e| e.index).collect::<Vec<_>>(), [1, 2]);
    assert!(events.iter().all(|e| matches!(
        e.observation,
        Observation::StageCompleted(_, StageResult::Played)
    )));
    let (value, wake, events) = paint(&mut state, 50);
    assert_eq!((value, wake), (300., Wake::Idle));
    assert_eq!(events.iter().map(|e| e.index).collect::<Vec<_>>(), [3, 33]);
    assert!(paint(&mut state, 1000).2.is_empty());
    cfg.program.delay_ms = -86_400_000;
    let mut finished = make_state(cfg);
    let terminal = sample(&mut finished, 0);
    assert!(!finished.is_finished());
    assert_eq!(terminal.wake, Wake::Idle);
    assert_eq!(finished.painted(terminal).len(), 4);
    assert!(finished.is_finished());

    let mut delayed = config(2, &[100.], 100);
    delayed.program.delay_ms = -50;
    delayed.program.stages[0].delay_ms = 100;
    let mut delayed = make_state(delayed);
    assert_eq!(paint(&mut delayed, 0), (0., Wake::At(ms(50)), vec![]));
    assert_eq!(paint(&mut delayed, 100).0, 50.);
}

#[test]
fn negative_delay_repetition_phase_uses_integer_cycles_without_cycle_events() {
    for (repeat, boundary) in [(Repeat::Loop, 0.), (Repeat::Alternate, 100.)] {
        let mut cfg = config(1, &[100.], 100);
        cfg.program.repeat = repeat;
        cfg.program.delay_ms = -1250;
        let mut state = make_state(cfg);
        assert_eq!(paint(&mut state, 0), (50., Wake::Frame, vec![]));
        assert_eq!(paint(&mut state, 50), (boundary, Wake::Frame, vec![]));
        assert_eq!(paint(&mut state, 100).0, 50.);
    }
}

#[test]
fn negative_delay_is_applied_once_across_hidden_paused_and_restarted_runs() {
    let mut cfg = config(1, &[1000.], 1000);
    cfg.program.delay_ms = -250;
    let mut state = make_state(cfg.clone());
    assert_eq!(paint(&mut state, 0).0, 250.);
    state.set_visible(false, ms(100));
    assert_eq!(paint(&mut state, 1000).0, 250.);
    state.set_visible(true, ms(1100));
    assert_eq!(paint(&mut state, 1100).0, 350.);
    cfg.generation = 2;
    cfg.playback = Playback::Paused;
    state.update(Arc::new(cfg.clone()), ms(1100)).unwrap();
    assert_eq!(paint(&mut state, 2000), (350., Wake::Idle, vec![]));
    cfg.generation = 3;
    cfg.playback = Playback::Running;
    state.update(Arc::new(cfg.clone()), ms(2100)).unwrap();
    assert_eq!(paint(&mut state, 2100).0, 350.);
    assert_eq!(paint(&mut state, 2200).0, 450.);
    cfg.generation = 4;
    cfg.restart = 1;
    state.update(Arc::new(cfg.clone()), ms(2200)).unwrap();
    assert_eq!(paint(&mut state, 2200).0, 250.);
    let mut reduced = State::new(Arc::new(cfg), ms(0), true).unwrap();
    let (value, wake, events) = paint(&mut reduced, 0);
    assert_eq!((value, wake), (1000., Wake::Idle));
    assert_eq!(
        events[0].observation,
        Observation::StageCompleted(0, StageResult::ReducedMotion)
    );
}

#[test]
fn negative_spring_delay_samples_the_existing_analytic_trajectory() {
    let mut cfg = config(1, &[100.], 1000);
    cfg.program.stages[0].timing = Timing::Spring(Spring {
        stiffness: 100.,
        damping: 4.,
        mass: 1.,
        epsilon: 0.001,
        max_duration_ms: 10000,
    });
    let mut original = make_state(cfg.clone());
    cfg.program.delay_ms = -123;
    let mut advanced = make_state(cfg);
    for elapsed in [0, 10, 100, 500, 10000] {
        let actual = sample(&mut advanced, elapsed);
        let expected = sample(&mut original, elapsed + 123);
        assert_eq!(actual.frame, expected.frame);
        assert_eq!(actual.finished, expected.finished);
    }
}

#[test]
fn explicit_finite_directions_use_timeline_time_and_one_painted_terminal() {
    use gpuio_protocol::animation::{Direction, IterationCount};
    for direction in [
        Direction::Normal,
        Direction::Reverse,
        Direction::Alternate,
        Direction::AlternateReverse,
    ] {
        for count in [0, 1, 2, 3] {
            let mut cfg = config(1, &[100.], 100);
            cfg.program.stages[0].timing = Timing::Tween(100, Easing::EaseIn);
            cfg.program.repeat = Repeat::Finite(IterationCount::new(count), direction);
            let mut state = make_state(cfg);
            for time in (0..count * 100).step_by(25) {
                let iteration = time / 100;
                let phase = (time % 100) as f64 / 100.;
                let phase = if direction.reverses(iteration as u128) {
                    1. - phase
                } else {
                    phase
                };
                let sample = sample(&mut state, time);
                assert!(
                    (value(&sample) - Easing::EaseIn.sample(phase) * 100.).abs() < 1e-9,
                    "{direction:?} {count} {time}"
                );
                assert!(state.painted(sample).is_empty());
            }
            let end = sample(&mut state, count * 100);
            assert!(!state.is_finished());
            let expected = if direction.reverses(count.saturating_sub(1) as u128) {
                0.
            } else {
                100.
            };
            assert_eq!(value(&end), expected);
            assert_eq!(end.wake, Wake::Idle);
            let duplicate = end.clone();
            let events = state.painted(end);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].observation, Observation::Finished);
            assert_eq!(events[0].index, TERMINAL_INDEX);
            assert!(state.painted(duplicate).is_empty());
        }
    }
}

#[test]
fn explicit_reverse_traverses_stage_pauses_without_frame_polling() {
    use gpuio_protocol::animation::Direction;
    let mut cfg = config(1, &[100., 200.], 100);
    cfg.program.repeat = Repeat::Infinite(Direction::Reverse);
    cfg.program.delay_ms = 50;
    cfg.program.stages[1].delay_ms = 80;
    let mut state = make_state(cfg);
    assert_eq!(paint(&mut state, 0), (200., Wake::At(ms(50)), vec![]));
    assert_eq!(paint(&mut state, 100).0, 150.);
    assert_eq!(paint(&mut state, 150), (100., Wake::At(ms(230)), vec![]));
    assert_eq!(paint(&mut state, 200), (100., Wake::At(ms(230)), vec![]));
    assert_eq!(paint(&mut state, 280).0, 50.);
    assert_eq!(paint(&mut state, 330).0, 200.);
}

#[test]
fn huge_finite_counts_are_bounded_and_do_not_finish_at_a_saturated_clock() {
    use gpuio_protocol::animation::{Direction, IterationCount};
    let mut cfg = config(1, &[0.], 86_400_000);
    cfg.program.repeat = Repeat::Finite(IterationCount::new(u64::MAX), Direction::Normal);
    let mut state = make_state(cfg);
    assert_eq!(paint(&mut state, 0), (0., Wake::At(ms(86_400_000)), vec![]));
    assert!(
        std::time::Instant::now()
            .checked_add(ms(86_400_000))
            .is_some()
    );
    assert!(paint(&mut state, 86_400_000).2.is_empty());
    let sample = state.sample(Duration::MAX, None).unwrap();
    assert!(!sample.finished);
    assert!(state.painted(sample).is_empty());
    assert!(!state.is_finished());
    assert!(state.retained_bytes() < 20_000);
}

#[test]
fn explicit_reverse_spring_uses_the_same_trajectory_with_reversed_velocity() {
    use gpuio_protocol::animation::{Direction, IterationCount};
    let mut cfg = config(1, &[100.], 100);
    cfg.program.stages[0].timing = Timing::Spring(Spring {
        stiffness: 100.,
        damping: 10.,
        mass: 1.,
        epsilon: 0.001,
        max_duration_ms: 10_000,
    });
    cfg.program.repeat = Repeat::Finite(IterationCount::new(1), Direction::Reverse);
    let mut reverse = make_state(cfg.clone());
    cfg.program.repeat = Repeat::Finite(IterationCount::new(1), Direction::Normal);
    let forward = make_state(cfg);
    let duration = forward.tracks.first.duration();
    for fraction in [0.1, 0.3, 0.7] {
        let at = duration.mul_f64(fraction);
        let a = reverse.sample(at, None).unwrap().frame;
        let b = forward.tracks.first.sample(duration - at).frame;
        assert_eq!(a.values, b.values);
        assert_eq!(
            a.velocity.get(Property::Left),
            b.velocity.get(Property::Left).map(|v| -v)
        );
    }
}

#[test]
fn explicit_shared_direction_retargets_declared_ranges_at_current_group_phase() {
    use gpuio_protocol::animation::Direction;
    let mut cfg = config(1, &[1000.], 1000);
    cfg.program.repeat = Repeat::Infinite(Direction::Reverse);
    cfg.program.clock = Selection::Application;
    let mut state = make_state(cfg.clone());
    let mut clock = Clock::new(ms(0));
    let at = state.sample(ms(250), Some(clock.sample(ms(250)))).unwrap();
    assert_eq!(value(&at), 750.);
    state.painted(at);
    cfg.generation = 2;
    cfg.program.stages[0].targets = targets(2000.);
    state.update(Arc::new(cfg), ms(250)).unwrap();
    let changed = state.sample(ms(250), Some(clock.sample(ms(250)))).unwrap();
    assert_eq!(
        value(&changed),
        1500.,
        "shared phase uses declared range, not retained retarget position"
    );
}

#[test]
fn reverse_active_endpoint_samples_linear_stops_before_forcing_finite_completion() {
    use gpuio_protocol::animation::{Direction, IterationCount, LinearStops};
    let mut cfg = config(1, &[100.], 100);
    cfg.program.stages[0].timing = Timing::Tween(
        100,
        Easing::LinearStops(LinearStops::new(vec![(0., 0.25), (1., 0.5)]).unwrap()),
    );
    cfg.program.repeat = Repeat::Finite(IterationCount::new(1), Direction::Reverse);
    let mut state = make_state(cfg);
    assert_eq!(paint(&mut state, 0).0, 50.);
    assert_eq!(paint(&mut state, 50).0, 37.5);
    let end = paint(&mut state, 100);
    assert_eq!(
        end.0, 0.,
        "finite completion keeps the exact directed endpoint contract"
    );
    assert_eq!(end.2.len(), 1);
}

#[test]
fn explicit_finite_offsets_suspend_restart_and_reduce_without_cycle_events() {
    use gpuio_protocol::animation::{Direction, IterationCount};
    let mut cfg = config(1, &[100.], 100);
    cfg.program.repeat = Repeat::Finite(IterationCount::new(3), Direction::AlternateReverse);
    cfg.program.delay_ms = -125;
    let mut state = make_state(cfg.clone());
    assert_eq!(paint(&mut state, 0).0, 25.);
    state.set_visible(false, ms(50));
    assert_eq!(paint(&mut state, 400), (25., Wake::Idle, vec![]));
    state.set_visible(true, ms(500));
    assert!((paint(&mut state, 500).0 - 75.).abs() < 1e-9);
    cfg.playback = Playback::Paused;
    cfg.generation = 2;
    state.update(Arc::new(cfg.clone()), ms(500)).unwrap();
    let paused = paint(&mut state, 1500);
    assert!((paused.0 - 75.).abs() < 1e-9);
    assert_eq!((paused.1, paused.2), (Wake::Idle, vec![]));
    cfg.playback = Playback::Running;
    cfg.restart = 1;
    cfg.generation = 3;
    state.update(Arc::new(cfg), ms(1500)).unwrap();
    assert_eq!(paint(&mut state, 1500).0, 25.);
    let stale = sample(&mut state, 1525);
    state.set_reduced(true, ms(1525));
    assert!(!state.accepts_sample(&stale));
    let end = paint(&mut state, 1525);
    assert_eq!(end.0, 0.);
    assert_eq!(end.1, Wake::Idle);
    assert_eq!(end.2.len(), 1);
    assert_eq!(end.2[0].observation, Observation::Finished);
}
