//! Navigation pages share the retained tree; only their presentation is native.
use super::*;
use crate::navigation_motion;
use std::time::Instant;

pub(super) struct State {
    origin: Instant,
    #[cfg(feature = "native-tests")]
    test_now: Option<std::time::Duration>,
    motion: navigation_motion::State,
    axis: gpuio_protocol::carousel::Axis,
}

impl State {
    fn now(&self) -> std::time::Duration {
        #[cfg(feature = "native-tests")]
        if let Some(now) = self.test_now {
            return now;
        }
        self.origin.elapsed()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn advance_test_time(&mut self, elapsed: std::time::Duration) {
        self.test_now = Some(self.now() + elapsed);
    }
    pub(super) fn previewing(&self) -> bool {
        self.motion.previewing()
    }
    pub(super) fn drag_origin(&self) -> f32 {
        self.motion.drag_origin()
    }
    pub(super) fn preview(&mut self, offset: f32, neighbor: Option<NodeId>) {
        self.motion
            .preview(offset, neighbor)
            .expect("admitted carousel preview");
    }
    pub(super) fn finish_preview(&mut self, immediate: bool) {
        self.motion.finish_preview(self.now(), immediate);
    }
}

impl View {
    pub(super) fn sync_navigation(&mut self, dirty: &[NodeId]) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.navigation.retain(|id, _| {
            tree.and_then(|t| t.get(*id))
                .is_some_and(|node| node.navigation_stack.is_some())
        });
        if let Some(tree) = tree {
            let mut presenters = dirty
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            for id in dirty {
                if let Some(owner) = tree.get(*id)
                    && owner.carousel.is_some()
                    && let Some(viewport) = owner.children.first()
                {
                    presenters.insert(*viewport);
                }
            }
            for id in &presenters {
                let Some(node) = tree.get(*id) else { continue };
                let Some(config) = node.navigation_stack else {
                    continue;
                };
                let axis = node
                    .parent
                    .and_then(|parent| tree.get(parent))
                    .and_then(|owner| owner.carousel.as_ref())
                    .map_or(gpuio_protocol::carousel::Axis::Horizontal, |config| {
                        config.axis
                    });
                if let Some(state) = self.navigation.get(id) {
                    let mut state = state.borrow_mut();
                    let now = state.now();
                    if state.axis != axis {
                        state.motion.settle(now);
                        state.axis = axis;
                    }
                    state
                        .motion
                        .update_with_direction(
                            &node.children,
                            config,
                            now,
                            node.parent
                                .and_then(|parent| tree.get(parent))
                                .and_then(|owner| owner.carousel.as_ref())
                                .and_then(|config| match config.direction {
                                    gpuio_protocol::carousel::Direction::Direct => None,
                                    gpuio_protocol::carousel::Direction::Previous => {
                                        Some(std::cmp::Ordering::Less)
                                    }
                                    gpuio_protocol::carousel::Direction::Next => {
                                        Some(std::cmp::Ordering::Greater)
                                    }
                                }),
                        )
                        .expect("admitted navigation");
                } else {
                    self.navigation.insert(
                        *id,
                        Rc::new(RefCell::new(State {
                            origin: Instant::now(),
                            #[cfg(feature = "native-tests")]
                            test_now: None,
                            axis,
                            motion: navigation_motion::State::new(
                                &node.children,
                                config,
                                std::time::Duration::ZERO,
                            )
                            .expect("admitted navigation"),
                        })),
                    );
                }
            }
        }
    }

    pub(super) fn navigation_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let state = self.navigation[&node.id].clone();
        let sample = {
            let mut state = state.borrow_mut();
            let now = state.now();
            if (cx.reduce_motion() && !state.motion.previewing())
                || !self.focus.borrow().visible(node.id)
            {
                state.motion.settle(now);
            }
            state.motion.sample(now)
        };
        let vertical = node
            .parent
            .and_then(|parent| tree.get(parent))
            .and_then(|owner| owner.carousel.as_ref())
            .is_some_and(|config| config.axis == gpuio_protocol::carousel::Axis::Vertical);
        let selected = sample.current.map(|layer| layer.page);
        let mut pages = Vec::with_capacity(node.children.len());
        // Build hidden retained descendants as usual to preserve their native owners.
        // Place outgoing before incoming regardless of their history order.
        for id in node
            .children
            .iter()
            .filter(|id| Some(**id) != selected)
            .chain(node.children.iter().filter(|id| Some(**id) == selected))
        {
            let body = self.element(tree, *id, interaction, window, cx);
            let layer = sample
                .current
                .into_iter()
                .chain(sample.outgoing)
                .find(|layer| layer.page == *id);
            let mut page = div().absolute().top_0().size_full().overflow_hidden();
            if let Some(layer) = layer {
                page = if vertical {
                    page.left_0().top(gpui::relative(layer.offset))
                } else {
                    page.left(gpui::relative(layer.offset))
                }
                .opacity(layer.opacity);
            } else {
                page = page.hidden();
            }
            let body = if Some(*id) != selected {
                let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
                let identity = ("gpuio-inactive-page", identity).into();
                if let Some(carousel) = node.parent.and_then(|parent| self.carousels.get(&parent))
                    && layer.is_some()
                {
                    carousel::input::inert_page(body, carousel, identity)
                } else {
                    crate::semantics::InteractionShield::inert(body, identity).into_any_element()
                }
            } else {
                body
            };
            pages.push(page.child(body).into_any_element());
        }
        let carousel = node
            .parent
            .and_then(|parent| self.carousels.get(&parent))
            .map(Rc::downgrade);
        let gate = self.focus.clone();
        let owner = node.id;
        let presenter = Rc::downgrade(&state);
        let state = presenter.clone();
        let element = div()
            .relative()
            .size_full()
            .overflow_hidden()
            .children(pages)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let Some(state) = state.upgrade() else { return };
                        let mut state = state.borrow_mut();
                        if !gate.borrow().visible(owner) {
                            let now = state.now();
                            state.motion.settle(now);
                            return;
                        }
                        let needs_frame = sample.needs_frame;
                        if state.motion.painted(sample) {
                            if let Some(carousel) =
                                carousel.as_ref().and_then(std::rc::Weak::upgrade)
                            {
                                carousel.borrow_mut().painted(bounds, !needs_frame, window);
                            }
                            if needs_frame {
                                window.request_animation_frame();
                            }
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element();
        if let Some(parent) = node.parent
            && let Some(carousel) = self.carousels.get(&parent)
        {
            carousel::input::Region {
                element,
                state: carousel.clone(),
                owner: cx.weak_entity(),
                node: parent,
                enabled: interaction.pointer,
                presenter,
            }
            .into_any_element()
        } else {
            element
        }
    }
}
