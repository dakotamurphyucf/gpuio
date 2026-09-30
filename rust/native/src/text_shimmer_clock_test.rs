use super::*;
use gpuio_protocol::text_shimmer::{Direction, Spread};

fn config(repeat: Repeat) -> Config {
    Config {
        duration_ms: 1000,
        spread: Spread::Relative(0.3),
        direction: Direction::LeftToRight,
        repeat,
        animated: true,
        highlight: Some(0xff0000ff),
        appearance: None,
    }
}

fn setup(repeat: Repeat) -> (Owner, Rc<Clock>) {
    let clock = Rc::new(Clock::default());
    clock.test_now.set(Some(Duration::ZERO));
    let owner = Owner::new(Arc::from("Working"), config(repeat), clock.clone()).unwrap();
    (owner, clock)
}

fn at(clock: &Clock, ms: u64) {
    clock.test_now.set(Some(Duration::from_millis(ms)));
}
fn sample(owner: &Owner) -> f32 {
    owner.0.borrow_mut().sample(false).phase
}
fn paint(owner: &Owner) -> bool {
    owner.0.borrow_mut().painted(Report::OutsideBand, false)
}

#[test]
fn elapsed_is_monotonic_paused_when_hidden_and_modulo_before_float_conversion() {
    let (owner, clock) = setup(Repeat::Loop);
    assert_eq!(sample(&owner), 0.);
    assert!(paint(&owner));
    at(&clock, 250);
    assert_eq!(sample(&owner), 0.25);
    at(&clock, 1250);
    assert_eq!(sample(&owner), 0.25);
    owner.suspend();
    at(&clock, 5000);
    assert_eq!(sample(&owner), 0.25);
    paint(&owner);
    at(&clock, 5250);
    assert_eq!(sample(&owner), 0.5);
    at(&clock, 5200);
    assert_eq!(sample(&owner), 0.5);
    at(&clock, 5500);
    assert_eq!(sample(&owner), 0.75);
    // Large wall-clock values don't lose sub-second phase precision.
    at(&clock, 9_000_000_000_000_500);
    assert_eq!(sample(&owner), 0.75);
}

#[test]
fn completed_once_stays_idle_across_cosmetic_updates_and_restarts_on_source_or_timing() {
    let (owner, clock) = setup(Repeat::Once);
    paint(&owner);
    at(&clock, 1500);
    assert_eq!(sample(&owner), 1.);
    assert!(!paint(&owner));
    assert!(!owner.0.borrow_mut().delivered(false));
    let mut next = config(Repeat::Once);
    next.spread = Spread::Pixels(30.);
    next.highlight = None;
    owner.update(Arc::from("Working"), next).unwrap();
    at(&clock, 8000);
    assert_eq!(sample(&owner), 1.);
    assert!(!paint(&owner));
    owner.update(Arc::from("Next"), next).unwrap();
    assert_eq!(sample(&owner), 0.);
    assert!(paint(&owner));
    at(&clock, 8250);
    assert_eq!(sample(&owner), 0.25);
    next.duration_ms = 2000;
    owner.update(Arc::from("Next"), next).unwrap();
    assert_eq!(sample(&owner), 0.);
    paint(&owner);
    at(&clock, 8750);
    assert_eq!(sample(&owner), 0.25);
    next.direction = Direction::RightToLeft;
    owner.update(Arc::from("Next"), next).unwrap();
    assert_eq!(sample(&owner), 0.);
}

#[test]
fn reduce_static_capacity_and_skipped_paint_pause_without_accumulating_wakes() {
    let (owner, clock) = setup(Repeat::Loop);
    assert!(paint(&owner));
    for _ in 0..1000 {
        assert!(!paint(&owner));
    }
    at(&clock, 250);
    assert!(owner.0.borrow_mut().sample(true).reduced_motion);
    assert!(!owner.0.borrow_mut().painted(Report::OutsideBand, true));
    assert!(!owner.0.borrow_mut().delivered(true));
    at(&clock, 2000);
    assert_eq!(sample(&owner), 0.25);
    assert!(paint(&owner));
    let mut next = config(Repeat::Loop);
    next.animated = false;
    owner.update(Arc::from("Working"), next).unwrap();
    at(&clock, 8000);
    assert_eq!(sample(&owner), 0.25);
    assert!(!paint(&owner));
    assert!(!owner.0.borrow_mut().delivered(false));
    next.animated = true;
    owner.update(Arc::from("Working"), next).unwrap();
    assert!(paint(&owner));
    at(&clock, 8250);
    assert_eq!(sample(&owner), 0.5);
    assert!(!owner.0.borrow_mut().painted(Report::Capacity, false));
    assert!(!owner.0.borrow_mut().delivered(false));
    at(&clock, 16000);
    assert_eq!(sample(&owner), 0.5);
    assert!(paint(&owner));
    let text = owner.element(
        StyledText::new("Working"),
        Appearance {
            foreground: gpui::black(),
            background: gpui::white(),
            dark: false,
        },
    );
    assert!(
        !owner.0.borrow_mut().delivered(false),
        "constructing but skipping paint disarms a wake"
    );
    drop(text);
    at(&clock, 18000);
    assert_eq!(sample(&owner), 0.5);
}

