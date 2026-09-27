//! One native InputState plus numeric policy. Editor notifications and semantic
//! commands publish asynchronously through the retained owner's event route.
use super::{SharedSession, View, editor, focus};
use crate::{
    number_input_state::{Outcome, State as Model},
    transport::Transport,
};
use gpui::{prelude::*, *};
use gpui_base::input::{self, InputState};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId, number_input as n,
    numeric::{Direction, Draft},
    v1::*,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};

#[path = "number_input_repeat.rs"]
mod repeat;
#[path = "number_input_semantics.rs"]
mod semantics;

struct Route {
    window: WindowId,
    node: NodeId,
    handler: HandlerId,
    session: SharedSession,
    gate: focus::Shared,
    transport: Arc<Transport>,
}
impl Route {
    fn fault(&self) {
        if self.session.borrow_mut().overload(self.window) {
            self.transport.fault(self.window);
        }
    }
    fn emit(&self, events: impl IntoIterator<Item = n::Event>) {
        for event in events {
            let routed = {
                let session = self.session.borrow();
                let Some(tree) = session.tree(self.window) else {
                    return;
                };
                session.number_input_event(
                    self.window,
                    self.node,
                    self.handler,
                    tree.revision(),
                    event,
                )
            };
            if let Some(event) = routed
                && !self.transport.input(event)
            {
                self.fault();
                return;
            }
        }
    }
}
struct Owner {
    model: Model,
    route: Route,
    repeat: repeat::Repeat,
}
impl Owner {
    fn accessibility(
        &self,
    ) -> (
        Option<Arc<gpuio_protocol::accessibility::Config>>,
        Option<String>,
    ) {
        let session = self.route.session.borrow();
        let Some(tree) = session.tree(self.route.window) else {
            return (None, None);
        };
        (
            tree.get(self.route.node)
                .and_then(|node| node.accessibility.clone()),
            tree.tooltip_description(self.route.node).map(str::to_owned),
        )
    }
    fn current(&self) -> bool {
        let session = self.route.session.borrow();
        session
            .tree(self.route.window)
            .and_then(|tree| tree.get(self.route.node))
            .is_some_and(|node| {
                node.handler == Some(self.route.handler)
                    && node
                        .number_input
                        .as_ref()
                        .is_some_and(|mount| mount.config.as_ref() == self.model.config())
            })
            && session
                .number_input_event(
                    self.route.window,
                    self.route.node,
                    self.route.handler,
                    0,
                    n::Event::Observed(self.model.snapshot().clone()),
                )
                .is_some()
    }
    fn result(&self, outcome: Result<Outcome, crate::number_input_state::Fault>) -> n::Response {
        match outcome {
            Ok(outcome) => {
                self.route.emit(outcome.events);
                outcome.response
            }
            Err(_) => {
                self.route.fault();
                n::Response::Failed(n::Error::NativeFailure)
            }
        }
    }
}
fn perform(
    owner: &Weak<RefCell<Owner>>,
    entity: &WeakEntity<InputState>,
    command: &n::Command,
    source: n::Source,
    window: &mut Window,
    cx: &mut App,
) -> n::Response {
    let Some(owner) = owner.upgrade() else {
        return n::Response::Failed(n::Error::StaleInput);
    };
    {
        let owner = owner.borrow();
        if !owner.current() {
            return n::Response::Failed(n::Error::StaleInput);
        }
        if (source != n::Source::Programmatic || matches!(command, n::Command::Focus))
            && !owner.route.gate.borrow().allows(owner.route.node)
        {
            return n::Response::Failed(n::Error::FocusBlocked);
        }
    }
    if source != n::Source::Stepper && !matches!(command, n::Command::ReadSnapshot) {
        owner.borrow_mut().stop_repeat(window);
    }
    entity
        .update(cx, |state, cx| {
            let live = editor::snapshot(state, window, cx);
            let mut owner = owner.borrow_mut();
            let outcome = owner.model.execute(&live, command, source, |command| {
                editor::apply(state, command, true, window, cx)
            });
            owner.result(outcome)
        })
        .unwrap_or(n::Response::Failed(n::Error::StaleInput))
}

