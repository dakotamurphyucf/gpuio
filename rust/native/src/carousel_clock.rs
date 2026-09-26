//! Pure automatic-advance scheduler. The mounted adapter owns the cancellable
//! task and supplies visibility/focus/hover/gesture/motion eligibility. No clock
//! method mutates application selection or schedules I/O by itself.
use gpuio_protocol::{
    NodeId,
    carousel::{MAX_INTERVAL_MS, MIN_INTERVAL_MS},
};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub revision: i64,
    pub from: NodeId,
    pub target: NodeId,
    pub interval: Duration,
}
impl Schedule {
    fn is_valid(self) -> bool {
        self.revision >= 0
            && self.from != self.target
            && (Duration::from_millis(MIN_INTERVAL_MS as u64)
                ..=Duration::from_millis(MAX_INTERVAL_MS as u64))
                .contains(&self.interval)
    }
}
#[derive(Clone, Debug)]
pub struct Ticket {
    deadline: Duration,
    epoch: Arc<()>,
}
impl Ticket {
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        self.deadline == other.deadline && Arc::ptr_eq(&self.epoch, &other.epoch)
    }
}
#[derive(Clone, Debug)]
pub enum Plan {
    Idle,
    Wait { deadline: Duration, ticket: Ticket },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSchedule,
    TimeOverflow,
}
#[derive(Debug)]
pub struct Clock {
    schedule: Option<Schedule>,
    deadline: Option<Duration>,
    pending: bool,
    epoch: Arc<()>,
}
impl Default for Clock {
    fn default() -> Self {
        Self {
            schedule: None,
            deadline: None,
            pending: false,
            epoch: Arc::new(()),
        }
    }
}
impl Clock {
    fn cancel_deadline(&mut self) {
        if self.deadline.take().is_some() {
            self.epoch = Arc::new(());
        }
    }
    /// A changed revision/identity/interval rearms. Pausing discards elapsed time;
    /// resuming waits a full interval. A previously emitted proposal remains
    /// pending through pause/resume until a changed schedule acknowledges it.
    /// Identical updates keep the same deadline and never postpone it per frame.
    pub fn update(
        &mut self,
        schedule: Option<Schedule>,
        eligible: bool,
        now: Duration,
    ) -> Result<Plan, Error> {
        if schedule.is_some_and(|schedule| !schedule.is_valid()) {
            return Err(Error::InvalidSchedule);
        }
        let changed = schedule != self.schedule;
        let should_arm = eligible && schedule.is_some() && (changed || !self.pending);
        let deadline = if should_arm {
            if changed || self.deadline.is_none() {
                Some(
                    now.checked_add(schedule.unwrap().interval)
                        .ok_or(Error::TimeOverflow)?,
                )
            } else {
                self.deadline
            }
        } else {
            None
        };
        // Validation/time arithmetic precede mutation, preserving atomic failure.
        if changed {
            self.cancel_deadline();
            self.schedule = schedule;
            self.pending = false;
            self.epoch = Arc::new(());
        }
        if deadline.is_none() {
            self.cancel_deadline();
            return Ok(Plan::Idle);
        }
        self.deadline = deadline;
        let deadline = deadline.unwrap();
        Ok(Plan::Wait {
            deadline,
            ticket: Ticket {
                deadline,
                epoch: self.epoch.clone(),
            },
        })
    }
    /// The adapter rechecks eligibility with update before invoking wake. Early
    /// and cancelled/obsolete tickets cannot consume a future deadline. A valid
    /// wake returns exactly one proposal, then remains idle until acknowledgement.
    pub fn wake(&mut self, ticket: &Ticket, now: Duration) -> Option<Schedule> {
        if !Arc::ptr_eq(&ticket.epoch, &self.epoch)
            || self.deadline != Some(ticket.deadline)
            || now < ticket.deadline
            || self.pending
        {
            return None;
        }
        let schedule = self.schedule?;
        self.pending = true;
        self.cancel_deadline();
        Some(schedule)
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn dispose(&mut self) {
        self.cancel_deadline();
        self.schedule = None;
        self.pending = false;
        self.epoch = Arc::new(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sec(value: u64) -> Duration {
        Duration::from_secs(value)
    }
    fn schedule(revision: i64) -> Schedule {
        Schedule {
            revision,
            from: NodeId::from_parts(0, 1).unwrap(),
            target: NodeId::from_parts(1, 1).unwrap(),
            interval: sec(5),
        }
    }
    fn wait(plan: Plan) -> (Duration, Ticket) {
        match plan {
            Plan::Wait { deadline, ticket } => (deadline, ticket),
            Plan::Idle => panic!("expected deadline"),
        }
    }
    #[test]
    fn stable_deadline_early_wake_and_only_one_pending_proposal() {
        let mut clock = Clock::default();
        let config = Some(schedule(0));
        let (deadline, ticket) = wait(clock.update(config, true, sec(0)).unwrap());
        assert_eq!(deadline, sec(5));
        for second in 1..5 {
            assert_eq!(
                wait(clock.update(config, true, sec(second)).unwrap()).0,
                sec(5)
            );
        }
        assert_eq!(clock.wake(&ticket, sec(4)), None);
        assert_eq!(clock.wake(&ticket, sec(5)), config);
        assert!(clock.pending());
        for second in 5..10_000 {
            assert!(matches!(
                clock.update(config, second % 2 == 0, sec(second)),
                Ok(Plan::Idle)
            ));
            assert_eq!(clock.wake(&ticket, sec(second)), None);
        }
        let next = Some(schedule(1));
        let (_, ticket) = wait(clock.update(next, true, sec(10_000)).unwrap());
        assert_eq!(clock.wake(&ticket, sec(10_005)), next);
    }
    #[test]
    fn pauses_restart_full_interval_and_invalidate_old_tasks() {
        let mut clock = Clock::default();
        let config = Some(schedule(0));
        let (_, old) = wait(clock.update(config, true, sec(0)).unwrap());
        assert!(matches!(
            clock.update(config, false, sec(4)),
            Ok(Plan::Idle)
        ));
        assert_eq!(clock.wake(&old, sec(100)), None);
        let (deadline, current) = wait(clock.update(config, true, sec(100)).unwrap());
        assert_eq!(deadline, sec(105));
        assert_eq!(clock.wake(&old, sec(105)), None);
        assert_eq!(clock.wake(&current, sec(105)), config);
        assert!(matches!(
            clock.update(config, false, sec(106)),
            Ok(Plan::Idle)
        ));
        assert!(matches!(
            clock.update(config, true, sec(200)),
            Ok(Plan::Idle)
        ));
    }
    #[test]
    fn revisions_node_generations_policy_and_disposal_reject_stale_wakes() {
        let mut clock = Clock::default();
        let config = schedule(0);
        let (_, old) = wait(clock.update(Some(config), true, sec(0)).unwrap());
        let revised = Schedule {
            revision: 1,
            ..config
        };
        let (_, revised_ticket) = wait(clock.update(Some(revised), true, sec(1)).unwrap());
        assert_eq!(clock.wake(&old, sec(6)), None);
        let recreated = Schedule {
            target: NodeId::from_parts(1, 2).unwrap(),
            ..revised
        };
        let (_, recreated_ticket) = wait(clock.update(Some(recreated), true, sec(2)).unwrap());
        assert_eq!(clock.wake(&revised_ticket, sec(6)), None);
        clock.update(None, true, sec(3)).unwrap();
        assert_eq!(clock.wake(&recreated_ticket, sec(10)), None);
        let (_, current) = wait(clock.update(Some(config), true, sec(20)).unwrap());
        clock.dispose();
        assert_eq!(clock.wake(&current, sec(100)), None);
        assert!(!clock.pending());
    }
    #[test]
    fn invalid_schedule_and_time_overflow_preserve_live_deadline() {
        let mut clock = Clock::default();
        let config = schedule(0);
        let (_, ticket) = wait(clock.update(Some(config), true, sec(0)).unwrap());
        for invalid in [
            Schedule {
                revision: -1,
                ..config
            },
            Schedule {
                target: config.from,
                ..config
            },
            Schedule {
                interval: Duration::ZERO,
                ..config
            },
            Schedule {
                interval: sec(3601),
                ..config
            },
        ] {
            assert!(matches!(
                clock.update(Some(invalid), true, sec(1)),
                Err(Error::InvalidSchedule)
            ));
        }
        assert!(matches!(
            clock.update(
                Some(Schedule {
                    revision: 1,
                    ..config
                }),
                true,
                Duration::MAX
            ),
            Err(Error::TimeOverflow)
        ));
        assert_eq!(clock.wake(&ticket, sec(5)), Some(config));
    }
}
