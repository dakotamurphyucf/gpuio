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
pub(super) fn presentation(window: &Window) -> wire::Presentation {
    let controls = window.window_controls();
    wire::Presentation {
        decorations: match window.window_decorations() {
            gpui::Decorations::Server => wire::Decorations::Server,
            gpui::Decorations::Client { tiling } => wire::Decorations::Client(wire::Tiling {
                top: tiling.top,
                right: tiling.right,
                bottom: tiling.bottom,
                left: tiling.left,
            }),
        },
        controls: wire::Controls {
            fullscreen: controls.fullscreen,
            maximize: controls.maximize && window.is_resizable(),
            minimize: controls.minimize && window.is_minimizable(),
            window_menu: controls.window_menu,
        },
        resizable: window.is_resizable(),
    }
}
/// Compare only presentation metadata here, not the whole snapshot on every paint.
/// Bounds/activation already have observers; capability changes need this comparison.
pub(super) fn observe_presentation(view: &mut View, window: &Window) {
    let current = presentation(window);
    if view
        .last_presentation
        .replace(current)
        .is_some_and(|old| old != current)
    {
        observe(view, window);
    }
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
        presentation: presentation(window),
        appearance: match window.appearance() {
            gpui::WindowAppearance::Light => wire::Appearance::Light,
            gpui::WindowAppearance::VibrantLight => wire::Appearance::VibrantLight,
            gpui::WindowAppearance::Dark => wire::Appearance::Dark,
            gpui::WindowAppearance::VibrantDark => wire::Appearance::VibrantDark,
        },
    }
}
pub(super) fn observe(view: &mut View, window: &Window) {
    let snapshot = snapshot(view, window);
    view.last_presentation = Some(snapshot.presentation);
    control(&view.transport, Event::WindowChanged(view.id, snapshot));
}
pub(super) fn command(
    view: &mut View,
    command: &wire::Command,
    window: &mut Window,
) -> wire::Response {
    if !command.is_valid() {
        return wire::Response::Failed(wire::Error::InvalidRequest);
    }
    let controls = presentation(window).controls;
    if matches!(command, wire::Command::Minimize) && !controls.minimize
        || matches!(command, wire::Command::Zoom) && !controls.maximize
        || matches!(command, wire::Command::ToggleFullscreen) && !controls.fullscreen
    {
        return wire::Response::Failed(wire::Error::Unsupported);
    }
    match command {
        wire::Command::FocusedInput
        | wire::Command::HasTextSelection
        | wire::Command::SelectedText(_)
        | wire::Command::ClearTextSelection
        | wire::Command::EndTextSelection => {
            return wire::Response::Failed(wire::Error::InvalidRequest);
        }
        wire::Command::Observe => (),
        wire::Command::SetTitle(title) => {
            window.set_window_title(title);
            view.window_title.clone_from(title);
        }
        wire::Command::Resize(w, h) => window.resize(size(px(*w as f32), px(*h as f32))),
        wire::Command::Activate => window.activate_window(),
        wire::Command::Zoom => window.zoom_window(),
        wire::Command::Minimize => window.minimize_window(),
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

/// All selection callbacks run on the native thread, outside retained-tree borrows.
pub(super) fn request(
    view: &mut View,
    command: &wire::Command,
    window: &mut Window,
    cx: &mut App,
) -> wire::Response {
    use gpui_base::TextSelection;
    if !command.is_valid() {
        return wire::Response::Failed(wire::Error::InvalidRequest);
    }
    match command {
        wire::Command::FocusedInput => {
            wire::Response::FocusedInput(focused_input(view, window, cx))
        }
        wire::Command::HasTextSelection => {
            wire::Response::SelectionPresent(TextSelection::has_selection(window, cx))
        }
        wire::Command::SelectedText(maximum) => {
            match TextSelection::selected_text_limited(window, *maximum as usize, cx) {
                Ok(text) => wire::Response::SelectedText(text),
                Err(_) => wire::Response::Failed(wire::Error::LimitExceeded),
            }
        }
        wire::Command::ClearTextSelection => {
            TextSelection::clear(window, cx);
            wire::Response::SelectionUpdated
        }
        wire::Command::EndTextSelection => {
            TextSelection::end(window, cx);
            wire::Response::SelectionUpdated
        }
        _ => self::command(view, command, window),
    }
}
/// Queries and selection mutations do not change native window geometry metadata.
pub(super) fn observes_window(command: &wire::Command) -> bool {
    !matches!(
        command,
        wire::Command::FocusedInput
            | wire::Command::HasTextSelection
            | wire::Command::SelectedText(_)
            | wire::Command::ClearTextSelection
            | wire::Command::EndTextSelection
    )
}

pub(super) fn focused_input(view: &View, window: &Window, cx: &App) -> Option<wire::Input> {
    use wire::InputKind;
    let handle = window.focused(cx)?;
    let gate = view.focus.borrow();
    let node = gate.focused_node(window, cx)?;
    if !gate.can_focus(&handle, window) || !gate.allows(node) {
        return None;
    }
    let session = view.session.borrow();
    let tree = session.tree(view.id)?;
    let owner = tree.get(node)?;
    let kind = match owner.kind {
        Kind::Input | Kind::Textarea | Kind::Combobox => {
            let editor = view.editors.get(&node)?;
            if !editor.focus_handle(cx).is_focused(window) {
                return None;
            }
            match owner.kind {
                Kind::Input => InputKind::Input,
                Kind::Textarea => InputKind::Textarea,
                _ => InputKind::Combobox,
            }
        }
        Kind::OtpInput if view.otps.get(&node)?.focus_handle(cx).is_focused(window) => {
            InputKind::Otp
        }
        Kind::NumberInput if view.numbers.get(&node)?.focus_handle(cx).is_focused(window) => {
            InputKind::Number
        }
        Kind::ColorInput if view.color_inputs.get(&node)?.text_focused(window, cx) => {
            InputKind::Color
        }
        Kind::CommandPalette => {
            let palette = view.palettes.get(&node)?;
            if palette.closed || !palette.query.read(cx).focus_handle(cx).is_focused(window) {
                return None;
            }
            InputKind::CommandPalette
        }
        _ => return None,
    };
    Some(wire::Input { node, kind })
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

pub(super) fn watch(view: &mut View, window: &mut Window, cx: &mut Context<View>) {
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
    watch_appearance(view, window, cx);
    observe(view, window);
}

fn watch_appearance(view: &mut View, window: &mut Window, cx: &mut Context<View>) {
    let owner = cx.entity().downgrade();
    view.appearance_subscription = Some(window.observe_window_appearance(move |window, cx| {
        let _ = owner.update(cx, |view, _| observe(view, window));
    }));
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "window_presentation_test.rs"]
mod tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "window_input_query_test.rs"]
mod input_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "window_selection_query_test.rs"]
mod selection_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "window_appearance_test.rs"]
mod appearance_tests;
