//! UI-owned chart interaction. Native preview is local; only user commits cross
//! the asynchronous bridge, using the exact currently published prepared data.
use super::*;
use crate::chart_geometry::Point;
use gpui::{Hitbox, HitboxId, MouseButton, Pixels};
use gpuio_protocol::chart_selection::Selection;
type Shared = Rc<RefCell<State>>;
#[path = "chart_data_view.rs"]
mod data_view;
#[cfg(feature = "native-canvas-tests")]
pub(super) use data_view::capture_browse;
pub(super) use data_view::element as data_element;
pub(super) struct Input {
    pub focus: gpui::FocusHandle,
    gate: crate::host::focus::Shared,
    bounds: Bounds<Pixels>,
    capture: Option<HitboxId>,
    pub token: Rc<()>,
    pub selected: Option<Selection>,
    pub selected_index: Option<usize>,
    hover: Option<usize>,
    cursor: Option<usize>,
    pub data_cursor: Option<usize>,
    blur: Option<gpui::Subscription>,
}
impl Input {
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
            cursor: None,
            data_cursor: None,
            blur: None,
        }
    }
    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.selected_index = None;
        self.cursor = None;
    }
}
impl State {
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
    fn redraw(&self, window: &Window, cx: &mut App) {
        let handle = window.window_handle();
        let node = self.node;
        cx.defer(move |cx| {
            let _ = handle.update(cx, |root, window, cx| {
                if let Ok(view) = root.downcast::<View>() {
                    view.update(cx, |view, cx| {
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
            self.cancel_input(window);
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
        self.redraw(window, cx);
        window.prevent_default();
        cx.stop_propagation();
    }
    fn cancel_capture(&mut self, window: &mut Window) {
        if let Some(capture) = self.input.capture.take()
            && window.captured_hitbox() == Some(capture)
        {
            window.release_pointer();
        }
    }
    pub(super) fn input_overlay(&self) -> Option<gpui::AnyElement> {
        if self.closed || self.config.disabled || self.input.data_cursor.is_some() {
            return None;
        }
        let ready = self.ready.as_ref()?;
        let frame = self.ready_frame?;
        let index = self
            .input
            .hover
            .or(self.input.cursor)
            .or(self.input.selected_index)?;
        let details = crate::chart_details::describe(
            ready.snapshot.data(),
            &ready.config.sampling,
            &ready.config.options,
            ready.plan.geometry(),
            index,
        )?;
        let selected = self.input.capture.is_none() && self.input.selected_index == Some(index);
        let color = ready.config.style.selection_color as u32;
        let backing = presentation::label_backing(color);
        let text_color = ready.config.style.label_color as u32;
        let card = div()
            .id("gpuio-chart-details")
            .role(gpui::Role::Group)
            .aria_label(format!("{}: {}", details.title, details.text))
            .absolute()
            .top(px(40.))
            .right(px(8.))
            .w(px((frame.width - 16.).clamp(0., 280.) as f32))
            .max_h(px((frame.legend.y - 48.).max(0.) as f32))
            .overflow_hidden()
            .p_2()
            .rounded_md()
            .bg(gpui::rgba(presentation::label_backing(text_color)))
            .text_color(gpui::rgba(text_color))
            .text_size(px(12.))
            .line_height(px(17.))
            .child(
                div()
                    .id("gpuio-chart-detail-title")
                    .role(gpui::Role::Label)
                    .aria_label(details.title.clone())
                    .text_ellipsis()
                    .child(details.title),
            )
            .child(
                div()
                    .id("gpuio-chart-detail-values")
                    .role(gpui::Role::Label)
                    .aria_label(details.text.clone())
                    .child(details.text),
            );
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(
                    div()
                        .absolute()
                        .left(px((frame.plot.x + details.anchor.x - 8.) as f32))
                        .top(px((frame.plot.y + details.anchor.y - 8.) as f32))
                        .size(px(16.))
                        .rounded_full()
                        .bg(gpui::rgba(color))
                        .text_color(gpui::rgba(backing))
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_center()
                        .child(if selected { "✓" } else { "○" }),
                )
                .child(card)
                .into_any_element(),
        )
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
        .on_key_down(move |event, window, cx| state.borrow_mut().key(&token, event, window, cx))
        .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
            let state = access.borrow();
            if Rc::ptr_eq(&state.input.token, &access_token)
                && state.base_input_allowed(window, false)
            {
                window.focus(&state.input.focus, cx);
            }
        })
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
    let hitbox = window.insert_hitbox(plot, gpui::HitboxBehavior::BlockMouse);
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
    let (token, focus, gate, node, eligible) = {
        let state = state.borrow();
        (
            state.input.token.clone(),
            state.input.focus.clone(),
            state.input.gate.clone(),
            state.node,
            state.base_input_allowed(window, false)
                && state.lease.as_ref().and_then(Lease::snapshot).is_some(),
        )
    };
    gate.borrow_mut()
        .record(node, focus.clone(), eligible, focus.is_focused(window));
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
        if next != state.input.hover {
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
            state.commit(target, cx);
        } else {
            state.input.hover = None;
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
