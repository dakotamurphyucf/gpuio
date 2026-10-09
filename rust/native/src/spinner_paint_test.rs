//! TestPlatform scene/cache/lifecycle evidence, not native GPU pixels or AX.
use super::*;
use crate::spinner_clock::{Clock, Owner};
use gpui::{Context, IntoElement, Render, TestAppContext, canvas, div, prelude::*, rgba};
use gpuio_protocol::{animation::Easing, spinner::Config};
use std::{cell::Cell, rc::Rc, time::Duration};

struct Fixture {
    owner: Option<Owner>,
    image: Option<Arc<RenderImage>>,
    report: Rc<Cell<Report>>,
    inert: bool,
    opacity: f32,
    color: gpui::Hsla,
    clipped: bool,
    hidden: bool,
    bounds_override: Option<Bounds<Pixels>>,
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let driver = self.owner.as_ref().map(|owner| {
            owner.prepare_frame();
            owner.driver()
        });
        let report = self.report.clone();
        let image = self.image.clone();
        let inert = self.inert;
        let clipped = self.clipped;
        let bounds_override = self.bounds_override;
        let mut element = div()
            .w(px(40.))
            .h(px(40.))
            .text_color(self.color)
            .opacity(self.opacity);
        if !self.hidden {
            element = element.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let bounds = bounds_override.unwrap_or(bounds);
                        let Some(driver) = driver else {
                            report.set(Report::Skipped);
                            return;
                        };
                        let mask = clipped.then_some(gpui::ContentMask {
                            bounds: Bounds::new(
                                point(px(1000.), px(1000.)),
                                size(px(10.), px(10.)),
                            ),
                        });
                        window.with_content_mask(mask, |window| {
                            report.set(paint(&driver, bounds, image.as_ref(), inert, window, cx));
                        });
                    },
                )
                .size_full(),
            );
        }
        element
    }
}
fn pixels() -> Arc<RenderImage> {
    Arc::new(RenderImage::new(vec![image::Frame::new(
        image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 255, 0, 128])),
    )]))
}
fn config() -> Arc<Config> {
    Arc::new(Config {
        label: "Spin".into(),
        animated: true,
        period_ms: 1000,
        easing: Easing::Linear,
        source: None,
    })
}

#[test]
fn shader_rotation_reuses_mask_and_inherited_color_opacity_with_static_fallback() {
    let mut app = TestAppContext::single();
    let clock = Rc::new(Clock::default());
    clock.set_test_time(Duration::ZERO);
    let image = pixels();
    let report = Rc::new(Cell::new(Report::Skipped));
    let (view, cx) = app.add_window_view(|_, _| Fixture {
        owner: Some(Owner::new(config(), clock.clone()).unwrap()),
        image: Some(image.clone()),
        report: report.clone(),
        inert: false,
        opacity: 0.5,
        color: rgba(0x44aa88cc).into(),
        clipped: false,
        hidden: false,
        bounds_override: None,
    });
    let tile = cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let painted = window.painted_monochrome_sprites();
        assert_eq!(painted.len(), 1);
        assert!((painted[0].color.a - 0.4).abs() < 0.0001);
        assert!(!window.has_image_atlas_entry(&image));
        painted[0].tile.tile_id
    });
    assert_eq!(report.get(), Report::Icon { turns: 0. });
    clock.set_test_time(Duration::from_millis(250));
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let painted = window.painted_monochrome_sprites();
        assert_eq!(painted[0].tile.tile_id, tile);
        let expected = TransformationMatrix::unit()
            .translate(point(px(20.), px(20.)).scale(window.scale_factor()))
            .rotate(radians(std::f32::consts::FRAC_PI_2))
            .translate(point(px(-20.), px(-20.)).scale(window.scale_factor()));
        assert_eq!(painted[0].transformation, expected);
    });
    assert_eq!(report.get(), Report::Icon { turns: 0.25 });
    view.update(cx, |view, cx| {
        view.inert = true;
        cx.notify();
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert_eq!(report.get(), Report::Icon { turns: 0. });
    for _ in 0..2 {
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
        });
    }
    cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
    view.update(cx, |view, cx| {
        view.image = None;
        cx.notify();
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        report.get(),
        Report::Fallback {
            turns: 0.,
            mask_failed: false
        }
    );
    view.update(cx, |view, cx| {
        view.image = Some(Arc::new(RenderImage::new(vec![])));
        cx.notify();
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        report.get(),
        Report::Fallback {
            turns: 0.,
            mask_failed: true
        }
    );
}

#[test]
fn clipped_transparent_hidden_and_removed_owners_stop_requesting_frames() {
    for case in 0..5 {
        let mut app = TestAppContext::single();
        let clock = Rc::new(Clock::default());
        clock.set_test_time(Duration::ZERO);
        let configuration = config();
        let weak_config = Arc::downgrade(&configuration);
        let image = pixels();
        let report = Rc::new(Cell::new(Report::Skipped));
        let (view, cx) = app.add_window_view(|_, _| Fixture {
            owner: Some(Owner::new(configuration, clock.clone()).unwrap()),
            image: Some(image.clone()),
            report: report.clone(),
            inert: false,
            opacity: 1.,
            color: rgba(0xff0000ff).into(),
            clipped: false,
            hidden: false,
            bounds_override: None,
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(matches!(report.get(), Report::Icon { .. }));
        view.update(cx, |view, cx| {
            match case {
                0 => view.clipped = true,
                1 => view.opacity = 0.,
                2 => view.color.a = 0.,
                3 => view.hidden = true,
                4 => {
                    view.owner = None;
                }
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
        assert_eq!(report.get(), Report::Skipped, "case={case}");
        cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0, "case={case}"));
        if case == 4 {
            assert!(weak_config.upgrade().is_none());
        }
        cx.update(|window, cx| {
            view.update(cx, |view, _| {
                view.owner = None;
                view.image = None;
            });
            window.remove_window();
        });
        cx.run_until_parked();
        assert!(weak_config.upgrade().is_none());
    }
}

#[test]
fn empty_and_malformed_bounds_do_not_upload_or_schedule() {
    let mut app = TestAppContext::single();
    let clock = Rc::new(Clock::default());
    let image = pixels();
    let report = Rc::new(Cell::new(Report::Skipped));
    let empty = Bounds::new(point(px(0.), px(0.)), size(px(0.), px(20.)));
    let (view, cx) = app.add_window_view(|_, _| Fixture {
        owner: Some(Owner::new(config(), clock).unwrap()),
        image: Some(image.clone()),
        report: report.clone(),
        inert: false,
        opacity: 1.,
        color: rgba(0xff0000ff).into(),
        clipped: false,
        hidden: false,
        bounds_override: Some(empty),
    });
    for bounds in [
        empty,
        Bounds::new(point(px(0.), px(0.)), size(px(20.), px(-1.))),
        Bounds::new(point(px(f32::NAN), px(0.)), size(px(20.), px(20.))),
        Bounds::new(point(px(0.), px(0.)), size(px(f32::INFINITY), px(20.))),
        Bounds::new(point(px(f32::MAX), px(0.)), size(px(f32::MAX), px(20.))),
    ] {
        view.update(cx, |view, cx| {
            view.bounds_override = Some(bounds);
            cx.notify();
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(report.get(), Report::Skipped);
            assert!(!window.has_image_mask_atlas_entry(&image, 0));
            assert_eq!(window.simulate_next_frame(cx), 0);
        });
    }
}
