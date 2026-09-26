//! One platform text-input entity for a retained segmented code field.
use super::{SharedSession, View, focus};
use crate::{
    otp_edit as edit,
    otp_input_state::{Access, Action, NativeError, State},
    transport::Transport,
};
use gpui::{prelude::*, *};
use gpui_base::input;
use gpuio_protocol::{HandlerId, NodeId, WindowId, otp_input as o, v1::*};
use std::{ops::Range, sync::Arc};

#[path = "otp_input_paint.rs"]
mod paint;

actions!(gpuio_otp, [SelectLeft, SelectRight]);
struct Bindings;
impl Global for Bindings {}

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
    fn current(&self, config: &o::Config) -> bool {
        let session = self.session.borrow();
        session.accepts_input(self.window)
            && session
                .tree(self.window)
                .and_then(|tree| tree.get(self.node))
                .is_some_and(|node| {
                    node.handler == Some(self.handler)
                        && node
                            .otp_input
                            .as_ref()
                            .is_some_and(|mount| mount.config.as_ref() == config)
                })
    }
    fn emit(&self, events: Vec<o::Event>) -> bool {
        if events.is_empty() {
            return true;
        }
        let routed = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.window) else {
                return false;
            };
            events
                .into_iter()
                .map(|event| {
                    session.otp_input_event(
                        self.window,
                        self.node,
                        self.handler,
                        tree.revision(),
                        event,
                    )
                })
                .collect::<Option<Vec<_>>>()
        };
        let Some(mut events) = routed else {
            return false;
        };
        let success = match events.len() {
            1 => self.transport.input(events.pop().unwrap()),
            2 => self
                .transport
                .otp_completion(events.try_into().expect("two routed events")),
            _ => false,
        };
        if !success {
            self.fault();
        }
        success
    }
}

