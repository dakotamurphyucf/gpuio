//! Bounded asynchronous OS operations. Native completion blocks may run on any
//! thread; they only publish owned responses, never access GPUI or call OCaml.
use gpuio_protocol::notification::{Error, Response};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

const MAX_PENDING: usize = 16;
type Completion = Box<dyn FnOnce(Response) + Send>;

#[derive(Default)]
struct Inner {
    closed: bool,
    sequence: u64,
    pending: BTreeMap<i64, (u64, Completion)>,
}

#[derive(Default)]
pub(crate) struct Operations(Mutex<Inner>);

#[derive(Clone)]
pub(crate) struct Ticket {
    owner: Weak<Operations>,
    id: i64,
    token: u64,
}

impl Operations {
    pub fn admit(
        self: &Arc<Self>,
        id: i64,
        complete: impl FnOnce(Response) + Send + 'static,
    ) -> Result<Ticket, Error> {
        let mut inner = self.0.lock().expect("notification operations poisoned");
        if inner.closed {
            return Err(Error::Closed);
        }
        if id <= 0 || inner.pending.contains_key(&id) {
            return Err(Error::InvalidRequest);
        }
        if inner.pending.len() >= MAX_PENDING {
            return Err(Error::Busy);
        }
        let token = inner.sequence.checked_add(1).ok_or(Error::Busy)?;
        inner.sequence = token;
        inner.pending.insert(id, (token, Box::new(complete)));
        Ok(Ticket {
            owner: Arc::downgrade(self),
            id,
            token,
        })
    }

    pub fn close(&self) {
        let pending = {
            let mut inner = self.0.lock().expect("notification operations poisoned");
            inner.closed = true;
            std::mem::take(&mut inner.pending)
        };
        for (_, (_, complete)) in pending {
            complete(Response::Failed(Error::Closed));
        }
    }
}

impl Ticket {
    pub fn finish(&self, response: Response) {
        if let Some(owner) = self.owner.upgrade() {
            let complete = {
                let mut inner = owner.0.lock().expect("notification operations poisoned");
                if inner
                    .pending
                    .get(&self.id)
                    .is_some_and(|(token, _)| *token == self.token)
                {
                    inner.pending.remove(&self.id).map(|(_, complete)| complete)
                } else {
                    None
                }
            };
            if let Some(complete) = complete {
                complete(response);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_completion_shutdown_and_late_callbacks_are_exactly_once() {
        let operations = Arc::new(Operations::default());
        let results = Arc::new(Mutex::new(Vec::new()));
        let tickets = (1..=16)
            .map(|id| {
                let results = results.clone();
                operations
                    .admit(id, move |response| {
                        results.lock().unwrap().push((id, response))
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(matches!(operations.admit(17, |_| ()), Err(Error::Busy)));
        assert!(matches!(
            operations.admit(1, |_| ()),
            Err(Error::InvalidRequest)
        ));
        tickets[0].finish(Response::Replaced);
        tickets[0].finish(Response::Failed(Error::NativeFailure));
        let ticket = operations.admit(17, |_| ()).unwrap();
        operations.close();
        operations.close();
        for ticket in tickets {
            ticket.finish(Response::Replaced);
        }
        ticket.finish(Response::Replaced);
        assert!(matches!(operations.admit(18, |_| ()), Err(Error::Closed)));
        let results = results.lock().unwrap();
        assert_eq!(results.len(), 16);
        assert_eq!(results[0], (1, Response::Replaced));
        assert!(
            results[1..]
                .iter()
                .all(|(_, response)| *response == Response::Failed(Error::Closed))
        );
    }

    #[test]
    fn completion_can_reenter_and_cross_threads_without_holding_the_registry_lock() {
        let operations = Arc::new(Operations::default());
        let reentrant = operations.clone();
        let ticket = operations.admit(1, move |_| reentrant.close()).unwrap();
        std::thread::spawn(move || ticket.finish(Response::Replaced))
            .join()
            .unwrap();
        assert!(matches!(operations.admit(2, |_| ()), Err(Error::Closed)));
    }

    #[test]
    fn retired_callback_cannot_complete_a_reused_correlation() {
        let operations = Arc::new(Operations::default());
        let old = operations.admit(1, |_| ()).unwrap();
        old.finish(Response::Replaced);
        let results = Arc::new(Mutex::new(Vec::new()));
        let observed = results.clone();
        let next = operations
            .admit(1, move |result| observed.lock().unwrap().push(result))
            .unwrap();
        old.finish(Response::Failed(Error::NativeFailure));
        assert!(results.lock().unwrap().is_empty());
        next.finish(Response::DismissRequested);
        assert_eq!(*results.lock().unwrap(), vec![Response::DismissRequested]);
    }
}
