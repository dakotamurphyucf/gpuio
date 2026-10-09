//! Captured background dragging for retained measured tracks. Nested controls
//! receive their normal pointer events before this region may claim a gesture.
use super::*;
use crate::carousel_track_gesture::{Drag, Progress};
use gpui::{Hitbox, HitboxBehavior};
pub(super) struct Gesture {
    pub model: Drag,
    hitbox: gpui::HitboxId,
    bounds: Bounds<Pixels>,
    claimed: bool,
}
impl State {
    pub(super) fn cancel_track_drag(&mut self, window: &mut Window) -> bool {
        let Some(drag) = self.drag.take() else {
            return false;
        };
        let owned = drag.claimed && window.captured_hitbox() == Some(drag.hitbox);
        if owned {
            window.release_pointer();
        }
        self.motion.finish_preview();
        if drag.claimed {
            window.refresh();
        }
        owned
    }
}
#[derive(Clone)]
struct Route {
    id: NodeId,
    state: Rc<RefCell<State>>,
    config: Arc<gpuio_protocol::carousel_track::Config>,
}
impl View {
    pub(in super::super) fn track_pointer_available(
        &self,
        id: NodeId,
        window: &Window,
        cx: &App,
    ) -> bool {
        self.carousel_tracks.get(&id).is_some_and(|state| {
            let state = state.borrow();
            state.painted
                && state.model.geometry().is_some()
                && !state.model.config().carousel.disabled
        }) && window.is_window_active()
            && !cx.has_active_drag()
            && self.focus.borrow().allows(id)
    }
    fn track_pointer_current(&self, route: &Route, window: &Window, cx: &App) -> bool {
        self.track_pointer_available(route.id, window, cx)
            && self.carousel_tracks.get(&route.id).is_some_and(|s| {
                Rc::ptr_eq(s, &route.state) && s.borrow().model.config() == route.config.as_ref()
            })
    }
    pub(in super::super) fn sync_track_drag(&self, id: NodeId, window: &mut Window, cx: &App) {
        let available = self.track_pointer_available(id, window, cx);
        if let Some(state) = self.carousel_tracks.get(&id) {
            let mut state = state.borrow_mut();
            let captured = window.captured_hitbox();
            if !available
                || state.drag.as_ref().is_some_and(|drag| {
                    if drag.claimed {
                        captured != Some(drag.hitbox)
                    } else {
                        captured.is_some()
                    }
                })
            {
                state.cancel_track_drag(window);
            }
        }
    }
    pub(in super::super) fn cancel_track_drags(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut claimed = false;
        for state in self.carousel_tracks.values() {
            let mut state = state.borrow_mut();
            claimed |= state.cancel_track_drag(window);
            if state.wheel.geometry().is_some() {
                let now = state.track_now(cx);
                state.interrupt_track_wheel(now, true);
                window.refresh();
                claimed = true;
            }
        }
        self.schedule_carousel_tracks(window, cx);
        claimed
    }
    pub(in super::super) fn track_pointer_down(
        &mut self,
        id: NodeId,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.carousel_tracks.get(&id) else {
            return;
        };
        let Some(hitbox) = state.borrow().pointer_hitbox.clone() else {
            return;
        };
        let route = Route {
            id,
            state: state.clone(),
            config: Arc::new(state.borrow().model.config().clone()),
        };
        self.track_down(&route, &hitbox, event, window, cx);
    }
    fn track_down(
        &mut self,
        route: &Route,
        hitbox: &Hitbox,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != gpui::MouseButton::Left
            || event.modifiers.modified()
            || window.default_prevented()
            || !hitbox.is_hovered(window)
            || window.captured_hitbox().is_some()
            || !self.track_pointer_current(route, window, cx)
        {
            return;
        }
        let mut state = route.state.borrow_mut();
        if state.drag.is_some() {
            return;
        }
        let Some(offset) = state.motion.painted_offset() else {
            return;
        };
        let Some(index) = state.model.config().carousel.selected else {
            return;
        };
        let Some(model) = Drag::new(
            state.model.config().carousel.axis,
            [event.position.x.into(), event.position.y.into()],
            offset,
            index as usize,
            state.model.geometry().expect("available geometry").clone(),
        ) else {
            return;
        };
        let now = state.track_now(cx);
        state.interrupt_track_wheel(now, false);
        state.drag = Some(Gesture {
            model,
            hitbox: hitbox.id,
            bounds: hitbox.bounds,
            claimed: false,
        });
        drop(state);
        self.schedule_carousel_track(route.id, window, cx);
    }
    fn track_move(
        &mut self,
        route: &Route,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if route.state.borrow().drag.is_none() {
            return;
        }
        if !self.track_pointer_current(route, window, cx)
            || event.pressed_button != Some(gpui::MouseButton::Left)
        {
            route.state.borrow_mut().cancel_track_drag(window);
            return;
        }
        self.sync_track_drag(route.id, window, cx);
        let mut state = route.state.borrow_mut();
        let painted = state.motion.painted_offset();
        let Some(drag) = state.drag.as_mut() else {
            return;
        };
        if let Some(offset) = painted {
            drag.model.follow_paint(offset);
        }
        match drag
            .model
            .update([event.position.x.into(), event.position.y.into()])
        {
            Progress::Pending => (),
            Progress::Rejected => {
                state.cancel_track_drag(window);
            }
            Progress::Dragging(offset) => {
                let hitbox = drag.hitbox;
                let first = !drag.claimed;
                drag.claimed = true;
                if state.motion.preview(f64::from(offset)) {
                    if first {
                        window.capture_pointer(hitbox);
                        window.focus(&state.focus, cx);
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    window.refresh();
                } else {
                    state.cancel_track_drag(window);
                }
            }
        }
        drop(state);
        self.schedule_carousel_track(route.id, window, cx);
    }
    fn track_up(
        &mut self,
        route: &Route,
        event: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let valid = self.track_pointer_current(route, window, cx)
            && event.button == gpui::MouseButton::Left;
        let request = {
            let mut state = route.state.borrow_mut();
            let index = state.drag.as_mut().and_then(|drag| {
                if !valid || !drag.claimed || window.captured_hitbox() != Some(drag.hitbox) {
                    return None;
                }
                drag.model
                    .update([event.position.x.into(), event.position.y.into()]);
                drag.model.finish()
            });
            let request = index
                .filter(|i| Some(*i as i64) != state.model.config().carousel.selected)
                .map(|i| Request::Select(state.model.config().carousel.ids[i].clone()));
            let click_focus = valid && state.drag.as_ref().is_some_and(|drag| !drag.claimed);
            if click_focus {
                window.focus(&state.focus, cx);
            }
            if state.cancel_track_drag(window) {
                window.prevent_default();
                cx.stop_propagation();
            }
            request
        };
        if let Some(request) = request {
            self.carousel_track_request(route.id, request);
        }
        self.schedule_carousel_track(route.id, window, cx);
    }
    pub(in super::super) fn wrap_carousel_track_pointer(
        &self,
        element: AnyElement,
        node: &crate::tree::Node,
        cx: &Context<Self>,
    ) -> AnyElement {
        let Some(config) = node.carousel_track.as_ref() else {
            return element;
        };
        let Some(state) = self.carousel_tracks.get(&node.id) else {
            return element;
        };
        Region {
            element,
            route: Route {
                id: node.id,
                state: state.clone(),
                config: config.clone(),
            },
            owner: cx.weak_entity(),
        }
        .into_any_element()
    }
}
struct Region {
    element: AnyElement,
    route: Route,
    owner: gpui::WeakEntity<View>,
}
impl IntoElement for Region {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Region {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouseExceptScroll);
        {
            let mut state = self.route.state.borrow_mut();
            state.pointer_hitbox = Some(hitbox.clone());
            let captured = window.captured_hitbox();
            if state.drag.as_ref().is_some_and(|drag| {
                drag.bounds != hitbox.bounds || (drag.claimed && captured != Some(drag.hitbox))
            }) {
                state.cancel_track_drag(window);
            }
            if let Some(drag) = &mut state.drag {
                drag.hitbox = hitbox.id;
                if drag.claimed {
                    window.capture_pointer(hitbox.id);
                }
            }
        }
        self.element.prepaint(window, cx);
        hitbox
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        let id = self.route.id;
        let state = self.route.state.clone();
        let config = self.route.config.clone();
        let owner = self.owner.clone();
        let hitbox = hitbox.clone();
        // Descendant scroll containers bubble first; their consumed deltas do not
        // reach this track. The track then precedes enclosing scrollers.
        window.on_mouse_event(move |event: &gpui::ScrollWheelEvent, phase, window, cx| {
            if phase.bubble() && hitbox.should_handle_scroll(window) {
                let _ = owner.update(cx, |view, cx| {
                    view.track_wheel(id, &state, &config, event, window, cx)
                });
            }
        });
        let route = self.route.clone();
        let owner = self.owner.clone();
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
            if phase.capture() {
                let _ = owner.update(cx, |view, cx| view.track_move(&route, event, window, cx));
            }
        });
        self.element.paint(window, cx);
        let route = self.route.clone();
        let owner = self.owner.clone();
        window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
            if phase.bubble() {
                let _ = owner.update(cx, |view, cx| view.track_up(&route, event, window, cx));
            }
        });
    }
}
