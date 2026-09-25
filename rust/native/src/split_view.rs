//! Native resizing stays on the GPUI thread; OCaml observes completed gestures.
use super::*;
use gpui::{Entity, FocusHandle, Subscription};
use gpui_base::{ResizablePanelEvent, ResizablePanelGroup, ResizableState, resizable_panel};
use gpuio_protocol::split::{Axis, Config, Snapshot};

pub(super) struct State {
    pub(super) native: Entity<ResizableState>,
    pub(super) focus: FocusHandle,
    config: Arc<Config>,
    _resized: Subscription,
}
impl State {
    fn new(id: NodeId, config: Arc<Config>, window: &mut Window, cx: &mut Context<View>) -> Self {
        let native = cx.new(|_| ResizableState::default());
        let resized = cx.subscribe_in(
            &native,
            window,
            move |view, entity, _: &ResizablePanelEvent, _, cx| view.emit_split(id, entity, cx),
        );
        Self {
            native,
            focus: cx.focus_handle().tab_stop(true),
            config,
            _resized: resized,
        }
    }
    fn cancel(&self, window: &mut Window, cx: &mut App) -> bool {
        let active = self.native.read(cx).is_resizing();
        self.native
            .update(cx, |state, cx| state.cancel_resize(window, cx));
        active
    }
}
fn sizes(entity: &Entity<ResizableState>, cx: &App) -> Option<Snapshot> {
    let values = entity.read(cx).sizes();
    if values.len() != 2 {
        return None;
    }
    let snapshot = Snapshot {
        first: f32::from(values[0]) as f64,
        second: f32::from(values[1]) as f64,
    };
    snapshot.is_valid().then_some(snapshot)
}
// Pointer-event inheritance uses the nearest explicit field, matching View rendering.
fn pointer_enabled(tree: &crate::tree::Tree, mut id: NodeId) -> bool {
    loop {
        let Some(node) = tree.get(id) else {
            return false;
        };
        for style in node.style.iter().rev() {
            if let Style::Fields(fields) = style {
                for field in fields.iter().rev() {
                    if let Field::PointerEvents(enabled) = field {
                        return *enabled;
                    }
                }
            }
        }
        let Some(parent) = node.parent else {
            return true;
        };
        id = parent;
    }
}
impl View {
    pub(super) fn cancel_split_drag(&mut self, window: &mut Window, cx: &mut App) -> bool {
        let mut cancelled = false;
        for state in self.splits.values() {
            cancelled = state.cancel(window, cx) || cancelled;
        }
        cancelled
    }
    pub(super) fn sync_splits(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.split_activation.is_none() {
            self.split_activation =
                Some(cx.observe_window_activation(window, |view, window, cx| {
                    if !window.is_window_active() {
                        view.cancel_split_drag(window, cx);
                    }
                }));
        }
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.splits.retain(|id, state| {
            let config = tree
                .and_then(|tree| tree.get(*id))
                .and_then(|node| node.split.as_ref());
            let keep = config.is_some_and(|config| {
                config.axis == state.config.axis
                    && config.reset_generation == state.config.reset_generation
            });
            if !keep
                || !self.focus.borrow().allows(*id)
                || tree.is_none_or(|tree| !pointer_enabled(tree, *id))
            {
                state.cancel(window, cx);
            }
            if let Some(config) = config {
                state.config = config.clone();
            }
            keep
        });
    }
    fn emit_split(&self, id: NodeId, entity: &Entity<ResizableState>, cx: &App) {
        if !self.focus.borrow().allows(id) {
            return;
        }
        let Some(state) = self.splits.get(&id) else {
            return;
        };
        if state.native.entity_id() != entity.entity_id() {
            return;
        }
        let Some(snapshot) = sizes(entity, cx) else {
            return;
        };
        let event = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return;
            };
            let Some(node) = tree.get(id) else {
                return;
            };
            let Some(config) = &node.split else {
                return;
            };
            if config.reset_generation != state.config.reset_generation {
                return;
            }
            let Some(handler) = node.handler else {
                return;
            };
            session
                .press(self.id, id, handler, tree.revision())
                .map(|_| {
                    Event::SplitResized(
                        self.id,
                        id,
                        handler,
                        tree.revision(),
                        config.reset_generation,
                        snapshot,
                    )
                })
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.id)
        {
            self.transport.fault(self.id);
        }
    }
    fn resize_split(
        &mut self,
        id: NodeId,
        amount: Resize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.focus.borrow().allows(id) {
            return;
        }
        let Some(state) = self.splits.get(&id) else {
            return;
        };
        let Some(current) = sizes(&state.native, cx) else {
            return;
        };
        let target = match amount {
            Resize::Decrease => current.first - state.config.keyboard_step,
            Resize::Increase => current.first + state.config.keyboard_step,
            Resize::Minimum => state.config.minimum_first,
            Resize::Maximum => state.config.maximum_first,
        };
        state.native.update(cx, |state, cx| {
            state.resize_panel(0, px(target as f32), window, cx)
        });
        cx.notify();
    }
    pub(super) fn split_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = node.id;
        let config = node.split.clone().expect("validated split");
        let state = self
            .splits
            .entry(id)
            .or_insert_with(|| State::new(id, config.clone(), window, cx));
        let native = state.native.clone();
        let focus = state.focus.clone();
        let value = sizes(&native, cx).map_or(config.initial_first, |snapshot| snapshot.first);
        let axis = match config.axis {
            Axis::Horizontal => gpui::Axis::Horizontal,
            Axis::Vertical => gpui::Axis::Vertical,
        };
        let gate = self.focus.clone();
        let owner = cx.weak_entity();
        let appearance_config = config.clone();
        let appearance = Rc::new(
            move |_: &gpui_base::ResizeHandleContext, window: &mut Window, cx: &mut App| {
                let config = &appearance_config;
                let color = window.text_style().color;
                let mut handle = div()
                    .id("gpuio-split-divider")
                    .flex_none()
                    .track_focus(&focus)
                    .tab_index(0)
                    .role(gpui::Role::Splitter)
                    .aria_label(config.label.clone())
                    .aria_orientation(if config.axis == Axis::Horizontal {
                        gpui::accesskit::Orientation::Vertical
                    } else {
                        gpui::accesskit::Orientation::Horizontal
                    })
                    .aria_numeric_value(value)
                    .aria_min_numeric_value(config.minimum_first)
                    .aria_max_numeric_value(config.maximum_first)
                    .aria_numeric_value_step(config.keyboard_step)
                    .bg(color);
                handle = if config.axis == Axis::Horizontal {
                    handle.w(px(1.)).h_full()
                } else {
                    handle.h(px(1.)).w_full()
                };
                let focus_action = focus.clone();
                let focus_gate = gate.clone();
                handle =
                    handle.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                        if focus_gate.borrow().allows(id) {
                            window.focus(&focus_action, cx);
                        }
                    });
                for (action, resize) in [
                    (gpui::AccessibleAction::Increment, Resize::Increase),
                    (gpui::AccessibleAction::Decrement, Resize::Decrease),
                ] {
                    let owner = owner.clone();
                    handle = handle.on_a11y_action(action, move |_, window, cx| {
                        let _ =
                            owner.update(cx, |view, cx| view.resize_split(id, resize, window, cx));
                    });
                }
                let key_owner = owner.clone();
                let direction = config.axis;
                handle = handle.on_key_down(move |event, window, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    let amount = match (direction, event.keystroke.key.as_str()) {
                        (_, "home") => Some(Resize::Minimum),
                        (_, "end") => Some(Resize::Maximum),
                        (Axis::Horizontal, "left") | (Axis::Vertical, "up") => {
                            Some(Resize::Decrease)
                        }
                        (Axis::Horizontal, "right") | (Axis::Vertical, "down") => {
                            Some(Resize::Increase)
                        }
                        _ => None,
                    };
                    if let Some(amount) = amount {
                        let _ = key_owner
                            .update(cx, |view, cx| view.resize_split(id, amount, window, cx));
                        cx.stop_propagation();
                    }
                });
                let record_gate = gate.clone();
                let record_focus = focus.clone();
                handle = handle.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, window, _| {
                            record_gate.borrow_mut().record(
                                id,
                                record_focus.clone(),
                                true,
                                record_focus.is_focused(window),
                            );
                        },
                    )
                    .absolute()
                    .size_full(),
                );
                let _ = cx;
                Some(handle.into_any_element())
            },
        );
        let first = self.element(tree, node.children[0], interaction, window, cx);
        let second = self.element(tree, node.children[1], interaction, window, cx);
        ResizablePanelGroup::new((
            "gpuio-split",
            ((id.generation() as u64) << 32) | id.slot() as u64,
        ))
        .axis(axis)
        .pointer_resizing(interaction.pointer)
        .with_state(&native)
        .with_handle_appearance(appearance)
        .child(
            resizable_panel()
                .size(px(config.initial_first as f32))
                .size_range(px(config.minimum_first as f32)..px(config.maximum_first as f32))
                .flex_none()
                .child(first),
        )
        .child(
            resizable_panel()
                .size_range(px(config.minimum_second as f32)..gpui::Pixels::MAX)
                .child(second),
        )
        .into_any_element()
    }
}
#[derive(Clone, Copy)]
enum Resize {
    Decrease,
    Increase,
    Minimum,
    Maximum,
}
