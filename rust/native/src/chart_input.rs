//! UI-owned chart interaction. Native preview is local; only user commits cross
//! the asynchronous bridge, using the exact currently published prepared data.
use super::*;
use crate::chart_geometry::Point;
use gpui::{Hitbox, HitboxId, MouseButton, Pixels};
use gpuio_protocol::chart_selection::Selection;
#[path = "chart_inspection_view.rs"]
mod inspection;
type Shared = Rc<RefCell<State>>;
#[path = "chart_data_view.rs"]
mod data_view;
#[cfg(feature = "native-canvas-tests")]
pub(super) use data_view::capture_browse;
pub(super) use data_view::element as data_element;
pub(super) struct Input {
    pub focus: gpui::FocusHandle,
    pub(super) gate: crate::host::focus::Shared,
    bounds: Bounds<Pixels>,
    capture: Option<HitboxId>,
    pub token: Rc<()>,
    pub selected: Option<Selection>,
    pub selected_index: Option<usize>,
    hover: Option<usize>,
    pub(super) pointer: Option<Point>,
    cursor: Option<usize>,
    pub data_cursor: Option<usize>,
    blur: Option<gpui::Subscription>,
}
impl Input {
    pub(super) fn preview_index(&self) -> Option<usize> {
        self.hover.or(self.cursor).or(self.selected_index)
    }
    pub fn new(gate: crate::host::focus::Shared, cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            gate,
            bounds: Bounds::default(),
            capture: None,
            token: Rc::new(()),
            selected: None,
            selected_index: None,
            hover: None,
            pointer: None,
            cursor: None,
            data_cursor: None,
            blur: None,
        }
    }
    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.selected_index = None;
        self.cursor = None;
        self.pointer = None;
    }
}
impl State {
    #[cfg(all(target_os = "macos", feature = "native-tests"))]
    pub(in crate::host) fn pointer_ready(&self, window: &Window) -> bool {
        self.input_allowed(window, true)
    }
    pub(in crate::host) fn chart_focused(&self, window: &Window) -> bool {
        self.input.focus.is_focused(window)
    }
    pub(super) fn cancel_input(&mut self, window: &mut Window) {
        if let Some(capture) = self.input.capture.take()
            && window.captured_hitbox() == Some(capture)
        {
            window.release_pointer();
        }
        self.input.hover = None;
        self.input.pointer = None;
        self.input.cursor = None;
    }
    fn base_input_allowed(&self, window: &Window, pointer: bool) -> bool {
        !self.closed
            && !self.config.disabled
            && window.is_window_active()
            && self.input.gate.borrow().allows(self.node)
            && (!pointer
                || self.session.upgrade().is_some_and(|session| {
                    session
                        .borrow()
                        .tree(self.window)
                        .is_some_and(|tree| pointer_enabled(tree, self.node))
                }))
    }
    fn input_allowed(&self, window: &Window, pointer: bool) -> bool {
        self.base_input_allowed(window, pointer)
            && self.input.data_cursor.is_none()
            && self.ready.as_ref().is_some_and(|ready| {
                self.lease
                    .as_ref()
                    .and_then(Lease::snapshot)
                    .is_some_and(|source| Arc::ptr_eq(&ready.snapshot, &source))
                    && ready.config.options == self.config.options
                    && ready.config.sampling == self.config.sampling
                    && ready.config.style == self.config.style
                    && ready.config.legend == self.config.legend
                    && self.requested_frame == self.ready_frame
            })
    }
    fn valid_callback(&self, token: &Rc<()>, window: &Window, pointer: bool) -> bool {
        Rc::ptr_eq(token, &self.input.token) && self.input_allowed(window, pointer)
    }
    fn local(&self, point: gpui::Point<Pixels>) -> Point {
        let frame = self.ready_frame.expect("input requires ready frame");
        Point {
            x: f64::from(f32::from(point.x - self.input.bounds.origin.x)) - frame.plot.x,
            y: f64::from(f32::from(point.y - self.input.bounds.origin.y)) - frame.plot.y,
        }
    }
    fn target(&self, point: gpui::Point<Pixels>) -> Option<usize> {
        self.ready.as_ref()?.plan.hit_index(self.local(point))
    }
    fn within_plot(&self, point: gpui::Point<Pixels>) -> bool {
        let p = self.local(point);
        let frame = self.ready_frame.unwrap();
        p.x >= 0. && p.y >= 0. && p.x <= frame.plot.width && p.y <= frame.plot.height
    }
    pub(super) fn redraw(&self, window: &Window, cx: &mut App) {
        let visibility_changed = self.sync_label_visibility();
        let handle = window.window_handle();
        let node = self.node;
        cx.defer(move |cx| {
            let _ = handle.update(cx, |root, window, cx| {
                if let Ok(view) = root.downcast::<View>() {
                    view.update(cx, |view, cx| {
                        if visibility_changed {
                            view.sync_tooltips(window, cx);
                        }
                        view.invalidate_resource_row(node);
                        cx.notify();
                    });
                }
                window.refresh();
            });
        });
    }
    fn commit(&mut self, index: Option<usize>, cx: &mut App) {
        let Some(ready) = &self.ready else { return };
        let selection = index
            .and_then(|index| ready.plan.geometry().marks.get(index))
            .and_then(|mark| {
                crate::chart_selection::resolve(
                    ready.snapshot.data(),
                    &ready.config.sampling,
                    mark.source,
                )
            });
        self.input.selected = selection;
        self.input.selected_index = index.filter(|_| selection.is_some());
        self.input.cursor = self.input.selected_index;
        self.emit(
            ready.snapshot.revision(),
            ready.snapshot.generation(),
            Observation::SelectionChanged(selection),
            cx,
        );
    }
    fn key(
        &mut self,
        token: &Rc<()>,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !Rc::ptr_eq(token, &self.input.token) || !self.input.focus.is_focused(window) {
            return;
        }
        let key = event.keystroke.key.as_str();
        if key == "tab" {
            if self.inspection_position().is_some() {
                self.cancel_capture(window);
            } else {
                self.cancel_input(window);
            }
            self.redraw(window, cx);
            return;
        }
        let mods = event.keystroke.modifiers;
        if mods.control || mods.platform || mods.alt {
            return;
        }
        if self.data_key(key, window, cx) {
            return;
        }
        if !self.valid_callback(token, window, false) {
            return;
        }
        if key == "escape" {
            if self.input.capture.is_some() {
                self.cancel_input(window);
            } else {
                self.input.hover = None;
                self.commit(None, cx);
            }
        } else {
            let count = self.ready.as_ref().unwrap().plan.geometry().marks.len();
            if count == 0 {
                return;
            }
            let previous = self.input.cursor.or(self.input.selected_index);
            let next = match key {
                "left" | "up" => Some(previous.map_or(count - 1, |i| i.saturating_sub(1))),
                "right" | "down" => Some(previous.map_or(0, |i| (i + 1).min(count - 1))),
                "home" => Some(0),
                "end" => Some(count - 1),
                "enter" | "space" => {
                    self.cancel_capture(window);
                    self.commit(previous.or(Some(0)), cx);
                    None
                }
                _ => return,
            };
            if let Some(next) = next {
                self.cancel_capture(window);
                self.input.hover = None;
                self.input.cursor = Some(next);
            }
        }
        self.input.pointer = None;
        self.input.hover = None;
        // An explicit command on the chart supersedes an older card's pointer
        // retention. Keys dispatched to a child never reach this branch.
        self.content.pointer_inside.set(false);
        self.redraw(window, cx);
        window.prevent_default();
        cx.stop_propagation();
    }
    pub(super) fn cancel_capture(&mut self, window: &mut Window) {
        if let Some(capture) = self.input.capture.take()
            && window.captured_hitbox() == Some(capture)
        {
            window.release_pointer();
        }
    }
    pub(super) fn input_overlay(
        &self,
        custom: Option<(super::inspection_content::Position, gpui::AnyElement)>,
    ) -> Option<gpui::AnyElement> {
        if self.closed || self.config.disabled || self.input.data_cursor.is_some() {
            return None;
        }
        let ready = self.ready.as_ref()?;
        let frame = self.ready_frame?;
        let index = custom
            .as_ref()
            .map(|(position, _)| position.index)
            .or(self.input.preview_index())?;
        let details = crate::chart_details::describe_with_radar_labels(
            ready.snapshot.data(),
            &ready.config.sampling,
            &ready.config.options,
            ready.plan.geometry(),
            index,
            &self.config.radar_labels,
        )?;
        let selected = self.input.capture.is_none() && self.input.selected_index == Some(index);
        let pointer = custom
            .as_ref()
            .and_then(|(position, _)| position.pointer)
            .or(self.input.hover.and(self.input.pointer));
        let custom = custom.map(|(position, element)| inspection::Custom {
            element,
            container: position.container,
            bounds: position.bounds,
            focus: self.content.focus.clone(),
        });
        Some(inspection::overlay(
            ready, frame, details, selected, pointer, custom,
        ))
    }
}
pub(super) fn install_blur(state: &Shared, window: &mut Window, cx: &mut App) {
    let weak = Rc::downgrade(state);
    let focus = state.borrow().input.focus.clone();
    state.borrow_mut().input.blur = Some(window.on_focus_out(&focus, cx, move |_, window, cx| {
        if let Some(state) = weak.upgrade() {
            let mut state = state.borrow_mut();
            state.cancel_input(window);
            state.redraw(window, cx);
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
            && state.lease.as_ref().and_then(Lease::snapshot).is_some()
            && state.input.gate.borrow().allows(state.node);
        state.input.focus = state.input.focus.clone().tab_stop(eligible);
        (state.input.focus.clone(), state.input.token.clone())
    };
    let access = state.clone();
    let access_token = token.clone();
    element
        .track_focus(&focus)
        .aria_description("Arrows browse plotted values; Enter or Space commits. D opens original data, including missing and unpainted values. Escape cancels a drag or clears selection.")
        .on_key_down(move |event, window, cx| {
            let gate = state.borrow().input.gate.clone();
            let visibility = gate.borrow().visibility_identity();
            state.borrow_mut().key(&token, event, window, cx);
            let changed = !Rc::ptr_eq(&visibility, &gate.borrow().visibility_identity());
            if changed {
                sync_label_inputs(window, cx);
            }
        })
        .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
            let state = access.borrow();
            if Rc::ptr_eq(&state.input.token, &access_token)
                && state.base_input_allowed(window, false)
            {
                window.focus(&state.input.focus, cx);
            }
        })
}

/// Run after releasing chart state: retiring focus/menu owners can invoke native
/// focus callbacks. This must not wait for a frame in an occluded window.
fn sync_label_inputs(window: &mut Window, cx: &mut App) {
    if let Some(view) = window.root::<View>().flatten() {
        view.update(cx, |view, cx| view.sync_tooltips(window, cx));
    }
}
pub(super) fn prepaint(
    state: &Shared,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) -> Option<Hitbox> {
    let mut state = state.borrow_mut();
    if state.input.bounds != bounds {
        state.cancel_input(window);
    }
    state.input.bounds = bounds;
    let frame = state.ready_frame?;
    let plot = Bounds::new(
        bounds.origin + gpui::point(px(frame.plot.x as f32), px(frame.plot.y as f32)),
        gpui::size(px(frame.plot.width as f32), px(frame.plot.height as f32)),
    );
    // Selection owns pointer input, but charts have no wheel gesture. Let the
    // enclosing scroll view receive wheel events even while inspecting a mark.
    let hitbox = window.insert_hitbox(plot, gpui::HitboxBehavior::BlockMouseExceptScroll);
    if let Some(old) = state.input.capture {
        if window.captured_hitbox() == Some(old) && state.input_allowed(window, true) {
            window.capture_pointer(hitbox.id);
            state.input.capture = Some(hitbox.id);
        } else {
            state.cancel_input(window);
        }
    }
    Some(hitbox)
}
pub(super) fn paint(state: &Shared, hitbox: Option<Hitbox>, window: &mut Window) {
    let (token, focus, gate, node, eligible, bounds) = {
        let state = state.borrow();
        (
            state.input.token.clone(),
            state.input.focus.clone(),
            state.input.gate.clone(),
            state.node,
            state.base_input_allowed(window, false)
                && state.lease.as_ref().and_then(Lease::snapshot).is_some(),
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
    let Some(hitbox) = hitbox else {
        return;
    };
    let down = state.clone();
    let down_token = token.clone();
    let down_hit = hitbox.clone();
    window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, window, cx| {
        if !phase.bubble()
            || event.button != MouseButton::Left
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
        state.input.hover = state.target(event.position);
        state.input.pointer = state.input.hover.map(|_| state.local(event.position));
        state.input.cursor = None;
        state.input.capture = Some(down_hit.id);
        window.capture_pointer(down_hit.id);
        window.focus(&state.input.focus, cx);
        state.redraw(window, cx);
        window.prevent_default();
        cx.stop_propagation();
    });
    let moved = state.clone();
    let move_token = token.clone();
    let move_hit = hitbox;
    window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
        if !phase.capture() {
            return;
        }
        let mut state = moved.borrow_mut();
        if !state.valid_callback(&move_token, window, true) {
            if state.input.capture.is_some() || state.input.hover.is_some() {
                state.cancel_input(window);
                state.redraw(window, cx);
            }
            return;
        }
        let captured = state.input.capture;
        let was_inside_content = state.content.pointer_inside.get();
        if captured.is_none() && state.hold_inspection(event.position, window, cx) {
            return;
        }
        if captured.is_some_and(|capture| {
            window.captured_hitbox() != Some(capture)
                || event.pressed_button != Some(MouseButton::Left)
        }) {
            state.cancel_input(window);
            state.redraw(window, cx);
            return;
        }
        let next = if captured.is_some() || move_hit.is_hovered(window) {
            state.target(event.position)
        } else {
            None
        };
        let pointer = next.map(|_| state.local(event.position));
        let follows = state.config.style.inspection.card.visible
            && state.config.style.inspection.card.placement
                == gpuio_protocol::chart_inspection::Placement::Cursor;
        let moved_card = follows && pointer != state.input.pointer;
        state.input.pointer = pointer;
        if next != state.input.hover || moved_card || was_inside_content {
            state.input.hover = next;
            state.input.cursor = None;
            state.redraw(window, cx);
        }
        if captured.is_some() {
            cx.stop_propagation();
        }
    });
    let up = state.clone();
    window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
        if !phase.bubble() {
            return;
        }
        let mut state = up.borrow_mut();
        let Some(capture) = state.input.capture else {
            if state.content.pointer_inside.replace(false) {
                if !state.hold_inspection(event.position, window, cx) {
                    state.input.hover = None;
                    state.input.pointer = None;
                }
                state.redraw(window, cx);
            }
            return;
        };
        let commit = state.valid_callback(&token, window, true)
            && window.captured_hitbox() == Some(capture)
            && event.button == MouseButton::Left
            && state.within_plot(event.position);
        let target = if commit {
            state.target(event.position)
        } else {
            None
        };
        state.cancel_capture(window);
        if commit {
            state.input.hover = target;
            state.input.pointer = target.map(|_| state.local(event.position));
            state.commit(target, cx);
        } else {
            state.input.hover = None;
            state.input.pointer = None;
        }
        state.redraw(window, cx);
        cx.stop_propagation();
    });
}
impl View {
    pub(in crate::host) fn cancel_chart_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        for (id, state) in &self.charts {
            state.borrow_mut().cancel_input(window);
            self.invalidate_resource_row(*id);
        }
        cx.notify();
    }
}
