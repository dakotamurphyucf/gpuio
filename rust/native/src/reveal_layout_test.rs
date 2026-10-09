use crate::{
    reveal_layout::{self, Measurement, Reveal},
    reveal_motion::{Presentation, State},
};
use gpui::{
    Context, IntoElement, Render, StyleRefinement, TestAppContext, Window, div, prelude::*, px,
};
use gpuio_protocol::animation::Spring;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn spring() -> Spring {
    Spring {
        stiffness: 400.,
        damping: 40.,
        mass: 1.,
        epsilon: 0.1,
        max_duration_ms: 2000,
    }
}
struct Fixture {
    state: Rc<RefCell<State>>,
    measurement: Rc<RefCell<Option<Measurement>>>,
    now: u64,
    width: f32,
    text: String,
    body_style: StyleRefinement,
    parent_style: StyleRefinement,
    needs_frame: Rc<Cell<bool>>,
}
impl Fixture {
    fn new(expanded: bool) -> Self {
        Self {
            state: Rc::new(RefCell::new(State::new(expanded, spring()).unwrap())),
            measurement: Rc::new(RefCell::new(None)),
            now: 0,
            width: 320.,
            text: "A long title with several words that must wrap inside the constrained padded panel.".into(),
            body_style: StyleRefinement::default(),
            parent_style: StyleRefinement::default(),
            needs_frame: Rc::new(Cell::new(false)),
        }
    }
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut body = div()
            .id("body")
            .debug_selector(|| "reveal-body".into())
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(240.))
            .p(px(12.))
            .border_2()
            .ml(px(11.))
            .mt(px(7.))
            .mb(px(5.))
            .text_size(px(16.))
            .child(self.text.clone());
        body.style().refine(&self.body_style);
        let mut container = div().flex().flex_col().w(px(self.width)).gap(px(6.));
        container.style().refine(&self.parent_style);
        let immediate = !reveal_layout::can_animate(body.style(), container.style());
        let frame = self
            .state
            .borrow()
            .frame(Duration::from_millis(self.now), immediate);
        let outer = reveal_layout::split_style(body.style(), frame.presentation);
        let state = Rc::downgrade(&self.state);
        let measurement = self.measurement.clone();
        let needs_frame = self.needs_frame.clone();
        container
            .child(div().h(px(20.)).debug_selector(|| "before".into()))
            .child(Reveal::new(
                "reveal",
                body.into_any_element(),
                outer,
                frame,
                move |frame, measured, _, _| {
                    *measurement.borrow_mut() = Some(measured);
                    if let Some(state) = state.upgrade() {
                        let next = if let Some(natural) = measured.natural {
                            state.borrow_mut().painted(
                                frame,
                                f64::from(natural.height),
                                f64::from(measured.bounds.size.height),
                            )
                        } else {
                            state.borrow_mut().painted_closed(frame)
                        };
                        needs_frame.set(next);
                    }
                },
            ))
            .child(div().h(px(20.)).debug_selector(|| "after".into()))
    }
}
#[test]
fn padded_wrapping_content_keeps_natural_measurement_and_exact_sibling_offsets() {
    let mut app = TestAppContext::single();
    let state = Rc::new(RefCell::new(State::new(true, spring()).unwrap()));
    let measurement = Rc::new(RefCell::new(None));
    let (owner, cx) = app.add_window_view(|_, _| Fixture {
        state: state.clone(),
        measurement: measurement.clone(),
        now: 0,
        width: 320.,
        text: "A long title with several words that must wrap inside the constrained padded panel."
            .into(),
        body_style: StyleRefinement::default(),
        parent_style: StyleRefinement::default(),
        needs_frame: Rc::new(Cell::new(false)),
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let natural = measurement.borrow().unwrap();
    let body = cx.debug_bounds("reveal-body").unwrap();
    let footer = cx.debug_bounds("after").unwrap();
    assert_eq!(body.size.width, px(240.));
    assert_eq!(body.origin.x, px(11.));
    assert_eq!(footer.origin.y, body.bottom() + px(11.));
    state
        .borrow_mut()
        .update(false, spring(), Duration::ZERO)
        .unwrap();
    owner.update(cx, |fixture, cx| {
        fixture.now = 40;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let partial = measurement.borrow().unwrap();
    assert!(
        partial.bounds.size.height > px(0.)
            && partial.bounds.size.height < natural.bounds.size.height
    );
    assert_eq!(
        partial.natural, natural.natural,
        "clipping must not rewrap or shrink padding"
    );
    let body = cx.debug_bounds("reveal-body").unwrap();
    assert_eq!(body.origin.x, px(11.));
    assert_eq!(body.size.width, px(240.));
    assert_eq!(
        cx.debug_bounds("after").unwrap().origin.y,
        partial.bounds.bottom() + px(11.)
    );
    state
        .borrow_mut()
        .update(true, spring(), Duration::from_millis(40))
        .unwrap();
    owner.update(cx, |fixture, cx| {
        fixture.now = 2040;
        fixture.width = 180.;
        fixture
            .text
            .push_str(" More streamed content arrives before this very frame.");
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let resized = measurement.borrow().unwrap();
    assert_eq!(
        state
            .borrow()
            .frame(Duration::from_millis(2040), false)
            .presentation,
        Presentation::Natural
    );
    assert!(resized.natural.unwrap().height > natural.natural.unwrap().height);
    assert_eq!(resized.bounds.size.height, resized.natural.unwrap().height);
    assert_eq!(
        cx.debug_bounds("after").unwrap().origin.y,
        resized.bounds.bottom() + px(11.)
    );
    owner.update(cx, |fixture, cx| {
        fixture.now = 2056;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        measurement.borrow().unwrap().bounds,
        resized.bounds,
        "settled resize has no correction-frame jump"
    );
}

#[test]
fn zero_clip_measures_before_opening_and_completion_removes_the_closed_box() {
    let mut app = TestAppContext::single();
    let fixture = Fixture::new(false);
    let state = fixture.state.clone();
    let measurement = fixture.measurement.clone();
    let needs_frame = fixture.needs_frame.clone();
    let (owner, cx) = app.add_window_view(|_, _| fixture);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(measurement.borrow().unwrap().natural.is_none());
    assert!(!needs_frame.get());
    let closed_footer = cx.debug_bounds("after").unwrap();
    state
        .borrow_mut()
        .update(true, spring(), Duration::ZERO)
        .unwrap();
    owner.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let initial = measurement.borrow().unwrap();
    assert_eq!(initial.bounds.size.height, px(0.));
    assert!(initial.natural.unwrap().height > px(0.));
    assert!(
        needs_frame.get(),
        "zero clip must still start a measured opening"
    );
    for (expanded, start) in [(true, 0), (false, 2000)] {
        state
            .borrow_mut()
            .update(expanded, spring(), Duration::from_millis(start))
            .unwrap();
        let mut settled = false;
        for tick in 1..125 {
            owner.update(cx, |fixture, cx| {
                fixture.now = start + tick * 16;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            if !needs_frame.get() {
                settled = true;
                break;
            }
        }
        assert!(settled, "no idle repaint loop after completing a reveal");
        let measured = measurement.borrow().unwrap();
        if expanded {
            assert_eq!(measured.bounds.size, measured.natural.unwrap());
            assert_eq!(
                cx.debug_bounds("after").unwrap().origin.y,
                measured.bounds.bottom() + px(11.)
            );
        } else {
            assert!(measured.natural.is_none());
            assert_eq!(
                cx.debug_bounds("after").unwrap(),
                closed_footer,
                "closed padding, border, margins and flex gap must not survive"
            );
            assert!(cx.debug_bounds("reveal-body").is_none());
        }
    }
}

#[test]
fn absolute_height_bounds_remain_on_the_natural_body_during_closing() {
    for case in 0..3 {
        let mut app = TestAppContext::single();
        let mut fixture = Fixture::new(true);
        match case {
            0 => fixture.body_style.size.height = Some(px(180.).into()),
            1 => fixture.body_style.min_size.height = Some(px(180.).into()),
            2 => fixture.body_style.max_size.height = Some(px(48.).into()),
            _ => unreachable!(),
        }
        let state = fixture.state.clone();
        let measurement = fixture.measurement.clone();
        let (owner, cx) = app.add_window_view(|_, _| fixture);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let natural = measurement.borrow().unwrap();
        assert_eq!(
            natural.bounds.size.height,
            px(if case == 2 { 48. } else { 180. })
        );
        state
            .borrow_mut()
            .update(false, spring(), Duration::ZERO)
            .unwrap();
        owner.update(cx, |fixture, cx| {
            fixture.now = 40;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let partial = measurement.borrow().unwrap();
        assert_eq!(partial.natural, natural.natural);
        assert!(partial.bounds.size.height < natural.bounds.size.height);
        assert!(partial.bounds.size.height > px(0.));
    }
}

#[test]
fn parent_dependent_height_uses_immediate_layout_without_changing_reopened_geometry() {
    for case in 0..4 {
        let mut app = TestAppContext::single();
        let mut fixture = Fixture::new(true);
        fixture.parent_style.size.height = Some(px(360.).into());
        match case {
            0 => fixture.body_style.flex_grow = Some(1.),
            1 => fixture.body_style.size.height = Some(gpui::relative(0.5).into()),
            2 => fixture.parent_style.flex_direction = Some(gpui::FlexDirection::Row),
            3 => fixture.body_style.min_size.height = Some(px(0.).into()),
            _ => unreachable!(),
        }
        let state = fixture.state.clone();
        let measurement = fixture.measurement.clone();
        let needs_frame = fixture.needs_frame.clone();
        let (owner, cx) = app.add_window_view(|_, _| fixture);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let natural = measurement.borrow().unwrap();
        for (expanded, now) in [(false, 40), (true, 80)] {
            state
                .borrow_mut()
                .update(expanded, spring(), Duration::from_millis(now))
                .unwrap();
            owner.update(cx, |fixture, cx| {
                fixture.now = now;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let measured = measurement.borrow().unwrap();
            assert!(!needs_frame.get());
            assert!(!state.borrow().is_animating());
            if expanded {
                assert_eq!(measured.bounds, natural.bounds);
            } else {
                assert!(measured.natural.is_none());
            }
        }
    }
}

#[test]
fn changing_to_a_parent_sized_panel_during_motion_settles_in_that_frame() {
    let mut app = TestAppContext::single();
    let fixture = Fixture::new(true);
    let state = fixture.state.clone();
    let measurement = fixture.measurement.clone();
    let needs_frame = fixture.needs_frame.clone();
    let (owner, cx) = app.add_window_view(|_, _| fixture);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    state
        .borrow_mut()
        .update(false, spring(), Duration::ZERO)
        .unwrap();
    owner.update(cx, |fixture, cx| {
        fixture.now = 40;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(needs_frame.get());
    state
        .borrow_mut()
        .update(true, spring(), Duration::from_millis(40))
        .unwrap();
    owner.update(cx, |fixture, cx| {
        fixture.now = 56;
        fixture.body_style.flex_grow = Some(1.);
        fixture.parent_style.size.height = Some(px(360.).into());
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let settled = measurement.borrow().unwrap();
    assert_eq!(settled.bounds.size, settled.natural.unwrap());
    assert!(!needs_frame.get());
    assert!(!state.borrow().is_animating());
    owner.update(cx, |fixture, cx| {
        fixture.now = 72;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(measurement.borrow().unwrap().bounds, settled.bounds);
}
