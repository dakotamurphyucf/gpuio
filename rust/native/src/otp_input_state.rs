//! Native ownership and event sequencing around the bounded OTP editing session.
//!
//! All text mutations happen here, before publication to the asynchronous bridge.
//! The adapter supplies actual visibility/modal access and confirmed focus; it
//! must check window/node generations and publish each returned batch in order.
//! Rust-only focus/clipboard callbacks must not reenter this owner or call OCaml.
use crate::otp_edit as edit;
use gpuio_protocol::otp_input::*;
use std::{ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Allowed,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeError {
    Denied(Error),
    Input(InputError),
}
impl From<Error> for NativeError {
    fn from(error: Error) -> Self {
        Self::Denied(error)
    }
}
impl From<InputError> for NativeError {
    fn from(error: InputError) -> Self {
        Self::Input(error)
    }
}
impl From<edit::Error> for NativeError {
    fn from(error: edit::Error) -> Self {
        match error {
            edit::Error::Composing => Self::Denied(Error::Composing),
            edit::Error::Input(error) => Self::Input(error),
        }
    }
}

/// Platform mutation ranges use UTF-16. Explicit selection uses UTF-8 offsets,
/// matching snapshots. Native IME keeps navigation while composing.
pub enum Action<'a> {
    Commit {
        range_utf16: Option<Range<usize>>,
        text: &'a str,
    },
    Mark {
        range_utf16: Option<Range<usize>>,
        text: &'a str,
        selected_utf16: Option<Range<usize>>,
    },
    Paste(&'a str),
    Delete(edit::Delete),
    Move {
        movement: edit::Movement,
        extend: bool,
    },
    Select(Selection),
    Undo,
    Redo,
    CancelComposition,
}
impl Action<'_> {
    fn changes_value(&self) -> bool {
        matches!(
            self,
            Self::Commit { .. }
                | Self::Mark { .. }
                | Self::Paste(_)
                | Self::Delete(_)
                | Self::Undo
                | Self::Redo
        )
    }
    fn event_capacity(&self) -> i64 {
        match self {
            Self::Commit { .. } | Self::Paste(_) | Self::Undo | Self::Redo => 2,
            _ => 1,
        }
    }
}

pub struct Outcome {
    /// At most one programmatic observation, preceding its correlated response.
    pub events: Vec<Event>,
    pub response: Response,
}

/// Policy is immutable per placement. No Debug implementation exposes the code.
pub struct State {
    config: Arc<Config>,
    editor: edit::State,
    revision: i64,
    focused: bool,
}

