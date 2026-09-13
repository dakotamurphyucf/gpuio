//! Retained arbitrary tooltip content with window-owned transient state. All
//! delayed callbacks weakly address the host and a generational node; unmount
//! drops timers and subscriptions, never retaining a host through its own task.
use super::{Interaction, View};
use gpui::{
    AnyElement, App, Context, FocusHandle, Subscription, Task, WeakEntity, Window, canvas,
    deferred, div, prelude::*, px, rgba,
};
use gpuio_protocol::{NodeId, v1::*};
use std::{
    cell::Cell,
    collections::BTreeSet,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) struct State {
    config: Arc<TooltipConfig>,
    open: bool,
    requested: bool,
    anchor_hover: bool,
    panel_hover: bool,
    focused: bool,
    suppressed: bool,
    epoch: u64,
    pending: Option<bool>,
    timer: Option<Task<()>>,
    focus_handle: Option<FocusHandle>,
    subscriptions: Vec<Subscription>,
    bounds: Rc<Cell<gpui::Bounds<gpui::Pixels>>>,
}
impl State {
    fn new(config: Arc<TooltipConfig>) -> Self {
        let open = !config.disabled
            && match config.open_state {
                TooltipOpenState::Managed(open) | TooltipOpenState::Controlled(open) => open,
            };
        Self {
            config,
            open,
            requested: open,
            anchor_hover: false,
            panel_hover: false,
            focused: false,
            suppressed: false,
            epoch: 0,
            pending: None,
            timer: None,
            focus_handle: None,
            subscriptions: Vec::new(),
            bounds: Default::default(),
        }
    }
    fn cancel(&mut self) {
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("tooltip timer epoch exhausted");
        self.timer = None;
        self.pending = None;
    }
    fn configure(&mut self, config: Arc<TooltipConfig>) {
        if self.config == config {
            return;
        }
        self.cancel();
        match config.open_state {
            TooltipOpenState::Controlled(open) => {
                if self.open != open {
                    self.requested = open;
                }
                self.open = open;
            }
            TooltipOpenState::Managed(_) => {
                // An unaccepted controlled intent is not the managed value.
                if matches!(self.config.open_state, TooltipOpenState::Controlled(_)) {
                    self.requested = self.open;
                }
            }
        }
        if config.disabled {
            self.open = false;
            self.requested = false;
        }
        if !self.open {
            self.panel_hover = false;
        }
        self.config = config;
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn diagnostics(&self) -> String {
        format!(
            "open={}, requested={}, anchor={}, panel={}, focused={}, suppressed={}, pending={:?}, config={:?}, bounds={:?}",
            self.open,
            self.requested,
            self.anchor_hover,
            self.panel_hover,
            self.focused,
            self.suppressed,
            self.pending,
            self.config,
            self.bounds.get()
        )
    }
    fn interested(&self) -> bool {
        self.anchor_hover || self.focused || (self.config.hoverable && self.panel_hover)
    }
}

#[derive(Clone, Copy)]
enum Input {
    Anchor(bool),
    Panel(bool),
    Focus,
    Dismiss,
}

// Input handlers can run during paint/focus callbacks while the host is borrowed.
// Defer the update and read current focus rather than replaying an old focus flag.
fn input(owner: WeakEntity<View>, id: NodeId, input: Input, window: &mut Window, cx: &mut App) {
    window.defer(cx, move |window, cx| {
        let _ = owner.update(cx, |view, cx| view.tooltip_input(id, input, window, cx));
    });
}

impl View {
    pub(super) fn sync_tooltips(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let nodes = {
            let session = self.session.borrow();
            let mut nodes = Vec::new();
            if let Some(tree) = session.tree(self.id) {
                let mut stack = tree.root().into_iter().collect::<Vec<_>>();
                while let Some(id) = stack.pop() {
                    let node = tree.get(id).expect("validated node");
                    if let Some(config) = &node.tooltip {
                        nodes.push((id, config.clone(), node.children[1]));
                    }
                    stack.extend(node.children.iter().copied());
                }
            }
            nodes
        };
        let present = nodes.iter().map(|(id, _, _)| *id).collect::<BTreeSet<_>>();
        self.tooltips.retain(|id, _| present.contains(id));
        let mut hidden = BTreeSet::new();
        let mut reschedule = Vec::new();
        for (id, config, content) in nodes {
            let state = self
                .tooltips
                .entry(id)
                .or_insert_with(|| State::new(config.clone()));
            if state.pending.is_some() && state.config != config {
                reschedule.push(id);
            }
            let was_open = state.open;
            state.configure(config);
            if was_open && !state.open {
                self.tooltip_last_closed = Some(Instant::now());
            }
            if !state.open {
                hidden.insert(content);
            }
        }
        hidden.extend(
            self.palettes
                .iter()
                .filter(|(_, state)| state.closed)
                .map(|(id, _)| *id),
        );
        hidden.extend(self.closed_toasts());
        self.focus.borrow_mut().set_hidden(hidden.clone());
        hidden.extend(self.dismiss_hidden_palettes());
        hidden.extend(self.invisible_toasts());
        self.focus.borrow_mut().set_hidden(hidden);
        self.focus.borrow_mut().sync(window, cx);
        self.pointer_capture.borrow_mut().sync(window);
        for (id, state) in &mut self.tooltips {
            let handle = self.focus.borrow().handle(*id);
            if self.focus.borrow().blocks_pointer(*id) {
                state.cancel();
            }
            if state.focus_handle == handle {
                continue;
            }
            state.subscriptions.clear();
            state.focus_handle = handle.clone();
            state.focused = handle
                .as_ref()
                .is_some_and(|handle| handle.contains_focused(window, cx));
            if let Some(handle) = handle {
                let owner = cx.weak_entity();
                let id = *id;
                state
                    .subscriptions
                    .push(window.on_focus_in(&handle, cx, move |window, cx| {
                        input(owner.clone(), id, Input::Focus, window, cx);
                    }));
                let owner = cx.weak_entity();
                state
                    .subscriptions
                    .push(window.on_focus_out(&handle, cx, move |_, window, cx| {
                        input(owner.clone(), id, Input::Focus, window, cx);
                    }));
            }
        }
        self.sync_toast_focus(window, cx);
        self.schedule_toasts(window, cx);
        for id in reschedule {
            self.tooltip_input(id, Input::Focus, window, cx);
        }
    }

