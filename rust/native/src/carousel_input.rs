//! Native wheel delivery. Child scroll owners get first refusal; one burst owns
//! at most one cancellable deadline and emits only a relative selection request.
use super::*;
use crate::carousel_gesture::{Phase, Step, Wheel};
use gpui::{
    A11ySubtreeBuilder, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, ScrollWheelEvent, TouchPhase,
};

#[derive(Default)]
pub(super) struct Input {
    wheel: Wheel,
    timer: Option<gpui::Task<()>>,
    pub(super) visible: bool,
    pub(super) enabled: bool,
}
impl Input {
    #[cfg(feature = "native-tests")]
    pub(super) fn has_timer(&self) -> bool {
        self.timer.is_some()
    }
    pub(super) fn active(&self) -> bool {
        self.wheel.active()
    }
    pub(super) fn interrupt(&mut self, now: Duration) {
        self.timer = None;
        self.wheel.interrupt(now);
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

pub(in super::super) struct Region<E> {
    pub element: E,
    pub state: Rc<RefCell<State>>,
    pub owner: gpui::WeakEntity<View>,
    pub node: NodeId,
    pub enabled: bool,
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
        self.element
            .paint(id, inspector, bounds, layout, &mut prepaint.1, window, cx);
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