pub(super) struct Input {
    model: State,
    route: Route,
    focus: FocusHandle,
    pointer: bool,
    dragging: bool,
    hitbox: Option<HitboxId>,
    capture: Option<HitboxId>,
    autofocus: bool,
    layout: Option<paint::Layout>,
    metadata: Option<Arc<gpuio_protocol::accessibility::Config>>,
    _subscriptions: Vec<Subscription>,
}
impl Input {
    fn access(&self) -> Access {
        if self.route.current(self.model.config())
            && self.route.gate.borrow().allows(self.route.node)
        {
            Access::Allowed
        } else {
            Access::Blocked
        }
    }
    fn publish(&mut self, result: Result<Vec<o::Event>, o::Error>, cx: &mut Context<Self>) {
        match result {
            Ok(events) => {
                if !events.is_empty() {
                    if self
                        .layout
                        .as_ref()
                        .is_some_and(|layout| !layout.matches(&self.model))
                    {
                        self.layout = None;
                    }
                    self.route.emit(events);
                    cx.notify();
                }
            }
            Err(o::Error::LimitExceeded | o::Error::NativeFailure) => self.route.fault(),
            Err(_) => (),
        }
    }
    fn act(&mut self, action: Action<'_>, cx: &mut Context<Self>) {
        if !self.route.current(self.model.config()) {
            return;
        }
        match self.model.native(action, self.access()) {
            Ok(events) => self.publish(Ok(events), cx),
            Err(NativeError::Denied(error)) => self.publish(Err(error), cx),
            // Invalid platform ranges/preedit admission preserve the editing session.
            Err(NativeError::Input(_)) => (),
        }
    }
    fn on_focus(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let result = self.model.observe_focus(true);
        self.publish(result, cx);
    }
    fn stop_drag(&mut self, window: &mut Window) {
        if self
            .capture
            .take()
            .is_some_and(|capture| window.captured_hitbox() == Some(capture))
        {
            window.release_pointer();
        }
        self.dragging = false;
    }
    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_drag(window);
        let result = self.model.cancel_for_lifecycle();
        self.publish(result, cx);
        let result = self.model.observe_focus(false);
        self.publish(result, cx);
    }
    fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_drag(window);
        self.layout = None;
        let result = self.model.cancel_for_lifecycle();
        self.publish(result, cx);
        if self.focus.is_focused(window) {
            window.blur(cx);
        }
    }
    fn move_caret(&mut self, movement: edit::Movement, extend: bool, cx: &mut Context<Self>) {
        if self.model.editor().is_composing() {
            cx.propagate();
            return;
        }
        self.act(Action::Move { movement, extend }, cx);
    }
    fn delete(&mut self, direction: edit::Delete, cx: &mut Context<Self>) {
        if self.model.editor().is_composing() {
            cx.propagate();
            return;
        }
        self.act(Action::Delete(direction), cx);
    }
    fn select_all(&mut self, cx: &mut Context<Self>) {
        if self.model.editor().is_composing() {
            cx.propagate();
            return;
        }
        self.act(
            Action::Select(o::Selection {
                anchor: 0,
                head: self.model.editor().value().len() as i64,
            }),
            cx,
        );
    }
    fn copy(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.model.copy_selection(self.access()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
        }
    }
    fn cut(&mut self, cx: &mut Context<Self>) {
        let result = self.model.cut(self.access(), |text| {
            cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
            Ok(())
        });
        self.publish(result, cx);
    }
    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.act(Action::Paste(&text), cx);
        }
    }
    fn point_selection(&mut self, position: Point<Pixels>, extend: bool, cx: &mut Context<Self>) {
        if self.model.editor().is_composing() {
            return;
        }
        let Some(layout) = &self.layout else {
            return;
        };
        let head = layout.byte_for_point(position);
        let anchor = if extend {
            self.model.editor().selection().anchor
        } else {
            head
        };
        self.act(
            Action::Select(o::Selection {
                anchor: anchor as i64,
                head: head as i64,
            }),
            cx,
        );
    }
    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.pointer || self.model.config().disabled || self.access() == Access::Blocked {
            return;
        }
        window.focus(&self.focus, cx);
        if self.model.editor().is_composing() || window.captured_hitbox().is_some() {
            return;
        }
        let Some(hitbox) = self.hitbox else {
            return;
        };
        window.capture_pointer(hitbox);
        self.capture = Some(hitbox);
        self.dragging = true;
        if event.click_count > 1 {
            self.select_all(cx);
        } else {
            self.point_selection(event.position, event.modifiers.shift, cx);
        }
        window.prevent_default();
        cx.stop_propagation();
    }
    fn mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.pointer
            || self.access() == Access::Blocked
            || self.model.config().disabled
            || self.capture != window.captured_hitbox()
        {
            self.stop_drag(window);
            return;
        }
        if self.dragging && event.pressed_button == Some(MouseButton::Left) {
            self.point_selection(event.position, true, cx);
        } else {
            self.stop_drag(window);
        }
    }
    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.dragging
            && self.pointer
            && self.access() == Access::Allowed
            && !self.model.config().disabled
        {
            self.point_selection(event.position, true, cx);
        }
        self.stop_drag(window);
    }
}
impl Focusable for Input {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl EntityInputHandler for Input {
    fn paste(&mut self, item: ClipboardItem, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = item.text() {
            self.act(Action::Paste(&text), cx);
        }
    }
    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Ok(range) = edit::utf16_range(self.model.editor().draft(), range) {
            self.act(
                Action::Select(o::Selection {
                    anchor: range.start as i64,
                    head: range.end as i64,
                }),
                cx,
            );
        }
    }
    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.model.editor().draft().encode_utf16().count())
    }
    fn accepts_text_input(&self, _: &mut Window, _: &mut Context<Self>) -> bool {
        self.access() == Access::Allowed
            && !self.model.config().disabled
            && !self.model.config().read_only
    }

    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let bytes = edit::utf16_range(self.model.editor().draft(), range.clone()).ok()?;
        *actual = Some(range);
        Some(self.model.editor().draft()[bytes].to_owned())
    }
    fn selected_text_range(
        &mut self,
        ignore_disabled: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if !ignore_disabled && (self.model.config().disabled || self.access() == Access::Blocked) {
            return None;
        }
        let editor = self.model.editor();
        let selection = editor.selection();
        let range = selection.range();
        Some(UTF16Selection {
            range: edit::byte_to_utf16(editor.draft(), range.start).ok()?
                ..edit::byte_to_utf16(editor.draft(), range.end).ok()?,
            reversed: selection.anchor > selection.head,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let editor = self.model.editor();
        let range = editor.marked()?;
        Some(
            edit::byte_to_utf16(editor.draft(), range.start).ok()?
                ..edit::byte_to_utf16(editor.draft(), range.end).ok()?,
        )
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let result = self.model.unmark(self.access());
        self.publish(result, cx);
    }
    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.act(Action::Commit { range_utf16, text }, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.act(
            Action::Mark {
                range_utf16,
                text,
                selected_utf16,
            },
            cx,
        );
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = edit::utf16_range(self.model.editor().draft(), range).ok()?;
        self.layout
            .as_ref()
            .map(|layout| layout.range_bounds(range))
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        if !layout.bounds.contains(&position) {
            return None;
        }
        edit::byte_to_utf16(self.model.editor().draft(), layout.byte_for_point(position)).ok()
    }
}

impl Render for Input {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = self.model.config();
        let value = if config.masked {
            "•".repeat(self.model.editor().draft().chars().count())
        } else {
            self.model.editor().draft().to_owned()
        };
        let mut element =
            div()
                .id("otp-field")
                .w_full()
                .h_full()
                .min_w(px(0.))
                .overflow_hidden()
                .key_context("Input GpuioOtp")
                .track_focus(
                    &self
                        .focus
                        .clone()
                        .tab_stop(!config.disabled && self.access() == Access::Allowed),
                )
                .cursor(CursorStyle::IBeam)
                .role(if config.masked {
                    Role::PasswordInput
                } else {
                    Role::TextInput
                })
                .aria_label(config.label.clone())
                .aria_value(value)
                .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
                .on_action(cx.listener(|s, _: &input::Backspace, _, cx| {
                    s.delete(edit::Delete::Backward, cx)
                }))
                .on_action(
                    cx.listener(|s, _: &input::Delete, _, cx| s.delete(edit::Delete::Forward, cx)),
                )
                .on_action(cx.listener(|s, _: &input::MoveLeft, _, cx| {
                    s.move_caret(edit::Movement::Left, false, cx)
                }))
                .on_action(cx.listener(|s, _: &input::MoveRight, _, cx| {
                    s.move_caret(edit::Movement::Right, false, cx)
                }))
                .on_action(cx.listener(|s, _: &SelectLeft, _, cx| {
                    s.move_caret(edit::Movement::Left, true, cx)
                }))
                .on_action(cx.listener(|s, _: &SelectRight, _, cx| {
                    s.move_caret(edit::Movement::Right, true, cx)
                }))
                .on_action(cx.listener(|s, _: &input::MoveHome, _, cx| {
                    s.move_caret(edit::Movement::Start, false, cx)
                }))
                .on_action(cx.listener(|s, _: &input::MoveEnd, _, cx| {
                    s.move_caret(edit::Movement::End, false, cx)
                }))
                .on_action(cx.listener(|s, _: &input::MoveToStartOfLine, _, cx| {
                    s.move_caret(edit::Movement::Start, false, cx)
                }))
                .on_action(cx.listener(|s, _: &input::MoveToEndOfLine, _, cx| {
                    s.move_caret(edit::Movement::End, false, cx)
                }))
                .on_action(cx.listener(|s, _: &input::SelectToStartOfLine, _, cx| {
                    s.move_caret(edit::Movement::Start, true, cx)
                }))
                .on_action(cx.listener(|s, _: &input::SelectToEndOfLine, _, cx| {
                    s.move_caret(edit::Movement::End, true, cx)
                }))
                .on_action(cx.listener(|s, _: &input::SelectAll, _, cx| s.select_all(cx)))
                .on_action(cx.listener(|s, _: &input::Copy, _, cx| s.copy(cx)))
                .on_action(cx.listener(|s, _: &input::Cut, _, cx| s.cut(cx)))
                .on_action(cx.listener(|s, _: &input::Paste, _, cx| s.paste(cx)))
                .on_action(cx.listener(|s, _: &input::Undo, _, cx| s.act(Action::Undo, cx)))
                .on_action(cx.listener(|s, _: &input::Redo, _, cx| s.act(Action::Redo, cx)))
                .on_action(cx.listener(|s, _: &input::Escape, _, cx| {
                    if s.model.editor().is_composing() {
                        s.act(Action::CancelComposition, cx);
                    } else {
                        cx.propagate();
                    }
                }))
                .on_action(cx.listener(|s, _: &input::IndentInline, window, cx| {
                    s.route.gate.borrow().traverse(false, window, cx);
                }))
                .on_action(cx.listener(|s, _: &input::OutdentInline, window, cx| {
                    s.route.gate.borrow().traverse(true, window, cx);
                }))
                .child(paint::Field { input: cx.entity() });
        if !config.disabled && !config.read_only {
            let entity = cx.weak_entity();
            element = element.on_a11y_action(AccessibleAction::SetValue, move |data, _, cx| {
                if let Some(accesskit::ActionData::Value(value)) = data {
                    let _ = entity.update(cx, |s, cx| {
                        if s.model.editor().is_composing() {
                            return;
                        }
                        s.act(
                            Action::Commit {
                                range_utf16: Some(0..s.model.editor().value().len()),
                                text: value,
                            },
                            cx,
                        );
                    });
                }
            });
        }
        crate::semantics::State {
            element,
            metadata: self.metadata.clone(),
            hidden: false,
            disabled: config.disabled,
            read_only: config.read_only,
            modal: false,
            live: None,
        }
    }
}

