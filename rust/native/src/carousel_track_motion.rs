//! Painted-offset motion for one retained track. No item resources or timers.
use crate::carousel_track_geometry::{Geometry, Loop};
use gpuio_protocol::{
    carousel::{Axis, Direction},
    carousel_track::{Config, Motion},
};
use std::{rc::Rc, time::Duration};

#[derive(Clone, PartialEq)]
struct Context {
    geometry: Geometry,
    ids: Vec<String>,
    selected: usize,
    axis: Axis,
    motion: Option<Motion>,
}
struct Transition {
    from: f64,
    to: f64,
    start: Duration,
}
#[derive(Clone)]
pub(super) struct Sample {
    pub offset: f32,
    pub needs_frame: bool,
    at: Duration,
    epoch: Rc<()>,
    context: Rc<Context>,
}
#[derive(Default)]
pub(super) struct State {
    context: Option<Rc<Context>>,
    transition: Option<Transition>,
    preview: Option<f32>,
    retarget: bool,
    painted: Option<Sample>,
    epoch: Rc<()>,
}
fn runway(geometry: &Geometry) -> f64 {
    match geometry.looping() {
        Some(Loop::Continuous { origin, .. }) => origin.into(),
        _ => 0.,
    }
}
fn normalize(geometry: &Geometry, offset: f64, fallback: f32) -> f32 {
    geometry.bounded_offset(offset).unwrap_or(fallback)
}
impl State {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn painted_offset(&self) -> Option<f32> {
        self.painted
            .as_ref()
            .filter(|sample| Rc::ptr_eq(&sample.epoch, &self.epoch))
            .map(|sample| sample.offset)
    }
    pub fn previewing(&self) -> bool {
        self.preview.is_some()
    }
    pub fn preview(&mut self, offset: f64) -> bool {
        let Some(offset) = self
            .context
            .as_ref()
            .and_then(|context| context.geometry.bounded_offset(offset))
        else {
            return false;
        };
        self.epoch = Rc::new(());
        self.transition = None;
        self.preview = Some(offset);
        self.retarget = false;
        true
    }
    /// Releasing forces a fresh target from accepted paint on the next frame.
    /// Until the controlled model changes it returns to the accepted selection.
    pub fn finish_preview(&mut self) {
        if self.preview.take().is_some() {
            self.retarget = true;
            self.epoch = Rc::new(());
        }
    }

