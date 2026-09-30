//! Native retained timing. Owners are held by the mounted node/window; elements
//! and queued frame callbacks borrow them weakly. No OCaml timer or callback.
use crate::text_shimmer_paint::{self as paint, Appearance, MAX_TEXT_BYTES, Report, Sample};
use gpui::{App, StyledText, Window};
use gpuio_protocol::text_shimmer::{Config, Repeat};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
    time::{Duration, Instant},
};

pub struct Clock {
    origin: Instant,
    #[cfg(any(test, feature = "native-image-tests"))]
    test_now: std::cell::Cell<Option<Duration>>,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
            #[cfg(any(test, feature = "native-image-tests"))]
            test_now: Default::default(),
        }
    }
}

impl Clock {
    fn now(&self) -> Duration {
        #[cfg(any(test, feature = "native-image-tests"))]
        if let Some(now) = self.test_now.get() {
            return now;
        }
        self.origin.elapsed()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    SourceLimit,
}

fn validate(source: &str, config: Config) -> Result<(), Error> {
    if !config.is_valid() {
        Err(Error::InvalidConfig)
    } else if source.len() > MAX_TEXT_BYTES {
        Err(Error::SourceLimit)
    } else {
        Ok(())
    }
}

struct State {
    clock: Rc<Clock>,
    source: Arc<str>,
    config: Config,
    has_text: bool,
    stamp: Rc<()>,
    last: Duration,
    elapsed: Duration,
    running: bool,
    pending: bool,
    #[cfg(feature = "native-image-tests")]
    notifications: usize,
}

impl State {
    fn advance(&mut self, now: Duration) {
        let now = now.max(self.last);
        if self.running {
            self.elapsed = self.elapsed.saturating_add(now - self.last);
            if self.config.repeat == Repeat::Once {
                self.elapsed = self.elapsed.min(self.duration());
            } else {
                self.elapsed = Duration::from_nanos(
                    (self.elapsed.as_nanos() % self.duration().as_nanos()) as u64,
                );
            }
        }
        self.last = now;
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(self.config.duration_ms as u64)
    }

    fn finished(&self) -> bool {
        self.config.repeat == Repeat::Once && self.elapsed >= self.duration()
    }

    fn suspend(&mut self) {
        self.advance(self.clock.now());
        self.running = false;
    }

    fn sample(&mut self, reduced: bool) -> Sample {
        self.advance(self.clock.now());
        if reduced || !self.config.animated {
            self.running = false;
        }
        Sample {
            phase: (self.elapsed.as_secs_f64() / self.duration().as_secs_f64()) as f32,
            reduced_motion: reduced,
        }
    }

    fn painted(&mut self, report: Report, reduced: bool) -> bool {
        self.running = self.has_text
            && self.config.animated
            && !reduced
            && !self.finished()
            && matches!(report, Report::OutsideBand | Report::Painted { .. });
        if self.running && !self.pending {
            self.pending = true;
            true
        } else {
            false
        }
    }

    fn delivered(&mut self, reduced: bool) -> bool {
        self.pending = false;
        if reduced {
            self.suspend();
            false
        } else {
            self.running
        }
    }
}

/// One native text-node lifetime. Deliberately not Clone: the retained tree/view
/// owns this strong handle; rendered elements and pending wakes cannot extend it.
pub struct Owner(Rc<RefCell<State>>);

impl Owner {
    pub fn new(source: Arc<str>, config: Config, clock: Rc<Clock>) -> Result<Self, Error> {
        validate(&source, config)?;
        let last = clock.now();
        Ok(Self(Rc::new(RefCell::new(State {
            has_text: source.chars().any(|c| !c.is_whitespace()),
            source,
            config,
            clock,
            last,
            stamp: Rc::new(()),
            elapsed: Duration::ZERO,
            running: false,
            pending: false,
            #[cfg(feature = "native-image-tests")]
            notifications: 0,
        }))))
    }

    /// Invalid updates have no effect. Source/timing changes restart; spread,
    /// highlight and external style changes preserve elapsed time. Turning off
    /// animation pauses; a later paint resumes it. Completed Once stays complete
    /// until a source/timing change or a new owner is mounted.
    pub fn update(&self, source: Arc<str>, config: Config) -> Result<(), Error> {
        validate(&source, config)?;
        let mut state = self.0.borrow_mut();
        if state.source == source && state.config == config {
            return Ok(());
        }
        let restart = state.source != source
            || state.config.duration_ms != config.duration_ms
            || state.config.direction != config.direction
            || state.config.repeat != config.repeat;
        let now = state.clock.now();
        state.advance(now);
        if restart {
            state.elapsed = Duration::ZERO;
            state.running = false;
        }
        if !config.animated {
            state.running = false;
        }
        state.has_text = source.chars().any(|c| !c.is_whitespace());
        state.source = source;
        state.config = config;
        state.stamp = Rc::new(());
        // Keep the one pending callback. It carries no source/config/phase and
        // checks current eligibility on delivery; replacing it could accumulate
        // arbitrarily many obsolete callbacks between platform frames.
        Ok(())
    }

    /// Call when a retained owner is omitted by a hidden branch. Unmounting or
    /// closing its window should drop the Owner instead. At most one queued wake
    /// can remain; it observes the suspension and does not notify the view.
    pub fn suspend(&self) {
        self.0.borrow_mut().suspend();
    }

    /// Supplied text must be the source of this owner. The native adapter keeps
    /// the source and its styled layout together; styles do not affect timing.
    pub fn element(&self, text: StyledText, appearance: Appearance) -> paint::Text {
        let mut state = self.0.borrow_mut();
        // Disarm before layout. Fully clipped elements can skip paint entirely.
        state.suspend();
        let config = state.config;
        let sample = state.sample(false);
        let driver = Driver {
            state: Rc::downgrade(&self.0),
            stamp: Rc::downgrade(&state.stamp),
        };
        paint::element(text, config, sample, appearance).with_driver(driver)
    }
}

pub(crate) struct Driver {
    state: Weak<RefCell<State>>,
    stamp: Weak<()>,
}

impl Driver {
    fn current(&self) -> Option<Rc<RefCell<State>>> {
        let stamp = self.stamp.upgrade()?;
        let state = self.state.upgrade()?;
        if Rc::ptr_eq(&stamp, &state.borrow().stamp) {
            Some(state)
        } else {
            None
        }
    }

    pub(crate) fn sample(&self, reduced: bool) -> Option<(Config, Sample)> {
        let state = self.current()?;
        let mut state = state.borrow_mut();
        let sample = state.sample(reduced);
        Some((state.config, sample))
    }

    pub(crate) fn painted(&self, report: Report, window: &mut Window, cx: &mut App) {
        let Some(state) = self.current() else { return };
        if !state.borrow_mut().painted(report, cx.reduce_motion()) {
            return;
        }
        let weak = Rc::downgrade(&state);
        let target = window.current_view();
        window.on_next_frame(move |_, cx| {
            let Some(state) = weak.upgrade() else { return };
            if state.borrow_mut().delivered(cx.reduce_motion()) {
                #[cfg(feature = "native-image-tests")]
                {
                    state.borrow_mut().notifications += 1;
                }
                cx.notify(target);
            }
        });
    }
}

#[cfg(test)]
#[path = "text_shimmer_clock_test.rs"]
mod tests;

#[cfg(feature = "native-image-tests")]
#[path = "text_shimmer_clock_native_test.rs"]
pub(crate) mod native_test;
