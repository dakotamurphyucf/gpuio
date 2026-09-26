//! Native carousel interaction and deadlines. Application selection remains in
//! the admitted tree; all requests cross the existing asynchronous mailbox.
use super::*;
#[path = "carousel_input.rs"]
pub(super) mod input;
use crate::carousel_clock::{Clock, Plan, Schedule, Ticket};
use gpuio_protocol::carousel::{Axis, Config, Request};
use std::time::{Duration, Instant};

pub(super) struct State {
    config: Arc<Config>,
    handler: Option<gpuio_protocol::HandlerId>,
    input: input::Input,
    viewport: NodeId,
    pages: Arc<[NodeId]>,
    origin: Instant,
    clock: Clock,
    focus: gpui::FocusHandle,
    hovered: bool,
    painted: bool,
    timer: Option<gpui::Task<()>>,
    ticket: Option<Ticket>,
    _subscriptions: Vec<gpui::Subscription>,
}
impl State {
    fn dispose(&mut self, window: &mut Window) {
        self.input.cancel_drag(window, true);
        self.timer = None;
        self.ticket = None;
        self.clock.dispose();
        self.input.interrupt(self.origin.elapsed());
        self._subscriptions.clear();
    }
    fn schedule(&self) -> Option<Schedule> {
        let target = self.config.target(&Request::Next)?;
        Some(Schedule {
            revision: self.config.revision,
            from: *self.pages.get(self.config.selected? as usize)?,
            target: *self.pages.get(target)?,
            interval: Duration::from_millis(self.config.auto_advance_ms? as u64),
        })
    }
    pub(super) fn painted(&mut self, bounds: Bounds<gpui::Pixels>, settled: bool, window: &Window) {
        let visible = bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&window.fully_visible_bounds());
        self.input.visible = visible.size.width > px(0.) && visible.size.height > px(0.);
        self.painted = settled && self.input.visible;
    }
    pub(super) fn focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn focus_handle(&self) -> gpui::FocusHandle {
        self.focus.clone()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn diagnostics(&self, window: &Window, cx: &App) -> String {
        format!(
            "painted={} hovered={} focused={} active={} reduced={} timer={} pending={}",
            self.painted,
            self.hovered,
            self.focus.contains_focused(window, cx),
            window.is_window_active(),
            cx.reduce_motion(),
            self.timer.is_some(),
            self.clock.pending()
        )
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn has_timer(&self) -> bool {
        self.timer.is_some()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn shield_hitbox(&self) -> Option<gpui::HitboxId> {
        self.input.shield_hitbox()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn dragging(&self) -> bool {
        self.input.dragging()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn wheel_timer(&self) -> bool {
        self.input.has_timer()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn pending(&self) -> bool {
        self.clock.pending()
    }
}
fn focus_changed(owner: gpui::WeakEntity<View>, id: NodeId, window: &mut Window, cx: &mut App) {
    window.defer(cx, move |window, cx| {
        let _ = owner.update(cx, |view, cx| view.schedule_carousel(id, window, cx));
    });
}
impl View {
    pub(super) fn sync_carousels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let nodes = {
            let session = self.session.borrow();
            let tree = session.tree(self.id);
            self.carousels.retain(|id, state| {
                let keep = tree
                    .and_then(|tree| tree.get(*id))
                    .is_some_and(|node| node.carousel.is_some());
                if !keep {
                    state.borrow_mut().dispose(window);
                }
                keep
            });
            let mut nodes = vec![];
            if let Some(tree) = tree {
                let mut stack = tree.root().into_iter().collect::<Vec<_>>();
                while let Some(id) = stack.pop() {
                    let node = tree.get(id).expect("admitted tree");
                    if let Some(config) = &node.carousel {
                        let viewport = tree.get(node.children[0]).expect("admitted viewport");
                        nodes.push((
                            id,
                            node.handler,
                            config.clone(),
                            viewport.id,
                            viewport.children.clone(),
                        ));
                    }
                    stack.extend(node.children.iter().copied());
                }
            }
            nodes
        };
        let present = nodes
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<std::collections::BTreeSet<_>>();
        self.carousels.retain(|id, state| {
            let keep = present.contains(id);
            if !keep {
                state.borrow_mut().dispose(window);
            }
            keep
        });
        for (id, handler, config, viewport, pages) in nodes {
            let state = self.carousels.entry(id).or_insert_with(|| {
                let focus = cx.focus_handle();
                let owner = cx.weak_entity();
                let enter = window.on_focus_in(&focus, cx, move |window, cx| {
                    focus_changed(owner.clone(), id, window, cx)
                });
                let owner = cx.weak_entity();
                let leave = window.on_focus_out(&focus, cx, move |_, window, cx| {
                    focus_changed(owner.clone(), id, window, cx)
                });
                Rc::new(RefCell::new(State {
                    config: config.clone(),
                    handler,
                    input: input::Input::default(),
                    viewport,
                    pages: pages.clone(),
                    origin: Instant::now(),
                    clock: Clock::default(),
                    focus,
                    hovered: false,
                    painted: false,
                    timer: None,
                    ticket: None,
                    _subscriptions: vec![enter, leave],
                }))
            });
            let mut state = state.borrow_mut();
            if state.config != config
                || state.viewport != viewport
                || state.pages != pages
                || state.handler != handler
            {
                let now = state.origin.elapsed();
                state.input.interrupt(now);
                state.input.cancel_drag(window, true);
                state.handler = handler;
                state.painted = false;
                state.timer = None;
                state.ticket = None;
                state.config = config;
                state.viewport = viewport;
                state.pages = pages;
            }
        }
        if self.carousels.is_empty() {
            self.carousel_activation = None;
        } else if self.carousel_activation.is_none() {
            self.carousel_activation =
                Some(cx.observe_window_activation(window, |view, window, cx| {
                    view.schedule_carousels(window, cx);
                }));
        }
        self.schedule_carousels(window, cx);
    }
    pub(super) fn begin_carousel_paint(&self) {
        for state in self.carousels.values() {
            let mut state = state.borrow_mut();
            state.painted = false;
            state.input.visible = false;
        }
    }
    pub(super) fn schedule_carousels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids = self.carousels.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.sync_carousel_drag(id, window, cx);
            self.schedule_carousel_wheel(id, window, cx);
            self.schedule_carousel(id, window, cx);
        }
    }
    fn schedule_carousel(&mut self, id: NodeId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.carousels.get(&id).cloned() else {
            return;
        };
        let mut state = state.borrow_mut();
        let eligible = !state.input.active()
            && state.painted
            && !state.hovered
            && !state.config.disabled
            && window.is_window_active()
            && window.captured_hitbox().is_none()
            && !cx.has_active_drag()
            && !cx.reduce_motion()
            && !state.focus.contains_focused(window, cx)
            && self.focus.borrow().allows(id);
        let schedule = state.schedule();
        let now = state.origin.elapsed();
        let plan = state
            .clock
            .update(schedule, eligible, now)
            .expect("bounded admitted carousel clock");
        match plan {
            Plan::Idle => {
                state.timer = None;
                state.ticket = None;
            }
            Plan::Wait { deadline, ticket } => {
                if state
                    .ticket
                    .as_ref()
                    .is_some_and(|old| old.same_as(&ticket))
                {
                    return;
                }
                state.timer = None;
                state.ticket = Some(ticket.clone());
                state.timer = Some(cx.spawn_in(window, async move |owner, cx| {
                    cx.background_executor()
                        .timer(deadline.saturating_sub(now))
                        .await;
                    let _ = owner.update_in(cx, |view, window, cx| {
                        view.wake_carousel(id, ticket, window, cx)
                    });
                }));
            }
        }
    }
    fn wake_carousel(
        &mut self,
        id: NodeId,
        ticket: Ticket,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Focus, visibility, policy and the admitted revision may have changed
        // since the task was created. Update before allowing its ticket to wake.
        self.schedule_carousel(id, window, cx);
        let request = {
            let Some(state) = self.carousels.get(&id) else {
                return;
            };
            let mut state = state.borrow_mut();
            if !state
                .ticket
                .as_ref()
                .is_some_and(|current| current.same_as(&ticket))
            {
                return;
            }
            let now = state.origin.elapsed();
            let woke = state.clock.wake(&ticket, now);
            state.timer = None;
            state.ticket = None;
            woke.and_then(|_| state.config.automatic_request())
        };
        if let Some(request) = request {
            self.carousel_request(id, request);
        }
        // Also rearms the remainder after a hypothetical early platform wake.
        self.schedule_carousel(id, window, cx);
    }
    fn carousel_request(&mut self, id: NodeId, request: Request) {
        if !self.focus.borrow().allows(id) {
            return;
        }
        let event = {
            let session = self.session.borrow();
            session.tree(self.id).and_then(|tree| {
                let node = tree.get(id)?;
                session.request_carousel(self.id, id, node.handler?, tree.revision(), request)
            })
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.id)
        {
            self.transport.fault(self.id);
        }
    }
    fn carousel_key(
        &mut self,
        id: NodeId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.modifiers.modified() || !self.focus.borrow().allows(id) {
            return;
        }
        let Some(state) = self.carousels.get(&id) else {
            return;
        };
        let state = state.borrow();
        if state.config.disabled {
            return;
        }
        let root_focused = state.focus.is_focused(window);
        // Only the carousel's own surface and ordinary controls use these keys.
        // Text/native widgets retain them even when they propagate an unbound key.
        let control_focused = self
            .focus
            .borrow()
            .focused_node(window)
            .is_some_and(|focused| {
                let session = self.session.borrow();
                let Some(tree) = session.tree(self.id) else {
                    return false;
                };
                let Some(node) = tree.get(focused) else {
                    return false;
                };
                if !matches!(node.kind, Kind::Button | Kind::CommandButton) {
                    return false;
                }
                let Some(controls) = tree.get(id).and_then(|root| root.children.get(1)).copied()
                else {
                    return false;
                };
                let mut current = Some(focused);
                while let Some(node) = current {
                    if node == controls {
                        return true;
                    }
                    current = tree.get(node).and_then(|node| node.parent);
                }
                false
            });
        if !root_focused && !control_focused {
            return;
        }
        let request = match (event.keystroke.key.as_str(), state.config.axis) {
            ("home", _) => Some(Request::First),
            ("end", _) => Some(Request::Last),
            ("left", Axis::Horizontal) | ("up", Axis::Vertical) => Some(Request::Previous),
            ("right", Axis::Horizontal) | ("down", Axis::Vertical) => Some(Request::Next),
            _ => None,
        };
        drop(state);
        if let Some(request) = request {
            self.carousel_request(id, request);
            cx.stop_propagation();
        }
    }
    pub(super) fn carousel_element(
        &mut self,
        element: gpui::Stateful<gpui::Div>,
        node: &crate::tree::Node,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = node.id;
        let state = self.carousels[&id].clone();
        let focus = state.borrow().focus.clone();
        let enabled = !state.borrow().config.disabled;
        let gate = self.focus.clone();
        let recorded = focus.clone();
        let owner = cx.weak_entity();
        element
            .track_focus(&focus.clone().tab_stop(enabled))
            .on_key_down(
                cx.listener(move |view, event, window, cx| {
                    view.carousel_key(id, event, window, cx)
                }),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let visible = bounds
                            .intersect(&window.content_mask().bounds)
                            .intersect(&window.fully_visible_bounds());
                        // Hover includes native child hitboxes, not just the
                        // parent's own hit target. Otherwise a child editor or
                        // gesture region could accidentally resume auto-advance.
                        state.borrow_mut().hovered = visible.contains(&window.mouse_position());
                        window.on_mouse_event(
                            move |event: &gpui::MouseMoveEvent, phase, window, cx| {
                                if phase.capture() {
                                    let _ = owner.update(cx, |view, cx| {
                                        if let Some(state) = view.carousels.get(&id) {
                                            state.borrow_mut().hovered =
                                                visible.contains(&event.position);
                                        }
                                        view.schedule_carousel(id, window, cx);
                                    });
                                }
                            },
                        );
                        if gate.borrow().visible(id) {
                            gate.borrow_mut().record(
                                id,
                                recorded.clone(),
                                enabled,
                                recorded.is_focused(window),
                            );
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
}
