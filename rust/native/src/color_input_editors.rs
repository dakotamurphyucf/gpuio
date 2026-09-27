//! Five retained platform editors, with bounded history and one shared color draft.
use super::*;
use gpui_base::input::{self, InputState};
use std::{ops::Range, rc::Rc};

const HISTORY_BYTES: usize = 64 * 1024;
const FIELDS: [c::Field; 5] = [
    c::Field::Hex,
    c::Field::Channel(c::Channel::Hue),
    c::Field::Channel(c::Channel::Saturation),
    c::Field::Channel(c::Channel::Lightness),
    c::Field::Channel(c::Channel::Alpha),
];
#[derive(Clone, PartialEq)]
struct Stamp {
    revision: i64,
    composing: Option<Range<usize>>,
}
impl Stamp {
    fn read(state: &InputState) -> Self {
        Self {
            revision: state.bridge_revision(),
            composing: state.bridge_composition(),
        }
    }
}
#[derive(Clone, PartialEq)]
struct Presentation {
    label: String,
    disabled: bool,
    read_only: bool,
    invalid: bool,
}
pub(super) struct FieldEditor {
    pub(super) state: Entity<InputState>,
    pub(super) focus: FocusHandle,
    seen: Stamp,
    presentation: Option<Presentation>,
}
#[derive(Default)]
pub(super) struct Editors {
    pub(super) fields: Vec<FieldEditor>,
    // Preserve raw spelling/selection/history after a successful text commit.
    preserved: Option<usize>,
    last_focused: Option<usize>,
    // An observer can retire a different field's interaction. Its copied new
    // draft must survive synchronization until the new interaction is installed.
    observing: Option<usize>,
    subscriptions: Vec<Subscription>,
}
fn canonical(snapshot: &c::Snapshot, field: c::Field) -> String {
    match field {
        c::Field::Hex => match snapshot.value {
            Value::Empty => String::new(),
            Value::Color(color) => color.to_hex(),
        },
        // Presentation only: retain full channel precision in the model. Focus
        // and Return without an edit do not start a draft or round the color.
        // User-entered spelling is preserved separately by Editors::preserved.
        c::Field::Channel(channel) => format!("{:.2}", channel.read(snapshot.channels))
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned(),
    }
}
impl ColorInput {
    fn focus_field(
        &mut self,
        field: c::Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), c::Error> {
        let index = FIELDS
            .iter()
            .position(|f| *f == field)
            .expect("known color field");
        if !self.field_enabled(index) {
            return Err(c::Error::FocusBlocked);
        }
        // Complete the old field before acknowledging the new focus; its later
        // blur notification must be an idempotent no-op.
        for previous in 0..self.editors.fields.len() {
            if previous != index && self.editors.fields[previous].focus.is_focused(window) {
                self.finish_editor(previous, true, window, cx);
            }
        }
        if !self.route.current(self.model.config()) {
            return Err(c::Error::NativeFailure);
        }
        window.focus(&self.editors.fields[index].focus, cx);
        if self.editors.fields[index].focus.is_focused(window) {
            Ok(())
        } else {
            Err(c::Error::NativeFailure)
        }
    }
    fn reset_editor_values(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Even an equal-value Set/Reset intentionally retires undo and marked
        // text. Configuration observations do not take this path.
        let snapshot = self.model.snapshot();
        for (index, editor) in self.editors.fields.iter_mut().enumerate() {
            let text = canonical(&snapshot, FIELDS[index]);
            editor.state.update(cx, |input, cx| {
                input.unmark_text(window, cx);
                input.bridge_replace_all(
                    text.clone().into(),
                    (text.len(), text.len()),
                    false,
                    window,
                    cx,
                );
                editor.seen = Stamp::read(input);
            });
        }
    }
    fn field_enabled(&self, index: usize) -> bool {
        self.access(false) == Access::Allowed
            && !self.model.config().disabled
            && !(FIELDS[index] == c::Field::Channel(c::Channel::Alpha)
                && self.model.config().alpha_policy == AlphaPolicy::OpaqueOnly)
    }
    fn editable_field(&self, index: usize) -> bool {
        self.field_enabled(index) && !self.model.config().read_only
    }
    pub(super) fn init_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (index, field) in FIELDS.into_iter().enumerate() {
            let text = canonical(&self.model.snapshot(), field);
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(text)
                    .bridge_max_bytes(c::MAX_DRAFT_BYTES)
                    .bridge_history_budget(HISTORY_BYTES)
                    .validate(|text, _| !text.contains(['\0', '\r', '\n']))
            });
            let focus = state.read(cx).focus_handle(cx);
            let seen = Stamp::read(state.read(cx));
            self.editors
                .subscriptions
                .push(cx.observe_in(&state, window, move |s, _, w, cx| {
                    s.observe_editor(index, w, cx);
                }));
            self.editors
                .subscriptions
                .push(cx.on_blur(&focus, window, move |s, w, cx| {
                    s.finish_editor(index, true, w, cx);
                }));
            self.editors
                .subscriptions
                .push(cx.on_focus(&focus, window, move |s, _, _| {
                    s.editors.last_focused = Some(index);
                }));
            self.editors.fields.push(FieldEditor {
                state,
                focus,
                seen,
                presentation: None,
            });
        }
        self.sync_editors(window, cx);
    }
    pub(super) fn editor_events(&mut self, events: &[c::Event]) {
        for event in events {
            match event {
                c::Event::Committed(c::Source::Text, _) => (),
                c::Event::Cancelled(..) | c::Event::Committed(..) => self.editors.preserved = None,
                c::Event::Observed(_) | c::Event::Started(_) | c::Event::Preview(_) => (),
            }
        }
    }
    pub(super) fn sync_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = self.model.snapshot();
        let active = snapshot.interaction.and_then(|i| match i.kind {
            c::InteractionKind::Text(field) => FIELDS.iter().position(|f| *f == field),
            _ => None,
        });
        for (index, field) in FIELDS
            .into_iter()
            .enumerate()
            .take(self.editors.fields.len())
        {
            let config = self.model.config();
            let label = match field {
                c::Field::Hex => &config.labels.hex,
                c::Field::Channel(c::Channel::Hue) => &config.labels.hue,
                c::Field::Channel(c::Channel::Saturation) => &config.labels.saturation,
                c::Field::Channel(c::Channel::Lightness) => &config.labels.lightness,
                c::Field::Channel(c::Channel::Alpha) => &config.labels.alpha,
            };
            let presentation = Presentation {
                label: label.clone(),
                disabled: !self.field_enabled(index),
                read_only: config.read_only,
                invalid: active == Some(index)
                    && snapshot
                        .draft
                        .as_ref()
                        .is_some_and(|d| d.status != c::DraftStatus::Valid && !d.composing),
            };
            let preserve = active == Some(index)
                || self.editors.preserved == Some(index)
                || self.editors.observing == Some(index);
            let text = canonical(&snapshot, field);
            let weak = cx.weak_entity();
            let editor = &mut self.editors.fields[index];
            let configure = editor.presentation.as_ref() != Some(&presentation);
            let replace = !preserve
                && (editor.state.read(cx).value().as_ref() != text
                    || editor.state.read(cx).bridge_composition().is_some());
            if configure || replace {
                editor.state.update(cx, |state, cx| {
                    if replace {
                        state.unmark_text(window, cx);
                        state.bridge_replace_all(
                            text.clone().into(),
                            (text.len(), text.len()),
                            false,
                            window,
                            cx,
                        );
                    }
                    if configure {
                        configure_input(state, presentation.clone(), index, weak, cx);
                    }
                    editor.seen = Stamp::read(state);
                });
                editor.presentation = Some(presentation);
            }
        }
    }
    pub(super) fn observe_editor(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = &mut self.editors.fields[index];
        let stamp = Stamp::read(editor.state.read(cx));
        if stamp == editor.seen {
            return;
        }
        let text = editor.state.read(cx).value().to_string();
        let composing = stamp.composing.is_some();
        editor.seen = stamp;
        if !self.editable_field(index) {
            self.editors.preserved = None;
            self.sync_editors(window, cx);
            return;
        }
        self.editors.observing = Some(index);
        self.editors.preserved = None;
        let field = FIELDS[index];
        let active = self.model.snapshot().interaction;
        if active.is_some_and(|i| i.kind != c::InteractionKind::Text(field)) {
            self.cancel(c::CancelReason::Interrupted, window, cx);
        }
        let access = self.access(false);
        let mut events = Vec::with_capacity(2);
        let result = (|| {
            let interaction = match self.model.snapshot().interaction {
                Some(interaction) => interaction,
                None => {
                    let started = self.model.begin_text(field, text.clone(), access)?;
                    let interaction = started.snapshot().interaction.unwrap();
                    events.push(started);
                    interaction
                }
            };
            if let Some(event) = self
                .model
                .preview_text(interaction.id, text, composing, access)?
            {
                events.push(event);
            }
            Ok(events)
        })();
        self.publish(result, window, cx);
        self.editors.observing = None;
        self.sync_editors(window, cx);
    }
    pub(super) fn finish_editor(
        &mut self,
        index: usize,
        blur: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.observe_editor(index, window, cx);
        let Some(interaction) = self
            .model
            .snapshot()
            .interaction
            .filter(|i| i.kind == c::InteractionKind::Text(FIELDS[index]))
        else {
            return;
        };
        if !self.editable_field(index) {
            self.cancel(c::CancelReason::Interrupted, window, cx);
            return;
        }
        let result = self.model.finish(interaction.id, self.access(false));
        match result {
            Ok(event) => {
                self.editors.preserved = Some(index);
                self.publish(Ok(vec![event]), window, cx);
            }
            Err(c::Error::InvalidDraft | c::Error::Composing | c::Error::InvalidValue) if blur => {
                self.cancel(c::CancelReason::Interrupted, window, cx);
            }
            Err(error) => self.publish(Err(error), window, cx),
        }
    }
    pub(super) fn editor_element(&self, index: usize, window: &Window) -> impl IntoElement + use<> {
        let editor = &self.editors.fields[index];
        let border = if editor.presentation.as_ref().is_some_and(|p| p.invalid) {
            rgba(0xef4444ff).into()
        } else if editor.focus.is_focused(window) {
            window.text_style().color
        } else {
            window.text_style().color.opacity(0.25)
        };
        div()
            .relative()
            .min_w(px(0.))
            .px(px(6.))
            .py(px(3.))
            .border_1()
            .rounded(px(4.))
            .border_color(border)
            .child(editor.state.clone())
            .child(self.record(
                &editor.focus,
                261 + index as u16,
                self.field_enabled(index),
                true,
            ))
    }
}
fn configure_input(
    state: &mut InputState,
    presentation: Presentation,
    index: usize,
    owner: WeakEntity<ColorInput>,
    cx: &mut Context<InputState>,
) {
    state.set_disabled(presentation.disabled, cx);
    state.set_readonly(presentation.read_only, cx);
    state.set_submit_on_enter(true, cx);
    state.set_bridge_decorator(Rc::new(move |element, state, _, _| {
        let mut element = element
            .role(Role::TextInput)
            .aria_label(presentation.label.clone())
            .aria_value(state.value());
        let enter = owner.clone();
        element = element.capture_action(move |_: &input::Enter, w, cx| {
            let _ = enter.update(cx, |s, cx| s.finish_editor(index, false, w, cx));
            cx.stop_propagation();
        });
        let next = owner.clone();
        element = element.capture_action(move |_: &input::IndentInline, w, cx| {
            let _ = next.update(cx, |s, cx| {
                s.finish_editor(index, true, w, cx);
                s.route.gate.borrow().traverse(false, w, cx);
            });
            cx.stop_propagation();
        });
        let previous = owner.clone();
        element = element.capture_action(move |_: &input::OutdentInline, w, cx| {
            let _ = previous.update(cx, |s, cx| {
                s.finish_editor(index, true, w, cx);
                s.route.gate.borrow().traverse(true, w, cx);
            });
            cx.stop_propagation();
        });
        let focus = owner.clone();
        element = element.on_a11y_action(AccessibleAction::Focus, move |_, w, cx| {
            let _ = focus.update(cx, |s, cx| {
                if s.field_enabled(index) {
                    w.focus(&s.editors.fields[index].focus, cx);
                }
            });
        });
        if !presentation.disabled && !presentation.read_only {
            let set = owner.clone();
            element = element.on_a11y_action(AccessibleAction::SetValue, move |data, w, cx| {
                if let Some(accesskit::ActionData::Value(text)) = data {
                    let _ = set.update(cx, |s, cx| {
                        if !s.editable_field(index) {
                            return;
                        }
                        let editor = s.editors.fields[index].state.clone();
                        editor.update(cx, |state, cx| {
                            let command = gpuio_protocol::v1::EditorCommand::Replace(
                                text.clone().into(),
                                gpuio_protocol::v1::EditorSelectionPolicy::End,
                                gpuio_protocol::v1::EditorUndoPolicy::Record,
                                None,
                            );
                            let _ = super::super::editor::apply(state, &command, true, w, cx);
                        });
                        s.observe_editor(index, w, cx);
                    });
                }
            });
        }
        let metadata = presentation.invalid.then(|| {
            Arc::new(gpuio_protocol::accessibility::Config {
                role: None,
                label: None,
                description: None,
                live: gpuio_protocol::accessibility::Live::Off,
                current: None,
                field: Some(gpuio_protocol::accessibility::Field {
                    label: presentation.label.clone(),
                    help: None,
                    error: Some(match FIELDS[index] {
                        c::Field::Hex => "Enter a hexadecimal color".into(),
                        c::Field::Channel(channel) => {
                            format!("Enter a number from 0 to {}", channel.maximum())
                        }
                    }),
                    required: false,
                }),
            })
        });
        crate::semantics::State {
            element,
            metadata,
            live: None,
            hidden: false,
            disabled: presentation.disabled,
            read_only: presentation.read_only,
            modal: false,
        }
        .into_any_element()
    }));
    cx.notify();
}

