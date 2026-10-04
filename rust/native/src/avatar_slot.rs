//! Native avatar slot selection. The retained fallback owns its resources even
//! while a primary image paints; visibility independently suspends its clocks.
use super::*;

pub(super) struct State {
    child: NodeId,
    visible: bool,
    painted: bool,
}

impl View {
    pub(super) fn sync_avatar_fallbacks(&mut self, dirty: &[NodeId]) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.avatar_fallbacks.retain(|id, _| {
            tree.and_then(|tree| tree.get(*id))
                .is_some_and(|node| node.avatar.is_some() && node.children.len() == 1)
        });
        if let Some(tree) = tree {
            for id in dirty {
                let Some(node) = tree.get(*id).filter(|node| node.avatar.is_some()) else {
                    continue;
                };
                let [child] = node.children.as_ref() else {
                    continue;
                };
                let state = self.avatar_fallbacks.entry(*id).or_insert(State {
                    child: *child,
                    visible: false,
                    painted: false,
                });
                if state.child != *child {
                    *state = State {
                        child: *child,
                        visible: false,
                        painted: false,
                    };
                }
            }
        }
        self.focus.borrow_mut().set_avatar_hidden(
            self.avatar_fallbacks
                .values()
                .filter_map(|state| (!state.visible).then_some(state.child))
                .collect(),
        );
    }

    pub(super) fn begin_avatar_paint(&mut self) {
        for state in self.avatar_fallbacks.values_mut() {
            state.painted = false;
        }
    }

    pub(super) fn finish_avatar_paint(&mut self) {
        let mut changed = false;
        for state in self.avatar_fallbacks.values_mut() {
            if state.visible && !state.painted {
                state.visible = false;
                self.focus
                    .borrow_mut()
                    .show_avatar_fallback(state.child, false);
                changed = true;
            }
        }
        if changed {
            self.suspend_hidden_animations();
            self.suspend_hidden_programs();
        }
    }

    fn select_avatar_fallback(&mut self, id: NodeId, child: NodeId, visible: bool) -> bool {
        let Some(state) = self
            .avatar_fallbacks
            .get_mut(&id)
            .filter(|state| state.child == child)
        else {
            return false;
        };
        if state.visible != visible {
            state.visible = visible;
            self.focus.borrow_mut().show_avatar_fallback(child, visible);
            self.suspend_hidden_animations();
            self.suspend_hidden_programs();
        }
        true
    }

    pub(super) fn avatar_slot(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        corners: image_corners::Shared,
        rendered_status: Option<ImageState>,
        passive_disabled: bool,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let [child] = node.children.as_ref() else {
            return div().into_any_element();
        };
        let child = *child;
        let id = node.id;
        let expected_source = node.image.as_ref().map(|config| config.source);
        // Late selection must not retire unselected keyed animations/resources.
        let mut pending = vec![child];
        while let Some(id) = pending.pop() {
            self.visited.insert(id);
            if let Some(node) = tree.get(id) {
                pending.extend(node.children.iter().copied());
            }
        }
        let weak = cx.entity().downgrade();
        let session = self.session.clone();
        let window_id = self.id;
        gpui::container_query(move |size, window, cx| {
            let empty = || div().into_any_element();
            weak.update(cx, |view, cx| {
                let session = session.borrow();
                let Some(tree) = session.tree(window_id) else {
                    return empty();
                };
                let Some(node) = tree.get(id).filter(|node| {
                    node.avatar.is_some()
                        && node.children.as_ref() == [child]
                        && node.image.as_ref().map(|config| config.source) == expected_source
                }) else {
                    return empty();
                };
                if !view.focus.borrow().paint_visible(tree, id)
                    || size.width <= px(0.)
                    || size.height <= px(0.)
                {
                    view.select_avatar_fallback(id, child, false);
                    return empty();
                }
                let primary = view.avatar_primary(node, size, corners, rendered_status, window, cx);
                let fallback = primary.is_none();
                if !view.select_avatar_fallback(id, child, fallback) {
                    return empty();
                }
                if let Some(primary) = primary {
                    return primary;
                }
                let child_element = view.element(
                    tree,
                    child,
                    Interaction {
                        selectable: Some(false),
                        link_content: true,
                        passive_disabled,
                        ..Default::default()
                    },
                    window,
                    cx,
                );
                let painted = cx.entity().downgrade();
                crate::semantics::State {
                    identity: None,
                    busy: false,
                    hidden: true,
                    metadata: None,
                    disabled: false,
                    read_only: false,
                    modal: false,
                    live: None,
                    element: div()
                        .id("avatar-fallback")
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .overflow_hidden()
                        .child(child_element)
                        .child(
                            canvas(
                                |_, _, _| (),
                                move |bounds, _, window, cx| {
                                    if bounds.intersects(&window.content_mask().bounds) {
                                        let _ = painted.update(cx, |view, _| {
                                            if let Some(state) = view.avatar_fallbacks.get_mut(&id)
                                                && state.child == child
                                                && state.visible
                                            {
                                                state.painted = true;
                                            }
                                        });
                                    }
                                },
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        ),
                }
                .into_any_element()
            })
            .unwrap_or_else(|_| empty())
        })
        .into_any_element()
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "avatar_slot_test.rs"]
mod test;
