//! Measured retained-card presentation. GPUI supplies fresh scroll measurements
//! before prepainting children; translations therefore affect both paint and input.
use super::*;
#[path = "carousel_track_navigation.rs"]
mod navigation;
#[path = "carousel_track_pointer.rs"]
pub(super) mod pointer;
#[path = "carousel_track_scroll.rs"]
mod wheel;
use crate::{
    carousel_track_geometry::{Geometry, Item as Interval, Loop},
    carousel_track_motion, carousel_track_state,
};
use gpui::{
    AnyElement, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, Pixels, Point,
    ScrollHandle,
};
use gpuio_protocol::{HandlerId, carousel::Axis, carousel_track::Request};

pub(super) struct State {
    handler: HandlerId,
    model: carousel_track_state::State,
    scroll: ScrollHandle,
    motion: carousel_track_motion::State,
    settings: Option<gpuio_protocol::carousel_track::Motion>,
    origin: Option<std::time::Instant>,
    painted: bool,
    settled: bool,
    visible_bounds: Option<Bounds<Pixels>>,
    group_bounds: Option<Bounds<Pixels>>,
    focus: gpui::FocusHandle,
    timer: Option<gpui::Task<()>>,
    ticket: Option<crate::carousel_clock::Ticket>,
    subscriptions: Vec<gpui::Subscription>,
    drag: Option<pointer::Gesture>,
    pointer_hitbox: Option<gpui::Hitbox>,
    wheel: crate::carousel_track_wheel::State,
    wheel_task: Option<(Rc<()>, gpui::Task<()>)>,
    wheel_axis: gpui::OngoingScroll,
}
impl State {
    fn dispose(&mut self, window: &mut Window) {
        self.cancel_track_drag(window);
        self.wheel_task = None;
        self.wheel.clear();
        self.model.dispose();
        self.motion.clear();
        self.timer = None;
        self.ticket = None;
        self.subscriptions.clear();
    }
    pub(super) fn focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }
    #[cfg(test)]
    pub(super) fn clock_snapshot(&self) -> (bool, bool) {
        (self.timer.is_some(), self.model.pending_proposal())
    }
    #[cfg(test)]
    pub(super) fn focus_handle(&self) -> gpui::FocusHandle {
        self.focus.clone()
    }
}
pub(super) struct Frame {
    state: Rc<RefCell<State>>,
    route: choice::Route,
    measured: Cell<bool>,
    sample: RefCell<Option<carousel_track_motion::Sample>>,
    offsets: RefCell<Vec<Point<Pixels>>>,
    clipped: RefCell<Vec<bool>>,
}
impl Frame {
    fn measure(&self, window: &mut Window, cx: &App) {
        if self.measured.replace(true) {
            return;
        }
        let mut state = self.state.borrow_mut();
        let model = &state.model.config().carousel;
        let vertical = model.axis == Axis::Vertical;
        let count = model.ids.len();
        let viewport = state.scroll.bounds();
        let visible = viewport.intersect(&window.content_mask().bounds);
        let available = visible.size.width > px(0.) && visible.size.height > px(0.);
        let origin = if vertical {
            viewport.origin.y
        } else {
            viewport.origin.x
        };
        let extent = if vertical {
            viewport.size.height
        } else {
            viewport.size.width
        };
        let limit = if vertical {
            state.scroll.max_offset().y
        } else {
            state.scroll.max_offset().x
        };
        let intervals = (0..count)
            .map(|i| {
                state.scroll.bounds_for_item(i).map(|b| Interval {
                    start: f32::from(if vertical {
                        b.origin.y - origin
                    } else {
                        b.origin.x - origin
                    }),
                    extent: f32::from(if vertical {
                        b.size.height
                    } else {
                        b.size.width
                    }),
                })
            })
            .collect::<Option<Vec<_>>>();
        // GPUI retains old child bounds when a Div becomes empty. Ignore that
        // cache only for a checked empty collection; nonempty measurements must match.
        let intervals = intervals
            .filter(|_| available && (count == 0 || state.scroll.children_count() == count));
        let geometry = intervals.and_then(|items| {
            Geometry::new(f32::from(extent), f32::from(limit), items, model.looping).ok()
        });
        if state
            .drag
            .as_ref()
            .is_some_and(|drag| geometry.as_ref() != Some(drag.model.geometry()))
        {
            state.cancel_track_drag(window);
        }
        if state
            .wheel
            .geometry()
            .is_some_and(|old| Some(old) != geometry.as_ref())
        {
            let now = state.track_now(cx);
            state.interrupt_track_wheel(now, true);
        }
        match state.model.measure(geometry) {
            Ok(Some(layout)) => {
                let event = self.route.session.borrow().request_carousel_track(
                    self.route.window,
                    self.route.node,
                    self.route.handler,
                    self.route.revision,
                    Request::Layout(layout.clone()),
                );
                if let Some(event) = event {
                    if self.route.transport.input(event) {
                        state.model.published(&layout);
                    } else if self.route.session.borrow_mut().overload(self.route.window) {
                        self.route.transport.fault(self.route.window);
                    }
                }
            }
            Ok(None) => (),
            Err(_) => {
                state.dispose(window);
            }
        }
        let mut offsets = self.offsets.borrow_mut();
        offsets.clear();
        offsets.resize(count, Point::default());
        self.clipped.borrow_mut().resize(count, true);
        let now = cx.background_executor().now();
        let origin = *state.origin.get_or_insert(now);
        let eligible = available
            && window.is_window_active()
            && !cx.reduce_motion()
            && self.route.gate.borrow().visible(self.route.node);
        let State {
            model,
            motion,
            settings,
            ..
        } = &mut *state;
        let sample = motion.sample(
            model.config(),
            model.geometry(),
            *settings,
            eligible,
            now.saturating_duration_since(origin),
        );
        *self.sample.borrow_mut() = sample.clone();
        let Some(sample) = sample else {
            return;
        };
        let offset = sample.offset;
        let Some(geometry) = model.geometry() else {
            return;
        };
        let runway = match geometry.looping() {
            Some(Loop::Continuous { origin, .. }) => origin,
            _ => 0.,
        };
        for (index, translation) in offsets.iter_mut().enumerate() {
            let value = offset + runway + geometry.item_translation(index, offset).unwrap_or(0.);
            *translation = if vertical {
                gpui::point(px(0.), px(value))
            } else {
                gpui::point(px(value), px(0.))
            };
        }
    }
    pub(super) fn style(
        &self,
        mut element: gpui::Stateful<gpui::Div>,
    ) -> gpui::Stateful<gpui::Div> {
        let state = self.state.borrow();
        element = element
            .flex()
            .size_full()
            .flex_nowrap()
            .flex_shrink_0()
            .track_scroll(&state.scroll);
        element.style().overflow.x = Some(gpui::Overflow::Visible);
        element.style().overflow.y = Some(gpui::Overflow::Visible);
        if state.model.config().carousel.axis == Axis::Vertical {
            element.flex_col()
        } else {
            element.flex_row()
        }
    }
    pub(super) fn item(
        self: &Rc<Self>,
        element: AnyElement,
        index: usize,
        node: NodeId,
    ) -> AnyElement {
        let element = CardMask {
            element,
            frame: self.clone(),
            index,
            node,
        }
        .into_any_element();
        Translated {
            element,
            frame: self.clone(),
            index,
            node,
        }
        .into_any_element()
    }
    pub(super) fn track(self: &Rc<Self>, element: AnyElement) -> AnyElement {
        Track {
            element,
            frame: self.clone(),
        }
        .into_any_element()
    }
}
impl View {
    pub(super) fn begin_carousel_track_paint(&self) {
        for state in self.carousel_tracks.values() {
            let mut state = state.borrow_mut();
            state.painted = false;
            state.group_bounds = None;
        }
    }
    pub(super) fn finish_carousel_track_paint(&self, window: &mut Window, cx: &mut App) {
        // Anchored surfaces consult visibility while rendering, before this
        // frame measures translated cards. Rebuild once when that mask changes
        // so a revealed retained popup does not wait for unrelated input.
        if self.focus.borrow().track_visibility_changed() {
            window.refresh();
        }
        for (id, state) in &self.carousel_tracks {
            let mut state = state.borrow_mut();
            if !state.painted {
                state.motion.clear();
            }
            if self.focus.borrow().allows(*id)
                && state.focus.contains_focused(window, cx)
                && let Some(focused) = window.focused(cx)
                && !self.focus.borrow().can_focus(&focused, window)
            {
                if state.painted
                    && !state.model.config().carousel.disabled
                    && self.focus.borrow().can_focus(&state.focus, window)
                {
                    window.focus(&state.focus, cx);
                } else {
                    window.blur(cx);
                }
            }
        }
    }
    pub(super) fn suspend_hidden_carousel_tracks(&self) {
        for (id, state) in &self.carousel_tracks {
            if !self.focus.borrow().visible(*id) {
                state.borrow_mut().motion.clear();
            }
        }
    }
    pub(super) fn sync_carousel_tracks(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.carousel_tracks.retain(|id, state| {
            let keep = tree
                .and_then(|tree| tree.get(*id))
                .is_some_and(|node| node.carousel_track.is_some());
            if !keep {
                state.borrow_mut().dispose(window);
            }
            keep
        });
        let Some(tree) = tree else {
            return;
        };
        // Admission guarantees every live node belongs to the root. Track-only
        // structural edits also dirty their owner, so unrelated text updates do
        // not require another full-tree traversal or clone all track models.
        for id in dirty.iter().copied() {
            let Some(node) = tree.get(id) else {
                continue;
            };
            let Some(config) = &node.carousel_track else {
                continue;
            };
            let handler = node.handler.expect("admitted handler");
            let items = tree
                .get(node.children[0])
                .expect("admitted track")
                .children
                .to_vec();
            if self
                .carousel_tracks
                .get(&id)
                .is_some_and(|state| state.borrow().handler != handler)
                && let Some(old) = self.carousel_tracks.remove(&id)
            {
                old.borrow_mut().dispose(window);
            }
            if let Some(state) = self.carousel_tracks.get(&id) {
                let mut state = state.borrow_mut();
                if state.model.config() != config.as_ref()
                    || state.settings != node.carousel_track_motion
                {
                    state.cancel_track_drag(window);
                    let now = state.track_now(cx);
                    state.interrupt_track_wheel(now, true);
                    state.painted = false;
                }
                if config.carousel.disabled && state.focus.is_focused(window) {
                    window.blur(cx);
                }
                state.settings = node.carousel_track_motion;
                state
                    .model
                    .sync(config.as_ref().clone(), items)
                    .expect("admitted model update");
            } else {
                let focus = cx.focus_handle();
                let subscriptions = self.track_focus_subscriptions(id, &focus, window, cx);
                self.carousel_tracks.insert(
                    id,
                    Rc::new(RefCell::new(State {
                        handler,
                        model: carousel_track_state::State::new(config.as_ref().clone(), items)
                            .expect("admitted track"),
                        scroll: ScrollHandle::new(),
                        motion: Default::default(),
                        settings: node.carousel_track_motion,
                        origin: None,
                        painted: false,
                        settled: false,
                        visible_bounds: None,
                        group_bounds: None,
                        focus,
                        timer: None,
                        ticket: None,
                        subscriptions,
                        drag: None,
                        pointer_hitbox: None,
                        wheel: Default::default(),
                        wheel_task: None,
                        wheel_axis: Default::default(),
                    })),
                );
            }
        }
        drop(session);
        self.suspend_hidden_carousel_tracks();
        if self.carousel_tracks.is_empty() {
            self.carousel_track_activation = None;
        } else if self.carousel_track_activation.is_none() {
            self.carousel_track_activation =
                Some(cx.observe_window_activation(window, |view, window, cx| {
                    if !window.is_window_active() {
                        for state in view.carousel_tracks.values() {
                            let mut state = state.borrow_mut();
                            state.cancel_track_drag(window);
                            state.motion.clear();
                        }
                    }
                    view.schedule_carousel_tracks(window, cx);
                    cx.notify();
                }));
        }
        self.schedule_carousel_tracks(window, cx);
    }
    pub(super) fn carousel_track_frame(
        &self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
    ) -> Option<Rc<Frame>> {
        let owner = tree.get(node.parent?)?;
        owner.carousel_track.as_ref()?;
        let state = self.carousel_tracks.get(&owner.id)?.clone();
        Some(Rc::new(Frame {
            state,
            measured: Cell::new(false),
            sample: RefCell::new(None),
            offsets: RefCell::new(Vec::new()),
            clipped: RefCell::new(Vec::new()),
            route: choice::Route {
                window: self.id,
                node: owner.id,
                handler: owner.handler?,
                revision: tree.revision(),
                session: self.session.clone(),
                gate: self.focus.clone(),
                transport: self.transport.clone(),
            },
        }))
    }
}
struct Track {
    element: AnyElement,
    frame: Rc<Frame>,
}
impl IntoElement for Track {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Track {
    type RequestLayoutState = ();
    type PrepaintState = ();
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
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.frame.measured.set(false);
        self.frame
            .state
            .borrow()
            .scroll
            .set_offset(Point::default());
        self.element.prepaint(window, cx);
        // Empty tracks have no first item but still publish a checked empty map.
        if self
            .frame
            .state
            .borrow()
            .model
            .config()
            .carousel
            .ids
            .is_empty()
        {
            self.frame.measure(window, cx);
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
        if let Some(sample) = self.frame.sample.borrow().as_ref() {
            let current = self
                .frame
                .route
                .session
                .borrow()
                .tree(self.frame.route.window)
                .and_then(|tree| tree.get(self.frame.route.node))
                .is_some_and(|node| node.handler == Some(self.frame.route.handler));
            let accepted = current && self.frame.state.borrow_mut().motion.painted(sample);
            if accepted {
                let mut state = self.frame.state.borrow_mut();
                state.painted = true;
                state.settled = !sample.needs_frame;
            }
            if accepted
                && sample.needs_frame
                && window.is_window_active()
                && !cx.reduce_motion()
                && self
                    .frame
                    .route
                    .gate
                    .borrow()
                    .visible(self.frame.route.node)
            {
                window.request_animation_frame();
            }
        }
    }
}
struct Translated {
    element: AnyElement,
    frame: Rc<Frame>,
    index: usize,
    node: NodeId,
}
impl IntoElement for Translated {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Translated {
    type RequestLayoutState = ();
    type PrepaintState = ();
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
        mut bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.frame.measure(window, cx);
        let offset = self.frame.offsets.borrow()[self.index];
        bounds.origin += offset;
        let visible = bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&window.fully_visible_bounds());
        self.frame.clipped.borrow_mut()[self.index] =
            visible.size.width <= px(0.) || visible.size.height <= px(0.);
        window.with_element_offset(offset, |window| self.element.prepaint(window, cx));
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.frame
            .route
            .gate
            .borrow_mut()
            .track_card_clipped(self.node, self.frame.clipped.borrow()[self.index]);
        self.element.paint(window, cx);
    }
}

/// Created inside the translation wrapper: its hidden bit is computed before
/// GPUI asks for accessibility metadata, and its identity follows the retained
/// card rather than the card's changing position in the collection.
struct CardMask {
    element: AnyElement,
    frame: Rc<Frame>,
    index: usize,
    node: NodeId,
}
impl IntoElement for CardMask {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for CardMask {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(
            (
                "gpuio-track-card-mask",
                ((self.node.generation() as u64) << 32) | self.node.slot() as u64,
            )
                .into(),
        )
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
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
    }
    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        Some(gpui::accesskit::Role::GenericContainer)
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        if self
            .frame
            .clipped
            .borrow()
            .get(self.index)
            .copied()
            .unwrap_or(true)
        {
            node.set_hidden();
        }
    }
}
