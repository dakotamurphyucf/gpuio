//! Retained native spinner time. Elements and frame callbacks hold weak owners.
use gpui::{App, Window};
use gpuio_protocol::spinner::Config;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
    time::{Duration, Instant},
};

/// Admission allowance for one owner/map entry and weak wake, excluding the
/// separately charged shared configuration. Not an RSS/allocator measurement.
pub const RESERVED_BYTES: usize = 512;

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
    #[cfg(any(test, feature = "native-image-tests"))]
    pub(crate) fn set_test_time(&self, now: Duration) {
        self.test_now.set(Some(now));
    }
    fn now(&self) -> Duration {
        #[cfg(any(test, feature = "native-image-tests"))]
        if let Some(now) = self.test_now.get() {
            return now;
        }
        self.origin.elapsed()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidConfig;

struct State {
    config: Arc<Config>,
    clock: Rc<Clock>,
    stamp: Rc<()>,
    last: Duration,
    elapsed: Duration,
    running: bool,
    resume_after_layout: bool,
    pending: bool,
}
impl State {
    fn advance(&mut self) {
        let now = self.clock.now().max(self.last);
        if self.running {
            let duration = Duration::from_millis(self.config.period_ms as u64);
            self.elapsed = Duration::from_nanos(
                (self.elapsed.saturating_add(now - self.last).as_nanos() % duration.as_nanos())
                    as u64,
            );
        }
        self.last = now;
    }
    fn suspend(&mut self) {
        self.advance();
        self.running = false;
        self.resume_after_layout = false;
    }
    fn prepare(&mut self) {
        if !self.resume_after_layout {
            self.advance();
            self.resume_after_layout = self.running;
            self.running = false;
        }
    }
    fn sample(&mut self, static_presentation: bool) -> f32 {
        self.running |= std::mem::take(&mut self.resume_after_layout);
        self.advance();
        if static_presentation || !self.config.animated {
            self.running = false;
            return 0.;
        }
        let phase = self.elapsed.as_secs_f64() / (self.config.period_ms as f64 / 1000.);
        // Keep arbitrary finite overshoot in f64 until reduced to a single turn.
        (self.config.easing.sample(phase).rem_euclid(1.) as f32).fract()
    }
    fn painted(&mut self, visible: bool, static_presentation: bool) -> bool {
        self.running = visible && !static_presentation && self.config.animated;
        if self.running && !self.pending {
            self.pending = true;
            true
        } else {
            false
        }
    }
    fn delivered(&mut self, reduced: bool) -> bool {
        self.pending = false;
        // Update preparation may precede this callback. Only paint or the
        // matching finish may decide whether to count that layout interval.
        if reduced {
            self.suspend();
            false
        } else {
            self.running || self.resume_after_layout
        }
    }
}

/// Exactly one strong owner per retained spinner node; never store this in an
/// element or pending callback. Each window may share one monotonic Clock.
pub struct Owner(Rc<RefCell<State>>);
impl Owner {
    pub fn new(config: Arc<Config>, clock: Rc<Clock>) -> Result<Self, InvalidConfig> {
        if !config.is_valid() {
            return Err(InvalidConfig);
        }
        let last = clock.now();
        Ok(Self(Rc::new(RefCell::new(State {
            config,
            clock,
            last,
            stamp: Rc::new(()),
            elapsed: Duration::ZERO,
            running: false,
            resume_after_layout: false,
            pending: false,
        }))))
    }
    /// Invalid updates are atomic. Cosmetic label changes preserve phase;
    /// timing/source/animated changes restart. Decode and density changes are
    /// independent image state, so they do not update this configuration.
    pub fn update(&self, config: Arc<Config>) -> Result<(), InvalidConfig> {
        if !config.is_valid() {
            return Err(InvalidConfig);
        }
        let mut state = self.0.borrow_mut();
        if state.config == config {
            state.config = config;
            return Ok(());
        }
        let restart = state.config.period_ms != config.period_ms
            || state.config.easing != config.easing
            || state.config.animated != config.animated
            || state.config.source != config.source;
        state.running |= std::mem::take(&mut state.resume_after_layout);
        state.advance();
        if restart {
            state.elapsed = Duration::ZERO;
            state.running = false;
            state.resume_after_layout = false;
        }
        state.config = config;
        state.stamp = Rc::new(());
        // The one queued wake carries no stale configuration. Keep it pending
        // rather than accumulating a callback for every intermediate update.
        Ok(())
    }
    /// Disarm every retained owner before a view builds its frame, including
    /// owners omitted from the element tree. Pair with finish_frame after paint.
    pub fn prepare_frame(&self) {
        self.0.borrow_mut().prepare();
    }
    pub fn finish_frame(&self) {
        self.0.borrow_mut().resume_after_layout = false;
    }
    pub fn suspend(&self) {
        self.0.borrow_mut().suspend();
    }
    pub fn driver(&self) -> Driver {
        let mut state = self.0.borrow_mut();
        state.prepare();
        Driver {
            state: Rc::downgrade(&self.0),
            stamp: Rc::downgrade(&state.stamp),
        }
    }
}

/// Weak, configuration-stamped paint access. Stale elements cannot restart an
/// updated or removed owner, nor keep configuration/source state alive.
pub struct Driver {
    state: Weak<RefCell<State>>,
    stamp: Weak<()>,
}
impl Driver {
    fn current(&self) -> Option<Rc<RefCell<State>>> {
        let stamp = self.stamp.upgrade()?;
        let state = self.state.upgrade()?;
        let current = Rc::ptr_eq(&stamp, &state.borrow().stamp);
        current.then_some(state)
    }
    pub fn sample(&self, static_presentation: bool) -> Option<f32> {
        self.current()
            .map(|state| state.borrow_mut().sample(static_presentation))
    }
    pub fn suspend(&self) {
        if let Some(state) = self.current() {
            state.borrow_mut().suspend();
        }
    }
    pub fn painted(
        &self,
        visible: bool,
        static_presentation: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(state) = self.current() else { return };
        if !state
            .borrow_mut()
            .painted(visible, static_presentation || cx.reduce_motion())
        {
            return;
        }
        let weak = Rc::downgrade(&state);
        let view = window.current_view();
        window.on_next_frame(move |_, cx| {
            let Some(state) = weak.upgrade() else { return };
            if state.borrow_mut().delivered(cx.reduce_motion()) {
                cx.notify(view);
            }
        });
    }
}

#[cfg(test)]
#[path = "spinner_clock_test.rs"]
mod tests;
