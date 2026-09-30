//! Paint-driven scheduling in a real background GPUI window, with deterministic
//! native time and explicit frame delivery. No desktop input or OCaml timers.
use super::*;
use gpui::{
    Bounds, Context, IntoElement, Render, SharedString, WindowBounds, WindowHandle, WindowOptions,
    div, point, prelude::*, px, rgba, size,
};
use gpuio_protocol::text_shimmer::{Direction, Spread};

struct Scene {
    owners: Vec<Owner>,
    clipped: bool,
    omitted: bool,
    color: u32,
}

impl Render for Scene {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut body = div()
            .absolute()
            .left(px(20.))
            .top(px(if self.clipped { 1000. } else { 20. }))
            .w(px(300.))
            .h(px(160.))
            .text_size(px(24.))
            .line_height(px(32.))
            .text_color(rgba(self.color));
        for owner in &self.owners {
            if self.omitted {
                owner.suspend();
                continue;
            }
            let source = owner.0.borrow().source.clone();
            body = body.child(owner.element(
                StyledText::new(SharedString::from(source)),
                Appearance {
                    foreground: gpui::black(),
                    background: gpui::white(),
                    dark: false,
                },
            ));
        }
        div()
            .size_full()
            .overflow_hidden()
            .bg(gpui::white())
            .child(body)
    }
}

fn config(repeat: Repeat) -> Config {
    Config {
        duration_ms: 1000,
        spread: Spread::Relative(0.3),
        direction: Direction::LeftToRight,
        repeat,
        animated: true,
        highlight: Some(0xff0000ff),
    }
}
fn at(clock: &Clock, ms: u64) {
    clock.test_now.set(Some(Duration::from_millis(ms)));
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
fn delivery(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>) -> usize {
    cx.update_window(handle.into(), |_, window, cx| {
        window.simulate_next_frame(cx)
    })
    .unwrap()
}
fn phase(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>) -> f32 {
    handle
        .update(cx, |s, _, _| {
            let state = s.owners[0].0.borrow();
            state.elapsed.as_secs_f32() / state.duration().as_secs_f32()
        })
        .unwrap()
}
fn notifications(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>) -> usize {
    handle
        .update(cx, |s, _, _| {
            s.owners.iter().map(|o| o.0.borrow().notifications).sum()
        })
        .unwrap()
}
fn pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>) -> image::RgbaImage {
    handle
        .update(cx, |_, window, _| window.render_to_image().unwrap())
        .unwrap()
}
fn change(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>, f: impl FnOnce(&mut Scene)) {
    handle.update(cx, |s, _, _| f(s)).unwrap();
}
fn replace(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>, source: &str, config: Config) {
    change(cx, handle, |s| {
        s.owners[0].update(Arc::from(source), config).unwrap()
    });
}
fn assert_idle(cx: &mut gpui::AsyncApp, handle: WindowHandle<Scene>) {
    let before = notifications(cx, handle);
    assert!(
        delivery(cx, handle) <= 1,
        "at most one obsolete wake may remain"
    );
    assert_eq!(
        notifications(cx, handle),
        before,
        "ineligible owner must not notify"
    );
    draw(cx, handle);
    assert_eq!(delivery(cx, handle), 0, "idle rendering must not requeue");
}

fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<Scene>,
    clock: &Rc<Clock>,
    retired: &Rc<RefCell<Vec<Weak<RefCell<State>>>>>,
) {
    draw(cx, handle);
    let initial = pixels(cx, handle);
    assert_eq!(phase(cx, handle), 0.);
    for _ in 0..50 {
        draw(cx, handle);
    }
    assert_eq!(
        delivery(cx, handle),
        1,
        "repeated renders coalesce into one wake"
    );
    at(clock, 250);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.25);
    assert_ne!(
        pixels(cx, handle),
        initial,
        "clock phase reaches actual glyph pixels"
    );
    assert_eq!(delivery(cx, handle), 1);
    at(clock, 1000);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 1.);
    assert_eq!(
        pixels(cx, handle),
        initial,
        "one-shot finishes with original text"
    );
    assert_idle(cx, handle);
    at(clock, 8000);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 1.);
    assert_idle(cx, handle);
    replace(
        cx,
        handle,
        "Working with native frames",
        config(Repeat::Loop),
    );
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.);
    delivery(cx, handle);
    at(clock, 8250);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.25);
    change(cx, handle, |s| s.color = 0x202080ff);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.25, "restyle preserves elapsed time");
    let mut cosmetic = config(Repeat::Loop);
    cosmetic.spread = Spread::Pixels(50.);
    cosmetic.highlight = Some(0x00ff00ff);
    replace(cx, handle, "Working with native frames", cosmetic);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.25);
    change(cx, handle, |s| s.clipped = true);
    draw(cx, handle);
    assert_idle(cx, handle);
    at(clock, 20_000);
    change(cx, handle, |s| s.clipped = false);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.25, "clipped time is not accumulated");
    change(cx, handle, |s| s.omitted = true);
    draw(cx, handle);
    assert_idle(cx, handle);
    at(clock, 30_000);
    change(cx, handle, |s| s.omitted = false);
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.25);
    cx.update(|cx| cx.set_reduce_motion(true));
    draw(cx, handle);
    assert_idle(cx, handle);
    at(clock, 40_000);
    draw(cx, handle);
    assert_idle(cx, handle);
    cx.update(|cx| cx.set_reduce_motion(false));
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.25);
    for mode in 0..5 {
        let mut next = config(Repeat::Loop);
        let source = match mode {
            0 => {
                next.animated = false;
                "Paused"
            }
            1 => {
                next.highlight = Some(0xff000000);
                "Transparent"
            }
            2 => " \t\u{2003}",
            3 => "👨‍👩‍👧‍👦",
            4 => "",
            _ => unreachable!(),
        };
        replace(cx, handle, source, next);
        draw(cx, handle);
        assert_idle(cx, handle);
    }
    replace(
        cx,
        handle,
        &"I".repeat(paint::MAX_GLYPHS + 1),
        config(Repeat::Loop),
    );
    draw(cx, handle);
    assert_idle(cx, handle);
    replace(cx, handle, "Restart", config(Repeat::Loop));
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.);
    let before = notifications(cx, handle);
    cx.update(|cx| cx.set_reduce_motion(true));
    assert_eq!(
        delivery(cx, handle),
        1,
        "the queued platform callback still arrives"
    );
    assert_eq!(
        notifications(cx, handle),
        before,
        "Reduce checked at delivery, not just old paint"
    );
    cx.update(|cx| cx.set_reduce_motion(false));
    change(cx, handle, |s| {
        retired.borrow_mut().push(Rc::downgrade(&s.owners[0].0));
        s.owners.push(
            Owner::new(
                Arc::from("Second owner"),
                config(Repeat::Loop),
                clock.clone(),
            )
            .unwrap(),
        );
    });
    draw(cx, handle);
    assert_eq!(
        delivery(cx, handle),
        2,
        "independent owners each request one frame"
    );
    at(clock, 40_250);
    draw(cx, handle);
    let old = handle
        .update(cx, |s, _, _| {
            s.owners
                .iter()
                .map(|o| Rc::downgrade(&o.0))
                .collect::<Vec<_>>()
        })
        .unwrap();
    change(cx, handle, |s| s.owners.clear());
    draw(cx, handle);
    assert!(
        old.iter().all(|owner| owner.upgrade().is_none()),
        "rendered elements cannot retain removed owners"
    );
    assert_eq!(delivery(cx, handle), 2, "retired weak callbacks drain once");
    assert_eq!(delivery(cx, handle), 0);
    change(cx, handle, |s| {
        s.owners
            .push(Owner::new(Arc::from("Remounted"), config(Repeat::Loop), clock.clone()).unwrap())
    });
    draw(cx, handle);
    assert_eq!(phase(cx, handle), 0.);
    handle
        .update(cx, |s, _, _| {
            retired.borrow_mut().push(Rc::downgrade(&s.owners[0].0))
        })
        .unwrap();
    eprintln!(
        "GPUIO_NATIVE_TEXT_SHIMMER_CLOCK_OK: visible GPU progression, once idle, coalesced frames, cosmetic reuse, clipped/omitted/Reduce/static/transparent/emoji/capacity idle, independent owners, weak unmount and fresh remount"
    );
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let retired = Rc::new(RefCell::new(Vec::new()));
    let task_retired = retired.clone();
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let clock = Rc::new(Clock::default());
        at(&clock, 0);
        let owner = Owner::new(
            Arc::from("Working with native frames"),
            config(Repeat::Once),
            clock.clone(),
        )
        .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    focus: false,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(100.), px(100.)),
                        size(px(360.), px(220.)),
                    ))),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| Scene {
                        owners: vec![owner],
                        clipped: false,
                        omitted: false,
                        color: 0x000000ff,
                    })
                },
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(async {
                exercise(cx, handle, &clock, &task_retired)
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
    assert!(
        retired
            .borrow()
            .iter()
            .all(|owner| owner.upgrade().is_none()),
        "closing the window releases owners despite pending weak wakes"
    );
    eprintln!("GPUIO_NATIVE_TEXT_SHIMMER_CLOSE_OK");
}
