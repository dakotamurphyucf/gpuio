//! Retained progress presentation. Semantic targets stay in the accepted tree.
use gpui::{App, Window};
use gpuio_protocol::{
    animation::Easing,
    progress_presentation::{Config, Shape, Transition},
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
    time::{Duration, Instant},
};

/// Native owner/map entry and one weak wake, excluding its shared configuration.
/// Admission allowance, not physical allocator/RSS accounting.
pub const RESERVED_BYTES: usize = 768;

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
    #[cfg(any(test, feature = "native-image-tests"))]
    pub(crate) fn set_test_time(&self, now: Duration) {
        self.test_now.set(Some(now));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Determinate(f64),
    Indeterminate {
        phase: f32,
        static_presentation: bool,
    },
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub shape: Shape,
    pub value: Value,
}

struct Tween {
    from: f64,
    target: f64,
    elapsed: Duration,
    duration: Duration,
    easing: Easing,
}
enum Motion {
    Determinate {
        displayed: f64,
        tween: Option<Tween>,
    },
    Indeterminate {
        elapsed: Duration,
    },
}
impl Motion {
    fn initial(target: Option<f64>) -> Self {
        match target {
            Some(displayed) => Self::Determinate {
                displayed,
                tween: None,
            },
            None => Self::Indeterminate {
                elapsed: Duration::ZERO,
            },
        }
    }
    fn needs_frame(&self) -> bool {
        match self {
            Self::Determinate { tween, .. } => tween.is_some(),
            Self::Indeterminate { .. } => true,
        }
    }
}

struct State {
    config: Arc<Config>,
    clock: Rc<Clock>,
    stamp: Rc<()>,
    last: Duration,
    motion: Motion,
    visible: bool,
    prepared: bool,
    painted: bool,
    running: bool,
    resume_after_layout: bool,
    pending: bool,
}
impl State {
    fn period(&self) -> Duration {
        Duration::from_millis(match self.config.shape {
            Shape::Linear => 1500,
            Shape::Circle => 1000,
        })
    }
    fn advance(&mut self) {
        let now = self.clock.now().max(self.last);
        let elapsed = now - self.last;
        self.last = now;
        if !self.running {
            return;
        }
        let period = self.period();
        match &mut self.motion {
            Motion::Determinate { displayed, tween } => {
                if let Some(active) = tween {
                    active.elapsed = active.elapsed.saturating_add(elapsed).min(active.duration);
                    if active.elapsed == active.duration {
                        *displayed = active.target;
                        *tween = None;
                    } else {
                        let phase = active.elapsed.as_secs_f64() / active.duration.as_secs_f64();
                        *displayed = (active.from
                            + (active.target - active.from) * active.easing.sample(phase))
                        .clamp(0., 1.);
                    }
                }
            }
            Motion::Indeterminate { elapsed: phase } => {
                *phase = Duration::from_nanos(
                    (phase.saturating_add(elapsed).as_nanos() % period.as_nanos()) as u64,
                );
            }
        }
        self.running = self.motion.needs_frame();
    }
    fn settle(&mut self) {
        if let Some(target) = self.config.progress.fraction {
            self.motion = Motion::initial(Some(target));
        }
        self.running = false;
        self.resume_after_layout = false;
    }
    fn suspend(&mut self) {
        // Do not replay time spent in a layout that never painted.
        self.advance();
        self.settle();
        self.visible = false;
        self.painted = false;
    }
    fn prepare(&mut self) {
        if !self.prepared {
            self.advance();
            self.resume_after_layout = self.running;
            self.running = false;
            self.painted = false;
            self.prepared = true;
        }
    }
    fn finish(&mut self) {
        if !self.painted {
            self.suspend();
        }
        self.resume_after_layout = false;
        self.prepared = false;
    }
    fn sample(&mut self, static_presentation: bool) -> Sample {
        self.running |= std::mem::take(&mut self.resume_after_layout);
        self.advance();
        if static_presentation {
            self.settle();
        }
        let value = match &self.motion {
            Motion::Determinate { displayed, .. } => Value::Determinate(*displayed),
            Motion::Indeterminate { elapsed } => Value::Indeterminate {
                phase: if static_presentation {
                    0.
                } else {
                    ((elapsed.as_secs_f64() / self.period().as_secs_f64()) as f32).fract()
                },
                static_presentation,
            },
        };
        Sample {
            shape: self.config.shape,
            value,
        }
    }
    fn painted(&mut self, visible: bool, static_presentation: bool) -> bool {
        if !visible {
            self.suspend();
            return false;
        }
        self.visible = true;
        self.painted = true;
        if static_presentation {
            self.settle();
        }
        self.running = !static_presentation && self.motion.needs_frame();
        if self.running && !self.pending {
            self.pending = true;
            true
        } else {
            false
        }
    }
    fn delivered(&mut self, reduced: bool) -> bool {
        self.pending = false;
        // Acceptance can prepare an owner before its pending frame callback
        // runs. Keep that interval until paint resumes it or finish discards it.
        let redraw = self.running || self.resume_after_layout;
        if reduced {
            self.settle();
        }
        // An active final wake must still draw the exact target (or reduced
        // static state). Do not settle elapsed time and suppress that paint.
        redraw
    }
    fn update(&mut self, config: Arc<Config>) {
        if self.config == config {
            self.config = config;
            return;
        }
        self.running |= std::mem::take(&mut self.resume_after_layout);
        self.advance();
        let retarget = config.progress.fraction != self.config.progress.fraction
            || config.transition != self.config.transition;
        match (config.progress.fraction, &mut self.motion) {
            (Some(target), Motion::Determinate { displayed, tween }) if retarget => {
                *tween = match config.transition {
                    Transition::Tween {
                        duration_ms,
                        easing,
                    } if self.visible && *displayed != target => Some(Tween {
                        from: *displayed,
                        target,
                        elapsed: Duration::ZERO,
                        duration: Duration::from_millis(duration_ms as u64),
                        easing,
                    }),
                    Transition::Immediate | Transition::Tween { .. } => {
                        *displayed = target;
                        None
                    }
                };
                self.running = false; // The first new paint starts the new interval.
            }
            (Some(_), Motion::Determinate { .. }) => (),
            (None, Motion::Indeterminate { elapsed }) => {
                if config.shape != self.config.shape {
                    *elapsed = Duration::ZERO;
                    self.running = false;
                }
            }
            (target, _) => {
                self.motion = Motion::initial(target);
                self.running = false;
            }
        }
        self.config = config;
        self.stamp = Rc::new(());
    }
}

