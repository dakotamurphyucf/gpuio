//! Application-owned shared clocks. No timer, window, or OCaml closure is owned.
use gpuio_protocol::{
    NodeId, WindowId,
    animation_program::{Clock as Selection, MAX_GROUPS, MAX_MEMBERS, Program},
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Debug)]
enum Scope {
    Application,
    Group(Arc<str>),
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    scope: Scope,
    pub elapsed: Duration,
    pub paused: bool,
    valid: Rc<Cell<bool>>,
}
impl Snapshot {
    pub fn matches(&self, selection: &Selection) -> bool {
        match (&self.scope, selection) {
            (Scope::Application, Selection::Application) => true,
            (Scope::Group(name), Selection::Group(expected)) => name.as_ref() == expected,
            _ => false,
        }
    }
    pub fn is_current(&self) -> bool {
        self.valid.get()
    }
}
pub struct Clock {
    scope: Scope,
    start: Duration,
    last: Duration,
    paused_at: Option<Duration>,
    user_paused: bool,
    reduced: bool,
    valid: Rc<Cell<bool>>,
}
impl Clock {
    pub fn new(now: Duration) -> Self {
        Self {
            scope: Scope::Application,
            start: now,
            last: now,
            paused_at: None,
            user_paused: false,
            reduced: false,
            valid: Rc::new(Cell::new(true)),
        }
    }
    fn time(&mut self, now: Duration) -> Duration {
        self.last = self.last.max(now);
        self.last
    }
    fn change(&mut self, now: Duration, user_paused: bool, reduced: bool) {
        let now = self.time(now);
        if (self.user_paused, self.reduced) == (user_paused, reduced) {
            return;
        }
        self.valid.set(false);
        self.valid = Rc::new(Cell::new(true));
        self.user_paused = user_paused;
        self.reduced = reduced;
        if user_paused || reduced {
            self.paused_at.get_or_insert(now);
        } else if let Some(paused) = self.paused_at.take() {
            self.start = self.start.saturating_add(now.saturating_sub(paused));
        }
    }
    pub fn set_paused(&mut self, paused: bool, now: Duration) {
        self.change(now, paused, self.reduced);
    }
    pub fn set_reduced(&mut self, reduced: bool, now: Duration) {
        self.change(now, self.user_paused, reduced);
    }
    pub fn sample(&mut self, now: Duration) -> Snapshot {
        let now = self.time(now);
        Snapshot {
            scope: self.scope.clone(),
            elapsed: self.paused_at.unwrap_or(now).saturating_sub(self.start),
            paused: self.paused_at.is_some(),
            valid: self.valid.clone(),
        }
    }
}
impl Drop for Clock {
    fn drop(&mut self) {
        self.valid.set(false);
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupId {
    pub name: String,
    pub generation: i64,
}
struct Group {
    id: GroupId,
    clock: Rc<RefCell<Clock>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    InvalidDeclaration,
    DuplicateMember,
    ConflictingSchedule,
    LimitExceeded,
    GenerationExhausted,
}
type Member = (WindowId, NodeId);

pub struct Registry {
    application: Option<Clock>,
    members: HashMap<Member, Arc<Program>>,
    groups: HashMap<String, Group>,
    next_generation: i64,
    reduced: bool,
}
impl Registry {
    pub fn new(now: Duration) -> Self {
        Self {
            application: Some(Clock::new(now)),
            members: HashMap::new(),
            groups: HashMap::new(),
            next_generation: 1,
            reduced: false,
        }
    }
    /// Atomically replaces one committed window's complete shared membership.
    /// Validation has no clock/registry side effects, including on rollback.
    pub fn replace_window(
        &mut self,
        window: WindowId,
        declarations: &[(NodeId, Arc<Program>)],
        now: Duration,
    ) -> Result<(), Error> {
        if self.application.is_none() {
            return Err(Error::Closed);
        }
        let retained = self.members.keys().filter(|(w, _)| *w != window).count();
        if declarations.len() > MAX_MEMBERS || retained + declarations.len() > MAX_MEMBERS {
            return Err(Error::LimitExceeded);
        }
        let mut ids = HashSet::new();
        for (node, program) in declarations {
            if !ids.insert(*node) {
                return Err(Error::DuplicateMember);
            }
            if !program.is_valid() || !program.clock.is_shared() {
                return Err(Error::InvalidDeclaration);
            }
        }
        let mut schedules: HashMap<&str, &Program> = HashMap::new();
        for program in self
            .members
            .iter()
            .filter(|((w, _), _)| *w != window)
            .map(|(_, p)| p.as_ref())
            .chain(declarations.iter().map(|(_, p)| p.as_ref()))
        {
            if let Selection::Group(name) = &program.clock {
                if let Some(previous) = schedules.get(name.as_str()) {
                    if !program.same_clock_schedule(previous) {
                        return Err(Error::ConflictingSchedule);
                    }
                } else {
                    schedules.insert(name, program);
                }
            }
        }
        if schedules.len() > MAX_GROUPS {
            return Err(Error::LimitExceeded);
        }
        let new_count = schedules
            .keys()
            .filter(|name| !self.groups.contains_key(**name))
            .count() as i64;
        self.next_generation
            .checked_add(new_count)
            .ok_or(Error::GenerationExhausted)?;
        // All admission checks precede mutation. Reusing a group preserves phase;
        // removing its final member drops the clock and invalidates old samples.
        let mut groups = HashMap::with_capacity(schedules.len());
        let mut names: Vec<_> = schedules.keys().copied().collect();
        names.sort_unstable();
        for name in names {
            let group = if let Some(previous) = self.groups.get(name) {
                Group {
                    id: previous.id.clone(),
                    clock: previous.clock.clone(),
                }
            } else {
                let mut clock = Clock::new(now);
                clock.scope = Scope::Group(Arc::from(name));
                clock.set_reduced(self.reduced, now);
                let id = GroupId {
                    name: name.to_owned(),
                    generation: self.next_generation,
                };
                self.next_generation += 1;
                Group {
                    id,
                    clock: Rc::new(RefCell::new(clock)),
                }
            };
            groups.insert(name.to_owned(), group);
        }
        self.members.retain(|(w, _), _| *w != window);
        self.members
            .extend(declarations.iter().map(|(n, p)| ((window, *n), p.clone())));
        self.groups = groups;
        Ok(())
    }
    pub fn sample(&mut self, window: WindowId, node: NodeId, now: Duration) -> Option<Snapshot> {
        match &self.members.get(&(window, node))?.clock {
            Selection::Application => Some(self.application.as_mut()?.sample(now)),
            Selection::Group(name) => Some(self.groups.get(name)?.clock.borrow_mut().sample(now)),
            Selection::Independent => None,
        }
    }
    pub fn group_id(&self, name: &str) -> Option<GroupId> {
        self.groups.get(name).map(|g| g.id.clone())
    }
    /// Stale handles cannot control a later group with the same name.
    pub fn set_group_paused(&mut self, id: &GroupId, paused: bool, now: Duration) -> bool {
        let Some(group) = self.groups.get(&id.name) else {
            return false;
        };
        if group.id != *id {
            return false;
        }
        group.clock.borrow_mut().set_paused(paused, now);
        true
    }
    pub fn set_application_paused(&mut self, paused: bool, now: Duration) {
        if let Some(clock) = &mut self.application {
            clock.set_paused(paused, now);
        }
    }
    pub fn set_reduced(&mut self, reduced: bool, now: Duration) {
        if self.reduced == reduced {
            return;
        }
        self.reduced = reduced;
        if let Some(clock) = &mut self.application {
            clock.set_reduced(reduced, now);
        }
        for group in self.groups.values() {
            group.clock.borrow_mut().set_reduced(reduced, now);
        }
    }
    pub fn close_window(&mut self, window: WindowId, now: Duration) {
        if self.application.is_some() {
            self.replace_window(window, &[], now)
                .expect("removing membership cannot fail admission");
        }
    }
    pub fn close(&mut self) {
        self.members.clear();
        self.groups.clear();
        self.application = None;
    }
    pub fn counts(&self) -> (usize, usize) {
        (self.groups.len(), self.members.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::{
        animation::{Easing, Property, Repeat, Target},
        animation_program::{Stage, Timing},
    };
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    fn window(n: u32) -> WindowId {
        WindowId::from_parts(i64::from(n), 1).unwrap()
    }
    fn node(n: u32) -> NodeId {
        NodeId::from_parts(i64::from(n), 1).unwrap()
    }
    fn program(clock: Selection, duration: i64) -> Arc<Program> {
        Arc::new(Program {
            initial: Some(vec![Target {
                property: Property::Opacity,
                value: 0.,
            }]),
            stages: vec![Stage {
                targets: vec![Target {
                    property: Property::Opacity,
                    value: 1.,
                }],
                timing: Timing::Tween(duration, Easing::Linear),
                delay_ms: 0,
            }],
            delay_ms: 0,
            repeat: Repeat::Loop,
            clock,
        })
    }
    #[test]
    fn phase_pause_reduce_and_destruction_invalidate_samples_without_timers() {
        let mut clock = Clock::new(ms(10));
        let prior = clock.sample(ms(30));
        assert_eq!(prior.elapsed, ms(20));
        clock.set_paused(true, ms(40));
        assert!(!prior.is_current());
        assert_eq!(clock.sample(ms(100)).elapsed, ms(30));
        clock.set_reduced(true, ms(100));
        clock.set_paused(false, ms(110));
        assert!(clock.sample(ms(120)).paused);
        clock.set_reduced(false, ms(130));
        let resumed = clock.sample(ms(150));
        assert_eq!(resumed.elapsed, ms(50));
        assert!(!resumed.paused);
        assert_eq!(clock.sample(ms(140)).elapsed, ms(50));
        drop(clock);
        assert!(!resumed.is_current());
    }
    #[test]
    fn late_members_and_windows_share_phase_and_last_removal_releases_group() {
        let mut registry = Registry::new(ms(0));
        let p = program(Selection::Group("pulse".into()), 100);
        registry
            .replace_window(window(0), &[(node(0), p.clone())], ms(10))
            .unwrap();
        let id = registry.group_id("pulse").unwrap();
        registry
            .replace_window(window(1), &[(node(0), p.clone())], ms(40))
            .unwrap();
        assert_eq!(
            registry.sample(window(0), node(0), ms(50)).unwrap().elapsed,
            ms(40)
        );
        let old = registry.sample(window(1), node(0), ms(50)).unwrap();
        assert_eq!(old.elapsed, ms(40));
        registry.close_window(window(0), ms(60));
        assert_eq!(registry.counts(), (1, 1));
        assert!(old.is_current());
        registry.close_window(window(1), ms(70));
        assert_eq!(registry.counts(), (0, 0));
        assert!(!old.is_current());
        registry
            .replace_window(window(1), &[(node(0), p)], ms(100))
            .unwrap();
        assert_ne!(registry.group_id("pulse").unwrap(), id);
        assert!(!registry.set_group_paused(&id, true, ms(110)));
        assert_eq!(
            registry
                .sample(window(1), node(0), ms(110))
                .unwrap()
                .elapsed,
            ms(10)
        );
    }
    #[test]
    fn conflicts_and_invalid_replacements_are_atomic() {
        let mut registry = Registry::new(ms(0));
        let p = program(Selection::Group("pulse".into()), 100);
        registry
            .replace_window(window(0), &[(node(0), p.clone())], ms(0))
            .unwrap();
        let id = registry.group_id("pulse").unwrap();
        let sample = registry.sample(window(0), node(0), ms(10)).unwrap();
        assert_eq!(
            registry.replace_window(
                window(1),
                &[(node(0), program(Selection::Group("pulse".into()), 200))],
                ms(20)
            ),
            Err(Error::ConflictingSchedule)
        );
        assert_eq!(
            registry.replace_window(window(0), &[(node(0), p.clone()), (node(0), p)], ms(20)),
            Err(Error::DuplicateMember)
        );
        assert_eq!(registry.counts(), (1, 1));
        assert_eq!(registry.group_id("pulse"), Some(id));
        assert!(sample.is_current());
        assert!(registry.sample(window(1), node(0), ms(20)).is_none());
    }
    #[test]
    fn group_and_application_pause_are_distinct_and_reduced_motion_freezes_both() {
        let mut registry = Registry::new(ms(0));
        registry
            .replace_window(
                window(0),
                &[
                    (node(0), program(Selection::Application, 100)),
                    (node(1), program(Selection::Group("pulse".into()), 100)),
                ],
                ms(10),
            )
            .unwrap();
        let id = registry.group_id("pulse").unwrap();
        assert!(registry.set_group_paused(&id, true, ms(20)));
        assert_eq!(
            registry.sample(window(0), node(1), ms(50)).unwrap().elapsed,
            ms(10)
        );
        assert_eq!(
            registry.sample(window(0), node(0), ms(50)).unwrap().elapsed,
            ms(50)
        );
        registry.set_reduced(true, ms(60));
        registry.set_group_paused(&id, false, ms(70));
        assert!(registry.sample(window(0), node(1), ms(80)).unwrap().paused);
        registry.set_reduced(false, ms(90));
        assert_eq!(
            registry
                .sample(window(0), node(0), ms(100))
                .unwrap()
                .elapsed,
            ms(70)
        );
        assert_eq!(
            registry
                .sample(window(0), node(1), ms(100))
                .unwrap()
                .elapsed,
            ms(20)
        );
        let live = registry.sample(window(0), node(0), ms(100)).unwrap();
        registry.close();
        assert!(!live.is_current());
        assert_eq!(registry.counts(), (0, 0));
        assert_eq!(
            registry.replace_window(window(0), &[], ms(110)),
            Err(Error::Closed)
        );
    }
    #[test]
    fn admission_limits_and_repeated_disposal_leave_no_name_tombstones() {
        let mut registry = Registry::new(ms(0));
        let groups: Vec<_> = (0..MAX_GROUPS)
            .map(|i| {
                (
                    node(i as u32),
                    program(Selection::Group(format!("g{i}")), 100),
                )
            })
            .collect();
        registry.replace_window(window(0), &groups, ms(0)).unwrap();
        assert_eq!(registry.counts(), (MAX_GROUPS, MAX_GROUPS));
        assert_eq!(
            registry.replace_window(
                window(1),
                &[(node(0), program(Selection::Group("extra".into()), 100))],
                ms(1)
            ),
            Err(Error::LimitExceeded)
        );
        registry.close_window(window(0), ms(2));
        let p = program(Selection::Application, 100);
        let members: Vec<_> = (0..MAX_MEMBERS)
            .map(|i| (node(i as u32), p.clone()))
            .collect();
        registry.replace_window(window(0), &members, ms(3)).unwrap();
        assert_eq!(
            registry.replace_window(window(1), &[(node(0), p)], ms(4)),
            Err(Error::LimitExceeded)
        );
        registry.close_window(window(0), ms(5));
        for i in 0..256 {
            registry
                .replace_window(
                    window(0),
                    &[(node(0), program(Selection::Group(format!("new-{i}")), 100))],
                    ms(i),
                )
                .unwrap();
            registry.close_window(window(0), ms(i));
            assert_eq!(registry.counts(), (0, 0));
        }
    }
    #[test]
    fn exhausted_group_identity_rejects_atomically_but_allows_disposal() {
        let mut registry = Registry::new(ms(0));
        let p = program(Selection::Group("existing".into()), 100);
        registry
            .replace_window(window(0), &[(node(0), p)], ms(0))
            .unwrap();
        let before = registry.sample(window(0), node(0), ms(10)).unwrap();
        registry.next_generation = i64::MAX;
        assert_eq!(
            registry.replace_window(
                window(1),
                &[(node(0), program(Selection::Group("new".into()), 100))],
                ms(20)
            ),
            Err(Error::GenerationExhausted)
        );
        assert_eq!(registry.counts(), (1, 1));
        assert!(before.is_current());
        registry.close_window(window(0), ms(30));
        assert_eq!(registry.counts(), (0, 0));
        assert!(!before.is_current());
    }
}
