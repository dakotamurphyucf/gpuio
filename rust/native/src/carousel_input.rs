//! Native carousel input. Child widgets keep precedence; direct manipulation
//! owns bounded preview/capture and wheel bursts own at most one deadline. Only
//! completed intent crosses the asynchronous application bridge.
use super::*;
use crate::carousel_gesture::{Drag, Phase, Progress, Step, Wheel};
use gpui::{
    A11ySubtreeBuilder, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, ScrollWheelEvent, TouchPhase,
};

#[derive(Default)]
pub(super) struct Input {
    wheel: Wheel,
    drag: Option<Gesture>,
    shield: Option<Hitbox>,
    presenter: std::rc::Weak<RefCell<navigation::State>>,
    timer: Option<gpui::Task<()>>,
    pub(super) visible: bool,
    pub(super) enabled: bool,
}
impl Input {
    #[cfg(feature = "native-tests")]
    pub(super) fn shield_hitbox(&self) -> Option<gpui::HitboxId> {
        self.shield.as_ref().map(|hitbox| hitbox.id)
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn dragging(&self) -> bool {
        self.drag.is_some()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn has_timer(&self) -> bool {
        self.timer.is_some()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn wheel_deadline(&self) -> Option<Duration> {
        self.wheel.deadline()
    }
    pub(super) fn active(&self) -> bool {
        self.wheel.active() || self.drag.is_some()
    }
    pub(super) fn interrupt(&mut self, now: Duration) {
        self.timer = None;
        self.wheel.interrupt(now);
    }
}
struct Gesture {
    model: Drag,
    claimed: bool,
    hitbox: gpui::HitboxId,
    bounds: Bounds<Pixels>,
}
impl Gesture {
    fn claimed(&self) -> bool {
        self.claimed
    }
}
impl Input {
    pub(super) fn cancel_drag(&mut self, window: &mut Window, immediate: bool) -> bool {
        let Some(gesture) = self.drag.take() else {
            return false;
        };
        let claimed = gesture.claimed();
        let owned = window.captured_hitbox() == Some(gesture.hitbox);
        if claimed {
            if owned {
                window.release_pointer();
            }
            if let Some(presenter) = self.presenter.upgrade() {
                presenter.borrow_mut().finish_preview(immediate);
            }
            window.refresh();
        }
        // A stolen capture belongs to its new owner; do not consume its release
        // or Escape just because this retired gesture once owned the pointer.
        claimed && owned
    }
    fn capture_valid(&self, window: &Window) -> bool {
        self.drag.as_ref().is_none_or(|drag| {
            if drag.claimed() {
                window.captured_hitbox() == Some(drag.hitbox)
                    && self
                        .presenter
                        .upgrade()
                        .is_some_and(|state| state.borrow().previewing())
            } else {
                window.captured_hitbox().is_none()
            }
        })
    }
    fn rebind(&mut self, hitbox: &Hitbox, window: &mut Window) {
        if !self.capture_valid(window)
            || self
                .drag
                .as_ref()
                .is_some_and(|drag| drag.bounds != hitbox.bounds)
        {
            self.cancel_drag(window, true);
        }
        if let Some(drag) = &mut self.drag {
            drag.hitbox = hitbox.id;
            if drag.claimed() {
                window.capture_pointer(hitbox.id);
            }
        }
    }
}
fn request(step: Step) -> Request {
    match step {
        Step::Previous => Request::Previous,
        Step::Next => Request::Next,
    }
}
impl View {
    pub(super) fn carousel_input_available(&self, id: NodeId, window: &Window) -> bool {
        self.carousels.get(&id).is_some_and(|state| {
            let state = state.borrow();
            !state.config.disabled
                && state.input.visible
                && state.input.enabled
                && window.is_window_active()
                && self.focus.borrow().allows(id)
                && self.focus.borrow().visible(id)
        })
    }
    fn carousel_wheel(
        &mut self,
        id: NodeId,
        expected: &Rc<RefCell<State>>,
        config: &Arc<Config>,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.carousel_input_available(id, window)
            || window.captured_hitbox().is_some()
            || cx.has_active_drag()
            || event.modifiers.modified()
            || expected.borrow().input.drag.is_some()
        {
            return;
        }
        let Some(state) = self.carousels.get(&id) else {
            return;
        };
        if !Rc::ptr_eq(state, expected) || state.borrow().config != *config {
            return;
        }
        let delta = event.delta.pixel_delta(window.line_height());
        let (expired, result) = {
            let mut state = state.borrow_mut();
            let axis = state.config.axis;
            let now = state.origin.elapsed();
            let expired = state.input.wheel.finish(now);
            let result = state.input.wheel.push(
                axis,
                [delta.x.into(), delta.y.into()],
                event.delta.precise(),
                match event.touch_phase {
                    TouchPhase::Started => Phase::Started,
                    TouchPhase::Moved => Phase::Moved,
                    TouchPhase::Ended => Phase::Ended,
                    TouchPhase::Cancelled => Phase::Cancelled,
                },
                now,
            );
            (expired, result)
        };
        if result.consumed {
            cx.stop_propagation();
        }
        for step in expired.into_iter().chain(result.request) {
            self.carousel_request(id, request(step));
        }
        self.schedule_carousel_wheel(id, window, cx);
        self.schedule_carousel(id, window, cx);
    }
    pub(super) fn schedule_carousel_wheel(
        &mut self,
        id: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.carousels.get(&id).cloned() else {
            return;
        };
        let available = self.carousel_input_available(id, window)
            && window.captured_hitbox().is_none()
            && !cx.has_active_drag();
        let mut input = state.borrow_mut();
        let now = input.origin.elapsed();
        if !available {
            input.input.interrupt(now);
            return;
        }
        let Some(deadline) = input.input.wheel.deadline() else {
            input.input.timer = None;
            return;
        };
        if input.input.timer.is_some() {
            return;
        }
        let generation = Rc::downgrade(&state);
        input.input.timer = Some(cx.spawn_in(window, async move |owner, cx| {
            cx.background_executor()
                .timer(deadline.saturating_sub(now))
                .await;
            let _ = owner.update_in(cx, |view, window, cx| {
                let Some(state) = generation.upgrade() else {
                    return;
                };
                if !view
                    .carousels
                    .get(&id)
                    .is_some_and(|current| Rc::ptr_eq(current, &state))
                {
                    return;
                }
                state.borrow_mut().input.timer = None;
                let available = view.carousel_input_available(id, window)
                    && window.captured_hitbox().is_none()
                    && !cx.has_active_drag();
                let step = {
                    let mut state = state.borrow_mut();
                    let now = state.origin.elapsed();
                    if available {
                        state.input.wheel.finish(now)
                    } else {
                        state.input.interrupt(now);
                        None
                    }
                };
                if let Some(step) = step {
                    view.carousel_request(id, request(step));
                }
                // Samples extend the deadline without spawning per sample. An
                // early wake schedules only the remaining interval.
                view.schedule_carousel_wheel(id, window, cx);
                view.schedule_carousel(id, window, cx);
            });
        }));
    }
}

#[derive(Clone)]
struct Route {
    node: NodeId,
    state: Rc<RefCell<State>>,
    config: Arc<Config>,
}
impl View {
    fn carousel_route_current(&self, route: &Route, window: &Window) -> bool {
        self.carousel_input_available(route.node, window)
            && self.carousels.get(&route.node).is_some_and(|state| {
                Rc::ptr_eq(state, &route.state) && state.borrow().config == route.config
            })
    }
    pub(in super::super) fn cancel_carousel_drags(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut claimed = false;
        for state in self.carousels.values() {
            claimed |= state
                .borrow_mut()
                .input
                .cancel_drag(window, cx.reduce_motion());
        }
        self.schedule_carousels(window, cx);
        claimed
    }
    pub(super) fn sync_carousel_drag(
        &mut self,
        id: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let available = self.carousel_input_available(id, window) && !cx.has_active_drag();
        if let Some(state) = self.carousels.get(&id) {
            let mut state = state.borrow_mut();
            if !available || !state.input.capture_valid(window) {
                state.input.cancel_drag(window, true);
            }
        }
    }
    fn carousel_drag_down(
        &mut self,
        route: &Route,
        hitbox: &Hitbox,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.carousel_route_current(route, window)
            || event.button != gpui::MouseButton::Left
            || event.modifiers.modified()
            || !(hitbox.is_hovered(window)
                || route
                    .state
                    .borrow()
                    .input
                    .shield
                    .as_ref()
                    .is_some_and(|shield| shield.is_hovered(window)))
            || window.default_prevented()
            || window.captured_hitbox().is_some()
            || cx.has_active_drag()
        {
            return;
        }
        let mut state = route.state.borrow_mut();
        if state.input.drag.is_some() {
            return;
        }
        let extent = match state.config.axis {
            Axis::Horizontal => hitbox.bounds.size.width,
            Axis::Vertical => hitbox.bounds.size.height,
        };
        let Some(model) = Drag::new(
            state.config.axis,
            [event.position.x.into(), event.position.y.into()],
            extent.into(),
            state.config.target(&Request::Previous).is_some(),
            state.config.target(&Request::Next).is_some(),
        ) else {
            return;
        };
        let now = state.origin.elapsed();
        state.input.interrupt(now);
        state.input.drag = Some(Gesture {
            model,
            claimed: false,
            hitbox: hitbox.id,
            bounds: hitbox.bounds,
        });
        drop(state);
        self.schedule_carousel(route.node, window, cx);
    }
    fn carousel_drag_move(
        &mut self,
        route: &Route,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if route.state.borrow().input.drag.is_none() {
            return;
        }
        if !self.carousel_route_current(route, window)
            || event.pressed_button != Some(gpui::MouseButton::Left)
            || cx.has_active_drag()
        {
            route.state.borrow_mut().input.cancel_drag(window, true);
            self.schedule_carousel(route.node, window, cx);
            return;
        }
        let mut state = route.state.borrow_mut();
        if !state.input.capture_valid(window) {
            state.input.cancel_drag(window, true);
            return;
        }
        let Some(presenter) = state.input.presenter.upgrade() else {
            state.input.cancel_drag(window, true);
            return;
        };
        let drag = state.input.drag.as_mut().expect("checked gesture");
        drag.model
            .set_start_offset(presenter.borrow().drag_origin());
        let was_claimed = drag.claimed();
        let progress = drag
            .model
            .update([event.position.x.into(), event.position.y.into()]);
        let hitbox = drag.hitbox;
        match progress {
            Progress::Pending => (),
            Progress::Rejected => {
                state.input.cancel_drag(window, true);
            }
            Progress::Dragging { offset, neighbor } => {
                let neighbor = neighbor
                    .and_then(|step| state.config.target(&request(step)))
                    .and_then(|index| state.pages.get(index).copied());
                presenter.borrow_mut().preview(offset, neighbor);
                if !was_claimed {
                    state.input.drag.as_mut().expect("active gesture").claimed = true;
                    window.capture_pointer(hitbox);
                    window.focus(&state.focus, cx);
                }
                window.prevent_default();
                cx.stop_propagation();
                window.refresh();
            }
        }
        drop(state);
        self.schedule_carousel(route.node, window, cx);
    }
    fn carousel_drag_up(
        &mut self,
        route: &Route,
        event: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self.carousel_route_current(route, window);
        let step = {
            let mut state = route.state.borrow_mut();
            let valid = current
                && state.input.capture_valid(window)
                && event.button == gpui::MouseButton::Left;
            let step = state.input.drag.as_mut().and_then(|drag| {
                if !valid || !drag.claimed() {
                    return None;
                }
                drag.model
                    .update([event.position.x.into(), event.position.y.into()]);
                drag.model.finish()
            });
            let claimed = state
                .input
                .cancel_drag(window, !valid || cx.reduce_motion());
            if claimed {
                window.prevent_default();
                cx.stop_propagation();
            }
            step
        };
        if let Some(step) = step {
            self.carousel_request(route.node, request(step));
        }
        self.schedule_carousel(route.node, window, cx);
    }
}

/// The outgoing page shields its widgets but remains a drag surface. The extra
/// hitbox is above its inert descendants and below the accepted incoming page,
/// so incoming editors/buttons retain ordinary precedence.
pub(in super::super) fn inert_page(
    body: gpui::AnyElement,
    state: &Rc<RefCell<State>>,
) -> gpui::AnyElement {
    let state = Rc::downgrade(state);
    div()
        .relative()
        .size_full()
        .child(crate::semantics::InteractionShield::inert(body))
        .child(
            canvas(
                move |bounds, window, _| {
                    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
                    if let Some(state) = state.upgrade() {
                        state.borrow_mut().input.shield = Some(hitbox);
                    }
                },
                |_, _, _, _| (),
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .into_any_element()
}

pub(in super::super) struct Region<E> {
    pub element: E,
    pub state: Rc<RefCell<State>>,
    pub owner: gpui::WeakEntity<View>,
    pub node: NodeId,
    pub enabled: bool,
    pub presenter: std::rc::Weak<RefCell<navigation::State>>,
}
impl<E: Element> IntoElement for Region<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for Region<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = (Hitbox, E::PrepaintState);
    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
        {
            let mut state = self.state.borrow_mut();
            state.input.presenter = self.presenter.clone();
            state.input.shield = None;
            state.input.rebind(&hitbox, window);
        }
        let child = self
            .element
            .prepaint(id, inspector, bounds, layout, window, cx);
        (hitbox, child)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.state.borrow_mut().input.enabled = self.enabled;
        let owner = self.owner.clone();
        let node = self.node;
        let state = self.state.clone();
        let config = state.borrow().config.clone();
        let hitbox = prepaint.0.clone();
        // Earlier registration means later bubbling: nested scroll views retain
        // their axes and stop propagation only when they actually scroll.
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
            if phase.bubble() && hitbox.should_handle_scroll(window) {
                let _ = owner.update(cx, |view, cx| {
                    view.carousel_wheel(node, &state, &config, event, window, cx)
                });
            }
        });
        let route = Route {
            node: self.node,
            state: self.state.clone(),
            config: self.state.borrow().config.clone(),
        };
        let owner = self.owner.clone();
        let down = route.clone();
        let hitbox = prepaint.0.clone();
        window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, window, cx| {
            if phase.bubble() {
                let _ = owner.update(cx, |view, cx| {
                    view.carousel_drag_down(&down, &hitbox, event, window, cx)
                });
            }
        });
        let owner = self.owner.clone();
        let motion = route.clone();
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
            if phase.capture() {
                let _ = owner.update(cx, |view, cx| {
                    view.carousel_drag_move(&motion, event, window, cx)
                });
            }
        });
        self.element
            .paint(id, inspector, bounds, layout, &mut prepaint.1, window, cx);
        // Bubble before descendant activation; their capture listeners first get
        // to clear ordinary pressed state, as in the native pointer adapter.
        let owner = self.owner.clone();
        window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
            if phase.bubble() {
                let _ = owner.update(cx, |view, cx| {
                    view.carousel_drag_up(&route, event, window, cx)
                });
            }
        });
    }
    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.element.write_a11y_info(node);
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element
            .a11y_synthetic_children(&mut prepaint.1, builder);
    }
}
