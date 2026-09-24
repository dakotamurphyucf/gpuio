//! Retained animation adapter. Samples stay in Rust; only endpoints enter transport.
use super::*;
use crate::motion::{Sample, State as Motion, Values, Wake};
use gpuio_protocol::animation::{Config, Endpoint, Property};
use std::time::{Duration, Instant};

pub(super) struct State {
    motion: Motion,
    config: Arc<Config>,
    source_style: Arc<[Style]>,
    pub(super) styles: Arc<[Style]>,
    origin: Instant,
    deadline: Option<Duration>,
    timer: Option<gpui::Task<()>>,
    window: WindowId,
    node: NodeId,
    session: SharedSession,
    transport: Arc<Transport>,
    gate: focus::Shared,
    #[cfg(feature = "native-tests")]
    test_now: Option<Duration>,
    #[cfg(feature = "native-tests")]
    pub(super) paint_count: u64,
}
impl State {
    #[cfg(feature = "native-tests")]
    pub(super) fn has_deadline(&self) -> bool {
        self.timer.is_some() && self.deadline.is_some()
    }
    fn now(&self) -> Duration {
        #[cfg(feature = "native-tests")]
        if let Some(now) = self.test_now {
            return now;
        }
        self.origin.elapsed()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn set_test_time(&mut self, now: Option<Duration>) {
        if now.is_none()
            && let Some(previous) = self.test_now
        {
            self.origin = Instant::now() - previous;
        }
        self.test_now = now;
    }

    fn publish(&self, endpoint: Endpoint) {
        let event = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.window) else {
                return;
            };
            let Some(handler) = tree.get(self.node).and_then(|node| node.handler) else {
                return;
            };
            session.animation_endpoint(self.window, self.node, handler, tree.revision(), endpoint)
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.window)
        {
            self.transport.fault(self.window);
        }
    }
    fn update(&mut self, node: &crate::tree::Node) -> Option<Endpoint> {
        let config = node.animation.as_ref().expect("validated animation");
        let changed = config != &self.config;
        let endpoint = if changed {
            self.timer = None;
            self.deadline = None;
            self.motion
                .retarget(config.clone(), self.now())
                .expect("validated animation generation")
        } else {
            None
        };
        if changed || !Arc::ptr_eq(&self.source_style, &node.style) {
            self.styles = filtered_styles(&node.style, config);
            self.source_style = node.style.clone();
        }
        self.config = config.clone();
        endpoint
    }
}
fn property(field: &Field) -> Option<Property> {
    Some(match field {
        Field::Width(_) => Property::Width,
        Field::Height(_) => Property::Height,
        Field::Top(_) => Property::Top,
        Field::Right(_) => Property::Right,
        Field::Bottom(_) => Property::Bottom,
        Field::Left(_) => Property::Left,
        Field::Opacity(_) => Property::Opacity,
        Field::TopLeftRadius(_) => Property::TopLeftRadius,
        Field::TopRightRadius(_) => Property::TopRightRadius,
        Field::BottomLeftRadius(_) => Property::BottomLeftRadius,
        Field::BottomRightRadius(_) => Property::BottomRightRadius,
        _ => return None,
    })
}
fn filtered_styles(styles: &[Style], config: &Config) -> Arc<[Style]> {
    let keep = |field: &Field| {
        property(field).is_none_or(|property| {
            !config
                .targets
                .iter()
                .any(|target| target.property == property)
        })
    };
    styles
        .iter()
        .map(|style| {
            let fields = match style {
                Style::Fields(fields) => {
                    return Style::Fields(
                        fields.iter().filter(|field| keep(field)).cloned().collect(),
                    );
                }
                Style::State(state, fields) => {
                    return Style::State(
                        *state,
                        fields.iter().filter(|field| keep(field)).cloned().collect(),
                    );
                }
                Style::Width(value) => vec![Field::Width(value.clone())],
                Style::Height(value) => vec![Field::Height(value.clone())],
                Style::Opacity(value) => vec![Field::Opacity(*value)],
                Style::Radius(value) => vec![
                    Field::TopLeftRadius(*value),
                    Field::TopRightRadius(*value),
                    Field::BottomLeftRadius(*value),
                    Field::BottomRightRadius(*value),
                ],
                _ => return style.clone(),
            };
            Style::Fields(fields.into_iter().filter(keep).collect())
        })
        .collect()
}
pub(super) fn apply(style: &mut gpui::StyleRefinement, values: &Values) {
    for property in [
        Property::Width,
        Property::Height,
        Property::Top,
        Property::Right,
        Property::Bottom,
        Property::Left,
        Property::Opacity,
        Property::TopLeftRadius,
        Property::TopRightRadius,
        Property::BottomLeftRadius,
        Property::BottomRightRadius,
    ] {
        let Some(value) = values.get(property) else {
            continue;
        };
        let field = match property {
            Property::Width => Field::Width(Length::Px(value)),
            Property::Height => Field::Height(Length::Px(value)),
            Property::Top => Field::Top(Length::Px(value)),
            Property::Right => Field::Right(Length::Px(value)),
            Property::Bottom => Field::Bottom(Length::Px(value)),
            Property::Left => Field::Left(Length::Px(value)),
            Property::Opacity => Field::Opacity(value),
            Property::TopLeftRadius => Field::TopLeftRadius(value),
            Property::TopRightRadius => Field::TopRightRadius(value),
            Property::BottomLeftRadius => Field::BottomLeftRadius(value),
            Property::BottomRightRadius => Field::BottomRightRadius(value),
        };
        crate::style::refine(style, std::slice::from_ref(&field));
    }
}
pub(super) fn paint(state: &Rc<RefCell<State>>, sample: Sample) -> gpui::AnyElement {
    let state = Rc::downgrade(state);
    canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            let Some(state) = state.upgrade() else {
                return;
            };
            let mut state = state.borrow_mut();
            if !state.gate.borrow().visible(state.node) {
                let now = state.now();
                state.motion.set_visible(false, now);
                state.timer = None;
                state.deadline = None;
                return;
            }
            if !state.motion.accepts_sample(&sample) {
                return;
            }
            #[cfg(feature = "native-tests")]
            {
                state.paint_count += 1;
            }
            let wake = sample.wake;
            let endpoint = state.motion.painted(sample);
            if let Some(endpoint) = endpoint {
                state.publish(endpoint);
            }
            if wake == Wake::Frame {
                window.request_animation_frame();
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}
impl View {
    fn ensure_animation(&mut self, node: &crate::tree::Node, cx: &App) -> Rc<RefCell<State>> {
        self.animations
            .entry(node.id)
            .or_insert_with(|| {
                let config = node
                    .animation
                    .as_ref()
                    .expect("validated animation")
                    .clone();
                Rc::new(RefCell::new(State {
                    motion: Motion::new(config.clone(), Duration::ZERO, cx.reduce_motion())
                        .expect("validated animation"),
                    styles: filtered_styles(&node.style, &config),
                    config,
                    source_style: node.style.clone(),
                    origin: Instant::now(),
                    deadline: None,
                    timer: None,
                    #[cfg(feature = "native-tests")]
                    test_now: None,
                    #[cfg(feature = "native-tests")]
                    paint_count: 0,
                    window: self.id,
                    node: node.id,
                    session: self.session.clone(),
                    transport: self.transport.clone(),
                    gate: self.focus.clone(),
                }))
            })
            .clone()
    }
    pub(super) fn sync_animations(&mut self, dirty: &[NodeId], cx: &App) {
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.animations.clear();
                return;
            };
            self.animations.retain(|id, state| {
                let keep = tree.get(*id).is_some_and(|node| node.animation.is_some());
                if !keep {
                    state
                        .borrow_mut()
                        .motion
                        .cancel(gpuio_protocol::animation::CancelReason::Removed);
                }
                keep
            });
            dirty
                .iter()
                .filter_map(|id| tree.get(*id))
                .filter(|node| node.animation.is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        for node in nodes {
            let state = self.ensure_animation(&node, cx);
            let endpoint = state.borrow_mut().update(&node);
            if let Some(endpoint) = endpoint {
                state.borrow().publish(endpoint);
            }
        }
    }
    pub(super) fn animation_frame(
        &mut self,
        node: &crate::tree::Node,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(Rc<RefCell<State>>, Sample)> {
        node.animation.as_ref()?;
        let state = self.ensure_animation(node, cx);
        let mut item = state.borrow_mut();
        let now = item.now();
        item.motion
            .set_visible(self.focus.borrow().visible(node.id), now);
        item.motion.set_reduced_motion(cx.reduce_motion(), now);
        let sample = item.motion.sample(now);
        match sample.wake {
            Wake::At(deadline) if item.deadline != Some(deadline) => {
                item.timer = None;
                item.deadline = Some(deadline);
                let id = node.id;
                let generation = item.config.generation;
                let delay = deadline.saturating_sub(now);
                item.timer = Some(cx.spawn_in(window, async move |owner, cx| {
                    cx.background_executor().timer(delay).await;
                    let _ = owner.update_in(cx, |view, window, _| {
                        let Some(state) = view.animations.get(&id) else {
                            return;
                        };
                        let mut state = state.borrow_mut();
                        if state.config.generation == generation && state.deadline == Some(deadline)
                        {
                            state.timer = None;
                            state.deadline = None;
                            if view.focus.borrow().visible(id) {
                                window.refresh();
                            }
                        }
                    });
                }));
            }
            Wake::At(_) => (),
            Wake::Idle | Wake::Frame => {
                item.timer = None;
                item.deadline = None;
            }
        }
        drop(item);
        Some((state, sample))
    }
}
