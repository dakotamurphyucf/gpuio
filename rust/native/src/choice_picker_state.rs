//! Validated popup transitions, independent of GPUI focus/layout and transport.
//!
//! Each call returns at most three ordered events and retains no event backlog.
//! The native adapter must supply current eligibility, honor IME key consumption,
//! and fence delivery by accepted node/handler generations. This state alone is
//! not a mounted widget or a transport lifetime fence.
use gpuio_protocol::choice_picker::{
    Collection, Config, Event, OpenReason, OpenState, Query, Request, Search, Selection,
    Visibility, VisibilityReason,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    Invalid,
}

pub struct State {
    config: Arc<Config>,
    eligible: bool,
    open: bool,
    query_node: Option<gpuio_protocol::NodeId>,
}

impl State {
    pub fn new(
        config: Arc<Config>,
        eligible: bool,
        query_node: Option<gpuio_protocol::NodeId>,
    ) -> Result<Self, ConfigError> {
        if !Self::valid_configuration(&config, query_node) {
            return Err(ConfigError::Invalid);
        }
        let (OpenState::Managed(preferred) | OpenState::Controlled(preferred)) = config.open_state;
        let open = eligible && !config.disabled && preferred;
        Ok(Self {
            config,
            eligible,
            open,
            query_node,
        })
    }

    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Emit when a live observer subscribes, including while closed.
    pub fn snapshot(&self) -> Event {
        Event::Visibility(Visibility::Snapshot(self.open))
    }

    fn available(&self) -> bool {
        self.eligible && !self.config.disabled
    }

    fn valid_configuration(config: &Config, query_node: Option<gpuio_protocol::NodeId>) -> bool {
        config.is_valid() && (config.search != Search::None) == query_node.is_some()
    }

    fn valid_query(&self, query: Option<&Query>) -> bool {
        match (self.query_node, query) {
            (None, None) => true,
            (Some(node), Some(query)) => {
                node == query.node && query.is_valid() && query.snapshot.composition.is_none()
            }
            (None, Some(_)) | (Some(_), None) => false,
        }
    }

    fn change_visibility(&mut self, open: bool, reason: VisibilityReason) -> Option<Event> {
        if self.open == open {
            return None;
        }
        self.open = open;
        Some(Event::Visibility(Visibility::Changed(open, reason)))
    }

    /// Validate before mutation. Managed initial state is read only at creation;
    /// switching from Controlled to Managed retains actual visibility.
    pub fn configure(
        &mut self,
        config: Arc<Config>,
        eligible: bool,
        query_node: Option<gpuio_protocol::NodeId>,
    ) -> Result<Vec<Event>, ConfigError> {
        if !Self::valid_configuration(&config, query_node) {
            return Err(ConfigError::Invalid);
        }
        self.config = config;
        self.eligible = eligible;
        self.query_node = query_node;
        let available = self.available();
        let open = available
            && match self.config.open_state {
                OpenState::Controlled(open) => open,
                OpenState::Managed(_) => self.open,
            };
        let reason = if available {
            VisibilityReason::Application
        } else {
            VisibilityReason::Unavailable
        };
        Ok(self.change_visibility(open, reason).into_iter().collect())
    }

    /// An unaccepted controlled request is retryable by later gestures. Requests
    /// are not coalesced and never optimistically change controlled visibility.
    pub fn request_open(&mut self, open: bool, reason: OpenReason) -> Vec<Event> {
        if !self.available() || !reason.allows(open) || self.open == open {
            return Vec::new();
        }
        let mut events = vec![Event::OpenRequested(open, reason)];
        if let OpenState::Managed(_) = self.config.open_state {
            events.extend(self.change_visibility(open, VisibilityReason::Interaction(reason)));
        }
        events
    }

    /// Current catalog/availability checks precede every intent. The application
    /// owns committed selection; rapid multiple activations remain distinct toggles.
    pub fn activate(&mut self, id: &str, composing: bool, query: Option<&Query>) -> Vec<Event> {
        if !self.open || !self.available() || composing || !self.valid_query(query) {
            return Vec::new();
        }
        let enabled = |items: &[gpuio_protocol::choice_picker::Item]| {
            items.iter().any(|item| item.id == id && !item.disabled)
        };
        let can_select = match &self.config.options {
            Collection::Flat(items) => enabled(items),
            Collection::Grouped(groups) => groups.iter().any(|group| enabled(&group.items)),
        };
        if !can_select {
            return Vec::new();
        }
        match &self.config.selected {
            Selection::Single(_) => {
                let mut events = vec![Event::SelectionRequested(
                    Request::Select(id.into()),
                    query.cloned(),
                )];
                events.extend(self.request_open(false, OpenReason::Selection));
                events
            }
            Selection::Multiple(_) => vec![Event::SelectionRequested(
                Request::Toggle(id.into()),
                query.cloned(),
            )],
        }
    }

    /// Explicit clear preserves mode and popup state, including while closed.
    pub fn clear(&self, composing: bool, query: Option<&Query>) -> Vec<Event> {
        if self.available() && self.config.clearable && !composing && self.valid_query(query) {
            vec![Event::SelectionRequested(Request::Clear, query.cloned())]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
#[path = "choice_picker_state_test.rs"]
mod tests;
