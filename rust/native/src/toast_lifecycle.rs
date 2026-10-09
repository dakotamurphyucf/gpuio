//! Finite native toast phases. Owns no timer, transport, entity or child content.
use gpuio_protocol::{animation::Easing, v1::ToastDismissal};
use std::{rc::Rc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub enter: Duration,
    pub exit: Duration,
    pub offset: f64,
}
impl Default for Motion {
    fn default() -> Self {
        Self {
            enter: Duration::from_millis(400),
            exit: Duration::from_millis(200),
            offset: 96.,
        }
    }
}
impl Motion {
    pub fn is_valid(self) -> bool {
        self.enter <= Duration::from_secs(60)
            && self.exit <= Duration::from_secs(60)
            && self.offset.is_finite()
            && (0.0..=16384.).contains(&self.offset)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Visual {
    pub opacity: f64,
    pub slide: f64,
}
const PRESENT: Visual = Visual {
    opacity: 1.,
    slide: 0.,
};
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Pending,
    Entering {
        start: Duration,
        duration: Duration,
        from: Visual,
    },
    Present,
    Ending {
        start: Duration,
        duration: Duration,
        from: Visual,
        target: f64,
        reason: ToastDismissal,
    },
    Closed,
}
#[derive(Clone)]
pub struct Frame {
    visual: Visual,
    needs_frame: bool,
    at: Duration,
    next: Phase,
    terminal: Option<ToastDismissal>,
    phase_epoch: Rc<()>,
    paint_epoch: Rc<()>,
}
impl Frame {
    pub fn visual(&self) -> Visual {
        self.visual
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
    pub fn same_phase(&self, other: &Self) -> bool {
        self.at == other.at && Rc::ptr_eq(&self.epoch, &other.epoch)
    }
    pub fn at(&self) -> Duration {
        self.at
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidMotion,
}

pub struct State {
    phase: Phase,
    painted: Option<(Duration, Visual)>,
    terminal: Option<ToastDismissal>,
    phase_epoch: Rc<()>,
    paint_epoch: Rc<()>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            phase: Phase::Pending,
            painted: None,
            terminal: None,
            phase_epoch: Rc::new(()),
            paint_epoch: Rc::new(()),
        }
    }
}
fn blend(from: Visual, to: Visual, progress: f64) -> Visual {
    let progress = Easing::Ease.sample(progress);
    Visual {
        opacity: from.opacity + (to.opacity - from.opacity) * progress,
        slide: from.slide + (to.slide - from.slide) * progress,
    }
}
impl State {
    pub fn accepts_input(&self) -> bool {
        !matches!(self.phase, Phase::Ending { .. } | Phase::Closed)
    }
    /// The host separately applies hidden/modal/focus/hover and timeout policy.
    pub fn allows_timeout(&self) -> bool {
        self.phase == Phase::Present
    }
    pub fn is_closed(&self) -> bool {
        self.phase == Phase::Closed
    }
    fn phase(&mut self, phase: Phase) {
        if self.phase != phase {
            self.phase = phase;
            self.phase_epoch = Rc::new(());
        }
    }
    pub fn deadline(&self) -> Option<Deadline> {
        let (start, duration) = match self.phase {
            Phase::Entering {
                start, duration, ..
            }
            | Phase::Ending {
                start, duration, ..
            } => (start, duration),
            Phase::Pending | Phase::Present | Phase::Closed => return None,
        };
        Some(Deadline {
            at: start.saturating_add(duration),
            epoch: self.phase_epoch.clone(),
        })
    }
    /// Immutable layout preview. Only an accepted paint starts a new entry.
    /// [motion=None] also implements reduced/inactive/immediate presentation.
    pub fn sample(
        &self,
        now: Duration,
        motion: Option<Motion>,
        bottom: bool,
    ) -> Result<Frame, Error> {
        if motion.is_some_and(|m| !m.is_valid()) {
            return Err(Error::InvalidMotion);
        }
        let phase = if self.phase == Phase::Pending {
            match motion.filter(|m| !m.enter.is_zero()) {
                Some(m) => Phase::Entering {
                    start: now,
                    duration: m.enter,
                    from: Visual {
                        opacity: 0.,
                        slide: if bottom { m.offset } else { -m.offset },
                    },
                },
                None => Phase::Present,
            }
        } else {
            self.phase
        };
        let (next, visual, terminal) = match phase {
            Phase::Pending => unreachable!(),
            Phase::Present => (phase, PRESENT, None),
            Phase::Closed => (
                phase,
                Visual {
                    opacity: 0.,
                    slide: 0.,
                },
                None,
            ),
            Phase::Entering {
                start,
                duration,
                from,
            } => {
                let elapsed = now.saturating_sub(start);
                if motion.is_none() || elapsed >= duration {
                    (Phase::Present, PRESENT, None)
                } else {
                    (
                        phase,
                        blend(
                            from,
                            PRESENT,
                            elapsed.as_secs_f64() / duration.as_secs_f64(),
                        ),
                        None,
                    )
                }
            }
            Phase::Ending {
                start,
                duration,
                from,
                target,
                reason,
            } => {
                let elapsed = now.saturating_sub(start);
                let end = Visual {
                    opacity: 0.,
                    slide: target,
                };
                if motion.is_none() || elapsed >= duration {
                    (Phase::Closed, end, Some(reason))
                } else {
                    (
                        phase,
                        blend(from, end, elapsed.as_secs_f64() / duration.as_secs_f64()),
                        None,
                    )
                }
            }
        };
        Ok(Frame {
            visual,
            needs_frame: matches!(next, Phase::Entering { .. } | Phase::Ending { .. }),
            at: now,
            next,
            terminal,
            phase_epoch: self.phase_epoch.clone(),
            paint_epoch: self.paint_epoch.clone(),
        })
    }
    pub fn painted(&mut self, frame: &Frame) -> bool {
        if !Rc::ptr_eq(&frame.phase_epoch, &self.phase_epoch)
            || !Rc::ptr_eq(&frame.paint_epoch, &self.paint_epoch)
            || self.painted.is_some_and(|(at, _)| frame.at < at)
        {
            return false;
        }
        self.phase(frame.next);
        self.painted = Some((frame.at, frame.visual));
        self.paint_epoch = Rc::new(());
        if let Some(reason) = frame.terminal {
            self.terminal = Some(reason);
        }
        true
    }
    /// Accept one native dismissal; interrupted entry exits from the last paint.
    /// A never-painted item has nothing to animate and becomes terminal immediately.
    pub fn dismiss(
        &mut self,
        reason: ToastDismissal,
        now: Duration,
        motion: Option<Motion>,
        bottom: bool,
    ) -> Result<bool, Error> {
        if motion.is_some_and(|m| !m.is_valid()) {
            return Err(Error::InvalidMotion);
        }
        if !self.accepts_input() {
            return Ok(false);
        }
        match (motion.filter(|m| !m.exit.is_zero()), self.painted) {
            (Some(m), Some((at, from))) if from.opacity > 0. => self.phase(Phase::Ending {
                start: now.max(at),
                duration: m.exit,
                from,
                target: if bottom { m.offset } else { -m.offset },
                reason,
            }),
            _ => {
                self.phase(Phase::Closed);
                self.terminal = Some(reason);
            }
        }
        Ok(true)
    }
    /// Timer identity survives intermediate paint samples but not a phase change.
    pub fn finish_deadline(&mut self, deadline: &Deadline, now: Duration) -> bool {
        if !Rc::ptr_eq(&deadline.epoch, &self.phase_epoch) || now < deadline.at {
            return false;
        }
        match self.phase {
            Phase::Entering { .. } => self.phase(Phase::Present),
            Phase::Ending { reason, .. } => {
                self.phase(Phase::Closed);
                self.terminal = Some(reason);
            }
            Phase::Pending | Phase::Present | Phase::Closed => return false,
        }
        true
    }
    /// Hidden/inactive/unpainted policy settles phases without scheduling frames.
    /// Pending remains pending until its first eligible paint.
    pub fn suspend(&mut self) {
        match self.phase {
            Phase::Entering { .. } => self.phase(Phase::Present),
            Phase::Ending { reason, .. } => {
                self.phase(Phase::Closed);
                self.terminal = Some(reason);
            }
            Phase::Pending | Phase::Present | Phase::Closed => {}
        }
        self.paint_epoch = Rc::new(());
    }
    pub fn take_dismissal(&mut self) -> Option<ToastDismissal> {
        self.terminal.take()
    }
    /// Unmount/window close/overload: no retained content and no terminal callback.
    pub fn discard(&mut self) {
        self.phase(Phase::Closed);
        self.terminal = None;
        self.painted = None;
        self.paint_epoch = Rc::new(());
    }
}
