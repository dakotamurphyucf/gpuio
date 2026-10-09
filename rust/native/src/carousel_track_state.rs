//! Bounded publication and automatic-proposal state for one mounted track source.
//! The presenter owns the source identity, task, visibility and settled-paint
//! checks. This module performs no I/O and never changes application selection.
use crate::{
    carousel_clock::{self, Clock, Plan, Schedule, Ticket},
    carousel_gesture::Step,
    carousel_track_geometry::Geometry,
};
use gpuio_protocol::{
    NodeId,
    carousel_track::{Config, Layout, Proposal, Request},
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    InvalidItems,
    InvalidGeometry,
    EpochExhausted,
    Clock(carousel_clock::Error),
}

pub struct State {
    config: Config,
    items: Vec<NodeId>,
    geometry: Option<Geometry>,
    observation: Option<Layout>,
    published: bool,
    measured: bool,
    clock: Clock,
}
impl State {
    /// Replacing the mounted window/node/handler source must create a fresh State.
    /// Its independent clock identity rejects tickets retained from old sources.
    pub fn new(config: Config, items: Vec<NodeId>) -> Result<Self, Error> {
        Self::validate(&config, &items)?;
        Ok(Self {
            config,
            items,
            geometry: None,
            observation: None,
            published: false,
            measured: false,
            clock: Clock::default(),
        })
    }
    fn validate(config: &Config, items: &[NodeId]) -> Result<(), Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if items.len() != config.carousel.ids.len()
            || items
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != items.len()
        {
            return Err(Error::InvalidItems);
        }
        Ok(())
    }
    /// Selection/policy changes rearm the clock but keep measured geometry.
    /// Collection lineage or retained owner changes invalidate it before input.
    pub fn sync(&mut self, config: Config, items: Vec<NodeId>) -> Result<(), Error> {
        Self::validate(&config, &items)?;
        if !config.can_replace(&self.config) {
            return Err(Error::InvalidConfig);
        }
        if config != self.config || items != self.items {
            self.clock.dispose();
        }
        if config.lineage != self.config.lineage || items != self.items {
            self.geometry = None;
            self.published = false;
            self.measured = false;
            // Keep the previous epoch fence, including same-map remeasurement.
        }
        self.config = config;
        self.items = items;
        Ok(())
    }
    /// Real geometry changes (including resize with the same canonical stops)
    /// retire pending automatic proposals. Animation offsets are not geometry.
    /// The returned observation must be queued before forwarding native input.
    /// Retry a failed publication with pending_layout; never acknowledge failure.
    pub fn measure(&mut self, geometry: Option<Geometry>) -> Result<Option<Layout>, Error> {
        let stops = geometry.as_ref().map(Geometry::stops);
        let candidate = Layout {
            lineage: self.config.lineage,
            epoch: 0,
            stops,
        };
        if !self.config.accepts_layout(&candidate) {
            return Err(Error::InvalidGeometry);
        }
        let changed = !self.measured
            || self
                .observation
                .as_ref()
                .is_none_or(|old| old.lineage != candidate.lineage || old.stops != candidate.stops)
            || self.geometry != geometry;
        if changed {
            let epoch = match &self.observation {
                None => 0,
                Some(old) => old.epoch.checked_add(1).ok_or(Error::EpochExhausted)?,
            };
            self.geometry = geometry;
            self.measured = true;
            self.observation = Some(Layout { epoch, ..candidate });
            self.published = false;
            self.clock.dispose();
        }
        Ok(self.pending_layout().cloned())
    }
    pub fn pending_layout(&self) -> Option<&Layout> {
        self.observation.as_ref().filter(|layout| {
            self.measured && !self.published && layout.lineage == self.config.lineage
        })
    }
    /// Only acknowledge a successful enqueue of this exact source's latest layout.
    /// No model-revision echo is needed; layout-only reducers keep that revision.
    pub fn published(&mut self, layout: &Layout) -> bool {
        if self.pending_layout() == Some(layout) {
            self.published = true;
            true
        } else {
            false
        }
    }
    pub fn geometry(&self) -> Option<&Geometry> {
        self.geometry.as_ref().filter(|_| self.published)
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn manual(&self, request: Request) -> Option<Request> {
        if matches!(request, Request::Layout(_) | Request::AutoNext(_)) {
            return None;
        }
        (self.geometry().is_some() && self.config.accepts_request(&request)).then_some(request)
    }
    fn schedule(&self) -> Option<Schedule> {
        let model = &self.config.carousel;
        if model.disabled {
            return None;
        }
        let current = model.selected? as usize;
        let next = self.geometry()?.step(current, Step::Next)?;
        Some(Schedule {
            revision: model.revision,
            from: self.items[current],
            target: self.items[next],
            interval: Duration::from_millis(model.auto_advance_ms? as u64),
        })
    }
    /// Eligible means settled, painted, visible, active and outside paused input
    /// or reduced motion. The presenter must supply it again on each timer wake.
    pub fn plan(&mut self, eligible: bool, now: Duration) -> Result<Plan, Error> {
        self.clock
            .update(self.schedule(), eligible, now)
            .map_err(Error::Clock)
    }
    pub fn wake(
        &mut self,
        ticket: &Ticket,
        eligible: bool,
        now: Duration,
    ) -> Result<Option<Proposal>, Error> {
        self.plan(eligible, now)?;
        let Some(schedule) = self.clock.wake(ticket, now) else {
            return Ok(None);
        };
        let from = self
            .items
            .iter()
            .position(|item| *item == schedule.from)
            .expect("current clock source");
        let target = self
            .items
            .iter()
            .position(|item| *item == schedule.target)
            .expect("current clock target");
        Ok(Some(Proposal {
            revision: self.config.carousel.revision,
            geometry_epoch: self
                .observation
                .as_ref()
                .expect("published measurement")
                .epoch,
            from: self.config.carousel.ids[from].clone(),
            target: self.config.carousel.ids[target].clone(),
        }))
    }
    pub fn pending_proposal(&self) -> bool {
        self.clock.pending()
    }
    pub fn dispose(&mut self) {
        self.clock.dispose();
        self.geometry = None;
        self.observation = None;
        self.published = false;
        self.measured = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carousel_track_geometry::Item;
    use gpuio_protocol::carousel::{self, Axis, Direction};
    fn sec(n: u64) -> Duration {
        Duration::from_secs(n)
    }
    fn initial() -> State {
        State::new(
            Config {
                carousel: carousel::Config {
                    revision: 0,
                    ids: vec!["a".into(), "b".into(), "c".into()],
                    selected: Some(0),
                    disabled: false,
                    looping: false,
                    axis: Axis::Horizontal,
                    auto_advance_ms: Some(1000),
                    direction: Direction::Direct,
                },
                lineage: 0,
            },
            (0..3)
                .map(|slot| NodeId::from_parts(slot, 1).unwrap())
                .collect(),
        )
        .unwrap()
    }
    fn geometry(extent: f32) -> Geometry {
        Geometry::new(
            extent,
            0.,
            vec![
                Item {
                    start: 0.,
                    extent: 50.,
                },
                Item {
                    start: 50.,
                    extent: 50.,
                },
                Item {
                    start: 100.,
                    extent: 50.,
                },
            ],
            false,
        )
        .unwrap()
    }
    fn publish(s: &mut State, extent: f32) -> Layout {
        let layout = s.measure(Some(geometry(extent))).unwrap().unwrap();
        assert!(s.published(&layout));
        layout
    }
    fn ticket(plan: Plan) -> (Duration, Ticket) {
        match plan {
            Plan::Wait { deadline, ticket } => (deadline, ticket),
            Plan::Idle => panic!("expected wait"),
        }
    }
    #[test]
    fn input_waits_for_successful_publication_without_a_model_revision_echo() {
        let mut s = initial();
        assert!(s.manual(Request::Next).is_none());
        let layout = s.measure(Some(geometry(100.))).unwrap().unwrap();
        assert!(s.manual(Request::Next).is_none());
        assert!(matches!(s.plan(true, sec(0)), Ok(Plan::Idle)));
        assert_eq!(
            s.measure(Some(geometry(100.))).unwrap(),
            Some(layout.clone())
        );
        assert!(s.published(&layout));
        assert_eq!(s.manual(Request::Next), Some(Request::Next));
        assert!(s.measure(Some(geometry(100.))).unwrap().is_none());
        assert_eq!(ticket(s.plan(true, sec(0)).unwrap()).0, sec(1));
        let (_, t) = ticket(s.plan(true, sec(0)).unwrap());
        let proposal = s.wake(&t, true, sec(1)).unwrap().unwrap();
        assert_eq!(
            (
                proposal.from.as_str(),
                proposal.target.as_str(),
                proposal.geometry_epoch
            ),
            ("a", "b", 0)
        );
        assert!(s.pending_proposal());
        for n in 2..100 {
            assert!(matches!(s.plan(true, sec(n)), Ok(Plan::Idle)));
        }
    }
    #[test]
    fn resize_and_unavailable_geometry_retire_proposals_even_with_equal_stop_maps() {
        let mut s = initial();
        let first = publish(&mut s, 100.);
        let (_, old) = ticket(s.plan(true, sec(0)).unwrap());
        assert!(s.wake(&old, true, sec(1)).unwrap().is_some());
        let resized = s.measure(Some(geometry(110.))).unwrap().unwrap();
        assert_eq!(first.stops, resized.stops);
        assert_eq!(resized.epoch, first.epoch + 1);
        assert!(!s.published(&first));
        assert!(s.wake(&old, true, sec(20)).unwrap().is_none());
        assert!(s.published(&resized));
        let (deadline, current) = ticket(s.plan(true, sec(20)).unwrap());
        assert_eq!(deadline, sec(21));
        let missing = s.measure(None).unwrap().unwrap();
        assert!(missing.stops.is_none());
        assert!(s.published(&missing));
        assert!(s.geometry().is_none());
        assert!(s.wake(&current, true, sec(21)).unwrap().is_none());
        publish(&mut s, 110.);
        assert_eq!(ticket(s.plan(true, sec(30)).unwrap()).0, sec(31));
    }
    #[test]
    fn pause_revision_lineage_and_remount_reject_obsolete_timer_tickets() {
        let mut s = initial();
        publish(&mut s, 100.);
        let (_, old) = ticket(s.plan(true, sec(0)).unwrap());
        assert!(s.wake(&old, false, sec(1)).unwrap().is_none());
        let (deadline, current) = ticket(s.plan(true, sec(10)).unwrap());
        assert_eq!(deadline, sec(11));
        assert!(s.wake(&old, true, sec(11)).unwrap().is_none());
        let mut config = s.config.clone();
        config.carousel.revision = 1;
        config.carousel.selected = Some(1);
        s.sync(config, s.items.clone()).unwrap();
        assert!(s.wake(&current, true, sec(11)).unwrap().is_none());
        // Last two logical items have the same stop; no automatic successor.
        assert!(matches!(s.plan(true, sec(12)), Ok(Plan::Idle)));
        let mut config = s.config.clone();
        config.lineage = 1;
        config.carousel.revision = 2;
        config.carousel.axis = Axis::Vertical;
        s.sync(config, s.items.clone()).unwrap();
        assert!(s.geometry().is_none());
        let next = publish(&mut s, 100.);
        assert_eq!(next.lineage, 1);
        assert_eq!(next.epoch, 1);
        let mut remounted = initial();
        publish(&mut remounted, 100.);
        assert!(remounted.wake(&old, true, sec(50)).unwrap().is_none());
        let (_, fresh) = ticket(remounted.plan(true, sec(50)).unwrap());
        remounted.dispose();
        assert!(remounted.wake(&fresh, true, sec(100)).unwrap().is_none());
    }
    #[test]
    fn invalid_updates_and_epoch_overflow_preserve_current_measurement() {
        let mut s = initial();
        let first = publish(&mut s, 100.);
        let mut config = s.config.clone();
        config.lineage = 1;
        assert_eq!(s.sync(config, s.items.clone()), Err(Error::InvalidConfig));
        assert_eq!(
            s.sync(s.config.clone(), vec![s.items[0]; 3]),
            Err(Error::InvalidItems)
        );
        let invalid = Geometry::new(100., 0., vec![], false).unwrap();
        assert_eq!(s.measure(Some(invalid)), Err(Error::InvalidGeometry));
        assert_eq!(s.observation.as_ref(), Some(&first));
        assert!(s.geometry().is_some());
        s.observation.as_mut().unwrap().epoch = i64::MAX;
        assert_eq!(s.measure(Some(geometry(110.))), Err(Error::EpochExhausted));
        assert_eq!(s.geometry(), Some(&geometry(100.)));
        assert!(s.published);
    }
    #[test]
    fn owner_changes_wait_for_fresh_measurement_and_automatic_navigation_skips_duplicate_stops() {
        let mut s = initial();
        publish(&mut s, 100.);
        let items = (0..3)
            .map(|slot| NodeId::from_parts(slot, 2).unwrap())
            .collect();
        s.sync(s.config.clone(), items).unwrap();
        assert!(s.pending_layout().is_none());
        assert!(s.geometry().is_none());
        let fresh = publish(&mut s, 100.);
        assert_eq!(fresh.epoch, 1);
        let skipped = Geometry::new(
            50.,
            0.,
            vec![
                Item {
                    start: 0.,
                    extent: 0.,
                },
                Item {
                    start: 0.,
                    extent: 50.,
                },
                Item {
                    start: 50.,
                    extent: 50.,
                },
            ],
            false,
        )
        .unwrap();
        let layout = s.measure(Some(skipped)).unwrap().unwrap();
        assert_eq!(layout.stops.as_ref().unwrap().canonical, vec![0, 0, 2]);
        assert!(s.published(&layout));
        let (_, ticket) = ticket(s.plan(true, sec(0)).unwrap());
        let proposal = s.wake(&ticket, true, sec(1)).unwrap().unwrap();
        assert_eq!(proposal.target, "c");
        assert!(s.config.accepts_request(&Request::AutoNext(proposal)));
        let mut disabled = s.config.clone();
        disabled.carousel.revision = 1;
        disabled.carousel.disabled = true;
        s.sync(disabled, s.items.clone()).unwrap();
        assert!(matches!(s.plan(true, sec(3)), Ok(Plan::Idle)));
        let missing = s.measure(None).unwrap().unwrap();
        assert!(s.published(&missing));
    }
}