impl State {
    pub fn new(config: Arc<Config>, initial: &str) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        let editor = edit::State::new(config.policy, initial).map_err(command_input_error)?;
        Ok(Self {
            config,
            editor,
            revision: 0,
            focused: false,
        })
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn editor(&self) -> &edit::State {
        &self.editor
    }
    pub fn revision(&self) -> i64 {
        self.revision
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn snapshot(&self) -> Snapshot {
        let selection = self.editor.selection();
        Snapshot {
            revision: self.revision,
            policy: self.config.policy,
            value: self.editor.value().into(),
            draft: self.editor.draft().into(),
            selection: Selection {
                anchor: selection.anchor as i64,
                head: selection.head as i64,
            },
            composition: self.editor.marked().map(|range| Selection {
                anchor: range.start as i64,
                head: range.end as i64,
            }),
            focused: self.focused,
            can_undo: self.editor.can_undo(),
            can_redo: self.editor.can_redo(),
        }
    }
    fn reserve(&self, count: i64) -> Result<(), Error> {
        self.revision
            .checked_add(count)
            .map(|_| ())
            .ok_or(Error::LimitExceeded)
    }
    fn advance(&mut self) -> Snapshot {
        self.revision += 1; // capacity reserved before any native/text side effect
        let snapshot = self.snapshot();
        debug_assert!(snapshot.is_valid());
        snapshot
    }
    fn access(&self, access: Access, editing: bool) -> Result<(), Error> {
        if self.config.disabled {
            return Err(Error::Disabled);
        }
        if access == Access::Blocked {
            return Err(Error::FocusBlocked);
        }
        if editing && self.config.read_only {
            return Err(Error::ReadOnly);
        }
        Ok(())
    }
    fn not_composing(&self) -> Result<(), Error> {
        if self.editor.is_composing() {
            Err(Error::Composing)
        } else {
            Ok(())
        }
    }
    fn native_events(&mut self, change: edit::Change) -> Vec<Event> {
        if let Some(reason) = change.rejection {
            return vec![Event::Rejected(reason, self.advance())];
        }
        if !change.state_changed {
            return Vec::new();
        }
        let mut events = vec![Event::Changed(self.advance())];
        if change.completed {
            events.push(Event::Complete(self.advance()));
        }
        events
    }
    fn observed(&mut self, change: edit::Change) -> Vec<Event> {
        debug_assert!(change.rejection.is_none());
        if change.state_changed {
            vec![Event::Observed(self.advance())]
        } else {
            Vec::new()
        }
    }

    /// Reserve the operation's maximum event count before mutation. Exhaustion
    /// leaves text/selection/history untouched, including a pending composition.
    /// Admission is conservative near i64::MAX, even if an operation is a no-op.
    pub fn native(
        &mut self,
        action: Action<'_>,
        access: Access,
    ) -> Result<Vec<Event>, NativeError> {
        if matches!(action, Action::Mark { text: "", .. }) {
            return self.cancel_for_lifecycle().map_err(Into::into);
        }
        self.access(access, action.changes_value())?;
        self.reserve(action.event_capacity())?;
        let change = match action {
            Action::Commit { range_utf16, text } => self.editor.commit_utf16(range_utf16, text)?,
            Action::Mark {
                range_utf16,
                text,
                selected_utf16,
            } => self.editor.mark_utf16(range_utf16, text, selected_utf16)?,
            Action::Paste(text) => self.editor.paste(text)?,
            Action::Delete(direction) => self.editor.delete(direction)?,
            Action::Move { movement, extend } => self.editor.move_caret(movement, extend)?,
            Action::Select(selection) => {
                if !selection.within(self.editor.draft()) {
                    return Err(InputError::InvalidSelection.into());
                }
                self.editor.select(edit_selection(selection))?
            }
            Action::Undo => self.editor.undo()?,
            Action::Redo => self.editor.redo()?,
            Action::CancelComposition => self.editor.cancel_composition(),
        };
        Ok(self.native_events(change))
    }

    /// A platform unmark commits only while editing is permitted. If the field
    /// became hidden/modal-blocked/disabled/read-only, it restores the checkpoint
    /// instead. Cleanup must not leave a stranded marked range or bypass gates.
    pub fn unmark(&mut self, access: Access) -> Result<Vec<Event>, Error> {
        if !self.editor.is_composing() {
            return Ok(Vec::new());
        }
        let commit = self.access(access, true).is_ok();
        self.reserve(if commit { 2 } else { 1 })?;
        let change = if commit {
            self.editor.unmark()
        } else {
            self.editor.cancel_composition()
        };
        Ok(self.native_events(change))
    }
    /// Explicit platform teardown/cancellation; safe even after input is gated.
    pub fn cancel_for_lifecycle(&mut self) -> Result<Vec<Event>, Error> {
        if !self.editor.is_composing() {
            return Ok(Vec::new());
        }
        self.reserve(1)?;
        let change = self.editor.cancel_composition();
        Ok(self.native_events(change))
    }
    /// Report actual platform focus; changing focus alone never commits a code.
    /// The adapter separately routes the platform's unmark/cancellation callback.
    pub fn observe_focus(&mut self, focused: bool) -> Result<Vec<Event>, Error> {
        if self.focused == focused {
            return Ok(Vec::new());
        }
        self.reserve(1)?;
        self.focused = focused;
        Ok(vec![Event::Changed(self.advance())])
    }

    /// Label/mask/permission updates retain code, caret, preedit and history.
    /// Reject a policy change atomically; callers must remount deliberately.
    pub fn configure(&mut self, config: Arc<Config>) -> Result<Vec<Event>, Error> {
        if !config.is_valid() || config.policy != self.config.policy {
            return Err(Error::InvalidConfig);
        }
        if config == self.config {
            return Ok(Vec::new());
        }
        self.reserve(1)?;
        self.config = config;
        Ok(vec![Event::Observed(self.advance())])
    }

    /// Unmasked read-only fields can be copied. Empty selection, composition,
    /// disabled/blocked access or masking never replaces the system clipboard.
    pub fn copy_selection(&self, access: Access) -> Option<&str> {
        if self.config.masked || self.editor.is_composing() || self.access(access, false).is_err() {
            return None;
        }
        let range = self.editor.selection().range();
        if range.is_empty() {
            None
        } else {
            Some(&self.editor.value()[range])
        }
    }
    /// Reserve before touching the clipboard. If writing fails, preserve the
    /// code/history. The Rust-only callback must leave this owner untouched.
    pub fn cut(
        &mut self,
        access: Access,
        write: impl FnOnce(&str) -> Result<(), Error>,
    ) -> Result<Vec<Event>, Error> {
        self.access(access, true)?;
        self.not_composing()?;
        if self.config.masked {
            return Ok(Vec::new());
        }
        let range = self.editor.selection().range();
        if range.is_empty() {
            return Ok(Vec::new());
        }
        self.reserve(1)?;
        write(&self.editor.value()[range.clone()])?;
        let change = self
            .editor
            .commit_utf16(Some(range), "")
            .map_err(command_input_error)?;
        Ok(self.native_events(change))
    }

    /// Focus callback must check live native visibility/modal gates, focus the
    /// handle, and confirm success before returning Ok. Errors must have no focus
    /// side effects. No callback is invoked for any other command.
    pub fn execute(
        &mut self,
        command: &Command,
        focus: impl FnOnce() -> Result<(), Error>,
    ) -> Outcome {
        match self.command(command, focus) {
            Ok(events) => Outcome {
                events,
                response: Response::Applied(self.snapshot()),
            },
            Err(error) => Outcome {
                events: Vec::new(),
                response: Response::Failed(error),
            },
        }
    }
    fn command(
        &mut self,
        command: &Command,
        focus: impl FnOnce() -> Result<(), Error>,
    ) -> Result<Vec<Event>, Error> {
        let guard = match command {
            Command::Replace { if_revision, .. } | Command::Clear { if_revision, .. } => {
                *if_revision
            }
            _ => None,
        };
        if guard.is_some_and(|revision| revision < 0 || revision != self.revision) {
            return Err(Error::StaleRevision);
        }
        if matches!(command, Command::ReadSnapshot) {
            return Ok(Vec::new());
        }
        if matches!(
            command,
            Command::Replace { .. }
                | Command::Clear { .. }
                | Command::Select(_)
                | Command::Undo
                | Command::Redo
        ) {
            self.not_composing()?;
        }
        if matches!(command, Command::Undo | Command::Redo) {
            self.access(Access::Allowed, true)?;
        }
        if matches!(command, Command::Focus) && self.config.disabled {
            return Err(Error::FocusBlocked);
        }
        // Validate text and selections before reserving or touching native state.
        let replacement = match command {
            Command::Replace {
                value, selection, ..
            } => {
                if !self.config.policy.canonical(value) {
                    return Err(Error::InvalidValue);
                }
                if !selection.within(value) {
                    return Err(Error::InvalidSelection);
                }
                Some(match selection {
                    SelectionPolicy::Start => edit::Selection::caret(0),
                    SelectionPolicy::End => edit::Selection::caret(value.len()),
                    SelectionPolicy::Preserve => {
                        let s = self.editor.selection();
                        edit::Selection {
                            anchor: s.anchor.min(value.len()),
                            head: s.head.min(value.len()),
                        }
                    }
                    SelectionPolicy::Select(selection) => edit_selection(*selection),
                })
            }
            Command::Select(selection) if !selection.within(self.editor.draft()) => {
                return Err(Error::InvalidSelection);
            }
            _ => None,
        };
        self.reserve(1)?;
        let change = match command {
            Command::Replace { value, undo, .. } => self
                .editor
                .replace(
                    value,
                    replacement.expect("validated replacement"),
                    history(*undo),
                )
                .map_err(command_edit_error)?,
            Command::Clear { undo, .. } => self
                .editor
                .replace("", edit::Selection::caret(0), history(*undo))
                .map_err(command_edit_error)?,
            Command::Select(selection) => self
                .editor
                .select(edit_selection(*selection))
                .map_err(command_input_error)?,
            Command::Focus => {
                focus()?;
                let changed = !self.focused;
                self.focused = true;
                edit::Change {
                    state_changed: changed,
                    ..edit::Change::default()
                }
            }
            Command::Undo => self.editor.undo().map_err(command_edit_error)?,
            Command::Redo => self.editor.redo().map_err(command_edit_error)?,
            Command::CancelComposition => self.editor.cancel_composition(),
            Command::ReadSnapshot => unreachable!("read handled before reservation"),
        };
        Ok(self.observed(change))
    }
}

fn edit_selection(selection: Selection) -> edit::Selection {
    edit::Selection {
        anchor: selection.anchor as usize,
        head: selection.head as usize,
    }
}
fn history(policy: UndoPolicy) -> edit::History {
    match policy {
        UndoPolicy::Record => edit::History::Record,
        UndoPolicy::Reset => edit::History::Reset,
    }
}
fn command_input_error(error: InputError) -> Error {
    match error {
        InputError::InvalidSelection => Error::InvalidSelection,
        InputError::InputTooLarge => Error::LimitExceeded,
        InputError::InvalidPolicy => Error::InvalidConfig,
        InputError::InvalidValue
        | InputError::InvalidUtf8
        | InputError::TooLong
        | InputError::UnexpectedCharacter { .. } => Error::InvalidValue,
    }
}
fn command_edit_error(error: edit::Error) -> Error {
    match error {
        edit::Error::Composing => Error::Composing,
        edit::Error::Input(error) => command_input_error(error),
    }
}

/// Given valid events, the retained bridge must additionally match window/node/handler identities.
/// Semantic events and responses are barriers; never coalesce across them.
pub fn can_coalesce(previous: &Event, next: &Event) -> bool {
    matches!((previous, next), (Event::Changed(a), Event::Changed(b)) if a.policy == b.policy && b.revision > a.revision)
}

#[cfg(test)]
#[path = "otp_input_state_test.rs"]
mod tests;
