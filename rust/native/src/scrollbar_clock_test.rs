//! Scheduling through GPUI TestPlatform; no OS window or desktop input.
use super::*;
use gpui::{
    Context, Entity, IntoElement, Render, TestAppContext, VisualTestContext, canvas, prelude::*, px,
};
struct Fixture {
    owner: Option<Owner>,
    omitted: bool,
    width: f64,
    delayed: Option<(Driver, Frame)>,
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let driver = self.owner.as_ref().and_then(|owner| {
            owner.prepare_frame();
            if self.omitted {
                None
            } else {
                owner.set_eligible(true);
                Some(owner.driver())
            }
        });
        let width = self.width;
        let delayed = self.delayed.take();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| {
                let sample = delayed.or_else(|| {
                    driver.and_then(|driver| {
                        driver
                            .sample(16., width, cx)
                            .unwrap()
                            .map(|frame| (driver, frame))
                    })
                });
                if let Some((driver, frame)) = sample {
                    window.paint_quad(gpui::fill(
                        bounds,
                        gpui::red().opacity(frame.visual().opacity as f32),
                    ));
                    assert!(driver.painted(&frame, window, cx));
                }
            },
        )
        .w(px(16.))
        .h(px(100.))
    }
}
fn draw(view: &Entity<Fixture>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    view.read_with(cx, |view, _| {
        if let Some(owner) = &view.owner {
            owner.finish_frame();
        }
    });
}
fn count(view: &Entity<Fixture>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |view, _| {
        view.owner.as_ref().unwrap().0.borrow().notifications
    })
}
fn active(view: &Entity<Fixture>, cx: &mut VisualTestContext) {
    view.update(cx, |view, cx| {
        let owner = view.owner.as_ref().unwrap();
        owner.set_eligible(true);
        owner.activity(cx);
        cx.notify();
    });
    draw(view, cx);
    cx.run_until_parked();
}
fn motion() -> Motion {
    Motion {
        idle_ms: 1000,
        ..Motion::default()
    }
}

#[test]
fn actual_idle_task_expires_once_and_dropping_owner_cancels_retention() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, cx| Fixture {
        owner: Some(Owner::new(Mode::Scrolling, motion(), cx).unwrap()),
        omitted: false,
        width: 6.,
        delayed: None,
    });
    active(&view, cx);
    view.read_with(cx, |view, _| {
        assert!(view.owner.as_ref().unwrap().0.borrow().timer.is_some())
    });
    let before = count(&view, cx);
    cx.executor().advance_clock(Duration::from_millis(999));
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before);
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before + 1);
    draw(&view, cx);
    view.read_with(cx, |view, _| {
        let state = view.owner.as_ref().unwrap().0.borrow();
        assert!(state.timer.is_none());
        assert!(!state.frame_needed);
        assert!(!state.model.accepts_pointer());
    });
    cx.executor().advance_clock(Duration::from_secs(60));
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before + 1);
    active(&view, cx);
    let weak = view.read_with(cx, |view, _| Rc::downgrade(&view.owner.as_ref().unwrap().0));
    view.update(cx, |view, cx| {
        view.owner = None;
        cx.notify();
    });
    assert!(
        weak.upgrade().is_none(),
        "timer and painted drivers must be weak"
    );
    cx.executor().advance_clock(Duration::from_secs(60));
    cx.run_until_parked();
    draw(&view, cx);
    cx.update(|window, _| window.remove_window());
}

#[test]
fn repeated_updates_have_one_frame_slot_and_omission_disarms_all_work() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, cx| Fixture {
        owner: Some(
            Owner::new(
                Mode::Always,
                Motion {
                    expand_ms: 100,
                    ..motion()
                },
                cx,
            )
            .unwrap(),
        ),
        omitted: false,
        width: 6.,
        delayed: None,
    });
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    draw(&view, cx);
    for i in 0..200 {
        view.update(cx, |view, cx| {
            view.width = if i % 2 == 0 { 10. } else { 8. };
            view.owner.as_ref().unwrap().activity(cx);
            cx.notify();
        });
        draw(&view, cx);
    }
    let before = count(&view, cx);
    cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 1));
    assert_eq!(count(&view, cx), before + 1);
    draw(&view, cx);
    view.update(cx, |view, cx| {
        view.omitted = true;
        cx.notify();
    });
    draw(&view, cx);
    let before = count(&view, cx);
    cx.update(|window, cx| {
        assert_eq!(window.simulate_next_frame(cx), 1);
        assert_eq!(window.simulate_next_frame(cx), 0);
    });
    assert_eq!(count(&view, cx), before);
    view.read_with(cx, |view, _| {
        let state = view.owner.as_ref().unwrap().0.borrow();
        assert!(state.timer.is_none());
        assert!(!state.frame_needed);
        assert!(!state.model.accepts_pointer());
    });
    cx.update(|window, _| window.remove_window());
}

