//! Single-flight application stepping for the retained numeric model.
//!
//! The mounted adapter must additionally fence the window/node generation and
//! current interaction gate. No task or UI entity is owned here. Request tokens
//! never repeat within this model's lifetime, including after cancellation.
use super::*;
use gpuio_protocol::numeric::Direction;

pub use gpuio_protocol::number_input::StepRequest as Request;
#[derive(Debug)]
pub struct RequestOutcome {
    pub events: Vec<Event>,
    pub request: Result<Request, Error>,
}
#[derive(Clone, Copy)]
struct Stamp {
    id: i64,
    revision: i64,
    source: Source,
}
#[derive(Default)]
pub(super) struct Pending {
    last_id: i64,
    stamp: Option<Stamp>,
}

impl State {
    pub fn matches_step_request(&self, id: i64, revision: i64) -> bool {
        self.step_requests
            .stamp
            .is_some_and(|stamp| stamp.id == id && stamp.revision == revision)
    }
    pub fn has_step_request(&self) -> bool {
        self.step_requests.stamp.is_some()
    }
    /// Call on policy-mode changes, deactivation, removal or
    /// closure. Ordinary editing/config changes also invalidate automatically.
    pub fn cancel_step_request(&mut self) -> bool {
        self.step_requests.stamp.take().is_some()
    }
    /// Request a caller-selected value without performing the default step.
    /// Invalid/incomplete/composing drafts retain ordinary step rejection rules.
    /// At most one request can be pending; additional requests return Busy.
    /// Reading state does not cancel a request. No numeric revision is consumed
    /// merely to send an intent; the independent request id fences duplicates.
    pub fn request_application_step(
        &mut self,
        editor: &EditorSnapshot,
        direction: Direction,
        source: Source,
    ) -> Result<RequestOutcome, Fault> {
        let events: Vec<_> = self.observe(editor)?.into_iter().collect();
        let fail = |events, error| RequestOutcome {
            events,
            request: Err(error),
        };
        if self.has_step_request() {
            return Ok(fail(events, Error::Busy));
        }
        match self.prepare(&Command::Step(direction), source) {
            Err(error) => return Ok(fail(events, error)),
            Ok(Preparation::Reject(reason)) => {
                let outcome =
                    self.apply_preparation(events, Preparation::Reject(reason), |_| {
                        unreachable!("rejection cannot edit")
                    })?;
                let Response::Failed(error) = outcome.response else {
                    unreachable!("rejection returns a failed response")
                };
                return Ok(fail(outcome.events, error));
            }
            Ok(Preparation::Edit(_)) => (),
            Ok(Preparation::Read) => unreachable!("Step cannot prepare a read"),
        }
        let Some(id) = self.step_requests.last_id.checked_add(1) else {
            return Ok(fail(events, Error::LimitExceeded));
        };
        self.step_requests.last_id = id;
        self.step_requests.stamp = Some(Stamp {
            id,
            revision: self.snapshot.revision,
            source,
        });
        Ok(RequestOutcome {
            events,
            request: Ok(Request {
                id,
                direction,
                source,
                snapshot: self.snapshot.clone(),
            }),
        })
    }
    /// Resolve exactly one still-current request. None explicitly declines it.
    /// A matching reply is consumed even if its proposed value is invalid or
    /// the editor rejects the replacement. Stale replies cannot consume a newer
    /// request. Accepted values use the current domain, end selection, one undo
    /// record, and a Committed event carrying the original interaction source.
    /// The adapter must check native route/gate eligibility before calling.
    pub fn resolve_application_step(
        &mut self,
        editor: &EditorSnapshot,
        request_id: i64,
        expected_revision: i64,
        value: Option<Value>,
        edit: impl FnOnce(&EditorCommand) -> Result<EditorSnapshot, EditorError>,
    ) -> Result<Outcome, Fault> {
        let events: Vec<_> = self.observe(editor)?.into_iter().collect();
        let fail = |events, error| Outcome {
            events,
            response: Response::Failed(error),
        };
        let Some(stamp) = self.step_requests.stamp.filter(|stamp| {
            stamp.id == request_id
                && stamp.revision == expected_revision
                && stamp.revision == self.snapshot.revision
        }) else {
            return Ok(fail(events, Error::StaleRevision));
        };
        self.cancel_step_request();
        if let Err(error) = self.user_mutation_allowed() {
            return Ok(fail(events, error));
        }
        if self.snapshot.composition.is_some() {
            return Ok(fail(events, Error::Composing));
        }
        let Some(value) = value else {
            return self.apply_preparation(events, Preparation::Read, edit);
        };
        if value == Value::Empty && !self.config.allow_empty {
            return Ok(fail(events, Error::Rejected(Rejection::EmptyRequired)));
        }
        let Some(value) = value.normalized(self.config.domain) else {
            return Ok(fail(events, Error::InvalidValue));
        };
        let preparation = self.replacement(
            format_value(value),
            value,
            SelectionPolicy::End,
            UndoPolicy::Record,
            Publication::Committed(stamp.source),
        );
        match preparation {
            Ok(preparation) => self.apply_preparation(events, preparation, edit),
            Err(error) => Ok(fail(events, error)),
        }
    }
}

#[cfg(test)]
#[path = "number_input_requests_test.rs"]
mod tests;
