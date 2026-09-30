//! Actual GPU pixels in a background window. Fixed phases isolate glyph paint
//! from the retained clock/bridge, which require separate mounted acceptance.
use super::*;
use gpui::{
    Context, HighlightStyle, Render, SharedString, TextLayout, TextOverflow, WindowBounds,
    WindowHandle, WindowOptions, div, prelude::*,
};
use gpuio_protocol::text_shimmer::Repeat;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

struct Scene {
    source: Arc<str>,
    width: f32,
    clip_height: f32,
    align: TextAlign,
    overflow: Option<TextOverflow>,
    selected: bool,
    enabled: bool,
    config: Config,
    sample: Sample,
    appearance: Appearance,
    layout: Option<TextLayout>,
    report: Rc<Cell<Report>>,
}

impl Render for Scene {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut text = StyledText::new(SharedString::from(self.source.clone()));
        if self.selected {
            text = text.with_highlights([(
                0..self.source.len(),
                HighlightStyle {
                    background_color: Some(gpui::blue()),
                    ..Default::default()
                },
            )]);
        }
        self.layout = Some(text.layout().clone());
        self.report.set(Report::Inactive);
        let content = if self.enabled {
            let mut text = element(text, self.config, self.sample, self.appearance);
            text.probe = Some(self.report.clone());
            text.into_any_element()
        } else {
            text.into_any_element()
        };
        let mut body = div()
            .absolute()
            .left(px(20.))
            .top(px(20.))
            .w(px(self.width))
            .h(px(self.clip_height))
            .overflow_hidden()
            .text_size(px(24.))
            .line_height(px(32.))
            .text_color(if self.appearance.dark {
                rgba(0x808080ff).into()
            } else {
                gpui::black()
            });
        body.style().text.text_align = Some(self.align);
        if let Some(overflow) = &self.overflow {
            body.style().text.text_overflow = Some(overflow.clone());
            body = body.whitespace_nowrap();
        }
        div()
            .size_full()
            .bg(self.appearance.background)
            .child(body.child(content))
    }
}

struct Capture {
    image: image::RgbaImage,
    scale: f32,
    report: Report,
    bounds: Bounds<Pixels>,
    positions: Vec<gpui::Point<Pixels>>,
    displayed: String,
    wrapped_rows: usize,
}

fn capture(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>, enabled: bool) -> Capture {
    handle
        .update(cx, |scene, window, cx| {
            scene.enabled = enabled;
            cx.notify();
            window.refresh();
        })
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    handle
        .update(cx, |scene, window, _| {
            let layout = scene.layout.as_ref().unwrap();
            let displayed = layout.text();
            Capture {
                image: window.render_to_image().expect("native GPU readback"),
                scale: window.scale_factor(),
                report: scene.report.get(),
                bounds: layout.bounds(),
                positions: displayed
                    .char_indices()
                    .filter_map(|(byte, _)| layout.position_for_index(byte))
                    .collect(),
                displayed,
                wrapped_rows: layout
                    .line_layouts()
                    .iter()
                    .map(|line| line.wrap_boundaries.len() + 1)
                    .sum(),
            }
        })
        .unwrap()
}

fn config() -> Config {
    Config {
        duration_ms: 2000,
        spread: Spread::Pixels(1_000_000.),
        direction: Direction::LeftToRight,
        repeat: Repeat::Loop,
        animated: true,
        highlight: Some(0xff0000ff),
        appearance: None,
    }
}

fn geometry_equal(before: &Capture, after: &Capture) {
    assert_eq!(before.bounds, after.bounds);
    assert_eq!(
        before.positions, after.positions,
        "effect must not change shaping/wrapping"
    );
    assert_eq!(before.displayed, after.displayed);
}

fn red(pixel: [u8; 4]) -> bool {
    let [r, g, b, _] = pixel;
    r > 55 && r > g.saturating_add(30) && r > b.saturating_add(30)
}

