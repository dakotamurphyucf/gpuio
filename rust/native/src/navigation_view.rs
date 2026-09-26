//! Navigation pages share the retained tree; only their presentation is native.
use super::*;
use crate::navigation_motion;
use std::time::Instant;

pub(super) struct State {
    origin: Instant,
    motion: navigation_motion::State,
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
            for id in dirty {
                let Some(node) = tree.get(*id) else { continue };
                let Some(config) = node.navigation_stack else {
                    continue;
                };
                if let Some(state) = self.navigation.get(id) {
                    let mut state = state.borrow_mut();
                    let now = state.origin.elapsed();
                    state
                        .motion
                        .update(&node.children, config, now)
                        .expect("admitted navigation");
                } else {
                    self.navigation.insert(
                        *id,
                        Rc::new(RefCell::new(State {
                            origin: Instant::now(),
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
            let now = state.origin.elapsed();
            if cx.reduce_motion() || !self.focus.borrow().visible(node.id) {
                state.motion.settle(now);
            }
            state.motion.sample(now)
        };
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
                page = page
                    .left(gpui::relative(layer.offset))
                    .opacity(layer.opacity);
            } else {
                page = page.hidden();
            }
            let body = if Some(*id) != selected {
                crate::semantics::Inert(body).into_any_element()
            } else {
                body
            };
            pages.push(page.child(body).into_any_element());
        }
        let gate = self.focus.clone();
        let owner = node.id;
        let state = Rc::downgrade(&state);
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .children(pages)
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, cx| {
                        let Some(state) = state.upgrade() else { return };
                        let mut state = state.borrow_mut();
                        if !gate.borrow().visible(owner) || cx.reduce_motion() {
                            let now = state.origin.elapsed();
                            state.motion.settle(now);
                            return;
                        }
                        let needs_frame = sample.needs_frame;
                        if state.motion.painted(sample) && needs_frame {
                            window.request_animation_frame();
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }
}
