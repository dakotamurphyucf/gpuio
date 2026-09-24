//! Synchronous native pointer capture. Gesture state is window-owned; immutable
//! samples cross the bridge and consecutive motion can coalesce in its mailbox.
use super::{View, choice::Route};
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Context, Element, ElementId, GlobalElementId, Hitbox,
    HitboxBehavior, HitboxId, InspectorElementId, IntoElement, LayoutId, Pixels, Window,
};
use gpuio_protocol::{NodeId, v1::*};
use std::{cell::RefCell, rc::Rc, sync::Arc};
pub(super) type Shared = Rc<RefCell<Capture>>;
#[derive(Default)]
pub(super) struct Capture {
    serial: i64,
    active: Option<Gesture>,
}
struct Gesture {
    route: Route,
    config: Arc<PointerConfig>,
    hitbox: HitboxId,
    sample: PointerSample,
}
fn button(value: gpui::MouseButton) -> PointerButton {
    match value {
        gpui::MouseButton::Left => PointerButton::Left,
        gpui::MouseButton::Right => PointerButton::Right,
        gpui::MouseButton::Middle => PointerButton::Middle,
        gpui::MouseButton::Navigate(gpui::NavigationDirection::Back) => PointerButton::Back,
        gpui::MouseButton::Navigate(gpui::NavigationDirection::Forward) => PointerButton::Forward,
    }
}
fn modifiers(value: gpui::Modifiers) -> PointerModifiers {
    PointerModifiers {
        shift: value.shift,
        control: value.control,
        alt: value.alt,
        command: value.platform,
        function: value.function,
    }
}
fn position(
    sample: &mut PointerSample,
    point: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
    keys: gpui::Modifiers,
) {
    sample.window_x = f32::from(point.x) as f64;
    sample.window_y = f32::from(point.y) as f64;
    sample.local_x = f32::from(point.x - bounds.origin.x) as f64;
    sample.local_y = f32::from(point.y - bounds.origin.y) as f64;
    sample.modifiers = modifiers(keys);
}
fn publish(route: &Route, sample: PointerSample) {
    let event = route.session.borrow().pointer_event(
        route.window,
        route.node,
        route.handler,
        route.revision,
        sample,
    );
    if let Some(event) = event
        && !route.transport.input(event)
        && route.session.borrow_mut().overload(route.window)
    {
        route.transport.fault(route.window);
    }
}
fn unavailable(route: &Route, initiating_button: PointerButton) -> Option<PointerCancel> {
    let session = route.session.borrow();
    let Some(tree) = session.tree(route.window) else {
        return Some(PointerCancel::Removed);
    };
    let Some(node) = tree.get(route.node) else {
        return Some(PointerCancel::Removed);
    };
    let Some(config) = &node.pointer else {
        return Some(PointerCancel::Removed);
    };
    if node.handler != Some(route.handler) || config.button != initiating_button {
        return Some(PointerCancel::Reconfigured);
    }
    if config.disabled {
        return Some(PointerCancel::Disabled);
    }
    if !route.gate.borrow().visible(route.node) {
        return Some(PointerCancel::Hidden);
    }
    if !route.gate.borrow().allows(route.node) {
        return Some(PointerCancel::Blocked);
    }
    // Nearest declaration wins; walk toward the root without allocating a
    // temporary ancestor vector for each mouse sample.
    let mut cursor = Some(route.node);
    while let Some(id) = cursor {
        let Some(node) = tree.get(id) else {
            return Some(PointerCancel::Removed);
        };
        for style in node.style.iter().rev() {
            if let Style::Fields(fields) = style {
                for field in fields.iter().rev() {
                    if let Field::PointerEvents(value) = field {
                        return (!value).then_some(PointerCancel::Disabled);
                    }
                }
            }
        }
        cursor = node.parent;
    }
    None
}
impl Capture {
    pub(super) fn is_active(&self, node: NodeId) -> bool {
        self.active
            .as_ref()
            .is_some_and(|gesture| gesture.route.node == node)
    }
    pub(super) fn cancel(&mut self, reason: PointerCancel, window: &mut Window) -> bool {
        let Some(mut gesture) = self.active.take() else {
            return false;
        };
        if window.captured_hitbox() == Some(gesture.hitbox) {
            window.release_pointer();
        }
        window.refresh();
        gesture.sample.phase = PointerPhase::Cancelled(reason);
        publish(&gesture.route, gesture.sample);
        true
    }
    pub(super) fn sync(&mut self, window: &mut Window) {
        if let Some(gesture) = &self.active {
            let reason = unavailable(&gesture.route, gesture.config.button).or_else(|| {
                (window.captured_hitbox() != Some(gesture.hitbox))
                    .then_some(PointerCancel::CaptureLost)
            });
            if let Some(reason) = reason {
                self.cancel(reason, window);
            }
        }
    }
    fn rebind(&mut self, node: NodeId, hitbox: HitboxId, window: &mut Window) {
        self.sync(window);
        if let Some(gesture) = &mut self.active
            && gesture.route.node == node
        {
            window.capture_pointer(hitbox);
            gesture.hitbox = hitbox;
        }
    }
    fn begin(
        &mut self,
        route: &Route,
        config: &Arc<PointerConfig>,
        hitbox: &Hitbox,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.active.is_some()
            || window.captured_hitbox().is_some()
            || window.default_prevented()
            || !hitbox.is_hovered(window)
            || button(event.button) != config.button
            || unavailable(route, config.button).is_some()
        {
            return;
        }
        let Some(serial) = self.serial.checked_add(1) else {
            return;
        };
        self.serial = serial;
        let mut sample = PointerSample {
            gesture: serial,
            phase: PointerPhase::Started,
            button: config.button,
            window_x: 0.,
            window_y: 0.,
            local_x: 0.,
            local_y: 0.,
            modifiers: Default::default(),
        };
        position(&mut sample, event.position, hitbox.bounds, event.modifiers);
        window.capture_pointer(hitbox.id);
        self.active = Some(Gesture {
            route: route.clone(),
            config: config.clone(),
            hitbox: hitbox.id,
            sample,
        });
        window.refresh();
        publish(route, sample);
        policy(config, window, cx);
    }
    fn motion(
        &mut self,
        node: NodeId,
        bounds: Bounds<Pixels>,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.sync(window);
        let Some(gesture) = &mut self.active else {
            return;
        };
        if gesture.route.node != node {
            return;
        }
        if event.pressed_button.map(button) != Some(gesture.config.button) {
            self.cancel(PointerCancel::CaptureLost, window);
            return;
        }
        gesture.sample.phase = PointerPhase::Moved;
        position(&mut gesture.sample, event.position, bounds, event.modifiers);
        publish(&gesture.route, gesture.sample);
        policy(&gesture.config, window, cx);
    }
    fn release(
        &mut self,
        node: NodeId,
        bounds: Bounds<Pixels>,
        event: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.sync(window);
        if self
            .active
            .as_ref()
            .is_none_or(|gesture| gesture.route.node != node)
        {
            return;
        }
        if self.active.as_ref().unwrap().config.button != button(event.button) {
            // GPUI releases capture after every MouseUp, including a different button.
            self.cancel(PointerCancel::CaptureLost, window);
            return;
        }
        let mut gesture = self.active.take().unwrap();
        window.release_pointer();
        window.refresh();
        gesture.sample.phase = PointerPhase::Released;
        position(&mut gesture.sample, event.position, bounds, event.modifiers);
        publish(&gesture.route, gesture.sample);
        policy(&gesture.config, window, cx);
    }
}
fn policy(config: &PointerConfig, window: &mut Window, cx: &mut App) {
    if config.prevent_default {
        window.prevent_default();
    }
    if config.stop_propagation {
        cx.stop_propagation();
    }
}

