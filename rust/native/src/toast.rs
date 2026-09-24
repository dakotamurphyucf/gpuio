//! Window-owned notification sessions. Declarative content stays in the tree;
//! native dismissal fuses the session until a new node identity is mounted.
use super::{
    Interaction, View,
    toast_clock::{Clock, Plan},
};
use gpui::{
    App, Context, FocusHandle, Task, WeakEntity, Window, canvas, deferred, div, prelude::*, px,
    rgba,
};
use gpuio_protocol::{NodeId, v1::*};
use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) struct State {
    config: Arc<ToastConfig>,
    clock: Clock,
    pub(super) close_focus: FocusHandle,
    timer: Option<Task<()>>,
    deadline: Option<Instant>,
    epoch: u64,
    hovered: bool,
    pub(super) bounds: std::rc::Rc<std::cell::Cell<gpui::Bounds<gpui::Pixels>>>,
}
impl State {
    #[cfg(feature = "native-tests")]
    pub(super) fn status(&self) -> (bool, bool) {
        (self.clock.is_closed(), self.timer.is_some())
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
}
fn inherit_interaction(mut interaction: Interaction, styles: &[Style]) -> Interaction {
    for style in styles {
        if let Style::Fields(fields) = style {
            for field in fields {
                match field {
                    Field::PointerEvents(value) => interaction.pointer = *value,
                    Field::UserSelect(value) => interaction.selectable = *value,
                    Field::SelectionColor(value) => {
                        interaction.selection_color = super::color(value)
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
            }
        });
    });
}
impl View {
    pub(super) fn sync_toasts(&mut self, cx: &mut Context<Self>) {
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
                    nodes.push((id, config.clone()));
                }
                if let Some(config) = &node.toast_stack {
                    stacks.push((id, config.max_visible as usize, node.children.clone()));
                }
                pending.extend(node.children.iter().copied());
            }
            (nodes, stacks)
        };
        for (id, config) in nodes {
            let state = self.toasts.entry(id).or_insert_with(|| State {
                clock: Clock::new(timeout(&config)),
                config: config.clone(),
                close_focus: cx.focus_handle().tab_stop(true),
                timer: None,
                deadline: None,
                epoch: 0,
                hovered: false,
                bounds: Default::default(),
            });
            state.config = config;
        }
        for (id, limit, children) in stacks {
            self.toast_stacks.entry(id).or_default();
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
                self.take_toast_dismissal(*id, ToastDismissal::Overflow);
            }
        }
    }
    fn take_toast_dismissal(&mut self, id: NodeId, reason: ToastDismissal) -> bool {
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
                session.toast_dismissed(self.id, id, handler, tree.revision(), reason)
            })
        };
        let Some(event) = event else {
            return false;
        };
        let state = self.toasts.get_mut(&id).unwrap();
        state.clock.close();
        state.cancel();
        if !self.transport.input(event) && self.session.borrow_mut().overload(self.id) {
            self.transport.fault(self.id);
        }
        true
    }
    fn close_toast(
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
        if self.take_toast_dismissal(id, reason) {
            self.sync_tooltips(window, cx);
            cx.notify();
        }
    }
    pub(super) fn closed_toasts(&self) -> BTreeSet<NodeId> {
        self.toasts
            .iter()
            .filter(|(_, state)| state.clock.is_closed())
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
        let now = Instant::now();
        let mut expired = Vec::new();
        for (id, state) in &mut self.toasts {
            let stack_id = self
                .session
                .borrow()
                .tree(self.id)
                .and_then(|tree| tree.get(*id))
                .and_then(|node| node.parent);
            let paused = stack_id.is_some_and(|id| hovered_stacks.contains(&id))
                || !self.focus.borrow().visible(*id)
                || !self.focus.borrow().allows(*id)
                || stack_id
                    .and_then(|id| self.toast_stacks.get(&id))
                    .is_some_and(|stack| {
                        stack.hovered
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
                                state.clock.advance(Instant::now());
                                view.schedule_toasts(window, cx);
                            }
                        });
                    }));
                }
            }
        }
        let mut changed = false;
        for id in expired {
            changed |= self.take_toast_dismissal(id, ToastDismissal::Timeout);
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
        let viewport = window.viewport_size();
        let owner = cx.weak_entity();
        let mut stack = div()
            .id(("toast-stack", id.slot()))
            .track_focus(&handle)
            .role(gpui::Role::Group)
            .aria_label(config.label.clone())
            .w(px(
                (config.width as f32).min((f32::from(viewport.width) - 32.).max(0.))
            ))
            .max_h((viewport.height - px(32.)).max(px(0.)))
            .flex()
            .flex_col()
            .gap(px(8.))
            .overflow_y_scroll()
            .on_hover(move |hover, window, cx| input(owner.clone(), id, Some(*hover), window, cx));
        let (styled, _) = super::apply_styles(stack, &node.style, interaction, false);
        stack = styled;
        for child in node.children.iter() {
            stack = stack.child(self.element(tree, *child, interaction, window, cx));
        }
        let mut frame = div()
            .w(viewport.width)
            .h(viewport.height)
            .p(px(16.))
            .flex()
            .flex_col();
        frame = match config.corner {
            ToastCorner::TopLeft => frame.items_start().justify_start(),
            ToastCorner::TopRight => frame.items_end().justify_start(),
            ToastCorner::BottomLeft => frame.items_start().justify_end(),
            ToastCorner::BottomRight => frame.items_end().justify_end(),
        };
        // The synthetic stack focus scope must not lift a newly mounted toast
        // over an existing modal outside its declarative ancestry.
        let priority = node
            .parent
            .map_or(0, |parent| self.focus.borrow().layer(parent))
            + 1;
        deferred(super::overlay::ViewportSurface {
            content: frame.child(stack).into_any_element(),
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
        if state.clock.is_closed() || !self.focus.borrow().visible(id) {
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
                    if bounds.intersects(&window.content_mask().bounds) {
                        gate.borrow_mut().record(
                            id,
                            record.clone(),
                            true,
                            record.is_focused(window),
                        );
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
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
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
        crate::semantics::State {
            element: panel,
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