#[test]
fn stale_drivers_and_replaced_policies_do_not_deliver_old_wakes() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, cx| Fixture {
        owner: Some(Owner::new(Mode::Scrolling, motion(), cx).unwrap()),
        omitted: false,
        width: 6.,
        delayed: None,
    });
    active(&view, cx);
    let stale = view.read_with(cx, |view, _| view.owner.as_ref().unwrap().driver());
    view.update(cx, |view, _| {
        view.owner
            .as_ref()
            .unwrap()
            .set_policy(
                Mode::Scrolling,
                Motion {
                    idle_ms: 2000,
                    ..motion()
                },
            )
            .unwrap();
    });
    assert!(stale.current().is_none());
    draw(&view, cx);
    cx.run_until_parked();
    let before = count(&view, cx);
    cx.executor().advance_clock(Duration::from_millis(1000));
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before);
    view.update(cx, |view, _| view.owner.as_ref().unwrap().close());
    cx.executor().advance_clock(Duration::from_millis(2000));
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before);
    let weak = view.read_with(cx, |view, _| Rc::downgrade(&view.owner.as_ref().unwrap().0));
    cx.update(|window, _| window.remove_window());
    drop(view);
    // Entity drops are collected by an App update, not executor polling alone.
    cx.cx.update(|_| ());
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
}

#[test]
fn early_platform_deadline_rearms_only_the_remaining_interval() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, cx| Fixture {
        owner: Some(Owner::new(Mode::Scrolling, motion(), cx).unwrap()),
        omitted: false,
        width: 6.,
        delayed: None,
    });
    active(&view, cx);
    // Deliberately skew the model origin after scheduling to simulate an early
    // platform timer. Production uses one executor clock for both operations.
    view.update(cx, |view, _| {
        view.owner.as_ref().unwrap().0.borrow_mut().origin += Duration::from_millis(50);
    });
    let before = count(&view, cx);
    cx.executor().advance_clock(Duration::from_millis(1000));
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before);
    view.read_with(cx, |view, _| {
        assert!(view.owner.as_ref().unwrap().0.borrow().timer.is_some())
    });
    cx.executor().advance_clock(Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before + 1);
    view.read_with(cx, |view, _| {
        assert!(view.owner.as_ref().unwrap().0.borrow().timer.is_none())
    });
    cx.update(|window, _| window.remove_window());
}

#[test]
fn deadline_crossing_between_sample_and_paint_still_wakes_to_hide() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, cx| Fixture {
        owner: Some(Owner::new(Mode::Scrolling, motion(), cx).unwrap()),
        omitted: false,
        width: 6.,
        delayed: None,
    });
    view.update(cx, |view, cx| {
        let owner = view.owner.as_ref().unwrap();
        owner.set_eligible(true);
        owner.activity(cx);
        let driver = owner.driver();
        let frame = driver.sample(16., 6., cx).unwrap().unwrap();
        view.delayed = Some((driver, frame));
        cx.notify();
    });
    let before = count(&view, cx);
    cx.executor().advance_clock(Duration::from_millis(1500));
    draw(&view, cx);
    cx.run_until_parked();
    assert_eq!(count(&view, cx), before + 1);
    draw(&view, cx);
    view.read_with(cx, |view, _| {
        let state = view.owner.as_ref().unwrap().0.borrow();
        assert!(!state.model.accepts_pointer());
        assert!(!state.frame_needed);
        assert!(state.timer.is_none());
    });
    cx.update(|window, _| window.remove_window());
}