#[test]
fn updates_are_atomic_drivers_are_stamped_and_no_rendered_element_owns_the_clock() {
    let (owner, clock) = setup(Repeat::Loop);
    let weak = Rc::downgrade(&owner.0);
    let driver = Driver {
        state: weak.clone(),
        stamp: Rc::downgrade(&owner.0.borrow().stamp),
    };
    paint(&owner);
    at(&clock, 250);
    assert_eq!(sample(&owner), 0.25);
    let old_source = Arc::downgrade(&owner.0.borrow().source);
    let replacement: Arc<str> = Arc::from("Working");
    owner
        .update(replacement.clone(), config(Repeat::Loop))
        .unwrap();
    assert!(
        Arc::ptr_eq(&replacement, &owner.0.borrow().source),
        "equal text must share the current tree allocation"
    );
    assert!(
        old_source.upgrade().is_none(),
        "an equal source update must release the old allocation"
    );
    assert_eq!(
        driver.sample(false).unwrap().1.phase,
        0.25,
        "allocation replacement preserves phase and paint-driver stamp"
    );
    let old_stamp = owner.0.borrow().stamp.clone();
    let mut invalid = config(Repeat::Loop);
    invalid.duration_ms = 0;
    assert_eq!(
        owner.update(Arc::from("Working"), invalid),
        Err(Error::InvalidConfig)
    );
    assert_eq!(
        owner.update(
            Arc::from("x".repeat(MAX_TEXT_BYTES + 1)),
            config(Repeat::Loop)
        ),
        Err(Error::SourceLimit)
    );
    assert!(Rc::ptr_eq(&old_stamp, &owner.0.borrow().stamp));
    assert_eq!(driver.sample(false).unwrap().1.phase, 0.25);
    let mut next = config(Repeat::Loop);
    next.highlight = None;
    owner.update(Arc::from("Working"), next).unwrap();
    assert_eq!(sample(&owner), 0.25);
    assert!(
        driver.sample(false).is_none(),
        "old driver cannot paint newer config even when old stamp is kept alive"
    );
    let text = owner.element(
        StyledText::new("Working"),
        Appearance {
            foreground: gpui::black(),
            background: gpui::white(),
            dark: false,
        },
    );
    assert_eq!(Rc::strong_count(&clock), 2);
    drop(owner);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        Rc::strong_count(&clock),
        1,
        "cached text and callback weak handle do not retain owner/clock"
    );
    drop(text);
}

#[test]
fn invisible_content_and_instances_do_not_share_progress() {
    let (first, clock) = setup(Repeat::Loop);
    paint(&first);
    at(&clock, 500);
    assert_eq!(sample(&first), 0.5);
    let second = Owner::new(Arc::from("Second"), config(Repeat::Loop), clock.clone()).unwrap();
    assert_eq!(sample(&second), 0.);
    paint(&second);
    at(&clock, 750);
    assert_eq!(sample(&first), 0.75);
    assert_eq!(sample(&second), 0.25);
    for source in ["", " \t\n\u{2003}"] {
        second
            .update(Arc::from(source), config(Repeat::Loop))
            .unwrap();
        assert!(!paint(&second));
    }
}

#[test]
fn admission_reservation_covers_fixed_state_and_callback_payloads() {
    // Budget fixed payloads explicitly; the extra 512 bytes allow for allocation,
    // BTreeMap entry and callback boxing overhead. Source bytes are shared with
    // the retained tree; no source-sized allocation belongs to this clock.
    let fixed = std::mem::size_of::<RefCell<State>>()
        + std::mem::size_of::<Config>()
        + std::mem::size_of::<Owner>()
        + std::mem::size_of::<Driver>()
        + std::mem::size_of::<gpui::EntityId>()
        + 4 * std::mem::size_of::<usize>();
    assert!(fixed + 512 <= RESERVED_BYTES);
}

#[test]
fn visible_layout_time_counts_toward_the_sweep() {
    let (owner, clock) = setup(Repeat::Once);
    paint(&owner);
    at(&clock, 100);
    let _element = owner.element(
        StyledText::new("Working"),
        Appearance {
            foreground: gpui::black(),
            background: gpui::white(),
            dark: false,
        },
    );
    // Model expensive layout without sleeping or depending on machine speed.
    at(&clock, 600);
    assert_eq!(
        sample(&owner),
        0.6,
        "visible layout must not slow native time"
    );
    paint(&owner);
    at(&clock, 1000);
    assert_eq!(sample(&owner), 1.);
}

#[test]
fn omitted_frame_discards_layout_interval_even_after_the_wake_was_delivered() {
    let (owner, clock) = setup(Repeat::Loop);
    paint(&owner);
    assert!(owner.0.borrow_mut().delivered(false));
    at(&clock, 100);
    owner.prepare_frame();
    at(&clock, 200);
    owner.prepare_frame(); // Repeated construction cannot change the cutoff.
    at(&clock, 600);
    owner.finish_frame(); // No paint, and no pending wake to clean it up.
    at(&clock, 10_000);
    owner.prepare_frame();
    assert_eq!(
        sample(&owner),
        0.1,
        "hidden time must not enter the next sweep"
    );
    paint(&owner);
    at(&clock, 10_250);
    assert_eq!(sample(&owner), 0.35);
    owner.prepare_frame();
    owner.suspend(); // Explicit lifecycle suspension also cancels resumption.
    at(&clock, 20_000);
    assert_eq!(sample(&owner), 0.35);
}
