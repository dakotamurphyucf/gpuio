//! Native ownership and scheduling for optional measured disclosure motion.
use super::*;
use crate::{reveal_layout, reveal_motion};
use std::time::Instant;

pub(super) struct State {
    origin: Instant,
    motion: reveal_motion::State,
    config: gpuio_protocol::reveal::Config,
    open_display: gpui::Display,
    parent: Option<NodeId>,
    pending: Option<Rc<()>>,
    painted: bool,
    #[cfg(feature = "native-image-tests")]
    measurement: Option<reveal_layout::Measurement>,
}
impl State {
    fn stop(&mut self) {
        self.motion.settle();
        self.pending = None;
    }
}
pub(super) struct Prepared {
    state: std::rc::Weak<RefCell<State>>,
    frame: reveal_motion::Frame,
    outer: gpui::Style,
    node: NodeId,
    focus: std::rc::Weak<RefCell<focus::Manager>>,
    closing: bool,
}

impl View {
    pub(super) fn sync_reveals(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.reveals.retain(|id, state| {
            let keep = tree
                .and_then(|t| t.get(*id))
                .is_some_and(|n| n.reveal.is_some());
            if !keep {
                state.borrow_mut().stop();
            }
            keep
        });
        if let Some(tree) = tree {
            let candidates = dirty
                .iter()
                .copied()
                .chain(self.reveals.keys().copied())
                .collect::<std::collections::BTreeSet<_>>();
            for id in candidates {
                let Some(node) = tree.get(id) else { continue };
                let Some(config) = node.reveal else { continue };
                let now = cx.background_executor().now();
                if let Some(state) = self.reveals.get(&node.id) {
                    let mut state = state.borrow_mut();
                    let elapsed = now.saturating_duration_since(state.origin);
                    state
                        .motion
                        .update(config.expanded, config.spring, elapsed)
                        .expect("admitted reveal");
                    if state.config.retain != config.retain || state.parent != node.parent {
                        state.stop();
                    }
                    state.config = config;
                    state.parent = node.parent;
                    if !config.expanded && !config.retain {
                        state.stop();
                    }
                } else {
                    self.reveals.insert(
                        node.id,
                        Rc::new(RefCell::new(State {
                            origin: now,
                            motion: reveal_motion::State::new(config.expanded, config.spring)
                                .expect("admitted reveal"),
                            config,
                            open_display: gpui::Display::Flex,
                            parent: node.parent,
                            pending: None,
                            painted: false,
                            #[cfg(feature = "native-image-tests")]
                            measurement: None,
                        })),
                    );
                }
            }
        }
        if self.reveals.is_empty() {
            self.reveal_activation = None;
        } else if self.reveal_activation.is_none() {
            self.reveal_activation =
                Some(cx.observe_window_activation(window, |view, window, cx| {
                    if !window.is_window_active() {
                        for state in view.reveals.values() {
                            state.borrow_mut().stop();
                        }
                    }
                    cx.notify();
                }));
        }
    }

    pub(super) fn begin_reveal_paint(&self) {
        for state in self.reveals.values() {
            state.borrow_mut().painted = false;
        }
    }
    pub(super) fn close_reveals(&mut self) {
        for state in self.reveals.values() {
            state.borrow_mut().stop();
        }
        self.reveals.clear();
        self.reveal_activation = None;
    }
    pub(super) fn finish_reveal_paint(&self) {
        for state in self.reveals.values() {
            let mut state = state.borrow_mut();
            if !state.painted {
                state.stop();
            }
        }
    }