pub(super) struct Instance {
    state: Entity<Input>,
}
impl Instance {
    fn new(
        view: &View,
        node: &crate::tree::Node,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Self, o::Error> {
        if !cx.has_global::<Bindings>() {
            cx.bind_keys([
                KeyBinding::new("shift-left", SelectLeft, Some("GpuioOtp")),
                KeyBinding::new("shift-right", SelectRight, Some("GpuioOtp")),
            ]);
            cx.set_global(Bindings);
        }
        let mount = node.otp_input.as_ref().expect("validated OTP");
        let model = State::new(mount.config.clone(), &mount.initial)?;
        let route = Route {
            window: view.id,
            node: node.id,
            handler: node.handler.expect("OTP handler"),
            session: view.session.clone(),
            gate: view.focus.clone(),
            transport: view.transport.clone(),
        };
        route.emit(vec![o::Event::Observed(model.snapshot())]);
        let state = cx.new(|cx| {
            let focus = cx.focus_handle().tab_stop(true);
            let subscriptions = vec![
                cx.on_focus(&focus, window, Input::on_focus),
                cx.on_blur(&focus, window, Input::on_blur),
                cx.observe_window_activation(window, |state, window, cx| {
                    if !window.is_window_active() {
                        state.stop_drag(window);
                        let result = state.model.cancel_for_lifecycle();
                        state.publish(result, cx);
                    }
                }),
            ];
            Input {
                model,
                route,
                focus,
                pointer: true,
                dragging: false,
                hitbox: None,
                capture: None,
                autofocus: mount.config.auto_focus,
                layout: None,
                metadata: node.accessibility.clone(),
                _subscriptions: subscriptions,
            }
        });
        Ok(Self { state })
    }
    pub(super) fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus.clone()
    }
    pub(super) fn is_composing(&self, cx: &App) -> bool {
        self.state.read(cx).model.editor().is_composing()
    }
    pub(super) fn command(
        &self,
        command: &o::Command,
        window: &mut Window,
        cx: &mut App,
    ) -> o::Response {
        self.state.update(cx, |state, cx| {
            if !state
                .route
                .session
                .borrow()
                .accepts_input(state.route.window)
            {
                return o::Response::Failed(o::Error::NativeFailure);
            }
            if !state.route.current(state.model.config()) {
                return o::Response::Failed(o::Error::StaleInput);
            }
            let focus = state.focus.clone();
            let gate = state.route.gate.clone();
            let node = state.route.node;
            let outcome = state.model.execute(command, || {
                if !gate.borrow().allows(node) {
                    return Err(o::Error::FocusBlocked);
                }
                window.focus(&focus, cx);
                if focus.is_focused(window) {
                    Ok(())
                } else {
                    Err(o::Error::NativeFailure)
                }
            });
            if matches!(
                outcome.response,
                o::Response::Failed(o::Error::LimitExceeded)
            ) {
                state.route.fault();
            }
            let changed = !outcome.events.is_empty();
            if state
                .layout
                .as_ref()
                .is_some_and(|layout| !layout.matches(&state.model))
            {
                state.layout = None;
            }
            if !state.route.emit(outcome.events) {
                return o::Response::Failed(o::Error::NativeFailure);
            }
            if changed {
                cx.notify();
            }
            outcome.response
        })
    }
    pub(super) fn command_available(&self, action: NativeCommand, cx: &App) -> bool {
        let state = self.state.read(cx);
        let model = &state.model;
        if state.access() == Access::Blocked
            || model.config().disabled
            || model.editor().is_composing()
        {
            return false;
        }
        match action {
            NativeCommand::Copy => model.copy_selection(Access::Allowed).is_some(),
            NativeCommand::Cut => {
                !model.config().read_only && model.copy_selection(Access::Allowed).is_some()
            }
            NativeCommand::Paste => !model.config().read_only,
            NativeCommand::Undo => !model.config().read_only && model.editor().can_undo(),
            NativeCommand::Redo => !model.config().read_only && model.editor().can_redo(),
            NativeCommand::SelectAll => !model.editor().value().is_empty(),
        }
    }
    pub(super) fn element(
        &self,
        base: Stateful<Div>,
        pointer: bool,
        cx: &mut App,
    ) -> Stateful<Div> {
        self.state.update(cx, |state, _| state.pointer = pointer);
        base.child(self.state.clone())
    }
}
impl View {
    pub(super) fn sync_otps(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                for instance in self.otps.values() {
                    instance
                        .state
                        .update(cx, |state, _| state.stop_drag(window));
                }
                self.otps.clear();
                return;
            };
            self.otps.retain(|id, instance| {
                if tree.get(*id).is_some() {
                    true
                } else {
                    instance.state.update(cx, |state, cx| {
                        state.stop_drag(window);
                        if state.focus.is_focused(window) {
                            window.blur(cx);
                        }
                    });
                    false
                }
            });
            dirty
                .iter()
                .filter_map(|id| tree.get(*id))
                .filter(|node| node.otp_input.is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        for node in nodes {
            if let Some(instance) = self.otps.get(&node.id) {
                instance.state.update(cx, |state, cx| {
                    state.route.handler = node.handler.expect("validated OTP handler");
                    state.metadata = node.accessibility.clone();
                    let result = state
                        .model
                        .configure(node.otp_input.as_ref().unwrap().config.clone());
                    state.publish(result, cx);
                    if state.model.config().disabled || state.access() == Access::Blocked {
                        state.hide(window, cx);
                    }
                    cx.notify();
                });
            } else {
                match Instance::new(self, &node, window, cx) {
                    Ok(instance) => {
                        self.otps.insert(node.id, instance);
                    }
                    Err(_) => {
                        if self.session.borrow_mut().overload(self.id) {
                            self.transport.fault(self.id);
                        }
                    }
                }
            }
        }
    }
    pub(super) fn hide_unvisited_otps(&self, window: &mut Window, cx: &mut App) {
        for (id, instance) in &self.otps {
            let state = instance.state.read(cx);
            if (!self.visited.contains(id) || state.access() == Access::Blocked)
                && (state.model.editor().is_composing()
                    || state.focus.is_focused(window)
                    || state.dragging)
            {
                let weak = instance.state.downgrade();
                window.defer(cx, move |window, cx| {
                    let _ = weak.update(cx, |s, cx| s.hide(window, cx));
                });
            }
        }
    }
}

#[cfg(feature = "native-tests")]
#[path = "otp_input_test.rs"]
pub(crate) mod test;
