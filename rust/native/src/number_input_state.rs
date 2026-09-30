//! Numeric policy around a native editor, independent of GPUI drawing.
//!
//! The editor owns text, selection, composition and history. This owner caches
//! one bounded observation for publication/deduplication and owns configuration,
//! committed value and the numeric revision. The mounted adapter supplies the
//! current live editor snapshot before every command/configuration update, and
//! executes the prepared editor command synchronously without reentering this
//! owner. A returned Fault requires faulting window input; it is sticky here too.
use gpuio_protocol::{
    number_input::*,
    numeric::{Draft, DraftError},
    v1::{
        EditorCommand, EditorError, EditorSelection, EditorSelectionPolicy, EditorSnapshot,
        EditorUndoPolicy,
    },
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    InvalidObservation,
    RevisionExhausted,
    UnexpectedEdit,
}

/// At most two events: a pending ordinary observation, then this operation's
/// boundary/change. Publish in order before its correlated response. Do not
/// coalesce semantic events or responses with ordinary changes.
#[derive(Debug)]
pub struct Outcome {
    pub events: Vec<Event>,
    pub response: Response,
}

pub struct State {
    config: Arc<Config>,
    snapshot: Snapshot,
    editor_revision: i64,
    fault: Option<Fault>,
}

/// Initial value normalization and text for the native editor's one-time seed.
/// Rust's decimal formatter roundtrips every finite binary float accepted here.
/// No formatting is performed on ordinary native draft observations.
pub fn initial_text(config: &Config, value: Value) -> Result<String, Error> {
    if !config.is_valid() {
        return Err(Error::InvalidConfig);
    }
    let value = value.normalized(config.domain).ok_or(Error::InvalidValue)?;
    Ok(format_value(value))
}
fn format_value(value: Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(value) => value.to_string(),
    }
}
fn selection(value: &EditorSelection) -> Selection {
    Selection {
        anchor: value.anchor,
        head: value.head,
    }
}
fn editor_selection(value: Selection) -> EditorSelection {
    EditorSelection {
        anchor: value.anchor,
        head: value.head,
    }
}
fn selection_policy(value: SelectionPolicy) -> EditorSelectionPolicy {
    match value {
        SelectionPolicy::Start => EditorSelectionPolicy::Start,
        SelectionPolicy::End => EditorSelectionPolicy::End,
        SelectionPolicy::Preserve => EditorSelectionPolicy::Preserve,
        SelectionPolicy::Select(s) => EditorSelectionPolicy::Select(editor_selection(s)),
    }
}
fn undo_policy(value: UndoPolicy) -> EditorUndoPolicy {
    match value {
        UndoPolicy::Record => EditorUndoPolicy::Record,
        UndoPolicy::Reset => EditorUndoPolicy::Reset,
    }
}
fn editor_error(value: EditorError) -> Error {
    match value {
        EditorError::NotMounted => Error::NotMounted,
        EditorError::Closed => Error::Closed,
        EditorError::StaleEditor => Error::StaleInput,
        EditorError::StaleRevision => Error::StaleRevision,
        EditorError::Composing => Error::Composing,
        EditorError::InvalidSelection => Error::InvalidSelection,
        EditorError::LimitExceeded => Error::LimitExceeded,
        EditorError::Busy => Error::Busy,
        EditorError::NativeFailure => Error::NativeFailure,
        EditorError::InvalidText => Error::InvalidText,
        EditorError::FocusBlocked => Error::FocusBlocked,
    }
}

#[derive(Clone, Copy)]
enum Publication {
    Changed,
    Observed,
    Committed(Source),
    Cancelled(CancelReason),
}
impl Publication {
    fn semantic(self) -> bool {
        matches!(self, Self::Committed(_) | Self::Cancelled(_))
    }
    fn event(self, snapshot: Snapshot) -> Event {
        match self {
            Self::Changed => Event::Changed(snapshot),
            Self::Observed => Event::Observed(snapshot),
            Self::Committed(source) => Event::Committed(source, snapshot),
            Self::Cancelled(reason) => Event::Cancelled(reason, snapshot),
        }
    }
}
struct Edit {
    command: EditorCommand,
    committed: Value,
    publication: Publication,
}
enum Preparation {
    Read,
    Edit(Edit),
    Reject(Rejection),
}

impl State {
    /// `editor` must contain the normalized initial text and no composition.
    /// Initial autofocus can be reflected here or subsequently observed.
    pub fn new(
        config: Arc<Config>,
        initial: Value,
        editor: &EditorSnapshot,
    ) -> Result<Self, Error> {
        let model = Self::with_initial_draft(config, initial, editor)?;
        if !model.snapshot.is_settled() {
            return Err(Error::NativeFailure);
        }
        Ok(model)
    }

