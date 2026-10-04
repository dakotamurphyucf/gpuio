use super::*;
use gpuio_protocol::progress::ProgressConfig;

fn config(fraction: Option<f64>) -> Arc<Config> {
    Arc::new(Config {
        progress: ProgressConfig {
            label: "Work".into(),
            fraction,
        },
        shape: Shape::Circle,
        transition: Transition::Tween {
            duration_ms: 1000,
            easing: Easing::Linear,
        },
    })
}
fn setup(fraction: Option<f64>) -> (Owner, Rc<Clock>) {
    let clock = Rc::new(Clock::default());
    clock.set_test_time(Duration::ZERO);
    (Owner::new(config(fraction), clock.clone()).unwrap(), clock)
}
fn at(clock: &Clock, ms: u64) {
    clock.set_test_time(Duration::from_millis(ms));
}
fn paint(owner: &Owner, static_presentation: bool) -> Sample {
    owner.prepare_frame();
    let sample = owner.driver().sample(static_presentation).unwrap();
    owner.0.borrow_mut().painted(true, static_presentation);
    owner.finish_frame();
    sample
}
fn fraction(sample: Sample, expected: f64) {
    let Value::Determinate(actual) = sample.value else {
        panic!("expected a measured fraction")
    };
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}
fn phase(sample: Sample, expected: f32, is_static: bool) {
    assert_eq!(
        sample.value,
        Value::Indeterminate {
            phase: expected,
            static_presentation: is_static
        }
    );
}

#[test]
fn initial_mount_has_no_intro_and_target_updates_finish_exactly_without_reset_on_label() {
    let (owner, clock) = setup(Some(0.25));
    fraction(paint(&owner, false), 0.25);
    assert!(!owner.0.borrow().running);
    owner.update(config(Some(0.75))).unwrap();
    fraction(paint(&owner, false), 0.25);
    assert_eq!(
        owner.0.borrow().config.progress.fraction,
        Some(0.75),
        "semantics stay at target"
    );
    at(&clock, 500);
    fraction(paint(&owner, false), 0.5);
    let mut renamed = (*config(Some(0.75))).clone();
    renamed.progress.label = "Renamed".into();
    let stale = owner.driver();
    owner.update(Arc::new(renamed)).unwrap();
    assert!(stale.sample(false).is_none());
    fraction(paint(&owner, false), 0.5);
    at(&clock, 750);
    fraction(paint(&owner, false), 0.625);
    at(&clock, 1000);
    fraction(paint(&owner, false), 0.75);
    assert!(!owner.0.borrow().running);
    assert!(!owner.0.borrow_mut().delivered(false));
    at(&clock, 10000);
    fraction(paint(&owner, false), 0.75);
}

#[test]
fn interruption_and_transition_policy_changes_start_from_current_displayed_value() {
    let (owner, clock) = setup(Some(0.));
    paint(&owner, false);
    owner.update(config(Some(1.))).unwrap();
    paint(&owner, false);
    at(&clock, 400);
    fraction(paint(&owner, false), 0.4);
    owner.update(config(Some(0.2))).unwrap();
    fraction(paint(&owner, false), 0.4);
    at(&clock, 900);
    fraction(paint(&owner, false), 0.3);
    let slower = Arc::new(Config {
        transition: Transition::Tween {
            duration_ms: 2000,
            easing: Easing::Linear,
        },
        ..(*config(Some(0.2))).clone()
    });
    owner.update(slower).unwrap();
    fraction(paint(&owner, false), 0.3);
    at(&clock, 1900);
    fraction(paint(&owner, false), 0.25);
    owner
        .update(Arc::new(Config {
            transition: Transition::Immediate,
            ..(*config(Some(1.))).clone()
        }))
        .unwrap();
    fraction(paint(&owner, false), 1.);
    assert!(!owner.0.borrow().running);
}

