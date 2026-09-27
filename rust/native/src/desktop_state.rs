//! Application-scoped admission before the OCaml runner is ready. Native OS
//! callbacks own this state on the main thread; no window identity is retained.
use gpuio_protocol::desktop::{
    Error, Identity, LinkBatch, MAX_LINK_BATCH_BYTES, MAX_LINK_BYTES, MAX_LINKS,
};
use std::collections::VecDeque;

#[derive(Default)]
pub struct DesktopState {
    identity: Option<Identity>,
    links: VecDeque<String>,
    bytes: usize,
    dropped: i64,
    closed: bool,
}

impl DesktopState {
    pub fn identity(&self) -> Option<&Identity> {
        self.identity.as_ref()
    }

    pub fn configure(&mut self, identity: Identity) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if self.identity.is_some() {
            return Err(Error::AlreadyConfigured);
        }
        if !identity.is_valid() {
            return Err(Error::InvalidRequest);
        }
        self.identity = Some(identity);
        Ok(())
    }

    pub fn pending(&self) -> bool {
        !self.links.is_empty() || self.dropped > 0
    }

    /// True means a transition to pending input, including an overflow-only
    /// notification. Signal once per transition; a take consumes the entire batch.
    /// Input may arrive before identity configuration. Invalid application syntax
    /// remains raw data until the OCaml application validates its scheme/routes.
    pub fn push(&mut self, link: String) -> bool {
        if self.closed {
            return false;
        }
        let was_pending = self.pending();
        if link.len() > MAX_LINK_BYTES
            || self.links.len() == MAX_LINKS
            || link.len() > MAX_LINK_BATCH_BYTES - self.bytes
        {
            self.dropped = self.dropped.saturating_add(1);
        } else {
            self.bytes += link.len();
            self.links.push_back(link);
        }
        !was_pending
    }

    /// Called only for an admitted correlated request. Its response reservation
    /// prevents loss of removed input due to the ordinary input lane being full.
    pub fn take_links(&mut self) -> Result<LinkBatch, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if self.identity.is_none() {
            return Err(Error::NotReady);
        }
        self.bytes = 0;
        Ok(LinkBatch {
            links: self.links.drain(..).collect(),
            dropped: std::mem::take(&mut self.dropped),
        })
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.identity = None;
        self.links.clear();
        self.bytes = 0;
        self.dropped = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> Identity {
        Identity {
            identifier: "com.example".into(),
            name: "Example".into(),
            schemes: vec!["example".into()],
        }
    }

    #[test]
    fn startup_input_waits_for_configuration_and_preserves_order() {
        let mut state = DesktopState::default();
        assert!(state.push("example://first".into()));
        assert!(!state.push("malformed".into()));
        assert!(!state.push("example://first".into()));
        assert_eq!(state.take_links(), Err(Error::NotReady));
        state.configure(identity()).unwrap();
        assert_eq!(state.configure(identity()), Err(Error::AlreadyConfigured));
        let batch = state.take_links().unwrap();
        assert_eq!(
            batch.links,
            ["example://first", "malformed", "example://first"]
        );
        assert_eq!(batch.dropped, 0);
        assert!(!state.pending());
        assert!(state.push("example://later".into()));
        state.close();
        assert!(!state.pending());
        assert_eq!(state.bytes, 0);
        assert!(state.identity().is_none());
        assert!(!state.push("late".into()));
        assert_eq!(state.configure(identity()), Err(Error::Closed));
        assert_eq!(state.take_links(), Err(Error::Closed));
    }

    #[test]
    fn overflow_is_bounded_counted_and_cannot_displace_accepted_input() {
        let mut state = DesktopState::default();
        state.configure(identity()).unwrap();
        for n in 0..MAX_LINKS {
            state.push(n.to_string());
        }
        state.push("newest".into());
        let batch = state.take_links().unwrap();
        assert_eq!(batch.links.len(), MAX_LINKS);
        assert_eq!(batch.links[0], "0");
        assert_eq!(batch.links[MAX_LINKS - 1], (MAX_LINKS - 1).to_string());
        assert_eq!(batch.dropped, 1);
        let size = MAX_LINK_BATCH_BYTES / MAX_LINK_BYTES;
        for _ in 0..size {
            state.push("x".repeat(MAX_LINK_BYTES));
        }
        state.push("x".into());
        state.push("x".repeat(MAX_LINK_BYTES + 1));
        assert_eq!(state.bytes, MAX_LINK_BATCH_BYTES);
        let batch = state.take_links().unwrap();
        assert!(batch.is_valid());
        assert_eq!(batch.links.len(), size);
        assert_eq!(batch.dropped, 2);
        assert_eq!(state.bytes, 0);
        assert!(state.push("x".repeat(MAX_LINK_BYTES + 1)));
        assert!(state.pending());
        let batch = state.take_links().unwrap();
        assert!(batch.links.is_empty());
        assert_eq!(batch.dropped, 1);
        state.dropped = i64::MAX;
        state.push("x".repeat(MAX_LINK_BYTES + 1));
        assert_eq!(state.take_links().unwrap().dropped, i64::MAX);
    }
}
