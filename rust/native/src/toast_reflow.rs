//! Keyed, paint-committed notification rectangles. No clock or child resources.
use crate::motion::spring::{Sample as Scalar, Trajectory};
use gpuio_protocol::{
    NodeId,
    animation::{Property, Spring},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    time::Duration,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    fn values(self) -> [f64; 4] {
        [self.x, self.y, self.width, self.height]
    }
    fn valid(self) -> bool {
        self.values().into_iter().all(f64::is_finite)
            && self.x.abs() <= 33_000_000.
            && self.y.abs() <= 33_000_000.
            && (0.0..=1_000_000.).contains(&self.width)
            && (0.0..=1_000_000.).contains(&self.height)
    }
    fn from_values(v: [f64; 4]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            width: v[2],
            height: v[3],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub id: NodeId,
    pub rect: Rect,
}
struct Travel {
    scalar: Trajectory,
    start: Duration,
}
struct Item {
    last: Option<[Scalar; 4]>,
    travel: [Option<Travel>; 4],
}
#[derive(Clone)]
pub struct Frame {
    items: Vec<Target>,
    needs_frame: bool,
    at: Duration,
    values: Vec<[Scalar; 4]>,
    epoch: Rc<()>,
    paint_epoch: Rc<()>,
}
impl Frame {
    pub fn items(&self) -> &[Target] {
        &self.items
    }
    pub fn needs_frame(&self) -> bool {
        self.needs_frame
    }
    pub fn needs_frame_for(&self, ids: &[NodeId]) -> bool {
        self.items
            .iter()
            .zip(&self.values)
            .any(|(item, values)| ids.contains(&item.id) && values.iter().any(|s| !s.finished))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidTargets,
    InvalidSpring,
}
pub struct State {
    targets: Vec<Target>,
    items: BTreeMap<NodeId, Item>,
    spring: Option<Spring>,
    painted_at: Option<Duration>,
    epoch: Rc<()>,
    paint_epoch: Rc<()>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            targets: vec![],
            items: BTreeMap::new(),
            spring: None,
            painted_at: None,
            epoch: Rc::new(()),
            paint_epoch: Rc::new(()),
        }
    }
}
fn settled(v: f64) -> Scalar {
    Scalar {
        position: v,
        velocity: 0.,
        finished: true,
    }
}
impl State {
    /// Accepted tree removal releases geometry even while the native window is
    /// not drawing. Surviving IDs retain their current paint/velocity history.
    pub fn retain(&mut self, ids: &[NodeId]) -> Result<(), Error> {
        let keep: BTreeSet<_> = ids.iter().copied().collect();
        if ids.len() > 32 || keep.len() != ids.len() {
            return Err(Error::InvalidTargets);
        }
        let before = self.items.len();
        self.items.retain(|id, _| keep.contains(id));
        self.targets.retain(|t| keep.contains(&t.id));
        if before != self.items.len() {
            self.epoch = Rc::new(());
        }
        Ok(())
    }
    /// Compose anchor and local card offset before calling this. Retargeting
    /// preserves painted velocity, not an uncommitted future layout sample.
    pub fn configure(
        &mut self,
        targets: &[Target],
        spring: Option<Spring>,
        now: Duration,
    ) -> Result<(), Error> {
        self.configure_at(targets, spring, now)
    }
    /// Layout discovers targets at a frame boundary. Integrate from the last
    /// committed paint, otherwise a target updated every frame would never move.
    /// Unlike command-time configure, this method consumes the elapsed frame.
    pub fn configure_frame(
        &mut self,
        targets: &[Target],
        spring: Option<Spring>,
        now: Duration,
    ) -> Result<(), Error> {
        self.configure_at(
            targets,
            spring,
            self.painted_at.map_or(now, |at| at.min(now)),
        )
    }
    /// Prepare widths before text measurement without restarting unchanged axes.
    /// The returned sample is a preview; only the final rectangle frame commits.
    pub fn measure_widths(
        &mut self,
        widths: &[(NodeId, f64)],
        spring: Option<Spring>,
        now: Duration,
    ) -> Result<Frame, Error> {
        let previous: BTreeMap<_, _> = self.targets.iter().map(|t| (t.id, t.rect)).collect();
        let targets: Vec<_> = widths
            .iter()
            .map(|&(id, width)| Target {
                id,
                rect: Rect {
                    width,
                    ..previous.get(&id).copied().unwrap_or(Rect {
                        x: 0.,
                        y: 0.,
                        width,
                        height: 0.,
                    })
                },
            })
            .collect();
        self.configure_frame(&targets, spring, now)?;
        let preview = self.sample(now);
        let mut changed = false;
        for (sample, (_, target)) in preview.items().iter().zip(widths) {
            if sample.rect.width <= 0. && *target > 0. {
                // An underdamped width spring may hit the property's zero
                // boundary. Keep a positive-width card measurable this frame.
                self.items
                    .get_mut(&sample.id)
                    .expect("configured width")
                    .travel[2] = None;
                changed = true;
            }
        }
        if changed {
            self.epoch = Rc::new(());
        }
        Ok(self.sample(now))
    }
    fn configure_at(
        &mut self,
        targets: &[Target],
        spring: Option<Spring>,
        start: Duration,
    ) -> Result<(), Error> {
        if spring.is_some_and(|s| !s.is_valid()) {
            return Err(Error::InvalidSpring);
        }
        let mut ids = BTreeSet::new();
        if targets.len() > 32 || targets.iter().any(|t| !t.rect.valid() || !ids.insert(t.id)) {
            return Err(Error::InvalidTargets);
        }
        if self.targets == targets && self.spring == spring {
            return Ok(());
        }
        let previous: BTreeMap<_, _> = self.targets.iter().map(|t| (t.id, t.rect)).collect();
        self.items.retain(|id, _| ids.contains(id));
        for target in targets {
            let item = self.items.entry(target.id).or_insert(Item {
                last: None,
                travel: std::array::from_fn(|_| None),
            });
            let previous = previous.get(&target.id).copied();
            if previous == Some(target.rect) && self.spring == spring {
                continue;
            }
            let (Some(spring), Some(last)) = (spring, item.last) else {
                item.travel = std::array::from_fn(|_| None);
                continue;
            };
            let to = target.rect.values();
            let before = previous.map(Rect::values);
            let axes = [
                Property::Left,
                Property::Top,
                Property::Width,
                Property::Height,
            ];
            for i in 0..4 {
                if self.spring == Some(spring) && before.is_some_and(|v| v[i] == to[i]) {
                    continue;
                }
                match Trajectory::new(spring, axes[i], last[i].position, last[i].velocity, to[i]) {
                    Ok(scalar) => item.travel[i] = Some(Travel { scalar, start }),
                    Err(_) => {
                        // Snap the entire rectangle outside the spring domain.
                        item.travel = std::array::from_fn(|_| None);
                        break;
                    }
                }
            }
        }
        self.targets = targets.to_vec();
        self.spring = spring;
        self.epoch = Rc::new(());
        Ok(())
    }
    pub fn sample(&self, now: Duration) -> Frame {
        let mut needs_frame = false;
        let values = self
            .targets
            .iter()
            .map(|target| {
                let item = &self.items[&target.id];
                let targets = target.rect.values();
                let values = std::array::from_fn(|i| {
                    item.travel[i].as_ref().map_or_else(
                        || settled(targets[i]),
                        |travel| travel.scalar.sample(now.saturating_sub(travel.start)),
                    )
                });
                needs_frame |= values.iter().any(|s| !s.finished);
                values
            })
            .collect::<Vec<_>>();
        let items = self
            .targets
            .iter()
            .zip(&values)
            .map(|(target, values)| Target {
                id: target.id,
                rect: Rect::from_values(values.map(|s| s.position)),
            })
            .collect();
        Frame {
            items,
            values,
            needs_frame,
            at: now,
            epoch: self.epoch.clone(),
            paint_epoch: self.paint_epoch.clone(),
        }
    }
    pub fn painted(&mut self, frame: &Frame) -> bool {
        let ids: Vec<_> = frame.items.iter().map(|t| t.id).collect();
        self.painted_subset(frame, &ids)
    }
    /// A resize may put the previous position entirely outside the usable area.
    /// Adopt current target positions while preserving any visible width travel.
    /// This invalidates speculative frames and never manufactures a paint sample.
    pub fn settle_positions(&mut self, ids: &[NodeId]) -> Result<(), Error> {
        if ids.len() > 32 || ids.iter().any(|id| !self.items.contains_key(id)) {
            return Err(Error::InvalidTargets);
        }
        for id in ids {
            let item = self.items.get_mut(id).expect("checked identity");
            item.travel[0] = None;
            item.travel[1] = None;
        }
        if !ids.is_empty() {
            self.epoch = Rc::new(());
        }
        Ok(())
    }
    /// Nonpainted layers retain their descriptor but no motion history. Revealing
    /// one cannot replay coordinates that were never presented to the user.
    pub fn painted_subset(&mut self, frame: &Frame, ids: &[NodeId]) -> bool {
        if !Rc::ptr_eq(&frame.epoch, &self.epoch)
            || !Rc::ptr_eq(&frame.paint_epoch, &self.paint_epoch)
            || self.painted_at.is_some_and(|at| frame.at < at)
            || ids.len() > 32
            || ids.iter().any(|id| !self.items.contains_key(id))
        {
            return false;
        }
        for (target, values) in frame.items.iter().zip(&frame.values) {
            let item = self
                .items
                .get_mut(&target.id)
                .expect("current epoch target");
            if !ids.contains(&target.id) {
                item.last = None;
                item.travel = std::array::from_fn(|_| None);
                continue;
            }
            item.last = Some(*values);
            for (travel, sample) in item.travel.iter_mut().zip(values) {
                if sample.finished {
                    *travel = None;
                }
            }
        }
        self.painted_at = Some(frame.at);
        self.paint_epoch = Rc::new(());
        true
    }
    /// Hidden/inactive/reduced-motion owners stop without replaying stale travel.
    /// Their first subsequent paint establishes a fresh resting pose.
    pub fn suspend(&mut self) {
        for item in self.items.values_mut() {
            item.travel = std::array::from_fn(|_| None);
            item.last = None;
        }
        self.epoch = Rc::new(());
    }
    pub fn retained_items(&self) -> usize {
        self.items.len()
    }
}
