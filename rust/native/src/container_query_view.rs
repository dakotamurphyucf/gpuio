//! Assigned-size layout stays native. Retained alternatives never size the query.
use super::*;
use gpuio_protocol::container_query::{Config, Snapshot};

pub(super) struct State {
    config: Arc<Config>,
    children: Arc<[NodeId]>,
    selected: Option<usize>,
    visible: bool,
    painted: bool,
    observed: Option<(i64, usize)>,
    sequence: i64,
}
impl View {
    pub(super) fn sync_container_queries(&mut self, dirty: &[NodeId]) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.container_queries.retain(|id, _| {
            tree.and_then(|t| t.get(*id))
                .is_some_and(|n| n.container_query.is_some())
        });
        if let Some(tree) = tree {
            for id in dirty.iter().copied() {
                let Some(node) = tree.get(id) else {
                    continue;
                };
                let Some(config) = &node.container_query else {
                    continue;
                };
                let state = self.container_queries.entry(id).or_insert_with(|| State {
                    config: config.clone(),
                    children: node.children.clone(),
                    selected: None,
                    visible: false,
                    painted: false,
                    observed: None,
                    sequence: 0,
                });
                if state.config != *config {
                    let previous = state.selected.and_then(|i| state.config.branches.get(i));
                    state.selected =
                        previous.and_then(|name| config.branches.iter().position(|n| n == name));
                    state.config = config.clone();
                }
                state.children = node.children.clone();
            }
        }
        self.focus.borrow_mut().set_query_hidden(
            self.container_queries
                .values()
                .flat_map(|state| {
                    state.children.iter().enumerate().filter_map(|(index, id)| {
                        (!state.visible || state.selected != Some(index)).then_some(*id)
                    })
                })
                .collect(),
        );
    }
    pub(super) fn begin_query_paint(&mut self) {
        for state in self.container_queries.values_mut() {
            state.painted = false;
        }
    }
    pub(super) fn finish_query_paint(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;
        for state in self.container_queries.values_mut() {
            if state.visible && !state.painted {
                state.visible = false;
                self.focus.borrow_mut().select_query(&state.children, None);
                changed = true;
            }
        }
        if changed {
            self.query_visibility_changed(window, cx);
        }
    }
    fn query_visibility_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_tooltips(window, cx);
        self.sync_extensions(&[], window, cx);
        self.sync_canvases(&[], window, cx);
        self.sync_splits(window, cx);
        self.sync_platform_menus(window, cx);
        self.suspend_hidden_animations();
        self.suspend_hidden_programs();
        // Finish after current paint has recorded the new eligible controls.
        if let Some(fallback) = self.root_focus.clone() {
            let focus = self.focus.clone();
            window.defer(cx, move |window, cx| {
                focus.borrow_mut().finish_frame(&fallback, window, cx)
            });
        }
    }
    fn select_container(
        &mut self,
        id: NodeId,
        generation: i64,
        branch: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<NodeId> {
        if !self.focus.borrow().visible(id) {
            return None;
        }
        let state = self.container_queries.get_mut(&id)?;
        if state.config.generation != generation {
            return None;
        }
        let child = *state.children.get(branch)?;
        let changed = !state.visible || state.selected != Some(branch);
        state.visible = true;
        state.selected = Some(branch);
        if changed {
            self.focus
                .borrow_mut()
                .select_query(&state.children, Some(child));
            self.query_visibility_changed(window, cx);
        }
        Some(child)
    }
    fn paint_container_selection(
        &mut self,
        id: NodeId,
        generation: i64,
        branch: usize,
        size: gpui::Size<gpui::Pixels>,
    ) {
        if !self.focus.borrow().visible(id) {
            return;
        }
        let Some(state) = self.container_queries.get_mut(&id) else {
            return;
        };
        if state.config.generation != generation || state.selected != Some(branch) || !state.visible
        {
            return;
        }
        state.painted = true;
        if state.observed == Some((generation, branch)) {
            return;
        }
        let Some(sequence) = state.sequence.checked_add(1) else {
            if self.session.borrow_mut().overload(self.id) {
                self.transport.fault(self.id);
            }
            return;
        };
        state.sequence = sequence;
        state.observed = Some((generation, branch));
        let snapshot = Snapshot {
            generation,
            sequence,
            branch: branch as i64,
            width: f32::from(size.width) as f64,
            height: f32::from(size.height) as f64,
        };
        let event = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return;
            };
            let Some(handler) = tree.get(id).and_then(|n| n.handler) else {
                return;
            };
            session.container_selected(self.id, id, handler, tree.revision(), snapshot)
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.id)
        {
            self.transport.fault(self.id);
        }
    }
    pub(super) fn container_query_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        // Preserve every supplied branch's keyed native state through late layout.
        let mut pending = vec![node.id];
        while let Some(id) = pending.pop() {
            self.visited.insert(id);
            if let Some(node) = tree.get(id) {
                pending.extend(node.children.iter().copied());
            }
        }
        let config = node.container_query.clone().expect("validated query");
        let id = node.id;
        let weak = cx.entity().downgrade();
        let shared = self.session.clone();
        let window_id = self.id;
        gpui::container_query(move |size, window, cx| {
            let empty = || div().into_any_element();
            let Some(branch) =
                config.select(f32::from(size.width) as f64, f32::from(size.height) as f64)
            else {
                return empty();
            };
            let painted = weak.clone();
            weak.update(cx, |view, cx| {
                let Some(child) = view.select_container(id, config.generation, branch, window, cx)
                else {
                    return empty();
                };
                let session = shared.borrow();
                let Some(tree) = session.tree(window_id) else {
                    return empty();
                };
                let child = view.element(tree, child, interaction, window, cx);
                div()
                    .size_full()
                    .child(child)
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |_, _, _, cx| {
                                let _ = painted.update(cx, |view, _| {
                                    view.paint_container_selection(
                                        id,
                                        config.generation,
                                        branch,
                                        size,
                                    )
                                });
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .into_any_element()
            })
            .unwrap_or_else(|_| empty())
        })
        .into_any_element()
    }
}
