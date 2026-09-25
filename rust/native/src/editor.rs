//! Native-owned editing sessions. The retained tree stores configuration and
//! initial text; commands are the only bridge operation that replaces live text.
use super::SharedSession;
use crate::transport::Transport;
use gpui::{
    App, AppContext, Context, Entity, EntityInputHandler, FocusHandle, Focusable,
    InteractiveElement, IntoElement, StatefulInteractiveElement, Subscription, Window,
};
use gpui_base::input::{
    BridgeSubmission, InputBaseState, InputModeKind, InputState, TextareaState,
};
use gpuio_protocol::{NodeId, WindowId, v1::*};
use std::{cell::RefCell, rc::Rc, sync::Arc};

struct Route {
    window: WindowId,
    node: NodeId,
    session: SharedSession,
    gate: super::focus::Shared,
    transport: Arc<Transport>,
    last: RefCell<Option<EditorSnapshot>>,
}
impl Route {
    fn publish(&self, snapshot: EditorSnapshot, kind: EditorEventKind) {
        let event = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.window) else {
                return;
            };
            let Some(node) = tree.get(self.node) else {
                return;
            };
            let Some(handler) = node.handler else {
                return;
            };
            if kind == EditorEventKind::Changed {
                if self.last.borrow().as_ref() == Some(&snapshot) {
                    return;
                }
                *self.last.borrow_mut() = Some(snapshot.clone());
            }
            Event::EditorEvent(
                self.window,
                self.node,
                handler,
                tree.revision(),
                kind,
                snapshot,
            )
        };
        if !self.transport.input(event) && self.session.borrow_mut().overload(self.window) {
            self.transport.fault(self.window);
        }
    }
}

pub(super) fn snapshot<M: InputModeKind>(
    state: &InputBaseState<M>,
    window: &Window,
    cx: &App,
) -> EditorSnapshot {
    let (anchor, head) = state.bridge_selection();
    EditorSnapshot {
        revision: state.bridge_revision(),
        text: state.value().to_string(),
        selection: EditorSelection {
            anchor: anchor as i64,
            head: head as i64,
        },
        composition: state.bridge_composition().map(|range| EditorSelection {
            anchor: range.start as i64,
            head: range.end as i64,
        }),
        focused: state.focus_handle(cx).is_focused(window),
    }
}

fn subscribe<T: 'static, M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    route: Rc<Route>,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Vec<Subscription> {
    let changed = route.clone();
    vec![
        cx.observe_in(state, window, move |_, state, window, cx| {
            changed.publish(
                snapshot(state.read(cx), window, cx),
                EditorEventKind::Changed,
            );
            cx.notify();
        }),
        cx.subscribe_in(
            state,
            window,
            move |_, _, event: &BridgeSubmission, _, _| {
                route.publish(
                    EditorSnapshot {
                        revision: event.revision,
                        text: event.text.to_string(),
                        selection: EditorSelection {
                            anchor: event.selection.0 as i64,
                            head: event.selection.1 as i64,
                        },
                        composition: None,
                        focused: event.focused,
                    },
                    EditorEventKind::Submitted,
                );
            },
        ),
    ]
}

