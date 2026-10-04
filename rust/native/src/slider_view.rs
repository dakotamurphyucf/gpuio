//! Retained native slider ownership. Pointer/keyboard/AX mutations share one model;
//! only bounded observations cross the OCaml bridge.
use super::{View, choice::Route, pointer_enabled};
use crate::slider_state::{Adjustment, State as Model};
use gpui::{prelude::*, *};
use gpuio_protocol::slider_presentation::{Appearance, Fill};
use gpuio_protocol::{
    NodeId,
    numeric::Direction,
    slider::{Axis, CancelReason, Command, Error, Event, Response, Snapshot, Source, Thumb, Value},
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[path = "slider_ring.rs"]
mod ring;

pub(super) type Shared = Rc<RefCell<State>>;
pub(super) struct State {
    pub model: Model,
    pub(super) appearance: Arc<Appearance>,
    rings: ring::Rings,
    pub focus: Vec<(Thumb, FocusHandle)>,
    route: Route,
    bounds: Bounds<Pixels>,
    pub(super) track_bounds: Bounds<Pixels>,
    hitbox: Option<HitboxId>,
    capture: Option<HitboxId>,
    drag_offset: f64,
    closed: bool,
}
impl State {
    fn command(
        &mut self,
        command: Command,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Snapshot, Error> {
        if self.closed
            || !self
                .route
                .session
                .borrow()
                .tree(self.route.window)
                .and_then(|tree| tree.get(self.route.node))
                .is_some_and(|node| {
                    node.slider.is_some() && node.handler == Some(self.route.handler)
                })
        {
            return Err(Error::StaleSlider);
        }
        if !self.current() {
            return Err(Error::Busy);
        }
        if !command.is_valid() {
            return Err(Error::InvalidValue);
        }
        match command {
            Command::ReadSnapshot => (),
            Command::Focus(thumb) => {
                let focus = self
                    .focus
                    .iter()
                    .find(|(t, _)| *t == thumb)
                    .ok_or(Error::WrongThumb)?
                    .1
                    .clone();
                if self.model.config().disabled {
                    return Err(Error::Disabled);
                }
                if !self.allowed(false) {
                    return Err(Error::FocusBlocked);
                }
                window.focus(&focus, cx);
            }
            Command::Replace { value, if_revision } => {
                let events = self.model.replace(value, if_revision)?;
                self.release_capture(window);
                self.emit(events);
                window.refresh();
            }
            Command::CancelDrag => {
                let event = self.model.cancel(CancelReason::Programmatic)?;
                self.release_capture(window);
                if let Some(event) = event {
                    self.emit([event]);
                    window.refresh();
                }
            }
        }
        Ok(self.model.snapshot())
    }
    fn emit(&self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            let routed = self.route.session.borrow().slider_event(
                self.route.window,
                self.route.node,
                self.route.handler,
                self.route.revision,
                event,
            );
            if let Some(event) = routed
                && !self.route.transport.input(event)
                && self.route.session.borrow_mut().overload(self.route.window)
            {
                self.route.transport.fault(self.route.window);
            }
        }
    }
    fn current(&self) -> bool {
        !self.closed
            && self
                .route
                .session
                .borrow()
                .slider_event(
                    self.route.window,
                    self.route.node,
                    self.route.handler,
                    self.route.revision,
                    Event::Observed(self.model.snapshot()),
                )
                .is_some()
    }
    fn allowed(&self, pointer: bool) -> bool {
        self.current()
            && !self.model.config().disabled
            && self.route.gate.borrow().allows(self.route.node)
            && (!pointer
                || self
                    .route
                    .session
                    .borrow()
                    .tree(self.route.window)
                    .is_some_and(|tree| pointer_enabled(tree, self.route.node)))
    }
    fn release_capture(&mut self, window: &mut Window) {
        if let Some(capture) = self.capture.take()
            && window.captured_hitbox() == Some(capture)
        {
            window.release_pointer();
        }
    }
    pub(super) fn cancel(&mut self, reason: CancelReason, window: &mut Window) -> bool {
        self.release_capture(window);
        if let Ok(Some(event)) = self.model.cancel(reason) {
            self.emit([event]);
            window.refresh();
            true
        } else {
            false
        }
    }
    fn close(&mut self, window: &mut Window) {
        self.rings.reset();
        self.cancel(CancelReason::Unmounted, window);
        self.closed = true;
        for (_, focus) in &self.focus {
            focus.clone().tab_stop(false);
        }
    }
    fn fraction(&self, position: Point<Pixels>) -> f64 {
        let bounds = self.track_bounds;
        let (offset, length) = match self.model.config().axis {
            Axis::Horizontal => (position.x - bounds.left(), bounds.size.width),
            Axis::Vertical => (bounds.bottom() - position.y, bounds.size.height),
        };
        if length <= px(0.) {
            0.
        } else {
            f64::from(f32::from(offset)) / f64::from(f32::from(length))
        }
    }
    fn thumb_value(&self, thumb: Thumb) -> f64 {
        match (self.model.snapshot().value, thumb) {
            (Value::Single(v), Thumb::Single) => v,
            (Value::Range { lower, .. }, Thumb::Lower) => lower,
            (Value::Range { upper, .. }, Thumb::Upper) => upper,
            _ => unreachable!("fixed mounted thumb mode"),
        }
    }
    fn nearest(&self, fraction: f64, window: &Window) -> Thumb {
        self.focus
            .iter()
            .min_by(|(a, fa), (b, fb)| {
                let distance = |thumb| {
                    (self
                        .model
                        .config()
                        .fraction(self.thumb_value(thumb))
                        .unwrap()
                        - fraction)
                        .abs()
                };
                distance(*a)
                    .total_cmp(&distance(*b))
                    .then_with(|| fb.is_focused(window).cmp(&fa.is_focused(window)))
            })
            .expect("one or two thumbs")
            .0
    }
    fn begin(
        &mut self,
        thumb: Option<Thumb>,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.allowed(true)
            || self.model.config().read_only
            || window.captured_hitbox().is_some()
            || window.default_prevented()
        {
            return;
        }
        let Some(hitbox) = self.hitbox else {
            return;
        };
        let fraction = self.fraction(position);
        let selected = thumb.unwrap_or_else(|| self.nearest(fraction, window));
        let Ok(start) = self.model.begin(selected) else {
            return;
        };
        self.drag_offset = if thumb.is_some() {
            fraction
                - self
                    .model
                    .config()
                    .fraction(self.thumb_value(selected))
                    .unwrap()
        } else {
            0.
        };
        window.capture_pointer(hitbox);
        self.capture = Some(hitbox);
        let focus = &self.focus.iter().find(|(t, _)| *t == selected).unwrap().1;
        window.focus(focus, cx);
        self.emit([start]);
        self.preview(position);
        window.refresh();
        window.prevent_default();
        cx.stop_propagation();
    }
    fn preview(&mut self, position: Point<Pixels>) {
        if let Ok(Some(event)) = self
            .model
            .preview_fraction(self.fraction(position) - self.drag_offset)
        {
            self.emit([event]);
        }
    }
    fn adjust(
        &mut self,
        thumb: Thumb,
        adjustment: Adjustment,
        source: Source,
        window: &mut Window,
    ) {
        if !self.allowed(false) {
            return;
        }
        if let Ok(events) = self.model.adjust(thumb, adjustment, source) {
            self.release_capture(window);
            if !events.is_empty() {
                self.emit(events);
                window.refresh();
            }
        }
    }
    fn unavailable(&self) -> Option<CancelReason> {
        if !self.current() {
            Some(CancelReason::Unmounted)
        } else if !self.route.gate.borrow().visible(self.route.node) {
            Some(CancelReason::Hidden)
        } else if self.route.gate.borrow().disabled(self.route.node) {
            Some(CancelReason::Disabled)
        } else if !self.route.gate.borrow().allows(self.route.node) {
            Some(CancelReason::Modal)
        } else if !self
            .route
            .session
            .borrow()
            .tree(self.route.window)
            .is_some_and(|tree| pointer_enabled(tree, self.route.node))
        {
            Some(CancelReason::Interrupted)
        } else {
            None
        }
    }
}
impl View {
    pub(super) fn slider_command(
        &self,
        node: NodeId,
        command: Command,
        window: &mut Window,
        cx: &mut App,
    ) -> Response {
        let result = self
            .sliders
            .get(&node)
            .ok_or(Error::StaleSlider)
            .and_then(|state| state.borrow_mut().command(command, window, cx));
        match result {
            Ok(snapshot) => Response::Applied(snapshot),
            Err(error) => Response::Failed(error),
        }
    }
    pub(super) fn sync_sliders(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let nodes = self.session.borrow().tree(self.id).map(|tree| {
            (
                tree.revision(),
                dirty
                    .iter()
                    .filter_map(|id| tree.get(*id))
                    .filter(|node| node.slider.is_some())
                    .cloned()
                    .collect::<Vec<_>>(),
                self.sliders
                    .keys()
                    .copied()
                    .filter(|id| tree.get(*id).is_some_and(|node| node.slider.is_some()))
                    .collect::<std::collections::BTreeSet<_>>(),
            )
        });
        let Some((revision, nodes, present)) = nodes else {
            for state in self.sliders.values() {
                state.borrow_mut().close(window);
            }
            self.sliders.clear();
            return;
        };
        self.sliders.retain(|id, state| {
            if present.contains(id) {
                true
            } else {
                state.borrow_mut().close(window);
                false
            }
        });
        for node in nodes {
            let mount = node.slider.as_ref().expect("filtered slider");
            let route = Route {
                window: self.id,
                node: node.id,
                handler: node.handler.expect("validated slider"),
                revision,
                session: self.session.clone(),
                gate: self.focus.clone(),
                transport: self.transport.clone(),
            };
            if let Some(shared) = self.sliders.get(&node.id) {
                let mut state = shared.borrow_mut();
                if state.route.handler != route.handler {
                    state.cancel(CancelReason::Interrupted, window);
                }
                state.route = route;
                let appearance = node.slider_appearance.clone().unwrap_or_default();
                if state.appearance.target_size != appearance.target_size {
                    state.cancel(CancelReason::Interrupted, window);
                }
                state.appearance = appearance;
                match state.model.reconfigure(mount.config.clone()) {
                    Ok(events) => {
                        if state.model.snapshot().dragging.is_none() {
                            state.release_capture(window);
                        }
                        state.emit(events);
                    }
                    Err(_) => {
                        // Exhausted revisions cannot accept the new native policy.
                        // Stop this window's input rather than editing under an old one.
                        state.release_capture(window);
                        if state
                            .route
                            .session
                            .borrow_mut()
                            .overload(state.route.window)
                        {
                            state.route.transport.fault(state.route.window);
                        }
                    }
                }
            } else {
                let model =
                    Model::new(mount.config.clone(), mount.initial).expect("validated slider");
                let thumbs = match mount.initial {
                    Value::Single(_) => vec![Thumb::Single],
                    Value::Range { .. } => vec![Thumb::Lower, Thumb::Upper],
                };
                let state = State {
                    model,
                    appearance: node.slider_appearance.clone().unwrap_or_default(),
                    rings: ring::Rings::default(),
                    focus: thumbs
                        .into_iter()
                        .map(|thumb| (thumb, cx.focus_handle().tab_stop(true)))
                        .collect(),
                    route,
                    bounds: Bounds::default(),
                    track_bounds: Bounds::default(),
                    hitbox: None,
                    capture: None,
                    drag_offset: 0.,
                    closed: false,
                };
                state.emit([Event::Observed(state.model.snapshot())]);
                self.sliders.insert(node.id, Rc::new(RefCell::new(state)));
            }
        }
        for shared in self.sliders.values() {
            let mut state = shared.borrow_mut();
            if let Some(reason) = state.unavailable() {
                state.rings.reset();
                state.cancel(reason, window);
            }
            if state.model.config().disabled || state.model.config().read_only {
                state.rings.reset();
            }
        }
    }
    pub(super) fn cancel_slider_drags(&self, reason: CancelReason, window: &mut Window) -> bool {
        let mut cancelled = false;
        for state in self.sliders.values() {
            cancelled = state.borrow_mut().cancel(reason, window) || cancelled;
        }
        cancelled
    }
    pub(super) fn hide_unvisited_sliders(&self, window: &mut Window, cx: &mut App) {
        for (id, shared) in &self.sliders {
            if !self.visited.contains(id) {
                shared.borrow_mut().rings.reset();
            }
            if !self.visited.contains(id) && shared.borrow().model.snapshot().dragging.is_some() {
                let weak = Rc::downgrade(shared);
                window.defer(cx, move |window, _| {
                    if let Some(state) = weak.upgrade() {
                        state.borrow_mut().cancel(CancelReason::Hidden, window);
                    }
                });
            }
        }
    }
}

pub(super) fn element(
    mut base: Stateful<Div>,
    shared: Shared,
    pointer: bool,
    window: &Window,
) -> Stateful<Div> {
    shared.borrow_mut().rings.prepare();
    let state = shared.borrow();
    let config = state.model.config();
    let axis = config.axis;
    let appearance = &state.appearance;
    let target_size = px(appearance.target_size as f32);
    let thumb_size = px(appearance.thumb_size as f32);
    let ring_width = px(appearance.ring_width as f32);
    let thickness = px(appearance.track_thickness as f32);
    let radius = px(appearance.track_radius as f32);
    let track_color = appearance.track_color;
    let fill_color = appearance.fill_color;
    let thumb_color = appearance.thumb_color;
    let ring_color = appearance.ring_color;
    let disabled = config.disabled;
    let read_only = config.read_only;
    let focusable = state.allowed(false);
    base = base.role(Role::Group).aria_label(config.label.clone());
    let mut track = div()
        .absolute()
        .left(target_size / 2.)
        .right(target_size / 2.)
        .top(target_size / 2.)
        .bottom(target_size / 2.);
    let (low, high) = match state.model.snapshot().value {
        Value::Single(v) => {
            let fraction = config.fraction(v).unwrap();
            match appearance.fill {
                Fill::Selected => (0., fraction),
                Fill::Remaining => (fraction, 1.),
            }
        }
        Value::Range { lower, upper } => (
            config.fraction(lower).unwrap(),
            config.fraction(upper).unwrap(),
        ),
    };
    let geometry = shared.clone();
    track = track.child(
        canvas(
            move |bounds, window, _| {
                let mut state = geometry.borrow_mut();
                if state.track_bounds != bounds && state.capture.is_some() {
                    state.cancel(CancelReason::Interrupted, window);
                }
                state.track_bounds = bounds;
            },
            move |bounds, _, window, _| {
                let color = window.text_style().color;
                let bar = match axis {
                    Axis::Horizontal => Bounds::new(
                        point(bounds.left(), bounds.center().y - thickness / 2.),
                        size(bounds.size.width, thickness),
                    ),
                    Axis::Vertical => Bounds::new(
                        point(bounds.center().x - thickness / 2., bounds.top()),
                        size(thickness, bounds.size.height),
                    ),
                };
                let selected = match axis {
                    Axis::Horizontal => Bounds::new(
                        point(bar.left() + bar.size.width * low as f32, bar.top()),
                        size(bar.size.width * (high - low) as f32, bar.size.height),
                    ),
                    Axis::Vertical => Bounds::new(
                        point(bar.left(), bar.bottom() - bar.size.height * high as f32),
                        size(bar.size.width, bar.size.height * (high - low) as f32),
                    ),
                };
                let track_color =
                    track_color.map_or(color.opacity(0.25), |c| rgba(c as u32).into());
                let fill_color = fill_color.map_or(color, |c| rgba(c as u32).into());
                for (bounds, color) in [(bar, track_color), (selected, fill_color)] {
                    if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                        let mut quad = fill(bounds, color);
                        quad.corner_radii = radius.into();
                        window.paint_quad(quad);
                    }
                }
            },
        )
        .absolute()
        .size_full(),
    );
    for (part, (thumb, focus)) in state.focus.iter().enumerate() {
        let thumb = *thumb;
        let value = state.thumb_value(thumb);
        let fraction = config.fraction(value).unwrap() as f32;
        let label = match thumb {
            Thumb::Single => &config.label,
            Thumb::Lower => &config.lower_label,
            Thumb::Upper => &config.upper_label,
        };
        let (min, max) = match (state.model.snapshot().value, thumb) {
            (Value::Range { upper, .. }, Thumb::Lower) => (config.domain.min(), upper),
            (Value::Range { lower, .. }, Thumb::Upper) => (lower, config.domain.max()),
            _ => (config.domain.min(), config.domain.max()),
        };
        focus.clone().tab_stop(focusable);
        let mut child = div()
            .id(part)
            .absolute()
            .size(target_size)
            .role(Role::Slider)
            .aria_label(label.clone())
            .aria_numeric_value(value)
            .aria_min_numeric_value(min)
            .aria_max_numeric_value(max)
            .aria_numeric_value_step(config.domain.step())
            .aria_orientation(match axis {
                Axis::Horizontal => accesskit::Orientation::Horizontal,
                Axis::Vertical => accesskit::Orientation::Vertical,
            });
        child = match axis {
            Axis::Horizontal => child
                .left(relative(fraction))
                .top(relative(0.5))
                .ml(-target_size / 2.)
                .mt(-target_size / 2.),
            Axis::Vertical => child
                .bottom(relative(fraction))
                .left(relative(0.5))
                .mb(-target_size / 2.)
                .ml(-target_size / 2.),
        };
        if focusable {
            child = child.track_focus(focus).tab_index(0);
        }
        let focused = focusable && focus.is_focused(window);
        let ring_state = shared.clone();
        child = child.on_hover(|_, window, _| window.refresh()).child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |bounds, hitbox, window, cx| {
                    ring::paint(&ring_state, part, &hitbox, bounds, window, cx)
                },
            )
            .absolute()
            .size_full(),
        );
        child = child.child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let color = window.text_style().color;
                    let mut thumb = fill(
                        bounds.dilate(-(target_size - thumb_size) / 2.),
                        thumb_color.map_or(color, |c| rgba(c as u32).into()),
                    );
                    thumb.corner_radii = (thumb_size / 2.).into();
                    window.paint_quad(thumb);
                    if focused && ring_width > px(0.) {
                        let mut ring = outline(
                            bounds,
                            ring_color.map_or(color, |c| rgba(c as u32).into()),
                            BorderStyle::Solid,
                        );
                        ring.corner_radii = (target_size / 2.).into();
                        ring.border_widths = ring_width.into();
                        window.paint_quad(ring);
                    }
                },
            )
            .absolute()
            .size_full(),
        );
        if pointer && focusable && !read_only {
            let down = shared.clone();
            child = child.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                move |event, window, cx| {
                    down.borrow_mut()
                        .begin(Some(thumb), event.position, window, cx);
                },
            );
        }
        let key = shared.clone();
        child = child.on_key_down(move |event, window, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            let adjustment = match event.keystroke.key.as_str() {
                "right" | "up" => Adjustment::Step {
                    direction: Direction::Increase,
                    page: false,
                },
                "left" | "down" => Adjustment::Step {
                    direction: Direction::Decrease,
                    page: false,
                },
                "pageup" => Adjustment::Step {
                    direction: Direction::Increase,
                    page: true,
                },
                "pagedown" => Adjustment::Step {
                    direction: Direction::Decrease,
                    page: true,
                },
                "home" => Adjustment::First,
                "end" => Adjustment::Last,
                _ => return,
            };
            key.borrow_mut()
                .adjust(thumb, adjustment, Source::Keyboard, window);
            cx.stop_propagation();
        });
        let focus_state = shared.clone();
        let focus_handle = focus.clone();
        child = child.on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
            if focus_state.borrow().allowed(false) {
                window.focus(&focus_handle, cx);
            }
        });
        if !disabled && !read_only {
            for (action, direction) in [
                (AccessibleAction::Increment, Direction::Increase),
                (AccessibleAction::Decrement, Direction::Decrease),
            ] {
                let access = shared.clone();
                child = child.on_a11y_action(action, move |_, window, cx| {
                    access.borrow_mut().adjust(
                        thumb,
                        Adjustment::Step {
                            direction,
                            page: false,
                        },
                        Source::Accessibility,
                        window,
                    );
                    cx.stop_propagation();
                });
            }
            let access = shared.clone();
            child = child.on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
                if let Some(accesskit::ActionData::NumericValue(value)) = data {
                    access.borrow_mut().adjust(
                        thumb,
                        Adjustment::Set(*value),
                        Source::Accessibility,
                        window,
                    );
                }
                cx.stop_propagation();
            });
        }
        let gate = state.route.gate.clone();
        let node = state.route.node;
        let record = focus.clone();
        child = child.child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                        gate.borrow_mut().record_part(
                            node,
                            part as u16,
                            super::focus::Target {
                                handle: record.clone(),
                                tab_stop: focusable,
                                bounds,
                            },
                            record.is_focused(window),
                        );
                    }
                },
            )
            .absolute()
            .size_full(),
        );
        track = track.child(crate::semantics::State {
            identity: None,
            busy: false,
            element: child,
            metadata: None,
            live: None,
            disabled,
            read_only,
            hidden: !state.route.gate.borrow().visible(node),
            modal: false,
        });
    }
    base.child(track)
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
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
        let mut state = self.state.borrow_mut();
        if let Some(capture) = state.capture {
            if window.captured_hitbox() == Some(capture)
                && state.allowed(true)
                && state.bounds == bounds
            {
                window.capture_pointer(hitbox.id);
                state.capture = Some(hitbox.id);
            } else {
                state.cancel(CancelReason::Interrupted, window);
            }
        }
        state.bounds = bounds;
        state.hitbox = Some(hitbox.id);
        drop(state);
        (
            hitbox,
            self.element
                .prepaint(id, inspector, bounds, layout, window, cx),
        )
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
        let down = self.state.clone();
        let hitbox = prepaint.0.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase.bubble() && event.button == MouseButton::Left && hitbox.is_hovered(window) {
                down.borrow_mut().begin(None, event.position, window, cx);
            }
        });
        let moved = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if !phase.capture() {
                return;
            }
            let mut state = moved.borrow_mut();
            let Some(capture) = state.capture else {
                return;
            };
            if !state.allowed(true)
                || window.captured_hitbox() != Some(capture)
                || event.pressed_button != Some(MouseButton::Left)
            {
                state.cancel(CancelReason::Interrupted, window);
                return;
            }
            state.preview(event.position);
            window.refresh();
            cx.stop_propagation();
        });
        self.element
            .paint(id, inspector, bounds, layout, &mut prepaint.1, window, cx);
        let up = self.state.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            let mut state = up.borrow_mut();
            let Some(capture) = state.capture else {
                return;
            };
            if !state.allowed(true)
                || window.captured_hitbox() != Some(capture)
                || event.button != MouseButton::Left
            {
                state.cancel(CancelReason::Interrupted, window);
                return;
            }
            state.preview(event.position);
            if let Ok(Some(event)) = state.model.finish() {
                state.emit([event]);
            }
            state.release_capture(window);
            window.refresh();
            window.prevent_default();
            cx.stop_propagation();
        });
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
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

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "slider_presentation_test.rs"]
mod presentation_test;
