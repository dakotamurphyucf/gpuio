//! Paint-committed height motion for a naturally sized disclosure panel.
//! Settled open content uses ordinary layout; only transitions consume measured
//! height. No timers, callbacks, layout objects or native owners are stored here.
use crate::motion::spring::Trajectory;
use gpuio_protocol::animation::{Property, Spring};
use std::{sync::Arc, time::Duration};

/// The shared animation geometry bound; larger content uses immediate layout.
pub const MAX_ANIMATED_HEIGHT: f64 = 1_000_000.;

/// Admission reservation for the native owner, trajectory and weak frame lease.
/// This is retained-resource accounting, not a claim about process RSS.
pub const RESERVED_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Presentation {
    /// Request ordinary natural layout, including new content and wrapping.
    Natural,
    /// No expanded layout. Retention policy is the caller's responsibility.
    Closed,
    /// Clip content to this physical height during a transition.
    Height(f64),
}

#[derive(Clone)]
pub struct Frame {
    pub presentation: Presentation,
    velocity: f64,
    complete: bool,
    forced: bool,
    at: Duration,
    epoch: Arc<()>,
}

struct Transition {
    deadline: Duration,
    target: Option<f64>,
    path: Option<(Trajectory, Duration)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSpring,
}

pub struct State {
    expanded: bool,
    spring: Spring,
    natural: Option<f64>,
    bounded: bool,
    painted: f64,
    velocity: f64,
    transition: Option<Transition>,
    last_paint: Duration,
    epoch: Arc<()>,
}

impl State {
    /// Initial placement is settled; mounting an expanded panel does not animate.
    pub fn new(expanded: bool, spring: Spring) -> Result<Self, Error> {
        if !spring.is_valid() {
            return Err(Error::InvalidSpring);
        }
        Ok(Self {
            expanded,
            spring,
            natural: None,
            bounded: true,
            painted: 0.,
            velocity: 0.,
            transition: None,
            last_paint: Duration::ZERO,
            epoch: Arc::new(()),
        })
    }

    /// Update semantic expansion. Retargeting preserves the last painted physical
    /// position and velocity. Measurement changes never extend this deadline.
    pub fn update(&mut self, expanded: bool, spring: Spring, now: Duration) -> Result<(), Error> {
        if !spring.is_valid() {
            return Err(Error::InvalidSpring);
        }
        let changed_expansion = self.expanded != expanded;
        let changed_spring = self.spring != spring;
        if !changed_expansion && !changed_spring {
            return Ok(());
        }
        self.epoch = Arc::new(());
        self.expanded = expanded;
        self.spring = spring;
        if !self.bounded {
            self.settle();
            return Ok(());
        }
        if !changed_expansion && self.transition.is_none() {
            return Ok(());
        }
        // A new semantic toggle owns a new deadline. Changing spring parameters
        // during the same transition cannot extend it.
        let deadline = if changed_expansion {
            now.saturating_add(Duration::from_millis(spring.max_duration_ms as u64))
        } else {
            self.transition.as_ref().unwrap().deadline
        };
        let target = if expanded { self.natural } else { Some(0.) };
        self.transition = Some(Transition {
            deadline,
            target: None,
            path: None,
        });
        if let Some(target) = target {
            self.retarget(target, now);
        }
        Ok(())
    }

    /// Hidden/inactive/removed owners settle without scheduling another frame.
    /// Reappearing content resumes ordinary layout, not a paused old transition.
    pub fn settle(&mut self) {
        self.epoch = Arc::new(());
        self.transition = None;
        self.velocity = 0.;
        if !self.expanded {
            self.painted = 0.;
        }
    }

    pub fn is_animating(&self) -> bool {
        self.transition.is_some()
    }

    /// A closed frame has no child layout to measure. Commit it only if it
    /// still belongs to this transition; retain the last natural measurement
    /// for a later opening. In particular, an old closed paint cannot settle
    /// a newly opened panel.
    pub fn painted_closed(&mut self, frame: &Frame) -> bool {
        if !Arc::ptr_eq(&frame.epoch, &self.epoch)
            || frame.at < self.last_paint
            || frame.presentation != Presentation::Closed
        {
            return self.is_animating();
        }
        self.last_paint = frame.at;
        self.transition = None;
        self.painted = 0.;
        self.velocity = 0.;
        false
    }