    pub(super) fn prepare_reveal(
        &self,
        node: &crate::tree::Node,
        parent: Option<&gpui::StyleRefinement>,
        style: &mut gpui::StyleRefinement,
        window: &Window,
        cx: &App,
    ) -> Option<Prepared> {
        let state = self.reveals.get(&node.id)?;
        let mut current = state.borrow_mut();
        let original_display = style.display;
        if current.config.expanded {
            current.open_display = style.display.unwrap_or_default();
        } else {
            // Only the painted body changes display. The retained tree still
            // hides this panel and its descendants from all interaction gates.
            style.display = Some(current.open_display);
        }
        let allowed = parent.is_some_and(|parent| reveal_layout::can_animate(style, parent))
            && style.opacity.unwrap_or(1.) > 0.
            && !node.style.iter().any(|s| matches!(s, Style::State(..)))
            && window.is_window_active()
            && current
                .parent
                .is_some_and(|p| self.focus.borrow().visible(p))
            && !self.focus.borrow().disabled(node.id)
            && !crate::style::inert(&node.style);
        if !allowed || cx.reduce_motion() || (!current.config.expanded && !current.config.retain) {
            current.stop();
        }
        let frame = current.motion.frame(
            cx.background_executor()
                .now()
                .saturating_duration_since(current.origin),
            !allowed || cx.reduce_motion(),
        );
        if !matches!(frame.presentation, reveal_motion::Presentation::Height(_)) {
            style.display = original_display;
        }
        let outer = reveal_layout::split_style(style, frame.presentation);
        Some(Prepared {
            state: Rc::downgrade(state),
            frame,
            outer,
            node: node.id,
            focus: Rc::downgrade(&self.focus),
            closing: !current.config.expanded,
        })
    }
}

impl Prepared {
    pub(super) fn wrap(self, mut body: gpui::AnyElement) -> gpui::AnyElement {
        let identity = ((self.node.generation() as u64) << 32) | self.node.slot() as u64;
        if self.closing {
            body = crate::semantics::InteractionShield::inert(
                body,
                ("gpuio-reveal-inert", identity).into(),
            )
            .into_any_element();
        }
        reveal_layout::Reveal::new(
            ("gpuio-reveal", identity),
            body,
            self.outer,
            self.frame,
            move |frame, measurement, window, _| {
                let (Some(state), Some(focus)) = (self.state.upgrade(), self.focus.upgrade())
                else {
                    return;
                };
                let mut current = state.borrow_mut();
                current.painted = true;
                #[cfg(feature = "native-image-tests")]
                {
                    current.measurement = Some(measurement);
                }
                let bounds = measurement.bounds;
                let mask = window.content_mask().bounds;
                // A zero-height opening still needs its first measurement.
                let visible = bounds.size.width > px(0.)
                    && mask.size.width > px(0.)
                    && mask.size.height > px(0.)
                    && bounds.right() > mask.left()
                    && bounds.left() < mask.right()
                    && bounds.bottom() >= mask.top()
                    && bounds.top() < mask.bottom()
                    && window.element_opacity() > 0.;
                if !visible
                    || !window.is_window_active()
                    || !current.parent.is_some_and(|p| focus.borrow().visible(p))
                {
                    current.stop();
                    return;
                }
                let needs_frame = match measurement.natural {
                    Some(size) => current.motion.painted(
                        frame,
                        f64::from(size.height),
                        f64::from(bounds.size.height),
                    ),
                    None => current.motion.painted_closed(frame),
                };
                if !needs_frame {
                    current.pending = None;
                    return;
                }
                if current.pending.is_some() {
                    return;
                }
                let token = Rc::new(());
                let lease = Rc::downgrade(&token);
                current.pending = Some(token);
                let weak = Rc::downgrade(&state);
                let gate = Rc::downgrade(&focus);
                window.on_next_frame(move |window, _| {
                    let (Some(state), Some(token), Some(gate)) =
                        (weak.upgrade(), lease.upgrade(), gate.upgrade())
                    else {
                        return;
                    };
                    let mut current = state.borrow_mut();
                    if !current
                        .pending
                        .as_ref()
                        .is_some_and(|p| Rc::ptr_eq(p, &token))
                    {
                        return;
                    }
                    current.pending = None;
                    if !window.is_window_active()
                        || !current.parent.is_some_and(|p| gate.borrow().visible(p))
                    {
                        current.stop();
                        return;
                    }
                    // Also honor the final layout request after the spring stops.
                    window.refresh();
                });
            },
        )
        .into_any_element()
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "reveal_view_test.rs"]
mod tests;