fn configure_input(
    state: &mut InputState,
    config: &n::Config,
    owner: Weak<RefCell<Owner>>,
    window: &mut Window,
    cx: &mut Context<InputState>,
) {
    if config.disabled && state.focus_handle(cx).is_focused(window) {
        window.blur(cx);
    }
    state.set_placeholder(config.placeholder.clone(), window, cx);
    state.set_readonly(config.read_only, cx);
    state.set_disabled(config.disabled, cx);
    state.set_submit_on_enter(true, cx);
    let config = config.clone();
    state.set_bridge_decorator(Rc::new(move |element, state, _, cx| {
        let mut element = element
            .role(Role::TextInput)
            .aria_label(config.label.clone())
            .aria_placeholder(config.placeholder.clone())
            .aria_value(state.value());
        let entity = cx.weak_entity();
        let route = owner.clone();
        element = element.capture_action(move |_: &input::Enter, window, cx| {
            if entity
                .read_with(cx, |state, _| state.bridge_composition().is_some())
                .unwrap_or(true)
            {
                return;
            }
            perform(
                &route,
                &entity,
                &n::Command::Commit,
                n::Source::Keyboard,
                window,
                cx,
            );
            cx.stop_propagation();
        });
        let entity = cx.weak_entity();
        let route = owner.clone();
        element = element.capture_action(move |_: &input::Escape, window, cx| {
            let composing = entity
                .read_with(cx, |state, _| state.bridge_composition().is_some())
                .unwrap_or(false);
            if composing {
                let _ = entity.update(cx, |state, cx| {
                    state.unmark_text(window, cx);
                    cx.notify();
                });
            } else {
                perform(
                    &route,
                    &entity,
                    &n::Command::Cancel,
                    n::Source::Keyboard,
                    window,
                    cx,
                );
            }
            cx.stop_propagation();
        });
        let entity = cx.weak_entity();
        let route = owner.clone();
        element = element.capture_action(move |_: &input::MoveUp, window, cx| {
            if entity
                .read_with(cx, |state, _| state.bridge_composition().is_some())
                .unwrap_or(true)
            {
                return;
            }
            perform(
                &route,
                &entity,
                &n::Command::Step(Direction::Increase),
                n::Source::Keyboard,
                window,
                cx,
            );
            cx.stop_propagation();
        });
        let entity = cx.weak_entity();
        let route = owner.clone();
        element = element.capture_action(move |_: &input::MoveDown, window, cx| {
            if entity
                .read_with(cx, |state, _| state.bridge_composition().is_some())
                .unwrap_or(true)
            {
                return;
            }
            perform(
                &route,
                &entity,
                &n::Command::Step(Direction::Decrease),
                n::Source::Keyboard,
                window,
                cx,
            );
            cx.stop_propagation();
        });
        if !config.disabled {
            let entity = cx.weak_entity();
            let route = owner.clone();
            element = element.on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
                perform(
                    &route,
                    &entity,
                    &n::Command::Focus,
                    n::Source::Accessibility,
                    window,
                    cx,
                );
            });
            if !config.read_only {
                let entity = cx.weak_entity();
                let route = owner.clone();
                element =
                    element.on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
                        if let Some(accesskit::ActionData::Value(text)) = data {
                            perform(
                                &route,
                                &entity,
                                &n::Command::ReplaceDraft {
                                    text: text.clone().into(),
                                    selection: n::SelectionPolicy::End,
                                    undo: n::UndoPolicy::Record,
                                    if_revision: None,
                                },
                                n::Source::Accessibility,
                                window,
                                cx,
                            );
                        }
                    });
            }
        }
        let (application, tooltip) = owner
            .upgrade()
            .map(|owner| {
                let owner = owner.borrow();
                owner.accessibility()
            })
            .unwrap_or_default();
        let metadata = semantics::metadata(
            &config,
            &state.value(),
            state.bridge_composition().is_some(),
            application.as_deref(),
            tooltip.as_deref(),
        );
        crate::semantics::State {
            element,
            metadata: Some(metadata),
            hidden: false,
            live: None,
            disabled: config.disabled,
            read_only: config.read_only,
            modal: false,
        }
        .into_any_element()
    }));
}

