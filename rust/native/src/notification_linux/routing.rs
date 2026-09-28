//! Logical lifetimes and physical daemon leases have different end points.
use crate::notification_state::{Signal as LifetimeSignal, State, Token};
use gpuio_portal::notifications::Signal;
use gpuio_protocol::notification::{Error, MAX_LIVE, Receipt};
use std::collections::BTreeMap;

const MAX_EARLY: usize = 64;
#[derive(Default)]
pub(super) struct Routing {
    pub state: State,
    leases: BTreeMap<u32, Receipt>,
    early: Vec<Signal>,
    replacing: u32,
    replacement_closed: bool,
}
fn native_id(signal: &Signal) -> u32 {
    match signal {
        Signal::Activated(id) | Signal::Closed(id, _) => *id,
        Signal::Action { native_id, .. } => *native_id,
    }
}
impl Routing {
    pub fn can_post(&self) -> bool {
        self.leases.len() < MAX_LIVE
    }
    pub fn native_id(&self, receipt: &Receipt) -> Result<u32, Error> {
        self.leases
            .iter()
            .find_map(|(id, owner)| (owner == receipt).then_some(*id))
            .ok_or(Error::Stale)
    }
    pub fn all_ids(&self) -> Vec<u32> {
        self.leases.keys().copied().collect()
    }
    pub fn cleanup_ids(&self) -> Vec<u32> {
        self.leases
            .iter()
            .filter_map(|(id, receipt)| self.state.receipt(receipt.id).is_none().then_some(*id))
            .collect()
    }
    pub fn released(&mut self, id: u32) {
        self.leases.remove(&id);
    }
    pub fn begin_show(&mut self, replaces: u32) {
        self.early.clear();
        self.replacing = replaces;
        self.replacement_closed = false;
    }
    pub fn bind(&mut self, id: u32, token: &Token) -> Result<(), Error> {
        if id == self.replacing && self.replacement_closed {
            return Ok(());
        }
        if let Some(owner) = self.leases.get(&id) {
            if owner != &token.receipt {
                return Err(Error::NativeFailure);
            }
        } else if !self.can_post() {
            return Err(Error::Busy);
        }
        self.leases.insert(id, token.receipt.clone());
        for signal in std::mem::take(&mut self.early) {
            if native_id(&signal) == id {
                self.signal(signal, None)?;
            }
        }
        Ok(())
    }
    pub fn signal(&mut self, signal: Signal, pending_new: Option<&Token>) -> Result<bool, Error> {
        let native = native_id(&signal);
        let Some(receipt) = self.leases.get(&native).cloned() else {
            if let Some(token) = pending_new {
                // Custom keys from another receipt/revision cannot concern this
                // submission. Default/Closed carry only a daemon-scoped ID.
                if matches!(&signal, Signal::Action { receipt, revision, .. }
                    if *receipt != token.receipt.id || *revision != token.serial)
                {
                    return Ok(false);
                }
                if self.early.len() >= MAX_EARLY {
                    return Err(Error::Busy);
                }
                self.early.push(signal);
            }
            return Ok(false);
        };
        let signal = match signal {
            Signal::Activated(_) => LifetimeSignal::Activated,
            Signal::Action {
                receipt: owner,
                revision,
                id,
                ..
            } => {
                if owner != receipt.id {
                    return Ok(false);
                }
                LifetimeSignal::Action { revision, id }
            }
            Signal::Closed(_, reason) => {
                if native == self.replacing {
                    self.replacement_closed = true;
                }
                self.released(native);
                LifetimeSignal::Closed(reason)
            }
        };
        Ok(self.state.signal(&receipt, signal))
    }
}

#[cfg(test)]
mod tests;
