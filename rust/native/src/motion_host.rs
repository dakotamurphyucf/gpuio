//! Session admission and shared native timing; rendering borrows this separately
//! from the retained tree. No timers or callbacks are retained here.
use crate::{motion_clock, motion_program, tree::ProgramChange};
use gpuio_protocol::{NodeId, WindowId, animation_program::Config, v1::ErrorCode};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

pub const MAX_OWNERS: usize = 1024;
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
pub struct Store {
    origin: Instant,
    #[cfg(feature = "native-tests")]
    test_now: Option<Duration>,
    pub(crate) clocks: motion_clock::Registry,
    owners: BTreeMap<(WindowId, NodeId), Arc<Config>>,
    reserved_bytes: usize,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
            #[cfg(feature = "native-tests")]
            test_now: None,
            clocks: motion_clock::Registry::new(Duration::ZERO),
            owners: BTreeMap::new(),
            reserved_bytes: 0,
        }
    }
}
impl Store {
    pub fn now(&self) -> Duration {
        #[cfg(feature = "native-tests")]
        if let Some(now) = self.test_now {
            return now;
        }
        self.origin.elapsed()
    }
    #[cfg(feature = "native-tests")]
    pub fn set_test_time(&mut self, now: Option<Duration>) {
        if now.is_none()
            && let Some(previous) = self.test_now
        {
            self.origin = Instant::now() - previous;
        }
        self.test_now = now;
    }
    pub fn counts(&self) -> (usize, usize, usize) {
        (
            self.owners.len(),
            self.clocks.counts().0,
            self.reserved_bytes,
        )
    }
    /// Called after tree validation and capacity reservation. On error neither
    /// the tree nor this store has changed. Style-only updates take the fast path.
    pub(crate) fn admit(
        &mut self,
        window: WindowId,
        changes: &[ProgramChange],
    ) -> Result<(), ErrorCode> {
        if changes.is_empty() {
            return Ok(());
        }
        let mut next = self.owners.clone();
        for (node, config) in changes {
            if let Some(config) = config {
                next.insert((window, *node), config.clone());
            } else {
                next.remove(&(window, *node));
            }
        }
        if next.len() > MAX_OWNERS {
            return Err(ErrorCode::LimitExceeded);
        }
        let bytes = next.values().try_fold(0usize, |total, config| {
            total
                .checked_add(motion_program::State::reservation(config))
                .filter(|sum| *sum <= MAX_BYTES)
                .ok_or(ErrorCode::LimitExceeded)
        })?;
        let declarations: Vec<_> = next
            .iter()
            .filter(|((w, _), c)| *w == window && c.program.clock.is_shared())
            .map(|((_, node), c)| (*node, Arc::new(c.program.clone())))
            .collect();
        self.clocks
            .replace_window(window, &declarations, self.now())
            .map_err(|error| match error {
                motion_clock::Error::Closed => ErrorCode::Closed,
                motion_clock::Error::LimitExceeded | motion_clock::Error::GenerationExhausted => {
                    ErrorCode::LimitExceeded
                }
                motion_clock::Error::InvalidDeclaration
                | motion_clock::Error::DuplicateMember
                | motion_clock::Error::ConflictingSchedule => ErrorCode::InvalidTree,
            })?;
        self.owners = next;
        self.reserved_bytes = bytes;
        Ok(())
    }
    pub(crate) fn close_window(&mut self, window: WindowId) {
        self.clocks.close_window(window, self.now());
        self.owners.retain(|(w, _), _| *w != window);
        self.reserved_bytes = self
            .owners
            .values()
            .map(|c| motion_program::State::reservation(c))
            .sum();
    }
    pub(crate) fn close(&mut self) {
        self.clocks.close();
        self.owners.clear();
        self.reserved_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use gpuio_protocol::{
        HandlerId,
        animation::{Easing, Property, Repeat, Spring, Target},
        animation_program::*,
        v1::*,
    };
    fn node(n: i64) -> NodeId {
        NodeId::from_parts(n, 1).unwrap()
    }
    fn wid(n: i64) -> WindowId {
        WindowId::from_parts(n, 1).unwrap()
    }
    fn config(generation: i64, clock: Clock, duration: i64) -> Config {
        Config {
            generation,
            playback: Playback::Running,
            restart: 0,
            program: Program {
                initial: Some(vec![Target {
                    property: Property::Width,
                    value: 0.,
                }]),
                stages: vec![Stage {
                    targets: vec![Target {
                        property: Property::Width,
                        value: 100.,
                    }],
                    timing: Timing::Tween(duration, Easing::Linear),
                    delay_ms: 0,
                }],
                delay_ms: 0,
                repeat: Repeat::Loop,
                clock,
            },
        }
    }
    fn session() -> Session {
        let mut s = Session::default();
        s.hello(VERSION, CAPABILITIES).unwrap();
        for i in 0..2 {
            s.open(i + 1, wid(i), "program", 400., 300.).unwrap();
        }
        s
    }
    fn tx(window: WindowId, base: i64, operations: Vec<Op>) -> Transaction {
        Transaction {
            window,
            base,
            revision: base + 1,
            operations,
        }
    }
    fn mount(s: &mut Session, w: WindowId, config: Config) {
        s.apply(&tx(
            w,
            0,
            vec![
                Op::Create(
                    node(0),
                    Kind::AnimationProgram,
                    "".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetAnimationProgram(node(0), config),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    }
    #[test]
    fn session_group_conflicts_and_invalid_trees_leave_membership_phase_and_revision_unchanged() {
        let mut s = session();
        for i in 0..2 {
            mount(
                &mut s,
                wid(i),
                config(1, Clock::Group("activity".into()), 1000),
            );
        }
        let motion = s.motion();
        let before = motion.borrow().counts();
        let group = motion.borrow().clocks.group_id("activity").unwrap();
        let snapshot = motion
            .borrow_mut()
            .clocks
            .sample(wid(0), node(0), Duration::from_secs(1))
            .unwrap();
        let bytes = s.retained_bytes();
        let conflict = tx(
            wid(0),
            1,
            vec![Op::SetAnimationProgram(
                node(0),
                config(2, Clock::Group("activity".into()), 2000),
            )],
        );
        assert_eq!(s.apply(&conflict), Err(ErrorCode::InvalidTree));
        assert_eq!(s.tree(wid(0)).unwrap().revision(), 1);
        assert_eq!(s.retained_bytes(), bytes);
        assert_eq!(motion.borrow().counts(), before);
        assert_eq!(motion.borrow().clocks.group_id("activity"), Some(group));
        assert!(snapshot.is_current());
        let invalid = tx(
            wid(0),
            1,
            vec![
                Op::SetAnimationProgram(node(0), config(2, Clock::Group("new".into()), 2000)),
                Op::Splice(node(0), 0, 0, vec![node(0)]),
            ],
        );
        assert_eq!(s.apply(&invalid), Err(ErrorCode::InvalidTree));
        assert_eq!(motion.borrow().counts(), before);
        assert!(motion.borrow().clocks.group_id("new").is_none());
        s.close(wid(0)).unwrap();
        assert!(snapshot.is_current());
        s.close(wid(1)).unwrap();
        assert!(!snapshot.is_current());
        assert_eq!(motion.borrow().counts(), (0, 0, 0));
        s.shutdown();
        assert_eq!(motion.borrow().counts(), (0, 0, 0));
    }
    #[test]
    fn program_updates_are_generation_fenced_and_event_batches_are_bounded() {
        let mut s = session();
        let c = config(1, Clock::Independent, 1000);
        mount(&mut s, wid(0), c.clone());
        let invalids = [
            vec![Op::SetAnimationProgram(
                node(0),
                config(1, Clock::Independent, 2000),
            )],
            vec![
                Op::SetAnimationProgram(node(0), c.clone()),
                Op::SetAnimationProgram(node(0), c),
            ],
        ];
        for operations in invalids {
            assert_eq!(
                s.apply(&tx(wid(0), 1, operations)),
                Err(ErrorCode::InvalidTree)
            );
        }
        let handler = HandlerId::from_parts(0, 1).unwrap();
        let signal = Signal {
            generation: 1,
            index: 1,
            observation: Observation::StageCompleted(0, StageResult::Played),
        };
        for signals in [
            vec![],
            vec![signal; 34],
            vec![signal; 2],
            vec![Signal {
                generation: 2,
                ..signal
            }],
        ] {
            assert!(
                s.animation_program_event(wid(0), node(0), handler, 1, signals)
                    .is_none()
            );
        }
        assert!(
            s.animation_program_event(wid(0), node(0), handler, 1, vec![signal])
                .is_some()
        );
        s.overload(wid(0));
        assert!(
            s.animation_program_event(wid(0), node(0), handler, 1, vec![signal])
                .is_none()
        );
    }
    #[test]
    fn owner_and_compiled_memory_limits_are_atomic_and_release_on_close() {
        let mut s = session();
        let mut ops = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
        for i in 1..=1024 {
            ops.extend([
                Op::Create(node(i), Kind::AnimationProgram, "".into(), None),
                Op::SetAnimationProgram(node(i), config(1, Clock::Independent, 1000)),
            ]);
        }
        ops.extend([
            Op::Splice(node(0), 0, 0, (1..=1024).map(node).collect()),
            Op::SetRoot(Some(node(0))),
        ]);
        s.apply(&tx(wid(0), 0, ops)).unwrap();
        let before = s.motion().borrow().counts();
        assert_eq!(before.0, MAX_OWNERS);
        assert_eq!(
            s.apply(&tx(
                wid(1),
                0,
                vec![
                    Op::Create(node(0), Kind::AnimationProgram, "".into(), None),
                    Op::SetAnimationProgram(node(0), config(1, Clock::Independent, 1000)),
                    Op::SetRoot(Some(node(0)))
                ]
            )),
            Err(ErrorCode::LimitExceeded)
        );
        assert_eq!(s.tree(wid(1)).unwrap().revision(), 0);
        assert_eq!(s.motion().borrow().counts(), before);
        let mut maximum = config(2, Clock::Independent, 1000);
        maximum.program.repeat = Repeat::Alternate;
        maximum.program.stages = vec![
            Stage {
                targets: maximum.program.stages[0].targets.clone(),
                timing: Timing::Spring(Spring {
                    stiffness: 100.,
                    damping: 10.,
                    mass: 1.,
                    epsilon: 0.001,
                    max_duration_ms: 1000
                }),
                delay_ms: 0
            };
            MAX_STAGES
        ];
        let cost = motion_program::State::reservation(&maximum);
        assert!(cost * MAX_OWNERS > MAX_BYTES);
        let ops = (1..=1024)
            .map(|i| Op::SetAnimationProgram(node(i), maximum.clone()))
            .collect();
        assert_eq!(s.apply(&tx(wid(0), 1, ops)), Err(ErrorCode::LimitExceeded));
        assert_eq!(s.motion().borrow().counts(), before);
        assert_eq!(s.tree(wid(0)).unwrap().revision(), 1);
        s.close(wid(0)).unwrap();
        assert_eq!(s.motion().borrow().counts(), (0, 0, 0));
        mount(&mut s, wid(1), maximum.clone());
        let mut owner =
            motion_program::State::new(Arc::new(maximum.clone()), Duration::ZERO, false).unwrap();
        maximum.generation += 1;
        maximum.program.stages[0].targets[0].value = 200.;
        owner
            .update(Arc::new(maximum.clone()), Duration::ZERO)
            .unwrap();
        assert!(owner.retained_bytes() <= motion_program::State::reservation(&maximum));
    }
}
