//! Application-scoped admission before the OCaml runner is ready. Native OS
//! callbacks use a short transport-owned mutex; no window identity is retained.
use gpuio_protocol::desktop::{
    Error, Identity, LinkBatch, MAX_LINK_BATCH_BYTES, MAX_LINK_BYTES, MAX_LINKS,
};
use std::collections::VecDeque;

#[derive(Default)]
pub struct DesktopState {
    identity: Option<Identity>,
    prepared: Option<Identity>,
    failure: Option<Error>,
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
        if self
            .prepared
            .as_ref()
            .is_some_and(|expected| expected != &identity)
        {
            return Err(Error::InvalidRequest);
        }
        self.identity = Some(identity);
        Ok(())
    }

    pub fn prepare(&mut self, identity: Identity) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if self.prepared.is_some() || self.identity.is_some() {
            return Err(Error::AlreadyConfigured);
        }
        if !identity.is_valid() {
            return Err(Error::InvalidRequest);
        }
        self.prepared = Some(identity);
        Ok(())
    }

    pub fn fail(&mut self, error: Error) -> bool {
        if self.closed {
            return false;
        }
        let notify = !self.pending();
        self.failure = Some(error);
        notify
    }

    pub fn pending(&self) -> bool {
        !self.links.is_empty() || self.dropped > 0 || self.failure.is_some()
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

    /// Forwarded launches are acknowledged only when their entire batch fits.
    /// Refusal changes neither accepted input nor the OS overflow counter. Unlike
    /// unacknowledged OS callbacks, callers can observe this backpressure directly.
    pub fn try_push_batch(&mut self, links: Vec<String>) -> Result<bool, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if links.len() > MAX_LINKS
            || links
                .iter()
                .any(|s| s.len() > MAX_LINK_BYTES || s.contains('\0'))
        {
            return Err(Error::InvalidRequest);
        }
        let bytes = links.iter().map(String::len).sum::<usize>();
        if bytes > MAX_LINK_BATCH_BYTES {
            return Err(Error::InvalidRequest);
        }
        if links.len() > MAX_LINKS - self.links.len() || bytes > MAX_LINK_BATCH_BYTES - self.bytes {
            return Err(Error::Busy);
        }
        let notify = !links.is_empty() && !self.pending();
        self.bytes += bytes;
        self.links.extend(links);
        Ok(notify)
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
        if let Some(error) = self.failure.take() {
            return Err(error);
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
        self.prepared = None;
        self.failure = None;
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

    #[test]
    fn prepared_identity_and_bus_failure_survive_readiness_without_reordering() {
        let mut state = DesktopState::default();
        state.prepare(identity()).unwrap();
        assert_eq!(state.prepare(identity()), Err(Error::AlreadyConfigured));
        let mut other = identity();
        other.identifier = "com.other".into();
        assert_eq!(state.configure(other), Err(Error::InvalidRequest));
        state.try_push_batch(vec!["early".into()]).unwrap();
        assert!(!state.fail(Error::Unavailable));
        assert_eq!(state.take_links(), Err(Error::NotReady));
        state.configure(identity()).unwrap();
        assert_eq!(state.take_links(), Err(Error::Unavailable));
        assert_eq!(state.take_links().unwrap().links, ["early"]);
        assert!(!state.pending());
        state.close();
        assert!(!state.fail(Error::Unavailable));
        assert!(!state.pending());
    }

    #[test]
    fn forwarded_batches_are_atomic_and_do_not_report_false_os_overflow() {
        let mut state = DesktopState::default();
        state.configure(identity()).unwrap();
        assert_eq!(state.try_push_batch(vec![]), Ok(false));
        assert_eq!(
            state.try_push_batch(vec!["same".into(); MAX_LINKS - 1]),
            Ok(true)
        );
        assert_eq!(
            state.try_push_batch(vec!["refused".into(); 2]),
            Err(Error::Busy)
        );
        assert_eq!(state.try_push_batch(vec!["last".into()]), Ok(false));
        let batch = state.take_links().unwrap();
        assert_eq!(batch.links.len(), MAX_LINKS);
        assert_eq!(batch.links.last().unwrap(), "last");
        assert_eq!(batch.dropped, 0);
        assert_eq!(
            state.try_push_batch(vec!["bad\0link".into()]),
            Err(Error::InvalidRequest)
        );
        assert!(!state.pending());
        let full = vec!["x".repeat(MAX_LINK_BYTES); MAX_LINK_BATCH_BYTES / MAX_LINK_BYTES];
        assert_eq!(state.try_push_batch(full), Ok(true));
        assert_eq!(state.try_push_batch(vec!["x".into()]), Err(Error::Busy));
        assert_eq!(state.take_links().unwrap().dropped, 0);
        state.close();
        assert_eq!(state.try_push_batch(vec![]), Err(Error::Closed));
    }
}