    fn settled(&self) -> Presentation {
        if self.expanded {
            Presentation::Natural
        } else {
            Presentation::Closed
        }
    }

    /// Sampling does not advance committed geometry; abandoned layout work must
    /// not move the next frame or seed a reversal from an unpainted position.
    pub fn frame(&self, now: Duration, immediate: bool) -> Frame {
        let mut frame = Frame {
            presentation: self.settled(),
            velocity: 0.,
            complete: true,
            forced: immediate,
            at: now,
            epoch: self.epoch.clone(),
        };
        if !immediate && let Some(transition) = &self.transition {
            if now >= transition.deadline {
                frame.forced = true;
            } else if let Some((path, start)) = &transition.path {
                let sample = path.sample(now.saturating_sub(*start));
                frame.presentation = Presentation::Height(sample.position);
                frame.velocity = sample.velocity;
                frame.complete = sample.finished;
            } else {
                // An initially closed panel has no measured height yet. The
                // renderer measures its retained content behind a zero clip.
                frame.presentation = Presentation::Height(self.painted);
                frame.velocity = self.velocity;
                frame.complete = false;
            }
        }
        frame
    }

    fn retarget(&mut self, target: f64, now: Duration) {
        let Ok(path) = Trajectory::new(
            self.spring,
            Property::Height,
            self.painted,
            self.velocity,
            target,
        ) else {
            // A valid layout may exceed animation's bounded geometry domain.
            // Keep natural layout rather than truncate content or panic.
            self.settle();
            return;
        };
        if let Some(transition) = &mut self.transition {
            transition.target = Some(target);
            transition.path = Some((path, now));
        }
    }