    fn tooltip_input(
        &mut self,
        id: NodeId,
        input: Input,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let allowed = !self.focus.borrow().blocks_pointer(id);
        let Some(state) = self.tooltips.get_mut(&id) else {
            return;
        };
        state.focused = state
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.contains_focused(window, cx));
        match input {
            Input::Anchor(hover) => state.anchor_hover = hover,
            Input::Panel(hover) => state.panel_hover = hover,
            Input::Focus => (),
            Input::Dismiss => state.suppressed = true,
        }
        if !state.interested() && !matches!(input, Input::Dismiss) {
            state.suppressed = false;
        }
        let desired = allowed && !state.config.disabled && !state.suppressed && state.interested();
        if state.pending == Some(desired) {
            return;
        }
        state.cancel();
        if desired == state.requested {
            return;
        }
        let delay = if desired {
            let recent = self.tooltip_last_closed.is_some_and(|closed| {
                closed.elapsed() < Duration::from_nanos(state.config.skip_delay_ns as u64)
            });
            if state.focused || recent {
                0
            } else {
                state.config.show_delay_ns
            }
        } else if matches!(input, Input::Dismiss) || !allowed || state.config.disabled {
            0
        } else {
            state.config.hide_delay_ns
        };
        if delay == 0 {
            self.tooltip_request(id, desired, window, cx);
        } else {
            state.pending = Some(desired);
            let epoch = state.epoch;
            state.timer = Some(cx.spawn_in(window, async move |owner, cx| {
                cx.background_executor()
                    .timer(Duration::from_nanos(delay as u64))
                    .await;
                let _ =
                    owner.update_in(cx, |view, window, cx| {
                        if view.tooltips.get(&id).is_some_and(|state| {
                            state.epoch == epoch && state.pending == Some(desired)
                        }) {
                            view.tooltip_request(id, desired, window, cx);
                        }
                    });
            }));
        }
    }

    fn tooltip_request(
        &mut self,
        id: NodeId,
        open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let blocked = self.focus.borrow().blocks_pointer(id);
        let Some(state) = self.tooltips.get_mut(&id) else {
            return;
        };
        state.cancel();
        if state.requested == open || (open && (blocked || state.config.disabled)) {
            return;
        }
        state.requested = open;
        if matches!(state.config.open_state, TooltipOpenState::Managed(_)) {
            state.open = open;
            if !open {
                state.panel_hover = false;
                self.tooltip_last_closed = Some(Instant::now());
            }
        }

        let event = {
            let session = self.session.borrow();
            session.tree(self.id).and_then(|tree| {
                let node = tree.get(id)?;
                session.tooltip_open_changed(self.id, id, node.handler?, tree.revision(), open)
            })
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.id)
        {
            self.transport.fault(self.id);
        }
        self.sync_tooltips(window, cx);
        cx.notify();
    }

