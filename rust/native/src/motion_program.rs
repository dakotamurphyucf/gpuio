//! Retained advanced motion; native paint alone commits positions and stage events.
use crate::{
    motion::Wake,
    motion_clock::Snapshot as ClockSnapshot,
    motion_timeline::{self, Frame, Next, Timeline},
};
use gpuio_protocol::{
    animation::Repeat,
    animation_program::{
        CancelReason, Config, Observation, Playback, Program, Signal, StageResult, TERMINAL_INDEX,
    },
};
use std::{sync::Arc, time::Duration};

struct Tracks {
    first: Arc<Timeline>,
    forward: Option<Arc<Timeline>>,
    reverse: Option<Arc<Timeline>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    StaleGeneration,
    StaleRestart,
    MissingClock,
    StaleClock,
    WrongClock,
}
impl Tracks {
    fn compile(program: &Program, painted: Option<Frame>) -> Result<Self, Error> {
        let first =
            Arc::new(Timeline::compile(program, painted).map_err(|_| Error::InvalidConfig)?);
        let mut result = Self {
            first,
            forward: None,
            reverse: None,
        };
        if program.repeat != Repeat::Once {
            let mut repeating = program.clone();
            repeating.delay_ms = 0;
            let forward = if program.delay_ms == 0 && painted.is_none() {
                result.first.clone()
            } else {
                Arc::new(Timeline::compile(&repeating, None).map_err(|_| Error::InvalidConfig)?)
            };
            if program.repeat == Repeat::Alternate {
                let mut previous = repeating.initial.clone().expect("validated repeat initial");
                for stage in &mut repeating.stages {
                    std::mem::swap(&mut previous, &mut stage.targets);
                }
                repeating.initial = Some(previous);
                repeating.stages.reverse();
                result.reverse = Some(Arc::new(
                    Timeline::compile(&repeating, None).map_err(|_| Error::InvalidConfig)?,
                ));
            }
            result.forward = Some(forward);
        }
        Ok(result)
    }
    fn retained_bytes(&self) -> usize {
        self.first.retained_bytes()
            + self.forward.as_ref().map_or(0, |t| {
                if Arc::ptr_eq(t, &self.first) {
                    0
                } else {
                    t.retained_bytes()
                }
            })
            + self.reverse.as_ref().map_or(0, |t| t.retained_bytes())
    }
    fn repeat(&self, elapsed: Duration, reverse_first: bool) -> motion_timeline::Sample {
        let forward = self.forward.as_ref().expect("repeat tracks");
        if forward.is_static() {
            return motion_timeline::Sample {
                frame: forward.initial(),
                completed: 0,
                finished: false,
                next: Next::Idle,
            };
        }
        let (first, second) = if reverse_first {
            (self.reverse.as_ref().unwrap(), Some(forward))
        } else {
            (forward, self.reverse.as_ref())
        };
        let period = first.duration() + second.map_or(Duration::ZERO, |t| t.duration());
        if period.is_zero() {
            return motion_timeline::Sample {
                frame: forward.initial(),
                completed: 0,
                finished: false,
                next: Next::Idle,
            };
        }
        // Validated cycle durations are <= one day apiece, so this remainder fits u64.
        let phase = Duration::from_nanos((elapsed.as_nanos() % period.as_nanos()) as u64);
        let mut sample = if phase < first.duration() {
            first.sample(phase)
        } else {
            second
                .expect("nonzero second interval")
                .sample(phase - first.duration())
        };
        sample.completed = 0;
        sample.finished = false;
        sample
    }
    fn sample(&self, elapsed: Duration, repeat: Repeat, shared: bool) -> motion_timeline::Sample {
        if shared
            || (repeat != Repeat::Once
                && self.first.is_static()
                && self.forward.as_ref().is_some_and(|track| track.is_static()))
        {
            return self.repeat(elapsed, false);
        }
        if repeat == Repeat::Once || elapsed < self.first.duration() {
            return self.first.sample(elapsed);
        }
        self.repeat(elapsed - self.first.duration(), repeat == Repeat::Alternate)
    }
}
#[derive(Clone, Debug)]
pub struct Sample {
    pub frame: Frame,
    pub wake: Wake,
    at: Duration,
    run: i64,
    epoch: Arc<()>,
    clock: Option<ClockSnapshot>,
    completed: usize,
    finished: bool,
    reduced: bool,
}
pub struct State {
    config: Arc<Config>,
    tracks: Tracks,
    painted: Frame,
    run: i64,
    completed: usize,
    finished: bool,
    start: Duration,
    last_time: Duration,
    last_paint: Duration,
    paused_at: Option<Duration>,
    visible: bool,
    reduced: bool,
    epoch: Arc<()>,
}
impl State {
    pub fn new(config: Arc<Config>, now: Duration, reduced: bool) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        let tracks = Tracks::compile(&config.program, None)?;
        let painted = tracks.first.initial();
        let paused_at = (reduced || config.playback == Playback::Paused).then_some(now);
        Ok(Self {
            run: config.generation,
            finished: config.playback == Playback::Cancelled,
            config,
            tracks,
            painted,
            completed: 0,
            start: now,
            last_time: now,
            last_paint: now,
            paused_at,
            visible: true,
            reduced,
            epoch: Arc::new(()),
        })
    }
    fn time(&mut self, now: Duration) -> Duration {
        self.last_time = self.last_time.max(now);
        self.last_time
    }
    fn pause(&mut self, now: Duration) {
        if !self.visible || self.reduced || self.config.playback == Playback::Paused {
            self.paused_at.get_or_insert(now);
        } else if let Some(paused) = self.paused_at.take() {
            self.start = self.start.saturating_add(now.saturating_sub(paused));
        }
    }
    pub fn set_visible(&mut self, visible: bool, now: Duration) {
        let now = self.time(now);
        if self.visible != visible {
            self.epoch = Arc::new(());
        }
        self.visible = visible;
        self.pause(now);
    }
    pub fn set_reduced(&mut self, reduced: bool, now: Duration) {
        let now = self.time(now);
        if self.reduced != reduced {
            self.epoch = Arc::new(());
        }
        self.reduced = reduced;
        self.pause(now);
    }
    pub fn run_generation(&self) -> i64 {
        self.run
    }
    pub fn is_finished(&self) -> bool {
        self.finished
    }
    /// Session quota for the maximum first/forward/reverse tracks, configuration
    /// copies and adapter bookkeeping. This is a conservative admission unit,
    /// not process RSS; a replacement also has one bounded transient compilation.
    pub fn reservation(config: &Config) -> usize {
        let tracks = match config.program.repeat {
            Repeat::Once => 1,
            Repeat::Loop => 2,
            Repeat::Alternate => 3,
        };
        std::mem::size_of::<Self>()
            + 3 * config.retained_bytes()
            + 4096
            + tracks * Timeline::reservation(&config.program)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.config.retained_bytes() + self.tracks.retained_bytes()
    }
    pub fn cancel(&mut self, reason: CancelReason) -> Option<Signal> {
        if self.finished {
            return None;
        }
        self.finished = true;
        self.epoch = Arc::new(());
        self.painted = motion_timeline::rest(self.painted.values);
        Some(Signal {
            generation: self.run,
            index: TERMINAL_INDEX,
            observation: Observation::Cancelled(reason),
        })
    }
    /// Compile before changing any accepted state. A reset token can restart the
    /// same program; rebuilding a different body with its default token retargets.
    pub fn update(&mut self, config: Arc<Config>, now: Duration) -> Result<Option<Signal>, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if config == self.config {
            return Ok(None);
        }
        if config.generation <= self.config.generation {
            return Err(Error::StaleGeneration);
        }
        if config.program == self.config.program && config.restart < self.config.restart {
            return Err(Error::StaleRestart);
        }
        if config.same_run(&self.config) {
            let now = self.time(now);
            self.epoch = Arc::new(());
            self.config = config;
            self.pause(now);
            return Ok(if self.config.playback == Playback::Cancelled {
                self.cancel(CancelReason::Requested)
            } else {
                None
            });
        }
        let reset = config.restart > self.config.restart;
        let tracks = Tracks::compile(
            &config.program,
            (!reset && !config.program.clock.is_shared()).then_some(self.painted),
        )?;
        let now = self.time(now);
        let cancelled = self.cancel(CancelReason::Replaced);
        self.epoch = Arc::new(());
        self.run = config.generation;
        self.completed = 0;
        self.start = now;
        self.last_paint = now;
        self.finished = config.playback == Playback::Cancelled;
        self.painted = tracks.first.initial();
        self.tracks = tracks;
        self.config = config;
        self.paused_at =
            (!self.visible || self.reduced || self.config.playback == Playback::Paused)
                .then_some(now);
        Ok(cancelled)
    }
    pub fn sample(&mut self, now: Duration, clock: Option<ClockSnapshot>) -> Result<Sample, Error> {
        let now = self.time(now);
        let mut sample = Sample {
            frame: self.painted,
            wake: Wake::Idle,
            at: now,
            run: self.run,
            epoch: self.epoch.clone(),
            clock: None,
            completed: self.completed,
            finished: false,
            reduced: false,
        };
        if self.finished || !self.visible || self.config.playback == Playback::Paused {
            return Ok(sample);
        }
        if self.reduced {
            if self.config.program.repeat == Repeat::Once {
                sample.frame = self.tracks.first.final_frame();
                sample.completed = self.config.program.stages.len();
                sample.finished = true;
                sample.reduced = true;
            } else {
                sample.frame = self.tracks.forward.as_ref().unwrap().initial();
            }
            return Ok(sample);
        }
        let shared = self.config.program.clock.is_shared();
        let (elapsed, paused) = if shared {
            let clock = clock.ok_or(Error::MissingClock)?;
            if !clock.matches(&self.config.program.clock) {
                return Err(Error::WrongClock);
            }
            if !clock.is_current() {
                return Err(Error::StaleClock);
            }
            let values = (clock.elapsed, clock.paused);
            sample.clock = Some(clock);
            values
        } else {
            (now.saturating_sub(self.start), false)
        };
        let result = self
            .tracks
            .sample(elapsed, self.config.program.repeat, shared);
        sample.frame = result.frame;
        if self.config.program.repeat == Repeat::Once {
            sample.completed = result.completed;
            sample.finished = result.finished;
        }
        if !paused {
            sample.wake = match result.next {
                Next::Idle => Wake::Idle,
                Next::Frame => Wake::Frame,
                Next::Wait(delay) => Wake::At(now.saturating_add(delay)),
            };
        }
        Ok(sample)
    }
    fn owns_sample(&self, sample: &Sample) -> bool {
        !self.finished
            && self.visible
            && sample.run == self.run
            && Arc::ptr_eq(&sample.epoch, &self.epoch)
            && sample.clock.as_ref().is_none_or(ClockSnapshot::is_current)
    }
    pub fn accepts_sample(&self, sample: &Sample) -> bool {
        self.owns_sample(sample) && sample.at >= self.last_paint
    }
    /// A later paint must not invalidate an otherwise current delayed wake.
    /// The adapter additionally checks that this is its currently owned deadline.
    pub fn accepts_wake(&self, sample: &Sample) -> bool {
        self.owns_sample(sample)
            && self.config.playback == Playback::Running
            && sample.wake != Wake::Idle
    }
    /// Bounded to the admitted stage count plus one terminal signal. Repeats emit
    /// no stage/cycle observations. The adapter must atomically admit this batch.
    pub fn painted(&mut self, sample: Sample) -> Vec<Signal> {
        if !self.accepts_sample(&sample) {
            return vec![];
        }
        self.painted = sample.frame;
        self.last_paint = sample.at;
        let mut events = Vec::with_capacity(
            sample.completed.saturating_sub(self.completed) + usize::from(sample.finished),
        );
        for stage in self.completed..sample.completed {
            events.push(Signal {
                generation: self.run,
                index: stage as i64 + 1,
                observation: Observation::StageCompleted(
                    stage as i64,
                    if sample.reduced {
                        StageResult::ReducedMotion
                    } else {
                        StageResult::Played
                    },
                ),
            });
        }
        self.completed = sample.completed;
        if sample.finished {
            self.finished = true;
            events.push(Signal {
                generation: self.run,
                index: TERMINAL_INDEX,
                observation: Observation::Finished,
            });
        }
        events
    }
}

#[cfg(test)]
#[path = "motion_program_test.rs"]
mod tests;