pub(super) struct Instance {
    state: Entity<InputState>,
    owner: Rc<RefCell<Owner>>,
    _subscriptions: Vec<Subscription>,
}
impl Instance {
    fn new(
        id: WindowId,
        node: &crate::tree::Node,
        session: SharedSession,
        gate: focus::Shared,
        transport: Arc<Transport>,
        window: &mut Window,
        cx: &mut Context<View>,
    ) -> Result<Self, n::Error> {
        let mount = node.number_input.as_ref().expect("validated numeric mount");
        let text = crate::number_input_state::initial_text(&mount.config, mount.initial)?;
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(text)
                .bridge_max_bytes(n::MAX_DRAFT_BYTES)
                .bridge_history_budget(EDITOR_HISTORY_BYTES)
        });
        if mount.config.auto_focus && !mount.config.disabled && gate.borrow().allows(node.id) {
            window.focus(&state.read(cx).focus_handle(cx), cx);
        }
        let model = Model::new(
            mount.config.clone(),
            mount.initial,
            &editor::snapshot(state.read(cx), window, cx),
        )?;
        let owner = Rc::new(RefCell::new(Owner {
            model,
            repeat: repeat::Repeat::default(),
            route: Route {
                window: id,
                node: node.id,
                handler: node.handler.expect("numeric handler"),
                session,
                gate,
                transport,
            },
        }));
        state.update(cx, |state, cx| {
            configure_input(state, &mount.config, Rc::downgrade(&owner), window, cx)
        });
        let weak = Rc::downgrade(&owner);
        let subscription = cx.observe_in(&state, window, move |_, state, window, cx| {
            if let Some(owner) = weak.upgrade() {
                let mut owner = owner.borrow_mut();
                if owner.current() {
                    let live = editor::snapshot(state.read(cx), window, cx);
                    match owner.model.observe(&live) {
                        Ok(Some(event)) => {
                            owner.stop_repeat(window);
                            owner.route.emit([event]);
                        }
                        Ok(None) => (),
                        Err(_) => owner.route.fault(),
                    }
                }
            }
            cx.notify();
        });
        owner
            .borrow()
            .route
            .emit([n::Event::Observed(owner.borrow().model.snapshot().clone())]);
        Ok(Self {
            state,
            owner,
            _subscriptions: vec![subscription],
        })
    }
    fn configure(
        &mut self,
        config: Arc<n::Config>,
        handler: HandlerId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let live = editor::snapshot(self.state.read(cx), window, cx);
        {
            let mut owner = self.owner.borrow_mut();
            let previous = owner.model.config();
            let policy_changed = previous.domain != config.domain
                || previous.step_controls != config.step_controls
                || previous.allow_empty != config.allow_empty
                || previous.disabled != config.disabled
                || previous.read_only != config.read_only;
            if policy_changed || owner.route.handler != handler {
                owner.stop_repeat(window);
            }
            owner.route.handler = handler;
            let outcome = owner.model.configure(config.clone(), &live);
            if matches!(owner.result(outcome), n::Response::Failed(_)) {
                owner.route.fault();
                return;
            }
        }
        self.state.update(cx, |state, cx| {
            configure_input(state, &config, Rc::downgrade(&self.owner), window, cx);
            cx.notify();
        });
    }
    pub(super) fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle(cx)
    }
    pub(super) fn is_composing(&self, cx: &App) -> bool {
        self.state.read(cx).bridge_composition().is_some()
    }
    pub(super) fn command(
        &self,
        command: &n::Command,
        window: &mut Window,
        cx: &mut App,
    ) -> n::Response {
        perform(
            &Rc::downgrade(&self.owner),
            &self.state.downgrade(),
            command,
            n::Source::Programmatic,
            window,
            cx,
        )
    }
    pub(super) fn command_available(&self, action: NativeCommand, cx: &App) -> bool {
        let owner = self.owner.borrow();
        let config = owner.model.config();
        !config.disabled
            && match action {
                NativeCommand::Copy => self.state.read(cx).is_copyable(),
                NativeCommand::Cut => !config.read_only && self.state.read(cx).is_copyable(),
                NativeCommand::Paste | NativeCommand::Undo | NativeCommand::Redo => {
                    !config.read_only
                }
                NativeCommand::SelectAll => !self.state.read(cx).value().is_empty(),
            }
    }
    pub(super) fn element(
        &self,
        mut base: Stateful<Div>,
        pointer: bool,
        cx: &App,
    ) -> Stateful<Div> {
        let owner = self.owner.borrow();
        let config = owner.model.config();
        let weak = Rc::downgrade(&self.owner);
        let state = self.state.read(cx);
        let text = state.value();
        let composing = state.bridge_composition().is_some();
        let (application, tooltip) = owner.accessibility();
        let metadata = semantics::metadata(
            config,
            &text,
            composing,
            application.as_deref(),
            tooltip.as_deref(),
        );
        base = base
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.))
            .role(Role::SpinButton)
            .aria_label(metadata.field.as_ref().unwrap().label.clone())
            .aria_min_numeric_value(config.domain.min())
            .aria_max_numeric_value(config.domain.max())
            .aria_numeric_value_step(config.domain.step());
        if !composing && let Draft::Valid(value) = Draft::parse(config.domain, &text) {
            base = base.aria_numeric_value(value);
        } else {
            base = base.aria_value(text);
        }
        if let Some(description) = semantics::description(&metadata) {
            base = base.aria_description(description);
        }
        if !config.disabled && !config.read_only {
            for (action, direction) in [
                (AccessibleAction::Increment, Direction::Increase),
                (AccessibleAction::Decrement, Direction::Decrease),
            ] {
                let route = weak.clone();
                let entity = self.state.downgrade();
                base = base.on_a11y_action(action, move |_, window, cx| {
                    perform(
                        &route,
                        &entity,
                        &n::Command::Step(direction),
                        n::Source::Accessibility,
                        window,
                        cx,
                    );
                });
            }
        }
        let button = |direction: Direction, label: String, glyph: &'static str| {
            let route = weak.clone();
            let entity = self.state.downgrade();
            let mut button = div()
                .id(if direction == Direction::Increase {
                    "number-increase"
                } else {
                    "number-decrease"
                })
                .role(Role::Button)
                .aria_label(label)
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(24.))
                .min_h(px(20.))
                .rounded(px(4.))
                .child(glyph);
            if config.step_controls == n::StepControls::Stacked {
                button = button
                    .flex_1()
                    .min_h(px(16.))
                    .text_size(px(12.))
                    .line_height(px(12.));
            }
            if !config.disabled && !config.read_only {
                let accessible = route.clone();
                let editor = entity.clone();
                button = button.on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    perform(
                        &accessible,
                        &editor,
                        &n::Command::Step(direction),
                        n::Source::Accessibility,
                        window,
                        cx,
                    );
                });
                if pointer {
                    button = button.cursor_pointer();
                }
            }
            repeat::Button {
                owner: route,
                entity,
                direction,
                enabled: pointer && !config.disabled && !config.read_only,
                element: crate::semantics::State {
                    element: button,
                    metadata: None,
                    hidden: false,
                    live: None,
                    disabled: config.disabled || config.read_only,
                    read_only: config.read_only,
                    modal: false,
                },
            }
        };
        let next = owner.route.gate.clone();
        let previous = owner.route.gate.clone();
        base = base
            .capture_action(move |_: &input::IndentInline, window, cx| {
                next.borrow().traverse(false, window, cx);
                cx.stop_propagation();
            })
            .capture_action(move |_: &input::OutdentInline, window, cx| {
                previous.borrow().traverse(true, window, cx);
                cx.stop_propagation();
            });
        let input = div().min_w(px(0.)).flex_1().child(self.state.clone());
        match config.step_controls {
            n::StepControls::Hidden => base.child(input),
            n::StepControls::Sides => base
                .child(button(
                    Direction::Decrease,
                    config.decrement_label.clone(),
                    "−",
                ))
                .child(input)
                .child(button(
                    Direction::Increase,
                    config.increment_label.clone(),
                    "+",
                )),
            n::StepControls::Stacked => base.child(input).child(
                div()
                    .flex()
                    .flex_col()
                    .self_stretch()
                    .gap(px(1.))
                    .child(button(
                        Direction::Increase,
                        config.increment_label.clone(),
                        "▴",
                    ))
                    .child(button(
                        Direction::Decrease,
                        config.decrement_label.clone(),
                        "▾",
                    )),
            ),
        }
    }
}
impl View {
    pub(super) fn sync_numbers(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.cancel_number_repeats(window);
                self.numbers.clear();
                return;
            };
            self.numbers.retain(|id, instance| {
                if tree.get(*id).is_some() {
                    true
                } else {
                    instance.owner.borrow_mut().stop_repeat(window);
                    false
                }
            });
            dirty
                .iter()
                .filter_map(|id| tree.get(*id))
                .filter(|node| node.number_input.is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        for node in nodes {
            if let Some(instance) = self.numbers.get_mut(&node.id) {
                instance.configure(
                    node.number_input.as_ref().unwrap().config.clone(),
                    node.handler.expect("validated numeric handler"),
                    window,
                    cx,
                );
            } else {
                match Instance::new(
                    self.id,
                    &node,
                    self.session.clone(),
                    self.focus.clone(),
                    self.transport.clone(),
                    window,
                    cx,
                ) {
                    Ok(instance) => {
                        self.numbers.insert(node.id, instance);
                    }
                    Err(_) => {
                        if self.session.borrow_mut().overload(self.id) {
                            self.transport.fault(self.id);
                        }
                    }
                }
            }
        }
        for instance in self.numbers.values() {
            instance.owner.borrow_mut().check_repeat(window);
        }
    }
    pub(super) fn cancel_number_repeats(&self, window: &mut Window) -> bool {
        let mut cancelled = false;
        for instance in self.numbers.values() {
            cancelled = instance.owner.borrow_mut().stop_repeat(window) || cancelled;
        }
        cancelled
    }
    pub(super) fn hide_unvisited_numbers(&self, window: &mut Window, cx: &mut App) {
        for (id, instance) in &self.numbers {
            if !self.visited.contains(id) && instance.owner.borrow().repeat.is_active() {
                let weak = Rc::downgrade(&instance.owner);
                window.defer(cx, move |window, _| {
                    if let Some(owner) = weak.upgrade() {
                        owner.borrow_mut().stop_repeat(window);
                    }
                });
            }
        }
    }
}

#[cfg(feature = "native-tests")]
#[path = "number_input_test.rs"]
pub(crate) mod test;
