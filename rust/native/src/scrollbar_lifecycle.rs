//! Paint-committed, finite scrollbar timing. No timer, task, handle or callback.
use crate::scrollbar_presentation::Interaction;
use gpuio_protocol::scrollbar::{Entrance, Mode, Motion, dimension};
use std::{rc::Rc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Visual {
    pub opacity: f64,
    /// Fraction of the edge envelope translated outward; zero is fully inside.
    pub slide: f64,
    pub track_width: f64,
    pub thumb_width: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidMotion,
    InvalidWidth,
}
#[derive(Clone, Copy)]
enum Ease {
    Linear,
    In,
    Out,
}
#[derive(Clone, Copy)]
struct Channel {
    from: f64,
    to: f64,
    start: Duration,
    duration: Duration,
    ease: Ease,
}
impl Channel {
    fn settled(value: f64) -> Self {
        Self {
            from: value,
            to: value,
            start: Duration::ZERO,
            duration: Duration::ZERO,
            ease: Ease::Linear,
        }
    }
    fn sample(self, now: Duration) -> (f64, bool) {
        let elapsed = now.saturating_sub(self.start);
        if self.from == self.to || elapsed >= self.duration {
            return (self.to, false);
        }
        let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        let progress = match self.ease {
            Ease::Linear => progress,
            Ease::In => progress.powi(3),
            Ease::Out => 1. - (1. - progress).powi(3),
        };
        (self.from + (self.to - self.from) * progress, true)
    }
    fn retarget(&mut self, from: f64, to: f64, duration: Duration, now: Duration, ease: Ease) {
        if duration.is_zero() {
            *self = Self::settled(to);
        } else if self.to != to {
            *self = Self {
                from,
                to,
                start: now,
                duration,
                ease,
            };
        }
    }
}
#[derive(Clone)]
struct History {
    visual: Visual,
    channels: [Channel; 4],
    entrance: Entrance,
    at: Duration,
}
#[derive(Clone)]
pub struct Frame {
    next: History,
    needs_frame: bool,
    epoch: Rc<()>,
}
impl Frame {
    pub fn at(&self) -> Duration {
        self.next.at
    }
    pub fn visual(&self) -> Visual {
        self.next.visual
    }
    pub fn needs_frame(&self) -> bool {
        self.needs_frame
    }
}
#[derive(Clone)]
pub struct Deadline {
    at: Duration,
    epoch: Rc<()>,
}
impl Deadline {
    pub fn at(&self) -> Duration {
        self.at
    }
    pub fn same_schedule(&self, other: &Self) -> bool {
        self.at == other.at && Rc::ptr_eq(&self.epoch, &other.epoch)
    }
}

/// One axis of one native node generation. Dropping this model releases only
/// bounded scalar state. The adapter separately cancels its weak wake/timer.
pub struct State {
    mode: Mode,
    motion: Motion,
    eligible: bool,
    reduced: bool,
    closed: bool,
    interaction: Interaction,
    focused: bool,
    activity: Option<Duration>,
    last: Duration,
    history: Option<History>,
    paint_epoch: Rc<()>,
    activity_epoch: Rc<()>,
}
impl State {
    pub fn new(mode: Mode, motion: Motion) -> Result<Self, Error> {
        if !motion.is_valid() {
            return Err(Error::InvalidMotion);
        }
        Ok(Self {
            mode,
            motion,
            eligible: false,
            reduced: false,
            closed: false,
            interaction: Interaction::Rest,
            focused: false,
            activity: None,
            last: Duration::ZERO,
            history: None,
            paint_epoch: Rc::new(()),
            activity_epoch: Rc::new(()),
        })
    }
    fn invalidate(&mut self) {
        self.paint_epoch = Rc::new(());
        self.activity_epoch = Rc::new(());
    }
    fn time(&mut self, now: Duration) -> Duration {
        self.last = self.last.max(now);
        self.last
    }
    pub fn set_policy(&mut self, mode: Mode, motion: Motion) -> Result<bool, Error> {
        if !motion.is_valid() {
            return Err(Error::InvalidMotion);
        }
        if self.closed || (self.mode == mode && self.motion == motion) {
            return Ok(false);
        }
        self.mode = mode;
        self.motion = motion;
        self.invalidate();
        Ok(true)
    }
    pub fn set_reduced(&mut self, reduced: bool) -> bool {
        if self.closed || self.reduced == reduced {
            return false;
        }
        self.reduced = reduced;
        self.invalidate();
        true
    }
    /// Caller includes measured overflow, visibility, inert/disabled/modal and
    /// active-window status. Losing eligibility immediately retires interaction.
    pub fn set_eligible(&mut self, eligible: bool) -> bool {
        if self.closed || self.eligible == eligible {
            return false;
        }
        self.eligible = eligible;
        if !eligible {
            self.interaction = Interaction::Rest;
            self.focused = false;
            self.activity = None;
            self.history = None;
        }
        self.invalidate();
        true
    }
    pub fn is_eligible(&self) -> bool {
        self.eligible && !self.closed
    }
    pub fn interaction(&self) -> Interaction {
        self.interaction
    }
    pub fn accepts_pointer(&self) -> bool {
        self.eligible
            && !self.closed
            && self.history.as_ref().is_some_and(|h| h.visual.opacity > 0.)
    }
    /// Called for actual offset changes or completion of native scrolling input.
    /// Hidden/inactive owners do not accumulate a reveal to replay later.
    pub fn activity(&mut self, now: Duration) -> bool {
        if !self.eligible || self.closed {
            return false;
        }
        self.activity = Some(self.time(now));
        self.invalidate();
        true
    }
    /// Hover may reveal a hidden Hover-mode bar; hidden Scrolling-mode bars
    /// cannot be hovered/pressed. Explicit range focus is independent of opacity.
    pub fn set_interaction(
        &mut self,
        mut interaction: Interaction,
        focused: bool,
        now: Duration,
    ) -> bool {
        if !self.eligible || self.closed {
            return false;
        }
        if interaction == Interaction::Pressed && !self.accepts_pointer() {
            return false;
        }
        if !self.accepts_pointer() && self.mode != Mode::Always {
            match interaction {
                Interaction::Pressed => return false,
                Interaction::TrackHover | Interaction::ThumbHover
                    if self.mode == Mode::Scrolling =>
                {
                    interaction = Interaction::Rest;
                }
                Interaction::Rest | Interaction::TrackHover | Interaction::ThumbHover => (),
            }
        }
        if self.interaction == interaction && self.focused == focused {
            return false;
        }
        let now = self.time(now);
        let released = (self.interaction != Interaction::Rest && interaction == Interaction::Rest)
            || (self.interaction == Interaction::Pressed && interaction != Interaction::Pressed)
            || (self.focused && !focused);
        if released {
            self.activity = Some(now);
        }
        self.interaction = interaction;
        self.focused = focused;
        self.invalidate();
        true
    }
    fn held(&self) -> bool {
        self.focused
            || self.interaction == Interaction::Pressed
            || (self.interaction != Interaction::Rest
                && (self.mode == Mode::Hover || self.accepts_pointer()))
    }
    fn activity_until(&self) -> Option<Duration> {
        self.activity
            .map(|at| at.saturating_add(Duration::from_millis(self.motion.idle_ms as u64)))
    }
    fn wants_visible(&self, now: Duration) -> bool {
        self.eligible
            && !self.closed
            && (self.mode == Mode::Always
                || self.held()
                || self.activity_until().is_some_and(|at| now < at))
    }
    /// Finite idle wake. Animation frames use Frame::needs_frame separately.
    /// Polling this query does not mutate or renew the activity identity.
    pub fn deadline(&self, now: Duration) -> Option<Deadline> {
        if self.closed || !self.eligible || self.mode == Mode::Always || self.held() {
            return None;
        }
        let at = self.activity_until()?;
        (at > now.max(self.last)).then(|| Deadline {
            at,
            epoch: self.activity_epoch.clone(),
        })
    }
    pub fn wake(&mut self, deadline: &Deadline, now: Duration) -> bool {
        if self.closed
            || !self.eligible
            || self.held()
            || self.mode == Mode::Always
            || !Rc::ptr_eq(&deadline.epoch, &self.activity_epoch)
            || self.activity_until() != Some(deadline.at)
            || now < deadline.at
        {
            return false;
        }
        self.time(now);
        self.activity = None;
        self.invalidate();
        true
    }
    /// Immutable candidate: no animation begins until painted accepts it.
    /// Widths are resolved by scrollbar_presentation for the current state.
    pub fn sample(
        &self,
        now: Duration,
        track_width: f64,
        thumb_width: f64,
    ) -> Result<Frame, Error> {
        if !dimension(track_width) || !dimension(thumb_width) {
            return Err(Error::InvalidWidth);
        }
        let now = now.max(self.last);
        let visible = self.wants_visible(now);
        let available = self.eligible && !self.closed;
        let entrance = if self.mode == Mode::Hover && self.interaction == Interaction::ThumbHover {
            self.motion.thumb_hover_entrance
        } else {
            self.motion.entrance
        };
        let mut next = self.history.clone().unwrap_or_else(|| {
            let slide = if entrance == Entrance::SlideAndFade {
                1.
            } else {
                0.
            };
            History {
                visual: Visual {
                    opacity: 0.,
                    slide,
                    track_width,
                    thumb_width,
                },
                channels: [
                    Channel::settled(0.),
                    Channel::settled(slide),
                    Channel::settled(track_width),
                    Channel::settled(thumb_width),
                ],
                entrance,
                at: now,
            }
        });
        // Select entrance only for a fresh reveal. Changing hover at full
        // visibility must not reintroduce positional travel or restart entry.
        if visible && next.visual.opacity == 0. && next.channels[0].to != 1. {
            next.entrance = entrance;
            let slide = if entrance == Entrance::SlideAndFade {
                1.
            } else {
                0.
            };
            next.visual.slide = slide;
            next.channels[1] = Channel::settled(slide);
        }
        let target_opacity = if visible { 1. } else { 0. };
        let target_slide = if !visible && next.entrance == Entrance::SlideAndFade {
            1.
        } else {
            0.
        };
        let distance = (target_opacity - next.visual.opacity)
            .abs()
            .max((target_slide - next.visual.slide).abs());
        let immediate = self.reduced || !available || self.mode == Mode::Always;
        let duration = if immediate {
            Duration::ZERO
        } else {
            Duration::from_millis(if visible {
                self.motion.enter_ms
            } else {
                self.motion.exit_ms
            } as u64)
            .mul_f64(distance)
        };
        next.channels[0].retarget(
            next.visual.opacity,
            target_opacity,
            duration,
            now,
            if visible { Ease::Linear } else { Ease::In },
        );
        next.channels[1].retarget(
            next.visual.slide,
            target_slide,
            duration,
            now,
            if visible { Ease::Out } else { Ease::In },
        );
        let expand = if self.reduced || !available || (!visible && next.visual.opacity == 0.) {
            Duration::ZERO
        } else {
            Duration::from_millis(self.motion.expand_ms as u64)
        };
        next.channels[2].retarget(next.visual.track_width, track_width, expand, now, Ease::Out);
        next.channels[3].retarget(next.visual.thumb_width, thumb_width, expand, now, Ease::Out);
        let samples = next.channels.map(|channel| channel.sample(now));
        next.visual = Visual {
            opacity: samples[0].0,
            slide: samples[1].0,
            track_width: samples[2].0,
            thumb_width: samples[3].0,
        };
        next.at = now;
        Ok(Frame {
            next,
            needs_frame: available && samples.into_iter().any(|(_, running)| running),
            epoch: self.paint_epoch.clone(),
        })
    }
    pub fn painted(&mut self, frame: &Frame) -> bool {
        if self.closed
            || !self.eligible
            || !Rc::ptr_eq(&self.paint_epoch, &frame.epoch)
            || frame.next.at < self.last
        {
            return false;
        }
        self.last = frame.next.at;
        self.history = Some(frame.next.clone());
        self.paint_epoch = Rc::new(());
        true
    }
    pub fn close(&mut self) -> bool {
        if self.closed {
            return false;
        }
        self.set_eligible(false);
        self.closed = true;
        self.invalidate();
        true
    }
}

#[cfg(test)]
#[path = "scrollbar_lifecycle_test.rs"]
mod test;