impl Instance {
    pub(in crate::host) fn command(
        &self,
        command: &c::Command,
        window: &mut Window,
        cx: &mut App,
    ) -> c::Response {
        self.state.update(cx, |state, cx| {
            if state.closed {
                return c::Response::Failed(c::Error::Closed);
            }
            if !state
                .route
                .session
                .borrow()
                .accepts_input(state.route.window)
            {
                return c::Response::Failed(c::Error::NativeFailure);
            }
            if !state.route.current(state.model.config()) {
                return c::Response::Failed(c::Error::StaleColorInput);
            }
            // Platform edits can precede their deferred entity notification.
            // Observe them before checking a guard or returning a snapshot.
            for index in 0..state.editors.fields.len() {
                state.observe_editor(index, window, cx);
            }
            if !state.route.current(state.model.config()) {
                return c::Response::Failed(c::Error::NativeFailure);
            }
            let reset_editors =
                matches!(command, c::Command::Set { .. } | c::Command::Reset { .. });
            let result = match command {
                c::Command::Set { value, if_revision } => state.model.set(*value, *if_revision),
                c::Command::Reset { if_revision } => state.model.reset(*if_revision),
                c::Command::Cancel => state
                    .model
                    .cancel(c::CancelReason::Programmatic)
                    .map(|event| event.into_iter().collect()),
                c::Command::ReadSnapshot => Ok(Vec::new()),
                c::Command::Focus(field) => {
                    state.focus_field(*field, window, cx).map(|()| Vec::new())
                }
            };
            let events = match result {
                Ok(events) => events,
                Err(error) => {
                    state.publish(Err(error), window, cx);
                    return c::Response::Failed(error);
                }
            };
            if reset_editors || matches!(command, c::Command::Cancel) {
                state.release(window);
            }
            if reset_editors {
                state.editors.preserved = None;
            }
            state.publish(Ok(events), window, cx);
            if !state.route.current(state.model.config()) {
                return c::Response::Failed(c::Error::NativeFailure);
            }
            if reset_editors {
                state.reset_editor_values(window, cx);
            }
            c::Response::Applied(state.model.snapshot())
        })
    }

