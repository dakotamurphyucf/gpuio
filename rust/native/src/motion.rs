//! Deterministic native motion state. Layout previews values; paint commits them.
//! No timers, GPUI entities, callbacks or transport are owned by this module.
use gpuio_protocol::animation::{
    CancelReason, Config, Endpoint, Outcome, PROPERTY_COUNT, Property, Repeat, Target,
};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Values([Option<f64>; PROPERTY_COUNT]);
impl Values {
    fn from_targets(targets: &[Target]) -> Self {
        let mut result = Self([None; PROPERTY_COUNT]);
        for target in targets {
            result.0[target.property as usize] = Some(target.value);
        }
        result
    }
    pub fn get(&self, property: Property) -> Option<f64> {
        self.0[property as usize]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    Idle,
    At(Duration),
    Frame,
}
#[derive(Clone, Debug)]
pub struct Sample {
    pub values: Values,
    pub wake: Wake,
    generation: i64,
    at: Duration,
    done: bool,
    epoch: Arc<()>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    StaleGeneration,
}

pub struct State {
    config: Arc<Config>,
    from: Values,
    painted: Values,
    start: Duration,
    last_time: Duration,
    last_paint: Duration,
    paused_at: Option<Duration>,
    visible: bool,
    reduced: bool,
    immediate: bool,
    finished: bool,
    epoch: Arc<()>,
}
impl State {
    pub fn new(config: Arc<Config>, now: Duration, reduced: bool) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        let from = Values::from_targets(config.initial.as_deref().unwrap_or(&config.targets));
        let immediate = config.initial.is_none()
            || (config.repeat == Repeat::Once && from == Values::from_targets(&config.targets));
        Ok(Self {
            config,
            from,
            painted: from,
            start: now,
            last_time: now,
            last_paint: now,
            paused_at: reduced.then_some(now),
            visible: true,
            reduced,
            immediate,
            finished: false,
            epoch: Arc::new(()),
        })
    }
    fn clock(&mut self, now: Duration) -> Duration {
        self.last_time = now.max(self.last_time);
        self.last_time
    }
    fn suspend(&mut self, now: Duration) {
        if !self.visible || self.reduced {
            self.paused_at.get_or_insert(now);
        } else if let Some(paused) = self.paused_at.take() {
            self.start = self.start.saturating_add(now.saturating_sub(paused));
        }
    }
    pub fn set_visible(&mut self, visible: bool, now: Duration) {
        let now = self.clock(now);
        if visible != self.visible {
            self.epoch = Arc::new(());
        }
        self.visible = visible;
        self.suspend(now);
    }
    pub fn set_reduced_motion(&mut self, reduced: bool, now: Duration) {
        let now = self.clock(now);
        if reduced != self.reduced {
            self.epoch = Arc::new(());
        }
        self.reduced = reduced;
        self.suspend(now);
    }
    /// Equal snapshots do not restart a run. A new generation cancels a live run
    /// and starts from values that were actually painted, including during delay.
    pub fn retarget(
        &mut self,
        config: Arc<Config>,
        now: Duration,
    ) -> Result<Option<Endpoint>, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if config == self.config {
            return Ok(None);
        }
        if config.generation <= self.config.generation {
            return Err(Error::StaleGeneration);
        }
        let now = self.clock(now);
        let endpoint = self.cancel(CancelReason::Replaced);
        self.epoch = Arc::new(());
        let initial = Values::from_targets(config.initial.as_deref().unwrap_or(&config.targets));
        let mut from = Values([None; PROPERTY_COUNT]);
        for target in &config.targets {
            from.0[target.property as usize] = self
                .painted
                .get(target.property)
                .or(initial.get(target.property));
        }
        self.immediate =
            config.repeat == Repeat::Once && from == Values::from_targets(&config.targets);
        self.config = config;
        self.from = from;
        self.painted = from;
        self.start = now;
        self.last_paint = now;
        self.paused_at = (!self.visible || self.reduced).then_some(now);
        self.finished = false;
        Ok(endpoint)
    }
    pub fn cancel(&mut self, reason: CancelReason) -> Option<Endpoint> {
        if self.finished {
            return None;
        }
        self.finished = true;
        self.epoch = Arc::new(());
        Some(Endpoint {
            generation: self.config.generation,
            outcome: Outcome::Cancelled(reason),
        })
    }
    /// A sample alone does not report completion or change the last painted value.
    /// The host commits it only if this generation reaches paint.
    pub fn sample(&mut self, now: Duration) -> Sample {
        let now = self.clock(now);
        let mut result = Sample {
            values: self.painted,
            wake: Wake::Idle,
            generation: self.config.generation,
            at: now,
            done: false,
            epoch: self.epoch.clone(),
        };
        if self.finished || !self.visible {
            return result;
        }
        let target = Values::from_targets(&self.config.targets);
        if self.reduced {
            if self.config.repeat == Repeat::Once {
                result.values = target;
                result.done = true;
            } else {
                result.values = Values::from_targets(self.config.initial.as_ref().unwrap());
            }
            return result;
        }
        if self.immediate {
            result.values = target;
            result.done = true;
            return result;
        }
        let delay = Duration::from_millis(self.config.delay_ms as u64);
        let begin = self.start.saturating_add(delay);
        if now < begin {
            result.values = self.from;
            result.wake = Wake::At(begin);
            return result;
        }
        let duration = Duration::from_millis(self.config.duration_ms as u64);
        let elapsed = now.saturating_sub(begin);
        let mut from = self.from;
        let phase = match self.config.repeat {
            Repeat::Once => {
                if duration.is_zero() || elapsed >= duration {
                    result.values = target;
                    result.done = true;
                    return result;
                }
                elapsed.as_secs_f64() / duration.as_secs_f64()
            }
            Repeat::Loop | Repeat::Alternate => {
                // Integer modulo preserves sub-frame precision after long uptimes.
                let cycle = elapsed.as_nanos() / duration.as_nanos();
                if cycle > 0 {
                    from = Values::from_targets(self.config.initial.as_ref().unwrap());
                }
                let phase =
                    (elapsed.as_nanos() % duration.as_nanos()) as f64 / duration.as_nanos() as f64;
                if self.config.repeat == Repeat::Alternate && cycle % 2 == 1 {
                    1. - phase
                } else {
                    phase
                }
            }
        };
        let phase = self.config.easing.sample(phase);
        for target in &self.config.targets {
            let from = from.get(target.property).unwrap();
            result.values.0[target.property as usize] =
                Some(target.property.clamp(from + (target.value - from) * phase));
        }
        result.wake = Wake::Frame;
        result
    }
    pub fn accepts_sample(&self, sample: &Sample) -> bool {
        !self.finished
            && self.visible
            && sample.generation == self.config.generation
            && Arc::ptr_eq(&sample.epoch, &self.epoch)
            && sample.at >= self.last_paint
    }
    pub fn painted(&mut self, sample: Sample) -> Option<Endpoint> {
        if !self.accepts_sample(&sample) {
            return None;
        }
        self.last_paint = sample.at;
        self.painted = sample.values;
        if sample.done {
            self.finished = true;
            Some(Endpoint {
                generation: self.config.generation,
                outcome: Outcome::Finished,
            })
        } else {
            None
        }
    }
}
