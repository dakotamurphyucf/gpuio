//! General input regions. Native policies run during dispatch; owned observations
//! cross the bounded mailbox. This module never calls into OCaml synchronously.
use super::{View, choice::Route};
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Context, Div, Element, ElementId, FocusHandle,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, LayoutId, Pixels, Stateful,
    Window, prelude::*,
};
use gpuio_protocol::{
    input::*,
    pointer::{PointerButton, PointerModifiers},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub(super) type Shared = Rc<RefCell<State>>;
pub(super) struct State {
    route: Route,
    config: Arc<Config>,
    pub(super) focus: FocusHandle,
    subscriptions: Vec<gpui::Subscription>,
    hovered: bool,
    pointer_inside_window: Rc<Cell<bool>>,
    pressed: [bool; 5],
    painted: bool,
}
impl State {
    fn reset(&mut self) {
        self.hovered = false;
        self.pressed = [false; 5];
    }
    pub(super) fn clear(&mut self) {
        self.reset();
        self.painted = false;
    }
}
fn button(value: gpui::MouseButton) -> PointerButton {
    super::pointer::button(value)
}
fn modifiers(value: gpui::Modifiers) -> PointerModifiers {
    super::pointer::modifiers(value)
}
fn slot(button: PointerButton) -> usize {
    match button {
        PointerButton::Left => 0,
        PointerButton::Right => 1,
        PointerButton::Middle => 2,
        PointerButton::Back => 3,
        PointerButton::Forward => 4,
    }
}
fn contains(hitbox: &Hitbox, point: &gpui::Point<Pixels>) -> bool {
    hitbox
        .bounds
        .intersect(&hitbox.content_mask.bounds)
        .contains(point)
}
fn location(point: gpui::Point<Pixels>, bounds: Bounds<Pixels>, keys: gpui::Modifiers) -> Location {
    Location {
        window: Position {
            x: f32::from(point.x) as f64,
            y: f32::from(point.y) as f64,
        },
        local: Position {
            x: f32::from(point.x - bounds.origin.x) as f64,
            y: f32::from(point.y - bounds.origin.y) as f64,
        },
        modifiers: modifiers(keys),
    }
}
fn eligible(state: &State, window: &Window, pointer: bool) -> bool {
    if !state.painted
        || state.config.disabled
        || !window.is_window_active()
        || !state.route.gate.borrow().allows(state.route.node)
    {
        return false;
    }
    let session = state.route.session.borrow();
    let Some(tree) = session.tree(state.route.window) else {
        return false;
    };
    let Some(node) = tree.get(state.route.node) else {
        return false;
    };
    node.handler == Some(state.route.handler)
        && node.input_region.as_ref() == Some(&state.config)
        && (!pointer || super::pointer_enabled(tree, state.route.node))
}
fn policy(value: Policy, window: &mut Window, cx: &mut App) {
    match value {
        Policy::Observe => (),
        Policy::StopPropagation => cx.stop_propagation(),
        Policy::PreventDefault => window.prevent_default(),
        Policy::PreventAndStop => {
            window.prevent_default();
            cx.stop_propagation();
        }
    }
}
fn emit(
    state: &Shared,
    event: Event,
    phase: Option<gpui::DispatchPhase>,
    window: &mut Window,
    cx: &mut App,
) {
    let (route, subscription) = {
        let state = state.borrow();
        let pointer = !matches!(
            event,
            Event::KeyDown(..) | Event::KeyUp(_) | Event::Focus | Event::Blur
        );
        if !eligible(&state, window, pointer) {
            return;
        }
        let Some(subscription) = state.config.subscription(event.kind()) else {
            return;
        };
        if phase.is_some_and(|phase| phase.capture() != (subscription.phase == Phase::Capture)) {
            return;
        }
        (state.route.clone(), subscription)
    };
    // Input validation and subscription admission precede applying native policy.
    let output = route.session.borrow().input_observed(
        route.window,
        route.node,
        route.handler,
        route.revision,
        event,
    );
    if let Some(output) = output {
        policy(subscription.policy, window, cx);
        if !route.transport.input(output) && route.session.borrow_mut().overload(route.window) {
            route.transport.fault(route.window);
        }
    }
}
fn hover(state: &Shared, hitbox: &Hitbox, window: &mut Window, cx: &mut App) {
    let transition = {
        let mut state = state.borrow_mut();
        if !eligible(&state, window, true) {
            state.reset();
            return;
        }
        let hovered = state.pointer_inside_window.get() && hitbox.is_hovered(window);
        if hovered == state.hovered {
            return;
        }
        state.hovered = hovered;
        if hovered {
            Event::MouseEnter
        } else {
            Event::MouseLeave
        }
    };
    emit(state, transition, None, window, cx);
}
fn key(value: &gpui::Keystroke) -> Key {
    Key {
        key: value.key.clone(),
        character: value.key_char.clone(),
        modifiers: modifiers(value.modifiers),
    }
}

impl View {
    pub(super) fn retire_ineligible_input_regions(&self, window: &Window) {
        for shared in self.input_regions.values() {
            let mut state = shared.borrow_mut();
            if !eligible(&state, window, true) {
                // A source can hide and restore a retained label without a
                // frame between transitions. Its pending click must not return.
                state.clear();
            }
        }
    }
    pub(super) fn input_region_element(
        &mut self,
        mut element: Stateful<Div>,
        node: &crate::tree::Node,
        revision: i64,
        config: &Arc<Config>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let id = node.id;
        self.visited.insert(id);
        let route = Route {
            window: self.id,
            node: id,
            handler: node.handler.expect("validated input region"),
            revision,
            session: self.session.clone(),
            gate: self.focus.clone(),
            transport: self.transport.clone(),
        };
        let pointer_inside_window = self.input_pointer_inside.clone();
        let state = self
            .input_regions
            .entry(id)
            .or_insert_with(|| {
                Rc::new(RefCell::new(State {
                    route: route.clone(),
                    config: config.clone(),
                    focus: cx.focus_handle().tab_stop(false),
                    subscriptions: Vec::new(),
                    hovered: false,
                    pointer_inside_window,
                    pressed: [false; 5],
                    painted: false,
                }))
            })
            .clone();
        let focus = {
            let mut state = state.borrow_mut();
            if state.route.handler != route.handler || state.config != *config {
                state.clear();
                state.subscriptions.clear();
            }
            state.route = route;
            state.config = config.clone();
            state
                .focus
                .clone()
                .tab_stop(!config.disabled && config.focus == Focus::Tab)
        };
        let binding = state.borrow().route.handler;
        if state.borrow().subscriptions.is_empty() {
            let weak = Rc::downgrade(&state);
            let focused = cx.on_focus(&focus, window, move |_, window, cx| {
                if let Some(state) = weak.upgrade()
                    && state.borrow().route.handler == binding
                {
                    emit(&state, Event::Focus, None, window, cx);
                }
            });
            let weak = Rc::downgrade(&state);
            let blurred = cx.on_blur(&focus, window, move |_, window, cx| {
                if let Some(state) = weak.upgrade()
                    && state.borrow().route.handler == binding
                {
                    emit(&state, Event::Blur, None, window, cx);
                }
            });
            state.borrow_mut().subscriptions = vec![focused, blurred];
        }
        if config.disabled || config.focus == Focus::None {
            if focus.is_focused(window) {
                window.blur(cx);
            }
        } else {
            element = element.track_focus(&focus);
            let state = state.clone();
            element =
                element.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                    let handle = {
                        let state = state.borrow();
                        (state.route.handler == binding && eligible(&state, window, false))
                            .then(|| state.focus.clone())
                    };
                    if let Some(handle) = handle {
                        window.focus(&handle, cx);
                    }
                });
        }
        // GPUI's dispatch tree scopes these to this region and its descendants.
        for phase in [gpui::DispatchPhase::Capture, gpui::DispatchPhase::Bubble] {
            let state = state.clone();
            let listener = move |event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App| {
                if state.borrow().route.handler != binding {
                    return;
                }
                if event.keystroke.key == "escape" {
                    state.borrow_mut().pressed = [false; 5];
                }
                emit(
                    &state,
                    Event::KeyDown(key(&event.keystroke), event.is_held),
                    Some(phase),
                    window,
                    cx,
                );
            };
            element = if phase.capture() {
                element.capture_key_down(listener)
            } else {
                element.on_key_down(listener)
            };
            let state = self.input_regions[&id].clone();
            let listener = move |event: &gpui::KeyUpEvent, window: &mut Window, cx: &mut App| {
                if state.borrow().route.handler != binding {
                    return;
                }
                emit(
                    &state,
                    Event::KeyUp(key(&event.keystroke)),
                    Some(phase),
                    window,
                    cx,
                );
            };
            element = if phase.capture() {
                element.capture_key_up(listener)
            } else {
                element.on_key_up(listener)
            };
        }
        element
            .role(gpui::Role::Group)
            .aria_label(config.label.clone())
    }
}