pub(super) struct Region<E> {
    pub element: E,
    pub capture: Shared,
    pub route: Route,
    pub config: Arc<PointerConfig>,
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
        // Before child prepaint so descendants retain normal hit-test precedence.
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
        self.capture
            .borrow_mut()
            .rebind(self.route.node, hitbox.id, window);
        let child = self
            .element
            .prepaint(id, inspector, bounds, layout, window, cx);
        (hitbox, child)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let capture = self.capture.clone();
        let route = self.route.clone();
        let config = self.config.clone();
        let hitbox = prepaint.0.clone();
        window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, window, cx| {
            if phase.bubble() {
                capture
                    .borrow_mut()
                    .begin(&route, &config, &hitbox, event, window, cx);
            }
        });
        let capture = self.capture.clone();
        let node = self.route.node;
        let bounds = prepaint.0.bounds;
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
            if phase.capture() {
                capture.borrow_mut().motion(node, bounds, event, window, cx);
            }
        });
        // Register after child paint so our release runs before descendant click
        // handlers in reverse-order bubbling. Wait until bubble phase to let all
        // native listeners clear pending click/pressed state during capture.
        self.element
            .paint(id, inspector, bounds, layout, &mut prepaint.1, window, cx);
        let capture = self.capture.clone();
        window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
            if phase.bubble() {
                capture
                    .borrow_mut()
                    .release(node, bounds, event, window, cx);
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
impl View {
    pub(super) fn install_pointer_observer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pointer_activation.is_none() {
            self.pointer_activation =
                Some(cx.observe_window_activation(window, |view, window, cx| {
                    if !window.is_window_active() {
                        super::drag_drop::cancel(
                            view.id,
                            gpuio_protocol::drag_drop::CancelReason::WindowInactive,
                            window,
                            cx,
                        );
                        view.pointer_capture
                            .borrow_mut()
                            .cancel(PointerCancel::WindowInactive, window);
                    }
                }));
        }
    }
}
