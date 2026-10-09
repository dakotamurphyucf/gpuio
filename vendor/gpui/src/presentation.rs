//! Opt-in, bounded native presentation diagnostics. Callback data has no window,
//! entity, renderer or drawable ownership. Host-clock timestamps are converted
//! using bracketed samples, never by subtracting an unrelated Instant epoch.
use crate::WindowId;
use hdrhistogram::Histogram;
use scheduler::Instant;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    marker::PhantomData,
    rc::Rc,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

/// Checked per-session storage limits.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pending: usize,
    trace: usize,
}
impl Limits {
    /// Admit 1..=128 unfinished frames and retain at most 4096 raw results.
    /// Trace truncation is counted independently from measurement loss.
    pub fn new(pending: usize, trace: usize) -> Result<Self, StartError> {
        if !(1..=128).contains(&pending) || trace > 4096 {
            return Err(StartError::InvalidLimits);
        }
        Ok(Self { pending, trace })
    }
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            pending: 128,
            trace: 4096,
        }
    }
}
/// Session admission errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartError {
    /// Another session (including one settling after stop) still owns this window.
    AlreadyStarted,
    /// Requested storage exceeds the fixed bounds.
    InvalidLimits,
    /// The process exhausted its session identifier space.
    IdentifiersExhausted,
}

/// Distinct terminal outcomes; zero/missing records are never latency samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Valid presentation timestamp and clock correspondence.
    Presented,
    /// The platform draw did not attach a supported native drawable.
    NotSubmitted,
    /// The native callback was released without reporting a presentation.
    Missing,
    /// The drawable reported zero (unpresented/dropped).
    Zero,
    /// Non-finite, reversed or inconsistent timestamps.
    InvalidClock,
}
/// Inclusive conservative latency bounds, in nanoseconds.
#[derive(Clone, Copy, Debug)]
pub struct LatencyBounds {
    /// Earliest possible duration after the bracketed clock conversion.
    pub lower_ns: u64,
    /// Latest possible duration; used for acceptance histograms.
    pub upper_ns: u64,
}
/// One admitted frame, in submission order when exported.
#[derive(Clone, Debug)]
pub struct Record {
    /// Session-local platform submission sequence.
    pub sequence: u64,
    /// Native drawable identity; absent if no callback was attached.
    pub drawable: Option<u64>,
    /// Whether this submission contains a newly drawn scene.
    pub new_scene: bool,
    /// Whether the GPUI window was active at submission.
    pub active: bool,
    /// Whether another frame was scheduled for this scene.
    pub animating: bool,
    /// Input events coalesced into this submission by the native profiler.
    pub inputs: u64,
    /// Outcome of this drawable/attempt.
    pub outcome: Outcome,
    /// Bracketed host-clock sample before native submission.
    pub submit_host_s: Option<f64>,
    /// Native presentation time, including zero when reported.
    pub presented_host_s: Option<f64>,
    /// Callback host-clock arrival, distinct from native presentation.
    pub callback_host_s: Option<f64>,
    /// Submission host sample to presentation, conservatively rounded.
    pub submission_latency: Option<LatencyBounds>,
    /// Oldest contributing native input to presentation, if present.
    pub input_latency: Option<LatencyBounds>,
}
/// Cumulative counts, including all outcomes excluded from histograms.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    /// Platform attempts while admission is enabled.
    pub attempted: u64,
    /// Attempts admitted to the bounded pending map.
    pub admitted: u64,
    /// Attempts rejected because the pending map was full.
    pub saturated: u64,
    /// Valid presented frames.
    pub presented: u64,
    /// Attempts that attached no supported drawable.
    pub not_submitted: u64,
    /// Callback holders released without a result.
    pub missing: u64,
    /// Zero native presentation timestamps.
    pub zero: u64,
    /// Invalid clock samples or backwards presentation order.
    pub invalid_clock: u64,
    /// Extra callback invocations for an already completed ticket.
    pub duplicate_callbacks: u64,
    /// Results beyond raw trace capacity (histograms are still updated).
    pub trace_truncated: u64,
    /// Histogram values outside the explicit 60-second duration bound.
    pub histogram_overflow: u64,
}
/// Owned cumulative diagnostics; no platform/window handles.
#[derive(Clone)]
pub struct Snapshot {
    /// Process-unique session identity.
    pub session: u64,
    /// GPUI window identity captured at session creation.
    pub window: WindowId,
    /// New attempts can still be admitted.
    pub accepting: bool,
    /// The owning WindowProfiler was destroyed.
    pub window_closed: bool,
    /// Admitted results still awaiting ordered settlement.
    pub pending: usize,
    /// Complete outcome accounting.
    pub counts: Counts,
    /// First bounded raw results in submission order.
    pub trace: Vec<Record>,
    /// Submission sample to native presentation (nanoseconds).
    pub submission_latency: Histogram<u64>,
    /// Oldest input to native presentation, using upper uncertainty bounds.
    pub input_latency: Histogram<u64>,
    /// Successive active animation presentation intervals (nanoseconds).
    pub animation_interval: Histogram<u64>,
}

