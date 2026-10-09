//! Viewport keyboard routing and one cancellable automatic deadline per owner.
use super::*;
use crate::carousel_clock::{Plan, Ticket};

impl View {
    pub(in super::super) fn track_focus_subscriptions(
        &self,
        id: NodeId,
        focus: &gpui::FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Subscription> {
        let owner = cx.weak_entity();
        let enter = window.on_focus_in(focus, cx, move |window, cx| {
            let owner = owner.clone();
            window.defer(cx, move |window, cx| {
                let _ = owner.update(cx, |view, cx| view.schedule_carousel_track(id, window, cx));
            });
        });
        let owner = cx.weak_entity();
        let leave = window.on_focus_out(focus, cx, move |_, window, cx| {
            let owner = owner.clone();
            window.defer(cx, move |window, cx| {
                let _ = owner.update(cx, |view, cx| view.schedule_carousel_track(id, window, cx));
            });
        });
        vec![enter, leave]
    }
    fn track_control_focused(&self, id: NodeId, window: &Window, cx: &App) -> bool {
        self.focus
            .borrow()
            .focused_node(window, cx)
            .is_some_and(|focused| {
                self.session
                    .borrow()
                    .tree(self.id)
                    .and_then(|tree| tree.carousel_track_control(focused))
                    == Some(id)
            })
    }
    pub(in super::super) fn focus_track_control_after_pointer(
        &mut self,
        control: NodeId,
        handler: gpuio_protocol::HandlerId,
        revision: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = {
            let session = self.session.borrow();
            if session.press(self.id, control, handler, revision).is_none() {
                return;
            }
            session
                .tree(self.id)
                .and_then(|tree| tree.carousel_track_control(control))
        };
        let Some(id) = id else {
            return;
        };
        if !self.focus.borrow().allows(control) || !self.focus.borrow().allows(id) {
            return;
        }
        let Some(state) = self.carousel_tracks.get(&id) else {
            return;
        };
        let state = state.borrow();
        if !state.painted
            || state.model.config().carousel.disabled
            || state.focus.contains_focused(window, cx)
        {
            return;
        }
        window.focus(&state.focus, cx);
        drop(state);
        self.schedule_carousel_track(id, window, cx);
    }
    pub(in super::super) fn track_related_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        mut element: gpui::Stateful<gpui::Div>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        if let Some((index, count)) = tree.carousel_track_item(node.id) {
            element = element
                .role(gpui::Role::Group)
                .aria_position_in_set(index + 1)
                .aria_size_of_set(count);
        }
        if let Some(id) = tree.carousel_track_control(node.id) {
            let control = node.id;
            element = element.on_key_down(cx.listener(move |view, event, window, cx| {
                let current = view
                    .session
                    .borrow()
                    .tree(view.id)
                    .and_then(|tree| tree.carousel_track_control(control));
                if current == Some(id) {
                    view.carousel_track_key(id, event, window, cx);
                }
            }));
        }
        if node.kind == Kind::CarouselTrackGroup {
            let id = node.children[0];
            let state = self.carousel_tracks[&id].clone();
            let owner = cx.weak_entity();
            element = element.role(gpui::Role::Group).child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        state.borrow_mut().group_bounds = Some(
                            bounds
                                .intersect(&window.content_mask().bounds)
                                .intersect(&window.fully_visible_bounds()),
                        );
                        window.on_mouse_event(
                            move |_: &gpui::MouseMoveEvent, phase, window, cx| {
                                if phase.capture() {
                                    let _ = owner.update(cx, |view, cx| {
                                        view.schedule_carousel_track(id, window, cx)
                                    });
                                }
                            },
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        }
        element
    }
    fn track_eligible(&self, id: NodeId, state: &State, window: &Window, cx: &App) -> bool {
        state.drag.is_none()
            && !state.wheel.active()
            && !state.motion.previewing()
            && state.painted
            && state.settled
            && state
                .group_bounds
                .or(state.visible_bounds)
                .is_some_and(|bounds| {
                    bounds.size.width > px(0.)
                        && bounds.size.height > px(0.)
                        && !bounds.contains(&window.mouse_position())
                })
            && !state.model.config().carousel.disabled
            && !state.focus.contains_focused(window, cx)
            && !self.track_control_focused(id, window, cx)
            && window.is_window_active()
            && !cx.reduce_motion()
            && window.captured_hitbox().is_none()
            && !cx.has_active_drag()
            && self.focus.borrow().allows(id)
    }
    pub(in super::super) fn schedule_carousel_tracks(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ids = self.carousel_tracks.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.sync_track_drag(id, window, cx);
            self.schedule_track_wheel(id, window, cx);
            self.schedule_carousel_track(id, window, cx);
        }
    }
    pub(in super::super) fn schedule_carousel_track(
        &mut self,
        id: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.carousel_tracks.get(&id).cloned() else {
            return;
        };
        let mut state = state.borrow_mut();
        let eligible = self.track_eligible(id, &state, window, cx);
        let instant = cx.background_executor().now();
        let origin = *state.origin.get_or_insert(instant);
        let now = instant.saturating_duration_since(origin);
        match state
            .model
            .plan(eligible, now)
            .expect("admitted track clock")
        {
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
                        view.wake_carousel_track(id, ticket, window, cx)
                    });
                }));
            }
        }
    }
    fn wake_carousel_track(
        &mut self,
        id: NodeId,
        ticket: Ticket,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.schedule_carousel_track(id, window, cx);
        let proposal = {
            let Some(state) = self.carousel_tracks.get(&id) else {
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
            let eligible = self.track_eligible(id, &state, window, cx);
            let now = cx
                .background_executor()
                .now()
                .saturating_duration_since(state.origin.expect("scheduled origin"));
            let proposal = state
                .model
                .wake(&ticket, eligible, now)
                .expect("admitted track deadline");
            state.timer = None;
            state.ticket = None;
            proposal
        };
        if let Some(proposal) = proposal {
            self.carousel_track_request(id, Request::AutoNext(proposal));
        }
        self.schedule_carousel_track(id, window, cx);
    }
    pub(in super::super) fn carousel_track_request(&self, id: NodeId, request: Request) {
        if !self.focus.borrow().allows(id) {
            return;
        }
        let event = {
            let session = self.session.borrow();
            session.tree(self.id).and_then(|tree| {
                let node = tree.get(id)?;
                session.request_carousel_track(self.id, id, node.handler?, tree.revision(), request)
            })
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.id)
        {
            self.transport.fault(self.id);
        }
    }
    fn carousel_track_key(
        &self,
        id: NodeId,
        event: &gpui::KeyDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.modifiers.modified() || !self.focus.borrow().allows(id) {
            return;
        }
        let Some(state) = self.carousel_tracks.get(&id) else {
            return;
        };
        let state = state.borrow();
        // Descendant editors/buttons keep their own keys even when they bubble.
        if !state.focus.is_focused(window) && !self.track_control_focused(id, window, cx) {
            return;
        }
        let request = match (
            event.keystroke.key.as_str(),
            state.model.config().carousel.axis,
        ) {
            ("home", _) => Request::First,
            ("end", _) => Request::Last,
            ("left", Axis::Horizontal) | ("up", Axis::Vertical) => Request::Previous,
            ("right", Axis::Horizontal) | ("down", Axis::Vertical) => Request::Next,
            _ => return,
        };
        let request = state.model.manual(request);
        drop(state);
        if let Some(request) = request {
            self.carousel_track_request(id, request);
            cx.stop_propagation();
        }
    }
    pub(in super::super) fn carousel_track_element(
        &mut self,
        element: gpui::Stateful<gpui::Div>,
        node: &crate::tree::Node,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = node.id;
        let state = self.carousel_tracks[&id].clone();
        let focus = state.borrow().focus.clone();
        let enabled = !state.borrow().model.config().carousel.disabled;
        let gate = self.focus.clone();
        let recorded = focus.clone();
        let owner = cx.weak_entity();
        let accessible = owner.clone();
        element
            .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                let _ = accessible.update(cx, |view, cx| {
                    if let Some(state) = view.carousel_tracks.get(&id) {
                        let state = state.borrow();
                        if state.painted && view.focus.borrow().can_focus(&state.focus, window) {
                            window.focus(&state.focus, cx);
                        }
                    }
                });
                cx.stop_propagation();
            })
            .track_focus(&focus.tab_stop(enabled))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |view, event, window, cx| {
                    if !view.focus.borrow().eligible(id) {
                        window.prevent_default();
                        return;
                    }
                    view.track_pointer_down(id, event, window, cx)
                }),
            )
            .on_key_down(cx.listener(move |view, event, window, cx| {
                view.carousel_track_key(id, event, window, cx)
            }))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let visible = bounds
                            .intersect(&window.content_mask().bounds)
                            .intersect(&window.fully_visible_bounds());
                        state.borrow_mut().visible_bounds = Some(visible);
                        window.on_mouse_event(
                            move |_: &gpui::MouseMoveEvent, phase, window, cx| {
                                if phase.capture() {
                                    let _ = owner.update(cx, |view, cx| {
                                        view.schedule_carousel_track(id, window, cx)
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
                                bounds,
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
