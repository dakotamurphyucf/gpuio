//! Main-thread canvas input. Capture and preview state never cross the bridge.
use super::*;
use crate::canvas_state::{Navigation, PointerMode};
use gpui::{Hitbox, HitboxId, MouseButton, Pixels};
use gpuio_protocol::canvas_view::Viewport;

type Shared = Rc<RefCell<State>>;
pub(super) struct Input {
    pub focus: gpui::FocusHandle,
    gate: super::super::focus::Shared,
    bounds: Bounds<Pixels>,
    capture: Option<HitboxId>,
    button: Option<MouseButton>,
    pub(super) token: Rc<()>,
    pending: Option<(i64, i64, Viewport)>,
    blur: Option<gpui::Subscription>,
}
impl Input {
    pub(super) fn new(gate: super::super::focus::Shared, cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            gate,
            bounds: Bounds::default(),
            capture: None,
            button: None,
            token: Rc::new(()),
            pending: None,
            blur: None,
        }
    }
}
impl State {
    pub(in crate::host) fn canvas_focused(&self, window: &Window) -> bool {
        self.input.focus.is_focused(window)
    }
    pub(in crate::host) fn cancel_input(&mut self, window: &mut Window) {
        self.input.pending = None;
        self.cancel_gesture(window);
    }
    fn cancel_gesture(&mut self, window: &mut Window) {
        if let Some(hitbox) = self.input.capture.take()
            && window.captured_hitbox() == Some(hitbox)
        {
            window.release_pointer();
        }
        self.input.button = None;
        if let Some(native) = &mut self.native {
            native.cancel();
        }
    }
    pub(super) fn input_allowed(&self, window: &Window, pointer: bool) -> bool {
        !self.closed
            && !self.config.disabled
            && window.is_window_active()
            && self.ready.is_some()
            && self.input.gate.borrow().allows(self.node)
            && self
                .native
                .as_ref()
                .zip(self.lease.as_ref())
                .is_some_and(|(native, lease)| Arc::ptr_eq(native.snapshot(), &lease.snapshot()))
            && (!pointer
                || self.session.upgrade().is_some_and(|session| {
                    session
                        .borrow()
                        .tree(self.window)
                        .is_some_and(|tree| pointer_enabled(tree, self.node))
                }))
    }
    pub(in crate::host) fn flush_canvas_frame(&mut self, window: &Window, cx: &mut App) {
        if self.input.pending.is_none() {
            return;
        }
        if self.input_allowed(window, true) {
            self.flush_viewport(cx);
        } else {
            self.input.pending = None;
        }
    }
    pub(super) fn flush_viewport(&mut self, cx: &mut App) {
        if let Some((revision, generation, viewport)) = self.input.pending.take() {
            self.emit_at(
                vec![Observation::ViewportChanged(viewport)],
                revision,
                generation,
                cx,
            );
        }
    }
    pub(super) fn valid_callback(&self, token: &Rc<()>, window: &Window, pointer: bool) -> bool {
        Rc::ptr_eq(token, &self.input.token) && self.input_allowed(window, pointer)
    }
    fn local(&self, point: gpui::Point<Pixels>) -> Point {
        Point {
            x: f64::from(f32::from(point.x - self.input.bounds.origin.x)),
            y: f64::from(f32::from(point.y - self.input.bounds.origin.y)),
        }
    }
    pub(super) fn center(&self) -> Point {
        Point {
            x: f64::from(f32::from(self.input.bounds.size.width)) / 2.,
            y: f64::from(f32::from(self.input.bounds.size.height)) / 2.,
        }
    }
    pub(super) fn enable(&mut self) {
        if let Some(native) = &mut self.native {
            native.set_input_enabled(true);
        }
    }
    fn key(
        &mut self,
        token: &Rc<()>,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.valid_callback(token, window, false) || !self.input.focus.is_focused(window) {
            return;
        }
        self.flush_viewport(cx);
        let key = event.keystroke.key.as_str();
        if key == "tab" {
            self.cancel_input(window);
            return;
        }
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform {
            return;
        }
        self.enable();
        if key == "escape" {
            if self.input.capture.is_some() {
                self.cancel_input(window);
                window.refresh();
                cx.stop_propagation();
            }
            return;
        }
        let direction = match key {
            "left" => Some((-1., 0.)),
            "right" => Some((1., 0.)),
            "up" => Some((0., -1.)),
            "down" => Some((0., 1.)),
            _ => None,
        };
        let center = self.center();
        let native = self.native.as_mut().unwrap();
        let mut events = if let Some((x, y)) = direction {
            if modifiers.shift {
                let step = if modifiers.alt { 10. } else { 1. };
                native.move_selected(Point {
                    x: x * step,
                    y: y * step,
                })
            } else if modifiers.alt {
                native.pan_by(Point {
                    x: x * 20.,
                    y: y * 20.,
                })
            } else {
                native.navigate(if x + y < 0. {
                    Navigation::Previous
                } else {
                    Navigation::Next
                })
            }
        } else if !modifiers.alt {
            match key {
                "home" => native.navigate(Navigation::First),
                "end" => native.navigate(Navigation::Last),
                "enter" | "space" => native
                    .selection()
                    .map_or_else(Vec::new, |id| native.activate(id)),
                "+" | "=" => native.zoom_at(center, 1.2),
                "-" => native.zoom_at(center, 1. / 1.2),
                _ => return,
            }
        } else {
            return;
        };
        if !modifiers.shift
            && !modifiers.alt
            && (direction.is_some() || matches!(key, "home" | "end"))
            && let Some(id) = native.selection()
        {
            events.extend(super::accessibility::reveal(native, id, center));
        }
        // Native keyboard operations cancel previews too; release owned capture.
        self.cancel_input(window);
        self.emit(events, cx);
        window.refresh();
        window.prevent_default();
        cx.stop_propagation();
    }
}

