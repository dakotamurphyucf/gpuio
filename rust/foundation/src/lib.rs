mod bridge;
mod diagnostics;
mod protocol;
mod text_input;
mod tree;
mod two_windows;

use gpui::{
    App, Bounds, Context, Entity, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, PlatformInput,
    Window, WindowBounds, WindowOptions, canvas, div, point, prelude::*, px, rgb, size,
};
use protocol::{Batch, Event, Op};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

struct View {
    tree: tree::Tree,
    last_frame: Rc<Cell<i64>>,
    inputs: BTreeMap<i64, Entity<text_input::TextInput>>,
    positions: Rc<RefCell<BTreeMap<i64, Bounds<Pixels>>>>,
}
impl View {
    fn apply(
        &mut self,
        batch: Batch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let next = self.tree.stage(&batch)?;
        self.inputs.retain(|id, _| next.nodes.contains_key(id));
        for node in next.nodes.values().filter(|n| n.kind == 3) {
            self.inputs
                .entry(node.id)
                .or_insert_with(|| cx.new(|cx| text_input::TextInput::new(node.id, cx)));
        }
        self.tree = next;
        for op in &batch.ops {
            if let Op::Edit(id, revision, text) = op {
                self.inputs[id].update(cx, |input, cx| {
                    input.replace_from_ocaml(*revision, text, window, cx)
                });
            }
        }
        cx.notify();
        log::debug!("foundation applied revision {}", self.tree.revision);
        bridge::emit(Event::Applied(
            self.tree.revision,
            self.tree.nodes.len() as i64,
            batch.ops.len() as i64,
        ));
        Ok(())
    }
    fn element(&self, id: i64) -> gpui::AnyElement {
        let node = &self.tree.nodes[&id];
        match node.kind {
            0 => div()
                .id(("node", id as u64))
                .flex()
                .flex_col()
                .gap_2()
                .children(node.children.iter().map(|id| self.element(*id)))
                .into_any_element(),
            1 => div()
                .id(("node", id as u64))
                .text_sm()
                .child(node.text.clone())
                .into_any_element(),
            2 => {
                let revision = self.tree.revision;
                let handler = node.handler.unwrap_or(0);
                let positions = self.positions.clone();
                div()
                    .id(("node", id as u64))
                    .relative()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(0x29415e))
                    .hover(|s| s.bg(rgb(0x395b82)))
                    .cursor_pointer()
                    .child(node.text.clone())
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                positions.borrow_mut().insert(id, bounds);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .on_click(move |_, _, _| bridge::emit(Event::Click(id, handler, revision)))
                    .into_any_element()
            }
            3 => div()
                .id(("node", id as u64))
                .text_color(rgb(0x152238))
                .child(self.inputs[&id].clone())
                .into_any_element(),
            _ => unreachable!(),
        }
    }
    fn probe(&mut self, code: i64, window: &mut Window, cx: &mut Context<Self>) {
        match code {
            -1 => self.inputs[&30].update(cx, |input, cx| input.exercise_ime(window, cx)),
            -2 => bridge::emit(Event::Probe(format!(
                "snapshot:{}:{}:{}",
                self.tree.revision,
                self.tree.nodes.len(),
                self.tree
                    .nodes
                    .values()
                    .map(|n| n.text.as_str())
                    .collect::<Vec<_>>()
                    .join("|")
            ))),
            -3 => bridge::emit(Event::Click(103, 103, self.tree.revision.saturating_sub(1))),
            id if id > 0 => {
                let bounds = self.positions.borrow().get(&id).copied();
                if let Some(bounds) = bounds {
                    let position = point(bounds.left() + px(8.), bounds.top() + px(8.));
                    window.dispatch_event(
                        PlatformInput::MouseDown(MouseDownEvent {
                            button: MouseButton::Left,
                            position,
                            modifiers: Default::default(),
                            click_count: 1,
                            first_mouse: false,
                        }),
                        cx,
                    );
                    window.dispatch_event(
                        PlatformInput::MouseUp(MouseUpEvent {
                            button: MouseButton::Left,
                            position,
                            modifiers: Default::default(),
                            click_count: 1,
                        }),
                        cx,
                    );
                } else {
                    bridge::emit(Event::Error(format!("no painted bounds for button {id}")));
                }
            }
            _ => bridge::emit(Event::Error("unknown probe".into())),
        }
    }
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        log::debug!("foundation render revision {}", self.tree.revision);
        self.positions.borrow_mut().clear();
        let mut root = div()
            .id("root")
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .bg(rgb(0x152238))
            .text_color(rgb(0xf0f4ff))
            .text_base();
        if let Some(id) = self.tree.root {
            root = root.child(self.element(id));
        } else {
            root = root.child("Starting Bonsai…");
        }
        let revision = self.tree.revision;
        let last_frame = self.last_frame.clone();
        root.child(
            canvas(
                |_, _, _| (),
                move |_, _, _, _| {
                    if last_frame.replace(revision) != revision {
                        bridge::emit(Event::Frame(revision));
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}
pub fn run() {
    diagnostics::init();
    gpui_platform::application().run(|cx: &mut App| {
        text_input::bind_keys(cx);
        let bounds = Bounds::centered(None, size(px(720.), px(900.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("GPUIO · Bonsai + Eio + Rust".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| View {
                        tree: Default::default(),
                        last_frame: Rc::new(Cell::new(-1)),
                        inputs: Default::default(),
                        positions: Default::default(),
                    })
                },
            )
            .unwrap();
        cx.activate(true);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.defer(stop_application);
            }
        })
        .detach();
        let rx = bridge::receiver();
        cx.spawn(async move |cx| {
            while let Ok(command) = rx.recv().await {
                match command {
                    bridge::Command::Apply(batch) => {
                        let result =
                            window.update(cx, |view, window, cx| view.apply(batch, window, cx));
                        match result {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => bridge::emit(Event::Error(e)),
                            Err(e) => {
                                bridge::emit(Event::Error(e.to_string()));
                                break;
                            }
                        }
                    }
                    bridge::Command::Probe(code) => {
                        if let Err(e) =
                            window.update(cx, |view, window, cx| view.probe(code, window, cx))
                        {
                            bridge::emit(Event::Error(e.to_string()));
                        }
                    }
                    bridge::Command::Quit => {
                        cx.update(stop_application);
                        break;
                    }
                }
            }
        })
        .detach();
        bridge::emit(Event::Ready);
    });
    bridge::closed();
}
pub fn hello() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(3));
        bridge::request_quit();
    });
    run();
}

#[allow(deprecated)]
#[cfg(target_os = "macos")]
fn stop_application(cx: &mut App) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STOPPING: AtomicBool = AtomicBool::new(false);
    if STOPPING.swap(true, Ordering::SeqCst) {
        return;
    }
    cx.shutdown();
    // NSApplication.terminate exits the process. An embedded OCaml runtime must
    // instead regain control, join its domain, and run its own shutdown sequence.
    use cocoa::{
        appkit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventSubtype, NSEventType},
        base::{YES, nil},
        foundation::NSPoint,
    };
    unsafe {
        let app = cocoa::appkit::NSApp();
        app.stop_(nil);
        let wake = cocoa::base::id::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2_(nil, NSEventType::NSApplicationDefined, NSPoint::new(0.,0.), NSEventModifierFlags::empty(),0.,0,nil,NSEventSubtype::NSWindowExposedEventType,0,0);
        app.postEvent_atStart_(wake, YES);
    }
}

#[cfg(not(target_os = "macos"))]
fn stop_application(cx: &mut App) {
    cx.quit();
}