struct Pending {
    metadata: Metadata,
    result: Option<Record>,
}
struct State {
    snapshot: Snapshot,
    limits: Limits,
    next_sequence: u64,
    pending: BTreeMap<u64, Pending>,
    previous: Option<(u64, f64, bool)>,
    last_presented: Option<f64>,
}
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// A window-scoped collection lifetime. Dropping stops admission. Admitted
/// callback holders contain only Weak references and cannot prolong this state.
pub struct Session {
    state: Arc<Mutex<State>>,
}
impl Session {
    /// Read unfinished admission count without cloning trace or histograms.
    pub fn pending(&self) -> usize {
        lock(&self.state).pending.len()
    }

    /// Copy bounded counters, raw results and cumulative histograms.
    pub fn snapshot(&self) -> Snapshot {
        let state = lock(&self.state);
        let mut snapshot = state.snapshot.clone();
        snapshot.pending = state.pending.len();
        snapshot
    }
    /// Stop new admission without discarding unfinished records. Poll snapshots
    /// until settled or report pending frames at the caller's explicit deadline.
    pub fn stop(&self) {
        lock(&self.state).snapshot.accepting = false;
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) struct Controller {
    window: WindowId,
    state: Mutex<Weak<Mutex<State>>>,
}
impl Controller {
    pub(crate) fn new(window: WindowId) -> Self {
        Self {
            window,
            state: Mutex::new(Weak::new()),
        }
    }
    pub(crate) fn start(&self, limits: Limits) -> Result<Session, StartError> {
        let mut slot = lock(&self.state);
        if slot.upgrade().is_some() {
            return Err(StartError::AlreadyStarted);
        }
        let session = NEXT_SESSION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| StartError::IdentifiersExhausted)?;
        let histogram =
            || Histogram::new_with_bounds(1, 60_000_000_000, 3).expect("fixed valid bounds");
        let state = Arc::new(Mutex::new(State {
            snapshot: Snapshot {
                session,
                window: self.window,
                accepting: true,
                window_closed: false,
                pending: 0,
                counts: Counts::default(),
                trace: Vec::with_capacity(limits.trace),
                submission_latency: histogram(),
                input_latency: histogram(),
                animation_interval: histogram(),
            },
            limits,
            next_sequence: 0,
            pending: BTreeMap::new(),
            previous: None,
            last_presented: None,
        }));
        *slot = Arc::downgrade(&state);
        Ok(Session { state })
    }
    pub(crate) fn enter(&self, metadata: Metadata) -> ContextGuard {
        let attempt = lock(&self.state).upgrade().and_then(|shared| {
            let mut state = lock(&shared);
            if !state.snapshot.accepting {
                return None;
            }
            state.snapshot.counts.attempted += 1;
            let sequence = state.next_sequence;
            state.next_sequence += 1;
            if state.pending.len() == state.limits.pending {
                state.snapshot.counts.saturated += 1;
                return None;
            }
            state.snapshot.counts.admitted += 1;
            state.pending.insert(
                sequence,
                Pending {
                    metadata,
                    result: None,
                },
            );
            Some(Arc::new(Attempt {
                state: Arc::downgrade(&shared),
                sequence,
                attached: AtomicBool::new(false),
            }))
        });
        let previous = CONTEXT.with(|slot| slot.replace(attempt.clone()));
        ContextGuard {
            previous,
            current: attempt,
            thread: PhantomData,
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        if let Some(state) = lock(&self.state).upgrade() {
            let mut state = lock(&state);
            state.snapshot.accepting = false;
            state.snapshot.window_closed = true;
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Metadata {
    pub input: Option<Instant>,
    pub inputs: u64,
    pub new_scene: bool,
    pub active: bool,
    pub animating: bool,
}
struct Attempt {
    state: Weak<Mutex<State>>,
    sequence: u64,
    attached: AtomicBool,
}
thread_local! { static CONTEXT: RefCell<Option<Arc<Attempt>>> = const { RefCell::new(None) }; }
pub(crate) struct ContextGuard {
    previous: Option<Arc<Attempt>>,
    current: Option<Arc<Attempt>>,
    thread: PhantomData<Rc<()>>,
}
impl Drop for ContextGuard {
    fn drop(&mut self) {
        CONTEXT.with(|slot| {
            slot.replace(self.previous.take());
        });
        if let Some(current) = &self.current {
            if !current.attached.load(Ordering::Acquire) {
                finish(current, None, Outcome::NotSubmitted);
            }
        }
    }
}

/// A host-clock reading bracketed in the same Instant clock as GPUI input.
#[derive(Clone, Copy)]
pub struct ClockSample {
    seconds: f64,
    before: Instant,
    after: Instant,
}
impl ClockSample {
    /// Read the platform host clock exactly once. Use CACurrentMediaTime on Metal.
    pub fn capture(read_host_seconds: impl FnOnce() -> f64) -> Self {
        let before = Instant::now();
        let seconds = read_host_seconds();
        Self {
            seconds,
            before,
            after: Instant::now(),
        }
    }
}
/// Renderer callback ticket. It retains only an attempt and Weak bounded state;
/// dropping the last ticket without completion accounts a missing callback.
#[derive(Clone)]
pub struct NativeFrame(Arc<Ticket>);
struct Ticket {
    attempt: Arc<Attempt>,
    drawable: u64,
    clock: ClockSample,
    done: AtomicBool,
}
impl Drop for Ticket {
    fn drop(&mut self) {
        if !self.done.load(Ordering::Acquire) {
            finish(
                &self.attempt,
                Some((self.drawable, self.clock, 0., 0.)),
                Outcome::Missing,
            );
        }
    }
}
/// Attach at most one supported drawable to the current platform attempt.
/// Outside a collecting window context this is a no-op.
pub fn attach(drawable: u64, clock: ClockSample) -> Option<NativeFrame> {
    CONTEXT.with(|slot| {
        let attempt = slot.borrow().clone()?;
        if attempt.attached.swap(true, Ordering::AcqRel) {
            return None;
        }
        Some(NativeFrame(Arc::new(Ticket {
            attempt,
            drawable,
            clock,
            done: AtomicBool::new(false),
        })))
    })
}
impl NativeFrame {
    /// Report the native presentation timestamp and separately observed callback
    /// arrival. Safe on driver threads; never calls GPUI or application code.
    pub fn complete(&self, presented_host_s: f64, callback_host_s: f64) {
        if self.0.done.swap(true, Ordering::AcqRel) {
            if let Some(state) = self.0.attempt.state.upgrade() {
                lock(&state).snapshot.counts.duplicate_callbacks += 1;
            }
            return;
        }
        finish(
            &self.0.attempt,
            Some((
                self.0.drawable,
                self.0.clock,
                presented_host_s,
                callback_host_s,
            )),
            if presented_host_s == 0. {
                Outcome::Zero
            } else {
                Outcome::Presented
            },
        );
    }
}

fn bounds(seconds: f64, slack_s: f64) -> Option<LatencyBounds> {
    if !seconds.is_finite() || !slack_s.is_finite() || seconds < 0. || slack_s < 0. {
        return None;
    }
    let lower = ((seconds - slack_s).max(0.) * 1e9).floor();
    let upper = ((seconds + slack_s) * 1e9).ceil();
    if upper >= u64::MAX as f64 {
        return None;
    }
    Some(LatencyBounds {
        lower_ns: lower as u64,
        upper_ns: upper as u64,
    })
}
fn finish(attempt: &Attempt, native: Option<(u64, ClockSample, f64, f64)>, outcome: Outcome) {
    let Some(shared) = attempt.state.upgrade() else {
        return;
    };
    let mut state = lock(&shared);
    let Some(pending) = state.pending.get_mut(&attempt.sequence) else {
        return;
    };
    let m = pending.metadata;
    let mut record = Record {
        sequence: attempt.sequence,
        drawable: native.map(|n| n.0),
        new_scene: m.new_scene,
        active: m.active,
        animating: m.animating,
        inputs: m.inputs,
        outcome,
        submit_host_s: native.map(|n| n.1.seconds),
        presented_host_s: native.map(|n| n.2),
        callback_host_s: native.map(|n| n.3),
        submission_latency: None,
        input_latency: None,
    };
    if let Some((_, clock, presented, callback)) = native.filter(|_| outcome == Outcome::Presented)
    {
        let valid = [clock.seconds, presented, callback]
            .iter()
            .all(|n| n.is_finite())
            && clock.seconds > 0.
            && presented >= clock.seconds
            && callback >= presented
            && clock.after >= clock.before;
        if valid {
            // Two floating-point host timestamps contribute rounding uncertainty.
            let slack = (clock.seconds.abs() + presented.abs()) * f64::EPSILON;
            record.submission_latency = bounds(presented - clock.seconds, slack);
            if let Some(input) = m.input {
                if let Some(age) = clock.before.checked_duration_since(input) {
                    let width = clock.after.duration_since(clock.before).as_secs_f64();
                    record.input_latency = bounds(
                        age.as_secs_f64() + presented - clock.seconds + width / 2.,
                        slack + width / 2.,
                    );
                }
            }
        }
        if !valid
            || record.submission_latency.is_none()
            || (m.input.is_some() && record.input_latency.is_none())
        {
            record.outcome = Outcome::InvalidClock;
            record.submission_latency = None;
            record.input_latency = None;
        }
    }
    pending.result = Some(record);
    state.flush();
}
impl State {
    fn flush(&mut self) {
        while self
            .pending
            .first_key_value()
            .is_some_and(|(_, p)| p.result.is_some())
        {
            let (_, pending) = self.pending.pop_first().expect("checked first");
            let mut record = pending.result.expect("checked result");
            if record.outcome == Outcome::Presented {
                let time = record.presented_host_s.expect("validated time");
                if self.last_presented.is_some_and(|previous| time <= previous) {
                    record.outcome = Outcome::InvalidClock;
                    record.submission_latency = None;
                    record.input_latency = None;
                }
            }
            let counts = &mut self.snapshot.counts;
            match record.outcome {
                Outcome::Presented => {
                    counts.presented += 1;
                    let time = record.presented_host_s.expect("validated time");
                    for (value, histogram) in [
                        (
                            record.submission_latency,
                            &mut self.snapshot.submission_latency,
                        ),
                        (record.input_latency, &mut self.snapshot.input_latency),
                    ] {
                        if let Some(value) = value {
                            if value.upper_ns > 60_000_000_000
                                || histogram.record(value.upper_ns).is_err()
                            {
                                counts.histogram_overflow += 1;
                            }
                        }
                    }
                    if let Some((sequence, previous, animating)) = self.previous {
                        if sequence + 1 == record.sequence
                            && animating
                            && record.active
                            && record.new_scene
                        {
                            if let Some(interval) = bounds(
                                time - previous,
                                (time.abs() + previous.abs()) * f64::EPSILON,
                            ) {
                                if interval.upper_ns > 60_000_000_000
                                    || self
                                        .snapshot
                                        .animation_interval
                                        .record(interval.upper_ns)
                                        .is_err()
                                {
                                    counts.histogram_overflow += 1;
                                }
                            }
                        }
                    }
                    self.last_presented = Some(time);
                    self.previous = Some((
                        record.sequence,
                        time,
                        record.active && record.animating && record.new_scene,
                    ));
                }
                Outcome::NotSubmitted => {
                    counts.not_submitted += 1;
                    self.previous = None;
                }
                Outcome::Missing => {
                    counts.missing += 1;
                    self.previous = None;
                }
                Outcome::Zero => {
                    counts.zero += 1;
                    self.previous = None;
                }
                Outcome::InvalidClock => {
                    counts.invalid_clock += 1;
                    self.previous = None;
                }
            }
            if self.snapshot.trace.len() < self.limits.trace {
                self.snapshot.trace.push(record);
            } else {
                counts.trace_truncated += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn metadata() -> Metadata {
        Metadata {
            input: None,
            inputs: 0,
            new_scene: true,
            active: true,
            animating: true,
        }
    }
    fn clock(seconds: f64) -> ClockSample {
        let before = Instant::now();
        ClockSample {
            seconds,
            before,
            after: before,
        }
    }
    fn frame(controller: &Controller, seconds: f64) -> NativeFrame {
        let _context = controller.enter(metadata());
        attach(42, clock(seconds)).unwrap()
    }

    #[test]
    fn bounds_and_single_owner_include_stopped_session() {
        assert_eq!(Limits::new(0, 1).unwrap_err(), StartError::InvalidLimits);
        assert_eq!(Limits::new(129, 0).unwrap_err(), StartError::InvalidLimits);
        assert_eq!(Limits::new(1, 4097).unwrap_err(), StartError::InvalidLimits);
        let controller = Controller::new(WindowId::from(1));
        let session = controller.start(Limits::default()).unwrap();
        assert!(matches!(
            controller.start(Limits::default()),
            Err(StartError::AlreadyStarted)
        ));
        session.stop();
        assert!(matches!(
            controller.start(Limits::default()),
            Err(StartError::AlreadyStarted)
        ));
        drop(controller.enter(metadata()));
        assert_eq!(session.snapshot().counts.attempted, 0);
        let id = session.snapshot().session;
        drop(session);
        assert_ne!(
            controller
                .start(Limits::default())
                .unwrap()
                .snapshot()
                .session,
            id
        );
    }

    #[test]
    fn out_of_order_callbacks_settle_in_submission_order() {
        let controller = Controller::new(WindowId::from(1));
        let session = controller.start(Limits::default()).unwrap();
        let first = frame(&controller, 100.);
        let second = frame(&controller, 100.01);
        std::thread::spawn(move || second.complete(100.03, 100.04))
            .join()
            .unwrap();
        assert_eq!(session.snapshot().pending, 2);
        assert_eq!(session.snapshot().counts.presented, 0);
        first.complete(100.02, 100.05);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.pending, 0);
        assert_eq!(
            snapshot
                .trace
                .iter()
                .map(|r| r.sequence)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert_eq!(snapshot.counts.presented, 2);
        assert_eq!(snapshot.submission_latency.len(), 2);
        assert_eq!(snapshot.animation_interval.len(), 1);
    }

    #[test]
    fn saturation_trace_and_histograms_remain_bounded() {
        let controller = Controller::new(WindowId::from(1));
        let session = controller.start(Limits::new(1, 1).unwrap()).unwrap();
        let first = frame(&controller, 100.);
        {
            let _context = controller.enter(metadata());
            assert!(attach(1, clock(100.)).is_none());
        }
        first.complete(100.01, 100.02);
        frame(&controller, 101.).complete(101.01, 101.02);
        frame(&controller, 102.).complete(162.1, 163.);
        let s = session.snapshot();
        assert_eq!(
            (s.counts.attempted, s.counts.admitted, s.counts.saturated),
            (4, 3, 1)
        );
        assert_eq!((s.trace.len(), s.counts.trace_truncated), (1, 2));
        assert_eq!(s.submission_latency.len(), 2);
        assert_eq!(s.animation_interval.len(), 0); // gap, then >60 seconds
        assert_eq!(s.counts.histogram_overflow, 2);
        assert!(!s.submission_latency.is_auto_resize());
    }

    #[test]
    fn missing_zero_no_hook_and_duplicate_have_distinct_counts() {
        let controller = Controller::new(WindowId::from(1));
        let session = controller.start(Limits::default()).unwrap();
        drop(controller.enter(metadata()));
        drop(frame(&controller, 1.));
        let zero = frame(&controller, 1.);
        zero.complete(0., 1.1);
        zero.complete(1.01, 1.1);
        let s = session.snapshot();
        assert_eq!(
            (
                s.counts.not_submitted,
                s.counts.missing,
                s.counts.zero,
                s.counts.duplicate_callbacks
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(s.pending, 0);
        assert_eq!(s.submission_latency.len(), 0);
    }

    #[test]
    fn clock_conversion_brackets_corresponding_oldest_input() {
        let controller = Controller::new(WindowId::from(1));
        let session = controller.start(Limits::default()).unwrap();
        let input = Instant::now();
        let native = {
            let _context = controller.enter(Metadata {
                input: Some(input),
                inputs: 3,
                ..metadata()
            });
            attach(
                9,
                ClockSample {
                    seconds: 100.,
                    before: input + Duration::from_millis(10),
                    after: input + Duration::from_millis(12),
                },
            )
            .unwrap()
        };
        native.complete(100.020, 100.050);
        let snapshot = session.snapshot();
        let record = &snapshot.trace[0];
        assert_eq!(record.inputs, 3);
        let bounds = record.input_latency.unwrap();
        assert!((29_999_998..=30_000_000).contains(&bounds.lower_ns));
        assert!((32_000_000..=32_000_002).contains(&bounds.upper_ns));
        assert_eq!(snapshot.input_latency.len(), 1);
    }

    #[test]
    fn invalid_clocks_never_enter_latency_histograms() {
        let controller = Controller::new(WindowId::from(1));
        let session = controller.start(Limits::default()).unwrap();
        for (submit, present, callback) in [
            (1., f64::NAN, 2.),
            (1., 2., f64::INFINITY),
            (2., 1., 3.),
            (1., 3., 2.),
        ] {
            frame(&controller, submit).complete(present, callback);
        }
        frame(&controller, 10.).complete(11., 12.);
        drop(frame(&controller, 12.));
        frame(&controller, 9.).complete(10., 13.); // monotonicity survives a missing frame
        let s = session.snapshot();
        assert_eq!(
            (s.counts.invalid_clock, s.counts.presented, s.counts.missing),
            (5, 1, 1)
        );
        assert_eq!(s.submission_latency.len(), 1);
        assert_eq!(s.animation_interval.len(), 0);
    }

    #[test]
    fn nested_windows_and_unwind_restore_attribution() {
        let outer = Controller::new(WindowId::from(1));
        let inner = Controller::new(WindowId::from(2));
        let unmeasured = Controller::new(WindowId::from(3));
        let a = outer.start(Limits::default()).unwrap();
        let b = inner.start(Limits::default()).unwrap();
        {
            let _outer = outer.enter(metadata());
            let result = std::panic::catch_unwind(|| {
                let _inner = inner.enter(metadata());
                attach(20, clock(1.)).unwrap().complete(1.1, 1.2);
                panic!("exercise context unwind");
            });
            assert!(result.is_err());
            {
                let _guard = unmeasured.enter(metadata());
                assert!(attach(30, clock(1.)).is_none());
            }
            attach(10, clock(1.)).unwrap().complete(1.2, 1.3);
            assert!(attach(11, clock(1.)).is_none());
        }
        assert!(attach(0, clock(1.)).is_none());
        assert_eq!(a.snapshot().trace[0].drawable, Some(10));
        assert_eq!(b.snapshot().trace[0].drawable, Some(20));
        assert_eq!(a.snapshot().window, WindowId::from(1));
        assert_eq!(b.snapshot().window, WindowId::from(2));
    }

    #[test]
    fn late_callbacks_do_not_retain_windows_or_cross_sessions() {
        let controller = Controller::new(WindowId::from(1));
        let session = controller.start(Limits::default()).unwrap();
        let old = frame(&controller, 1.);
        let weak = Arc::downgrade(&session.state);
        drop(session);
        assert!(weak.upgrade().is_none());
        let replacement = controller.start(Limits::default()).unwrap();
        old.complete(1.1, 1.2);
        assert_eq!(replacement.snapshot().counts.presented, 0);
        let pending = frame(&controller, 2.);
        replacement.stop();
        drop(controller);
        pending.complete(2.1, 2.2);
        let s = replacement.snapshot();
        assert!(s.window_closed);
        assert!(!s.accepting);
        assert_eq!((s.counts.presented, s.pending), (1, 0));
    }
}
