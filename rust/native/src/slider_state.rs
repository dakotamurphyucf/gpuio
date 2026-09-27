//! Native-owned slider values and gesture lifecycle, independent of GPUI drawing.
//! The mounted adapter supplies node/handler and focus/visibility gates. Each
//! operation emits at most two small events; this owner has no queue or timer.
use gpuio_protocol::{numeric::Direction, slider::*};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Adjustment {
    Set(f64),
    Step { direction: Direction, page: bool },
    First,
    Last,
}

pub struct State {
    config: Arc<Config>,
    snapshot: Snapshot,
}
impl State {
    pub fn new(config: Arc<Config>, initial: Value) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        let value = initial
            .normalized(config.domain)
            .ok_or(Error::InvalidValue)?;
        Ok(Self {
            config,
            snapshot: Snapshot {
                revision: 0,
                value,
                committed: value,
                dragging: None,
            },
        })
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
    }
    fn reserve(&self, count: i64) -> Result<(), Error> {
        self.snapshot
            .revision
            .checked_add(count)
            .map(|_| ())
            .ok_or(Error::LimitExceeded)
    }
    fn check_input(&self, thumb: Thumb) -> Result<(), Error> {
        if self.config.disabled {
            return Err(Error::Disabled);
        }
        if self.config.read_only {
            return Err(Error::ReadOnly);
        }
        if !self.snapshot.value.supports(thumb) {
            return Err(Error::WrongThumb);
        }
        Ok(())
    }
    pub fn begin(&mut self, thumb: Thumb) -> Result<Event, Error> {
        self.check_input(thumb)?;
        if self.snapshot.dragging.is_some() {
            return Err(Error::Busy);
        }
        self.reserve(1)?;
        self.snapshot.revision += 1;
        self.snapshot.dragging = Some(thumb);
        Ok(Event::DragStarted(self.snapshot))
    }
    pub fn preview(&mut self, value: f64) -> Result<Option<Event>, Error> {
        let thumb = self.snapshot.dragging.ok_or(Error::Busy)?;
        self.check_input(thumb)?;
        let next = self
            .snapshot
            .value
            .set(thumb, value, self.config.domain)
            .ok_or(Error::InvalidValue)?;
        if next == self.snapshot.value {
            return Ok(None);
        }
        self.reserve(1)?;
        self.snapshot.value = next;
        self.snapshot.revision += 1;
        Ok(Some(Event::Preview(self.snapshot)))
    }
    pub fn preview_fraction(&mut self, fraction: f64) -> Result<Option<Event>, Error> {
        let value = self
            .config
            .from_fraction(fraction)
            .ok_or(Error::InvalidValue)?;
        self.preview(value)
    }
    pub fn finish(&mut self) -> Result<Option<Event>, Error> {
        if self.snapshot.dragging.is_none() {
            return Ok(None);
        }
        self.reserve(1)?;
        self.snapshot.dragging = None;
        self.snapshot.committed = self.snapshot.value;
        self.snapshot.revision += 1;
        Ok(Some(Event::Committed(Source::Pointer, self.snapshot)))
    }
    pub fn cancel(&mut self, reason: CancelReason) -> Result<Option<Event>, Error> {
        if self.snapshot.dragging.is_none() {
            return Ok(None);
        }
        self.reserve(1)?;
        Ok(self.cancel_reserved(reason))
    }
    fn cancel_reserved(&mut self, reason: CancelReason) -> Option<Event> {
        self.snapshot.dragging?;
        self.snapshot.value = self.snapshot.committed;
        self.snapshot.dragging = None;
        self.snapshot.revision += 1;
        Some(Event::Cancelled(reason, self.snapshot))
    }
    pub fn reconfigure(&mut self, config: Arc<Config>) -> Result<Vec<Event>, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if config == self.config {
            self.config = config;
            return Ok(Vec::new());
        }
        let reason = if config.domain != self.config.domain
            || config.axis != self.config.axis
            || config.scale != self.config.scale
        {
            Some(CancelReason::ConfigurationChanged)
        } else if config.disabled {
            Some(CancelReason::Disabled)
        } else if config.read_only {
            Some(CancelReason::ReadOnly)
        } else {
            None
        };
        let cancel = reason.is_some() && self.snapshot.dragging.is_some();
        self.reserve(if cancel { 2 } else { 1 })?;
        let mut events = Vec::with_capacity(2);
        if cancel {
            events.push(self.cancel_reserved(reason.unwrap()).unwrap());
        }
        // A valid old snapshot is finite and mode-preserving normalization is total.
        self.snapshot.value = self
            .snapshot
            .value
            .normalized(config.domain)
            .expect("validated numeric domain");
        self.snapshot.committed = self
            .snapshot
            .committed
            .normalized(config.domain)
            .expect("validated numeric domain");
        self.config = config;
        self.snapshot.revision += 1;
        events.push(Event::Observed(self.snapshot));
        Ok(events)
    }
    pub fn replace(&mut self, value: Value, if_revision: Option<i64>) -> Result<Vec<Event>, Error> {
        if if_revision.is_some_and(|r| r != self.snapshot.revision) {
            return Err(Error::StaleRevision);
        }
        if !value.is_valid() {
            return Err(Error::InvalidValue);
        }
        if !value.same_mode(self.snapshot.value) {
            return Err(Error::WrongMode);
        }
        let next = value
            .normalized(self.config.domain)
            .ok_or(Error::InvalidValue)?;
        self.reserve(if self.snapshot.dragging.is_some() {
            2
        } else {
            1
        })?;
        let mut events = Vec::with_capacity(2);
        if let Some(event) = self.cancel_reserved(CancelReason::Programmatic) {
            events.push(event);
        }
        self.snapshot.value = next;
        self.snapshot.committed = next;
        self.snapshot.revision += 1;
        events.push(Event::Observed(self.snapshot));
        Ok(events)
    }
    pub fn adjust(
        &mut self,
        thumb: Thumb,
        adjustment: Adjustment,
        source: Source,
    ) -> Result<Vec<Event>, Error> {
        self.check_input(thumb)?;
        let old = self.snapshot.committed;
        let next = match adjustment {
            Adjustment::Set(value) => old
                .set(thumb, value, self.config.domain)
                .ok_or(Error::InvalidValue)?,
            Adjustment::First => old
                .set(thumb, self.config.domain.min(), self.config.domain)
                .unwrap(),
            Adjustment::Last => old
                .set(thumb, self.config.domain.max(), self.config.domain)
                .unwrap(),
            Adjustment::Step { direction, page } => {
                let mut value = old;
                for _ in 0..if page { 10 } else { 1 } {
                    value = value.advance(thumb, self.config.domain, direction).unwrap();
                }
                value
            }
        };
        let cancelled = self.snapshot.dragging.is_some();
        let changed = next != old;
        self.reserve(i64::from(cancelled) + i64::from(changed))?;
        let mut events = Vec::with_capacity(2);
        if let Some(event) = self.cancel_reserved(CancelReason::Interrupted) {
            events.push(event);
        }
        if changed {
            self.snapshot.value = next;
            self.snapshot.committed = next;
            self.snapshot.revision += 1;
            events.push(Event::Committed(source, self.snapshot));
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::numeric::Domain;
    fn config() -> Config {
        Config {
            domain: Domain::new(0., 10., 1.).unwrap(),
            label: "Range".into(),
            lower_label: "Minimum".into(),
            upper_label: "Maximum".into(),
            axis: Axis::Horizontal,
            scale: Scale::Linear,
            disabled: false,
            read_only: false,
        }
    }
    fn state() -> State {
        State::new(
            config().into(),
            Value::Range {
                lower: 2.,
                upper: 8.,
            },
        )
        .unwrap()
    }
    fn range(lower: f64, upper: f64) -> Value {
        Value::Range { lower, upper }
    }
    #[test]
    fn preview_commit_cancel_and_thumb_identity() {
        let mut s = state();
        assert!(matches!(
            s.begin(Thumb::Lower).unwrap(),
            Event::DragStarted(_)
        ));
        s.preview(9.).unwrap();
        assert_eq!(s.snapshot.value, range(8., 8.));
        assert_eq!(s.snapshot.committed, range(2., 8.));
        assert!(s.preview(100.).unwrap().is_none());
        let final_event = s.finish().unwrap().unwrap();
        assert!(final_event.is_valid());
        assert_eq!(s.snapshot.committed, range(8., 8.));
        assert!(s.finish().unwrap().is_none());
        s.begin(Thumb::Upper).unwrap();
        s.preview(10.).unwrap();
        s.cancel(CancelReason::Escape).unwrap();
        assert_eq!(s.snapshot.value, range(8., 8.));
        assert_eq!(s.begin(Thumb::Single), Err(Error::WrongThumb));
    }
    #[test]
    fn bounds_change_cancels_before_new_domain_observation() {
        let mut s = state();
        s.begin(Thumb::Lower).unwrap();
        s.preview(6.).unwrap();
        let revision = s.snapshot.revision;
        let mut c = config();
        c.domain = Domain::new(3., 5., 0.5).unwrap();
        let events = s.reconfigure(c.into()).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Cancelled(CancelReason::ConfigurationChanged, _)
        ));
        assert_eq!(events[0].snapshot().value, range(2., 8.));
        assert_eq!(events[1].snapshot().value, range(3., 5.));
        assert_eq!(events[0].snapshot().revision, revision + 1);
        assert_eq!(events[1].snapshot().revision, revision + 2);
        assert!(events.iter().all(|e| e.is_valid()));
    }
    #[test]
    fn labels_preserve_drag_policy_changes_cancel_and_commands_can_replace() {
        let mut s = state();
        s.begin(Thumb::Upper).unwrap();
        s.preview(9.).unwrap();
        let mut c = config();
        c.label = "Translated".into();
        let events = s.reconfigure(c.clone().into()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(s.snapshot.dragging, Some(Thumb::Upper));
        assert_eq!(s.snapshot.value, range(2., 9.));
        assert!(s.reconfigure(c.clone().into()).unwrap().is_empty());
        c.read_only = true;
        let events = s.reconfigure(c.into()).unwrap();
        assert!(matches!(
            events[0],
            Event::Cancelled(CancelReason::ReadOnly, _)
        ));
        assert_eq!(s.snapshot.value, range(2., 8.));
        assert_eq!(s.begin(Thumb::Lower), Err(Error::ReadOnly));
        s.replace(range(1., 4.), None).unwrap();
        assert_eq!(s.snapshot.value, range(1., 4.));
    }
    #[test]
    fn guarded_or_invalid_commands_never_cancel_an_active_drag() {
        let mut s = state();
        s.begin(Thumb::Lower).unwrap();
        s.preview(5.).unwrap();
        let old = s.snapshot;
        assert_eq!(s.replace(range(3., 9.), Some(0)), Err(Error::StaleRevision));
        assert_eq!(s.replace(Value::Single(3.), None), Err(Error::WrongMode));
        assert_eq!(s.replace(range(9., 1.), None), Err(Error::InvalidValue));
        assert_eq!(
            s.adjust(
                Thumb::Lower,
                Adjustment::Set(f64::NAN),
                Source::Accessibility
            ),
            Err(Error::InvalidValue)
        );
        assert_eq!(s.snapshot, old);
        let events = s.replace(range(-1., 20.), Some(old.revision)).unwrap();
        assert!(matches!(
            events[0],
            Event::Cancelled(CancelReason::Programmatic, _)
        ));
        assert_eq!(events[1].snapshot().value, range(0., 10.));
    }
    #[test]
    fn discrete_input_interrupts_drag_and_native_bursts_use_latest_value() {
        let mut s = state();
        s.begin(Thumb::Upper).unwrap();
        s.preview(10.).unwrap();
        let events = s
            .adjust(
                Thumb::Lower,
                Adjustment::Step {
                    direction: Direction::Increase,
                    page: false,
                },
                Source::Keyboard,
            )
            .unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Cancelled(CancelReason::Interrupted, _)
        ));
        assert_eq!(s.snapshot.value, range(3., 8.));
        for _ in 0..10 {
            s.adjust(
                Thumb::Lower,
                Adjustment::Step {
                    direction: Direction::Increase,
                    page: false,
                },
                Source::Keyboard,
            )
            .unwrap();
        }
        assert_eq!(s.snapshot.value, range(8., 8.));
        s.adjust(Thumb::Upper, Adjustment::Last, Source::Accessibility)
            .unwrap();
        assert_eq!(s.snapshot.value, range(8., 10.));
        s.adjust(Thumb::Lower, Adjustment::First, Source::Keyboard)
            .unwrap();
        assert_eq!(s.snapshot.value, range(0., 10.));
    }
    #[test]
    fn revision_overflow_is_atomic_even_for_two_event_operations() {
        let mut s = state();
        s.begin(Thumb::Lower).unwrap();
        s.preview(4.).unwrap();
        s.snapshot.revision = i64::MAX - 1;
        let old = s.snapshot;
        assert_eq!(s.replace(range(3., 7.), None), Err(Error::LimitExceeded));
        assert_eq!(s.snapshot, old);
        let mut c = config();
        c.disabled = true;
        assert_eq!(s.reconfigure(c.into()), Err(Error::LimitExceeded));
        assert_eq!(s.snapshot, old);
        assert!(!s.config.disabled);
        s.cancel(CancelReason::Hidden).unwrap();
        assert_eq!(s.snapshot.revision, i64::MAX);
        assert_eq!(s.begin(Thumb::Lower), Err(Error::LimitExceeded));
        assert!(s.snapshot.dragging.is_none());
    }
    #[test]
    fn configurations_are_shared_and_old_owners_release_without_resetting_values() {
        let initial = Arc::new(config());
        let weak = Arc::downgrade(&initial);
        let mut s = State::new(initial.clone(), range(2., 8.)).unwrap();
        assert_eq!(Arc::strong_count(&initial), 2);
        drop(initial);
        s.begin(Thumb::Lower).unwrap();
        s.preview(3.).unwrap();
        let old = s.snapshot;
        let replacement = Arc::new(config());
        assert!(s.reconfigure(replacement.clone()).unwrap().is_empty());
        assert!(weak.upgrade().is_none());
        assert_eq!(s.snapshot, old);
        assert_eq!(Arc::strong_count(&replacement), 2);
        drop(s);
        assert_eq!(Arc::strong_count(&replacement), 1);
    }
    #[test]
    fn cancellation_reasons_disability_and_page_steps_are_explicit() {
        for reason in [
            CancelReason::Escape,
            CancelReason::Hidden,
            CancelReason::Modal,
            CancelReason::WindowInactive,
            CancelReason::Unmounted,
        ] {
            let mut s = state();
            s.begin(Thumb::Lower).unwrap();
            s.preview(5.).unwrap();
            let event = s.cancel(reason).unwrap().unwrap();
            assert!(event.is_valid());
            assert_eq!(s.snapshot.value, range(2., 8.));
            assert!(s.cancel(reason).unwrap().is_none());
        }
        let mut s = State::new(config().into(), Value::Single(2.)).unwrap();
        s.adjust(
            Thumb::Single,
            Adjustment::Step {
                direction: Direction::Increase,
                page: true,
            },
            Source::Keyboard,
        )
        .unwrap();
        assert_eq!(s.snapshot.value, Value::Single(10.));
        s.begin(Thumb::Single).unwrap();
        s.preview(1.).unwrap();
        let mut c = config();
        c.disabled = true;
        let events = s.reconfigure(c.into()).unwrap();
        assert!(matches!(
            events[0],
            Event::Cancelled(CancelReason::Disabled, _)
        ));
        assert_eq!(s.begin(Thumb::Single), Err(Error::Disabled));
        s.replace(Value::Single(5.), None).unwrap();
        assert_eq!(s.snapshot.value, Value::Single(5.));
    }
}