    /// Called after geometry publication. Hidden/unavailable owners clear history;
    /// initial appearance never travels from an invented coordinate.
    pub fn sample(
        &mut self,
        config: &Config,
        geometry: Option<&Geometry>,
        motion: Option<Motion>,
        eligible: bool,
        now: Duration,
    ) -> Option<Sample> {
        let Some((geometry, selected)) = geometry.zip(config.carousel.selected) else {
            self.clear();
            return None;
        };
        let selected = selected as usize;
        let target = geometry.snap(selected)?;
        let model = &config.carousel;
        let changed = self.retarget
            || self.context.as_ref().is_none_or(|old| {
                old.geometry != *geometry
                    || old.ids != model.ids
                    || old.selected != selected
                    || old.axis != model.axis
                    || old.motion != motion
            });
        if changed {
            self.retarget = false;
            self.preview = None;
            let next = Rc::new(Context {
                geometry: geometry.clone(),
                ids: model.ids.clone(),
                selected,
                axis: model.axis,
                motion,
            });
            let from = self.painted.as_ref().and_then(|painted| {
                let old = &painted.context;
                if old.axis != next.axis {
                    return None;
                }
                let anchor = &old.ids[old.selected];
                let index = next.ids.iter().position(|id| id == anchor)?;
                let old_item = old.geometry.items()[old.selected];
                let visible = f64::from(old_item.start)
                    + f64::from(painted.offset)
                    + runway(&old.geometry)
                    + f64::from(
                        old.geometry
                            .item_translation(old.selected, painted.offset)?,
                    );
                let offset = visible
                    - f64::from(next.geometry.items()[index].start)
                    - runway(&next.geometry);
                Some((f64::from(normalize(geometry, offset, target)), index))
            });
            self.epoch = Rc::new(());
            self.transition = from.and_then(|(mut from, previous)| {
                if !eligible || motion.is_none() {
                    return None;
                }
                let selection_changed = self
                    .context
                    .as_ref()
                    .is_some_and(|old| old.ids[old.selected] != next.ids[selected]);
                let wrap = selection_changed
                    && match model.direction {
                        Direction::Next => selected < previous,
                        Direction::Previous => selected > previous,
                        Direction::Direct => false,
                    };
                if wrap && geometry.looping() == Some(Loop::Jump) {
                    return None;
                }
                let mut to = f64::from(target);
                match geometry.looping() {
                    Some(Loop::Continuous { cycle, .. }) => {
                        let cycle = f64::from(cycle);
                        match if selection_changed {
                            model.direction
                        } else {
                            Direction::Direct
                        } {
                            Direction::Next if to > from => to -= cycle,
                            Direction::Previous if to < from => to += cycle,
                            Direction::Direct => to += ((from - to) / cycle).round() * cycle,
                            _ => (),
                        }
                    }
                    _ => {
                        // Keep resized finite tracks inside their new measured endpoints.
                        let min = geometry.snaps().iter().copied().fold(0., f32::min);
                        from = from.clamp(f64::from(min), 0.);
                    }
                }
                (from != to).then_some(Transition {
                    from,
                    to,
                    start: now,
                })
            });
            self.context = Some(next);
        }
        if !eligible && self.transition.take().is_some() {
            self.epoch = Rc::new(());
        }
        let mut offset = target;
        let mut needs_frame = false;
        if let Some(transition) = &self.transition {
            let motion = motion.expect("transition has motion");
            let elapsed = now.saturating_sub(transition.start).as_secs_f64() * 1000.;
            let progress = (elapsed / motion.duration_ms as f64).clamp(0., 1.);
            needs_frame = progress < 1.;
            if needs_frame {
                let eased = motion.easing.sample(progress);
                let eased = if eased.is_finite() {
                    eased.clamp(0., 1.)
                } else {
                    progress
                };
                offset = normalize(
                    geometry,
                    transition.from + (transition.to - transition.from) * eased,
                    target,
                );
            }
        }
        if let Some(preview) = self.preview {
            offset = preview;
            needs_frame = false;
        }
        Some(Sample {
            offset,
            needs_frame,
            at: now,
            epoch: self.epoch.clone(),
            context: self.context.as_ref().unwrap().clone(),
        })
    }
    /// Only an accepted paint becomes history; obsolete preparation cannot
    /// resurrect movement or overwrite a newer visible position.
    pub fn painted(&mut self, sample: &Sample) -> bool {
        if !Rc::ptr_eq(&self.epoch, &sample.epoch)
            || self
                .painted
                .as_ref()
                .is_some_and(|last| sample.at < last.at)
        {
            return false;
        }
        self.painted = Some(sample.clone());
        if !sample.needs_frame {
            self.transition = None;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carousel_track_geometry::Item;
    use gpuio_protocol::{animation::Easing, carousel};
    fn config(selected: i64, looping: bool) -> Config {
        Config {
            lineage: 0,
            carousel: carousel::Config {
                revision: 0,
                ids: vec!["a".into(), "b".into(), "c".into()],
                selected: Some(selected),
                looping,
                disabled: false,
                axis: Axis::Horizontal,
                auto_advance_ms: None,
                direction: Direction::Direct,
            },
        }
    }
    fn geometry(viewport: f32, looping: bool) -> Geometry {
        Geometry::new(
            viewport,
            (300. - viewport).max(0.),
            (0..3)
                .map(|i| Item {
                    start: i as f32 * 100.,
                    extent: 100.,
                })
                .collect(),
            looping,
        )
        .unwrap()
    }
    fn motion() -> Option<Motion> {
        Some(Motion {
            duration_ms: 200,
            easing: Easing::Linear,
        })
    }
    fn sample(state: &mut State, config: &Config, geometry: &Geometry, time: u64) -> Sample {
        state
            .sample(
                config,
                Some(geometry),
                motion(),
                true,
                Duration::from_millis(time),
            )
            .unwrap()
    }
    #[test]
    fn retargets_from_paint_not_speculation_and_rejects_obsolete_samples() {
        let g = geometry(100., false);
        let mut state = State::default();
        let first = sample(&mut state, &config(0, false), &g, 0);
        assert!(!first.needs_frame);
        assert!(state.painted(&first));
        let start = sample(&mut state, &config(2, false), &g, 10);
        assert_eq!(start.offset, 0.);
        state.painted(&start);
        let half = sample(&mut state, &config(2, false), &g, 110);
        assert_eq!(half.offset, -100.);
        state.painted(&half);
        let speculative = sample(&mut state, &config(2, false), &g, 160);
        assert_eq!(speculative.offset, -150.);
        let redirect = sample(&mut state, &config(0, false), &g, 160);
        assert_eq!(redirect.offset, -100.);
        assert!(!state.painted(&speculative));
        let done = sample(&mut state, &config(0, false), &g, 360);
        assert_eq!(done.offset, 0.);
        assert!(!done.needs_frame);
        assert!(state.painted(&done));
        assert!(!state.painted(&redirect));
        assert!(!sample(&mut state, &config(0, false), &g, 400).needs_frame);
    }
    #[test]
    fn resize_preserves_visible_anchor_and_removed_anchor_or_axis_settles() {
        let g = geometry(100., true);
        let mut state = State::default();
        let mut c = config(1, true);
        let initial = sample(&mut state, &c, &g, 0);
        state.painted(&initial);
        // Item a grows by 20; b's visible position must not jump before travel.
        let resized = Geometry::new(
            100.,
            220.,
            vec![
                Item {
                    start: 0.,
                    extent: 120.,
                },
                Item {
                    start: 120.,
                    extent: 100.,
                },
                Item {
                    start: 220.,
                    extent: 100.,
                },
            ],
            true,
        )
        .unwrap();
        let start = sample(&mut state, &c, &resized, 10);
        let visible = |g: &Geometry, s: &Sample, i: usize| {
            f64::from(g.items()[i].start + s.offset + g.item_translation(i, s.offset).unwrap())
                + runway(g)
        };
        assert_eq!(visible(&g, &initial, 1), visible(&resized, &start, 1));
        state.painted(&start);
        c.carousel.ids[1] = "replacement".into();
        let replaced = sample(&mut state, &c, &resized, 20);
        assert!(!replaced.needs_frame);
        state.painted(&replaced);
        c.carousel.axis = Axis::Vertical;
        assert!(!sample(&mut state, &c, &resized, 30).needs_frame);
    }
    #[test]
    fn continuous_boundary_rebases_every_frame_without_duplicate_owners() {
        let g = geometry(100., true);
        let mut state = State::default();
        let mut c = config(2, true);
        let initial = sample(&mut state, &c, &g, 0);
        state.painted(&initial);
        // Repeated boundary moves never accumulate large offsets.
        for step in 0..150 {
            c.carousel.selected = Some((step % 3) as i64);
            c.carousel.direction = Direction::Next;
            let now = step * 300 + 1;
            let start = sample(&mut state, &c, &g, now);
            state.painted(&start);
            for tick in 0..=20 {
                let s = sample(&mut state, &c, &g, now + tick * 10);
                assert!(s.offset >= -600. && s.offset <= -300.);
                // The viewport remains fully covered with exactly one owner per card.
                for x in [0.5, 25., 50., 75., 99.5] {
                    let covered = (0..3)
                        .filter(|&i| {
                            let left = g.items()[i].start
                                + s.offset
                                + 300.
                                + g.item_translation(i, s.offset).unwrap();
                            x >= left && x < left + 100.
                        })
                        .count();
                    assert_eq!(
                        covered, 1,
                        "step={step} tick={tick} x={x} offset={}",
                        s.offset
                    );
                }
                assert!(state.painted(&s));
            }
        }
        c.carousel.selected = Some(2);
        c.carousel.direction = Direction::Previous;
        sample(&mut state, &c, &g, 50_000);
        let half = sample(&mut state, &c, &g, 50_100);
        assert!(half.offset.is_finite());
    }
    #[test]
    fn jump_wrap_reduce_unavailable_and_clear_do_not_replay_motion() {
        let g = geometry(250., true);
        assert_eq!(g.looping(), Some(Loop::Jump));
        let mut state = State::default();
        let mut c = config(2, true);
        let initial = sample(&mut state, &c, &g, 0);
        state.painted(&initial);
        c.carousel.selected = Some(0);
        c.carousel.direction = Direction::Next;
        assert!(!sample(&mut state, &c, &g, 10).needs_frame);
        let g = geometry(100., false);
        state.clear();
        c = config(0, false);
        let s = sample(&mut state, &c, &g, 20);
        state.painted(&s);
        c.carousel.selected = Some(2);
        let stale = sample(&mut state, &c, &g, 30);
        assert!(stale.needs_frame);
        let reduced = state
            .sample(&c, Some(&g), motion(), false, Duration::from_millis(40))
            .unwrap();
        assert!(!reduced.needs_frame);
        assert_eq!(reduced.offset, -200.);
        assert!(!state.painted(&stale));
        state.painted(&reduced);
        assert!(!sample(&mut state, &c, &g, 50).needs_frame);
        assert!(
            state
                .sample(&c, None, motion(), true, Duration::from_millis(60))
                .is_none()
        );
        assert!(!state.painted(&reduced));
        c.carousel.selected = Some(0);
        assert!(!sample(&mut state, &c, &g, 70).needs_frame);
        state.clear();
        assert!(!state.painted(&stale));
    }
    #[test]
    fn extreme_easing_is_finite_and_motion_policy_changes_do_not_change_selection() {
        let g = geometry(100., false);
        let mut state = State::default();
        let c = config(0, false);
        let s = sample(&mut state, &c, &g, 0);
        state.painted(&s);
        let c = config(2, false);
        let settings = Some(Motion {
            duration_ms: 200,
            easing: Easing::CubicBezier(0., f64::MAX, 1., -f64::MAX),
        });
        state.sample(&c, Some(&g), settings, true, Duration::ZERO);
        for time in 0..=200 {
            let s = state
                .sample(&c, Some(&g), settings, true, Duration::from_millis(time))
                .unwrap();
            assert!(s.offset.is_finite() && (-200. ..=0.).contains(&s.offset));
            state.painted(&s);
        }
        let s = state
            .sample(&c, Some(&g), None, true, Duration::from_millis(210))
            .unwrap();
        assert!(!s.needs_frame);
        assert_eq!(s.offset, -200.);
    }

    #[test]
    fn preview_moves_without_frames_and_returns_to_controlled_selection_from_paint() {
        let g = geometry(100., false);
        let c = config(0, false);
        let mut state = State::default();
        let initial = sample(&mut state, &c, &g, 0);
        state.painted(&initial);
        assert!(state.preview(-35.));
        assert!(!state.painted(&initial));
        let preview = sample(&mut state, &c, &g, 10);
        assert_eq!(preview.offset, -35.);
        assert!(!preview.needs_frame);
        assert!(state.previewing());
        state.painted(&preview);
        state.finish_preview();
        assert!(!state.previewing());
        let return_start = sample(&mut state, &c, &g, 20);
        assert_eq!(return_start.offset, -35.);
        assert!(return_start.needs_frame);
        state.painted(&return_start);
        let done = sample(&mut state, &c, &g, 220);
        assert_eq!(done.offset, 0.);
        assert!(!done.needs_frame);
        state.painted(&done);
        assert!(!state.preview(f64::NAN));
        assert!(!state.previewing());
        assert!(state.preview(-75.));
        let preview = sample(&mut state, &c, &g, 230);
        state.painted(&preview);
        let updated = sample(&mut state, &config(1, false), &g, 240);
        assert_eq!(updated.offset, -75.);
        assert!(!state.previewing());
        state.clear();
        assert!(!state.preview(0.));
        assert!(state.painted_offset().is_none());
    }
}