/// One strong owner per mounted presentation. Rendered elements and pending
/// native callbacks borrow it weakly; the adapter must remove it on window close.
pub struct Owner(Rc<RefCell<State>>);
impl Owner {
    pub fn new(config: Arc<Config>, clock: Rc<Clock>) -> Result<Self, InvalidConfig> {
        if !config.is_valid() {
            return Err(InvalidConfig);
        }
        let last = clock.now();
        let motion = Motion::initial(config.progress.fraction);
        Ok(Self(Rc::new(RefCell::new(State {
            config,
            clock,
            last,
            motion,
            stamp: Rc::new(()),
            visible: false,
            prepared: false,
            painted: false,
            running: false,
            resume_after_layout: false,
            pending: false,
        }))))
    }
    /// Invalid updates leave the old owner/configuration untouched.
    pub fn update(&self, config: Arc<Config>) -> Result<(), InvalidConfig> {
        if !config.is_valid() {
            return Err(InvalidConfig);
        }
        self.0.borrow_mut().update(config);
        Ok(())
    }
    pub fn prepare_frame(&self) {
        self.0.borrow_mut().prepare();
    }
    pub fn finish_frame(&self) {
        self.0.borrow_mut().finish();
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
    pub fn sample(&self, static_presentation: bool) -> Option<Sample> {
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
        let Some(state) = self.current() else {
            return;
        };
        if !state
            .borrow_mut()
            .painted(visible, static_presentation || cx.reduce_motion())
        {
            return;
        }
        let weak = Rc::downgrade(&state);
        let view = window.current_view();
        window.on_next_frame(move |_, cx| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            if state.borrow_mut().delivered(cx.reduce_motion()) {
                cx.notify(view);
            }
        });
    }
}

#[cfg(test)]
#[path = "progress_clock_test.rs"]
mod tests;
