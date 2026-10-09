//! Native measured flat-group widget. The bridge supplies accepted descriptions
//! and an asynchronous observer; no OCaml callback participates in layout/input.
use crate::split_group_state::{self as model, State as Model};
use gpui::{
    App, Bounds, Context, Element, ElementId, FocusHandle, GlobalElementId, Hitbox, HitboxBehavior,
    HitboxId, InspectorElementId, IntoElement, LayoutId, MouseButton, Pixels, Point, Window,
    canvas, div, prelude::*, px,
};
use gpui_base::ElementExt as _;
use gpuio_protocol::{
    split::Axis,
    split_group::{Config, Snapshot, Source},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
    sync::Arc,
};
pub type Shared = Rc<RefCell<State>>;
pub type Observer = Rc<dyn Fn(Snapshot, &mut Window, &mut App)>;
pub type FocusObserver = Rc<dyn Fn(&str, &FocusHandle, Bounds<Pixels>, &mut Window, &mut App)>;
pub type VisibilityObserver = Rc<dyn Fn(&str, bool, &mut Window, &mut App)>;
pub type PanelDecorator = Rc<dyn Fn(&str, gpui::Stateful<gpui::Div>) -> gpui::AnyElement>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    pub visible: bool,
    pub enabled: bool,
    pub pointer: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            visible: true,
            enabled: true,
            pointer: true,
        }
    }
}
struct Capture {
    after: String,
    origin: Point<Pixels>,
    hitbox: HitboxId,
}
pub struct State {
    model: Model,
    scope: FocusHandle,
    focus: BTreeMap<String, FocusHandle>,
    policy: Policy,
    closed: bool,
    capture: Option<Capture>,
    bounds: Option<Bounds<Pixels>>,
    painted: bool,
    activation: Option<gpui::Subscription>,
    hovered: Option<String>,
    visible_handles: BTreeSet<String>,
}
impl State {
    pub fn new<T: 'static>(
        config: Arc<Config>,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> Result<Shared, model::Error> {
        let state = Rc::new(RefCell::new(Self {
            model: Model::new(config)?,
            scope: cx.focus_handle().tab_stop(false),
            focus: BTreeMap::new(),
            policy: Policy::default(),
            closed: false,
            capture: None,
            bounds: None,
            painted: false,
            activation: None,
            hovered: None,
            visible_handles: BTreeSet::new(),
        }));
        let weak = Rc::downgrade(&state);
        state.borrow_mut().activation =
            Some(cx.observe_window_activation(window, move |_, window, _| {
                if !window.is_window_active()
                    && let Some(state) = weak.upgrade()
                {
                    state.borrow_mut().cancel(window);
                }
            }));
        Ok(state)
    }
    pub fn reconcile(
        &mut self,
        config: Arc<Config>,
        policy: Policy,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), model::Error> {
        if self.model.reconcile(config)? {
            self.release(window);
        }
        self.policy = policy;
        if !self.allowed(true) {
            self.cancel(window);
        }
        let visible: Vec<_> = self
            .model
            .config()
            .panels
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.id.as_str())
            .collect();
        let handles = visible
            .get(..visible.len().saturating_sub(1))
            .unwrap_or_default();
        for (id, focus) in &self.focus {
            let active = self.allowed(false) && handles.contains(&id.as_str());
            focus.clone().tab_stop(active);
            if !active && focus.is_focused(window) {
                window.blur(cx);
            }
        }
        self.focus
            .retain(|id, _| self.model.config().panels.iter().any(|p| &p.id == id));
        Ok(())
    }
    fn allowed(&self, pointer: bool) -> bool {
        !self.closed
            && self.policy.visible
            && self.policy.enabled
            && (!pointer || self.policy.pointer)
    }
    fn allowed_handle(&self, after: &str, pointer: bool) -> bool {
        self.allowed(pointer) && self.visible_handles.contains(after)
    }
    fn release(&mut self, window: &mut Window) {
        if let Some(c) = self.capture.take()
            && window.captured_hitbox() == Some(c.hitbox)
        {
            window.release_pointer();
        }
    }
    pub fn cancel(&mut self, window: &mut Window) {
        let changed = self.model.cancel_drag();
        self.release(window);
        if changed {
            window.refresh();
        }
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut App) {
        self.cancel(window);
        self.closed = true;
        self.activation = None;
        for f in self.focus.values() {
            f.clone().tab_stop(false);
            if f.is_focused(window) {
                window.blur(cx);
            }
        }
        self.focus.clear();
    }
    pub fn begin_frame(&mut self) {
        self.painted = false;
    }
    pub fn finish_frame(&mut self, window: &mut Window, cx: &mut App) {
        if !self.painted {
            self.cancel(window);
            let _ = self.model.measure(0.);
            self.bounds = None;
            for focus in self.focus.values() {
                focus.clone().tab_stop(false);
                if focus.is_focused(window) {
                    window.blur(cx);
                }
            }
        }
    }
    pub fn focus(&self, after: &str) -> Option<&FocusHandle> {
        self.focus.get(after)
    }
    pub fn focused(&self, window: &Window) -> bool {
        self.focus.values().any(|focus| focus.is_focused(window))
    }
    pub fn contains_focused(&self, window: &Window, cx: &App) -> bool {
        self.scope.contains_focused(window, cx)
    }
    pub fn is_dragging(&self) -> bool {
        self.model.is_dragging()
    }
    fn set_bounds(&mut self, bounds: Bounds<Pixels>, window: &mut Window) {
        if self.bounds.is_some_and(|previous| previous != bounds) {
            self.cancel(window);
        }
        self.bounds = Some(bounds);
    }
    fn preview(&mut self, position: Point<Pixels>) {
        if let Some(c) = &self.capture {
            let delta = match self.model.config().axis {
                Axis::Horizontal => position.x - c.origin.x,
                Axis::Vertical => position.y - c.origin.y,
            };
            let _ = self.model.drag_to(f64::from(f32::from(delta)));
        }
    }
    fn begin(
        &mut self,
        after: &str,
        hitbox: HitboxId,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.allowed_handle(after, true)
            || !window.is_window_active()
            || self.model.begin_drag(after).is_err()
        {
            return;
        }
        self.release(window);
        self.capture = Some(Capture {
            after: after.into(),
            origin: position,
            hitbox,
        });
        window.capture_pointer(hitbox);
        if let Some(focus) = self.focus.get(after) {
            window.focus(focus, cx);
        }
        window.prevent_default();
        window.refresh();
        cx.stop_propagation();
    }
}
pub use gpuio_protocol::split_group_appearance::Config as Appearance;
pub struct Content {
    pub panel: gpui::AnyElement,
    pub handle: Option<gpui::AnyElement>,
}
pub struct Render {
    pub state: Shared,
    pub contents: Vec<Content>,
    pub appearance: Arc<Appearance>,
    pub observe: Observer,
    pub record_focus: FocusObserver,
    pub record_visibility: VisibilityObserver,
    pub decorate_panel: PanelDecorator,
}
/// Parent owns begin/finish-frame sweeping and close. All event closures and
/// measured content hold weak state, so old frames cannot retain a retired owner.
pub fn element(render: Render) -> Result<gpui::AnyElement, model::Error> {
    let Render {
        state,
        contents,
        appearance,
        observe,
        record_focus,
        record_visibility,
        decorate_panel,
    } = render;
    if contents.len() != state.borrow().model.config().panels.len()
        || crate::split_group_appearance::validate(&appearance).is_err()
    {
        return Err(model::Error::InvalidConfig);
    }
    state.borrow_mut().visible_handles.clear();
    let weak = Rc::downgrade(&state);
    let bounds_state = weak.clone();
    let scope = state.borrow().scope.clone();
    let label = state.borrow().model.config().label.clone();
    Ok(div()
        .id("split-group-body")
        .track_focus(&scope)
        .role(gpui::Role::Group)
        .aria_label(label)
        .size_full()
        .overflow_hidden()
        .on_prepaint(move |bounds, w, _| {
            if let Some(state) = bounds_state.upgrade() {
                state.borrow_mut().set_bounds(bounds, w);
            }
        })
        .child(gpui::container_query(move |size, w, cx| {
            let mut root = div().size_full().relative().overflow_hidden();
            let Some(state) = weak.upgrade() else {
                return root;
            };
            let mut owner = state.borrow_mut();
            let visible = owner
                .bounds
                .map(|bounds| bounds.intersect(&w.content_mask().bounds));
            if owner.closed
                || !owner.policy.visible
                || size.width <= px(0.)
                || size.height <= px(0.)
                || visible.is_none_or(|r| r.size.width <= px(0.) || r.size.height <= px(0.))
            {
                owner.cancel(w);
                let _ = owner.model.measure(0.);
                return root;
            }
            let axis = owner.model.config().axis;
            let extent = match axis {
                Axis::Horizontal => size.width,
                Axis::Vertical => size.height,
            };
            let Some(measured) = owner
                .model
                .measure(f64::from(f32::from(extent)))
                .ok()
                .flatten()
            else {
                owner.release(w);
                return root;
            };
            if measured.cancelled_drag {
                owner.release(w);
            }
            let config = owner.model.config().clone();
            let visible: Vec<_> = config
                .panels
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.visible.then_some(i))
                .collect();
            let mut offset = 0.;
            let mut handles = vec![];
            let mut visibility = Vec::with_capacity(config.panels.len());
            for (i, content) in contents.into_iter().enumerate() {
                let p = &config.panels[i];
                if !p.visible {
                    visibility.push((p.id.clone(), false));
                    continue;
                }
                let extent = measured.layout.sizes[i] as f32;
                let mut panel = div()
                    .id(gpui::SharedString::from(format!("panel:{}", p.id)))
                    .absolute()
                    .overflow_hidden()
                    .child(content.panel);
                panel = match axis {
                    Axis::Horizontal => panel.left(px(offset)).top_0().w(px(extent)).h_full(),
                    Axis::Vertical => panel.top(px(offset)).left_0().h(px(extent)).w_full(),
                };
                let mut rect = owner.bounds.expect("measured group");
                match axis {
                    Axis::Horizontal => {
                        rect.origin.x += px(offset);
                        rect.size.width = px(extent);
                    }
                    Axis::Vertical => {
                        rect.origin.y += px(offset);
                        rect.size.height = px(extent);
                    }
                }
                let rect = rect
                    .intersect(&owner.bounds.unwrap())
                    .intersect(&w.content_mask().bounds);
                let shown = rect.size.width > px(0.) && rect.size.height > px(0.);
                visibility.push((p.id.clone(), shown));
                root = root.child(crate::semantics::State {
                    // The decorator erases the panel's outer element identity.
                    // Keep an identified ancestor for clipped accessibility content.
                    identity: Some(
                        gpui::SharedString::from(format!("panel-semantics:{}", p.id)).into(),
                    ),
                    busy: false,
                    hidden: !shown,
                    metadata: None,
                    live: None,
                    element: decorate_panel(&p.id, panel),
                    disabled: false,
                    read_only: false,
                    modal: false,
                });
                offset += extent;
                if visible.last() == Some(&i) {
                    continue;
                }
                let next = visible.iter().copied().find(|&j| j > i).unwrap();
                let focus = owner
                    .focus
                    .entry(p.id.clone())
                    .or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                focus.clone().tab_stop(owner.allowed(false));
                let range = owner
                    .model
                    .boundary_range(&p.id)
                    .expect("measured visible boundary");
                handles.push(handle(
                    &state,
                    &p.id,
                    format!("Resize {} and {}", p.label, config.panels[next].label),
                    axis,
                    offset,
                    range,
                    focus,
                    content.handle,
                    &appearance,
                    observe.clone(),
                    record_focus.clone(),
                    &owner,
                    w,
                ));
            }
            drop(owner);
            for handle in handles {
                root = root.child(handle);
            }
            let painted = Rc::downgrade(&state);
            let observe = observe.clone();
            let mut observation = measured.observation;
            root.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, cx| {
                        if let Some(state) = painted.upgrade() {
                            let live = {
                                let mut state = state.borrow_mut();
                                state.painted = true;
                                !state.closed
                            };
                            if live && let Some(snapshot) = observation.take() {
                                observe(snapshot, w, cx);
                            }
                            if live {
                                for (id, shown) in &visibility {
                                    record_visibility(id, *shown, w, cx);
                                }
                            }
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
        }))
        .into_any_element())
}
#[expect(
    clippy::too_many_arguments,
    reason = "one measured native splitter and its paint/event context"
)]
fn handle(
    state: &Shared,
    after: &str,
    label: String,
    axis: Axis,
    offset: f32,
    range: (f64, f64, f64),
    focus: FocusHandle,
    decoration: Option<gpui::AnyElement>,
    appearance: &Appearance,
    observe: Observer,
    record: FocusObserver,
    owner: &State,
    window: &Window,
) -> gpui::AnyElement {
    let enabled = owner.allowed(false);
    let pointer = owner.allowed(true);
    let active = owner.capture.as_ref().is_some_and(|c| c.after == after);
    let hovered = owner.hovered.as_deref() == Some(after);
    let focused = focus.is_focused(window);
    let color = window
        .text_style()
        .color
        .alpha(if active || hovered || focused {
            1.
        } else {
            0.3
        });
    let mut target = div()
        .id(gpui::SharedString::from(format!("handle:{after}")))
        .absolute()
        .flex()
        .items_center()
        .justify_center()
        .track_focus(&focus)
        .tab_index(0)
        .role(gpui::Role::Splitter)
        .aria_label(label)
        .aria_numeric_value(range.1)
        .aria_min_numeric_value(range.0)
        .aria_max_numeric_value(range.2)
        .aria_numeric_value_step(owner.model.config().keyboard_step)
        .aria_orientation(if axis == Axis::Horizontal {
            gpui::accesskit::Orientation::Vertical
        } else {
            gpui::accesskit::Orientation::Horizontal
        });
    target = match axis {
        Axis::Horizontal => target
            .left(px(offset - appearance.hit_extent as f32 / 2.))
            .top_0()
            .w(px(appearance.hit_extent as f32))
            .h_full(),
        Axis::Vertical => target
            .top(px(offset - appearance.hit_extent as f32 / 2.))
            .left_0()
            .h(px(appearance.hit_extent as f32))
            .w_full(),
    };
    if pointer {
        target = target.cursor(if axis == Axis::Horizontal {
            gpui::CursorStyle::ResizeLeftRight
        } else {
            gpui::CursorStyle::ResizeUpDown
        });
    }
    let mut line = match decoration {
        Some(content) => div().child(crate::semantics::State {
            identity: None,
            busy: false,
            hidden: true,
            metadata: None,
            live: None,
            element: div().id("split-grip-decoration").child(content),
            disabled: false,
            read_only: false,
            modal: false,
        }),
        None => {
            let line = div().bg(color);
            if axis == Axis::Horizontal {
                line.w(px(appearance.thickness as f32)).h_full()
            } else {
                line.h(px(appearance.thickness as f32)).w_full()
            }
        }
    };
    if !enabled {
        line = line.opacity(0.5);
    }
    line.style()
        .refine(&crate::split_group_appearance::refinement(
            appearance, after, hovered, focused, active, !enabled,
        ));
    target = target.child(line);
    let weak = Rc::downgrade(state);
    let key_id = after.to_owned();
    let key_observe = observe.clone();
    let key_focus = focus.clone();
    target = target.on_key_down(move |event, w, cx| {
        if !key_focus.is_focused(w) || event.keystroke.modifiers.modified() {
            return;
        }
        let Some(state) = weak.upgrade() else {
            return;
        };
        let mut s = state.borrow_mut();
        if !s.allowed_handle(&key_id, false) {
            return;
        }
        if event.keystroke.key == "escape" && s.model.is_dragging() {
            s.cancel(w);
            cx.stop_propagation();
            return;
        }
        let step = s.model.config().keyboard_step;
        let delta = match (s.model.config().axis, event.keystroke.key.as_str()) {
            (_, "home") => s.model.boundary_range(&key_id).ok().map(|r| r.0 - r.1),
            (_, "end") => s.model.boundary_range(&key_id).ok().map(|r| r.2 - r.1),
            (Axis::Horizontal, "left") | (Axis::Vertical, "up") => Some(-step),
            (Axis::Horizontal, "right") | (Axis::Vertical, "down") => Some(step),
            _ => None,
        };
        if let Some(delta) = delta {
            let snapshot = s
                .model
                .adjust_boundary(&key_id, delta, Source::Keyboard)
                .ok()
                .flatten();
            s.release(w);
            drop(s);
            if let Some(snapshot) = snapshot {
                key_observe(snapshot, w, cx);
            }
            w.refresh();
            cx.stop_propagation();
        }
    });
    for action in [
        gpui::AccessibleAction::Focus,
        gpui::AccessibleAction::Increment,
        gpui::AccessibleAction::Decrement,
        gpui::AccessibleAction::SetValue,
    ] {
        let weak = Rc::downgrade(state);
        let id = after.to_owned();
        let observe = observe.clone();
        target = target.on_a11y_action(action, move |data, w, cx| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if !s.allowed_handle(&id, false) {
                return;
            }
            if action == gpui::AccessibleAction::Focus {
                if let Some(f) = s.focus.get(&id) {
                    w.focus(f, cx);
                }
                return;
            }
            let delta = match action {
                gpui::AccessibleAction::Increment => Some(s.model.config().keyboard_step),
                gpui::AccessibleAction::Decrement => Some(-s.model.config().keyboard_step),
                gpui::AccessibleAction::SetValue => {
                    if let Some(gpui::accesskit::ActionData::NumericValue(v)) = data {
                        s.model.boundary_range(&id).ok().map(|r| v - r.1)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(delta) = delta {
                let snapshot = s
                    .model
                    .adjust_boundary(&id, delta, Source::Accessibility)
                    .ok()
                    .flatten();
                s.release(w);
                drop(s);
                if let Some(snapshot) = snapshot {
                    observe(snapshot, w, cx);
                }
                w.refresh();
                cx.stop_propagation();
            }
        });
    }
    let id = after.to_owned();
    let record_focus = focus.clone();
    let record_state = Rc::downgrade(state);
    target = target.child(
        canvas(
            |_, _, _| (),
            move |bounds, _, w, cx| {
                if record_state
                    .upgrade()
                    .is_some_and(|state| state.borrow().visible_handles.contains(&id))
                {
                    record(&id, &record_focus, bounds, w, cx);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full(),
    );
    // GPUI collects the element's accessibility metadata before prepaint. Use
    // the already measured group rectangle for the initial semantic visibility;
    // Region rechecks the actual hitbox bounds during prepaint for input/focus.
    let visible = owner.bounds.is_some_and(|group| {
        let mut bounds = group;
        match axis {
            Axis::Horizontal => {
                bounds.origin.x += px(offset - appearance.hit_extent as f32 / 2.);
                bounds.size.width = px(appearance.hit_extent as f32);
            }
            Axis::Vertical => {
                bounds.origin.y += px(offset - appearance.hit_extent as f32 / 2.);
                bounds.size.height = px(appearance.hit_extent as f32);
            }
        }
        let visible = bounds
            .intersect(&group)
            .intersect(&window.content_mask().bounds);
        visible.size.width > px(0.) && visible.size.height > px(0.)
    });
    Region {
        element: crate::semantics::State {
            identity: None,
            busy: false,
            hidden: false,
            metadata: None,
            live: None,
            element: target,
            disabled: !enabled,
            read_only: false,
            modal: false,
        },
        state: Rc::downgrade(state),
        after: after.into(),
        observe,
        visible,
    }
    .into_any_element()
}

struct Region<E> {
    element: E,
    state: Weak<RefCell<State>>,
    after: String,
    observe: Observer,
    visible: bool,
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
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, w, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        w: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let visible = bounds.intersect(&w.content_mask().bounds);
        self.visible = visible.size.width > px(0.) && visible.size.height > px(0.);
        if let Some(state) = self.state.upgrade() {
            let mut s = state.borrow_mut();
            if self.visible {
                s.visible_handles.insert(self.after.clone());
            } else {
                s.visible_handles.remove(&self.after);
                if s.capture.as_ref().is_some_and(|c| c.after == self.after) {
                    s.cancel(w);
                }
                if let Some(focus) = s.focus.get(&self.after) {
                    focus.clone().tab_stop(false);
                    if focus.is_focused(w) {
                        w.blur(cx);
                    }
                }
            }
        }
        let hitbox = w.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
        if let Some(state) = self.state.upgrade() {
            let mut s = state.borrow_mut();
            if let Some(c) = &s.capture
                && c.after == self.after
            {
                if w.captured_hitbox() == Some(c.hitbox) && s.allowed(true) && s.model.is_dragging()
                {
                    w.capture_pointer(hitbox.id);
                    s.capture.as_mut().unwrap().hitbox = hitbox.id;
                } else {
                    s.cancel(w);
                }
            }
        }
        (
            hitbox,
            self.element.prepaint(id, inspector, bounds, layout, w, cx),
        )
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        w: &mut Window,
        cx: &mut App,
    ) {
        let weak = self.state.clone();
        let after = self.after.clone();
        let hitbox = prepaint.0.clone();
        w.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, w, cx| {
            if phase.bubble()
                && event.button == MouseButton::Left
                && hitbox.is_hovered(w)
                && let Some(state) = weak.upgrade()
            {
                state
                    .borrow_mut()
                    .begin(&after, hitbox.id, event.position, w, cx);
            }
        });
        let weak = self.state.clone();
        let after = self.after.clone();
        let hitbox = prepaint.0.clone();
        w.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, w, cx| {
            if !phase.capture() {
                return;
            }
            let Some(state) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if s.capture.as_ref().is_some_and(|c| c.after == after) {
                if !s.allowed(true)
                    || s.capture
                        .as_ref()
                        .is_none_or(|c| w.captured_hitbox() != Some(c.hitbox))
                    || event.pressed_button != Some(MouseButton::Left)
                {
                    s.cancel(w);
                    return;
                }
                s.preview(event.position);
                w.refresh();
                cx.stop_propagation();
            } else {
                let hovered = s.allowed_handle(&after, true) && hitbox.is_hovered(w);
                if hovered && s.hovered.as_deref() != Some(&after) {
                    s.hovered = Some(after.clone());
                    w.refresh();
                } else if !hovered && s.hovered.as_deref() == Some(&after) {
                    s.hovered = None;
                    w.refresh();
                }
            }
        });
        self.element
            .paint(id, inspector, bounds, layout, &mut prepaint.1, w, cx);
        let weak = self.state.clone();
        let after = self.after.clone();
        let observe = self.observe.clone();
        w.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, w, cx| {
            if !phase.bubble() {
                return;
            }
            let Some(state) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if s.capture.as_ref().is_none_or(|c| c.after != after) {
                return;
            }
            if !s.allowed(true)
                || s.capture
                    .as_ref()
                    .is_none_or(|c| w.captured_hitbox() != Some(c.hitbox))
                || event.button != MouseButton::Left
            {
                s.cancel(w);
                return;
            }
            s.preview(event.position);
            let snapshot = s.model.finish_drag();
            s.release(w);
            drop(s);
            if let Some(snapshot) = snapshot {
                observe(snapshot, w, cx);
            }
            w.refresh();
            w.prevent_default();
            cx.stop_propagation();
        });
    }
    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.element.write_a11y_info(node);
        if !self.visible {
            node.set_hidden();
            node.clear_actions();
        }
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        self.element
            .a11y_synthetic_children(&mut prepaint.1, builder)
    }
}
