//! Bounded notification lifetime state. Platform adapters hold a short mutex
//! around these transitions and perform OS calls only after releasing it.
use gpuio_protocol::notification::{
    ClosedReason, Content, Error, Event, MAX_LIVE, MAX_PENDING, Receipt, valid_tag,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::atomic::{AtomicI64, Ordering},
};

static NEXT_ID: AtomicI64 = AtomicI64::new(1);
fn next_id() -> Result<i64, Error> {
    NEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| Error::Busy)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub receipt: Receipt,
    pub serial: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Signal {
    Activated,
    Action { revision: i64, id: String },
    Closed(ClosedReason),
}
#[derive(Clone, Debug)]
struct Current {
    revision: i64,
    content: Content,
}
#[derive(Clone, Debug)]
enum Operation {
    Show(Content),
    Dismiss,
}
#[derive(Clone, Debug)]
struct Pending {
    receipt: Receipt,
    operation: Operation,
}
struct Entry {
    receipt: Receipt,
    current: Option<Current>,
    pending: Option<i64>,
    early: Option<Signal>,
    retiring: bool,
    os_closed: bool,
}

/// Completion can require removal of an OS artifact whose lifetime ended while
/// the request was in flight. Cleanup must use the exact OS identifier captured
/// by the operation, never a tag lookup that might find a later notification.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Completion {
    pub result: Result<(), Error>,
    pub cleanup: bool,
    pub notify: bool,
}

