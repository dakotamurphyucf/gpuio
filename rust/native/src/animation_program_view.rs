//! GPUI adapter for retained programs. Rust owns all timing, samples and tasks.
use super::*;
use crate::{
    motion::Wake,
    motion_program::{Sample, State as Motion},
};
use gpuio_protocol::animation_program::{Config, Signal};
use std::time::Duration;

pub(super) struct State {
    motion: Motion,
    config: Arc<Config>,
    source_style: Arc<[Style]>,
    pub(super) styles: Arc<[Style]>,
    clock: Rc<RefCell<crate::motion_host::Store>>,
    deadline: Option<(Duration, Sample)>,
    timer: Option<gpui::Task<()>>,
    window: WindowId,
    node: NodeId,
    session: SharedSession,
    transport: Arc<Transport>,
    gate: focus::Shared,
    #[cfg(feature = "native-tests")]
    pub(super) paint_count: u64,
}
impl State {
    #[cfg(feature = "native-tests")]
    pub(super) fn has_deadline(&self) -> bool {
        self.timer.is_some() && self.deadline.is_some()
    }
    fn stop_timer(&mut self) {
        self.timer = None;
        self.deadline = None;
    }
    fn hide(&mut self) {
        let now = self.clock.borrow().now();
        self.motion.set_visible(false, now);
        self.stop_timer();
    }
    fn publish(&self, signals: Vec<Signal>) {
        if signals.is_empty() {
            return;
        }
        let event = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.window) else {
                return;
            };
            let Some(handler) = tree.get(self.node).and_then(|n| n.handler) else {
                return;
            };
            session.animation_program_event(
                self.window,
                self.node,
                handler,
                tree.revision(),
                signals,
            )
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.window)
        {
            self.transport.fault(self.window);
        }
    }
    fn update(&mut self, node: &crate::tree::Node) -> Option<Signal> {
        let config = node.animation_program.as_ref().expect("validated program");
        let changed = config != &self.config;
        let signal = if changed {
            self.stop_timer();
            self.motion
                .update(config.clone(), self.clock.borrow().now())
                .expect("admitted program update")
        } else {
            None
        };
        if changed || !Arc::ptr_eq(&self.source_style, &node.style) {
            self.styles =
                animation::filtered_targets(&node.style, &config.program.stages[0].targets);
            self.source_style = node.style.clone();
        }
        self.config = config.clone();
        signal
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
                state.hide();
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
            let signals = state.motion.painted(sample);
            state.publish(signals);
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
    fn ensure_program(&mut self, node: &crate::tree::Node, cx: &App) -> Rc<RefCell<State>> {
        self.animation_programs
            .entry(node.id)
            .or_insert_with(|| {
                let config = node
                    .animation_program
                    .as_ref()
                    .expect("validated program")
                    .clone();
                let clock = self.session.borrow().motion();
                let now = clock.borrow().now();
                Rc::new(RefCell::new(State {
                    motion: Motion::new(config.clone(), now, cx.reduce_motion())
                        .expect("admitted program"),
                    styles: animation::filtered_targets(
                        &node.style,
                        &config.program.stages[0].targets,
                    ),
                    config,
                    source_style: node.style.clone(),
                    clock,
                    deadline: None,
                    timer: None,
                    window: self.id,
                    node: node.id,
                    session: self.session.clone(),
                    transport: self.transport.clone(),
                    gate: self.focus.clone(),
                    #[cfg(feature = "native-tests")]
                    paint_count: 0,
                }))
            })
            .clone()
    }
    pub(super) fn sync_programs(&mut self, dirty: &[NodeId], cx: &App) {
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.animation_programs.clear();
                return;
            };
            self.animation_programs.retain(|id, state| {
                let keep = tree.get(*id).is_some_and(|n| n.animation_program.is_some());
                if !keep {
                    let mut state = state.borrow_mut();
                    state
                        .motion
                        .cancel(gpuio_protocol::animation_program::CancelReason::Removed);
                    state.stop_timer();
                }
                keep
            });
            dirty
                .iter()
                .filter_map(|id| tree.get(*id))
                .filter(|n| n.animation_program.is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        for node in nodes {
            let state = self.ensure_program(&node, cx);
            let signal = state.borrow_mut().update(&node);
            if let Some(signal) = signal {
                state.borrow().publish(vec![signal]);
            }
        }
    }
    pub(super) fn hide_unvisited_programs(&self) {
        for (id, state) in &self.animation_programs {
            if !self.visited.contains(id) {
                state.borrow_mut().hide();
            }
        }
    }
    pub(super) fn program_frame(
        &mut self,
        node: &crate::tree::Node,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(Rc<RefCell<State>>, Sample)> {
        node.animation_program.as_ref()?;
        self.visited.insert(node.id);
        let state = self.ensure_program(node, cx);
        let mut item = state.borrow_mut();
        let (now, clock) = {
            let mut store = item.clock.borrow_mut();
            let now = store.now();
            store.clocks.set_reduced(cx.reduce_motion(), now);
            (now, store.clocks.sample(self.id, node.id, now))
        };
        item.motion
            .set_visible(self.focus.borrow().visible(node.id), now);
        item.motion.set_reduced(cx.reduce_motion(), now);
        let sample = item
            .motion
            .sample(now, clock)
            .expect("admitted program clock");
        match sample.wake {
            Wake::At(deadline) => {
                let reusable = item.deadline.as_ref().is_some_and(|(at, captured)| {
                    *at == deadline && item.motion.accepts_wake(captured)
                });
                if !reusable {
                    item.stop_timer();
                    item.deadline = Some((deadline, sample.clone()));
                    let id = node.id;
                    let captured = sample.clone();
                    item.timer = Some(cx.spawn_in(window, async move |owner, cx| {
                        cx.background_executor()
                            .timer(deadline.saturating_sub(now))
                            .await;
                        let _ = owner.update_in(cx, |view, window, _| {
                            let Some(state) = view.animation_programs.get(&id) else {
                                return;
                            };
                            let mut state = state.borrow_mut();
                            if state
                                .deadline
                                .as_ref()
                                .is_some_and(|(at, _)| *at == deadline)
                                && state.motion.accepts_wake(&captured)
                            {
                                state.stop_timer();
                                if view.focus.borrow().visible(id) {
                                    window.refresh();
                                }
                            }
                        });
                    }));
                }
            }
            Wake::Idle | Wake::Frame => item.stop_timer(),
        }
        drop(item);
        Some((state, sample))
    }
}