    /// Explicit draft recovery on creation. The draft can be incomplete/invalid;
    /// the committed value is normalized independently. This is not a restored
    /// IME session or undo history, and cannot contain active composition.
    pub fn with_initial_draft(
        config: Arc<Config>,
        initial: Value,
        editor: &EditorSnapshot,
    ) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if editor.revision < 0 || !valid_text(&editor.text) {
            return Err(Error::NativeFailure);
        }
        let committed = initial
            .normalized(config.domain)
            .ok_or(Error::InvalidValue)?;
        let snapshot = Snapshot {
            revision: 0,
            domain: config.domain,
            draft: editor.text.clone(),
            committed,
            selection: selection(&editor.selection),
            composition: editor.composition.as_ref().map(selection),
            focused: editor.focused,
        };
        if !snapshot.is_valid() || snapshot.composition.is_some() {
            return Err(Error::NativeFailure);
        }
        Ok(Self {
            config,
            snapshot,
            editor_revision: editor.revision,
            fault: None,
        })
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    fn fault<T>(&mut self, fault: Fault) -> Result<T, Fault> {
        self.fault = Some(fault);
        Err(fault)
    }
    fn reserve(&self) -> Result<i64, Error> {
        self.snapshot
            .revision
            .checked_add(1)
            .ok_or(Error::LimitExceeded)
    }
    fn observation(&self, editor: &EditorSnapshot, committed: Value) -> Snapshot {
        Snapshot {
            revision: self.snapshot.revision,
            domain: self.config.domain,
            draft: editor.text.clone(),
            committed,
            selection: selection(&editor.selection),
            composition: editor.composition.as_ref().map(selection),
            focused: editor.focused,
        }
    }
    /// Ordinary native changes may already have happened. On overflow/invalid
    /// state we cannot rewind them: fault rather than publish an inconsistent
    /// observation. Repeated notifications and hidden inner text revisions
    /// without exposed state changes do not consume numeric revisions.
    pub fn observe(&mut self, editor: &EditorSnapshot) -> Result<Option<Event>, Fault> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        if editor.revision < self.editor_revision || !valid_text(&editor.text) {
            return self.fault(Fault::InvalidObservation);
        }
        let mut next = self.observation(editor, self.snapshot.committed);
        if !next.is_valid() {
            return self.fault(Fault::InvalidObservation);
        }
        if next == self.snapshot {
            self.editor_revision = editor.revision;
            return Ok(None);
        }
        let Ok(revision) = self.reserve() else {
            return self.fault(Fault::RevisionExhausted);
        };
        next.revision = revision;
        self.editor_revision = editor.revision;
        self.snapshot = next.clone();
        Ok(Some(Event::Changed(next)))
    }
    /// Update policy without changing text/caret/composition/history. Synchronize
    /// the live editor first, then normalize committed value in the new domain.
    /// Failed admission or exhausted command revision leaves config unchanged.
    pub fn configure(
        &mut self,
        config: Arc<Config>,
        editor: &EditorSnapshot,
    ) -> Result<Outcome, Fault> {
        let mut events: Vec<_> = self.observe(editor)?.into_iter().collect();
        let result = if !config.is_valid() {
            Err(Error::InvalidConfig)
        } else if self.config == config {
            Ok(())
        } else {
            self.reserve().map(|revision| {
                self.snapshot.revision = revision;
                self.snapshot.domain = config.domain;
                self.snapshot.committed = self
                    .snapshot
                    .committed
                    .normalized(config.domain)
                    .expect("validated finite committed value");
                self.config = config;
                events.push(Event::Observed(self.snapshot.clone()));
            })
        };
        Ok(Outcome {
            events,
            response: match result {
                Ok(()) => Response::Applied(self.snapshot.clone()),
                Err(error) => Response::Failed(error),
            },
        })
    }
    fn user_mutation_allowed(&self) -> Result<(), Error> {
        if self.config.disabled {
            Err(Error::Disabled)
        } else if self.config.read_only {
            Err(Error::ReadOnly)
        } else {
            Ok(())
        }
    }
    fn draft_value(&self, stepping: bool) -> Result<Value, Rejection> {
        match self.snapshot.classification() {
            Draft::Empty if stepping => Ok(Value::Number(
                self.config.domain.normalize(0.).expect("finite zero"),
            )),
            Draft::Empty if self.config.allow_empty => Ok(Value::Empty),
            Draft::Empty => Err(Rejection::EmptyRequired),
            Draft::Incomplete => Err(Rejection::Incomplete),
            Draft::Invalid(DraftError::Syntax) => Err(Rejection::Syntax),
            Draft::Invalid(DraftError::NonFinite) => Err(Rejection::NonFinite),
            // Snapshot admission has already enforced the same 4096-byte bound.
            Draft::Invalid(DraftError::TooLong) => unreachable!("bounded numeric snapshot"),
            Draft::Valid(value) | Draft::OutOfRange(value) => Ok(Value::Number(
                self.config
                    .domain
                    .normalize(value)
                    .expect("finite parsed number"),
            )),
        }
    }
    fn replacement(
        &self,
        text: String,
        value: Value,
        policy: SelectionPolicy,
        undo: UndoPolicy,
        publication: Publication,
    ) -> Result<Preparation, Error> {
        if !valid_text(&text) {
            return Err(Error::InvalidText);
        }
        if !policy.within(&text) {
            return Err(Error::InvalidSelection);
        }
        Ok(Preparation::Edit(Edit {
            command: EditorCommand::Replace(
                text,
                selection_policy(policy),
                undo_policy(undo),
                Some(self.editor_revision),
            ),
            committed: value,
            publication,
        }))
    }
    fn prepare(&self, command: &Command, source: Source) -> Result<Preparation, Error> {
        let guard = match command {
            Command::ReplaceDraft { if_revision, .. }
            | Command::ReplaceValue { if_revision, .. } => *if_revision,
            _ => None,
        };
        if guard.is_some_and(|revision| revision != self.snapshot.revision) {
            return Err(Error::StaleRevision);
        }
        // Programmatic replacement/commit/cancel are explicit edits even when
        // disabled/read-only. User mutation and every Step obey editing policy.
        if matches!(command, Command::Step(_))
            || (source != Source::Programmatic
                && !matches!(
                    command,
                    Command::ReadSnapshot | Command::Focus | Command::Select(_)
                ))
        {
            self.user_mutation_allowed()?;
        }
        if source != Source::Programmatic
            && self.config.disabled
            && matches!(command, Command::Select(_))
        {
            return Err(Error::Disabled);
        }
        if self.snapshot.composition.is_some()
            && !matches!(command, Command::Focus | Command::ReadSnapshot)
        {
            return if matches!(command, Command::Commit | Command::Step(_)) {
                Ok(Preparation::Reject(Rejection::Composing))
            } else {
                Err(Error::Composing)
            };
        }
        let ordinary = |command| {
            Preparation::Edit(Edit {
                command,
                committed: self.snapshot.committed,
                publication: Publication::Changed,
            })
        };
        match command {
            Command::ReadSnapshot => Ok(Preparation::Read),
            Command::Focus if self.config.disabled => Err(Error::FocusBlocked),
            Command::Focus => Ok(ordinary(EditorCommand::Focus)),
            Command::Select(s) => {
                if !s.within(&self.snapshot.draft) {
                    return Err(Error::InvalidSelection);
                }
                Ok(ordinary(EditorCommand::Select(editor_selection(*s))))
            }
            Command::Undo => Ok(ordinary(EditorCommand::Undo)),
            Command::Redo => Ok(ordinary(EditorCommand::Redo)),
            Command::ReplaceDraft {
                text,
                selection,
                undo,
                ..
            } => self.replacement(
                text.clone(),
                self.snapshot.committed,
                *selection,
                *undo,
                Publication::Changed,
            ),
            Command::ReplaceValue {
                value,
                selection,
                undo,
                ..
            } => {
                let value = value
                    .normalized(self.config.domain)
                    .ok_or(Error::InvalidValue)?;
                self.replacement(
                    format_value(value),
                    value,
                    *selection,
                    *undo,
                    Publication::Observed,
                )
            }
            Command::Commit | Command::Step(_) => {
                let stepping = matches!(command, Command::Step(_));
                let value = match self.draft_value(stepping) {
                    Ok(value) => value,
                    Err(reason) => return Ok(Preparation::Reject(reason)),
                };
                let value = match (command, value, self.snapshot.classification()) {
                    (
                        Command::Step(direction),
                        Value::Number(value),
                        Draft::Valid(_) | Draft::OutOfRange(_),
                    ) => Value::Number(
                        self.config
                            .domain
                            .advance(value, *direction)
                            .expect("finite normalized number"),
                    ),
                    _ => value,
                };
                self.replacement(
                    format_value(value),
                    value,
                    SelectionPolicy::End,
                    UndoPolicy::Record,
                    Publication::Committed(source),
                )
            }
            Command::Cancel => self.replacement(
                format_value(self.snapshot.committed),
                self.snapshot.committed,
                SelectionPolicy::End,
                UndoPolicy::Record,
                Publication::Cancelled(if source == Source::Programmatic {
                    CancelReason::Programmatic
                } else {
                    CancelReason::Escape
                }),
            ),
        }
    }
    /// Synchronize pending native changes, enforce numeric guards and reserve
    /// capacity before calling `edit`. The callback must either fail without a
    /// mutation or return the final live snapshot. Its errors are recoverable;
    /// invalid success states are fatal. It must also enforce native focus gates.
    pub fn execute(
        &mut self,
        editor: &EditorSnapshot,
        command: &Command,
        source: Source,
        edit: impl FnOnce(&EditorCommand) -> Result<EditorSnapshot, EditorError>,
    ) -> Result<Outcome, Fault> {
        let mut events: Vec<_> = self.observe(editor)?.into_iter().collect();
        let preparation = match self.prepare(command, source) {
            Ok(preparation) => preparation,
            Err(error) => {
                return Ok(Outcome {
                    events,
                    response: Response::Failed(error),
                });
            }
        };
        if matches!(preparation, Preparation::Read) {
            return Ok(Outcome {
                events,
                response: Response::Applied(self.snapshot.clone()),
            });
        }
        let revision = match self.reserve() {
            Ok(revision) => revision,
            Err(error) => {
                return Ok(Outcome {
                    events,
                    response: Response::Failed(error),
                });
            }
        };
        match preparation {
            Preparation::Read => unreachable!("read handled without reservation"),
            Preparation::Reject(reason) => {
                self.snapshot.revision = revision;
                events.push(Event::Rejected(reason, self.snapshot.clone()));
                let error = if reason == Rejection::Composing {
                    Error::Composing
                } else {
                    Error::Rejected(reason)
                };
                Ok(Outcome {
                    events,
                    response: Response::Failed(error),
                })
            }
            Preparation::Edit(prepared) => {
                let next = match edit(&prepared.command) {
                    Ok(next) => next,
                    Err(error) => {
                        return Ok(Outcome {
                            events,
                            response: Response::Failed(editor_error(error)),
                        });
                    }
                };
                if next.revision < self.editor_revision || !valid_text(&next.text) {
                    return self.fault(Fault::UnexpectedEdit);
                }
                let mut observed = self.observation(&next, prepared.committed);
                if !observed.is_valid()
                    || (prepared.publication.semantic() && !observed.is_settled())
                {
                    return self.fault(Fault::UnexpectedEdit);
                }
                // Replacements and selection must do exactly what was admitted;
                // otherwise the adapter must fault, never return false success.
                if !edit_matches(&prepared.command, &self.snapshot, &observed) {
                    return self.fault(Fault::UnexpectedEdit);
                }
                if observed != self.snapshot || prepared.publication.semantic() {
                    observed.revision = revision;
                    self.snapshot = observed.clone();
                    events.push(prepared.publication.event(observed));
                }
                self.editor_revision = next.revision;
                Ok(Outcome {
                    events,
                    response: Response::Applied(self.snapshot.clone()),
                })
            }
        }
    }
}
fn edit_matches(command: &EditorCommand, previous: &Snapshot, next: &Snapshot) -> bool {
    match command {
        EditorCommand::Replace(text, policy, _, _) => {
            let expected = match policy {
                EditorSelectionPolicy::Start => Selection { anchor: 0, head: 0 },
                EditorSelectionPolicy::End => Selection {
                    anchor: text.len() as i64,
                    head: text.len() as i64,
                },
                EditorSelectionPolicy::Select(s) => selection(s),
                EditorSelectionPolicy::Preserve => {
                    let clip = |offset: i64| {
                        let mut offset = (offset as usize).min(text.len());
                        while !text.is_char_boundary(offset) {
                            offset -= 1;
                        }
                        offset as i64
                    };
                    Selection {
                        anchor: clip(previous.selection.anchor),
                        head: clip(previous.selection.head),
                    }
                }
            };
            next.draft == *text && next.selection == expected && next.composition.is_none()
        }
        EditorCommand::Select(s) => {
            next.draft == previous.draft
                && next.selection == selection(s)
                && next.composition.is_none()
        }
        EditorCommand::Focus => {
            next.draft == previous.draft
                && next.selection == previous.selection
                && next.composition == previous.composition
                && next.focused
        }
        EditorCommand::Undo | EditorCommand::Redo => next.composition.is_none(),
        EditorCommand::Submit | EditorCommand::ReadSnapshot => false, // never prepared
    }
}

#[cfg(test)]
mod tests;
