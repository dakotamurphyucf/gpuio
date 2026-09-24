//! Active-time accounting for one native notification. The adapter owns the
//! cancellable deadline task; a closed session can never be rearmed by metadata.
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Plan {
    Idle,
    After(Duration),
    Expired,
}
#[derive(Debug)]
pub(super) struct Clock {
    configured: Option<Duration>,
    remaining: Option<Duration>,
    running_since: Option<Instant>,
    closed: bool,
}
impl Clock {
    pub(super) fn new(timeout: Option<Duration>) -> Self {
        Self {
            configured: timeout,
            remaining: timeout,
            running_since: None,
            closed: false,
        }
    }
    fn consume(&mut self, now: Instant) {
        if let Some(since) = self.running_since {
            if let Some(remaining) = &mut self.remaining {
                *remaining = remaining.saturating_sub(now.saturating_duration_since(since));
            }
            self.running_since = Some(now);
        }
    }
    fn plan(&self) -> Plan {
        if self.closed {
            return Plan::Idle;
        }
        match self.remaining {
            Some(Duration::ZERO) => Plan::Expired,
            Some(remaining) if self.running_since.is_some() => Plan::After(remaining),
            Some(_) | None => Plan::Idle,
        }
    }
    pub(super) fn update(&mut self, timeout: Option<Duration>, paused: bool, now: Instant) -> Plan {
        if self.closed {
            return Plan::Idle;
        }
        self.consume(now);
        if self.configured != timeout {
            self.configured = timeout;
            self.remaining = timeout;
        }
        self.running_since = (!paused && self.remaining.is_some()).then_some(now);
        self.plan()
    }
    pub(super) fn advance(&mut self, now: Instant) -> Plan {
        self.consume(now);
        self.plan()
    }
    pub(super) fn is_closed(&self) -> bool {
        self.closed
    }
    pub(super) fn close(&mut self) -> bool {
        self.running_since = None;
        self.remaining = None;
        !std::mem::replace(&mut self.closed, true)
    }
}
