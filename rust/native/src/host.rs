use crate::{session::Session, transport::Transport};
use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowHandle, WindowOptions, canvas, div,
    prelude::*, px, rgba, size,
};
use gpuio_protocol::{
    NodeId, WindowId,
    v1::{self, *},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};

type SharedSession = Rc<RefCell<Session>>;
struct View {
    id: WindowId,
    session: SharedSession,
    transport: Arc<Transport>,
}
fn color(value: &Color) -> gpui::Hsla {
    // Named token resolution will be supplied by the typed theme adapter (OCH-8).
    match value {
        Color::Rgba(value) => rgba(*value as u32).into(),
        Color::Token(_) => unreachable!("unresolved theme token passed validation"),
    }
}
fn length(value: &v1::Length) -> gpui::Length {
    match value {
        v1::Length::Auto => gpui::Length::Auto,
        v1::Length::Px(v) => px(*v as f32).into(),
        v1::Length::Percent(v) => gpui::relative(*v as f32 / 100.).into(),
    }
}
impl View {
    fn element(&self, tree: &crate::tree::Tree, id: NodeId) -> gpui::AnyElement {
        let node = tree.get(id).expect("validated retained node");
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        let mut element = div().id(("gpuio-node", identity));
        if node.kind == Kind::Container {
            element = element.flex().flex_col();
        }
        if node.kind == Kind::Button {
            element = element.focusable().cursor_pointer();
        }
        let mut pointer_enabled = true;
        for style in node.style.iter() {
            match style {
                Style::Fields(fields) => {
                    crate::style::refine(element.style(),fields);
                    for field in fields { if let Field::PointerEvents(enabled)=field {pointer_enabled=*enabled;} }
                },
                Style::State(state,fields) => {
                    let mut refinement=gpui::StyleRefinement::default();
                    crate::style::refine(&mut refinement,fields);
                    element=match state {
                        1=>element.focus(move |_|refinement),
                        2=>element.hover(move |_|refinement),
                        3=>element.active(move |_|refinement),
                        _=>unreachable!("validated state"),
                    };
                },
                Style::Width(v) => element.style().size.width = Some(length(v)),
                Style::Height(v) => element.style().size.height = Some(length(v)),
                Style::MinWidth(v) => element.style().min_size.width = Some(length(v)),
                Style::MinHeight(v) => element.style().min_size.height = Some(length(v)),
                Style::MaxWidth(v) => element.style().max_size.width = Some(length(v)),
                Style::MaxHeight(v) => element.style().max_size.height = Some(length(v)),
                Style::Padding(v) => element = element.p(px(*v as f32)),
                Style::Gap(v) => element = element.gap(px(*v as f32)),
                Style::Grow(v) => element.style().flex_grow = Some(*v as f32),
                Style::Shrink(v) => element.style().flex_shrink = Some(*v as f32),
                Style::Direction(v) => {
                    element.style().flex_direction = Some(match v {
                        0 => gpui::FlexDirection::Row,
                        1 => gpui::FlexDirection::Column,
                        2 => gpui::FlexDirection::RowReverse,
                        _ => gpui::FlexDirection::ColumnReverse,
                    })
                }
                Style::Background(v) => element = element.bg(color(v)),
                Style::Foreground(v) => element = element.text_color(color(v)),
                Style::FontSize(v) => element = element.text_size(px(*v as f32)),
                Style::Radius(v) => element = element.rounded(px(*v as f32)),
                Style::Opacity(v) => element = element.opacity(*v as f32),
                Style::HoverBackground(v) => {
                    let v = color(v);
                    element = element.hover(move |s| s.bg(v));
                }
                Style::PressedBackground(v) => {
                    let v = color(v);
                    element = element.active(move |s| s.bg(v));
                }
                Style::FocusBackground(v) => {
                    let v = color(v);
                    element = element.focus(move |s| s.bg(v));
                }
            }
        }
        if !node.text.is_empty() {
            element = element.child(gpui::SharedString::from(node.text.clone()));
        }
        element = element.children(node.children.iter().map(|id| self.element(tree, *id)));
        if let Some(handler) = node.handler && pointer_enabled {
            let window = self.id;
            let revision = tree.revision();
            let session = self.session.clone();
            let transport = self.transport.clone();
            element = element.on_click(move |_, _, _| {
                let event = session.borrow().press(window, id, handler, revision);
                if let Some(event) = event
                    && !transport.input(event)
                    && session.borrow_mut().overload(window)
                {
                    transport.fault(window);
                }
            });
        }
        element.into_any_element()
    }
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let session = self.session.borrow();
        let mut root = div().size_full();
        let revision = if let Some(tree) = session.tree(self.id) {
            if let Some(id) = tree.root() {
                root = root.child(self.element(tree, id));
            }
            tree.revision()
        } else {
            0
        };
        let id = self.id;
        let session = self.session.clone();
        let transport = self.transport.clone();
        root.child(
            canvas(
                |_, _, _| (),
                move |_, _, _, _| {
                    let events = session.borrow_mut().painted(id, revision);
                    for event in events {
                        if matches!(event, Event::FrameRequested(..)) {
                            transport.respond(event);
                        } else if !transport.input(event) && session.borrow_mut().overload(id) {
                            transport.fault(id);
                        }
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

fn failed(transport: &Transport, correlation: i64, result: Result<Event, ErrorCode>) {
    transport.respond(result.unwrap_or_else(|error| Event::Failed(correlation, error)));
}

pub fn run(transport: Arc<Transport>) {
    #[cfg(target_os = "linux")]
    if let Ok(expected) = std::env::var("GPUIO_EXPECT_BACKEND") {
        assert_eq!(gpui::guess_compositor(), expected);
    }
    let stopping = Rc::new(Cell::new(false));
    gpui_platform::application().run(move |cx: &mut App| {
        let session = Rc::new(RefCell::new(Session::default()));
        let mut windows: BTreeMap<WindowId, WindowHandle<View>> = BTreeMap::new();
        let closing = stopping.clone();
        cx.on_window_closed(move |cx, _| {
            if cx.windows().is_empty() && !closing.replace(true) {
                cx.defer(stop_application);
            }
        })
        .detach();
        let rx = transport.rx.clone();
        cx.spawn(async move |cx| {
            while rx.recv().await.is_ok() {
                if transport
                    .aborting
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    if !stopping.replace(true) {
                        cx.update(stop_application);
                    }
                    return;
                }
                loop {
                    let id = transport
                        .mailbox
                        .lock()
                        .expect("mailbox poisoned")
                        .pop_close();
                    let Some(id) = id else {
                        break;
                    };
                    if let Ok(pending) = session.borrow_mut().close(id) {
                        if let Some(correlation) = pending {
                            transport.respond(Event::Failed(correlation, ErrorCode::Closed));
                        }
                        transport
                            .mailbox
                            .lock()
                            .expect("mailbox poisoned")
                            .native_closed(id);
                        transport.wake_ocaml();
                        if let Some(window) = windows.remove(&id) {
                            let _ = window.update(cx, |_, window, _| window.remove_window());
                        }
                    }
                }
                // Bound work per wake; yield to native input/paint between batches.
                for _ in 0..crate::mailbox::MAX_COMMANDS {
                    let message = transport.mailbox.lock().expect("mailbox poisoned").pop();
                    let Some(message) = message else {
                        break;
                    };
                    match message {
                        Message::Hello(version, caps) => {
                            failed(&transport, 0, session.borrow_mut().hello(version, caps))
                        }
                        Message::Open(correlation, id, title, width, height) => {
                            let validation = if transport
                                .mailbox
                                .lock()
                                .unwrap()
                                .has_window_output(id.slot())
                            {
                                Err(ErrorCode::Busy)
                            } else {
                                session.borrow().validate_open(id, &title, width, height)
                            };
                            if let Err(error) = validation {
                                transport.respond(Event::Failed(correlation, error));
                                continue;
                            }
                            let native = cx.update(|cx| {
                                let bounds = Bounds::centered(
                                    None,
                                    size(px(width as f32), px(height as f32)),
                                    cx,
                                );
                                cx.open_window(
                                    WindowOptions {
                                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                                        titlebar: Some(gpui::TitlebarOptions {
                                            title: Some(title.clone().into()),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        let close_transport = transport.clone();
                                        window.on_window_should_close(cx, move |_, _| {
                                            close_transport
                                                .mailbox
                                                .lock()
                                                .expect("mailbox poisoned")
                                                .request_close(id);
                                            let _ = close_transport.tx.try_send(());
                                            false
                                        });
                                        cx.new(|_| View {
                                            id,
                                            session: session.clone(),
                                            transport: transport.clone(),
                                        })
                                    },
                                )
                            });
                            match native {
                                Ok(window) => {
                                    windows.insert(id, window);
                                    failed(
                                        &transport,
                                        correlation,
                                        session.borrow_mut().open(
                                            correlation,
                                            id,
                                            &title,
                                            width,
                                            height,
                                        ),
                                    );
                                    cx.update(|cx| cx.activate(true));
                                }
                                Err(_) => transport
                                    .respond(Event::Failed(correlation, ErrorCode::NativeFailure)),
                            }
                        }
                        Message::Apply(tx) => {
                            let result = session.borrow_mut().apply(&tx);
                            match result {
                                Ok(_) => {
                                    transport.respond(Event::Accepted(tx.window, tx.revision));
                                    if let Some(window) = windows.get(&tx.window) {
                                        let _ = window.update(cx, |_, _, cx| cx.notify());
                                    }
                                }
                                Err(error) => transport.respond(Event::Rejected(
                                    tx.window,
                                    tx.revision,
                                    error,
                                )),
                            }
                        }
                        Message::RequestFrame(correlation, id) => {
                            let result = session.borrow_mut().request_frame(correlation, id);
                            match result {
                                Ok(()) => {
                                    if let Some(window) = windows.get(&id) {
                                        let _ = window.update(cx, |_, _, cx| cx.notify());
                                    }
                                }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
                        }
                        Message::Close(correlation, id) => {
                            let result = session.borrow_mut().close(id);
                            match result {
                                Ok(pending) => {
                                    if let Some(pending) = pending {
                                        transport
                                            .respond(Event::Failed(pending, ErrorCode::Closed));
                                    }
                                    transport.respond(Event::Closed(correlation, id));
                                    if let Some(window) = windows.remove(&id) {
                                        let _ = window
                                            .update(cx, |_, window, _| window.remove_window());
                                    }
                                }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
                        }
                        Message::Shutdown => {
                            for event in session.borrow_mut().shutdown() {
                                transport.respond(event);
                            }
                            if !stopping.replace(true) {
                                cx.update(stop_application);
                            }
                            return;
                        }
                    }
                }
                // A ready channel alone need not yield its future. Give native
                // input/paint a turn even with a continuously producing client.
                // This timer exists only after work, never as idle polling.
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(1))
                    .await;
            }
        })
        .detach();
    });
}

#[allow(deprecated)]
#[cfg(target_os = "macos")]
fn stop_application(cx: &mut App) {
    use cocoa::{
        appkit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventSubtype, NSEventType},
        base::{YES, nil},
        foundation::NSPoint,
    };
    cx.shutdown();
    // Embedded runtime must regain control instead of NSApplication.terminate.
    unsafe {
        let app = cocoa::appkit::NSApp();
        app.stop_(nil);
        let wake=cocoa::base::id::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2_(nil,NSEventType::NSApplicationDefined,NSPoint::new(0.,0.),NSEventModifierFlags::empty(),0.,0,nil,NSEventSubtype::NSWindowExposedEventType,0,0);
        app.postEvent_atStart_(wake, YES);
    }
}
#[cfg(not(target_os = "macos"))]
fn stop_application(cx: &mut App) {
    cx.quit();
}
