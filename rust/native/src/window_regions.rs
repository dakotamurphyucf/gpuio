//! Native window gestures from admitted, live container regions.
use super::*;
use crate::tree::Tree;
use gpui::{Div, MouseButton, ResizeEdge, Stateful};
use gpuio_protocol::window_region::{Edge, Region};
use std::cell::Cell;

// Conservative per-region accounting for Arc config, state, observer and frame listeners.
pub(crate) const RESERVED_BYTES: usize = 2048;

pub(super) struct State {
    config: Arc<Region>,
    armed: Cell<bool>,
    _activation: gpui::Subscription,
}
impl State {
    pub(super) fn cancel(&self) {
        self.armed.set(false);
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Request {
    Move,
    DoubleClick,
    Resize(Edge),
    Menu(gpui::Point<gpui::Pixels>),
}
#[cfg(all(test, feature = "native-image-tests"))]
#[derive(Default)]
struct Recorded(Vec<Request>);
#[cfg(all(test, feature = "native-image-tests"))]
impl gpui::Global for Recorded {}
fn request(request: Request, window: &mut Window, cx: &mut App) {
    #[cfg(all(test, feature = "native-image-tests"))]
    if cx.has_global::<Recorded>() {
        cx.global_mut::<Recorded>().0.push(request);
        return;
    }
    let _ = cx;
    match request {
        Request::Move => window.start_window_move(),
        Request::DoubleClick => {
            if cfg!(target_os = "macos") {
                window.titlebar_double_click();
            } else if window.is_resizable() && window.window_controls().maximize {
                window.zoom_window();
            }
        }
        Request::Resize(edge) => window.start_window_resize(match edge {
            Edge::Top => ResizeEdge::Top,
            Edge::Bottom => ResizeEdge::Bottom,
            Edge::Left => ResizeEdge::Left,
            Edge::Right => ResizeEdge::Right,
            Edge::TopLeft => ResizeEdge::TopLeft,
            Edge::TopRight => ResizeEdge::TopRight,
            Edge::BottomLeft => ResizeEdge::BottomLeft,
            Edge::BottomRight => ResizeEdge::BottomRight,
        }),
        Request::Menu(position) => window.show_window_menu(position),
    }
}

pub(super) fn within_title_bar(tree: &Tree, id: NodeId, window: &Window) -> bool {
    let mut ancestor = tree.get(id).and_then(|node| node.parent);
    while let Some(id) = ancestor {
        let Some(node) = tree.get(id) else {
            return false;
        };
        if let Some(region) = &node.window_region
            && !(matches!(region.as_ref(), Region::Resize(_)) && !supports_resize(window))
        {
            return matches!(region.as_ref(), Region::TitleBar);
        }
        ancestor = node.parent;
    }
    false
}
fn eligible(view: &View, id: NodeId, state: &Rc<State>, window: &Window) -> bool {
    if !window.is_window_active()
        || window.captured_hitbox().is_some()
        || !view.focus.borrow().allows(id)
        || !view
            .window_regions
            .get(&id)
            .is_some_and(|current| Rc::ptr_eq(current, state))
    {
        return false;
    }
    let session = view.session.borrow();
    let Some(tree) = session.tree(view.id) else {
        return false;
    };
    tree.get(id)
        .and_then(|node| node.window_region.as_ref())
        .is_some_and(|config| Arc::ptr_eq(config, &state.config))
        && super::pointer_enabled(tree, id)
}

fn supports_resize(window: &Window) -> bool {
    // Pinned GPUI's macOS backend inherits the no-op PlatformWindow method;
    // AppKit owns resizing at the native frame. X11/Wayland implement this API.
    cfg!(target_os = "linux") && window.is_resizable()
}

impl View {
    pub(super) fn window_region_element(
        &mut self,
        mut element: Stateful<Div>,
        node: &crate::tree::Node,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(config) = &node.window_region else {
            return element;
        };
        if matches!(config.as_ref(), Region::Resize(_)) && !supports_resize(window) {
            if let Some(state) = self.window_regions.remove(&node.id) {
                state.cancel();
            }
            return element;
        }
        let id = node.id;
        self.visited.insert(id);
        if !self
            .window_regions
            .get(&id)
            .is_some_and(|state| Arc::ptr_eq(&state.config, config))
        {
            let state = Rc::new_cyclic(|weak: &std::rc::Weak<State>| {
                let weak = weak.clone();
                State {
                    config: config.clone(),
                    armed: Cell::new(false),
                    _activation: cx.observe_window_activation(window, move |_, window, _| {
                        if !window.is_window_active()
                            && let Some(state) = weak.upgrade()
                        {
                            state.cancel();
                        }
                    }),
                }
            });
            self.window_regions.insert(id, state);
        }
        let state = self.window_regions[&id].clone();
        if !eligible(self, id, &state, window) {
            state.cancel();
        }
        element = element.block_mouse_except_scroll();
        if let Region::Resize(edge) = config.as_ref()
            && supports_resize(window)
        {
            element = element.cursor(match edge {
                Edge::Top | Edge::Bottom => gpui::CursorStyle::ResizeUpDown,
                Edge::Left | Edge::Right => gpui::CursorStyle::ResizeLeftRight,
                Edge::TopLeft | Edge::BottomRight => gpui::CursorStyle::ResizeUpLeftDownRight,
                Edge::TopRight | Edge::BottomLeft => gpui::CursorStyle::ResizeUpRightDownLeft,
            });
        }
        if matches!(config.as_ref(), Region::Exclude) {
            return element;
        }
        let clear = state.clone();
        element = element.capture_any_mouse_down(move |_, _, _| clear.cancel());
        let down = state.clone();
        element = element.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, event: &gpui::MouseDownEvent, window, cx| {
                if !eligible(view, id, &down, window) {
                    down.cancel();
                    return;
                }
                match down.config.as_ref() {
                    Region::TitleBar if event.click_count == 2 => {
                        down.cancel();
                        request(Request::DoubleClick, window, cx);
                    }
                    Region::TitleBar => down.armed.set(event.click_count == 1),
                    Region::Resize(edge) if supports_resize(window) => {
                        request(Request::Resize(*edge), window, cx);
                    }
                    Region::Resize(_) | Region::Exclude => (),
                }
                cx.stop_propagation();
            }),
        );
        let moving = state.clone();
        element = element.on_mouse_move(cx.listener(
            move |view, event: &gpui::MouseMoveEvent, window, cx| {
                if event.pressed_button != Some(MouseButton::Left)
                    || !eligible(view, id, &moving, window)
                {
                    moving.cancel();
                    return;
                }
                if moving.armed.replace(false) {
                    request(Request::Move, window, cx);
                    cx.stop_propagation();
                }
            },
        ));
        let up = state.clone();
        element = element.on_mouse_up(MouseButton::Left, move |_, _, _| up.cancel());
        let outside = state.clone();
        element = element.on_mouse_up_out(MouseButton::Left, move |_, _, _| outside.cancel());
        let outside = state.clone();
        element = element.on_mouse_down_out(move |_, _, _| outside.cancel());
        if cfg!(target_os = "linux") && matches!(config.as_ref(), Region::TitleBar) {
            element = element.on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &gpui::MouseDownEvent, window, cx| {
                    if eligible(view, id, &state, window)
                        && matches!(
                            window.window_decorations(),
                            gpui::Decorations::Client { .. }
                        )
                        && window.window_controls().window_menu
                    {
                        request(Request::Menu(event.position), window, cx);
                        cx.stop_propagation();
                    }
                }),
            );
        }
        element
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "window_regions_test.rs"]
mod tests;
