//! Actual TestPlatform paint/queue checks; no OS window or GPU-pixel claim.
use super::*;
use crate::progress_clock::{Clock, Owner};
use gpui::{
    Context, IntoElement, Render, TestAppContext, canvas, div, point, prelude::*, rgba, size,
};
use gpuio_protocol::{
    progress::ProgressConfig,
    progress_presentation::{Config, Transition},
};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

struct Fixture {
    owner: Option<Owner>,
    report: Rc<Cell<Report>>,
    inert: bool,
    opacity: f32,
    color: gpui::Hsla,
    hidden: bool,
    clipped: bool,
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let driver = self.owner.as_ref().map(Owner::driver);
        let report = self.report.clone();
        let (inert, clipped) = (self.inert, self.clipped);
        let element = div()
            .w(px(80.))
            .h(px(40.))
            .text_color(self.color)
            .opacity(self.opacity);
        if self.hidden {
            return element;
        }
        element.child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    let Some(driver) = driver else {
                        report.set(Report::Skipped);
                        return;
                    };
                    let mask = clipped.then_some(gpui::ContentMask {
                        bounds: Bounds::new(point(px(1000.), px(1000.)), size(px(10.), px(10.))),
                    });
                    window.with_content_mask(mask, |window| {
                        report.set(paint(
                            &driver,
                            bounds,
                            Corners::all(px(10.)),
                            inert,
                            window,
                            cx,
                        ));
                    });
                },
            )
            .size_full(),
        )
    }
}
fn config(shape: Shape, fraction: Option<f64>) -> Arc<Config> {
    Arc::new(Config {
        progress: ProgressConfig {
            label: "Work".into(),
            fraction,
        },
        shape,
        transition: Transition::Tween {
            duration_ms: 200,
            easing: Easing::Linear,
        },
    })
}
fn fixture(configuration: Arc<Config>, clock: Rc<Clock>, report: Rc<Cell<Report>>) -> Fixture {
    Fixture {
        owner: Some(Owner::new(configuration, clock).unwrap()),
        report,
        inert: false,
        opacity: 1.,
        color: rgba(0x4488aaff).into(),
        hidden: false,
        clipped: false,
    }
}
#[test]
fn zero_fill_transition_keeps_waking_until_exact_target_then_idles() {
    let mut app = TestAppContext::single();
    let clock = Rc::new(Clock::default());
    clock.set_test_time(Duration::ZERO);
    let report = Rc::new(Cell::new(Report::Skipped));
    let (view, cx) = app.add_window_view(|_, _| {
        fixture(
            config(Shape::Linear, Some(0.)),
            clock.clone(),
            report.clone(),
        )
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert!(matches!(report.get(), Report::Visible { paths: 0, .. }));
    view.update(cx, |view, cx| {
        view.owner.as_ref().unwrap().finish_frame();
        view.owner
            .as_ref()
            .unwrap()
            .update(config(Shape::Linear, Some(1.)))
            .unwrap();
        cx.notify();
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert!(matches!(report.get(), Report::Visible { paths: 0, .. }));
    for (millis, expected) in [(100, 0.5), (200, 1.)] {
        clock.set_test_time(Duration::from_millis(millis));
        cx.update(|window, cx| {
            assert_eq!(window.simulate_next_frame(cx), 1);
            window.draw(cx).clear(cx);
            view.update(cx, |view, _| view.owner.as_ref().unwrap().finish_frame());
        });
        assert!(
            matches!(report.get(),Report::Visible {sample:Sample {value:Value::Determinate(value),..},paths:1,..} if value==expected),
            "{:?}",
            report.get()
        );
    }
    cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
}
#[test]
fn circular_track_and_arc_paint_and_suspended_owners_stop_wakes() {
    for case in 0..6 {
        let mut app = TestAppContext::single();
        let clock = Rc::new(Clock::default());
        clock.set_test_time(Duration::ZERO);
        let configuration = config(Shape::Circle, None);
        let weak_config = Arc::downgrade(&configuration);
        let report = Rc::new(Cell::new(Report::Skipped));
        let (view, cx) =
            app.add_window_view(|_, _| fixture(configuration, clock.clone(), report.clone()));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(matches!(report.get(), Report::Visible { paths: 1, .. }));
        clock.set_test_time(Duration::from_millis(250));
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            view.update(cx, |view, _| view.owner.as_ref().unwrap().finish_frame());
        });
        assert!(matches!(report.get(), Report::Visible { paths: 2, .. }));
        view.update(cx, |view, cx| {
            match case {
                0 => view.clipped = true,
                1 => view.opacity = 0.,
                2 => view.color.a = 0.,
                3 => view.hidden = true,
                4 => view.owner = None,
                5 => view.inert = true,
                _ => unreachable!(),
            }
            cx.notify();
        });
        report.set(Report::Skipped);
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                view.update(cx, |view, _| {
                    if let Some(owner) = &view.owner {
                        owner.finish_frame();
                    }
                });
                window.simulate_next_frame(cx);
            });
        }
        if case == 5 {
            assert!(matches!(
                report.get(),
                Report::Visible {
                    sample: Sample {
                        value: Value::Indeterminate {
                            static_presentation: true,
                            ..
                        },
                        ..
                    },
                    paths: 2,
                    ..
                }
            ));
        } else {
            assert_eq!(report.get(), Report::Skipped, "case={case}");
        }
        cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0, "case={case}"));
        view.update(cx, |view, _| view.owner = None);
        assert!(weak_config.upgrade().is_none());
        cx.update(|window, _| window.remove_window());
        cx.run_until_parked();
    }
}
