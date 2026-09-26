//! Bounded navigation presentation. Owns IDs and geometry, never page resources.
//! The host supplies ordered retained page IDs after each admitted transaction.
use gpuio_protocol::{
    NodeId,
    navigation_stack::{Config, Motion},
};
use std::{collections::BTreeSet, sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub page: NodeId,
    /// Offset as a fraction of the presenter's assigned extent on its motion axis.
    pub offset: f32,
    pub opacity: f32,
}
impl Layer {
    fn settled(page: NodeId) -> Self {
        Self {
            page,
            offset: 0.,
            opacity: 1.,
        }
    }
    fn interpolate(self, target: Self, progress: f32) -> Self {
        Self {
            page: self.page,
            offset: self.offset + (target.offset - self.offset) * progress,
            opacity: self.opacity + (target.opacity - self.opacity) * progress,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sample {
    pub current: Option<Layer>,
    pub outgoing: Option<Layer>,
    pub needs_frame: bool,
    at: Duration,
    epoch: Arc<()>,
}
impl Sample {
    fn layer(&self, page: NodeId) -> Option<Layer> {
        self.current
            .into_iter()
            .chain(self.outgoing)
            .find(|layer| layer.page == page)
    }
}

struct Transition {
    start: Duration,
    incoming: Layer,
    outgoing: Option<(Layer, Layer)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPages,
}

pub struct State {
    config: Config,
    selected: Option<NodeId>,
    transition: Option<Transition>,
    painted: Sample,
    epoch: Arc<()>,
}
impl State {
    fn valid(pages: &[NodeId], config: Config) -> bool {
        config.valid_children(pages.len())
            && pages.iter().collect::<BTreeSet<_>>().len() == pages.len()
    }

    pub fn new(pages: &[NodeId], config: Config, now: Duration) -> Result<Self, Error> {
        if !Self::valid(pages, config) {
            return Err(Error::InvalidPages);
        }
        let selected = config.selected.map(|index| pages[index as usize]);
        let epoch = Arc::new(());
        Ok(Self {
            config,
            selected,
            transition: None,
            painted: Sample {
                current: selected.map(Layer::settled),
                outgoing: None,
                needs_frame: false,
                at: now,
                epoch: epoch.clone(),
            },
            epoch,
        })
    }

    pub fn selected(&self) -> Option<NodeId> {
        self.selected
    }

    /// Rejected updates leave the current presentation intact. Identical selection
    /// preserves its timeline; a policy change settles it. Removed pages disappear
    /// immediately. Retargeting starts from accepted paint, never speculative layout.
    pub fn update(&mut self, pages: &[NodeId], config: Config, now: Duration) -> Result<(), Error> {
        self.update_with_direction(pages, config, now, None)
    }

    /// A looping presenter can specify the requested direction independently of
    /// numeric positions. Retargeting still starts at accepted painted geometry.
    pub fn update_with_direction(
        &mut self,
        pages: &[NodeId],
        config: Config,
        now: Duration,
        direction: Option<std::cmp::Ordering>,
    ) -> Result<(), Error> {
        if !Self::valid(pages, config) {
            return Err(Error::InvalidPages);
        }
        let selected = config.selected.map(|index| pages[index as usize]);
        let policy_changed = config.motion != self.config.motion
            || config.duration_ms != self.config.duration_ms
            || config.retain != self.config.retain;
        if selected == self.selected {
            self.config = config;
            if policy_changed {
                self.settle(now);
            } else if let Some(transition) = &mut self.transition
                && transition
                    .outgoing
                    .is_some_and(|(layer, _)| !pages.contains(&layer.page))
            {
                transition.outgoing = None;
                self.epoch = Arc::new(());
            }
            return Ok(());
        }

        let previous_index = self.config.selected;
        // Stable IDs take precedence over positions: inserting/removing unrelated
        // history entries must not reverse a back/forward transition.
        let old_position = self
            .selected
            .and_then(|id| pages.iter().position(|page| *page == id));
        let new_position = config.selected.map(|index| index as usize);
        let direction = direction.unwrap_or_else(|| match (old_position, new_position) {
            (Some(old), Some(new)) => new.cmp(&old),
            _ => config.selected.cmp(&previous_index),
        });
        let slide = config.motion == Motion::Slide && direction != std::cmp::Ordering::Equal;
        let sign = if direction == std::cmp::Ordering::Less {
            -1.
        } else {
            1.
        };
        let outgoing = if config.retain {
            self.selected
                .and_then(|id| self.painted.layer(id))
                .or(self.painted.current)
                .filter(|layer| Some(layer.page) != selected && pages.contains(&layer.page))
        } else {
            None
        };
        self.transition = selected.filter(|_| self.selected.is_some()).map(|page| {
            let incoming = self.painted.layer(page).unwrap_or(Layer {
                page,
                offset: if slide { sign } else { 0. },
                opacity: if slide { 1. } else { 0. },
            });
            Transition {
                start: now,
                incoming,
                outgoing: outgoing.map(|layer| {
                    (
                        layer,
                        Layer {
                            page: layer.page,
                            offset: if slide { -sign } else { 0. },
                            opacity: if slide { 1. } else { 0. },
                        },
                    )
                }),
            }
        });
        self.selected = selected;
        self.config = config;
        self.epoch = Arc::new(());
        if config.motion == Motion::Immediate || config.duration_ms == 0 {
            self.settle(now);
        }
        Ok(())
    }

    /// Hidden presenters and reduced-motion policy settle; becoming visible again
    /// does not replay old navigation. No polling or elapsed hidden time is needed.
    pub fn settle(&mut self, now: Duration) {
        self.transition = None;
        self.epoch = Arc::new(());
        self.painted = self.sample(now);
    }

    pub fn sample(&self, now: Duration) -> Sample {
        let mut current = self.selected.map(Layer::settled);
        let mut outgoing = None;
        let mut needs_frame = false;
        if let Some(transition) = &self.transition {
            let duration = Duration::from_millis(self.config.duration_ms as u64);
            let elapsed = now.saturating_sub(transition.start);
            needs_frame = elapsed < duration;
            if needs_frame {
                let fraction = elapsed.as_secs_f32() / duration.as_secs_f32();
                let eased = fraction * fraction * (3. - 2. * fraction);
                current = current.map(|target| transition.incoming.interpolate(target, eased));
                outgoing = transition
                    .outgoing
                    .map(|(from, to)| from.interpolate(to, eased));
            }
        }
        Sample {
            current,
            outgoing,
            needs_frame,
            at: now,
            epoch: self.epoch.clone(),
        }
    }

    /// Returns false for an obsolete render, including a render before disposal or
    /// reduced-motion settling. The caller only schedules frames for accepted paint.
    pub fn painted(&mut self, sample: Sample) -> bool {
        if !Arc::ptr_eq(&sample.epoch, &self.epoch) || sample.at < self.painted.at {
            return false;
        }
        if !sample.needs_frame {
            self.transition = None;
        }
        self.painted = sample;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(slot: i64) -> NodeId {
        NodeId::from_parts(slot, 1).unwrap()
    }
    fn config(selected: Option<i64>) -> Config {
        Config {
            selected,
            retain: true,
            motion: Motion::Slide,
            duration_ms: 200,
        }
    }
    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn first_placement_push_pop_and_painted_reversal() {
        let pages = [page(0), page(1), page(2)];
        let mut state = State::new(&pages, config(Some(0)), ms(0)).unwrap();
        assert!(!state.sample(ms(0)).needs_frame);
        state.update(&pages, config(Some(1)), ms(10)).unwrap();
        let start = state.sample(ms(10));
        assert_eq!(start.current.unwrap().offset, 1.);
        assert_eq!(start.outgoing.unwrap().page, page(0));
        let half = state.sample(ms(110));
        assert_eq!(half.current.unwrap().offset, 0.5);
        assert_eq!(half.outgoing.unwrap().offset, -0.5);
        assert!(state.painted(half.clone()));
        // A layout sample that was never painted cannot affect retargeting.
        let stale = state.sample(ms(160));
        state.update(&pages, config(Some(0)), ms(170)).unwrap();
        assert!(!state.painted(stale));
        let reversed = state.sample(ms(170));
        assert_eq!(reversed.current, half.outgoing);
        assert_eq!(reversed.outgoing, half.current);
        let end = state.sample(ms(370));
        assert!(!end.needs_frame);
        assert!(end.outgoing.is_none());
        assert_eq!(end.current, Some(Layer::settled(page(0))));
        assert!(state.painted(end));
    }

    #[test]
    fn third_destination_retires_obsolete_exit_and_removed_pages() {
        let pages = [page(0), page(1), page(2)];
        let mut state = State::new(&pages, config(Some(0)), ms(0)).unwrap();
        state.update(&pages, config(Some(1)), ms(0)).unwrap();
        state.painted(state.sample(ms(100)));
        state.update(&pages, config(Some(2)), ms(100)).unwrap();
        let sample = state.sample(ms(100));
        assert_eq!(sample.outgoing.unwrap().page, page(1));
        assert_eq!(sample.outgoing.unwrap().offset, 0.5);
        assert_eq!(sample.current.unwrap().page, page(2));
        let stale = sample.clone();
        state.update(&[page(2)], config(Some(0)), ms(110)).unwrap();
        assert!(state.sample(ms(110)).outgoing.is_none());
        assert!(!state.painted(stale));
    }

    #[test]
    fn replacement_fades_without_keeping_removed_resource_identity() {
        let mut state = State::new(&[page(0)], config(Some(0)), ms(0)).unwrap();
        state.update(&[page(1)], config(Some(0)), ms(5)).unwrap();
        let sample = state.sample(ms(105));
        assert!(sample.outgoing.is_none());
        assert_eq!(sample.current.unwrap().offset, 0.);
        assert_eq!(sample.current.unwrap().opacity, 0.5);
        state.update(&[], config(None), ms(110)).unwrap();
        assert!(state.sample(ms(110)).current.is_none());
        assert!(!state.sample(ms(110)).needs_frame);
        state.update(&[page(2)], config(Some(0)), ms(115)).unwrap();
        assert!(!state.sample(ms(115)).needs_frame);
    }

    #[test]
    fn unmount_reduced_hidden_and_policy_changes_do_not_keep_exit() {
        let pages = [page(0), page(1)];
        let mut state = State::new(&pages, config(Some(0)), ms(0)).unwrap();
        let mut unmount = config(Some(1));
        unmount.retain = false;
        state.update(&pages, unmount, ms(5)).unwrap();
        assert!(state.sample(ms(5)).outgoing.is_none());
        assert!(state.sample(ms(5)).needs_frame);
        let stale = state.sample(ms(6));
        state.settle(ms(7));
        assert!(!state.painted(stale));
        assert!(!state.sample(ms(500)).needs_frame);
        state.update(&pages, config(Some(0)), ms(510)).unwrap();
        let mut immediate = config(Some(0));
        immediate.motion = Motion::Immediate;
        state.update(&pages, immediate, ms(511)).unwrap();
        assert!(!state.sample(ms(511)).needs_frame);
    }

    #[test]
    fn invalid_updates_are_atomic_and_same_selection_does_not_restart() {
        let pages = [page(0), page(1)];
        let mut state = State::new(&pages, config(Some(0)), ms(0)).unwrap();
        state.update(&pages, config(Some(1)), ms(10)).unwrap();
        let half = state.sample(ms(110));
        assert_eq!(
            state.update(&[page(0), page(0)], config(Some(1)), ms(110)),
            Err(Error::InvalidPages)
        );
        assert_eq!(
            state.update(&pages, config(None), ms(110)),
            Err(Error::InvalidPages)
        );
        state.update(&pages, config(Some(1)), ms(110)).unwrap();
        assert_eq!(state.sample(ms(110)).current, half.current);
        assert!(!state.sample(ms(210)).needs_frame);
        assert!(State::new(&vec![page(0); 129], config(Some(0)), ms(0)).is_err());
    }

    #[test]
    fn maximum_history_and_repeated_interruptions_keep_only_two_layers() {
        let pages = (0..128).map(page).collect::<Vec<_>>();
        let mut state = State::new(&pages, config(Some(0)), ms(0)).unwrap();
        for step in 1..10_000 {
            let index = step % 128;
            let now = ms(step as u64 * 3);
            state.update(&pages, config(Some(index)), now).unwrap();
            let sample = state.sample(now + ms(2));
            assert_eq!(sample.current.unwrap().page, page(index));
            for layer in sample.current.into_iter().chain(sample.outgoing) {
                assert!(layer.offset.is_finite() && (-1.0..=1.0).contains(&layer.offset));
                assert!((0.0..=1.0).contains(&layer.opacity));
            }
            assert!(sample.outgoing.is_none_or(|out| out.page != page(index)));
            assert!(state.painted(sample));
        }
        state.settle(ms(30_000));
        assert!(state.sample(ms(30_000)).outgoing.is_none());
        assert!(!state.sample(ms(30_000)).needs_frame);
    }

    #[test]
    fn history_edit_uses_identity_and_never_painted_destinations_do_not_become_exits() {
        let pages = [page(0), page(1), page(2)];
        let mut state = State::new(&pages, config(Some(0)), ms(0)).unwrap();
        state.update(&pages, config(Some(1)), ms(5)).unwrap();
        state.update(&pages, config(Some(2)), ms(6)).unwrap();
        assert_eq!(state.sample(ms(6)).outgoing.unwrap().page, page(0));
        state.painted(state.sample(ms(206)));
        // Insertion shifts numeric indices, but current page 2 is still after 0.
        let inserted = [page(3), page(4), page(5), page(0), page(2)];
        state.update(&inserted, config(Some(3)), ms(207)).unwrap();
        assert_eq!(state.sample(ms(207)).current.unwrap().offset, -1.);
        let stale = state.sample(ms(210));
        state.painted(state.sample(ms(220)));
        assert!(!state.painted(stale));
    }

    #[test]
    fn looping_direction_overrides_positions_without_resetting_painted_reversal() {
        use std::cmp::Ordering::{Greater, Less};
        let pages = [page(0), page(1), page(2)];
        let mut state = State::new(&pages, config(Some(2)), ms(0)).unwrap();
        state
            .update_with_direction(&pages, config(Some(0)), ms(1), Some(Greater))
            .unwrap();
        assert_eq!(state.sample(ms(1)).current.unwrap().offset, 1.);
        let half = state.sample(ms(101));
        assert_eq!(half.outgoing.unwrap().offset, -0.5);
        assert!(state.painted(half.clone()));
        state
            .update_with_direction(&pages, config(Some(2)), ms(102), Some(Less))
            .unwrap();
        let reverse = state.sample(ms(102));
        assert_eq!(reverse.current, half.outgoing);
        assert_eq!(reverse.outgoing, half.current);
        assert!(state.painted(state.sample(ms(302))));
        state.update(&pages, config(Some(0)), ms(303)).unwrap();
        assert_eq!(state.sample(ms(303)).current.unwrap().offset, -1.);
    }
}