    pub(super) fn tooltip_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = node.id;
        let config = node.tooltip.as_ref().expect("validated tooltip");
        let state = self.tooltips.get(&id).expect("mounted tooltip");
        let open = state.open && !self.focus.borrow().blocks_pointer(id);
        let panel_bounds = state.bounds.clone();
        let handle = self
            .focus
            .borrow()
            .handle(id)
            .expect("visible tooltip scope");
        let anchor_bounds = self
            .focus
            .borrow()
            .anchor(id)
            .expect("visible tooltip scope");
        let bounds = anchor_bounds.clone();
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        let owner = cx.weak_entity();
        let click_owner = owner.clone();
        let escape_owner = owner.clone();
        let action_owner = owner.clone();
        let mut wrapper = div()
            .id(("gpuio-tooltip", identity))
            .track_focus(&handle)
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() && open
                {
                    input(escape_owner.clone(), id, Input::Dismiss, window, cx);
                    cx.stop_propagation();
                }
            })
            .on_action(move |_: &gpui_base::input::Escape, window, cx| {
                if open {
                    input(action_owner.clone(), id, Input::Dismiss, window, cx);
                    cx.stop_propagation();
                } else {
                    cx.propagate();
                }
            })
            .child(
                div()
                    .id(("gpuio-tooltip-anchor", identity))
                    .on_hover(move |hover, window, cx| {
                        input(owner.clone(), id, Input::Anchor(*hover), window, cx)
                    })
                    .capture_any_mouse_down(move |_, window, cx| {
                        input(click_owner.clone(), id, Input::Dismiss, window, cx)
                    })
                    .child(self.element(tree, node.children[0], interaction, window, cx))
                    .child(
                        canvas(
                            move |bounds_value, _, _| bounds.set(bounds_value),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    ),
            );
        if open {
            let owner = cx.weak_entity();
            let hoverable = config.hoverable;
            let mut panel_interaction = Interaction {
                pointer: interaction.pointer && hoverable,
                ..interaction
            };
            for style in node.style.iter() {
                if let Style::Fields(fields) = style {
                    for field in fields {
                        match field {
                            Field::PointerEvents(value) => {
                                panel_interaction.pointer = *value && hoverable
                            }
                            Field::UserSelect(value) => panel_interaction.selectable = *value,
                            Field::SelectionColor(value) => {
                                panel_interaction.selection_color = super::color(value)
                            }
                            _ => (),
                        }
                    }
                }
            }
            let mut panel = div()
                .id(("gpuio-tooltip-panel", identity))
                .role(gpui::Role::Tooltip)
                .aria_label(config.label.clone())
                .flex()
                .flex_col()
                .w(px(config.width as f32)
                    .min((window.viewport_size().width - px(16.)).max(px(1.))))
                .max_h((window.viewport_size().height - px(16.)).max(px(1.)))
                .p(px(8.))
                .rounded(px(4.))
                .bg(rgba(0x25272aff))
                .border_1()
                .border_color(rgba(0x80808080));
            let (styled, states) =
                super::apply_styles(panel, &node.style, panel_interaction, false);
            panel = styled;
            let [focused, hovered, pressed, _, _, _, _] = states;
            if let Some(style) = focused
                && handle.contains_focused(window, cx)
            {
                panel.style().refine(&style);
            }
            if let Some(style) = hovered {
                panel = panel.hover(move |_| style);
            }
            if let Some(style) = pressed {
                panel = panel.active(move |_| style);
            }
            if panel_interaction.pointer {
                panel = panel
                    .occlude()
                    .overflow_y_scroll()
                    .on_hover(move |hover, window, cx| {
                        input(owner.clone(), id, Input::Panel(*hover), window, cx)
                    })
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
            }
            if !panel_interaction.pointer {
                panel = panel.capture_any_mouse_down(|_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                });
            }
            self.focus.borrow_mut().surface(id, panel_bounds.clone());
            let panel = panel
                .child(self.element(tree, node.children[1], panel_interaction, window, cx))
                .child(
                    canvas(
                        move |bounds, _, _| panel_bounds.set(bounds),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                );
            let priority = self.focus.borrow().layer(id);
            wrapper = wrapper.child(
                deferred(super::popup::Surface {
                    trigger: anchor_bounds,
                    placement: node.placement.unwrap_or(Placement {
                        side: Side::Top,
                        align: Align::Center,
                        offset: 6.,
                    }),
                    content: panel.into_any_element(),
                })
                .with_priority(priority),
            );
        }
        wrapper.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_ownership_does_not_apply_an_unaccepted_controlled_intent() {
        let config = Arc::new(TooltipConfig {
            label: "Details".into(),
            width: 200.,
            open_state: TooltipOpenState::Controlled(false),
            disabled: false,
            hoverable: true,
            show_delay_ns: 0,
            hide_delay_ns: 0,
            skip_delay_ns: 0,
        });
        let mut state = State::new(config.clone());
        state.requested = true;
        state.panel_hover = true;
        state.configure(Arc::new(TooltipConfig {
            open_state: TooltipOpenState::Managed(true),
            ..(*config).clone()
        }));
        assert!(!state.open, "initial value is only used when mounting");
        assert!(
            !state.panel_hover,
            "a closed panel cannot retain hover without a mouse-leave event"
        );
        assert!(
            !state.requested,
            "the next managed hover must be able to open"
        );
        state.configure(Arc::new(TooltipConfig {
            open_state: TooltipOpenState::Controlled(true),
            ..(*config).clone()
        }));
        assert!(state.open);
        state.configure(Arc::new(TooltipConfig {
            open_state: TooltipOpenState::Managed(false),
            ..(*config).clone()
        }));
        assert!(
            state.open,
            "returning to managed mode retains accepted visibility"
        );
    }
}