fn glyph_pixels(before: &Capture, after: &Capture, name: &str) {
    geometry_equal(before, after);
    let mut red_pixels = 0;
    let mut ink = 0;
    let (width, height) = before.image.dimensions();
    for y in 0..height {
        for x in 0..width {
            let old = before.image.get_pixel(x, y).0;
            let next = after.image.get_pixel(x, y).0;
            if old[0] < 30 && old[1] < 30 && old[2] < 30 {
                ink += 1;
            }
            if red(next) && !red(old) {
                red_pixels += 1;
                // A one-device-pixel halo accommodates color-dependent font
                // raster dilation; misplaced runs/rows cannot pass this oracle.
                let near_ink = (y.saturating_sub(1)..=(y + 1).min(height - 1)).any(|yy| {
                    (x.saturating_sub(1)..=(x + 1).min(width - 1)).any(|xx| {
                        let p = before.image.get_pixel(xx, yy).0;
                        p[0] < 230 && p[1] < 230 && p[2] < 230
                    })
                });
                assert!(
                    near_ink,
                    "{name}: highlight outside original glyph at {x},{y}: {old:?}->{next:?}"
                );
            }
        }
    }
    assert!(ink > 20, "{name}: fixture has dark text");
    assert!(
        red_pixels > ink / 4,
        "{name}: all-band highlight must reach visible glyphs ({red_pixels}/{ink})"
    );
    let Report::Painted {
        eligible_glyphs,
        calls,
    } = after.report
    else {
        panic!("{name}: {:?}", after.report);
    };
    assert!(calls > 0 && calls <= eligible_glyphs * LAYERS);
    eprintln!(
        "SHIMMER_GLYPHS {name}: ink={ink} red={red_pixels} glyphs={eligible_glyphs} calls={calls}"
    );
}

fn update(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>, f: impl FnOnce(&mut Scene)) {
    handle.update(cx, |s, _, _| f(s)).unwrap();
}

fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>) {
    let cases = [
        ("Latin", "Alpha café e\u{301} words wrap to another line"),
        ("Hebrew", "אבג דהו זחט יכל מנס עף"),
        ("Arabic", "مرحبا بالعالم مرحبا بالعالم"),
        ("Mixed", "A אבג Z مرحبا B"),
    ];
    for (name, source) in cases {
        for width in [280., 90.] {
            for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
                update(cx, handle, |s| {
                    s.source = source.into();
                    s.width = width;
                    s.align = align;
                });
                let before = capture(cx, handle, false);
                if width == 90. {
                    assert!(before.wrapped_rows > 1, "{name}: narrow fixture must wrap");
                }
                let after = capture(cx, handle, true);
                glyph_pixels(&before, &after, &format!("{name}/{width}/{align:?}"));
            }
        }
    }
    for overflow in [
        TextOverflow::Truncate("…".into()),
        TextOverflow::TruncateStart("…".into()),
    ] {
        update(cx, handle, |s| {
            s.source = "prefix prefix café tail".into();
            s.width = 140.;
            s.align = TextAlign::Left;
            s.overflow = Some(overflow.clone());
        });
        let before = capture(cx, handle, false);
        assert!(before.displayed.contains('…'), "fixture must truncate");
        let after = capture(cx, handle, true);
        glyph_pixels(&before, &after, &format!("{overflow:?}"));
    }
    update(cx, handle, |s| {
        s.source = "AAAA BBBB CCCC DDDD".into();
        s.width = 280.;
        s.clip_height = 18.;
        s.overflow = None;
    });
    let clipped = capture(cx, handle, true);
    let first_outside = ((20. + 18.) * clipped.scale).ceil() as u32;
    for y in first_outside..clipped.image.height() {
        for x in 0..clipped.image.width() {
            assert_eq!(
                clipped.image.get_pixel(x, y).0,
                [255, 255, 255, 255],
                "ancestor clips overlay"
            );
        }
    }
    update(cx, handle, |s| {
        s.source = "Thinking about café".into();
        s.clip_height = 220.;
        s.config.spread = Spread::Relative(0.3);
    });
    let baseline = capture(cx, handle, false);
    for phase in [0., 1.] {
        update(cx, handle, |s| s.sample.phase = phase);
        let edge = capture(cx, handle, true);
        assert_eq!(edge.report, Report::OutsideBand);
        assert_eq!(
            edge.image, baseline.image,
            "sweep endpoints have no overlay"
        );
    }
    update(cx, handle, |s| s.sample.phase = 0.35);
    let left = capture(cx, handle, true);
    update(cx, handle, |s| {
        s.sample.phase = 0.65;
        s.config.direction = Direction::RightToLeft;
    });
    let right = capture(cx, handle, true);
    // Fractional .35/.65 are not bitwise complements; pixel tolerance covers
    // subpixel mask quantization only, not a different physical sweep direction.
    let max_difference = left
        .image
        .as_raw()
        .iter()
        .zip(right.image.as_raw())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(
        max_difference <= 2,
        "reversed sweep uses mirrored phase: {max_difference}"
    );
    assert_ne!(
        left.image, baseline.image,
        "mid-sweep must visibly alter text"
    );
    for mode in 0..4 {
        update(cx, handle, |s| {
            s.config = config();
            s.sample.phase = 0.5;
            s.sample.reduced_motion = false;
            match mode {
                0 => s.config.animated = false,
                1 => s.sample.reduced_motion = true,
                2 => s.config.highlight = Some(0xff000000),
                3 => s.sample.phase = f32::NAN,
                _ => unreachable!(),
            }
        });
        let static_text = capture(cx, handle, true);
        assert_eq!(static_text.report, Report::Inactive);
        assert_eq!(
            static_text.image, baseline.image,
            "static/Reduce/transparent/invalid phase preserves text"
        );
    }
    update(cx, handle, |s| {
        s.config = config();
        s.sample = Sample {
            phase: 0.5,
            reduced_motion: false,
        };
        s.source = "👨‍👩‍👧‍👦👩‍💻".into();
    });
    let emoji = capture(cx, handle, false);
    let effect = capture(cx, handle, true);
    assert_eq!(
        effect.report,
        Report::Inactive,
        "color emoji are not monochrome glyphs"
    );
    assert_eq!(effect.image, emoji.image, "emoji pixels stay unchanged");
    update(cx, handle, |s| s.source = "".into());
    let empty = capture(cx, handle, true);
    assert_eq!(empty.report, Report::Inactive);
    for source in [
        "a".repeat(MAX_TEXT_BYTES + 1),
        "I".repeat(MAX_GLYPHS + 1),
        "a\n".repeat(MAX_LINES + 1),
    ] {
        update(cx, handle, |s| s.source = source.into());
        let plain = capture(cx, handle, false);
        let capped = capture(cx, handle, true);
        assert_eq!(capped.report, Report::Capacity);
        assert_eq!(
            plain.image, capped.image,
            "capacity falls back to whole ordinary text"
        );
    }
    update(cx, handle, |s| {
        s.source = "Selected café text".into();
        s.selected = true;
    });
    let plain = capture(cx, handle, false);
    let selected = capture(cx, handle, true);
    geometry_equal(&plain, &selected);
    let mut blue = 0;
    for (a, b) in plain.image.pixels().zip(selected.image.pixels()) {
        if a.0 == [0, 0, 255, 255] {
            assert_eq!(a, b, "glyph overlay preserves selection background");
            blue += 1;
        }
    }
    assert!(blue > 100);
    for dark in [false, true] {
        update(cx, handle, |s| {
            s.selected = false;
            s.config.highlight = None;
            s.appearance.dark = dark;
            s.appearance.background = if dark { gpui::black() } else { gpui::white() };
            s.appearance.foreground = if dark { gpui::white() } else { gpui::black() };
        });
        let plain = capture(cx, handle, false);
        let lit = capture(cx, handle, true);
        geometry_equal(&plain, &lit);
        let brighter = plain
            .image
            .pixels()
            .zip(lit.image.pixels())
            .filter(|(a, b)| b.0[0] > a.0[0].saturating_add(20))
            .count();
        assert!(
            brighter > 40,
            "theme-aware default must visibly brighten glyphs ({dark}): {brighter}"
        );
    }
    eprintln!(
        "GPUIO_NATIVE_TEXT_SHIMMER_PAINT_OK: 24 shape/RTL/wrap/alignment cases, both ellipses, clipping, reversed band, static/Reduce/transparent states, emoji, bounded work, selection background and both default themes"
    );
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(340.), px(270.)),
                        cx,
                    ))),
                    focus: false,
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| Scene {
                        source: "Text".into(),
                        width: 280.,
                        clip_height: 220.,
                        align: TextAlign::Left,
                        overflow: None,
                        selected: false,
                        enabled: true,
                        config: config(),
                        sample: Sample {
                            phase: 0.5,
                            reduced_motion: false,
                        },
                        appearance: Appearance {
                            foreground: gpui::black(),
                            background: gpui::white(),
                            dark: false,
                        },
                        layout: None,
                        report: Default::default(),
                    })
                },
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(async { exercise(cx, handle) }).await;
            *task_failure.borrow_mut() = result.err();
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
