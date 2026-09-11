//! Bootstrap identity/lifetime check, independent of the future multi-window protocol.

use crate::text_input::{self, TextInput};
use gpui::{
    App, Bounds, Context, Entity, Window, WindowBounds, WindowHandle, WindowOptions, div,
    prelude::*, px, rgb, size,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

struct View {
    name: &'static str,
    input: Entity<TextInput>,
}

impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_6()
            .bg(rgb(0xf0f4ff))
            .text_color(rgb(0x152238))
            .child(self.name)
            .child(self.input.clone())
    }
}

fn open(cx: &mut App, name: &'static str, editor_id: i64) -> WindowHandle<View> {
    let bounds = Bounds::centered(None, size(px(460.), px(240.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some(name.into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        |_, cx| {
            cx.new(|cx| View {
                name,
                input: cx.new(|cx| TextInput::new(editor_id, cx)),
            })
        },
    )
    .expect("open smoke window")
}

pub fn run() {
    crate::diagnostics::init();
    let completed = Arc::new(AtomicBool::new(false));
    let result = completed.clone();
    gpui_platform::application().run(move |cx: &mut App| {
        let checked = Arc::new(AtomicBool::new(false));
        let checked_on_close = checked.clone();
        cx.on_window_closed(move |cx, _| {
            if cx.windows().is_empty() {
                assert!(checked_on_close.load(Ordering::SeqCst), "windows closed before checks finished");
                println!("TWO_WINDOWS_PASS distinct_ids isolated_editors stale_window_rejected survivor_usable closed");
                result.store(true, Ordering::SeqCst);
                cx.defer(crate::stop_application);
            }
        }).detach();
        text_input::bind_keys(cx);
        let first = open(cx, "GPUIO · first window", 101);
        let second = open(cx, "GPUIO · second window", 202);
        assert_ne!(first.window_id(), second.window_id());
        cx.activate(true);
        cx.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_millis(300)).await;
            first.update(cx, |view, window, cx| {
                view.input.update(cx, |input, cx| input.exercise_ime(window, cx));
                assert_eq!(view.input.read(cx).text(), "A日本語Z");
            }).expect("first window input");
            second.update(cx, |view, window, cx| {
                assert_eq!(view.input.read(cx).text(), "");
                view.input.update(cx, |input, cx| {
                    input.replace_from_ocaml(0, "second window state", window, cx);
                });
            }).expect("second window isolated input");
            first.update(cx, |_, window, _| window.remove_window())
                .expect("close first window");
            cx.background_executor().timer(Duration::from_millis(300)).await;
            cx.update(|cx| assert_eq!(cx.windows().len(), 1));
            assert!(first.update(cx, |_, _, _| ()).is_err());
            second.update(cx, |view, window, cx| {
                assert_eq!(view.input.read(cx).text(), "second window state");
                view.input.update(cx, |input, cx| input.exercise_ime(window, cx));
                assert_eq!(view.input.read(cx).text(), "A日本語Z");
                checked.store(true, Ordering::SeqCst);
                window.remove_window();
            }).expect("surviving window remains usable");
        }).detach();
    });
    assert!(
        completed.load(Ordering::SeqCst),
        "two-window checks did not finish"
    );
}
