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
struct ButtonState {
    focus: gpui::FocusHandle,
    space_down: Cell<bool>,
}
#[derive(Clone, Copy)]
struct Interaction {
    pointer: bool,
    selectable: bool,
    selection_color: gpui::Hsla,
}
impl Default for Interaction {
    fn default() -> Self {
        Self {
            pointer: true,
            selectable: false,
            selection_color: rgba(0x386ac880).into(),
        }
    }
}

struct View {
    id: WindowId,
    session: SharedSession,
    transport: Arc<Transport>,
    buttons: BTreeMap<NodeId, Rc<ButtonState>>,
    selections: BTreeMap<NodeId, Rc<RefCell<crate::selection::State>>>,
    visited: std::collections::BTreeSet<NodeId>,
    #[cfg(feature = "native-tests")]
    probes: Rc<RefCell<BTreeMap<NodeId, native_test::Probe>>>,
}
fn emit_press(
    session: &SharedSession,
    transport: &Transport,
    window: WindowId,
    node: NodeId,
    handler: gpuio_protocol::HandlerId,
    revision: i64,
) {
    let event = session.borrow().press(window, node, handler, revision);
    if let Some(event) = event
        && !transport.input(event)
        && session.borrow_mut().overload(window)
    {
        transport.fault(window);
    }
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
    fn new(id: WindowId, session: SharedSession, transport: Arc<Transport>) -> Self {
        Self {
            id,
            session,
            transport,
            buttons: BTreeMap::new(),
            selections: BTreeMap::new(),
            visited: Default::default(),
            #[cfg(feature = "native-tests")]
            probes: Default::default(),
        }
    }
    fn element(
        &mut self,
        tree: &crate::tree::Tree,
        id: NodeId,
        mut interaction: Interaction,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let node = tree.get(id).expect("validated retained node");
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        let mut accessible_name = gpui::SharedString::from(node.text.clone());
        for style in node.style.iter() {
            if let Style::Fields(fields) = style {
                for field in fields {
                    match field {
                        Field::PointerEvents(v) => interaction.pointer = *v,
                        Field::UserSelect(v) => interaction.selectable = *v,
                        Field::SelectionColor(v) => interaction.selection_color = color(v),
                        Field::AccessibleName(v) => accessible_name = v.clone().into(),
                        _ => (),
                    }
                }
            }
        }
        let mut element = div().id(("gpuio-node", identity));
        if node.kind == Kind::Container {
            element = element.flex().flex_col();
        }
        let button = if node.kind == Kind::Button {
            self.visited.insert(id);
            let state = self
                .buttons
                .entry(id)
                .or_insert_with(|| {
                    Rc::new(ButtonState {
                        focus: cx.focus_handle().tab_stop(true),
                        space_down: Cell::new(false),
                    })
                })
                .clone();
            element = element
                .track_focus(&state.focus)
                .tab_index(0)
                .role(gpui::Role::Button)
                .aria_label(accessible_name);
            if interaction.pointer {
                element = element.cursor_pointer();
            }
            Some(state)
        } else {
            None
        };
        let mut states: [Option<gpui::StyleRefinement>; 3] = Default::default();
        for style in node.style.iter() {
            match style {
                Style::Fields(fields) => {
                    crate::style::refine(element.style(), fields);
                }
                Style::State(state, fields) => {
                    if *state != 1 && !interaction.pointer {
                        continue;
                    }
                    let refinement =
                        states[(*state - 1) as usize].get_or_insert_with(Default::default);
                    crate::style::refine(refinement, fields);
                }
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
                    if interaction.pointer {
                        states[1].get_or_insert_with(Default::default).background =
                            Some(color(v).into());
                    }
                }
                Style::PressedBackground(v) => {
                    if interaction.pointer {
                        states[2].get_or_insert_with(Default::default).background =
                            Some(color(v).into());
                    }
                }
                Style::FocusBackground(v) => {
                    states[0].get_or_insert_with(Default::default).background =
                        Some(color(v).into());
                }
            }
        }
        let [focused, hovered, pressed] = states;
        if let Some(style) = focused {
            element = element.focus(move |_| style);
        }
        if let Some(style) = hovered {
            element = element.hover(move |_| style);
        }
        if let Some(style) = pressed {
            element = element.active(move |_| style);
        }
        if !interaction.pointer {
            element.style().mouse_cursor = None;
        }
        if node.kind == Kind::Text && interaction.selectable {
            self.visited.insert(id);
            let selection = self
                .selections
                .entry(id)
                .or_insert_with(|| {
                    Rc::new(RefCell::new(crate::selection::State::new(
                        node.text.clone(),
                        cx,
                    )))
                })
                .clone();
            selection.borrow_mut().update(node.text.clone());
            element = element.child(crate::selection::element(
                selection,
                interaction.selection_color,
                interaction.pointer,
                cx.entity_id(),
            ));
        } else if !node.text.is_empty() {
            element = element.child(gpui::SharedString::from(node.text.clone()));
        }
        element = element.children(
            node.children
                .iter()
                .map(|id| self.element(tree, *id, interaction, cx))
                .collect::<Vec<_>>(),
        );
        if let Some(handler) = node.handler {
            let window = self.id;
            let revision = tree.revision();
            let session = self.session.clone();
            let transport = self.transport.clone();
            if let Some(button) = button {
                let down_button = button.clone();
                let down_session = session.clone();
                let down_transport = transport.clone();
                element = element
                    .on_key_down(move |event, _, cx| {
                        if event.keystroke.modifiers.modified() || event.is_held {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "enter" => emit_press(
                                &down_session,
                                &down_transport,
                                window,
                                id,
                                handler,
                                revision,
                            ),
                            "space" => down_button.space_down.set(true),
                            _ => return,
                        }
                        cx.stop_propagation();
                    })
                    .on_key_up(move |event, _, cx| {
                        if event.keystroke.key == "space" && button.space_down.replace(false) {
                            emit_press(&session, &transport, window, id, handler, revision);
                            cx.stop_propagation();
                        }
                    });
            }
            let session = self.session.clone();
            let transport = self.transport.clone();
            if interaction.pointer {
                if let Some(button) = self.buttons.get(&id) {
                    let focus = button.focus.clone();
                    element = element
                        .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                            window.focus(&focus, cx)
                        });
                }
                element = element.on_click(move |_, _, cx| {
                    let event = session.borrow().press(window, id, handler, revision);
                    if let Some(event) = event
                        && !transport.input(event)
                        && session.borrow_mut().overload(window)
                    {
                        transport.fault(window);
                    }
                    cx.stop_propagation();
                });
            }
        }
        #[cfg(feature = "native-tests")]
        {
            let probes = self.probes.clone();
            element = element.relative().child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds, window, _| {
                        probes.borrow_mut().insert(
                            id,
                            native_test::Probe {
                                bounds,
                                color: window.text_style().color,
                            },
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        }
        element.into_any_element()
    }
}
impl Render for View {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.visited.clear();
        for button in self.buttons.values() {
            if !button.focus.is_focused(window) {
                button.space_down.set(false);
            }
        }
        let shared = self.session.clone();
        let session = shared.borrow();
        let mut root = div().size_full().on_key_down(|event, window, cx| {
            if event.keystroke.key == "tab" {
                if event.keystroke.modifiers.shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                cx.stop_propagation();
            }
        });
        let revision = if let Some(tree) = session.tree(self.id) {
            if let Some(id) = tree.root() {
                root = root.child(self.element(tree, id, Interaction::default(), cx));
            }
            tree.revision()
        } else {
            0
        };
        self.buttons.retain(|id, _| self.visited.contains(id));
        self.selections.retain(|id, _| self.visited.contains(id));
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
        let exit_on_last_window = transport.exit_on_last_window;
        cx.on_window_closed(move |cx, _| {
            if exit_on_last_window && cx.windows().is_empty() && !closing.replace(true) {
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
                                        cx.new(|_| {
                                            View::new(id, session.clone(), transport.clone())
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

#[cfg(feature = "native-tests")]
#[path = "native_test.rs"]
pub(crate) mod native_test;