pub(super) struct Region<E> {
    pub element: E,
    pub state: Shared,
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
        let child = self
            .element
            .prepaint(id, inspector, bounds, layout, window, cx);
        // A non-occluding subtree hitbox, after descendants. Capture observers can
        // see child input even when that child blocks its ordinary ancestor hitboxes.
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.state.borrow_mut().painted = true;
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
        let binding = self.state.borrow().route.handler;
        {
            // Register the region before its native descendants so Tab follows
            // the same parent-before-child order as the public view tree.
            let state = self.state.borrow();
            if !state.config.disabled
                && state.config.focus != Focus::None
                && bounds.size.width > gpui::px(0.)
                && bounds.size.height > gpui::px(0.)
            {
                state.route.gate.borrow_mut().record(
                    state.route.node,
                    state.focus.clone(),
                    state.config.focus == Focus::Tab,
                    state.focus.is_focused(window),
                    bounds,
                );
            }
        }
        let state = self.state.clone();
        let hitbox = prepaint.0.clone();
        // Registered before child paint: capture is parent-first, bubble is child-first.
        window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, window, cx| {
            if state.borrow().route.handler != binding {
                return;
            }
            let allowed = eligible(&state.borrow(), window, true);
            if !allowed {
                state.borrow_mut().reset();
                return;
            }
            let inside = hitbox.is_hovered(window) && contains(&hitbox, &event.position);
            let sample = Mouse {
                location: location(event.position, bounds, event.modifiers),
                button: button(event.button),
                click_count: event.click_count as i64,
            };
            if phase.capture() && !contains(&hitbox, &event.position) {
                emit(&state, Event::MouseDownOutside(sample), None, window, cx);
            }
            if inside {
                emit(&state, Event::MouseDown(sample), Some(phase), window, cx);
                if phase.bubble() && window.captured_hitbox().is_none() {
                    state.borrow_mut().pressed[slot(sample.button)] = true;
                }
            } else if phase.capture() {
                state.borrow_mut().pressed[slot(sample.button)] = false;
            }
        });
        let state = self.state.clone();
        let hitbox = prepaint.0.clone();
        // Each mouse-up captures and clears pending state even if a child consumes
        // bubbling. Only the same dispatch's bubble phase can complete its click.
        let pending = Rc::new(RefCell::new(false));
        window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
            if state.borrow().route.handler != binding {
                return;
            }
            let index = slot(button(event.button));
            if phase.capture() {
                let mut state = state.borrow_mut();
                *pending.borrow_mut() = state.pressed[index];
                state.pressed[index] = false;
            }
            if !eligible(&state.borrow(), window, true) {
                state.borrow_mut().reset();
                *pending.borrow_mut() = false;
                return;
            }
            if !hitbox.is_hovered(window) || !contains(&hitbox, &event.position) {
                return;
            }
            let sample = Mouse {
                location: location(event.position, bounds, event.modifiers),
                button: button(event.button),
                click_count: event.click_count as i64,
            };
            emit(&state, Event::MouseUp(sample), Some(phase), window, cx);
            if phase.bubble() && *pending.borrow_mut() && window.captured_hitbox().is_none() {
                *pending.borrow_mut() = false;
                emit(
                    &state,
                    if sample.button == PointerButton::Left {
                        Event::Click(sample)
                    } else {
                        Event::AuxiliaryClick(sample)
                    },
                    None,
                    window,
                    cx,
                );
            }
        });
        let state = self.state.clone();
        let hitbox = prepaint.0.clone();
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
            if state.borrow().route.handler != binding {
                return;
            }
            if phase.capture() {
                if window.captured_hitbox().is_some() {
                    state.borrow_mut().pressed = [false; 5];
                }
                hover(&state, &hitbox, window, cx);
            }
            if hitbox.is_hovered(window) && contains(&hitbox, &event.position) {
                emit(
                    &state,
                    Event::MouseMove(Motion {
                        location: location(event.position, bounds, event.modifiers),
                        pressed_button: event.pressed_button.map(button),
                    }),
                    Some(phase),
                    window,
                    cx,
                );
            }
        });
        let state = self.state.clone();
        window.on_mouse_event(move |_: &gpui::MouseExitEvent, phase, window, cx| {
            if !phase.capture() || state.borrow().route.handler != binding {
                return;
            }
            let was_hovered = {
                let mut state = state.borrow_mut();
                let hovered = state.hovered;
                state.reset();
                hovered
            };
            if was_hovered {
                emit(&state, Event::MouseLeave, None, window, cx);
            }
        });
        let state = self.state.clone();
        let hitbox = prepaint.0.clone();
        window.on_mouse_event(move |event: &gpui::ScrollWheelEvent, phase, window, cx| {
            if state.borrow().route.handler != binding {
                return;
            }
            if !hitbox.should_handle_scroll(window) {
                return;
            }
            let delta = match event.delta {
                gpui::ScrollDelta::Pixels(p) => Delta::Pixels(Position {
                    x: f32::from(p.x) as f64,
                    y: f32::from(p.y) as f64,
                }),
                gpui::ScrollDelta::Lines(p) => Delta::Lines(Position {
                    x: p.x as f64,
                    y: p.y as f64,
                }),
            };
            let touch = match event.touch_phase {
                gpui::TouchPhase::Started => TouchPhase::Started,
                gpui::TouchPhase::Moved => TouchPhase::Moved,
                gpui::TouchPhase::Ended => TouchPhase::Ended,
                gpui::TouchPhase::Cancelled => TouchPhase::Cancelled,
            };
            emit(
                &state,
                Event::Scroll(Scroll {
                    location: location(event.position, bounds, event.modifiers),
                    delta,
                    phase: touch,
                }),
                Some(phase),
                window,
                cx,
            );
        });
        let state = self.state.clone();
        let hitbox = prepaint.0.clone();
        window.defer(cx, move |window, cx| {
            if state.borrow().route.handler == binding {
                hover(&state, &hitbox, window, cx);
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

/// Installed before descendant listeners, even when no region is mounted. GPUI
/// preserves its last in-window mouse position on MouseExited, so deferred hover
/// checks must not infer re-entry from that stale position after a repaint.
pub(super) fn install_pointer_presence(inside: Rc<Cell<bool>>, window: &mut Window) {
    let moved = inside.clone();
    window.on_mouse_event(move |_: &gpui::MouseMoveEvent, phase, _, _| {
        if phase.capture() {
            moved.set(true);
        }
    });
    let down = inside.clone();
    window.on_mouse_event(move |_: &gpui::MouseDownEvent, phase, _, _| {
        if phase.capture() {
            down.set(true);
        }
    });
    let up = inside.clone();
    window.on_mouse_event(move |_: &gpui::MouseUpEvent, phase, _, _| {
        if phase.capture() {
            up.set(true);
        }
    });
    let wheel = inside.clone();
    window.on_mouse_event(move |_: &gpui::ScrollWheelEvent, phase, _, _| {
        if phase.capture() {
            wheel.set(true);
        }
    });
    window.on_mouse_event(move |_: &gpui::MouseExitEvent, phase, _, _| {
        if phase.capture() {
            inside.set(false);
        }
    });
}