#[test]
fn reduced_inert_and_hidden_progress_snap_to_current_target_and_drop_time_debt() {
    let (owner, clock) = setup(Some(0.));
    paint(&owner, false);
    owner.update(config(Some(1.))).unwrap();
    paint(&owner, false);
    at(&clock, 300);
    fraction(paint(&owner, true), 1.);
    assert!(!owner.0.borrow().running);
    at(&clock, 10000);
    fraction(paint(&owner, false), 1.);
    owner.update(config(Some(0.))).unwrap();
    paint(&owner, false);
    at(&clock, 10200);
    owner.prepare_frame();
    // A frame that never paints must not continue the old transition on reveal.
    at(&clock, 10800);
    owner.finish_frame();
    assert!(!owner.0.borrow().visible);
    owner.update(config(Some(0.6))).unwrap();
    at(&clock, 20000);
    fraction(paint(&owner, false), 0.6);
    assert!(!owner.0.borrow().running);
    owner.update(config(Some(1.))).unwrap();
    paint(&owner, false);
    assert!(
        owner.0.borrow_mut().delivered(true),
        "reduced change still needs a final static paint"
    );
    fraction(paint(&owner, true), 1.);
    assert!(!owner.0.borrow_mut().delivered(true));
}

#[test]
fn indeterminate_cycles_pause_omitted_intervals_and_never_become_measured_values() {
    let (owner, clock) = setup(None);
    phase(paint(&owner, false), 0., false);
    at(&clock, 250);
    phase(paint(&owner, false), 0.25, false);
    at(&clock, 200); // Defensive monotonic source guard.
    phase(paint(&owner, false), 0.25, false);
    at(&clock, 300);
    owner.prepare_frame();
    at(&clock, 700);
    // Before finish, paint eligibility is unresolved. One pending wake may
    // request that paint; only the completed omitted frame can declare idle.
    assert!(owner.0.borrow_mut().delivered(false));
    owner.finish_frame();
    assert!(!owner.0.borrow_mut().delivered(false));
    at(&clock, 10000);
    phase(paint(&owner, false), 0.3, false);
    at(&clock, 10100);
    phase(paint(&owner, true), 0., true);
    at(&clock, 20000);
    phase(paint(&owner, false), 0.4, false);
    owner.update(config(Some(0.7))).unwrap();
    fraction(paint(&owner, false), 0.7);
    owner.update(config(None)).unwrap();
    phase(paint(&owner, false), 0., false);
    at(&clock, 20250);
    phase(paint(&owner, false), 0.25, false);
    owner
        .update(Arc::new(Config {
            shape: Shape::Linear,
            ..(*config(None)).clone()
        }))
        .unwrap();
    phase(paint(&owner, false), 0., false);
    at(&clock, 21000);
    phase(paint(&owner, false), 0.5, false);
}

#[test]
fn elapsed_is_reduced_before_float_conversion_and_extreme_easing_remains_bounded() {
    let (owner, clock) = setup(None);
    paint(&owner, false);
    at(&clock, 9_000_000_000_000_750);
    phase(paint(&owner, false), 0.75, false);
    for target in [0., 1.] {
        let (owner, clock) = setup(Some(1. - target));
        paint(&owner, false);
        owner
            .update(Arc::new(Config {
                transition: Transition::Tween {
                    duration_ms: 1000,
                    easing: Easing::CubicBezier(0., -f64::MAX, 1., f64::MAX),
                },
                ..(*config(Some(target))).clone()
            }))
            .unwrap();
        paint(&owner, false);
        for ms in 1..1000 {
            at(&clock, ms);
            let Value::Determinate(value) = paint(&owner, false).value else {
                unreachable!()
            };
            assert!(value.is_finite() && (0. ..=1.).contains(&value));
        }
        at(&clock, 1000);
        fraction(paint(&owner, false), target);
        assert!(!owner.0.borrow().running);
    }
}

#[test]
fn invalid_updates_are_atomic_and_stale_drivers_cannot_keep_or_revive_owners() {
    let (owner, clock) = setup(Some(0.));
    paint(&owner, false);
    owner.update(config(Some(1.))).unwrap();
    paint(&owner, false);
    at(&clock, 250);
    let stale = owner.driver();
    let original = owner.0.borrow().config.clone();
    let invalid = Arc::new(Config {
        transition: Transition::Tween {
            duration_ms: 0,
            easing: Easing::Linear,
        },
        ..(*config(Some(0.5))).clone()
    });
    assert_eq!(owner.update(invalid.clone()), Err(InvalidConfig));
    assert!(Owner::new(invalid, clock).is_err());
    assert!(Arc::ptr_eq(&original, &owner.0.borrow().config));
    fraction(stale.sample(false).unwrap(), 0.25);
    for index in 0..200 {
        owner.update(config(Some(index as f64 / 200.))).unwrap();
        paint(&owner, false);
        assert!(
            owner.0.borrow().pending,
            "configuration changes must not stack wakes"
        );
    }
    assert!(stale.sample(false).is_none());
    let current = owner.driver();
    let weak = Rc::downgrade(&owner.0);
    drop(owner);
    assert!(weak.upgrade().is_none());
    assert!(current.sample(false).is_none());
    current.suspend();
}