pub(super) fn install_blur(state: &Shared, window: &mut Window, cx: &mut App) {
    let weak = Rc::downgrade(state);
    let focus = state.borrow().input.focus.clone();
    state.borrow_mut().input.blur = Some(window.on_focus_out(&focus, cx, move |_, window, cx| {
        if let Some(state) = weak.upgrade() {
            let mut state = state.borrow_mut();
            if state.input_allowed(window, false) {
                state.flush_viewport(cx);
            }
            state.cancel_input(window);
            window.refresh();
        }
    }));
}
pub(super) fn keyboard(
    element: gpui::Stateful<gpui::Div>,
    state: Shared,
) -> gpui::Stateful<gpui::Div> {
    let (focus, token) = {
        let mut state = state.borrow_mut();
        let eligible = !state.closed
            && !state.config.disabled
            && state.lease.is_some()
            && state.input.gate.borrow().allows(state.node);
        state.input.focus = state.input.focus.clone().tab_stop(eligible);
        (state.input.focus.clone(), state.input.token.clone())
    };
    let access = state.clone();
    let access_token = token.clone();
    element
        .track_focus(&focus)
        .on_key_down(move |event, window, cx| state.borrow_mut().key(&token, event, window, cx))
        .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
            let state = access.borrow();
            if state.valid_callback(&access_token, window, false) {
                window.focus(&state.input.focus, cx);
            }
        })
}
pub(super) fn prepaint(state: &Shared, bounds: Bounds<Pixels>, window: &mut Window) -> Hitbox {
    let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::BlockMouse);
    let mut state = state.borrow_mut();
    if state.input.bounds != bounds && state.input.capture.is_some() {
        state.cancel_input(window);
    }
    state.input.bounds = bounds;
    if let Some(old) = state.input.capture {
        if window.captured_hitbox() == Some(old) && state.input_allowed(window, true) {
            window.capture_pointer(hitbox.id);
            state.input.capture = Some(hitbox.id);
        } else {
            state.cancel_input(window);
        }
    }
    hitbox
}
pub(super) fn paint(state: &Shared, hitbox: Hitbox, window: &mut Window) {
    let (token, focus, gate, node, eligible, bounds) = {
        let state = state.borrow();
        (
            state.input.token.clone(),
            state.input.focus.clone(),
            state.input.gate.clone(),
            state.node,
            state.input_allowed(window, false),
            state.input.bounds,
        )
    };
    gate.borrow_mut().record(
        node,
        focus.clone(),
        eligible,
        focus.is_focused(window),
        bounds,
    );
    let down = state.clone();
    let down_token = token.clone();
    let down_hit = hitbox.clone();
    window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, window, cx| {
        if !phase.bubble()
            || !down_hit.is_hovered(window)
            || window.default_prevented()
            || window.captured_hitbox().is_some()
        {
            return;
        }
        let mut state = down.borrow_mut();
        if !state.valid_callback(&down_token, window, true) {
            return;
        }
        let mode = match event.button {
            MouseButton::Left => PointerMode::Select,
            MouseButton::Middle => PointerMode::Pan,
            _ => return,
        };
        if mode == PointerMode::Pan && !state.config.pan_zoom {
            return;
        }
        state.flush_viewport(cx);
        state.enable();
        let point = state.local(event.position);
        let native = state.native.as_mut().unwrap();
        let mut events = native.begin_pointer(point, mode);
        if mode == PointerMode::Select
            && event.click_count == 2
            && let Some(id) = native.hit_test(point)
        {
            events.extend(native.activate(id));
        }
        if native.has_gesture() {
            window.capture_pointer(down_hit.id);
            state.input.capture = Some(down_hit.id);
            state.input.button = Some(event.button);
        }
        window.focus(&state.input.focus, cx);
        state.emit(events, cx);
        window.refresh();
        window.prevent_default();
        cx.stop_propagation();
    });
    let moved = state.clone();
    let move_token = token.clone();
    window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
        if !phase.capture() {
            return;
        }
        let mut state = moved.borrow_mut();
        let Some(capture) = state.input.capture else {
            return;
        };
        if !state.valid_callback(&move_token, window, true)
            || window.captured_hitbox() != Some(capture)
            || event.pressed_button != state.input.button
        {
            state.cancel_input(window);
            window.refresh();
            return;
        }
        let point = state.local(event.position);
        if state.native.as_mut().unwrap().move_pointer(point) {
            window.refresh();
        }
        cx.stop_propagation();
    });
    let up = state.clone();
    let up_token = token.clone();
    window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
        if !phase.bubble() {
            return;
        }
        let mut state = up.borrow_mut();
        let Some(capture) = state.input.capture else {
            return;
        };
        if !state.valid_callback(&up_token, window, true)
            || window.captured_hitbox() != Some(capture)
            || Some(event.button) != state.input.button
        {
            state.cancel_input(window);
            window.refresh();
            return;
        }
        let point = state.local(event.position);
        let native = state.native.as_mut().unwrap();
        native.move_pointer(point);
        let events = native.finish_pointer();
        state.cancel_input(window);
        state.emit(events, cx);
        window.refresh();
        cx.stop_propagation();
    });
    let wheel = state.clone();
    window.on_mouse_event(move |event: &gpui::ScrollWheelEvent, phase, window, cx| {
        if !phase.bubble() || !hitbox.should_handle_scroll(window) {
            return;
        }
        let mut state = wheel.borrow_mut();
        if !state.valid_callback(&token, window, true) || !state.config.pan_zoom {
            return;
        }
        let point = state.local(event.position);
        let delta = event.delta.pixel_delta(px(16.));
        state.enable();
        // Wheel supersedes a pointer gesture, without losing earlier wheel samples.
        if state.input.capture.is_some() {
            state.cancel_input(window);
        }
        let native = state.native.as_mut().unwrap();
        let events = if event.modifiers.control || event.modifiers.platform {
            native.zoom_at(
                point,
                (f64::from(f32::from(delta.y)) * 0.01).clamp(-4., 4.).exp(),
            )
        } else {
            native.pan_by(Point {
                x: f64::from(f32::from(delta.x)),
                y: f64::from(f32::from(delta.y)),
            })
        };
        if let Some(Observation::ViewportChanged(viewport)) = events.last() {
            state.input.pending = Some((
                native.snapshot().revision,
                native.snapshot().generation,
                *viewport,
            ));
        }
        window.refresh();
        window.prevent_default();
        cx.stop_propagation();
    });
}