    fn command_field(&self, window: &Window, cx: &App) -> Option<usize> {
        let state = self.state.read(cx);
        state
            .editors
            .fields
            .iter()
            .position(|editor| editor.focus.is_focused(window))
            .or(state.editors.last_focused)
            .filter(|index| state.field_enabled(*index))
    }
    pub(in crate::host) fn editing(&self, window: &Window, cx: &App) -> bool {
        self.state
            .read(cx)
            .editors
            .fields
            .iter()
            .any(|editor| editor.focus.is_focused(window))
    }
    pub(in crate::host) fn is_composing(&self, cx: &App) -> bool {
        self.state
            .read(cx)
            .editors
            .fields
            .iter()
            .any(|editor| editor.state.read(cx).bridge_composition().is_some())
    }
    pub(in crate::host) fn command_focus(&self, window: &Window, cx: &App) -> Option<FocusHandle> {
        self.command_field(window, cx)
            .map(|index| self.state.read(cx).editors.fields[index].focus.clone())
    }
    pub(in crate::host) fn command_available(
        &self,
        action: gpuio_protocol::v1::NativeCommand,
        window: &Window,
        cx: &App,
    ) -> bool {
        use gpuio_protocol::v1::NativeCommand;
        let Some(index) = self.command_field(window, cx) else {
            return false;
        };
        let state = self.state.read(cx);
        let field = state.editors.fields[index].state.read(cx);
        match action {
            NativeCommand::Copy => field.is_copyable(),
            NativeCommand::Cut => state.editable_field(index) && field.is_copyable(),
            NativeCommand::Paste | NativeCommand::Undo | NativeCommand::Redo => {
                state.editable_field(index)
            }
            NativeCommand::SelectAll => !field.value().is_empty(),
        }
    }
}
