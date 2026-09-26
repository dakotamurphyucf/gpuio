//! Bounded, platform-independent carousel gesture decisions. Geometry is in
//! logical pixels; only the final step crosses into application-owned selection.
use gpuio_protocol::carousel::Axis;
use std::time::Duration;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Previous,
    Next,
}
impl Step {
    pub fn from_delta(delta: f32) -> Self {
        if delta > 0. {
            Self::Previous
        } else {
            Self::Next
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Progress {
    Pending,
    Rejected,
    Dragging { offset: f32, neighbor: Option<Step> },
}
#[derive(Debug)]
pub struct Drag {
    axis: Axis,
    origin: [f32; 2],
    extent: f32,
    previous: bool,
    next: bool,
    progress: Progress,
    delta: f32,
}
fn components(axis: Axis, point: [f32; 2]) -> (f32, f32) {
    match axis {
        Axis::Horizontal => (point[0], point[1]),
        Axis::Vertical => (point[1], point[0]),
    }
}
impl Drag {
    pub fn new(
        axis: Axis,
        origin: [f32; 2],
        extent: f32,
        previous: bool,
        next: bool,
    ) -> Option<Self> {
        (origin.iter().all(|v| v.is_finite())
            && extent.is_finite()
            && extent > 0.
            && (previous || next))
            .then_some(Self {
                axis,
                origin,
                extent,
                previous,
                next,
                progress: Progress::Pending,
                delta: 0.,
            })
    }
    pub fn progress(&self) -> Progress {
        self.progress
    }
    pub fn update(&mut self, point: [f32; 2]) -> Progress {
        if !point.iter().all(|v| v.is_finite()) {
            self.progress = Progress::Rejected;
        }
        if self.progress == Progress::Rejected {
            return self.progress;
        }
        let (main, cross) = components(
            self.axis,
            [point[0] - self.origin[0], point[1] - self.origin[1]],
        );
        if !main.is_finite() || !cross.is_finite() {
            self.progress = Progress::Rejected;
            return self.progress;
        }
        if self.progress == Progress::Pending {
            if cross.abs() >= 8. && cross.abs() > main.abs() * 1.25 {
                self.progress = Progress::Rejected;
                return self.progress;
            }
            if main.abs() < 8. || main.abs() < cross.abs() * 1.25 {
                return self.progress;
            }
        }
        self.delta = main;
        let step = Step::from_delta(main);
        let available = match step {
            Step::Previous => self.previous,
            Step::Next => self.next,
        };
        let offset = (main / self.extent).clamp(-1., 1.);
        self.progress = Progress::Dragging {
            offset: if available {
                offset
            } else {
                offset / (1. + offset.abs() * 7.)
            },
            neighbor: available.then_some(step),
        };
        self.progress
    }
    pub fn finish(&self) -> Option<Step> {
        let Progress::Dragging { neighbor, .. } = self.progress else {
            return None;
        };
        (self.delta.abs() >= (self.extent * 0.2).clamp(12., 80.))
            .then_some(neighbor)
            .flatten()
    }
}

pub const WHEEL_QUIET: Duration = Duration::from_millis(150);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Started,
    Moved,
    Ended,
    Cancelled,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WheelResult {
    pub consumed: bool,
    pub request: Option<Step>,
}
struct Burst {
    delta: f32,
    deadline: Duration,
}
#[derive(Default)]
pub struct Wheel {
    burst: Option<Burst>,
    fence: Option<Duration>,
}
impl Wheel {
    pub fn deadline(&self) -> Option<Duration> {
        self.burst.as_ref().map(|b| b.deadline)
    }
    pub fn active(&self) -> bool {
        self.burst.is_some()
    }
    pub fn cancel(&mut self) {
        self.burst = None;
        self.fence = None;
    }
    /// Invalidate unfinished work on a model/policy change while swallowing the
    /// remainder of its momentum. Resetting the fence would navigate the new page
    /// again when the application's accepted selection arrives mid-swipe.
    pub fn interrupt(&mut self, now: Duration) {
        if self.burst.take().is_some() || self.fence.is_some() {
            self.fence = now.checked_add(WHEEL_QUIET);
        }
    }
    fn complete(&mut self, now: Duration, commit: bool) -> Option<Step> {
        let burst = self.burst.take()?;
        self.fence = now.checked_add(WHEEL_QUIET);
        (commit && burst.delta.abs() >= 32.).then(|| Step::from_delta(burst.delta))
    }
    /// The host calls this before push, and from its one cancellable deadline
    /// task. A missing platform Ended event cannot retain an unfinished burst.
    pub fn finish(&mut self, now: Duration) -> Option<Step> {
        if self.deadline().is_some_and(|deadline| now >= deadline) {
            self.complete(now, true)
        } else {
            None
        }
    }
    pub fn push(
        &mut self,
        axis: Axis,
        delta: [f32; 2],
        precise: bool,
        phase: Phase,
        now: Duration,
    ) -> WheelResult {
        if !delta.iter().all(|v| v.is_finite()) {
            self.cancel();
            return WheelResult::default();
        }
        if phase == Phase::Started {
            self.cancel();
        }
        if phase == Phase::Cancelled {
            let consumed = self.active();
            self.complete(now, false);
            return WheelResult {
                consumed,
                request: None,
            };
        }
        if phase == Phase::Ended {
            let consumed = self.active() || self.fence.is_some_and(|until| now < until);
            return WheelResult {
                consumed,
                request: self.complete(now, true),
            };
        }
        let (main, cross) = components(axis, delta);
        if main == 0. || main.abs() < cross.abs() * 1.25 {
            return WheelResult::default();
        }
        let Some(deadline) = now.checked_add(WHEEL_QUIET) else {
            self.cancel();
            return WheelResult::default();
        };
        if self.fence.is_some_and(|until| now < until) {
            self.fence = Some(deadline);
            return WheelResult {
                consumed: true,
                request: None,
            };
        }
        self.fence = None;
        if !precise {
            self.burst = None;
            self.fence = Some(deadline);
            return WheelResult {
                consumed: true,
                request: Some(Step::from_delta(main)),
            };
        }
        let burst = self.burst.get_or_insert(Burst {
            delta: 0.,
            deadline,
        });
        // One burst yields at most one step; magnitudes beyond this do not carry
        // additional meaning and must not accumulate nonfinite state.
        burst.delta = (burst.delta + main.clamp(-4096., 4096.)).clamp(-4096., 4096.);
        burst.deadline = deadline;
        WheelResult {
            consumed: true,
            request: None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    #[test]
    fn axis_lock_threshold_reversal_and_edge_resistance() {
        let mut drag = Drag::new(Axis::Horizontal, [0., 0.], 300., true, true).unwrap();
        assert_eq!(drag.update([5., 2.]), Progress::Pending);
        assert_eq!(
            drag.update([-90., 4.]),
            Progress::Dragging {
                offset: -0.3,
                neighbor: Some(Step::Next)
            }
        );
        assert_eq!(drag.finish(), Some(Step::Next));
        assert_eq!(
            drag.update([15., 1000.]),
            Progress::Dragging {
                offset: 0.05,
                neighbor: Some(Step::Previous)
            }
        );
        assert_eq!(drag.finish(), None);
        drag.update([80., 1000.]);
        assert_eq!(drag.finish(), Some(Step::Previous));
        let mut cross = Drag::new(Axis::Vertical, [0., 0.], 300., true, true).unwrap();
        assert_eq!(cross.update([50., 5.]), Progress::Rejected);
        assert_eq!(cross.update([0., 200.]), Progress::Rejected);
        let mut edge = Drag::new(Axis::Horizontal, [0., 0.], 300., false, true).unwrap();
        let Progress::Dragging { offset, neighbor } = edge.update([1000., 0.]) else {
            panic!()
        };
        assert_eq!(neighbor, None);
        assert_eq!(offset, 0.125);
        assert_eq!(edge.finish(), None);
        assert!(Drag::new(Axis::Horizontal, [0., 0.], 300., false, false).is_none());
        assert!(Drag::new(Axis::Horizontal, [0., 0.], f32::NAN, true, true).is_none());
    }
    #[test]
    fn bounded_invalid_and_vertical_drag_workload() {
        let mut drag = Drag::new(Axis::Vertical, [50., 50.], 400., true, true).unwrap();
        for step in 0..10_000 {
            let point = [50., 50. + ((step % 1000) as f32 - 500.)];
            let Progress::Dragging { offset, .. } = drag.update(point) else {
                panic!()
            };
            assert!(offset.is_finite() && (-1.0..=1.).contains(&offset));
        }
        assert_eq!(drag.update([f32::INFINITY, 0.]), Progress::Rejected);
        assert_eq!(drag.finish(), None);
    }
    #[test]
    fn wheel_end_cancel_timeout_and_momentum_fence() {
        let mut wheel = Wheel::default();
        assert!(
            !wheel
                .push(Axis::Horizontal, [1., 20.], true, Phase::Started, ms(0))
                .consumed
        );
        assert!(
            wheel
                .push(Axis::Horizontal, [-20., 0.], true, Phase::Started, ms(1))
                .consumed
        );
        wheel.push(Axis::Horizontal, [-20., 0.], true, Phase::Moved, ms(10));
        assert_eq!(wheel.finish(ms(159)), None);
        assert_eq!(
            wheel
                .push(Axis::Horizontal, [0., 0.], true, Phase::Ended, ms(20))
                .request,
            Some(Step::Next)
        );
        for now in 21..1000 {
            assert_eq!(
                wheel
                    .push(Axis::Horizontal, [-40., 0.], true, Phase::Moved, ms(now))
                    .request,
                None
            );
        }
        assert!(!wheel.active());
        wheel.push(Axis::Horizontal, [40., 0.], true, Phase::Started, ms(1001));
        assert_eq!(
            wheel
                .push(Axis::Horizontal, [0., 0.], true, Phase::Cancelled, ms(1002))
                .request,
            None
        );
        assert_eq!(wheel.finish(ms(2000)), None);
        wheel.push(Axis::Vertical, [0., 40.], true, Phase::Started, ms(2001));
        assert_eq!(wheel.finish(ms(2150)), None);
        assert_eq!(wheel.finish(ms(2151)), Some(Step::Previous));
        assert_eq!(wheel.finish(ms(9999)), None);
    }
    #[test]
    fn accepted_selection_interrupts_work_but_preserves_momentum_fence() {
        let mut wheel = Wheel::default();
        wheel.push(Axis::Horizontal, [-40., 0.], true, Phase::Started, ms(0));
        wheel.interrupt(ms(20));
        assert!(!wheel.active());
        assert_eq!(wheel.finish(ms(200)), None);
        assert!(
            wheel
                .push(Axis::Horizontal, [-80., 0.], true, Phase::Moved, ms(30))
                .consumed
        );
        assert!(!wheel.active());
        assert_eq!(
            wheel
                .push(Axis::Horizontal, [0., 0.], true, Phase::Ended, ms(40))
                .request,
            None
        );
        wheel.push(Axis::Horizontal, [40., 0.], true, Phase::Started, ms(50));
        assert_eq!(
            wheel
                .push(Axis::Horizontal, [0., 0.], true, Phase::Ended, ms(60))
                .request,
            Some(Step::Previous)
        );
    }
    #[test]
    fn ordinary_wheel_bursts_and_nonfinite_samples_do_not_retain_work() {
        let mut wheel = Wheel::default();
        assert_eq!(
            wheel
                .push(Axis::Vertical, [0., -1.], false, Phase::Moved, ms(0))
                .request,
            Some(Step::Next)
        );
        assert_eq!(
            wheel
                .push(Axis::Vertical, [0., -1.], false, Phase::Moved, ms(5))
                .request,
            None
        );
        assert_eq!(
            wheel
                .push(Axis::Vertical, [0., -1.], false, Phase::Moved, ms(200))
                .request,
            Some(Step::Next)
        );
        assert!(wheel.deadline().is_none());
        wheel.push(Axis::Vertical, [0., 100.], true, Phase::Started, ms(300));
        wheel.push(Axis::Vertical, [0., f32::NAN], true, Phase::Moved, ms(301));
        assert!(wheel.deadline().is_none());
        wheel.push(
            Axis::Vertical,
            [0., 100.],
            true,
            Phase::Moved,
            Duration::MAX,
        );
        assert!(!wheel.active());
    }
}
