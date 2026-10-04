//! Window-owned notification sessions. Declarative content stays in the tree;
//! native dismissal fuses the session until a new node identity is mounted.
use super::{
    Interaction, View,
    toast_clock::{Clock, Plan},
};
use crate::toast_lifecycle as phase;
use gpui::{
    App, Context, FocusHandle, Task, WeakEntity, Window, canvas, deferred, div, prelude::*, px,
    rgba,
};
use gpuio_protocol::{NodeId, v1::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) struct State {
    config: Arc<ToastConfig>,
    clock: Clock,
    phase: Rc<RefCell<phase::State>>,
    painted: Rc<Cell<bool>>,
    origin: Instant,
    motion: Option<gpuio_protocol::toast_motion::Config>,
    bottom: bool,
    phase_timer: Option<Task<()>>,
    phase_deadline: Option<phase::Deadline>,
    accepted: Option<crate::session::AcceptedToastDismissal>,
    pub(super) close_focus: FocusHandle,
    timer: Option<Task<()>>,
    deadline: Option<Instant>,
    epoch: u64,
    hovered: bool,
    pub(super) bounds: std::rc::Rc<std::cell::Cell<gpui::Bounds<gpui::Pixels>>>,
}
impl State {
    #[cfg(any(feature = "native-tests", all(test, feature = "native-image-tests")))]
    pub(super) fn status(&self) -> (bool, bool) {
        (self.clock.is_closed(), self.timer.is_some())
    }
    #[cfg(all(test, feature = "native-image-tests"))]
    pub(super) fn phase_status(&self) -> (bool, bool, bool) {
        (
            self.phase.borrow().is_closed(),
            self.phase_timer.is_some(),
            self.accepted.is_some(),
        )
    }
    fn elapsed(&self, cx: &App) -> Duration {
        cx.background_executor()
            .now()
            .saturating_duration_since(self.origin)
    }
    fn motion(&self) -> Option<phase::Motion> {
        self.motion.map(|m| phase::Motion {
            enter: Duration::from_millis(m.enter_ms as u64),
            exit: Duration::from_millis(m.exit_ms as u64),
            offset: m.offset,
        })
    }
    fn cancel(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.timer = None;
        self.deadline = None;
    }
}
#[derive(Default)]
pub(super) struct Stack {
    hovered: bool,
    focus: Option<FocusHandle>,
    subscriptions: Vec<gpui::Subscription>,
    usable: std::rc::Rc<std::cell::Cell<bool>>,
    pub(super) layered: crate::toast_stack_widget::Shared,
    layered_enabled: bool,
    origin: Option<Instant>,
    painted: std::rc::Rc<std::cell::Cell<bool>>,
}
fn inherit_interaction(mut interaction: Interaction, styles: &[Style]) -> Interaction {
    for style in styles {
        if let Style::Fields(fields) = style {
            for field in fields {
                match field {
                    Field::PointerEvents(value) => interaction.pointer = *value,
                    Field::UserSelect(value) => interaction.selectable = Some(*value),
                    Field::SelectionColor(value) => {
                        interaction.selection_color = Some(super::color(value))
                    }
                    _ => (),
                }
            }
        }
    }
    interaction
}
fn timeout(config: &ToastConfig) -> Option<Duration> {
    config.timeout_ns.map(|ns| Duration::from_nanos(ns as u64))
}
fn input(
    owner: WeakEntity<View>,
    id: NodeId,
    hover: Option<bool>,
    window: &mut Window,
    cx: &mut App,
) {
    window.defer(cx, move |window, cx| {
        let _ = owner.update(cx, |view, cx| {
            if let Some(stack) = view.toast_stacks.get_mut(&id) {
                if let Some(hover) = hover {
                    stack.hovered = hover;
                }
                view.schedule_toasts(window, cx);
                cx.notify();
            }
        });
    });
}
impl View {
    pub(super) fn begin_layered_toast_paint(&mut self) {
        for state in self.toasts.values() {
            state.painted.set(false);
        }
        for stack in self.toast_stacks.values() {
            stack.painted.set(false);
        }
    }
    pub(super) fn finish_layered_toast_paint(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for stack in self.toast_stacks.values_mut() {
            if !stack.painted.get() {
                stack.layered.borrow_mut().suspend();
                if stack.layered_enabled {
                    stack.hovered = false;
                }
            }
        }
        self.schedule_toasts(window, cx);
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "checked stack description, placement and native render context"
    )]
    fn layered_toast_content(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        anchor: gpuio_protocol::toast_placement::Anchor,
        config: gpuio_protocol::toast_layering::Layering,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use crate::toast_stack_widget as widget;
        let id = node.id;
        let scope = self
            .focus
            .borrow()
            .handle(id)
            .expect("retained stack scope");
        let state = &self.toast_stacks[&id];
        let expanded = node.toast_layering.is_none()
            || state.hovered
            || scope.contains_focused(window, cx)
            || node.children.iter().any(|id| {
                self.toasts
                    .get(id)
                    .is_some_and(|s| s.hovered && !s.clock.is_closed())
            });
        let now = cx
            .background_executor()
            .now()
            .saturating_duration_since(state.origin.expect("stack origin"));
        let shared = state.layered.clone();
        let painted = state.painted.clone();
        let enabled = self.focus.borrow().allows(id);
        scope.clone().tab_stop(enabled);
        let mut samples = Vec::new();
        let contents: Vec<_> = node
            .children
            .iter()
            .filter_map(|child| {
                let state = self.toasts.get(child)?;
                if state.phase.borrow().is_closed() {
                    return None;
                }
                let motion = state
                    .motion()
                    .filter(|_| enabled && window.is_window_active() && !cx.reduce_motion());
                let sample = state
                    .phase
                    .borrow()
                    .sample(state.elapsed(cx), motion, state.bottom)
                    .expect("validated phase");
                let ending = !state.phase.borrow().accepts_input();
                samples.push((
                    *child,
                    Rc::downgrade(&state.phase),
                    state.painted.clone(),
                    sample.clone(),
                ));
                Some(widget::Content {
                    id: *child,
                    ending,
                    presentation: Some(sample),
                    element: self.element(tree, *child, interaction, window, cx),
                })
            })
            .collect();
        if contents.is_empty() {
            scope.tab_stop(false);
            return div().into_any_element();
        }
        let gate = self.focus.clone();
        let focus = scope.clone();
        let owner = cx.weak_entity();
        let element = widget::element(widget::Render {
            state: shared.clone(),
            width: px(node.toast_stack.as_ref().unwrap().width as f32),
            anchor,
            layering: crate::toast_geometry::Layering {
                peek: config.peek,
                gap: config.gap,
                width_step: config.width_step,
                visible: config.visible as usize,
            },
            expanded,
            oldest_first: node.toast_layering.is_none(),
            enabled,
            pointer: interaction.pointer,
            spring: node.toast_motion.map(|m| m.spring),
            now,
            contents,
            observe: std::rc::Rc::new(move |frame, window, _| {
                painted.set(true);
                let mut gate = gate.borrow_mut();
                gate.record(
                    id,
                    focus.clone(),
                    true,
                    focus.is_focused(window),
                    frame.viewport,
                );
                for item in &frame.items {
                    gate.track_card_clipped(item.id, !item.interactive);
                }
            }),
            after_paint: Rc::new(move |frame, _, _| {
                let mut pending = false;
                for (id, weak, painted, sample) in &samples {
                    let Some(state) = weak.upgrade() else {
                        continue;
                    };
                    if frame
                        .items
                        .iter()
                        .any(|item| item.id == *id && item.painted)
                    {
                        painted.set(true);
                        if state.borrow_mut().painted(sample) {
                            pending |= sample.needs_frame();
                        }
                    }
                }
                pending
            }),
            hover: std::rc::Rc::new(move |hover, window, cx| {
                input(owner.clone(), id, Some(hover), window, cx)
            }),
        })
        .expect("admitted layered notification configuration");
        let key_scope = scope.clone();
        let key_gate = self.focus.clone();
        let weak = std::rc::Rc::downgrade(&shared);
        let ax_scope = scope.clone();
        let ax_gate = self.focus.clone();
        let body = div()
            .id(("toast-layered-scope", id.slot()))
            .size_full()
            .track_focus(&scope)
            .tab_index(0)
            .role(gpui::Role::Group)
            .aria_label(node.toast_stack.as_ref().unwrap().label.clone())
            .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                if ax_gate.borrow().allows(id) {
                    window.focus(&ax_scope, cx);
                }
            })
            .on_key_down(move |event, window, cx| {
                if !key_scope.is_focused(window)
                    || !key_gate.borrow().allows(id)
                    || event.keystroke.modifiers.modified()
                {
                    return;
                }
                let Some(state) = weak.upgrade() else {
                    return;
                };
                let mut state = state.borrow_mut();
                let delta = match event.keystroke.key.as_str() {
                    "pageup" => -crate::window_frame::content_bounds(window).size.height * 0.8,
                    "pagedown" => crate::window_frame::content_bounds(window).size.height * 0.8,
                    "home" => -state.max_scroll(),
                    "end" => state.max_scroll(),
                    _ => return,
                };
                if state.scroll_by(delta) {
                    window.refresh();
                }
                cx.stop_propagation();
            })
            .child(element);
        let (mut body, states) = super::apply_styles(body, &node.style, interaction, false);
        let [focused, hover, pressed, _, _, _, _] = states;
        if let Some(style) = focused
            && scope.contains_focused(window, cx)
        {
            gpui::Refineable::refine(body.style(), &style);
        }
        if interaction.pointer {
            if let Some(style) = hover
                && self.toast_stacks[&id].hovered
            {
                gpui::Refineable::refine(body.style(), &style);
            }
            if let Some(style) = pressed {
                body = body.active(move |_| style);
            }
        }
        super::highlight_style::Frame::new(body, node, &self.focus).into_any_element()
    }
    pub(super) fn sync_toasts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (nodes, stacks) = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.toasts.clear();
                self.toast_stacks.clear();
                return;
            };
            self.toasts.retain(|id, _| tree.get(*id).is_some());
            self.toast_stacks.retain(|id, _| tree.get(*id).is_some());
            let mut nodes = Vec::new();
            let mut stacks = Vec::new();
            let mut pending = tree.root().into_iter().collect::<Vec<_>>();
            while let Some(id) = pending.pop() {
                let node = tree.get(id).expect("validated node");
                if let Some(config) = &node.toast {
                    let stack = node.parent.and_then(|p| tree.get(p));
                    let motion = stack.and_then(|n| n.toast_motion);
                    let anchor = stack.and_then(|n| n.toast_placement).map(|p| p.anchor);
                    let bottom = anchor.map_or_else(
                        || {
                            stack.and_then(|n| n.toast_stack.as_ref()).is_some_and(|c| {
                                matches!(
                                    c.corner,
                                    ToastCorner::BottomLeft | ToastCorner::BottomRight
                                )
                            })
                        },
                        |a| {
                            matches!(
                                a,
                                gpuio_protocol::toast_placement::Anchor::BottomLeft
                                    | gpuio_protocol::toast_placement::Anchor::BottomRight
                                    | gpuio_protocol::toast_placement::Anchor::BottomCenter
                            )
                        },
                    );
                    nodes.push((id, config.clone(), motion, bottom));
                }
                if let Some(config) = &node.toast_stack {
                    stacks.push((
                        id,
                        config.max_visible as usize,
                        node.children.clone(),
                        node.toast_layering.is_some() || node.toast_motion.is_some(),
                        node.toast_motion,
                    ));
                }
                pending.extend(node.children.iter().copied());
            }
            (nodes, stacks)
        };
        for (id, config, motion, bottom) in nodes {
            let state = self.toasts.entry(id).or_insert_with(|| State {
                clock: Clock::new(timeout(&config)),
                phase: Default::default(),
                painted: Default::default(),
                origin: cx.background_executor().now(),
                motion,
                bottom,
                phase_timer: None,
                phase_deadline: None,
                accepted: None,
                config: config.clone(),
                close_focus: cx.focus_handle().tab_stop(true),
                timer: None,
                deadline: None,
                epoch: 0,
                hovered: false,
                bounds: Default::default(),
            });
            state.config = config;
            state.motion = motion;
            state.bottom = bottom;
        }
        for (id, limit, children, layered, motion) in stacks {
            let stack = self.toast_stacks.entry(id).or_default();
            stack.layered_enabled = layered;
            if !layered {
                stack.layered.borrow_mut().suspend();
            } else if motion.is_none() {
                stack.layered.borrow_mut().settle_motion();
            }
            stack
                .origin
                .get_or_insert_with(|| cx.background_executor().now());
            stack
                .layered
                .borrow_mut()
                .retain(&children)
                .expect("validated stack identities");
            let open = children
                .iter()
                .copied()
                .filter(|id| {
                    self.toasts
                        .get(id)
                        .is_some_and(|state| !state.clock.is_closed())
                })
                .collect::<Vec<_>>();
            for id in open.iter().take(open.len().saturating_sub(limit)) {
                self.take_toast_dismissal(*id, ToastDismissal::Overflow, window, cx);
            }
        }
    }
    fn take_toast_dismissal(
        &mut self,
        id: NodeId,
        reason: ToastDismissal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .toasts
            .get(&id)
            .is_none_or(|state| state.clock.is_closed())
        {
            return false;
        }
        let event = {
            let session = self.session.borrow();
            session.tree(self.id).and_then(|tree| {
                let handler = tree.get(id)?.handler?;
                session.accept_toast_dismissal(self.id, id, handler, tree.revision(), reason)
            })
        };
        let Some(event) = event else {
            return false;
        };
        let state = self.toasts.get_mut(&id).unwrap();
        let motion = state
            .motion()
            .filter(|_| state.painted.get() && window.is_window_active() && !cx.reduce_motion());
        state
            .phase
            .borrow_mut()
            .dismiss(reason, state.elapsed(cx), motion, state.bottom)
            .expect("validated phase");
        state.accepted = Some(event);
        state.clock.close();
        state.cancel();
        self.focus.borrow_mut().retire_input(id, window, cx);
        self.finish_toast_dismissals();
        true
    }
    /// Called from input/timers or the deferred frame finish, never child paint.
    fn finish_toast_dismissals(&mut self) -> bool {
        let mut changed = false;
        let mut accepted = Vec::new();
        for state in self.toasts.values_mut() {
            if state.phase.borrow_mut().take_dismissal().is_some() {
                changed = true;
                state.phase_timer = None;
                state.phase_deadline = None;
                accepted.extend(state.accepted.take());
            }
        }
        for token in accepted {
            let event = self.session.borrow().complete_toast_dismissal(token);
            if let Some(event) = event
                && !self.transport.input(event)
                && self.session.borrow_mut().overload(self.id)
            {
                self.transport.fault(self.id);
            }
        }
        changed
    }
    fn schedule_toast_phases(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let live = self.session.borrow().accepts_input(self.id);
        let mut changed = false;
        for (id, state) in &mut self.toasts {
            if !live {
                changed |= !state.phase.borrow().is_closed();
                state.phase.borrow_mut().discard();
                state.clock.close();
                state.cancel();
                state.accepted = None;
                state.phase_timer = None;
                state.phase_deadline = None;
                continue;
            }
            let stack = self
                .session
                .borrow()
                .tree(self.id)
                .and_then(|tree| tree.get(*id))
                .and_then(|node| node.parent);
            let eligible = state.motion.is_some()
                && state.painted.get()
                && window.is_window_active()
                && !cx.reduce_motion()
                && self.focus.borrow().visible(*id)
                && !self.focus.borrow().disabled(*id)
                && stack.is_some_and(|id| {
                    self.focus.borrow().allows(id)
                        && self.toast_stacks.get(&id).is_some_and(|s| s.usable.get())
                });
            if !eligible {
                let before = state.phase.borrow().allows_timeout();
                state.phase.borrow_mut().suspend();
                changed |= before != state.phase.borrow().allows_timeout();
            }
            let now = state.elapsed(cx);
            let due = state.phase.borrow().deadline().filter(|d| now >= d.at());
            if let Some(deadline) = due {
                changed |= state.phase.borrow_mut().finish_deadline(&deadline, now);
            }
            let deadline = state.phase.borrow().deadline();
            let Some(deadline) = deadline else {
                state.phase_timer = None;
                state.phase_deadline = None;
                continue;
            };
            if state
                .phase_deadline
                .as_ref()
                .is_some_and(|d| d.same_phase(&deadline))
            {
                continue;
            }
            state.phase_deadline = Some(deadline.clone());
            let delay = deadline.at().saturating_sub(now);
            let id = *id;
            state.phase_timer = Some(cx.spawn_in(window, async move |owner, cx| {
                cx.background_executor().timer(delay).await;
                let _ = owner.update_in(cx, |view, window, cx| {
                    let Some(state) = view.toasts.get_mut(&id) else {
                        return;
                    };
                    let now = state.elapsed(cx);
                    if !state
                        .phase
                        .borrow()
                        .deadline()
                        .is_some_and(|d| d.same_phase(&deadline))
                    {
                        return;
                    }
                    state.phase_timer = None;
                    state.phase_deadline = None;
                    // Clear before rescheduling so an early timer wake re-arms
                    // the remaining interval instead of keeping a completed task.
                    state.phase.borrow_mut().finish_deadline(&deadline, now);
                    view.schedule_toasts(window, cx);
                    view.sync_tooltips(window, cx);
                    cx.notify();
                });
            }));
        }
        changed | self.finish_toast_dismissals()
    }
    pub(super) fn close_toast(
        &mut self,
        id: NodeId,
        reason: ToastDismissal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.focus.borrow().allows(id) || !self.focus.borrow().visible(id) {
            return;
        }
        if reason == ToastDismissal::Escape {
            if !self
                .focus
                .borrow()
                .handle(id)
                .is_some_and(|handle| handle.contains_focused(window, cx))
            {
                return;
            }
            if self
                .editors
                .values()
                .any(|editor| editor.focus_handle(cx).is_focused(window) && editor.is_composing(cx))
            {
                return;
            }
        }
        if self.take_toast_dismissal(id, reason, window, cx) {
            self.sync_tooltips(window, cx);
            cx.notify();
        }
    }
    pub(super) fn retiring_toasts(&self) -> BTreeSet<NodeId> {
        self.toasts
            .iter()
            .filter(|(_, s)| s.clock.is_closed())
            .map(|(id, _)| *id)
            .collect()
    }
    pub(super) fn closed_toasts(&self) -> BTreeSet<NodeId> {
        self.toasts
            .iter()
            .filter(|(_, state)| state.phase.borrow().is_closed())
            .map(|(id, _)| *id)
            .collect()
    }
    // Called with this synchronization's base hidden set, before adding these
    // visibility-derived ids. Otherwise an unhidden toast could never reappear.
    pub(super) fn invisible_toasts(&self) -> BTreeSet<NodeId> {
        self.toasts
            .keys()
            .filter(|id| !self.focus.borrow().visible(**id))
            .copied()
            .collect()
    }
    pub(super) fn sync_toast_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (id, state) in &mut self.toasts {
            if !self.focus.borrow().visible(*id) || !self.focus.borrow().allows(*id) {
                state.hovered = false;
            }
        }
        for (id, stack) in &mut self.toast_stacks {
            if !self.focus.borrow().visible(*id) || self.focus.borrow().blocks_pointer(*id) {
                stack.hovered = false;
            }
            let handle = self.focus.borrow().handle(*id);
            if handle == stack.focus {
                continue;
            }
            stack.subscriptions.clear();
            stack.focus = handle.clone();
            if let Some(handle) = handle {
                let owner = cx.weak_entity();
                let id = *id;
                stack
                    .subscriptions
                    .push(window.on_focus_in(&handle, cx, move |window, cx| {
                        input(owner.clone(), id, None, window, cx)
                    }));
                let owner = cx.weak_entity();
                stack
                    .subscriptions
                    .push(window.on_focus_out(&handle, cx, move |_, window, cx| {
                        input(owner.clone(), id, None, window, cx)
                    }));
            }
        }
    }
    pub(super) fn schedule_toasts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = self.schedule_toast_phases(window, cx);
        let hovered_stacks = {
            let session = self.session.borrow();
            let tree = session.tree(self.id);
            self.toasts
                .iter()
                .filter(|(id, state)| {
                    state.hovered && !state.clock.is_closed() && self.focus.borrow().visible(**id)
                })
                .filter_map(|(id, _)| tree?.get(*id)?.parent)
                .collect::<BTreeSet<_>>()
        };
        let now = cx.background_executor().now();
        let mut expired = Vec::new();
        for (id, state) in &mut self.toasts {
            let stack_id = self
                .session
                .borrow()
                .tree(self.id)
                .and_then(|tree| tree.get(*id))
                .and_then(|node| node.parent);
            let paused = (state.motion.is_some() && !state.phase.borrow().allows_timeout())
                || stack_id.is_some_and(|id| hovered_stacks.contains(&id))
                || !self.focus.borrow().visible(*id)
                || !self.focus.borrow().allows(*id)
                || stack_id
                    .and_then(|id| self.toast_stacks.get(&id))
                    .is_some_and(|stack| {
                        !stack.usable.get()
                            || (stack.layered_enabled && !stack.painted.get())
                            || stack.hovered
                            || stack
                                .focus
                                .as_ref()
                                .is_some_and(|focus| focus.contains_focused(window, cx))
                    });
            match state.clock.update(timeout(&state.config), paused, now) {
                Plan::Idle => {
                    if state.timer.is_some() {
                        state.cancel();
                    }
                }
                Plan::Expired => {
                    state.cancel();
                    expired.push(*id);
                }
                Plan::After(delay) => {
                    let deadline = now + delay;
                    if state.deadline == Some(deadline) {
                        continue;
                    }
                    state.cancel();
                    state.deadline = Some(deadline);
                    let epoch = state.epoch;
                    let id = *id;
                    state.timer = Some(cx.spawn_in(window, async move |owner, cx| {
                        cx.background_executor().timer(delay).await;
                        let _ = owner.update_in(cx, |view, window, cx| {
                            if view
                                .toasts
                                .get(&id)
                                .is_some_and(|state| state.epoch == epoch)
                            {
                                // Clear before rescheduling; an early wake must get a fresh task.
                                let state = view.toasts.get_mut(&id).unwrap();
                                state.timer = None;
                                state.deadline = None;
                                state.clock.advance(cx.background_executor().now());
                                view.schedule_toasts(window, cx);
                            }
                        });
                    }));
                }
            }
        }
        for id in expired {
            changed |= self.take_toast_dismissal(id, ToastDismissal::Timeout, window, cx);
        }
        if changed {
            self.sync_tooltips(window, cx);
            cx.notify();
        }
    }
    pub(super) fn toast_stack_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let interaction = inherit_interaction(interaction, &node.style);
        let id = node.id;
        if !self.focus.borrow().visible(id) {
            return div().into_any_element();
        }
        let Some(handle) = self.focus.borrow().handle(id) else {
            return div().into_any_element();
        };
        let config = node.toast_stack.as_ref().expect("validated stack");
        let viewport = crate::window_frame::content_bounds(window).size;
        use gpuio_protocol::toast_placement::{Anchor, Placement};
        let placement = node.toast_placement.unwrap_or(Placement {
            anchor: match config.corner {
                ToastCorner::TopLeft => Anchor::TopLeft,
                ToastCorner::TopRight => Anchor::TopRight,
                ToastCorner::BottomLeft => Anchor::BottomLeft,
                ToastCorner::BottomRight => Anchor::BottomRight,
            },
            top: 16.,
            right: 16.,
            bottom: 16.,
            left: 16.,
        });
        fn fit_pair(first: f64, last: f64, extent: gpui::Pixels) -> (gpui::Pixels, gpui::Pixels) {
            let extent = f64::from(f32::from(extent)).max(0.);
            let scale = if first + last > extent {
                extent / (first + last)
            } else {
                1.
            };
            (px((first * scale) as f32), px((last * scale) as f32))
        }
        let (left, right) = fit_pair(placement.left, placement.right, viewport.width);
        let (top, bottom) = fit_pair(placement.top, placement.bottom, viewport.height);
        let width = (viewport.width - left - right).max(px(0.));
        let height = (viewport.height - top - bottom).max(px(0.));
        let usable = width > px(0.) && height > px(0.);
        let visibility = self.focus.clone();
        let observed = self
            .toast_stacks
            .get(&id)
            .expect("retained stack")
            .usable
            .clone();
        let scope = handle.clone();
        let visibility_owner = cx.weak_entity();
        let owner = cx.weak_entity();
        let mut stack = div()
            .id(("toast-stack", id.slot()))
            .track_focus(&handle)
            .role(gpui::Role::Group)
            .aria_label(config.label.clone())
            .w(px((config.width as f32).min(f32::from(width))))
            .min_w_0()
            .min_h_0()
            .max_h(height)
            .flex()
            .flex_col()
            .gap(px(8.))
            .overflow_y_scroll()
            .on_hover(move |hover, window, cx| input(owner.clone(), id, Some(*hover), window, cx));
        let (styled, states) = super::apply_styles(stack, &node.style, interaction, false);
        stack = styled;
        let [focused, hover, pressed, _, _, _, _] = states;
        if let Some(style) = focused
            && handle.contains_focused(window, cx)
        {
            gpui::Refineable::refine(stack.style(), &style);
        }
        if interaction.pointer {
            if let Some(style) = hover {
                stack = stack.hover(move |_| style);
            }
            if let Some(style) = pressed {
                stack = stack.active(move |_| style);
            }
        }
        let content = if node.toast_layering.is_some() || node.toast_motion.is_some() {
            let layering =
                node.toast_layering
                    .unwrap_or(gpuio_protocol::toast_layering::Layering {
                        peek: 0.,
                        gap: 8.,
                        width_step: 0.,
                        visible: 8,
                    });
            self.layered_toast_content(
                tree,
                node,
                interaction,
                placement.anchor,
                layering,
                window,
                cx,
            )
        } else {
            handle.clone().tab_stop(false);
            for child in node.children.iter() {
                stack = stack.child(self.element(tree, *child, interaction, window, cx));
            }
            super::highlight_style::Frame::new(stack, node, &self.focus).into_any_element()
        };
        let mut frame = div()
            .id(("toast-placement", id.slot()))
            .absolute()
            .left(left)
            .top(top)
            .w(width)
            .h(height)
            .overflow_hidden()
            .flex()
            .flex_col();
        frame = match placement.anchor {
            Anchor::TopLeft => frame.items_start().justify_start(),
            Anchor::TopRight => frame.items_end().justify_start(),
            Anchor::BottomLeft => frame.items_start().justify_end(),
            Anchor::BottomRight => frame.items_end().justify_end(),
            Anchor::TopCenter => frame.items_center().justify_start(),
            Anchor::BottomCenter => frame.items_center().justify_end(),
            Anchor::LeftCenter => frame.items_start().justify_center(),
            Anchor::RightCenter => frame.items_end().justify_center(),
        };
        // The synthetic stack focus scope must not lift a newly mounted toast
        // over an existing modal outside its declarative ancestry.
        let priority = node
            .parent
            .map_or(0, |parent| self.focus.borrow().layer(parent))
            + 1;
        deferred(super::overlay::ViewportSurface {
            content: div()
                .relative()
                .w(viewport.width)
                .h(viewport.height)
                .child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, window, cx| {
                            visibility.borrow_mut().track_card_clipped(id, !usable);
                            if !usable && scope.contains_focused(window, cx) {
                                window.blur(cx);
                            }
                            if observed.replace(usable) != usable {
                                input(visibility_owner.clone(), id, None, window, cx);
                            }
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
                .child(crate::semantics::State {
                    identity: None,
                    hidden: !usable,
                    metadata: None,
                    busy: false,
                    disabled: false,
                    read_only: false,
                    modal: false,
                    live: None,
                    element: super::highlight_style::Frame::clip(
                        frame.child(content),
                        id,
                        &self.focus,
                    ),
                })
                .into_any_element(),
        })
        .with_priority(priority)
        .into_any_element()
    }
    pub(super) fn toast_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let interaction = inherit_interaction(interaction, &node.style);
        let id = node.id;
        let Some(state) = self.toasts.get(&id) else {
            return div().into_any_element();
        };
        if state.phase.borrow().is_closed() || !self.focus.borrow().visible(id) {
            return div().into_any_element();
        }
        let Some(scope) = self.focus.borrow().handle(id) else {
            return div().into_any_element();
        };
        let focus = state.close_focus.clone();
        let config = state.config.clone();
        let bounds = state.bounds.clone();
        self.focus.borrow_mut().surface(id, bounds.clone());
        let mut panel = div()
            .id(("toast", id.slot()))
            .track_focus(&scope)
            .w_full()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .p(px(12.))
            .gap(px(8.))
            .rounded(px(6.))
            .bg(rgba(0x20242aff))
            .text_color(rgba(0xffffffff))
            .occlude()
            .role(match config.politeness {
                ToastPoliteness::Polite => gpui::Role::Status,
                ToastPoliteness::Assertive => gpui::Role::Alert,
            })
            .aria_label(config.label.clone());
        let (styled, states) = super::apply_styles(panel, &node.style, interaction, false);
        panel = styled;
        let [focused, hover, pressed, _, _, _, _] = states;
        if let Some(style) = focused
            && scope.contains_focused(window, cx)
        {
            gpui::Refineable::refine(panel.style(), &style);
        }
        if interaction.pointer {
            if let Some(style) = hover {
                panel = panel.hover(move |_| style);
            }
            if let Some(style) = pressed {
                panel = panel.active(move |_| style);
            }
        }
        for child in node.children.iter() {
            panel = panel.child(self.element(tree, *child, interaction, window, cx));
        }
        let hover_owner = cx.weak_entity();
        panel = panel.on_hover(move |hover, window, cx| {
            let owner = hover_owner.clone();
            let hover = *hover;
            window.defer(cx, move |window, cx| {
                let _ = owner.update(cx, |view, cx| {
                    if let Some(state) = view.toasts.get_mut(&id) {
                        state.hovered = hover;
                    }
                    view.schedule_toasts(window, cx);
                    if view
                        .session
                        .borrow()
                        .tree(view.id)
                        .and_then(|tree| tree.get(id))
                        .and_then(|node| node.parent)
                        .and_then(|parent| {
                            view.session
                                .borrow()
                                .tree(view.id)
                                .and_then(|tree| tree.get(parent))
                                .map(|n| n.toast_layering.is_some())
                        })
                        .unwrap_or(false)
                    {
                        cx.notify();
                    }
                });
            });
        });
        let owner = cx.weak_entity();
        let ax_owner = owner.clone();
        let key_owner = owner.clone();
        let action_owner = owner.clone();
        let focus_gate = self.focus.clone();
        let ax_focus = focus.clone();
        let mut close = div()
            .id(("toast-close", id.slot()))
            .track_focus(&focus)
            .role(gpui::Role::Button)
            .aria_label(config.close_label.clone())
            .child("×")
            .px(px(8.))
            .py(px(4.))
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                let _ = ax_owner.update(cx, |view, cx| {
                    view.close_toast(id, ToastDismissal::CloseButton, window, cx)
                });
            })
            .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                if focus_gate.borrow().allows(id) && focus_gate.borrow().visible(id) {
                    window.focus(&ax_focus, cx);
                }
            });
        close = close.on_click(move |event, window, cx| {
            if interaction.pointer || matches!(event, gpui::ClickEvent::Keyboard(_)) {
                let _ = owner.update(cx, |view, cx| {
                    view.close_toast(id, ToastDismissal::CloseButton, window, cx)
                });
                cx.stop_propagation();
            }
        });
        if interaction.pointer {
            close = close.cursor_pointer();
        } else {
            close = close.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                window.prevent_default()
            });
        }
        let pointer_gate = self.focus.clone();
        panel = panel.capture_any_mouse_down(move |_, window, cx| {
            if !interaction.pointer || pointer_gate.borrow().blocks_pointer(id) {
                window.prevent_default();
                cx.stop_propagation();
            }
        });
        let gate = self.focus.clone();
        let record = focus.clone();
        close = close.child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                        gate.borrow_mut().record(
                            id,
                            record.clone(),
                            true,
                            record.is_focused(window),
                            bounds,
                        );
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
        let layered = node
            .parent
            .and_then(|p| tree.get(p))
            .is_some_and(|p| p.toast_layering.is_some() || p.toast_motion.is_some());
        panel = panel
            .child(
                canvas(move |rect, _, _| bounds.set(rect), |_, _, _, _| ())
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .child(close)
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    let _ = key_owner.update(cx, |view, cx| {
                        view.close_toast(id, ToastDismissal::Escape, window, cx)
                    });
                    cx.stop_propagation();
                }
            })
            .on_action(move |_: &gpui_base::input::Escape, window, cx| {
                let _ = action_owner.update(cx, |view, cx| {
                    view.close_toast(id, ToastDismissal::Escape, window, cx)
                });
                cx.stop_propagation();
            })
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation());
        if !layered {
            let phase = Rc::downgrade(&self.toasts[&id].phase);
            let painted = self.toasts[&id].painted.clone();
            let sample = self.toasts[&id]
                .phase
                .borrow()
                .sample(self.toasts[&id].elapsed(cx), None, false)
                .expect("immediate phase");
            panel = panel
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let clipped = bounds.intersect(&window.content_mask().bounds);
                            if clipped.size.width > px(0.)
                                && clipped.size.height > px(0.)
                                && let Some(state) = phase.upgrade()
                            {
                                painted.set(true);
                                state.borrow_mut().painted(&sample);
                            }
                        },
                    )
                    .absolute()
                    .size_full(),
                );
        }
        crate::semantics::State {
            identity: None,
            busy: false,
            hidden: false,
            metadata: None,
            element: super::highlight_style::Frame::new(panel, node, &self.focus),
            disabled: false,
            read_only: false,
            modal: false,
            live: Some(match config.politeness {
                ToastPoliteness::Polite => gpui::accesskit::Live::Polite,
                ToastPoliteness::Assertive => gpui::accesskit::Live::Assertive,
            }),
        }
        .into_any_element()
    }
}
