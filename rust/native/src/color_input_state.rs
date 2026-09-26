//! Native color policy independent of GPUI drawing. One active interaction owns
//! a full HSLA/value baseline. Callbacks carry the begin revision as identity.
//! The adapter owns editors/capture, admits each returned batch atomically and
//! restores children from snapshots; this model has no timers or event queue.
use gpuio_protocol::{color_input::*, color_value::*};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Allowed,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Selection {
    value: Value,
    channels: Hsla,
}
impl Selection {
    fn new(value: Value) -> Self {
        let channels = match value {
            Value::Empty => Hsla::new(0., 0., 0., 1.).unwrap(),
            Value::Color(color) => color.to_hsla(),
        };
        Self { value, channels }
    }
    fn channels(channels: Hsla) -> Self {
        Self {
            value: Value::Color(Rgba::of_hsla(channels)),
            channels,
        }
    }
}

pub struct State {
    config: Arc<Config>,
    seed: Value,
    committed: Selection,
    current: Selection,
    revision: i64,
    interaction: Option<Interaction>,
    draft: Option<Draft>,
    closed: bool,
    faulted: bool,
}

impl State {
    pub fn new(config: Arc<Config>, seed: Value) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if !config.allows(seed) {
            return Err(Error::InvalidValue);
        }
        Self::from_retained(config, seed)
    }
    /// The retained tree validates the seed when first admitted. Configuration
    /// may become restrictive in the same atomic transaction before mounting.
    /// Preserve that accepted history rather than silently coercing it.
    pub fn from_retained(config: Arc<Config>, seed: Value) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            config,
            seed,
            committed: Selection::new(seed),
            current: Selection::new(seed),
            revision: 0,
            interaction: None,
            draft: None,
            closed: false,
            faulted: false,
        })
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            value: self.current.value,
            committed: self.committed.value,
            channels: self.current.channels,
            interaction: self.interaction,
            draft: self.draft.clone(),
            value_allowed: self.config.allows(self.current.value),
            committed_allowed: self.config.allows(self.committed.value),
        }
    }
    fn live(&self) -> Result<(), Error> {
        if self.closed {
            Err(Error::Closed)
        } else if self.faulted {
            Err(Error::NativeFailure)
        } else {
            Ok(())
        }
    }
    fn editable(&self, access: Access) -> Result<(), Error> {
        self.live()?;
        if self.config.disabled {
            return Err(Error::Disabled);
        }
        if access == Access::Blocked {
            return Err(Error::FocusBlocked);
        }
        if self.config.read_only {
            return Err(Error::ReadOnly);
        }
        Ok(())
    }
    fn reserve(&self, count: i64) -> Result<(), Error> {
        self.live()?;
        self.revision
            .checked_add(count)
            .map(|_| ())
            .ok_or(Error::LimitExceeded)
    }
    fn advance(&mut self) -> Snapshot {
        self.revision += 1;
        let snapshot = self.snapshot();
        debug_assert!(snapshot.is_valid());
        snapshot
    }
    fn active(&self, id: i64) -> Result<InteractionKind, Error> {
        self.live()?;
        self.interaction
            .filter(|i| i.id == id)
            .map(|i| i.kind)
            .ok_or(Error::StaleInteraction)
    }
    fn check_revision(&self, guard: Option<i64>) -> Result<(), Error> {
        self.live()?;
        if guard.is_some_and(|r| r != self.revision) {
            Err(Error::StaleRevision)
        } else {
            Ok(())
        }
    }
    pub fn begin_drag(&mut self, channel: Channel, access: Access) -> Result<Event, Error> {
        self.begin(InteractionKind::Drag(channel), None, access)
    }
    /// `text` is the native field's current presentation. No synthetic edit or
    /// canonical replacement is imposed on its selection, composition or history.
    pub fn begin_text(
        &mut self,
        field: Field,
        text: String,
        access: Access,
    ) -> Result<Event, Error> {
        if !valid_draft(&text) {
            return Err(Error::InvalidDraft);
        }
        let (status, _) = classify(&self.config, field, &text, self.current.channels);
        self.begin(
            InteractionKind::Text(field),
            Some(Draft {
                text,
                status,
                composing: false,
            }),
            access,
        )
    }
    fn begin(
        &mut self,
        kind: InteractionKind,
        draft: Option<Draft>,
        access: Access,
    ) -> Result<Event, Error> {
        self.editable(access)?;
        if self.interaction.is_some() {
            return Err(Error::Busy);
        }
        self.reserve(1)?;
        self.interaction = Some(Interaction {
            id: self.revision + 1,
            kind,
        });
        self.draft = draft;
        Ok(Event::Started(self.advance()))
    }
    pub fn preview_drag(
        &mut self,
        id: i64,
        value: f64,
        access: Access,
    ) -> Result<Option<Event>, Error> {
        let InteractionKind::Drag(channel) = self.active(id)? else {
            return Err(Error::Busy);
        };
        self.editable(access)?;
        let channels = channel
            .set(self.current.channels, value)
            .ok_or(Error::InvalidValue)?;
        if !self.config.allows_channels(channels) {
            return Err(Error::InvalidValue);
        }
        let next = Selection::channels(channels);
        if next == self.current {
            return Ok(None);
        }
        self.reserve(1)?;
        self.current = next;
        Ok(Some(Event::Preview(self.advance())))
    }
    pub fn preview_text(
        &mut self,
        id: i64,
        text: String,
        composing: bool,
        access: Access,
    ) -> Result<Option<Event>, Error> {
        let InteractionKind::Text(field) = self.active(id)? else {
            return Err(Error::Busy);
        };
        self.editable(access)?;
        if !valid_draft(&text) {
            return Err(Error::InvalidDraft);
        }
        let (status, candidate) = classify(&self.config, field, &text, self.current.channels);
        let draft = Draft {
            text,
            composing,
            status,
        };
        let next = if composing {
            self.current
        } else {
            candidate.map(Selection::channels).unwrap_or(self.current)
        };
        if self.draft.as_ref() == Some(&draft) && next == self.current {
            return Ok(None);
        }
        self.reserve(1)?;
        self.current = next;
        self.draft = Some(draft);
        Ok(Some(Event::Preview(self.advance())))
    }
    pub fn finish(&mut self, id: i64, access: Access) -> Result<Event, Error> {
        let kind = self.active(id)?;
        self.editable(access)?;
        let mut next = self.current;
        if let Some(draft) = &self.draft {
            if draft.composing {
                return Err(Error::Composing);
            }
            if draft.status != DraftStatus::Valid {
                return Err(Error::InvalidDraft);
            }
            let InteractionKind::Text(field) = kind else {
                unreachable!("draft belongs to text interaction");
            };
            let (_, candidate) = classify(&self.config, field, &draft.text, self.current.channels);
            next = Selection::channels(candidate.ok_or(Error::InvalidDraft)?);
        }
        if !self.config.allows(next.value) || !self.config.allows_channels(next.channels) {
            return Err(Error::InvalidValue);
        }
        self.reserve(1)?;
        self.current = next;
        self.committed = next;
        self.interaction = None;
        self.draft = None;
        let source = match kind {
            InteractionKind::Drag(_) => Source::Pointer,
            InteractionKind::Text(_) => Source::Text,
        };
        Ok(Event::Committed(source, self.advance()))
    }
    fn cancel_reserved(&mut self, reason: CancelReason) -> Option<Event> {
        self.interaction.take()?;
        self.current = self.committed;
        self.draft = None;
        Some(Event::Cancelled(reason, self.advance()))
    }
    /// Lifecycle cancellation needs no input access. Late callbacks must continue
    /// carrying their old id, even if a new interaction has already begun.
    pub fn cancel(&mut self, reason: CancelReason) -> Result<Option<Event>, Error> {
        self.live()?;
        if self.interaction.is_none() {
            return Ok(None);
        }
        self.reserve(1)?;
        Ok(self.cancel_reserved(reason))
    }
    pub fn set(&mut self, value: Value, guard: Option<i64>) -> Result<Vec<Event>, Error> {
        self.check_revision(guard)?;
        if !self.config.allows(value) {
            return Err(Error::InvalidValue);
        }
        self.install(Selection::new(value), None, CancelReason::Programmatic)
    }
    pub fn reset(&mut self, guard: Option<i64>) -> Result<Vec<Event>, Error> {
        self.set(self.seed, guard)
    }
    fn install(
        &mut self,
        next: Selection,
        source: Option<Source>,
        reason: CancelReason,
    ) -> Result<Vec<Event>, Error> {
        self.reserve(if self.interaction.is_some() { 2 } else { 1 })?;
        let mut events = Vec::with_capacity(2);
        if let Some(event) = self.cancel_reserved(reason) {
            events.push(event);
        }
        self.current = next;
        self.committed = next;
        let snapshot = self.advance();
        events.push(match source {
            Some(source) => Event::Committed(source, snapshot),
            None => Event::Observed(snapshot),
        });
        Ok(events)
    }
    /// Discrete keyboard/AX adjustments start from the committed baseline after
    /// interrupting any drag/text edit. Invalid attempts never cancel that edit.
    pub fn set_channel(
        &mut self,
        channel: Channel,
        value: f64,
        source: Source,
        access: Access,
    ) -> Result<Vec<Event>, Error> {
        self.editable(access)?;
        let channels = channel
            .set(self.committed.channels, value)
            .ok_or(Error::InvalidValue)?;
        if !self.config.allows_channels(channels) {
            return Err(Error::InvalidValue);
        }
        self.install(
            Selection::channels(channels),
            Some(source),
            CancelReason::Interrupted,
        )
    }
    pub fn choose(
        &mut self,
        value: Value,
        source: Source,
        access: Access,
    ) -> Result<Vec<Event>, Error> {
        self.editable(access)?;
        if !self.config.allows(value) {
            return Err(Error::InvalidValue);
        }
        self.install(
            Selection::new(value),
            Some(source),
            CancelReason::Interrupted,
        )
    }
    pub fn configure(&mut self, config: Arc<Config>) -> Result<Vec<Event>, Error> {
        self.live()?;
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if config == self.config {
            self.config = config;
            return Ok(Vec::new());
        }
        let reason = if config.alpha_policy != self.config.alpha_policy
            || config.allow_empty != self.config.allow_empty
        {
            Some(CancelReason::ConfigurationChanged)
        } else if config.disabled {
            Some(CancelReason::Disabled)
        } else if config.read_only {
            Some(CancelReason::ReadOnly)
        } else {
            None
        };
        let cancel = reason.is_some() && self.interaction.is_some();
        self.reserve(if cancel { 2 } else { 1 })?;
        let mut events = Vec::with_capacity(2);
        if cancel {
            events.push(self.cancel_reserved(reason.unwrap()).unwrap());
        }
        // Historical values survive incompatible alpha/empty policy changes.
        // Labels and palette changes do not discard editor drafts or drags.
        self.config = config;
        events.push(Event::Observed(self.advance()));
        Ok(events)
    }
    /// Called when required output cannot be admitted. No later input/command
    /// may report success from a model whose observations were lost.
    pub fn fault(&mut self) {
        self.faulted = true;
    }
    /// Native resources live in the adapter. After close no mutation succeeds.
    pub fn close(&mut self) {
        self.current = self.committed;
        self.interaction = None;
        self.draft = None;
        self.closed = true;
    }
}

#[cfg(test)]
#[path = "color_input_state_test.rs"]
mod tests;
