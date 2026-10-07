//! Frame-bounded accessibility replay shared by cached views and deferred draws.
use super::*;
use std::{mem, ops::Range};

pub(crate) type Listeners = FxHashMap<NodeId, Vec<(Action, Option<A11yActionListener>)>>;
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Context {
    active: bool,
    disabled: bool,
    hidden: bool,
    focus: Option<FocusId>,
}
#[derive(Clone, Copy, Default)]
pub(crate) struct PaintIndex {
    actions: usize,
    selections: usize,
}
#[derive(Default)]
pub(crate) struct Cache {
    capture_current: bool,
    current_nodes: Vec<(NodeId, accesskit::Node)>,
    nodes: Vec<(NodeId, accesskit::Node)>,
    focus: FxHashMap<NodeId, FocusId>,
    bounds: FxHashMap<NodeId, Bounds<Pixels>>,
    listeners: Listeners,
    action_order: Vec<(NodeId, usize)>,
    previous_action_order: Vec<(NodeId, usize)>,
    selections: Vec<(NodeId, Option<accesskit::TextSelection>)>,
    previous_selections: Vec<(NodeId, Option<accesskit::TextSelection>)>,
    active_descendant: Option<(NodeId, FocusId)>,
    #[cfg(debug_assertions)]
    provenance: FxHashMap<NodeId, debug::NodeDebugInfo>,
}
impl A11y {
    /// All cached and deferred paint ranges have been consumed. Callbacks that
    /// were not moved into this frame belong to retired content, and must not
    /// keep its owners alive until another frame happens to be drawn.
    pub(crate) fn finish_cache_replay(&mut self) {
        self.cache.listeners.clear();
    }

    /// Advance even while inactive, so old callbacks and node snapshots cannot
    /// accumulate or be mistaken for the immediately preceding frame.
    pub(crate) fn advance_cache(&mut self) {
        self.cache.active_descendant = self.explicit_active_descendant.or_else(|| {
            Some((
                self.nodes.active_descendant?,
                *self.focus_ids.get(&self.nodes.focus?)?,
            ))
        });
        let captured = self.cache.capture_current;
        self.cache.nodes = mem::take(&mut self.cache.current_nodes);
        self.cache.capture_current = false;
        self.cache.focus = mem::take(&mut self.focus_ids);
        self.cache.bounds = mem::take(&mut self.node_bounds);
        self.cache.listeners = mem::take(&mut self.action_listeners);
        self.cache.previous_action_order = mem::take(&mut self.cache.action_order);
        self.cache.previous_selections = mem::take(&mut self.cache.selections);
        #[cfg(debug_assertions)]
        {
            self.cache.provenance = mem::take(&mut self.nodes.node_info);
        }
        if !captured {
            // Ordinary frames must release retired callback owners immediately,
            // just as before caching gained accessibility replay.
            self.cache.focus.clear();
            self.cache.bounds.clear();
            self.cache.listeners.clear();
            self.cache.previous_action_order.clear();
            self.cache.previous_selections.clear();
            self.cache.active_descendant = None;
            #[cfg(debug_assertions)]
            self.cache.provenance.clear();
        }
    }
    pub(crate) fn capture_cache_nodes(&mut self) {
        if self.cache.capture_current {
            // Capture raw nodes before output-only inheritance/repair. One flat
            // snapshot per frame, never a snapshot per nested cached view.
            self.cache.current_nodes = self.nodes.all_nodes.clone();
        }
    }
    pub(crate) fn cache_context(&mut self, focus: Option<FocusId>) -> Context {
        self.cache.capture_current |= self.is_active();
        Context {
            active: self.is_active(),
            disabled: self.nodes.disabled_scope
                || self.nodes.nodes_stack.iter().any(|node| node.is_disabled()),
            hidden: self.nodes.nodes_stack.iter().any(|node| node.is_hidden()),
            focus,
        }
    }
    pub(crate) fn prepaint_index(&self) -> usize {
        if self.is_active() {
            self.nodes.all_nodes.len()
        } else {
            0
        }
    }
    pub(crate) fn paint_index(&self) -> PaintIndex {
        PaintIndex {
            actions: self.cache.action_order.len(),
            selections: self.cache.selections.len(),
        }
    }
    pub(crate) fn register_action(
        &mut self,
        node: NodeId,
        action: Action,
        listener: A11yActionListener,
    ) {
        let listeners = self.action_listeners.entry(node).or_default();
        self.cache.action_order.push((node, listeners.len()));
        listeners.push((action, Some(listener)));
    }
    pub(crate) fn publish_selection(
        &mut self,
        node: NodeId,
        selection: Option<accesskit::TextSelection>,
    ) {
        self.cache.selections.push((node, selection));
        self.document_selections.insert(node, selection);
    }
    pub(crate) fn reuse_prepaint(&mut self, range: Range<usize>, focused: Option<FocusId>) {
        if !self.is_active() {
            return;
        }
        // Nodes are postorder. Only roots of this closed range attach to the
        // current open parent; internal child links remain unchanged.
        let nodes = self.cache.nodes[range].to_vec();
        let active_descendant = self
            .cache
            .active_descendant
            .filter(|(id, _)| nodes.iter().any(|(node, _)| node == id));
        let children: FxHashSet<_> = nodes
            .iter()
            .flat_map(|(_, node)| node.children().iter().copied())
            .collect();
        for (id, node) in nodes {
            if !self.nodes.can_push(id) {
                continue;
            }
            if !children.contains(&id) {
                self.nodes
                    .current_node_mut()
                    .expect("cache has a parent")
                    .push_child(id);
            }
            self.nodes.all_nodes.push((id, node));
            if let Some(bounds) = self.cache.bounds.get(&id).copied() {
                self.set_bounds(id, bounds);
            }
            if let Some(owner) = self.cache.focus.get(&id).copied() {
                self.set_focusable(id, owner);
                if Some(owner) == focused && self.nodes.focus.is_none() {
                    self.set_focus(id);
                }
            }
            #[cfg(debug_assertions)]
            if let Some(info) = self.cache.provenance.get(&id).cloned() {
                self.nodes.record_node_info(id, info);
            }
        }
        if let Some((node, owner)) = active_descendant
            && Some(owner) == focused
            && self.nodes.has_node(node)
        {
            self.set_active_descendant_for(node, owner);
        }
    }
    pub(crate) fn reuse_paint(&mut self, range: Range<PaintIndex>) {
        if !self.is_active() {
            return;
        }
        for index in range.start.actions..range.end.actions {
            let (node, listener_index) = self.cache.previous_action_order[index];
            if let Some((action, listener)) = self
                .cache
                .listeners
                .get_mut(&node)
                .and_then(|items| items.get_mut(listener_index))
                && let Some(listener) = listener.take()
            {
                let action = *action;
                self.register_action(node, action, listener);
            }
        }
        for index in range.start.selections..range.end.selections {
            let (node, selection) = self.cache.previous_selections[index];
            self.publish_selection(node, selection);
        }
    }
}
