//! Short actual-renderer qualification, separate from workload performance budgets.
//! Runs two visible windows, then verifies settlement and window retirement.
#[cfg(target_os = "macos")]
mod macos {
    use gpui::{
        AppContext, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window,
        WindowBounds, WindowOptions, div, point, px, rgb, size,
    };
    use gpuio_native::performance::{
        WindowObservation,
        presentation::{Limits, Outcome, Session},
    };
    use serde_json::{Value, json};
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    const FRAMES: usize = 90;
    struct Probe {
        frames: usize,
        running: bool,
        coasting: bool,
        renders: usize,
        index: usize,
    }
    impl Render for Probe {
        fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            if self.running {
                self.renders += 1;
                if self.frames < FRAMES {
                    self.frames += 1;
                }
                if self.frames < FRAMES || self.coasting {
                    window.request_animation_frame();
                }
            }
            div()
                .size_full()
                .bg(rgb(0x182234))
                .text_color(rgb(0xe2ebf8))
                .p_6()
                .flex()
                .flex_col()
                .gap_4()
                .child(format!("GPUIO presentation probe {}", self.index + 1))
                .child("Measuring Metal callbacks; closes automatically.")
                .child(
                    div()
                        .h(px(12.))
                        .w(px(280. * self.frames as f32 / FRAMES as f32))
                        .bg(rgb(if self.index == 0 { 0x6bc9b5 } else { 0x97b8f4 })),
                )
                .child(format!("Frame {} / {FRAMES}", self.frames))
        }
    }

    fn report(session: &Session) -> Value {
        let s = session.snapshot();
        let c = s.counts;
        json!({"session": s.session, "window": format!("{:?}", s.window),
            "accepting": s.accepting, "window_closed": s.window_closed, "pending": s.pending,
            "counts": {"attempted": c.attempted, "admitted": c.admitted,
                "saturated": c.saturated, "presented": c.presented,
                "not_submitted": c.not_submitted, "missing": c.missing,
                "zero": c.zero, "invalid_clock": c.invalid_clock,
                "duplicate_callbacks": c.duplicate_callbacks,
                "trace_truncated": c.trace_truncated, "histogram_overflow": c.histogram_overflow},
            "submission_samples": s.submission_latency.len(),
            "input_samples": s.input_latency.len(),
            "animation_samples": s.animation_interval.len(),
            "records": s.trace.iter().map(|r| json!({"sequence": r.sequence,
                "drawable": r.drawable, "new_scene": r.new_scene, "active": r.active,
                "animating": r.animating, "inputs": r.inputs, "outcome": format!("{:?}", r.outcome),
                "submit_host_s": r.submit_host_s, "presented_host_s": r.presented_host_s,
                "callback_host_s": r.callback_host_s,
                "submission_lower_ns": r.submission_latency.map(|v| v.lower_ns),
                "submission_upper_ns": r.submission_latency.map(|v| v.upper_ns)
            })).collect::<Vec<_>>()})
    }

    fn accepted(session: &Session) -> bool {
        let s = session.snapshot();
        let c = s.counts;
        s.window_closed
            && !s.accepting
            && s.pending == 0
            && c.presented >= FRAMES as u64
            && c.attempted == c.presented
            && c.admitted == c.presented
            && c.saturated == 0
            && c.not_submitted == 0
            && c.missing == 0
            && c.zero == 0
            && c.invalid_clock == 0
            && c.duplicate_callbacks == 0
            && c.trace_truncated == 0
            && c.histogram_overflow == 0
            && s.trace.len() == c.presented as usize
            && s.submission_latency.len() == c.presented
            && s.input_latency.is_empty()
            && s.trace
                .iter()
                .all(|r| r.outcome == Outcome::Presented && r.inputs == 0)
    }

    pub fn run(path: PathBuf, idle_before_frames: Duration, warmup: bool, continuous_warmup: bool) {
        gpui_platform::application().run(move |cx| {
            cx.set_quit_mode(gpui::QuitMode::Explicit);
            let bounds = Bounds::centered(None, size(px(760.), px(280.)), cx);
            let windows = (0..2)
                .map(|index| {
                    cx.open_window(
                        WindowOptions {
                            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                                point(bounds.origin.x + px(index as f32 * 380.), bounds.origin.y),
                                size(px(360.), px(280.)),
                            ))),
                            focus: index == 1,
                            ..Default::default()
                        },
                        |_, cx| {
                            cx.new(|_| Probe {
                                frames: 0,
                                running: false,
                                coasting: false,
                                renders: 0,
                                index,
                            })
                        },
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            cx.activate(true);
            cx.spawn(async move |cx| {
                let start = Instant::now();
                let mut visible = false;
                while start.elapsed() < Duration::from_secs(5) {
                    visible = windows.iter().all(|w| {
                        w.update(cx, |_, window, _| {
                            WindowObservation::capture(window).visible == Some(true)
                        })
                        .unwrap_or(false)
                    });
                    if visible {
                        break;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(20))
                        .await;
                }
                // Diagnostic isolation of idle-to-active behavior: this runs no
                // OCaml/Eio bridge and does not alter renderer presentation policy.
                if !idle_before_frames.is_zero() {
                    cx.background_executor().timer(idle_before_frames).await;
                }
                // Separate, fully retained active warmup. The measured phase
                // below keeps its original 90-frame, zero-loss/zero-skip checks.
                // This is steady-state hookup qualification, not cold startup.
                let mut warmup_reports = Vec::new();
                let mut warmup_finished = !warmup;
                let mut transition_unmeasured_render_counts = Vec::new();
                if warmup {
                    let preflight = windows.iter().map(|w| {
                        w.update(cx, |view, window, cx| {
                            let session = Session::start(window, Limits::default()).unwrap();
                            view.running = true;
                            view.coasting = continuous_warmup;
                            cx.notify();
                            session
                        }).unwrap()
                    }).collect::<Vec<_>>();
                    let start = Instant::now();
                    while start.elapsed() < Duration::from_secs(12) {
                        warmup_finished = windows.iter().all(|w| {
                            w.update(cx, |view, _, _| view.frames == FRAMES).unwrap_or(false)
                        });
                        if warmup_finished { break; }
                        cx.background_executor().timer(Duration::from_millis(20)).await;
                    }
                    for session in &preflight { session.stop(); }
                    // Continue scheduling frames while old callbacks settle.
                    // These transition renders are explicitly unmeasured; do not
                    // count them as successful warmup or measured presentations.
                    let transition_start = windows.iter().map(|window| {
                        window.update(cx, |view, _, _| view.renders).unwrap()
                    }).collect::<Vec<_>>();
                    let settlement = Instant::now();
                    while settlement.elapsed() < Duration::from_secs(2)
                        && preflight.iter().any(|s| s.pending() != 0) {
                        cx.background_executor().timer(Duration::from_millis(10)).await;
                    }
                    warmup_finished &= preflight.iter().all(|s| s.pending() == 0);
                    warmup_reports = preflight.iter().map(report).collect();
                    // Snapshots retain values only. Drop the old measurement
                    // owners before starting distinct replacement sessions.
                    drop(preflight);
                    transition_unmeasured_render_counts = windows.iter().zip(transition_start).map(|(window, before)| {
                        window.update(cx, |view, _, _| view.renders - before).unwrap()
                    }).collect::<Vec<_>>();
                    for window in &windows {
                        window.update(cx, |view, _, _| {
                            view.frames = 0;
                            view.running = continuous_warmup;
                            view.coasting = false;
                        }).unwrap();
                    }
                }
                let sessions = windows
                    .iter()
                    .map(|w| {
                        w.update(cx, |view, window, cx| {
                            let session = Session::start(window, Limits::default()).unwrap();
                            view.running = true;
                            // Coasting already queued the next animation frame.
                            // Avoid an idle-to-active notify at this boundary.
                            if !continuous_warmup { cx.notify(); }
                            session
                        })
                        .unwrap()
                    })
                    .collect::<Vec<_>>();
                let start = Instant::now();
                let mut finished = false;
                while start.elapsed() < Duration::from_secs(12) {
                    finished = windows.iter().all(|w| {
                        w.update(cx, |view, _, _| view.frames == FRAMES)
                            .unwrap_or(false)
                    });
                    if finished {
                        break;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(20))
                        .await;
                }
                for session in &sessions {
                    session.stop();
                }
                let settlement = Instant::now();
                while settlement.elapsed() < Duration::from_secs(2)
                    && sessions.iter().any(|s| s.snapshot().pending != 0)
                {
                    cx.background_executor()
                        .timer(Duration::from_millis(10))
                        .await;
                }
                let end_visible = windows.iter().all(|w| {
                    w.update(cx, |_, window, _| {
                        WindowObservation::capture(window).visible == Some(true)
                    })
                    .unwrap_or(false)
                });
                for window in windows {
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                }
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let distinct = sessions[0].snapshot().session != sessions[1].snapshot().session
                    && sessions[0].snapshot().window != sessions[1].snapshot().window;
                let animation_samples = sessions.iter().any(|s| s.snapshot().animation_interval.len() >= (FRAMES / 2) as u64);
                let valid = warmup_finished && animation_samples &&
                    visible && end_visible && finished && distinct && sessions.iter().all(accepted);
                let output = json!({"schema": 1, "kind": "gpui_metal_hook_qualification",
                    "passed": valid, "continuous_warmup": continuous_warmup, "transition_unmeasured_render_counts": transition_unmeasured_render_counts, "warmup": warmup, "warmup_finished": warmup_finished, "warmup_sessions": warmup_reports, "idle_before_frames_ms": idle_before_frames.as_millis(), "visible_at_start": visible, "visible_at_end": end_visible,
                    "finished": finished, "frames_per_window": FRAMES, "distinct": distinct, "animation_samples": animation_samples,
                    "sessions": sessions.iter().map(report).collect::<Vec<_>>()});
                std::fs::write(&path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
                println!(
                    "GPUIO_METAL_PRESENTATION_REPORT {} passed={valid}",
                    path.display()
                );
                cx.update(|cx| cx.quit());
            })
            .detach();
        });
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        let path = std::env::var_os("GPUIO_PRESENTATION_REPORT")
            .expect("set GPUIO_PRESENTATION_REPORT to a fresh report path");
        let idle_ms = std::env::var("GPUIO_PRESENTATION_IDLE_MS")
            .map(|value| {
                value
                    .parse::<u64>()
                    .expect("idle milliseconds must be an integer")
            })
            .unwrap_or(0);
        assert!(idle_ms <= 5000, "idle delay exceeds diagnostic limit");
        macos::run(
            path.into(),
            std::time::Duration::from_millis(idle_ms),
            std::env::var("GPUIO_PRESENTATION_WARMUP").is_ok_and(|v| v == "1"),
            std::env::var("GPUIO_PRESENTATION_CONTINUOUS_WARMUP").is_ok_and(|v| v == "1"),
        );
    }
    #[cfg(not(target_os = "macos"))]
    eprintln!("GPUIO_METAL_PRESENTATION_UNSUPPORTED: macOS Metal qualification only");
}