fn configure<M: InputModeKind>(
    state: &mut InputBaseState<M>,
    config: &EditorConfig,
    combobox: Option<Rc<RefCell<super::combobox::State>>>,
    route: Rc<Route>,
    window: &mut Window,
    cx: &mut Context<InputBaseState<M>>,
) {
    if config.disabled && state.focus_handle(cx).is_focused(window) {
        window.blur(cx);
    }
    state.set_placeholder(config.placeholder.clone(), window, cx);
    state.set_readonly(config.read_only, cx);
    state.set_disabled(config.disabled, cx);
    state.set_submit_on_enter(config.submit_on_enter, cx);
    let config = config.clone();
    state.set_bridge_decorator(Rc::new(move |element, state, _, cx| {
        let mut element = element
            .role(if combobox.is_some() {
                gpui::Role::EditableComboBox
            } else if state.is_single_line() {
                gpui::Role::TextInput
            } else {
                gpui::Role::MultilineTextInput
            })
            .aria_label(config.label.clone())
            .aria_placeholder(config.placeholder.clone())
            .aria_value(state.value());
        if let Some(description) = route
            .session
            .borrow()
            .tree(route.window)
            .and_then(|tree| tree.tooltip_description(route.node))
        {
            element = element.aria_description(description.to_owned());
        }
        let composing_editor = cx.entity();
        element = element.capture_action(move |_: &gpui_base::input::Escape, window, cx| {
            if composing_editor.read(cx).bridge_composition().is_some() {
                composing_editor.update(cx, |state, cx| {
                    state.unmark_text(window, cx);
                    cx.notify();
                });
                cx.stop_propagation();
            }
        });
        if let Some(combo) = &combobox {
            element = element.aria_expanded(combo.borrow().popup.borrow().open);
        }
        if !config.disabled {
            let focus = state.focus_handle(cx);
            let gate = route.gate.clone();
            let node = route.node;
            element =
                element.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                    if gate.borrow().allows(node) {
                        window.focus(&focus, cx);
                    }
                });
            if !config.read_only {
                let entity = cx.weak_entity();
                let gate = route.gate.clone();
                let node = route.node;
                element = element.on_a11y_action(
                    gpui::AccessibleAction::SetValue,
                    move |data, window, cx| {
                        if !gate.borrow().allows(node) {
                            return;
                        }
                        if let Some(gpui::accesskit::ActionData::Value(value)) = data {
                            let _ = entity.update(cx, |state, cx| {
                                let command = EditorCommand::Replace(
                                    value.clone().into(),
                                    EditorSelectionPolicy::End,
                                    EditorUndoPolicy::Record,
                                    None,
                                );
                                let _ = apply(state, &command, state.is_single_line(), window, cx);
                            });
                        }
                    },
                );
            }
        }
        crate::semantics::State {
            live: None,
            element,
            disabled: config.disabled,
            read_only: config.read_only,
            modal: false,
        }
        .into_any_element()
    }));
}

fn apply<M: InputModeKind>(
    state: &mut InputBaseState<M>,
    command: &EditorCommand,
    single_line: bool,
    window: &mut Window,
    cx: &mut Context<InputBaseState<M>>,
) -> Result<EditorSnapshot, EditorError> {
    if state.bridge_composition().is_some() && !matches!(command, EditorCommand::Focus) {
        return Err(EditorError::Composing);
    }
    let valid_selection = |selection: &EditorSelection, text: &str| {
        usize::try_from(selection.anchor)
            .ok()
            .filter(|offset| text.is_char_boundary(*offset))
            .zip(
                usize::try_from(selection.head)
                    .ok()
                    .filter(|offset| text.is_char_boundary(*offset)),
            )
    };
    match command {
        EditorCommand::Replace(text, selection, undo, expected) => {
            if expected.is_some_and(|expected| expected != state.bridge_revision()) {
                return Err(EditorError::StaleRevision);
            }
            if text.len() > MAX_TEXT_BYTES || state.bridge_revision() == i64::MAX {
                return Err(EditorError::LimitExceeded);
            }
            if text.contains('\0') || (single_line && text.contains(['\n', '\r'])) {
                return Err(EditorError::InvalidText);
            }
            let (anchor, head) = match selection {
                EditorSelectionPolicy::Start => (0, 0),
                EditorSelectionPolicy::End => (text.len(), text.len()),
                EditorSelectionPolicy::Preserve => {
                    let (anchor, head) = state.bridge_selection();
                    let clip = |offset: usize| {
                        let mut offset = offset.min(text.len());
                        while !text.is_char_boundary(offset) {
                            offset -= 1;
                        }
                        offset
                    };
                    (clip(anchor), clip(head))
                }
                EditorSelectionPolicy::Select(selection) => {
                    valid_selection(selection, text).ok_or(EditorError::InvalidSelection)?
                }
            };
            state.bridge_replace_all(
                text.clone().into(),
                (anchor, head),
                matches!(undo, EditorUndoPolicy::Record),
                window,
                cx,
            );
        }
        EditorCommand::Select(selection) => {
            let text = state.value();
            let (anchor, head) =
                valid_selection(selection, &text).ok_or(EditorError::InvalidSelection)?;
            assert!(state.bridge_select(anchor, head, cx));
        }
        EditorCommand::Focus => state.focus(window, cx),
        EditorCommand::Undo => state.bridge_undo(window, cx),
        EditorCommand::Redo => state.bridge_redo(window, cx),
        EditorCommand::Submit => (),
    }
    Ok(snapshot(state, window, cx))
}