/// One decorative outline, bounded by the selected item's (at most 256-point)
/// hit-region bounds. Preserve its affine transform and world clip stack.
pub(super) fn selection(native: &canvas_state::State, bounds: Bounds<Pixels>, window: &mut Window) {
    let Some(item) = native.selection().and_then(|id| native.item(id)) else {
        return;
    };
    let Some(interaction) = &item.interaction else {
        return;
    };
    let rect = hit_bounds(&interaction.hit_region);
    let placement = Placement {
        origin: Point { x: 0., y: 0. },
        transform: native.transform(item),
        viewport: native.viewport(),
        bounds,
        clips: &item.clips,
    };
    let Ok(Some(clip)) = placement.clip() else {
        return;
    };
    let mut path = gpui::PathBuilder::stroke(px(1.5));
    for (index, point) in [
        Point {
            x: rect.x,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
        Point {
            x: rect.x,
            y: rect.y + rect.height,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let point = placement.project_local(point);
        if index == 0 {
            path.move_to(point);
        } else {
            path.line_to(point);
        }
    }
    path.close();
    if let Ok(path) = path.build() {
        window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
            window.paint_path(path, rgba(native.config().selection_color as u32))
        });
    }
}

pub(super) fn hit_bounds(hit: &gpuio_protocol::canvas::HitRegion) -> gpuio_protocol::canvas::Rect {
    use gpuio_protocol::canvas::{HitRegion, Rect};
    match hit {
        HitRegion::Rectangle(rect) | HitRegion::Ellipse(rect) => *rect,
        HitRegion::Polygon(points) => {
            let mut lo = points[0];
            let mut hi = lo;
            for point in points {
                lo.x = lo.x.min(point.x);
                lo.y = lo.y.min(point.y);
                hi.x = hi.x.max(point.x);
                hi.y = hi.y.max(point.y);
            }
            Rect {
                x: lo.x,
                y: lo.y,
                width: hi.x - lo.x,
                height: hi.y - lo.y,
            }
        }
    }
}