#[derive(Default)]
pub(crate) struct State {
    entries: BTreeMap<i64, Entry>,
    tags: BTreeMap<String, i64>,
    operations: BTreeMap<i64, Pending>,
    events: VecDeque<Event>,
    failure: Option<Error>,
    next_operation: i64,
    closed: bool,
}
impl State {
    pub(crate) fn receipt(&self, id: i64) -> Option<Receipt> {
        self.entries.get(&id).map(|entry| entry.receipt.clone())
    }
    #[cfg(any(test, target_os = "macos"))]
    pub(crate) fn receipts(&self) -> Vec<Receipt> {
        self.entries
            .values()
            .map(|entry| entry.receipt.clone())
            .collect()
    }
    /// Only current and admitted candidate content, bounded by live + pending.
    /// Native category registration must not retain historical action sets.
    #[cfg(any(test, target_os = "macos"))]
    pub(crate) fn contents(&self) -> Vec<(Token, Content)> {
        let mut contents = Vec::new();
        for entry in self.entries.values() {
            if !entry.retiring {
                if let Some(current) = &entry.current {
                    contents.push((
                        Token {
                            receipt: entry.receipt.clone(),
                            serial: current.revision,
                        },
                        current.content.clone(),
                    ));
                }
                if let Some((
                    serial,
                    Pending {
                        operation: Operation::Show(content),
                        ..
                    },
                )) = entry
                    .pending
                    .and_then(|serial| self.operations.get(&serial).map(|op| (serial, op)))
                {
                    contents.push((
                        Token {
                            receipt: entry.receipt.clone(),
                            serial,
                        },
                        content.clone(),
                    ));
                }
            }
        }
        contents
    }
    fn admission(&self) -> Result<i64, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if self.operations.len() >= MAX_PENDING {
            return Err(Error::Busy);
        }
        self.next_operation.checked_add(1).ok_or(Error::Busy)
    }
    fn entry(&self, receipt: &Receipt) -> Result<&Entry, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if !receipt.is_valid() {
            return Err(Error::InvalidRequest);
        }
        self.entries
            .get(&receipt.id)
            .filter(|e| &e.receipt == receipt)
            .ok_or(Error::Stale)
    }
    pub(crate) fn post(&mut self, tag: String, content: Content) -> Result<Token, Error> {
        if !valid_tag(&tag) || !content.is_valid() {
            return Err(Error::InvalidRequest);
        }
        let serial = self.admission()?;
        if self.tags.contains_key(&tag) || self.entries.len() + self.events.len() >= MAX_LIVE {
            return Err(Error::Busy);
        }
        let receipt = Receipt {
            id: next_id()?,
            tag,
        };
        self.tags.insert(receipt.tag.clone(), receipt.id);
        self.entries.insert(
            receipt.id,
            Entry {
                receipt: receipt.clone(),
                current: None,
                pending: Some(serial),
                early: None,
                retiring: false,
                os_closed: false,
            },
        );
        self.record(serial, receipt, Operation::Show(content))
    }
    fn record(
        &mut self,
        serial: i64,
        receipt: Receipt,
        operation: Operation,
    ) -> Result<Token, Error> {
        self.next_operation = serial;
        self.operations.insert(
            serial,
            Pending {
                receipt: receipt.clone(),
                operation,
            },
        );
        Ok(Token { receipt, serial })
    }
    pub(crate) fn replace(&mut self, receipt: &Receipt, content: Content) -> Result<Token, Error> {
        if !content.is_valid() {
            return Err(Error::InvalidRequest);
        }
        let serial = self.admission()?;
        let entry = self.entry(receipt)?;
        if entry.retiring {
            return Err(Error::Stale);
        }
        if entry.pending.is_some() {
            return Err(Error::Busy);
        }
        self.entries.get_mut(&receipt.id).unwrap().pending = Some(serial);
        self.record(serial, receipt.clone(), Operation::Show(content))
    }
    pub(crate) fn dismiss(&mut self, receipt: &Receipt) -> Result<Token, Error> {
        let serial = self.admission()?;
        if self.entry(receipt)?.pending.is_some() {
            return Err(Error::Busy);
        }
        let entry = self.entries.get_mut(&receipt.id).unwrap();
        entry.retiring = true;
        entry.early = None;
        entry.pending = Some(serial);
        self.record(serial, receipt.clone(), Operation::Dismiss)
    }
    fn remove(&mut self, id: i64) -> Option<Entry> {
        let entry = self.entries.remove(&id)?;
        self.tags.remove(&entry.receipt.tag);
        Some(entry)
    }
    #[cfg(test)]
    pub(crate) fn revision(&self, receipt: &Receipt) -> Option<i64> {
        self.entry(receipt)
            .ok()?
            .current
            .as_ref()
            .map(|c| c.revision)
    }
    /// Exactly one matching completion consumes a token. Wrong/cross-state tokens
    /// must not consume an unrelated pending operation with the same serial.
    pub(crate) fn complete(&mut self, token: &Token, result: Result<(), Error>) -> Completion {
        if !self
            .operations
            .get(&token.serial)
            .is_some_and(|op| op.receipt == token.receipt)
        {
            return Completion {
                result: Err(if self.closed {
                    Error::Closed
                } else {
                    Error::Stale
                }),
                cleanup: false,
                notify: false,
            };
        }
        let operation = self.operations.remove(&token.serial).unwrap().operation;
        if self.closed {
            return Completion {
                result: Err(Error::Closed),
                cleanup: matches!(operation, Operation::Show(_)) && result.is_ok(),
                notify: false,
            };
        }
        let Some(entry) = self.entries.get_mut(&token.receipt.id) else {
            return Completion {
                result: Err(Error::Stale),
                cleanup: matches!(operation, Operation::Show(_)) && result.is_ok(),
                notify: false,
            };
        };
        entry.pending = None;
        let mut notify = false;
        match operation {
            Operation::Show(content) => {
                if result.is_ok() {
                    entry.current = Some(Current {
                        revision: token.serial,
                        content,
                    });
                    let early = entry.early.take();
                    if let Some(signal) = early {
                        notify = self.signal(&token.receipt, signal);
                    }
                } else if entry.current.is_none() {
                    self.remove(token.receipt.id);
                } else {
                    entry.early = None;
                }
            }
            Operation::Dismiss => {
                if result.is_ok() || entry.os_closed {
                    self.remove(token.receipt.id);
                    return Completion {
                        result: Ok(()),
                        cleanup: false,
                        notify: false,
                    };
                }
                // Failed dismissal remains retired and consumes its bounded slot.
                // Retrying dismissal is permitted; actions/replacement are not.
            }
        }
        Completion {
            result,
            cleanup: false,
            notify,
        }
    }
    fn matches(content: &Content, signal: &Signal) -> bool {
        match signal {
            Signal::Action { id, .. } => content.actions.iter().any(|a| &a.id == id),
            Signal::Activated | Signal::Closed(_) => true,
        }
    }
    /// Default activation belongs to a lifetime, not a content revision. Named
    /// actions additionally require the revision that minted their transport key.
    /// Before submission completes, retain at most one candidate terminal event.
    pub(crate) fn signal(&mut self, receipt: &Receipt, signal: Signal) -> bool {
        let Ok(entry) = self.entry(receipt) else {
            return false;
        };
        if entry.retiring {
            if matches!(signal, Signal::Closed(_)) {
                if entry.pending.is_none() {
                    self.remove(receipt.id);
                } else {
                    self.entries.get_mut(&receipt.id).unwrap().os_closed = true;
                }
            }
            return false;
        }
        if entry.early.is_some() {
            return false;
        }
        let current = entry.current.as_ref().is_some_and(|current| {
            let revision_matches = match &signal {
                Signal::Action { revision, .. } => *revision == current.revision,
                _ => true,
            };
            revision_matches && Self::matches(&current.content, &signal)
        });
        if !current {
            let pending = entry
                .pending
                .and_then(|serial| self.operations.get(&serial).map(|op| (serial, op)));
            let candidate = pending.is_some_and(|(serial, op)| match &op.operation {
                Operation::Show(content) => {
                    let revision_matches = match &signal {
                        Signal::Action { revision, .. } => *revision == serial,
                        _ => entry.current.is_none(),
                    };
                    revision_matches && Self::matches(content, &signal)
                }
                Operation::Dismiss => false,
            });
            if candidate {
                self.entries.get_mut(&receipt.id).unwrap().early = Some(signal);
            }
            return false;
        }
        let entry = self.remove(receipt.id).unwrap();
        let event = match signal {
            Signal::Activated => Event::Activated(entry.receipt),
            Signal::Action { id, .. } => Event::Action(entry.receipt, id),
            Signal::Closed(reason) => Event::Closed(entry.receipt, reason),
        };
        let notify = !self.pending();
        self.events.push_back(event);
        notify
    }
    pub(crate) fn pending(&self) -> bool {
        !self.events.is_empty() || self.failure.is_some()
    }
    pub(crate) fn take(&mut self) -> Result<Vec<Event>, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let mut events: Vec<_> = self.events.drain(..).collect();
        if let Some(error) = self.failure.take() {
            events.push(Event::Failed(error));
        }
        Ok(events)
    }
    /// Loss of the native service retires every live receipt. Already-admitted
    /// terminal events retain their original identity; there is no implicit replay.
    #[cfg(any(test, target_os = "linux"))]
    pub(crate) fn fail(&mut self, error: Error) -> bool {
        if self.closed {
            return false;
        }
        let notify = !self.pending();
        self.entries.clear();
        self.tags.clear();
        self.failure = Some(error);
        notify
    }
    pub(crate) fn close(&mut self) {
        self.closed = true;
        self.entries.clear();
        self.tags.clear();
        self.events.clear();
        self.failure = None;
    }
}

#[cfg(test)]
mod tests;
