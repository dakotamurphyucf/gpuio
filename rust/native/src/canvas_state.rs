//! Retained canvas interaction without GPUI callbacks. The mounted owner supplies
//! an admitted snapshot and gates input before hiding/modal exclusion/deactivation.
//! Intermediate gestures are native previews; only completed changes emit events.
use crate::canvas_store::Snapshot;
use gpuio_protocol::{
    canvas::{COORDINATE_LIMIT, HitRegion, PathCommand, Point, Rect, Transform},
    canvas_scene::{Drawing, Item, ResourceData, Shape},
    canvas_view::{Action, Command, Config, Error, Observation, Viewport},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerMode {
    Select,
    Pan,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigation {
    First,
    Last,
    Next,
    Previous,
}
#[derive(Clone, Copy)]
struct Extent {
    min: Point,
    max: Point,
}
impl Extent {
    fn point(point: Point) -> Self {
        Self {
            min: point,
            max: point,
        }
    }
    fn extend(&mut self, point: Point) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
    }
    fn corners(self) -> [Point; 4] {
        [
            self.min,
            Point {
                x: self.max.x,
                y: self.min.y,
            },
            self.max,
            Point {
                x: self.min.x,
                y: self.max.y,
            },
        ]
    }
}
fn corners(rect: Rect) -> [Point; 4] {
    Extent {
        min: Point {
            x: rect.x,
            y: rect.y,
        },
        max: Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
    }
    .corners()
}

// Deltas from the published transform which keep its translation and the source
// drawing/hit control hull in the admitted coordinate domain. Linear coefficients
// never change during native manipulation. Computing this once avoids a full
// scene/path traversal for every pointer move.
#[derive(Clone, Copy)]
struct TranslationLimits {
    min: Point,
    max: Point,
}
impl TranslationLimits {
    fn constrain(self, source: Transform, desired: Transform) -> Transform {
        Transform {
            tx: source.tx + (desired.tx - source.tx).clamp(self.min.x, self.max.x),
            ty: source.ty + (desired.ty - source.ty).clamp(self.min.y, self.max.y),
            ..source
        }
    }
}
struct Interactive {
    index: usize,
    limits: TranslationLimits,
}
fn interactive_index(snapshot: &Snapshot) -> (BTreeMap<i64, Interactive>, Vec<i64>) {
    let mut paths = BTreeMap::new();
    for resource in &snapshot.scene.resources {
        if let ResourceData::Path(path) = &resource.data {
            let mut extent: Option<Extent> = None;
            let mut add = |point: Point| match &mut extent {
                Some(extent) => extent.extend(point),
                None => extent = Some(Extent::point(point)),
            };
            for command in &path.0 {
                match command {
                    PathCommand::Move(p) | PathCommand::Line(p) => add(*p),
                    PathCommand::Quadratic(a, p) => {
                        add(*a);
                        add(*p);
                    }
                    PathCommand::Cubic(a, b, p) => {
                        add(*a);
                        add(*b);
                        add(*p);
                    }
                    PathCommand::Close => {}
                }
            }
            paths.insert(resource.key, extent.expect("admitted nonempty path"));
        }
    }
    let mut index = BTreeMap::new();
    let mut order = Vec::new();
    for (position, item) in snapshot.scene.items.iter().enumerate() {
        let Some(interaction) = &item.interaction else {
            continue;
        };
        let mut extent = Extent::point(Point {
            x: item.transform.tx,
            y: item.transform.ty,
        });
        let mut add = |point| extent.extend(item.transform.apply(point));
        match &item.drawing {
            Drawing::Shape(Shape::Rectangle(rect) | Shape::Ellipse(rect), _)
            | Drawing::Image(_, rect) => corners(*rect).into_iter().for_each(&mut add),
            Drawing::Shape(Shape::Path(key), _) => {
                paths[key].corners().into_iter().for_each(&mut add)
            }
            Drawing::Text(_, origin, _) => add(*origin),
        }
        match &interaction.hit_region {
            HitRegion::Rectangle(rect) | HitRegion::Ellipse(rect) => {
                corners(*rect).into_iter().for_each(&mut add)
            }
            HitRegion::Polygon(points) => points.iter().copied().for_each(&mut add),
        }
        index.insert(
            item.id,
            Interactive {
                index: position,
                limits: TranslationLimits {
                    min: Point {
                        x: -COORDINATE_LIMIT - extent.min.x,
                        y: -COORDINATE_LIMIT - extent.min.y,
                    },
                    max: Point {
                        x: COORDINATE_LIMIT - extent.max.x,
                        y: COORDINATE_LIMIT - extent.max.y,
                    },
                },
            },
        );
        order.push(item.id);
    }
    (index, order)
}

#[derive(Clone, Copy)]
enum Gesture {
    Object {
        id: i64,
        start: Point,
        base: Transform,
        preview: Transform,
        moved: bool,
    },
    Pan {
        start: Point,
        base: Viewport,
    },
}

pub struct State {
    snapshot: Arc<Snapshot>,
    config: Config,
    interactive: BTreeMap<i64, Interactive>,
    order: Vec<i64>,
    positions: BTreeMap<i64, Transform>,
    selection: Option<i64>,
    viewport: Viewport,
    gesture: Option<Gesture>,
    input_enabled: bool,
    last_command: Option<Command>,
    command_conflict_reported: bool,
}
impl State {
    /// The host must acquire this snapshot using config.source in its own
    /// application. Input starts disabled until the mounted lifecycle enables it.
    pub fn new(config: Config, snapshot: Arc<Snapshot>) -> Result<(Self, Vec<Observation>), Error> {
        if !config.is_valid() {
            return Err(Error::NativeFailure);
        }
        if config.source.is_none() {
            return Err(Error::UnavailableScene);
        }
        let (interactive, order) = interactive_index(&snapshot);
        let command = config.command.clone();
        let mut state = Self {
            snapshot,
            viewport: config.initial_viewport,
            config,
            interactive,
            order,
            positions: BTreeMap::new(),
            selection: None,
            gesture: None,
            input_enabled: false,
            last_command: None,
            command_conflict_reported: false,
        };
        let events = command.map_or_else(Vec::new, |command| state.command(&command));
        Ok((state, events))
    }
    pub fn snapshot(&self) -> &Arc<Snapshot> {
        &self.snapshot
    }
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
    pub fn selection(&self) -> Option<i64> {
        self.selection
    }
    pub fn has_gesture(&self) -> bool {
        self.gesture.is_some()
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn interactive_ids(&self) -> &[i64] {
        &self.order
    }
    fn item(&self, id: i64) -> Option<&Item> {
        self.interactive
            .get(&id)
            .map(|entry| &self.snapshot.scene.items[entry.index])
    }
    pub fn transform(&self, item: &Item) -> Transform {
        if let Some(Gesture::Object { id, preview, .. }) = self.gesture
            && id == item.id
        {
            return preview;
        }
        self.positions
            .get(&item.id)
            .copied()
            .unwrap_or(item.transform)
    }
    fn enabled(&self) -> bool {
        self.input_enabled && !self.config.disabled
    }
    /// Must be called on each lifecycle transition, not merely from render. A
    /// hide/show cycle cannot resume the old gesture even if no frame was drawn.
    pub fn set_input_enabled(&mut self, enabled: bool) -> bool {
        self.input_enabled = enabled;
        !enabled && self.cancel()
    }
    /// Cancel previews. Selection remains; a drag never mutates its last completed
    /// override until release, and a cancelled pan restores its initial viewport.
    pub fn cancel(&mut self) -> bool {
        let Some(gesture) = self.gesture.take() else {
            return false;
        };
        if let Gesture::Pan { base, .. } = gesture {
            self.viewport = base;
        }
        true
    }
    fn select(&mut self, id: Option<i64>) -> Vec<Observation> {
        if self.selection == id {
            return vec![];
        }
        self.selection = id;
        vec![Observation::SelectionChanged(id)]
    }
    /// Updates are immutable publications. Same-generation updates preserve only
    /// overrides whose published source transform and interaction still exist.
    /// Resetting the scene does not reset the command high watermark.
    pub fn publish(&mut self, snapshot: Arc<Snapshot>) -> Result<Vec<Observation>, Error> {
        if Arc::ptr_eq(&self.snapshot, &snapshot) {
            return Ok(vec![]);
        }
        if snapshot.revision <= self.snapshot.revision
            || snapshot.generation < self.snapshot.generation
        {
            return Err(Error::UnavailableScene);
        }
        self.cancel();
        let (interactive, order) = interactive_index(&snapshot);
        let reset = snapshot.generation != self.snapshot.generation;
        if reset {
            self.positions.clear();
        } else {
            self.positions.retain(|id, transform| {
                let Some(old) = self.interactive.get(id) else {
                    return false;
                };
                let Some(new) = interactive.get(id) else {
                    return false;
                };
                let source = snapshot.scene.items[new.index].transform;
                // A changed drawing/hit hull may narrow the legal translation.
                // Drop an override that no longer fits rather than silently
                // moving the object to an unreported third position.
                source == self.snapshot.scene.items[old.index].transform
                    && new.limits.constrain(source, *transform) == *transform
            });
        }
        self.snapshot = snapshot;
        self.interactive = interactive;
        self.order = order;
        let mut events = Vec::new();
        if reset
            || self
                .selection
                .is_some_and(|id| !self.interactive.contains_key(&id))
        {
            events.extend(self.select(None));
        }
        if reset && self.viewport != self.config.initial_viewport {
            self.viewport = self.config.initial_viewport;
            events.push(Observation::ViewportChanged(self.viewport));
        }
        Ok(events)
    }
    pub fn configure(&mut self, config: Config) -> Result<Vec<Observation>, Error> {
        if !config.is_valid() {
            return Err(Error::NativeFailure);
        }
        if config.source != self.config.source {
            return Err(Error::UnavailableScene);
        }
        if config.disabled != self.config.disabled
            || config.selectable != self.config.selectable
            || config.draggable != self.config.draggable
            || config.pan_zoom != self.config.pan_zoom
            || config.minimum_zoom != self.config.minimum_zoom
            || config.maximum_zoom != self.config.maximum_zoom
        {
            self.cancel();
        }
        let command = config.command.clone();
        self.config = config;
        let previous = self.viewport;
        self.viewport.zoom = self
            .viewport
            .zoom
            .clamp(self.config.minimum_zoom, self.config.maximum_zoom);
        let mut events = Vec::new();
        if self.viewport != previous {
            events.push(Observation::ViewportChanged(self.viewport));
        }
        if let Some(command) = command {
            events.extend(self.command(&command));
        }
        Ok(events)
    }
    /// Local logical coordinates; the caller handles GPUI bounds and device scale.
    pub fn world_point(&self, local: Point) -> Option<Point> {
        if !local.is_valid() {
            return None;
        }
        let world = Point {
            x: self.viewport.origin.x + local.x / self.viewport.zoom,
            y: self.viewport.origin.y + local.y / self.viewport.zoom,
        };
        world.is_valid().then_some(world)
    }
    pub fn hit_test(&self, local: Point) -> Option<i64> {
        let world = self.world_point(local)?;
        self.order.iter().rev().copied().find(|id| {
            let item = self.item(*id).expect("indexed item");
            item.interaction
                .as_ref()
                .expect("interactive item")
                .hit_region
                .hit(self.transform(item), &item.clips, world)
        })
    }
    pub fn begin_pointer(&mut self, local: Point, mode: PointerMode) -> Vec<Observation> {
        if !self.enabled() || !local.is_valid() {
            return vec![];
        }
        self.cancel();
        if mode == PointerMode::Pan {
            if self.config.pan_zoom {
                self.gesture = Some(Gesture::Pan {
                    start: local,
                    base: self.viewport,
                });
            }
            return vec![];
        }
        let target = self.hit_test(local);
        let events = if self.config.selectable {
            self.select(target)
        } else {
            vec![]
        };
        if self.config.draggable
            && let Some(id) = target
        {
            let item = self.item(id).expect("hit item");
            if item
                .interaction
                .as_ref()
                .expect("interactive item")
                .draggable
            {
                let base = self.transform(item);
                self.gesture = Some(Gesture::Object {
                    id,
                    start: local,
                    base,
                    preview: base,
                    moved: false,
                });
            }
        }
        events
    }
    pub fn move_pointer(&mut self, local: Point) -> bool {
        if !self.enabled() || !local.is_valid() {
            return false;
        }
        match self.gesture {
            Some(Gesture::Object {
                id,
                start,
                base,
                preview,
                moved,
            }) => {
                let dx = local.x - start.x;
                let dy = local.y - start.y;
                let moved = moved || dx.hypot(dy) >= 3.;
                if !moved {
                    return false;
                }
                let desired = Transform {
                    tx: base.tx + dx / self.viewport.zoom,
                    ty: base.ty + dy / self.viewport.zoom,
                    ..base
                };
                let next = self.constrain(id, desired);
                self.gesture = Some(Gesture::Object {
                    id,
                    start,
                    base,
                    preview: next,
                    moved,
                });
                next != preview
            }
            Some(Gesture::Pan { start, base }) => {
                let previous = self.viewport;
                self.viewport.origin = bounded_point(Point {
                    x: base.origin.x - (local.x - start.x) / base.zoom,
                    y: base.origin.y - (local.y - start.y) / base.zoom,
                });
                previous != self.viewport
            }
            None => false,
        }
    }
    fn constrain(&self, id: i64, desired: Transform) -> Transform {
        let entry = &self.interactive[&id];
        entry
            .limits
            .constrain(self.snapshot.scene.items[entry.index].transform, desired)
    }
    pub fn finish_pointer(&mut self) -> Vec<Observation> {
        if !self.enabled() {
            self.cancel();
            return vec![];
        }
        match self.gesture.take() {
            Some(Gesture::Object {
                id, base, preview, ..
            }) if preview != base => {
                self.commit_position(id, preview);
                vec![Observation::Moved(id, preview)]
            }
            Some(Gesture::Pan { base, .. }) if base != self.viewport => {
                vec![Observation::ViewportChanged(self.viewport)]
            }
            _ => vec![],
        }
    }
    fn commit_position(&mut self, id: i64, transform: Transform) {
        if self.item(id).expect("indexed item").transform == transform {
            self.positions.remove(&id);
        } else {
            self.positions.insert(id, transform);
        }
    }
    pub fn navigate(&mut self, direction: Navigation) -> Vec<Observation> {
        if !self.enabled() || !self.config.selectable || self.order.is_empty() {
            return vec![];
        }
        self.cancel();
        let current = self
            .selection
            .and_then(|id| self.order.iter().position(|candidate| *candidate == id));
        let index = match direction {
            Navigation::First => 0,
            Navigation::Last => self.order.len() - 1,
            Navigation::Next => current.map_or(0, |i| (i + 1).min(self.order.len() - 1)),
            Navigation::Previous => current.map_or(self.order.len() - 1, |i| i.saturating_sub(1)),
        };
        self.select(Some(self.order[index]))
    }
    pub fn activate(&mut self, id: i64) -> Vec<Observation> {
        if !self.enabled()
            || !self
                .item(id)
                .is_some_and(|item| item.interaction.as_ref().is_some_and(|i| i.activatable))
        {
            return vec![];
        }
        self.cancel();
        vec![Observation::Activated(id)]
    }
    /// Keyboard/accessibility movement uses logical-pixel deltas and the same
    /// native limits/position ownership as pointer dragging.
    pub fn move_selected(&mut self, delta: Point) -> Vec<Observation> {
        if !self.enabled() || !self.config.draggable || !delta.is_valid() {
            return vec![];
        }
        let Some(id) = self.selection else {
            return vec![];
        };
        let Some(item) = self.item(id) else {
            return vec![];
        };
        if !item
            .interaction
            .as_ref()
            .expect("interactive item")
            .draggable
        {
            return vec![];
        }
        self.cancel();
        let previous = self.transform(self.item(id).expect("selected item"));
        let transform = self.constrain(
            id,
            Transform {
                tx: previous.tx + delta.x / self.viewport.zoom,
                ty: previous.ty + delta.y / self.viewport.zoom,
                ..previous
            },
        );
        if transform == previous {
            return vec![];
        }
        self.commit_position(id, transform);
        vec![Observation::Moved(id, transform)]
    }
    /// Move content by a logical-pixel delta (positive x moves content right).
    /// The host coalesces wheel observations before crossing the bridge.
    pub fn pan_by(&mut self, delta: Point) -> Vec<Observation> {
        if !self.enabled() || !self.config.pan_zoom || !delta.is_valid() {
            return vec![];
        }
        self.cancel();
        let next = Viewport {
            origin: bounded_point(Point {
                x: self.viewport.origin.x - delta.x / self.viewport.zoom,
                y: self.viewport.origin.y - delta.y / self.viewport.zoom,
            }),
            ..self.viewport
        };
        self.set_viewport(next)
    }
    pub fn zoom_at(&mut self, local: Point, factor: f64) -> Vec<Observation> {
        if !self.enabled()
            || !self.config.pan_zoom
            || !local.is_valid()
            || !factor.is_finite()
            || factor <= 0.
        {
            return vec![];
        }
        self.cancel();
        let zoom =
            (self.viewport.zoom * factor).clamp(self.config.minimum_zoom, self.config.maximum_zoom);
        let next = Viewport {
            origin: bounded_point(Point {
                x: self.viewport.origin.x + local.x / self.viewport.zoom - local.x / zoom,
                y: self.viewport.origin.y + local.y / self.viewport.zoom - local.y / zoom,
            }),
            zoom,
        };
        self.set_viewport(next)
    }
    fn set_viewport(&mut self, viewport: Viewport) -> Vec<Observation> {
        if self.viewport == viewport {
            return vec![];
        }
        self.viewport = viewport;
        vec![Observation::ViewportChanged(viewport)]
    }
    /// Explicit commands remain usable while user input is disabled. A failed new
    /// sequence is consumed once; callers retry with a larger sequence. Older
    /// commands are ignored. Mutating the latest sequence reports one failure.
    pub fn command(&mut self, command: &Command) -> Vec<Observation> {
        if !command.is_valid() {
            return vec![Observation::Failed(Error::InvalidCommand)];
        }
        if let Some(last) = &self.last_command {
            if command.sequence < last.sequence {
                return vec![];
            }
            if command.sequence == last.sequence {
                if command.action != last.action && !self.command_conflict_reported {
                    self.command_conflict_reported = true;
                    return vec![Observation::Failed(Error::InvalidCommand)];
                }
                return vec![];
            }
        }
        self.last_command = Some(command.clone());
        self.command_conflict_reported = false;
        let valid = match command.action {
            Action::Select(id) => id.is_none_or(|id| self.interactive.contains_key(&id)),
            Action::SetViewport(viewport) => {
                (self.config.minimum_zoom..=self.config.maximum_zoom).contains(&viewport.zoom)
            }
            Action::ResetViewport | Action::ResetPositions => true,
        };
        if !valid {
            return vec![Observation::Failed(Error::InvalidCommand)];
        }
        self.cancel();
        let mut events = match command.action {
            Action::Select(id) => self.select(id),
            Action::SetViewport(viewport) => self.set_viewport(viewport),
            Action::ResetViewport => self.set_viewport(self.config.initial_viewport),
            Action::ResetPositions => {
                self.positions.clear();
                vec![]
            }
        };
        events.push(Observation::CommandCompleted(command.sequence));
        events
    }
}
fn bounded_point(point: Point) -> Point {
    Point {
        x: point.x.clamp(-COORDINATE_LIMIT, COORDINATE_LIMIT),
        y: point.y.clamp(-COORDINATE_LIMIT, COORDINATE_LIMIT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{asset_store, canvas_store};
    use binprot::BinProtWrite;
    use gpuio_protocol::{ResourceId, canvas_resource::Update, canvas_scene::*};

    fn point(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
    fn item(id: i64, x: f64) -> Item {
        let rect = rectangle(0., 0., 20., 20.);
        Item {
            id,
            transform: Transform {
                tx: x,
                ..Transform::IDENTITY
            },
            clips: vec![],
            drawing: Drawing::Shape(
                Shape::Rectangle(rect),
                Paint {
                    fill: Some(0xffffffff),
                    stroke: None,
                },
            ),
            interaction: Some(Interaction {
                label: format!("Item {id}"),
                hit_region: HitRegion::Rectangle(rect),
                draggable: true,
                activatable: true,
            }),
        }
    }
    fn scene() -> Scene {
        Scene {
            version: 1,
            description: "Interaction fixture".into(),
            resources: vec![],
            items: vec![item(9, 0.), item(3, 10.), item(7, 50.)],
        }
    }
    struct Fixture {
        store: canvas_store::Store,
        id: ResourceId,
        revision: i64,
    }
    impl Fixture {
        fn new() -> Self {
            let mut store = canvas_store::Store::default();
            let id = store.create().unwrap();
            Self {
                store,
                id,
                revision: 0,
            }
        }
        fn publish(&mut self, scene: Scene, generation: i64) -> Arc<Snapshot> {
            let mut bytes = Vec::new();
            scene.binprot_write(&mut bytes).unwrap();
            self.store
                .begin(Update {
                    id: self.id,
                    base: self.revision,
                    revision: self.revision + 1,
                    generation,
                    bytes: bytes.len() as i64,
                })
                .unwrap();
            self.revision += 1;
            let chunk_size = gpuio_protocol::canvas_resource::MAX_CHUNK_BYTES;
            for (index, chunk) in bytes.chunks(chunk_size).enumerate() {
                self.store
                    .chunk(self.id, self.revision, index * chunk_size, chunk)
                    .unwrap();
            }
            self.store
                .publish(self.id, self.revision, &asset_store::Store::default())
                .unwrap();
            self.store.acquire(self.id).unwrap().snapshot()
        }
        fn state(&mut self) -> State {
            let snapshot = self.publish(scene(), 1);
            let (mut state, events) = State::new(
                Config {
                    source: Some(self.id),
                    label: "Fixture".into(),
                    initial_viewport: Viewport::default(),
                    minimum_zoom: 0.5,
                    maximum_zoom: 4.,
                    selectable: true,
                    draggable: true,
                    pan_zoom: true,
                    disabled: false,
                    selection_color: 0xff0000ff,
                    command: None,
                },
                snapshot,
            )
            .unwrap();
            assert!(events.is_empty());
            state.set_input_enabled(true);
            state
        }
    }
    fn command(sequence: i64, action: Action) -> Command {
        Command { sequence, action }
    }
    fn position(state: &State, id: i64) -> Transform {
        state.transform(state.item(id).unwrap())
    }
    fn drag(state: &mut State, from: Point, to: Point) -> Vec<Observation> {
        state.begin_pointer(from, PointerMode::Select);
        state.move_pointer(to);
        state.finish_pointer()
    }

    #[test]
    fn hits_follow_z_order_world_clips_and_completed_positions() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        assert_eq!(state.hit_test(point(15., 5.)), Some(3));
        assert_eq!(
            state.begin_pointer(point(15., 5.), PointerMode::Select),
            vec![Observation::SelectionChanged(Some(3))]
        );
        assert!(
            !state.move_pointer(point(16., 5.)),
            "click jitter must not drag"
        );
        assert!(state.move_pointer(point(35., 5.)));
        assert_eq!(state.hit_test(point(35., 5.)), Some(3));
        assert_eq!(state.hit_test(point(15., 5.)), Some(9));
        let expected = Transform {
            tx: 30.,
            ..Transform::IDENTITY
        };
        assert_eq!(
            state.finish_pointer(),
            vec![Observation::Moved(3, expected)]
        );
        assert!(state.finish_pointer().is_empty());
        assert_eq!(position(&state, 3), expected);
        let mut scene = scene();
        scene.items[1].clips = vec![rectangle(35., 0., 5., 20.)];
        let mut decorative = item(100, 30.);
        decorative.interaction = None;
        scene.items.push(decorative);
        state.publish(fixture.publish(scene, 1)).unwrap();
        assert_eq!(state.hit_test(point(34., 5.)), None);
        assert_eq!(state.hit_test(point(36., 5.)), Some(3));
        state.zoom_at(point(0., 0.), 2.);
        assert_eq!(state.hit_test(point(72., 10.)), Some(3));
    }

    #[test]
    fn cancellation_restores_completed_override_and_new_source_accepts_without_double_move() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        drag(&mut state, point(15., 5.), point(35., 5.));
        let completed = position(&state, 3);
        state.begin_pointer(point(35., 5.), PointerMode::Select);
        state.move_pointer(point(60., 20.));
        assert_ne!(position(&state, 3), completed);
        assert!(state.cancel());
        assert_eq!(position(&state, 3), completed);
        assert!(state.finish_pointer().is_empty());
        // Streaming content that preserves the source transform keeps completion,
        // but every new publication cancels unfinished previews.
        state.begin_pointer(point(35., 5.), PointerMode::Select);
        state.move_pointer(point(45., 5.));
        let same = state.snapshot.clone();
        state.publish(same).unwrap();
        assert!(state.has_gesture(), "same snapshot is not reconfiguration");
        state.publish(fixture.publish(scene(), 1)).unwrap();
        assert!(!state.has_gesture());
        assert_eq!(position(&state, 3), completed);
        let mut accepted = scene();
        accepted.items[1].transform = completed;
        state.publish(fixture.publish(accepted, 1)).unwrap();
        assert_eq!(position(&state, 3), completed);
        assert!(state.positions.is_empty());
        state.publish(fixture.publish(scene(), 2)).unwrap();
        assert_eq!(position(&state, 3).tx, 10.);
        assert_eq!(state.selection(), None);
    }

    #[test]
    fn hide_disable_policy_changes_and_removal_cannot_resume_a_gesture() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        for transition in 0..3 {
            state.begin_pointer(point(15., 5.), PointerMode::Select);
            state.move_pointer(point(25., 5.));
            match transition {
                0 => {
                    assert!(state.set_input_enabled(false));
                    state.set_input_enabled(true);
                }
                1 => {
                    let mut config = state.config.clone();
                    config.disabled = true;
                    state.configure(config).unwrap();
                    assert!(
                        state
                            .begin_pointer(point(15., 5.), PointerMode::Select)
                            .is_empty()
                    );
                    assert!(state.activate(3).is_empty());
                    let mut config = state.config.clone();
                    config.disabled = false;
                    state.configure(config).unwrap();
                }
                _ => {
                    let mut config = state.config.clone();
                    config.draggable = false;
                    state.configure(config).unwrap();
                }
            }
            assert!(!state.has_gesture());
            assert!(!state.move_pointer(point(50., 50.)));
            assert!(state.finish_pointer().is_empty());
            assert_eq!(position(&state, 3).tx, 10.);
        }
        let mut next = scene();
        next.items.remove(1);
        assert_eq!(
            state.publish(fixture.publish(next, 1)).unwrap(),
            vec![Observation::SelectionChanged(None)]
        );
        assert!(state.positions.is_empty());
    }

    #[test]
    fn affine_movement_clamps_drawing_and_hit_hulls_without_changing_linear_transform() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        let mut source = scene();
        source.items[0].transform = Transform {
            a: -2.,
            b: 0.5,
            c: 0.25,
            d: 3.,
            tx: 999_995.,
            ty: 10.,
        };
        // A smaller polygon lets this reflected/sheared rectangle fit the domain.
        source.items[0].interaction.as_mut().unwrap().hit_region =
            HitRegion::Polygon(vec![point(0., 0.), point(20., 0.), point(20., 20.)]);
        state.publish(fixture.publish(source, 1)).unwrap();
        state.command(&command(1, Action::Select(Some(9))));
        let before = position(&state, 9);
        // Drawing right edge already reaches 1,000,000; positive x clamps to zero.
        assert!(state.move_selected(point(500., 0.)).is_empty());
        let events = state.move_selected(point(-100., 50.));
        let after = position(&state, 9);
        assert_eq!(
            (after.a, after.b, after.c, after.d),
            (before.a, before.b, before.c, before.d)
        );
        assert_eq!((after.tx, after.ty), (before.tx - 100., before.ty + 50.));
        assert_eq!(events, vec![Observation::Moved(9, after)]);
        assert!(state.move_selected(point(f64::NAN, 0.)).is_empty());
        assert!(after.is_valid());
    }

    #[test]
    fn pan_preview_cancels_zoom_anchors_and_config_limits_clamp() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        state.begin_pointer(point(10., 10.), PointerMode::Pan);
        assert!(state.move_pointer(point(40., 20.)));
        assert_eq!(state.viewport.origin, point(-30., -10.));
        state.cancel();
        assert_eq!(state.viewport, Viewport::default());
        state.begin_pointer(point(10., 10.), PointerMode::Pan);
        state.move_pointer(point(40., 20.));
        assert_eq!(
            state.finish_pointer(),
            vec![Observation::ViewportChanged(state.viewport)]
        );
        let anchor = state.world_point(point(120., 80.)).unwrap();
        state.zoom_at(point(120., 80.), 2.);
        assert_eq!(state.world_point(point(120., 80.)).unwrap(), anchor);
        assert_eq!(state.viewport.zoom, 2.);
        state.zoom_at(point(120., 80.), f64::MAX);
        assert_eq!(state.viewport.zoom, 4.);
        assert!(state.viewport.is_valid());
        let before = state.viewport;
        assert!(state.zoom_at(point(0., 0.), f64::NAN).is_empty());
        assert_eq!(state.viewport, before);
        let mut config = state.config.clone();
        config.maximum_zoom = 1.5;
        let events = state.configure(config).unwrap();
        assert_eq!(state.viewport.zoom, 1.5);
        assert_eq!(events, vec![Observation::ViewportChanged(state.viewport)]);
        for _ in 0..3 {
            state.pan_by(point(1_000_000., -1_000_000.));
        }
        assert_eq!(state.viewport.origin, point(-1_000_000., 1_000_000.));
        assert!(state.pan_by(point(1., -1.)).is_empty());
    }

    #[test]
    fn commands_are_monotone_consumed_once_and_do_not_replay_after_scene_reset() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        state.set_input_enabled(false);
        let select = command(5, Action::Select(Some(3)));
        assert_eq!(
            state.command(&select),
            vec![
                Observation::SelectionChanged(Some(3)),
                Observation::CommandCompleted(5)
            ]
        );
        assert!(state.command(&select).is_empty());
        assert!(state.command(&command(4, Action::Select(None))).is_empty());
        assert_eq!(
            state.command(&command(5, Action::Select(None))),
            vec![Observation::Failed(Error::InvalidCommand)]
        );
        assert!(
            state
                .command(&command(5, Action::Select(Some(7))))
                .is_empty(),
            "bounded conflict report"
        );
        assert_eq!(state.selection, Some(3));
        let missing = command(6, Action::Select(Some(123)));
        assert_eq!(
            state.command(&missing),
            vec![Observation::Failed(Error::InvalidCommand)]
        );
        assert!(state.command(&missing).is_empty());
        let bad_zoom = command(
            7,
            Action::SetViewport(Viewport {
                zoom: 10.,
                ..Viewport::default()
            }),
        );
        assert_eq!(
            state.command(&bad_zoom),
            vec![Observation::Failed(Error::InvalidCommand)]
        );
        let viewport = Viewport {
            origin: point(10., 20.),
            zoom: 2.,
        };
        state.command(&command(8, Action::SetViewport(viewport)));
        assert_eq!(state.viewport, viewport);
        assert_eq!(
            state.publish(fixture.publish(scene(), 2)).unwrap(),
            vec![
                Observation::SelectionChanged(None),
                Observation::ViewportChanged(Viewport::default())
            ]
        );
        assert!(
            state
                .command(&command(8, Action::SetViewport(viewport)))
                .is_empty()
        );
        assert_eq!(state.viewport, Viewport::default());
        assert_eq!(
            state.command(&command(9, Action::ResetViewport)),
            vec![Observation::CommandCompleted(9)]
        );
        assert_eq!(
            state.command(&command(10, Action::ResetPositions)),
            vec![Observation::CommandCompleted(10)]
        );
    }

    #[test]
    fn keyboard_navigation_uses_scene_order_and_item_policies() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        assert_eq!(state.interactive_ids(), &[9, 3, 7]);
        assert_eq!(
            state.navigate(Navigation::Next),
            vec![Observation::SelectionChanged(Some(9))]
        );
        assert_eq!(
            state.navigate(Navigation::Next),
            vec![Observation::SelectionChanged(Some(3))]
        );
        assert_eq!(state.activate(3), vec![Observation::Activated(3)]);
        state.navigate(Navigation::Last);
        assert_eq!(state.selection(), Some(7));
        assert!(state.navigate(Navigation::Next).is_empty());
        state.navigate(Navigation::Previous);
        assert_eq!(state.selection(), Some(3));
        state.navigate(Navigation::First);
        assert_eq!(state.selection(), Some(9));
        assert!(state.activate(123).is_empty());
        let mut source = scene();
        let policy = source.items[0].interaction.as_mut().unwrap();
        policy.draggable = false;
        policy.activatable = false;
        state.publish(fixture.publish(source, 1)).unwrap();
        assert!(state.activate(9).is_empty());
        assert!(state.move_selected(point(1., 0.)).is_empty());
        state.set_input_enabled(false);
        assert!(state.navigate(Navigation::Next).is_empty());
    }

    #[test]
    fn changed_hit_hull_or_removed_interaction_drops_illegal_overrides_and_stale_publications_do_nothing()
     {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        let stale = state.snapshot.clone();
        state.command(&command(1, Action::Select(Some(3))));
        state.move_selected(point(999_000., 0.));
        assert!(!state.positions.is_empty());
        let mut source = scene();
        source.items[1].interaction.as_mut().unwrap().hit_region =
            HitRegion::Rectangle(rectangle(0., 0., 100_000., 20.));
        state.publish(fixture.publish(source, 1)).unwrap();
        assert!(state.positions.is_empty());
        assert_eq!(position(&state, 3).tx, 10.);
        assert_eq!(state.publish(stale), Err(Error::UnavailableScene));
        assert_eq!(state.snapshot.revision, 2);
        let mut source = scene();
        source.items[1].interaction = None;
        assert_eq!(
            state.publish(fixture.publish(source, 1)).unwrap(),
            vec![Observation::SelectionChanged(None)]
        );
    }

    #[test]
    fn drag_limits_include_path_control_points_outside_the_hit_region() {
        let mut fixture = Fixture::new();
        let mut state = fixture.state();
        let mut source = scene();
        let key = ResourceKey {
            id: 1,
            generation: 1,
        };
        source.resources.push(Resource {
            key,
            data: ResourceData::Path(gpuio_protocol::canvas::Path(vec![
                PathCommand::Move(point(0., 0.)),
                PathCommand::Quadratic(point(999_999., 1.), point(0., 2.)),
            ])),
        });
        source.items[0].drawing = Drawing::Shape(
            Shape::Path(key),
            Paint {
                fill: None,
                stroke: Some(Stroke {
                    color: 0xffffffff,
                    width: 1.,
                }),
            },
        );
        state.publish(fixture.publish(source, 1)).unwrap();
        state.command(&command(1, Action::Select(Some(9))));
        let expected = Transform {
            tx: 1.,
            ..Transform::IDENTITY
        };
        assert_eq!(
            state.move_selected(point(100., 0.)),
            vec![Observation::Moved(9, expected)]
        );
        assert!(state.move_selected(point(100., 0.)).is_empty());
        assert_eq!(position(&state, 9), expected);
    }

    #[test]
    fn large_scene_state_stays_bounded_and_repeated_disposal_releases_snapshots() {
        let mut fixture = Fixture::new();
        let mut initial = fixture.state();
        let config = initial.config.clone();
        let mut large = scene();
        large.items = (0..MAX_ITEMS)
            .map(|i| {
                let mut value = item(i as i64 + 1, (i % 200) as f64 * 30.);
                value.transform.ty = (i / 200) as f64 * 30.;
                if i >= MAX_INTERACTIVE_ITEMS {
                    value.interaction = None;
                }
                value
            })
            .collect();
        let snapshot = fixture.publish(large.clone(), 1);
        initial.publish(snapshot.clone()).unwrap();
        assert_eq!(initial.order.len(), MAX_INTERACTIVE_ITEMS);
        for id in 1..=MAX_INTERACTIVE_ITEMS as i64 {
            initial.command(&command(id, Action::Select(Some(id))));
            initial.move_selected(point(5., 5.));
        }
        assert_eq!(initial.positions.len(), MAX_INTERACTIVE_ITEMS);
        let started = std::time::Instant::now();
        for _ in 0..32 {
            let (state, _) = State::new(config.clone(), snapshot.clone()).unwrap();
            assert_eq!(state.interactive.len(), MAX_INTERACTIVE_ITEMS);
            drop(state);
        }
        let elapsed = started.elapsed();
        let revision = fixture.publish(large, 1);
        initial.publish(revision).unwrap();
        assert_eq!(initial.positions.len(), MAX_INTERACTIVE_ITEMS);
        assert_eq!(initial.positions[&1].tx, 5.);
        fixture.store.release(fixture.id).unwrap();
        assert!(fixture.store.reserved_bytes() > 0);
        drop(initial);
        drop(snapshot);
        assert_eq!(fixture.store.reserved_bytes(), 0);
        eprintln!(
            "canvas state: 20,000 items, 2,048 interactive/overrides, 32 mount/dispose cycles {elapsed:?}, final scene charge 0; not frame latency/RSS"
        );
    }
}
