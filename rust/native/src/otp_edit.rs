//! Bounded editing session for one segmented OTP field.
//!
//! GPUI's platform adapter supplies UTF-16 ranges; snapshots/rendering use UTF-8
//! offsets. Accepted text is canonical ASCII. IME preedit is a separate bounded
//! draft and never enters undo history until a valid commit. This module has no
//! platform calls, focus/configuration gates, bridge revisions or event queue.
//! The owner must apply those gates and reserve event revisions before mutation.
use gpuio_protocol::otp::{InputError, MAX_INPUT_BYTES, Policy};
use std::{collections::VecDeque, ops::Range};

/// Across undo and redo, at most 128 edits with two <=32-byte values each.
/// Composition additionally retains at most MAX_INPUT_BYTES of draft text.
pub const MAX_HISTORY_EDITS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}
impl Selection {
    pub fn caret(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }
    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
    fn valid(self, text: &str) -> bool {
        text.is_char_boundary(self.anchor) && text.is_char_boundary(self.head)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input(InputError),
    Composing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    Left,
    Right,
    Start,
    End,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delete {
    Backward,
    Forward,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum History {
    Record,
    Reset,
}
impl From<InputError> for Error {
    fn from(error: InputError) -> Self {
        Self::Input(error)
    }
}

/// The owner emits observations for selection/composition/history changes too.
/// `completed` is true only when a changed accepted value is full. Programmatic
/// replacements use this result for observations, not user completion events.
/// Rejection ends a pending composition and restores its exact checkpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Change {
    pub state_changed: bool,
    pub value_changed: bool,
    pub completed: bool,
    pub rejection: Option<InputError>,
}

#[derive(Clone, PartialEq, Eq)]
struct Text {
    value: String,
    selection: Selection,
}
struct Composition {
    draft: Text,
    marked: Range<usize>,
}
struct Edit {
    before: Text,
    after: Text,
}

/// Policy is immutable for a placement. Remount explicitly to change it.
/// Deliberately no Debug implementation: diagnostics must not print OTP values.
pub struct State {
    policy: Policy,
    accepted: Text,
    composition: Option<Composition>,
    undo: VecDeque<Edit>,
    redo: VecDeque<Edit>,
}

impl State {
    pub fn new(policy: Policy, value: &str) -> Result<Self, InputError> {
        if !policy.canonical(value) {
            return Err(InputError::InvalidValue);
        }
        Ok(Self {
            policy,
            accepted: Text {
                value: value.into(),
                selection: Selection::caret(value.len()),
            },
            composition: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
        })
    }
    pub fn policy(&self) -> Policy {
        self.policy
    }
    pub fn value(&self) -> &str {
        &self.accepted.value
    }
    fn current(&self) -> &Text {
        self.composition
            .as_ref()
            .map_or(&self.accepted, |c| &c.draft)
    }
    pub fn draft(&self) -> &str {
        &self.current().value
    }
    pub fn selection(&self) -> Selection {
        self.current().selection
    }
    pub fn marked(&self) -> Option<Range<usize>> {
        self.composition.as_ref().map(|c| c.marked.clone())
    }
    pub fn is_composing(&self) -> bool {
        self.composition.is_some()
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn history_edits(&self) -> usize {
        self.undo.len() + self.redo.len()
    }
    /// Text payload accounting only, not allocator/struct overhead or process RSS.
    pub fn history_bytes(&self) -> usize {
        self.undo
            .iter()
            .chain(&self.redo)
            .map(|edit| edit.before.value.len() + edit.after.value.len())
            .sum()
    }
    fn ensure_not_composing(&self) -> Result<(), Error> {
        if self.is_composing() {
            Err(Error::Composing)
        } else {
            Ok(())
        }
    }
    fn replacement_range(&self, utf16: Option<Range<usize>>) -> Result<Range<usize>, InputError> {
        match utf16 {
            Some(range) => utf16_range(self.draft(), range),
            None => Ok(self.marked().unwrap_or_else(|| self.selection().range())),
        }
    }
    fn accept(&mut self, next: Text, reset_history: bool) -> Change {
        let value_changed = next.value != self.accepted.value;
        let state_changed = self.is_composing()
            || next != self.accepted
            || reset_history && self.history_edits() != 0;
        if reset_history {
            self.undo.clear();
            self.redo.clear();
        } else if value_changed {
            self.redo.clear();
            if self.undo.len() == MAX_HISTORY_EDITS {
                self.undo.pop_front();
            }
            self.undo.push_back(Edit {
                before: self.accepted.clone(),
                after: next.clone(),
            });
        }
        self.composition = None;
        self.accepted = next;
        Change {
            state_changed,
            value_changed,
            completed: value_changed && self.value().len() == self.policy.length(),
            rejection: None,
        }
    }
    fn reject(&mut self, error: InputError) -> Change {
        Change {
            state_changed: self.composition.take().is_some(),
            rejection: Some(error),
            ..Change::default()
        }
    }

    /// Native committed insertion. Malformed ranges fail without changing state;
    /// invalid text produces a rejected edit (and rolls back active composition).
    pub fn commit_utf16(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
    ) -> Result<Change, InputError> {
        let range = self.replacement_range(range)?;
        if !self.is_composing() {
            return Ok(
                match self
                    .policy
                    .replace(self.value(), range.start, range.end, text, false)
                {
                    Ok((value, anchor, head)) => self.accept(
                        Text {
                            value,
                            selection: Selection { anchor, head },
                        },
                        false,
                    ),
                    Err(error) => self.reject(error),
                },
            );
        }
        let candidate = match replace_raw(self.draft(), range.clone(), text) {
            Ok(value) => Text {
                value,
                selection: Selection::caret(range.start + text.len()),
            },
            Err(error) => return Ok(self.reject(error)),
        };
        Ok(self.finish(candidate))
    }

    /// Paste is distinct from typed input and rejects while composing. The
    /// adapter must never silently discard an active IME session to paste.
    pub fn paste(&mut self, text: &str) -> Result<Change, Error> {
        self.ensure_not_composing()?;
        let selection = self.selection();
        Ok(
            match self
                .policy
                .replace(self.value(), selection.anchor, selection.head, text, true)
            {
                Ok((value, anchor, head)) => self.accept(
                    Text {
                        value,
                        selection: Selection { anchor, head },
                    },
                    false,
                ),
                Err(error) => self.reject(error),
            },
        )
    }

    /// Preedit accepts arbitrary single-line Unicode, with a bounded total draft.
    /// The selected range is relative to the inserted preedit, in UTF-16 units.
    /// Invalid ranges/oversized preedit leave the previous composition untouched.
    /// An empty marked string cancels and restores the original code/selection.
    pub fn mark_utf16(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
    ) -> Result<Change, InputError> {
        if text.is_empty() {
            return Ok(self.cancel_composition());
        }
        if text.len() > MAX_INPUT_BYTES {
            return Err(InputError::InputTooLarge);
        }
        if let Some((byte_offset, _)) = text
            .char_indices()
            .find(|(_, ch)| matches!(ch, '\0' | '\r' | '\n'))
        {
            return Err(InputError::UnexpectedCharacter { byte_offset });
        }
        let range = self.replacement_range(range)?;
        let selected = match selected {
            Some(range) => utf16_range(text, range)?,
            None => text.len()..text.len(),
        };
        let draft = Text {
            value: replace_raw(self.draft(), range.clone(), text)?,
            selection: Selection {
                anchor: range.start + selected.start,
                head: range.start + selected.end,
            },
        };
        let marked = range.start..range.start + text.len();
        let state_changed = self
            .composition
            .as_ref()
            .is_none_or(|c| c.draft != draft || c.marked != marked);
        self.composition = Some(Composition { draft, marked });
        Ok(Change {
            state_changed,
            ..Change::default()
        })
    }
    fn finish(&mut self, draft: Text) -> Change {
        match self.policy.normalize(&draft.value, false) {
            Ok(value) => {
                // Every accepted scalar normalizes to precisely one ASCII cell.
                let selection = Selection {
                    anchor: draft.value[..draft.selection.anchor].chars().count(),
                    head: draft.value[..draft.selection.head].chars().count(),
                };
                self.accept(Text { value, selection }, false)
            }
            Err(error) => self.reject(error),
        }
    }
    pub fn unmark(&mut self) -> Change {
        match self.composition.as_ref() {
            Some(composition) => self.finish(composition.draft.clone()),
            None => Change::default(),
        }
    }
    pub fn cancel_composition(&mut self) -> Change {
        Change {
            state_changed: self.composition.take().is_some(),
            ..Change::default()
        }
    }

    /// Platform selection may move inside preedit; accepted selection remains the
    /// rollback checkpoint. Programmatic Select must first reject composition.
    pub fn select(&mut self, selection: Selection) -> Result<Change, InputError> {
        if !selection.valid(self.draft()) {
            return Err(InputError::InvalidSelection);
        }
        let current = match self.composition.as_mut() {
            Some(c) => &mut c.draft,
            None => &mut self.accepted,
        };
        let state_changed = current.selection != selection;
        current.selection = selection;
        Ok(Change {
            state_changed,
            ..Change::default()
        })
    }
    /// Outside composition, one byte is one complete code cell. Native IME owns
    /// navigation while composing; never apply ASCII movement to its raw draft.
    pub fn move_caret(&mut self, movement: Movement, extend: bool) -> Result<Change, Error> {
        self.ensure_not_composing()?;
        let selection = self.selection();
        let range = selection.range();
        let head = match movement {
            Movement::Left if !extend && !range.is_empty() => range.start,
            Movement::Right if !extend && !range.is_empty() => range.end,
            Movement::Left => selection.head.saturating_sub(1),
            Movement::Right => (selection.head + 1).min(self.value().len()),
            Movement::Start => 0,
            Movement::End => self.value().len(),
        };
        Ok(self.select(Selection {
            anchor: if extend { selection.anchor } else { head },
            head,
        })?)
    }
    pub fn delete(&mut self, direction: Delete) -> Result<Change, Error> {
        self.ensure_not_composing()?;
        let mut range = self.selection().range();
        if range.is_empty() {
            match direction {
                Delete::Backward => range.start = range.start.saturating_sub(1),
                Delete::Forward => range.end = (range.end + 1).min(self.value().len()),
            }
        }
        Ok(self.commit_utf16(Some(range), "")?)
    }
    /// Explicit replacement uses an already canonical value. Invalid text,
    /// selection or composition cannot clear history or partially replace text.
    pub fn replace(
        &mut self,
        value: &str,
        selection: Selection,
        history: History,
    ) -> Result<Change, Error> {
        self.ensure_not_composing()?;
        if !self.policy.canonical(value) {
            return Err(InputError::InvalidValue.into());
        }
        if !selection.valid(value) {
            return Err(InputError::InvalidSelection.into());
        }
        Ok(self.accept(
            Text {
                value: value.into(),
                selection,
            },
            history == History::Reset,
        ))
    }
    pub fn undo(&mut self) -> Result<Change, Error> {
        self.ensure_not_composing()?;
        let Some(edit) = self.undo.pop_back() else {
            return Ok(Change::default());
        };
        let change = self.restore_history(edit.before.clone());
        self.redo.push_back(edit);
        Ok(change)
    }
    pub fn redo(&mut self) -> Result<Change, Error> {
        self.ensure_not_composing()?;
        let Some(edit) = self.redo.pop_back() else {
            return Ok(Change::default());
        };
        let change = self.restore_history(edit.after.clone());
        self.undo.push_back(edit);
        Ok(change)
    }
    fn restore_history(&mut self, text: Text) -> Change {
        let value_changed = text.value != self.accepted.value;
        self.accepted = text;
        Change {
            state_changed: true,
            value_changed,
            completed: value_changed && self.value().len() == self.policy.length(),
            rejection: None,
        }
    }
}

fn replace_raw(value: &str, range: Range<usize>, text: &str) -> Result<String, InputError> {
    if text.len() > MAX_INPUT_BYTES || value.len() - range.len() + text.len() > MAX_INPUT_BYTES {
        return Err(InputError::InputTooLarge);
    }
    Ok(format!(
        "{}{}{}",
        &value[..range.start],
        text,
        &value[range.end..]
    ))
}

/// Exact scalar boundaries only. Surrogate-interior and out-of-document edit
/// positions reject rather than silently modifying a neighboring character.
pub fn utf16_to_byte(text: &str, offset: usize) -> Result<usize, InputError> {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units == offset {
            return Ok(byte);
        }
        units += ch.len_utf16();
        if units > offset {
            return Err(InputError::InvalidSelection);
        }
    }
    if units == offset {
        Ok(text.len())
    } else {
        Err(InputError::InvalidSelection)
    }
}
pub fn byte_to_utf16(text: &str, offset: usize) -> Result<usize, InputError> {
    if !text.is_char_boundary(offset) {
        return Err(InputError::InvalidSelection);
    }
    Ok(text[..offset].encode_utf16().count())
}
pub fn utf16_range(text: &str, range: Range<usize>) -> Result<Range<usize>, InputError> {
    if range.start > range.end {
        return Err(InputError::InvalidSelection);
    }
    Ok(utf16_to_byte(text, range.start)?..utf16_to_byte(text, range.end)?)
}

#[cfg(test)]
#[path = "otp_edit_test.rs"]
mod tests;
