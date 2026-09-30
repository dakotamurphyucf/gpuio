//! Main-thread window operations and coalesced observations.
use super::*;
use gpuio_protocol::window as wire;
pub(super) fn control(transport: &Transport, event: Event) {
    transport
        .mailbox
        .lock()
        .expect("mailbox poisoned")
        .control(event);
    transport.wake_ocaml();
}
pub(super) fn snapshot(view: &View, window: &Window) -> wire::Snapshot {
    let bounds = window.bounds();
    wire::Snapshot {
        // GPUI Linux uses the default empty get_title; retain the host-owned title.
        title: view.window_title.clone(),
        x: f32::from(bounds.origin.x) as f64,
        y: f32::from(bounds.origin.y) as f64,
        width: f32::from(bounds.size.width) as f64,
        height: f32::from(bounds.size.height) as f64,
        content_width: f32::from(window.viewport_size().width) as f64,
        content_height: f32::from(window.viewport_size().height) as f64,
        active: window.is_window_active(),
        fullscreen: window.is_fullscreen(),
        maximized: window.is_maximized(),
        document: document(window),
    }
}
pub(super) fn observe(view: &View, window: &Window) {
    control(
        &view.transport,
        Event::WindowChanged(view.id, snapshot(view, window)),
    );
}
pub(super) fn command(
    view: &mut View,
    command: &wire::Command,
    window: &mut Window,
) -> wire::Response {
    if !command.is_valid() {
        return wire::Response::Failed(wire::Error::InvalidRequest);
    }
    match command {
        wire::Command::Observe => (),
        wire::Command::SetTitle(title) => {
            window.set_window_title(title);
            view.window_title.clone_from(title);
        }
        wire::Command::Resize(w, h) => window.resize(size(px(*w as f32), px(*h as f32))),
        wire::Command::Activate => window.activate_window(),
        wire::Command::Zoom => window.zoom_window(),
        wire::Command::ToggleFullscreen => window.toggle_fullscreen(),
        wire::Command::SetEdited(edited) => {
            if let Err(error) = set_edited(window, *edited) {
                return wire::Response::Failed(error);
            }
        }
        wire::Command::SetDocument(document) => {
            if let Err(error) = set_document(window, document) {
                return wire::Response::Failed(error);
            }
        }
    }
    wire::Response::Observed(snapshot(view, window))
}

#[cfg(target_os = "macos")]
use super::window_macos::{document, set_document, set_edited};

#[cfg(not(target_os = "macos"))]
fn document(_: &Window) -> Option<wire::Document> {
    None
}
#[cfg(not(target_os = "macos"))]
fn set_document(_: &Window, _: &wire::Document) -> Result<(), wire::Error> {
    Err(wire::Error::Unsupported)
}
#[cfg(not(target_os = "macos"))]
fn set_edited(_: &Window, _: bool) -> Result<(), wire::Error> {
    Err(wire::Error::Unsupported)
}
pub(super) fn capabilities() -> wire::Capabilities {
    #[cfg(target_os = "macos")]
    let backend = wire::Backend::Macos;
    #[cfg(not(target_os = "macos"))]
    let backend = if gpui::guess_compositor() == "Wayland" {
        wire::Backend::Wayland
    } else {
        wire::Backend::X11
    };
    wire::Capabilities {
        backend,
        native_quit_decision: cfg!(target_os = "macos"),
    }
}

pub(super) fn watch(view: &View, window: &mut Window, cx: &mut Context<View>) {
    #[cfg(target_os = "macos")]
    super::window_macos::install_text_input_reset(window);
    let transport = view.transport.clone();
    let id = view.id;
    window.on_window_should_close(cx, move |_, _| {
        control(&transport, Event::CloseRequested(id));
        false
    });
    cx.observe_window_bounds(window, |view, window, _| observe(view, window))
        .detach();
    cx.observe_window_activation(window, |view, window, cx| {
        if !window.is_window_active() {
            view.cancel_chart_input(window, cx);
        }
        observe(view, window);
    })
    .detach();
    observe(view, window);
}
