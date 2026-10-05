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

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_escape_test.rs"]
mod escape_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_range_test.rs"]
mod range_tests;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_viewport_test.rs"]
mod viewport_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_search_test.rs"]
mod search_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_search_command_test.rs"]
mod search_command_tests;

#[path = "editor_search.rs"]
mod search;

#[path = "editor_viewport.rs"]
mod viewport;

#[path = "editor_layout.rs"]
mod layout;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_layout_test.rs"]
mod layout_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_format_test.rs"]
mod format_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_format_view_test.rs"]
mod format_view_tests;

struct Route {
    window: WindowId,
    node: NodeId,
    session: SharedSession,
    gate: super::focus::Shared,
    transport: Arc<Transport>,
    last: RefCell<Option<EditorSnapshot>>,
    last_search: RefCell<Option<gpuio_protocol::editor_search::Snapshot>>,
}
impl Route {
    fn publish_search<M: InputModeKind>(&self, state: &InputBaseState<M>) {
        use gpuio_protocol::editor_search::Mode;
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
            if node.kind != Kind::Textarea {
                return;
            }
            let mut last = self.last_search.borrow_mut();
            if !node.editor_searchable && last.is_none() {
                return;
            }
            let search = state.search_session();
            let can_replace = node.editor_searchable
                && search.open
                && state.is_replaceable()
                && state.bridge_composition().is_none();
            let closed = !node.editor_searchable || !search.open;
            // Blink/layout notifications must not clone a query or enqueue metadata.
            if last.as_ref().is_some_and(|old| {
                old.stamp.editor_revision == state.bridge_revision()
                    && old.stamp.search_revision == search.revision()
                    && old.activation_revision == state.search_activation_revision() as i64
                    && old.can_replace == can_replace
                    && (old.mode == Mode::Closed) == closed
            }) {
                return;
            }
            let mut value = search::capture(state);
            if !node.editor_searchable {
                value.mode = Mode::Closed;
                value.can_replace = false;
            }
            *last = Some(value.clone());
            Event::EditorSearchObserved(self.window, self.node, handler, tree.revision(), value)
        };
        if !self.transport.input(event) && self.session.borrow_mut().overload(self.window) {
            self.transport.fault(self.window);
        }
    }

    fn picker_owner<'a>(&self, tree: &'a crate::tree::Tree) -> Option<&'a crate::tree::Node> {
        let child = tree.get(self.node)?;
        let wrapper = tree.get(child.parent?)?;
        let owner = tree.get(wrapper.parent?)?;
        let presentation = owner.choice_picker.as_ref()?;
        owner
            .children
            .iter()
            .zip(&presentation.slots)
            .any(|(id, slot)| {
                *id == wrapper.id && matches!(slot, gpuio_protocol::choice_picker::Slot::Query)
            })
            .then_some(owner)
    }
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
            if let Some(owner) = self.picker_owner(tree) {
                if kind != EditorEventKind::Changed {
                    return;
                }
                let event = gpuio_protocol::choice_picker::Event::QueryChanged(
                    gpuio_protocol::choice_picker::Query {
                        node: self.node,
                        snapshot,
                    },
                );
                let Some(event) = session.choice_picker_event(
                    self.window,
                    owner.id,
                    owner.handler.expect("admitted picker observer"),
                    tree.revision(),
                    event,
                ) else {
                    return;
                };
                event
            } else {
                Event::EditorEvent(
                    self.window,
                    self.node,
                    handler,
                    tree.revision(),
                    kind,
                    snapshot,
                )
            }
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
            changed.publish_search(state.read(cx));
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
    privacy: EditorPrivacy,
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
    let hint = route
        .session
        .borrow()
        .tree(route.window)
        .and_then(|tree| tree.get(route.node))
        .and_then(|node| node.editor_content_hint);
    let format = route
        .session
        .borrow()
        .tree(route.window)
        .and_then(|tree| tree.get(route.node))
        .and_then(|node| node.editor_format.clone());
    let validation = route
        .session
        .borrow()
        .tree(route.window)
        .and_then(|tree| tree.get(route.node))
        .and_then(|node| node.editor_validation.clone());
    state.set_bridge_input_format(crate::input_format::editing_policy(format, validation));
    let config = config.clone();
    state.set_bridge_decorator(Rc::new(move |element, state, _, cx| {
        let mut element = element
            .role(if privacy != EditorPrivacy::Plain {
                gpui::Role::PasswordInput
            } else if combobox.is_some() {
                gpui::Role::EditableComboBox
            } else if state.is_single_line() {
                use gpuio_protocol::input_content_hint::Hint;
                match hint {
                    Some(Hint::TelephoneNumber) => gpui::Role::PhoneNumberInput,
                    Some(Hint::EmailAddress) => gpui::Role::EmailInput,
                    Some(Hint::Url) => gpui::Role::UrlInput,
                    Some(Hint::DateTime) => gpui::Role::DateTimeInput,
                    Some(Hint::Birthdate) => gpui::Role::DateInput,
                    _ => gpui::Role::TextInput,
                }
            } else {
                gpui::Role::MultilineTextInput
            })
            .aria_label(config.label.clone())
            .aria_placeholder(config.placeholder.clone());
        if let Some(description) = route
            .session
            .borrow()
            .tree(route.window)
            .and_then(|tree| tree.tooltip_description(route.node))
        {
            element = element.aria_description(description.to_owned());
        }
        let escape_route = route.clone();
        let composing_editor = cx.entity();
        element = element.capture_action(move |_: &gpui_base::input::Escape, window, cx| {
            if composing_editor.read(cx).bridge_composition().is_some() {
                composing_editor.update(cx, |state, cx| {
                    state.unmark_text(window, cx);
                    cx.notify();
                });
                cx.stop_propagation();
                return;
            }
            let allowed = escape_route
                .session
                .borrow()
                .tree(escape_route.window)
                .and_then(|tree| tree.get(escape_route.node))
                .is_some_and(|node| {
                    node.editor_clear_on_escape
                        && node
                            .editor
                            .as_ref()
                            .is_some_and(|config| !config.disabled && !config.read_only)
                });
            if !allowed || !escape_route.gate.borrow().allows(escape_route.node) {
                return;
            }
            let consumed = composing_editor.update(cx, |state, cx| {
                if !state.is_editable()
                    || state.text().len() == 0
                    || !state.focus_handle(cx).is_focused(window)
                {
                    return false;
                }
                // An enabled clear attempt belongs to this field even when the
                // current filter rejects empty text. Do not dismiss its parent.
                let command = EditorCommand::Replace(
                    String::new(),
                    EditorSelectionPolicy::Start,
                    EditorUndoPolicy::Record,
                    Some(state.bridge_revision()),
                );
                let _ = apply(state, &command, state.is_single_line(), window, cx);
                true
            });
            if consumed {
                cx.stop_propagation();
            }
        });
        if let Some(combo) = &combobox {
            element = element.aria_expanded(combo.borrow().popup.borrow().open);
        }
        // Revealing glyphs does not publish password values or text runs.
        if privacy == EditorPrivacy::Plain {
            let selection_route = route.clone();
            element = crate::editor_accessibility::attach(element, state, cx, move |_| {
                selection_route.gate.borrow().allows(selection_route.node)
                    && selection_route
                        .session
                        .borrow()
                        .tree(selection_route.window)
                        .and_then(|tree| tree.get(selection_route.node))
                        .filter(|node| node.editor_privacy == EditorPrivacy::Plain)
                        .and_then(|node| node.editor.as_ref())
                        .is_some_and(|config| !config.disabled)
            });
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
            identity: None,
            busy: route
                .session
                .borrow()
                .tree(route.window)
                .and_then(|tree| tree.get(route.node))
                .and_then(|node| node.editor_frame.as_ref())
                .is_some_and(|frame| frame.loading),
            hidden: false,
            metadata: route
                .session
                .borrow()
                .tree(route.window)
                .and_then(|tree| tree.get(route.node))
                .and_then(|node| node.accessibility.clone()),
            live: None,
            element,
            disabled: config.disabled,
            read_only: config.read_only,
            modal: false,
        }
        .into_any_element()
    }));
}

