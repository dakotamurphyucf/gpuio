//! Measured track scrolling. Ownership is assigned by the first useful delta and
//! remains stable through the burst, including at finite endpoints.
use crate::{carousel_gesture::Step, carousel_track_geometry::Geometry};
use std::{rc::Rc, time::Duration};
pub(super) const QUIET: Duration = Duration::from_millis(28);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Started,
    Moved,
    Ended,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Intent {
    Select(usize),
    Step(Step),
}
#[derive(Default, Debug)]
pub(super) struct Output {
    pub consumed: bool,
    pub preview: Option<f32>,
    pub finished: bool,
    pub intents: Vec<Intent>,
}
pub(super) struct Input {
    pub delta: f64,
    pub precise: bool,
    pub phase: Phase,
    pub now: Duration,
}
struct Precise {
    geometry: Geometry,
    start: usize,
    offset: f32,
    total: f64,
}
enum Mode {
    Precise(Precise),
    Fence(bool),
}
#[derive(Default)]
pub(super) struct State {
    mode: Option<Mode>,
    deadline: Option<Duration>,
    epoch: Rc<()>,
}
impl State {
    pub fn active(&self) -> bool {
        matches!(self.mode, Some(Mode::Precise(_) | Mode::Fence(true)))
    }
    pub fn geometry(&self) -> Option<&Geometry> {
        match &self.mode {
            Some(Mode::Precise(p)) => Some(&p.geometry),
            _ => None,
        }
    }
    pub fn deadline(&self) -> Option<Duration> {
        self.deadline
    }
    pub fn epoch(&self) -> Rc<()> {
        self.epoch.clone()
    }
    pub fn matches(&self, epoch: &Rc<()>) -> bool {
        Rc::ptr_eq(&self.epoch, epoch)
    }
    fn end(&mut self, commit: bool) -> Output {
        let mode = self.mode.take();
        self.deadline = None;
        self.epoch = Rc::new(());
        let mut output = Output::default();
        match mode {
            Some(Mode::Precise(p)) => {
                output.consumed = true;
                output.finished = true;
                if commit
                    && let Some(index) = p
                        .geometry
                        .release_target(p.start, p.total, p.offset)
                        .filter(|index| *index != p.start)
                {
                    output.intents.push(Intent::Select(index));
                }
            }
            Some(Mode::Fence(consumed)) => output.consumed = consumed,
            None => (),
        }
        output
    }
    pub fn expire(&mut self, now: Duration) -> Output {
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.end(true)
        } else {
            Output::default()
        }
    }
    /// Retire unfinished work but preserve who owns its remainder until quiet.
    pub fn interrupt(&mut self, now: Duration) -> bool {
        let preview = matches!(self.mode, Some(Mode::Precise(_)));
        if self.mode.is_some() {
            let consumed = self.active();
            self.mode = Some(Mode::Fence(consumed));
            self.deadline = now.checked_add(QUIET);
            self.epoch = Rc::new(());
        }
        preview
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    /// Expire any elapsed burst before push. Delta is the already filtered primary
    /// axis in logical pixels. Zero terminal deltas still finish a precise gesture.
    pub fn push(
        &mut self,
        geometry: &Geometry,
        current: usize,
        painted: f32,
        input: Input,
    ) -> Output {
        let Input {
            delta,
            precise,
            phase,
            now,
        } = input;
        if !delta.is_finite() || !painted.is_finite() || geometry.snap(current).is_none() {
            return Output::default();
        }
        if precise && matches!(phase, Phase::Ended | Phase::Cancelled) {
            return self.end(phase == Phase::Ended);
        }
        let mut output = if precise && phase == Phase::Started {
            self.end(true)
        } else {
            Output::default()
        };
        if delta == 0. {
            return output;
        }
        self.deadline = now.checked_add(QUIET);
        if let Some(Mode::Fence(consumed)) = self.mode {
            output.consumed = consumed;
            return output;
        }
        if !precise {
            // Switching devices completes any pixel preview before assigning a line burst.
            if matches!(self.mode, Some(Mode::Precise(_))) {
                output = self.end(true);
                self.deadline = now.checked_add(QUIET);
            }
            let step = Step::from_delta(delta as f32);
            let consumed = geometry.step(current, step).is_some();
            self.mode = Some(Mode::Fence(consumed));
            output.consumed = consumed;
            if consumed {
                output.intents.push(Intent::Step(step));
            }
            return output;
        }
        if self.mode.is_none() {
            let next = geometry.bounded_offset(f64::from(painted) + delta);
            let consumed = next.is_some_and(|next| next != painted) || geometry.looping().is_some();
            if !consumed {
                self.mode = Some(Mode::Fence(false));
                return output;
            }
            self.mode = Some(Mode::Precise(Precise {
                geometry: geometry.clone(),
                start: current,
                offset: painted,
                total: 0.,
            }));
        }
        let Some(Mode::Precise(p)) = &mut self.mode else {
            return output;
        };
        p.total = (p.total + delta).clamp(-f64::from(f32::MAX), f64::from(f32::MAX));
        if let Some(offset) = p.geometry.bounded_offset(f64::from(p.offset) + delta) {
            p.offset = offset;
            output.preview = Some(offset);
        }
        output.consumed = true;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carousel_track_geometry::Item;
    fn geometry() -> Geometry {
        Geometry::new(
            100.,
            140.,
            vec![
                Item {
                    start: 0.,
                    extent: 40.,
                },
                Item {
                    start: 48.,
                    extent: 120.,
                },
                Item {
                    start: 176.,
                    extent: 64.,
                },
            ],
            false,
        )
        .unwrap()
    }
    fn push(
        state: &mut State,
        current: usize,
        delta: f64,
        precise: bool,
        phase: Phase,
        ms: u64,
    ) -> Output {
        let g = geometry();
        state.push(
            &g,
            current,
            g.snap(current).unwrap(),
            Input {
                delta,
                precise,
                phase,
                now: Duration::from_millis(ms),
            },
        )
    }
    #[test]
    fn precise_pixels_finish_once_and_cancel_returns_without_selection() {
        let mut state = State::default();
        let out = push(&mut state, 0, -30., true, Phase::Started, 0);
        assert_eq!(out.preview, Some(-30.));
        assert!(out.consumed);
        assert!(out.intents.is_empty());
        assert_eq!(
            push(&mut state, 0, -100., true, Phase::Moved, 10).preview,
            Some(-130.)
        );
        let out = push(&mut state, 0, 0., true, Phase::Ended, 11);
        assert!(out.finished);
        assert_eq!(out.intents, vec![Intent::Select(2)]);
        assert!(!state.active());
        assert_eq!(state.deadline(), None);
        assert!(
            push(&mut state, 0, 0., true, Phase::Ended, 12)
                .intents
                .is_empty()
        );
        push(&mut state, 0, -130., true, Phase::Started, 20);
        let out = push(&mut state, 0, 0., true, Phase::Cancelled, 21);
        assert!(out.finished);
        assert!(out.intents.is_empty());
        assert_eq!(state.deadline(), None);
    }
    #[test]
    fn quiet_deadline_extends_and_old_epoch_cannot_complete_a_new_gesture() {
        let mut state = State::default();
        push(&mut state, 0, -30., true, Phase::Started, 0);
        let first = state.epoch();
        push(&mut state, 0, -20., true, Phase::Moved, 20);
        assert!(state.matches(&first));
        assert!(!state.expire(Duration::from_millis(28)).finished);
        assert_eq!(state.deadline(), Some(Duration::from_millis(48)));
        let out = state.expire(Duration::from_millis(48));
        assert!(out.finished);
        assert_eq!(out.intents, vec![Intent::Select(1)]);
        assert!(!state.matches(&first));
        assert!(state.expire(Duration::from_secs(1)).intents.is_empty());
        push(&mut state, 0, -30., true, Phase::Started, 1000);
        let next = state.epoch();
        let out = push(&mut state, 0, -10., true, Phase::Started, 1001);
        assert!(out.finished);
        assert_eq!(out.preview, Some(-10.));
        assert!(!state.matches(&next));
    }
    #[test]
    fn ownership_stays_with_first_delta_and_line_bursts_step_once() {
        let mut state = State::default();
        assert!(!push(&mut state, 0, 20., true, Phase::Started, 0).consumed);
        assert!(!push(&mut state, 0, -60., true, Phase::Moved, 1).consumed);
        push(&mut state, 0, 0., true, Phase::Ended, 2);
        assert!(push(&mut state, 0, -300., true, Phase::Started, 3).consumed);
        assert!(push(&mut state, 0, -10., true, Phase::Moved, 4).consumed);
        assert_eq!(
            push(&mut state, 0, 20., true, Phase::Moved, 5).preview,
            Some(-120.)
        );
        push(&mut state, 0, 0., true, Phase::Cancelled, 6);
        assert_eq!(
            push(&mut state, 0, -1., false, Phase::Moved, 7).intents,
            vec![Intent::Step(Step::Next)]
        );
        assert!(
            push(&mut state, 1, -1., false, Phase::Moved, 8)
                .intents
                .is_empty()
        );
        state.expire(Duration::from_millis(36));
        assert!(!push(&mut state, 2, -1., false, Phase::Moved, 37).consumed);
        assert!(!push(&mut state, 2, 1., false, Phase::Moved, 38).consumed);
    }
    #[test]
    fn interruption_fences_momentum_and_invalid_input_cannot_poison_offsets() {
        let mut state = State::default();
        push(&mut state, 0, -30., true, Phase::Started, 0);
        assert!(state.interrupt(Duration::from_millis(1)));
        let out = push(&mut state, 1, -100., true, Phase::Moved, 2);
        assert!(out.consumed);
        assert!(out.preview.is_none());
        assert!(out.intents.is_empty());
        assert!(!state.expire(Duration::from_millis(30)).finished);
        for delta in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(!push(&mut state, 0, delta, true, Phase::Started, 31).consumed);
        }
        assert_eq!(state.deadline(), None);
        assert_eq!(
            push(&mut state, 0, -f64::MAX, true, Phase::Started, 32).preview,
            Some(-140.)
        );
        state.clear();
        assert!(!state.active());
        assert_eq!(state.deadline(), None);
    }
}