    /// Commit one painted sample and actual layout measurements. Returns whether
    /// this owner needs another frame. Nonfinite/out-of-domain measurements fall
    /// back to immediate layout; they never feed an analytic trajectory.
    /// A frame captured before a toggle/configuration/settle is ignored.
    pub fn painted(&mut self, frame: &Frame, natural: f64, presented: f64) -> bool {
        if !Arc::ptr_eq(&frame.epoch, &self.epoch) || frame.at < self.last_paint {
            return self.is_animating();
        }
        self.last_paint = frame.at;
        let valid = |value: f64| value.is_finite() && (0. ..=MAX_ANIMATED_HEIGHT).contains(&value);
        self.natural = valid(natural).then_some(natural);
        if !valid(natural) || !valid(presented) {
            self.bounded = false;
            // A clipped frame falling back to natural layout needs one final
            // correction frame even though the spring is already retired.
            let needs_layout = frame.presentation != self.settled();
            self.settle();
            return needs_layout;
        }
        self.bounded = true;
        self.painted = presented;
        self.velocity = frame.velocity;
        if frame.forced || self.transition.is_none() {
            self.transition = None;
            self.velocity = 0.;
            return false;
        }
        if !self.expanded {
            if frame.complete {
                self.transition = None;
                self.painted = 0.;
                self.velocity = 0.;
            }
            // Height(0) still owns a clipping layout (and its margins). Paint
            // the Closed layout once to remove it, then stop requesting frames.
            return self.is_animating() || frame.presentation != self.settled();
        }
        let target = self
            .transition
            .as_ref()
            .and_then(|transition| transition.target);
        if target != Some(natural) {
            self.retarget(natural, frame.at);
        } else if frame.complete && (presented - natural).abs() <= self.spring.epsilon {
            self.transition = None;
            self.velocity = 0.;
        }
        // The final clipped sample must be followed by ordinary natural layout.
        // Otherwise subsequent content growth can remain clipped until an
        // unrelated application update requests a frame.
        self.is_animating() || frame.presentation != self.settled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spring() -> Spring {
        Spring {
            stiffness: 400.,
            damping: 40.,
            mass: 1.,
            epsilon: 0.1,
            max_duration_ms: 2000,
        }
    }
    fn at(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }
    fn paint(state: &mut State, now: u64, natural: f64) -> (Presentation, bool) {
        let frame = state.frame(at(now), false);
        let presented = match frame.presentation {
            Presentation::Natural => natural,
            Presentation::Closed => 0.,
            Presentation::Height(h) => h,
        };
        let needs_frame = state.painted(&frame, natural, presented);
        (frame.presentation, needs_frame)
    }
    #[test]
    fn settled_open_streaming_and_wrapping_never_use_cached_height_or_request_frames() {
        let mut state = State::new(true, spring()).unwrap();
        for (tick, height) in [20., 100., 450., 72., 190.].into_iter().enumerate() {
            assert_eq!(
                paint(&mut state, tick as u64 * 16, height),
                (Presentation::Natural, false)
            );
        }
    }
    #[test]
    fn unmeasured_opening_starts_at_zero_then_reveals_without_an_initial_full_height_frame() {
        let mut state = State::new(false, spring()).unwrap();
        state.update(true, spring(), at(0)).unwrap();
        assert_eq!(paint(&mut state, 0, 120.), (Presentation::Height(0.), true));
        let (Presentation::Height(height), true) = paint(&mut state, 40, 120.) else {
            panic!("expected animation")
        };
        assert!(height > 0. && height < 120.);
        assert_eq!(
            paint(&mut state, 2000, 120.),
            (Presentation::Natural, false)
        );
    }
    #[test]
    fn reversal_and_resize_retarget_from_painted_position_without_layout_only_jumps() {
        let mut state = State::new(true, spring()).unwrap();
        paint(&mut state, 0, 180.);
        state.update(false, spring(), at(0)).unwrap();
        let (Presentation::Height(height), true) = paint(&mut state, 40, 180.) else {
            panic!("expected closing")
        };
        let abandoned = state.frame(at(100), false);
        state.update(true, spring(), at(40)).unwrap();
        assert_eq!(
            state.frame(at(40), false).presentation,
            Presentation::Height(height)
        );
        state.painted(&abandoned, 400., 1.);
        assert_eq!(
            state.frame(at(40), false).presentation,
            Presentation::Height(height)
        );
        let frame = state.frame(at(40), false);
        assert!(state.painted(&frame, 300., height));
        assert_eq!(
            state.frame(at(40), false).presentation,
            Presentation::Height(height)
        );
        assert_eq!(
            paint(&mut state, 2040, 300.),
            (Presentation::Natural, false)
        );
    }
    #[test]
    fn ongoing_growth_and_spring_changes_cannot_extend_the_transition_deadline() {
        let mut state = State::new(false, spring()).unwrap();
        state.update(true, spring(), at(0)).unwrap();
        for tick in 0..200 {
            if tick == 100 {
                state
                    .update(
                        true,
                        Spring {
                            damping: 45.,
                            ..spring()
                        },
                        at(tick * 10),
                    )
                    .unwrap();
            }
            paint(&mut state, tick * 10, 100. + tick as f64);
        }
        assert_eq!(
            paint(&mut state, 2000, 500.),
            (Presentation::Natural, false)
        );
        assert_eq!(
            paint(&mut state, 2016, 700.),
            (Presentation::Natural, false)
        );
    }
    #[test]
    fn an_older_paint_cannot_rewind_geometry_within_the_same_transition() {
        let mut state = State::new(true, spring()).unwrap();
        paint(&mut state, 0, 180.);
        state.update(false, spring(), at(0)).unwrap();
        let old = state.frame(at(10), false);
        let (Presentation::Height(height), true) = paint(&mut state, 40, 180.) else {
            panic!("expected closing")
        };
        assert!(state.painted(&old, 999., 1.));
        state.update(true, spring(), at(40)).unwrap();
        assert_eq!(
            state.frame(at(40), false).presentation,
            Presentation::Height(height)
        );
        assert_eq!(
            paint(&mut state, 2040, 180.),
            (Presentation::Natural, false)
        );
    }
    #[test]
    fn reduced_motion_and_hidden_owners_settle_and_discard_old_frames() {
        let mut state = State::new(true, spring()).unwrap();
        paint(&mut state, 0, 180.);
        state.update(false, spring(), at(0)).unwrap();
        let old = state.frame(at(10), false);
        let reduced = state.frame(at(10), true);
        assert_eq!(reduced.presentation, Presentation::Closed);
        assert!(!state.painted(&reduced, 180., 0.));
        state.settle();
        assert!(!state.painted(&old, 180., 170.));
        assert_eq!(
            state.frame(at(20), false).presentation,
            Presentation::Closed
        );
        state.update(true, spring(), at(20)).unwrap();
        state.settle();
        assert_eq!(paint(&mut state, 30, 240.), (Presentation::Natural, false));
    }
    #[test]
    fn completion_requests_exactly_one_final_unclipped_layout() {
        let mut state = State::new(false, spring()).unwrap();
        for expanded in [true, false] {
            let start = if expanded { 0 } else { 2000 };
            state.update(expanded, spring(), at(start)).unwrap();
            let mut finished = None;
            for tick in 0..125 {
                let now = start + tick * 16;
                let (presentation, needs_frame) = paint(&mut state, now, 120.);
                if !state.is_animating() {
                    assert!(matches!(presentation, Presentation::Height(_)));
                    assert!(needs_frame, "retiring the spring must repaint the layout");
                    finished = Some(now);
                    break;
                }
            }
            let now = finished.expect("spring settles before its deadline") + 16;
            let frame = state.frame(at(now), false);
            if expanded {
                assert_eq!(frame.presentation, Presentation::Natural);
                assert!(!state.painted(&frame, 150., 150.));
            } else {
                assert_eq!(frame.presentation, Presentation::Closed);
                assert!(!state.painted_closed(&frame));
            }
        }
    }

    #[test]
    fn closed_paint_preserves_measurement_and_cannot_cancel_a_later_opening() {
        let mut state = State::new(true, spring()).unwrap();
        paint(&mut state, 0, 120.);
        state.update(false, spring(), at(0)).unwrap();
        let closed = state.frame(at(2000), false);
        assert!(!state.painted_closed(&closed));
        state.update(true, spring(), at(2016)).unwrap();
        assert!(state.painted_closed(&closed));
        let (Presentation::Height(height), true) = paint(&mut state, 2056, 120.) else {
            panic!("reopening uses the retained natural height")
        };
        assert!(height > 0. && height < 120.);
        state.settle();
        assert!(!state.painted_closed(&closed));
        assert_eq!(
            state.frame(at(2072), false).presentation,
            Presentation::Natural
        );
    }

    #[test]
    fn oversized_content_keeps_natural_layout_and_invalid_springs_do_not_change_state() {
        let mut state = State::new(false, spring()).unwrap();
        state.update(true, spring(), at(0)).unwrap();
        let frame = state.frame(at(0), false);
        assert!(state.painted(&frame, MAX_ANIMATED_HEIGHT + 1., 0.));
        assert_eq!(
            state.frame(at(10), false).presentation,
            Presentation::Natural
        );
        let natural = state.frame(at(10), false);
        assert!(!state.painted(&natural, MAX_ANIMATED_HEIGHT + 1., MAX_ANIMATED_HEIGHT + 1.));
        state.update(false, spring(), at(20)).unwrap();
        assert_eq!(
            state.frame(at(20), false).presentation,
            Presentation::Closed
        );
        state.update(true, spring(), at(30)).unwrap();
        assert_eq!(
            state.frame(at(30), false).presentation,
            Presentation::Natural
        );
        assert_eq!(
            state.update(
                false,
                Spring {
                    stiffness: f64::NAN,
                    ..spring()
                },
                at(10)
            ),
            Err(Error::InvalidSpring)
        );
        assert_eq!(
            state.frame(at(10), false).presentation,
            Presentation::Natural
        );
    }
}