pub(super) fn apply<M: InputModeKind>(
    state: &mut InputBaseState<M>,
    command: &EditorCommand,
    single_line: bool,
    window: &mut Window,
    cx: &mut Context<InputBaseState<M>>,
) -> Result<EditorSnapshot, EditorError> {
    if state.bridge_composition().is_some()
        && !matches!(command, EditorCommand::Focus | EditorCommand::ReadSnapshot)
    {
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
            if !state.bridge_accepts_exact_text(text) {
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
        EditorCommand::Submit | EditorCommand::ReadSnapshot => (),
        EditorCommand::ReadContentHintStatus
        | EditorCommand::ReadViewport
        | EditorCommand::ScrollViewport(_)
        | EditorCommand::Search(_)
        | EditorCommand::ReadRangeBounds(..) => return Err(EditorError::NativeFailure),
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
    privacy: EditorPrivacy,
    content_hint: Option<gpuio_protocol::input_content_hint::Hint>,
    format: Option<Arc<gpuio_protocol::input_format::Config>>,
    validation: Option<Arc<crate::input_validation::Policy>>,
    layout: Option<gpuio_protocol::text_area_layout::Config>,
    searchable: bool,
    frame: Option<Arc<gpuio_protocol::editor_frame::Config>>,
    accessibility: Option<Arc<gpuio_protocol::accessibility::Config>>,
    route: Rc<Route>,
    _subscriptions: Vec<Subscription>,
    combobox: Option<Rc<RefCell<super::combobox::State>>>,
}
impl Instance {
    #[cfg(feature = "native-tests")]
    pub(super) fn input_bounds(&self, cx: &App) -> gpui::Bounds<gpui::Pixels> {
        match &self.state {
            State::Input(state) => state.read(cx).input_bounds(),
            State::Textarea(state) => state.read(cx).input_bounds(),
        }
    }

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
            last_search: RefCell::new(None),
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
                state.set_masked(
                    node.editor_privacy == EditorPrivacy::PasswordHidden,
                    window,
                    cx,
                );
                configure(
                    state,
                    &config,
                    node.editor_privacy,
                    combobox.clone(),
                    route.clone(),
                    window,
                    cx,
                )
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
                state.set_searchable(node.editor_searchable, cx);
                layout::configure(state, None, node.textarea_layout, window, cx);
                configure(
                    state,
                    &config,
                    node.editor_privacy,
                    combobox.clone(),
                    route.clone(),
                    window,
                    cx,
                )
            });
            (State::Textarea(entity), subscriptions)
        };
        let instance = Self {
            state,
            config,
            privacy: node.editor_privacy,
            content_hint: node.editor_content_hint,
            format: node.editor_format.clone(),
            validation: node.editor_validation.clone(),
            layout: node.textarea_layout,
            searchable: node.editor_searchable,
            frame: node.editor_frame.clone(),
            accessibility: node.accessibility.clone(),
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
        // Picker mount observations must follow the owner's visibility snapshot.
        let picker_query = instance
            .route
            .session
            .borrow()
            .tree(id)
            .is_some_and(|tree| instance.route.picker_owner(tree).is_some());
        if !picker_query {
            instance
                .route
                .publish(instance.snapshot(window, cx), EditorEventKind::Changed);
        }
        if let State::Textarea(entity) = &instance.state {
            instance.route.publish_search(entity.read(cx));
        }
        instance
    }
    pub(super) fn configure(
        &mut self,
        node: &crate::tree::Node,
        window: &mut Window,
        cx: &mut App,
    ) {
        let config = node.editor.as_deref().expect("admitted editor");
        let accessibility = &node.accessibility;
        let privacy = node.editor_privacy;
        if self.accessibility != *accessibility || self.frame != node.editor_frame {
            self.accessibility = accessibility.clone();
            self.frame = node.editor_frame.clone();
            match &self.state {
                State::Input(entity) => entity.update(cx, |_, cx| cx.notify()),
                State::Textarea(entity) => entity.update(cx, |_, cx| cx.notify()),
            }
        }
        if &self.config == config
            && self.privacy == privacy
            && self.content_hint == node.editor_content_hint
            && self.format == node.editor_format
            && self.validation == node.editor_validation
            && self.layout == node.textarea_layout
            && self.searchable == node.editor_searchable
        {
            return;
        }
        match &self.state {
            State::Input(entity) => entity.update(cx, |state, cx| {
                state.set_masked(privacy == EditorPrivacy::PasswordHidden, window, cx);
                configure(
                    state,
                    config,
                    privacy,
                    self.combobox.clone(),
                    self.route.clone(),
                    window,
                    cx,
                )
            }),
            State::Textarea(entity) => entity.update(cx, |state, cx| {
                state.set_searchable(node.editor_searchable, cx);
                layout::configure(state, self.layout, node.textarea_layout, window, cx);
                configure(
                    state,
                    config,
                    privacy,
                    self.combobox.clone(),
                    self.route.clone(),
                    window,
                    cx,
                );
                state.set_auto_grow(config.min_rows as usize, config.max_rows as usize, cx);
            }),
        }
        self.config = config.clone();
        self.privacy = privacy;
        self.content_hint = node.editor_content_hint;
        self.format = node.editor_format.clone();
        self.validation = node.editor_validation.clone();
        self.layout = node.textarea_layout;
        self.searchable = node.editor_searchable;
    }
    pub(super) fn publish_picker_snapshot(&self, window: &Window, cx: &App) {
        *self.route.last.borrow_mut() = None;
        self.route
            .publish(self.snapshot(window, cx), EditorEventKind::Changed);
    }
    pub(super) fn snapshot(&self, window: &Window, cx: &App) -> EditorSnapshot {
        match &self.state {
            State::Input(entity) => snapshot(entity.read(cx), window, cx),
            State::Textarea(entity) => snapshot(entity.read(cx), window, cx),
        }
    }
    #[cfg(all(test, feature = "native-image-tests"))]
    pub(super) fn mark_test_text(&self, text: &str, window: &mut Window, cx: &mut App) {
        match &self.state {
            State::Input(entity) => entity.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(
                    None,
                    text,
                    Some(0..text.encode_utf16().count()),
                    window,
                    cx,
                );
            }),
            State::Textarea(entity) => entity.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(
                    None,
                    text,
                    Some(0..text.encode_utf16().count()),
                    window,
                    cx,
                );
            }),
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
    pub(super) fn content_hint(&self) -> Option<gpuio_protocol::input_content_hint::Hint> {
        (!self.config.disabled && !self.config.read_only)
            .then_some(self.content_hint)
            .flatten()
    }
    pub(super) fn clear_revision(&self, cx: &App) -> Option<i64> {
        let State::Input(entity) = &self.state else {
            return None;
        };
        let state = entity.read(cx);
        (state.is_editable() && state.text().len() > 0 && state.bridge_composition().is_none())
            .then(|| state.bridge_revision())
    }
    pub(super) fn clear(&self, revision: i64, window: &mut Window, cx: &mut App) {
        if self.clear_revision(cx) != Some(revision) {
            return;
        }
        let State::Input(entity) = &self.state else {
            return;
        };
        entity.update(cx, |state, cx| {
            let command = EditorCommand::Replace(
                String::new(),
                EditorSelectionPolicy::Start,
                EditorUndoPolicy::Record,
                Some(revision),
            );
            if apply(state, &command, true, window, cx).is_ok() {
                window.focus(&state.focus_handle(cx), cx);
                cx.notify();
            }
        });
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
        if matches!(
            command,
            EditorCommand::ReadViewport
                | EditorCommand::ScrollViewport(_)
                | EditorCommand::ReadRangeBounds(..)
        ) {
            return match &self.state {
                State::Input(entity) => {
                    entity.update(cx, |state, cx| viewport::command(state, command, cx))
                }
                State::Textarea(entity) => {
                    entity.update(cx, |state, cx| viewport::command(state, command, cx))
                }
            };
        }
        if let EditorCommand::Search(command) = command {
            use gpuio_protocol::editor_search::Command;
            if !self.searchable {
                return EditorResult::Failed(EditorError::SearchUnavailable);
            }
            if !matches!(
                command,
                Command::Read | Command::Close | Command::CloseAndFocus(_)
            ) && !self.route.gate.borrow().allows(self.route.node)
            {
                return EditorResult::Failed(EditorError::FocusBlocked);
            }
            let focus_allowed =
                !self.config.disabled && self.route.gate.borrow().allows(self.route.node);
            let result = match &self.state {
                State::Textarea(entity) => entity.update(cx, |state, cx| {
                    search::command(
                        state,
                        command,
                        self.config.disabled,
                        focus_allowed,
                        window,
                        cx,
                    )
                }),
                State::Input(_) => EditorResult::Failed(EditorError::SearchUnavailable),
            };
            if let EditorResult::SearchReplaced(snapshot, _, count) = &result
                && *count > 0
            {
                self.route
                    .publish(snapshot.clone(), EditorEventKind::Changed);
            }
            if let State::Textarea(entity) = &self.state {
                self.route.publish_search(entity.read(cx));
            }
            return result;
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