enum State {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
}
pub(super) struct Instance {
    state: State,
    config: EditorConfig,
    route: Rc<Route>,
    _subscriptions: Vec<Subscription>,
    combobox: Option<Rc<RefCell<super::combobox::State>>>,
}
impl Instance {
    #[cfg(feature = "native-tests")]
    pub(super) fn liveness_probe(&self) -> Box<dyn Fn() -> bool> {
        match &self.state {
            State::Input(state) => {
                let weak = state.downgrade();
                Box::new(move || weak.upgrade().is_some())
            }
            State::Textarea(state) => {
                let weak = state.downgrade();
                Box::new(move || weak.upgrade().is_some())
            }
        }
    }
    pub(super) fn new<T: 'static>(
        id: WindowId,
        node: &crate::tree::Node,
        session: SharedSession,
        gate: super::focus::Shared,
        transport: Arc<Transport>,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> Self {
        let config = node
            .editor
            .as_ref()
            .expect("validated editor")
            .as_ref()
            .clone();
        let route = Rc::new(Route {
            window: id,
            node: node.id,
            session,
            gate,
            transport,
            last: RefCell::new(None),
        });
        let combobox = (node.kind == Kind::Combobox)
            .then(|| Rc::new(RefCell::new(super::combobox::State::default())));
        let (state, subscriptions) = if matches!(node.kind, Kind::Input | Kind::Combobox) {
            let entity = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(node.text.to_string())
                    .bridge_max_bytes(MAX_TEXT_BYTES)
                    .bridge_history_budget(EDITOR_HISTORY_BYTES)
            });
            let subscriptions = subscribe(&entity, route.clone(), window, cx);
            entity.update(cx, |state, cx| {
                configure(state, &config, combobox.clone(), route.clone(), window, cx)
            });
            (State::Input(entity), subscriptions)
        } else {
            let entity = cx.new(|cx| {
                TextareaState::new(window, cx)
                    .default_value(node.text.to_string())
                    .bridge_max_bytes(MAX_TEXT_BYTES)
                    .bridge_history_budget(EDITOR_HISTORY_BYTES)
                    .auto_grow(config.min_rows as usize, config.max_rows as usize)
            });
            let subscriptions = subscribe(&entity, route.clone(), window, cx);
            entity.update(cx, |state, cx| {
                configure(state, &config, combobox.clone(), route.clone(), window, cx)
            });
            (State::Textarea(entity), subscriptions)
        };
        let instance = Self {
            state,
            config,
            route,
            _subscriptions: subscriptions,
            combobox,
        };
        if instance.config.auto_focus
            && !instance.config.disabled
            && instance.route.gate.borrow().allows(node.id)
        {
            window.focus(&instance.focus_handle(cx), cx);
        }
        instance
            .route
            .publish(instance.snapshot(window, cx), EditorEventKind::Changed);
        instance
    }
    pub(super) fn configure(&mut self, config: &EditorConfig, window: &mut Window, cx: &mut App) {
        if &self.config == config {
            return;
        }
        match &self.state {
            State::Input(entity) => entity.update(cx, |state, cx| {
                configure(
                    state,
                    config,
                    self.combobox.clone(),
                    self.route.clone(),
                    window,
                    cx,
                )
            }),
            State::Textarea(entity) => entity.update(cx, |state, cx| {
                configure(
                    state,
                    config,
                    self.combobox.clone(),
                    self.route.clone(),
                    window,
                    cx,
                );
                state.set_auto_grow(config.min_rows as usize, config.max_rows as usize, cx);
            }),
        }
        self.config = config.clone();
    }
    pub(super) fn snapshot(&self, window: &Window, cx: &App) -> EditorSnapshot {
        match &self.state {
            State::Input(entity) => snapshot(entity.read(cx), window, cx),
            State::Textarea(entity) => snapshot(entity.read(cx), window, cx),
        }
    }
    pub(super) fn is_composing(&self, cx: &App) -> bool {
        match &self.state {
            State::Input(entity) => entity.read(cx).bridge_composition().is_some(),
            State::Textarea(entity) => entity.read(cx).bridge_composition().is_some(),
        }
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn scroll_offset(&self, cx: &App) -> gpui::Point<gpui::Pixels> {
        match &self.state {
            State::Input(entity) => entity.read(cx).scroll_offset(),
            State::Textarea(entity) => entity.read(cx).scroll_offset(),
        }
    }
    pub(super) fn command_available(&self, action: NativeCommand, cx: &App) -> bool {
        if self.config.disabled {
            return false;
        }
        fn available<M: InputModeKind>(
            state: &InputBaseState<M>,
            action: NativeCommand,
            read_only: bool,
        ) -> bool {
            match action {
                NativeCommand::Copy => state.is_copyable(),
                NativeCommand::Cut => state.is_copyable() && !read_only,
                NativeCommand::Paste | NativeCommand::Undo | NativeCommand::Redo => !read_only,
                NativeCommand::SelectAll => state.text().len() > 0,
            }
        }
        match &self.state {
            State::Input(entity) => available(entity.read(cx), action, self.config.read_only),
            State::Textarea(entity) => available(entity.read(cx), action, self.config.read_only),
        }
    }
    pub(super) fn focus_handle(&self, cx: &App) -> FocusHandle {
        match &self.state {
            State::Input(entity) => entity.read(cx).focus_handle(cx),
            State::Textarea(entity) => entity.read(cx).focus_handle(cx),
        }
    }
    pub(super) fn combobox(
        &self,
    ) -> Option<(Entity<InputState>, Rc<RefCell<super::combobox::State>>)> {
        match (&self.state, &self.combobox) {
            (State::Input(entity), Some(state)) => Some((entity.clone(), state.clone())),
            _ => None,
        }
    }
    pub(super) fn element(&self) -> gpui::AnyElement {
        match &self.state {
            State::Input(entity) => entity.clone().into_any_element(),
            State::Textarea(entity) => entity.clone().into_any_element(),
        }
    }
    pub(super) fn command(
        &mut self,
        command: &EditorCommand,
        window: &mut Window,
        cx: &mut App,
    ) -> EditorResult {
        // Check retained identity even if an old native entity survived until this turn.
        if !self
            .route
            .session
            .borrow()
            .tree(self.route.window)
            .is_some_and(|tree| tree.get(self.route.node).is_some())
        {
            return EditorResult::Failed(EditorError::StaleEditor);
        }
        if matches!(command, EditorCommand::Focus | EditorCommand::Submit)
            && (!self.route.gate.borrow().allows(self.route.node)
                || (matches!(command, EditorCommand::Submit) && self.config.disabled))
        {
            return EditorResult::Failed(EditorError::FocusBlocked);
        }
        let result = match &self.state {
            State::Input(entity) => {
                entity.update(cx, |state, cx| apply(state, command, true, window, cx))
            }
            State::Textarea(entity) => {
                entity.update(cx, |state, cx| apply(state, command, false, window, cx))
            }
        };
        match result {
            Ok(snapshot) => {
                if matches!(command, EditorCommand::Replace(..))
                    && let Some(combo) = &self.combobox
                {
                    combo.borrow_mut().replaced(&snapshot.text);
                }
                self.route
                    .publish(snapshot.clone(), EditorEventKind::Changed);
                EditorResult::Applied(snapshot)
            }
            Err(error) => EditorResult::Failed(error),
        }
    }
}