#[test]
fn visible_layout_time_counts_and_the_final_wake_is_not_lost() {
    let (owner, clock) = setup(Some(0.));
    paint(&owner, false);
    owner.update(config(Some(1.))).unwrap();
    paint(&owner, false);
    at(&clock, 200);
    owner.prepare_frame();
    let driver = owner.driver();
    at(&clock, 250);
    fraction(driver.sample(false).unwrap(), 0.25);
    owner.0.borrow_mut().painted(true, false);
    owner.finish_frame();
    at(&clock, 1000);
    assert!(owner.0.borrow_mut().delivered(false));
    fraction(paint(&owner, false), 1.);
    assert!(!owner.0.borrow().pending);
}

#[cfg(feature = "native-image-tests")]
#[test]
fn native_frame_queue_is_bounded_and_cannot_retain_a_removed_owner() {
    use gpui::{Context, IntoElement, Render, TestAppContext, canvas, prelude::*, px, rgb};
    struct Fixture {
        owner: Option<Owner>,
    }
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let driver = self.owner.as_ref().map(Owner::driver);
            canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    if let Some(driver) = driver
                        && driver.sample(false).is_some()
                    {
                        window.paint_quad(gpui::fill(bounds, rgb(0xff0000)));
                        driver.painted(true, false, window, cx);
                    }
                },
            )
            .w(px(10.))
            .h(px(10.))
        }
    }
    let mut app = TestAppContext::single();
    let (owner, _) = setup(None);
    let weak = Rc::downgrade(&owner.0);
    let config_weak = Arc::downgrade(&owner.0.borrow().config);
    let (view, cx) = app.add_window_view(|_, _| Fixture { owner: Some(owner) });
    // Consume any startup frame callback, then build one pending clock frame.
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
    view.read_with(cx, |view, _| view.owner.as_ref().unwrap().finish_frame());
    for index in 0..200 {
        view.update(cx, |view, cx| {
            let mut next = (*config(None)).clone();
            next.progress.label = format!("Stage {index}");
            view.owner.as_ref().unwrap().update(Arc::new(next)).unwrap();
            cx.notify();
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        view.read_with(cx, |view, _| view.owner.as_ref().unwrap().finish_frame());
    }
    cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 1));
    assert!(
        config_weak.upgrade().is_none(),
        "replacement does not retain old labels"
    );
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    view.update(cx, |view, cx| {
        view.owner = None;
        cx.notify();
    });
    assert!(
        weak.upgrade().is_none(),
        "rendered elements and pending callbacks must be weak"
    );
    cx.update(|window, cx| {
        assert_eq!(window.simulate_next_frame(cx), 1);
        window.draw(cx).clear(cx);
        assert_eq!(window.simulate_next_frame(cx), 0);
        window.remove_window();
    });
}

#[test]
fn wake_during_prepared_layout_preserves_only_intervals_that_actually_paint() {
    let (owner, clock) = setup(Some(0.));
    paint(&owner, false);
    owner.update(config(Some(1.))).unwrap();
    paint(&owner, false);
    at(&clock, 100);
    owner.prepare_frame();
    at(&clock, 200);
    assert!(owner.0.borrow_mut().delivered(false));
    at(&clock, 250);
    fraction(paint(&owner, false), 0.25);

    let (owner, clock) = setup(None);
    paint(&owner, false);
    at(&clock, 100);
    owner.prepare_frame();
    at(&clock, 200);
    assert!(owner.0.borrow_mut().delivered(false));
    at(&clock, 600);
    owner.finish_frame(); // Hidden after prepare: do not replay this interval.
    at(&clock, 10000);
    phase(paint(&owner, false), 0.1, false);
    at(&clock, 10100);
    owner.prepare_frame();
    assert!(owner.0.borrow_mut().delivered(true));
    phase(paint(&owner, true), 0., true);
    assert!(!owner.0.borrow().running);
    assert!(!owner.0.borrow_mut().delivered(true));
}
