//! Window-owned focus scopes. Tab order is observed from painted controls;
//! eligibility and modal gating use the current retained tree, never a probe focus.
use super::SharedSession;
use gpui::{App, Bounds, FocusHandle, Pixels, WeakFocusHandle, Window};
use gpuio_protocol::{NodeId, WindowId, v1::*};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

fn style_hidden(node: &crate::tree::Node) -> bool {
    crate::style::inert(&node.style)
        || node.style.iter().any(|style| {
            matches!(style,
        Style::Fields(fields) if fields.iter().any(|field|
            matches!(field, Field::Display(3) | Field::Visibility(1))))
        })
}

fn navigation_hidden(tree: &crate::tree::Tree, node: &crate::tree::Node) -> bool {
    node.parent
        .and_then(|id| tree.get(id))
        .is_some_and(|parent| {
            parent.navigation_stack.is_some_and(|config| {
                config
                    .selected
                    .and_then(|index| parent.children.get(index as usize))
                    != Some(&node.id)
            })
        })
}

#[derive(Default)]
struct Navigation {
    selected: Option<NodeId>,
    remembered: BTreeMap<NodeId, WeakFocusHandle>,
}

pub(super) type Shared = Rc<RefCell<Manager>>;
struct Scope {
    handle: FocusHandle,
    restore: Option<WeakFocusHandle>,
    config: FocusScopeConfig,
    order: u64,
    overlay: Option<OverlayKind>,
    anchor: Rc<Cell<Bounds<Pixels>>>,
}
struct Entry {
    node: NodeId,
    handle: FocusHandle,
    tab_stop: bool,
    disclosure_path: Vec<(NodeId, NodeId)>,
    navigation_path: Vec<(NodeId, NodeId)>,
}
pub(super) struct Manager {
    window: WindowId,
    session: SharedSession,
    scopes: BTreeMap<NodeId, Scope>,
    entries: Vec<Entry>,
    surfaces: BTreeMap<NodeId, Vec<Rc<Cell<Bounds<Pixels>>>>>,
    seen: BTreeSet<(NodeId, u16)>,
    active: Option<NodeId>,
    hidden: BTreeSet<NodeId>,
    query_hidden: BTreeSet<NodeId>,
    last_editor: Option<NodeId>,
    order: u64,
    enter: Option<NodeId>,
    pending: bool,
    navigation: BTreeMap<NodeId, Navigation>,
    navigation_enter: Vec<(NodeId, Option<WeakFocusHandle>)>,
}
impl Manager {
    pub(super) fn new(window: WindowId, session: SharedSession) -> Shared {
        Rc::new(RefCell::new(Self {
            window,
            session,
            scopes: BTreeMap::new(),
            entries: Vec::new(),
            surfaces: BTreeMap::new(),
            seen: BTreeSet::new(),
            active: None,
            hidden: BTreeSet::new(),
            query_hidden: BTreeSet::new(),
            last_editor: None,
            order: 0,
            enter: None,
            pending: false,
            navigation: BTreeMap::new(),
            navigation_enter: Vec::new(),
        }))
    }
    fn within(&self, node: NodeId, scope: NodeId) -> bool {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.window) else {
            return false;
        };
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            if id == scope {
                return tree.get(id).is_some();
            }
            cursor = tree.get(id).and_then(|node| node.parent);
        }
        false
    }
    pub(super) fn hidden(&self, node: NodeId) -> bool {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.window) else {
            return true;
        };
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            if self.hidden.contains(&id) || self.query_hidden.contains(&id) {
                return true;
            }
            cursor = tree.get(id).and_then(|node| node.parent);
        }
        false
    }
    pub(super) fn set_query_hidden(&mut self, hidden: BTreeSet<NodeId>) {
        if self.query_hidden != hidden {
            self.query_hidden = hidden;
            self.pending = true;
        }
    }
    pub(super) fn select_query(&mut self, children: &[NodeId], selected: Option<NodeId>) {
        for child in children {
            if Some(*child) == selected {
                self.query_hidden.remove(child);
            } else {
                self.query_hidden.insert(*child);
            }
        }
        self.pending = true;
    }
    pub(super) fn set_hidden(&mut self, hidden: BTreeSet<NodeId>) {
        if self.hidden != hidden {
            self.hidden = hidden;
            self.pending = true;
        }
    }
    pub(super) fn allows(&self, node: NodeId) -> bool {
        self.visible(node) && self.active.is_none_or(|scope| self.within(node, scope))
    }
    pub(super) fn allows_without(&self, excluded: NodeId, node: NodeId) -> bool {
        self.visible(node)
            && self
                .scopes
                .iter()
                .filter(|(id, scope)| **id != excluded && scope.config.trap)
                .max_by_key(|(_, scope)| scope.order)
                .is_none_or(|(scope, _)| self.within(node, *scope))
    }
    pub(super) fn blocks_pointer(&self, node: NodeId) -> bool {
        !self.visible(node)
            || self
                .active
                .is_some_and(|scope| !self.within(node, scope) && !self.within(scope, node))
    }
    fn eligible(&self, node: NodeId) -> bool {
        if !self.allows(node) {
            return false;
        }
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.window) else {
            return false;
        };
        let Some(item) = tree.get(node) else {
            return false;
        };
        if item.canvas.as_ref().is_some_and(|config| config.disabled)
            || item
                .extension
                .as_ref()
                .is_some_and(|config| config.disabled)
            || item.control.is_some_and(Control::disabled)
            || item.editor.as_ref().is_some_and(|config| config.disabled)
            || item
                .number_input
                .as_ref()
                .is_some_and(|n| n.config.disabled)
            || item.otp_input.as_ref().is_some_and(|n| n.config.disabled)
            || item.calendar.as_ref().is_some_and(|n| n.config.disabled)
            || item.color_input.as_ref().is_some_and(|n| n.config.disabled)
            || item.choice.as_ref().is_some_and(|config| config.disabled)
            || item.rating.as_ref().is_some_and(|config| config.disabled)
            || item
                .slider
                .as_ref()
                .is_some_and(|slider| slider.config.disabled)
            || item
                .menu
                .as_ref()
                .is_some_and(|config| config.menus.iter().all(|menu| menu.disabled))
        {
            return false;
        }
        self.visible(node)
    }
    pub(super) fn visible(&self, node: NodeId) -> bool {
        if self.hidden(node) {
            return false;
        }
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.window) else {
            return false;
        };
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let Some(item) = tree.get(id) else {
                return false;
            };
            if style_hidden(item) || navigation_hidden(tree, item) {
                return false;
            }
            cursor = item.parent;
        }
        true
    }
    pub(super) fn handle(&self, node: NodeId) -> Option<FocusHandle> {
        self.scopes.get(&node).map(|scope| scope.handle.clone())
    }
    pub(super) fn clear_surfaces(&mut self) {
        self.surfaces.clear();
    }
    pub(super) fn surface(&mut self, node: NodeId, bounds: Rc<Cell<Bounds<Pixels>>>) {
        self.surfaces.entry(node).or_default().push(bounds);
    }
    pub(super) fn surface_contains(&self, scope: NodeId, position: gpui::Point<Pixels>) -> bool {
        self.surfaces.iter().any(|(node, bounds)| {
            self.within(*node, scope)
                && bounds.iter().any(|bounds| bounds.get().contains(&position))
        })
    }
    pub(super) fn anchor(&self, node: NodeId) -> Option<Rc<Cell<Bounds<Pixels>>>> {
        self.scopes.get(&node).map(|scope| scope.anchor.clone())
    }
    pub(super) fn layer(&self, node: NodeId) -> usize {
        self.scopes
            .iter()
            .filter(|(id, _)| self.within(node, **id))
            // Reserve room for the bounded eight-level menu cascade beneath the next overlay.
            .map(|(_, scope)| scope.order as usize * 16)
            .max()
            .unwrap_or(0)
    }
    pub(super) fn top_overlay(&self, node: NodeId) -> bool {
        !self.blocks_pointer(node)
            && self
                .scopes
                .iter()
                .filter(|(_, scope)| scope.overlay.is_some())
                .max_by_key(|(_, scope)| scope.order)
                .is_some_and(|(id, _)| *id == node)
    }
    pub(super) fn contains_focus(&self, window: &Window) -> bool {
        self.scopes
            .values()
            .any(|scope| scope.handle.is_focused(window))
    }
    pub(super) fn sync(&mut self, window: &mut Window, cx: &mut App) {
        self.sync_navigation_focus(window);
        // Keep the previous painted ancestry: removed editor nodes are already
        // absent from the new tree. Select the first still-eligible outer trigger
        // only when its content region or previously focused child became ineligible.
        let previous_focused = self
            .entries
            .iter()
            .find(|entry| entry.handle.is_focused(window))
            .map(|entry| entry.node);
        let disclosure_restore = self
            .entries
            .iter()
            .find(|entry| entry.handle.is_focused(window))
            .map(|entry| entry.disclosure_path.clone())
            .unwrap_or_default();
        let had_scopes = !self.scopes.is_empty();
        let configs = {
            let session = self.session.borrow();
            let mut result = Vec::new();
            if let Some(tree) = session.tree(self.window) {
                let mut stack = tree.root().into_iter().collect::<Vec<_>>();
                while let Some(id) = stack.pop() {
                    let node = tree.get(id).expect("validated node");
                    // Traversal already skipped hidden ancestors; inspect this
                    // node once rather than rewalking its ancestry for every node.
                    if self.hidden.contains(&id)
                        || self.query_hidden.contains(&id)
                        || style_hidden(node)
                        || navigation_hidden(tree, node)
                    {
                        continue;
                    }
                    let config = node
                        .focus_scope
                        .or_else(|| {
                            node.palette.as_ref().map(|_| FocusScopeConfig {
                                trap: true,
                                auto_focus: true,
                                restore_focus: true,
                            })
                        })
                        .or_else(|| {
                            node.tooltip.as_ref().map(|_| FocusScopeConfig {
                                trap: false,
                                auto_focus: false,
                                restore_focus: false,
                            })
                        });
                    let config = config.or_else(|| {
                        matches!(node.kind, Kind::Toast | Kind::ToastStack).then_some(
                            FocusScopeConfig {
                                trap: false,
                                auto_focus: false,
                                restore_focus: node.kind == Kind::Toast,
                            },
                        )
                    });
                    if let Some(config) = config {
                        result.push((
                            id,
                            config,
                            node.overlay
                                .as_ref()
                                .map(|config| config.kind)
                                .or_else(|| node.palette.as_ref().map(|_| OverlayKind::Dialog)),
                        ));
                    }
                    stack.extend(node.children.iter().rev().copied());
                }
            }
            result
        };
        let present = configs
            .iter()
            .map(|(id, _, _)| *id)
            .collect::<BTreeSet<_>>();
        let mut restore = None;
        let mut removed = self
            .scopes
            .iter()
            .filter(|(id, _)| !present.contains(id))
            .map(|(id, scope)| (*id, scope.order))
            .collect::<Vec<_>>();
        removed.sort_by_key(|(_, order)| *order);
        for (id, _) in removed.into_iter().rev() {
            let scope = self.scopes.remove(&id).expect("known scope");
            if scope.config.restore_focus && scope.handle.contains_focused(window, cx) {
                restore = scope.restore.and_then(|handle| handle.upgrade());
            }
        }
        let mut enter = None;
        for (id, config, overlay) in configs {
            if let Some(scope) = self.scopes.get_mut(&id) {
                if !scope.config.trap && config.trap {
                    enter = Some(id);
                }
                scope.config = config;
                scope.overlay = overlay;
            } else {
                self.order += 1;
                self.scopes.insert(
                    id,
                    Scope {
                        handle: cx.focus_handle(),
                        restore: window.focused(cx).map(|handle| handle.downgrade()),
                        config,
                        order: self.order,
                        overlay,
                        anchor: Rc::new(Cell::new(Bounds::default())),
                    },
                );
                if config.trap || config.auto_focus {
                    enter = Some(id);
                }
            }
        }
        self.active = self
            .scopes
            .iter()
            .filter(|(_, scope)| scope.config.trap)
            .max_by_key(|(_, scope)| scope.order)
            .map(|(id, _)| *id);
        let mut scope_restored = false;
        if let Some(ref handle) = restore
            && self
                .entries
                .iter()
                .any(|entry| &entry.handle == handle && self.eligible(entry.node))
        {
            window.focus(handle, cx);
            scope_restored = true;
        }
        if !scope_restored
            && let Some(handle) = disclosure_restore.iter().find_map(|(panel, trigger)| {
                if (self.visible(*panel)
                    && previous_focused.is_some_and(|node| self.eligible(node)))
                    || !self.eligible(*trigger)
                {
                    return None;
                }
                self.entries
                    .iter()
                    .find(|entry| entry.node == *trigger)
                    .map(|entry| entry.handle.clone())
            })
        {
            window.focus(&handle, cx);
        }
        if let Some(id) = enter.filter(|id| self.allows(*id)) {
            self.enter = Some(id);
            window.focus(&self.scopes[&id].handle, cx);
        }
        // A retained panel can become hidden without removing its editor or
        // introducing a focus scope. Release its focus immediately, then use
        // the normal post-layout fallback. Hidden views retain editing state.
        let hidden_focus = self
            .entries
            .iter()
            .any(|entry| entry.handle.is_focused(window) && !self.eligible(entry.node));
        if hidden_focus {
            window.blur(cx);
        }
        self.pending = had_scopes
            || !self.scopes.is_empty()
            || hidden_focus
            || !self.navigation_enter.is_empty();
    }

    fn sync_navigation_focus(&mut self, window: &Window) {
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.handle.is_focused(window))
        {
            for (owner, page) in &entry.navigation_path {
                if let Some(state) = self.navigation.get_mut(owner) {
                    state.remembered.insert(*page, entry.handle.downgrade());
                }
            }
        }
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.window) else {
            self.navigation.clear();
            self.navigation_enter.clear();
            return;
        };
        self.navigation.retain(|id, _| {
            tree.get(*id)
                .is_some_and(|node| node.navigation_stack.is_some())
        });
        self.navigation_enter
            .retain(|(page, _)| tree.get(*page).is_some());
        let mut stack = tree.root().into_iter().collect::<Vec<_>>();
        while let Some(id) = stack.pop() {
            let node = tree.get(id).expect("admitted tree");
            if let Some(config) = node.navigation_stack {
                let selected = config.selected.map(|index| node.children[index as usize]);
                let visible = self.visible(id);
                let state = self.navigation.entry(id).or_default();
                state
                    .remembered
                    .retain(|page, _| node.children.contains(page));
                if state.selected != selected {
                    // Supersede pending focus for older selections of this presenter.
                    self.navigation_enter
                        .retain(|(page, _)| !node.children.contains(page));
                    if visible && let Some(page) = selected {
                        self.navigation_enter
                            .push((page, state.remembered.get(&page).cloned()));
                    }
                    state.selected = selected;
                }
            }
            stack.extend(node.children.iter().rev().copied());
        }
    }
    pub(super) fn begin_frame(&mut self) {
        self.entries.clear();
        self.seen.clear();
    }
    pub(super) fn focused_node(&self, window: &Window) -> Option<NodeId> {
        self.entries
            .iter()
            .find(|entry| entry.handle.is_focused(window))
            .map(|entry| entry.node)
    }
    pub(super) fn can_restore(&self, handle: &FocusHandle) -> bool {
        self.entries
            .iter()
            .any(|entry| &entry.handle == handle && self.eligible(entry.node))
    }
    pub(super) fn remember_editor(&mut self, node: NodeId) {
        if self.eligible(node) {
            self.last_editor = Some(node);
        }
    }
    pub(super) fn last_editor(&self) -> Option<NodeId> {
        self.last_editor.filter(|node| self.eligible(*node))
    }
    pub(super) fn record(
        &mut self,
        node: NodeId,
        handle: FocusHandle,
        tab_stop: bool,
        focused: bool,
    ) {
        self.record_part(node, 0, handle, tab_stop, focused);
    }
    /// Distinct native controls can share one retained owner (e.g. range thumbs).
    /// Part identity only deduplicates paint; all eligibility stays owner-scoped.
    pub(super) fn record_part(
        &mut self,
        node: NodeId,
        part: u16,
        handle: FocusHandle,
        tab_stop: bool,
        focused: bool,
    ) {
        if focused
            && self
                .session
                .borrow()
                .tree(self.window)
                .and_then(|tree| tree.get(node))
                .is_some_and(|node| {
                    node.editor.is_some() || node.number_input.is_some() || node.otp_input.is_some()
                })
        {
            self.last_editor = Some(node);
        }
        if self.seen.insert((node, part)) {
            let (disclosure_path, navigation_path) = {
                let session = self.session.borrow();
                let mut path = Vec::new();
                let mut navigation_path = Vec::new();
                if let Some(tree) = session.tree(self.window) {
                    let mut child = node;
                    while let Some(parent) = tree
                        .get(child)
                        .and_then(|node| node.parent)
                        .and_then(|id| tree.get(id))
                    {
                        if parent.kind == Kind::Disclosure
                            && parent.children.get(1) == Some(&child)
                            && let Some(trigger) = tree.disclosure_trigger(parent.id)
                        {
                            path.push((child, trigger));
                        }
                        if parent.navigation_stack.is_some() {
                            navigation_path.push((parent.id, child));
                        }
                        child = parent.id;
                    }
                }
                (path, navigation_path)
            };
            self.entries.push(Entry {
                node,
                handle,
                tab_stop,
                disclosure_path,
                navigation_path,
            });
        }
    }
    pub(super) fn disclosure_key(
        &self,
        node: NodeId,
        key: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if !matches!(key, "up" | "down" | "home" | "end") || !self.eligible(node) {
            return false;
        }
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.window) else {
            return false;
        };
        let Some(disclosure) = tree.disclosure_for_trigger(node) else {
            return false;
        };
        let Some(accordion) = disclosure
            .parent
            .and_then(|id| tree.get(id))
            .filter(|node| node.kind == Kind::Accordion)
        else {
            return false;
        };
        let handles = self
            .entries
            .iter()
            .map(|entry| (entry.node, &entry.handle))
            .collect::<BTreeMap<_, _>>();
        let targets = accordion
            .children
            .iter()
            .filter_map(|id| {
                let trigger = tree.disclosure_trigger(*id)?;
                if !self.eligible(trigger) {
                    return None;
                }
                handles.get(&trigger).map(|handle| (trigger, *handle))
            })
            .collect::<Vec<_>>();
        let Some(index) = targets.iter().position(|(id, _)| *id == node) else {
            return false;
        };
        let target = match key {
            "home" => 0,
            "end" => targets.len() - 1,
            "up" => (index + targets.len() - 1) % targets.len(),
            "down" => (index + 1) % targets.len(),
            _ => unreachable!(),
        };
        window.focus(targets[target].1, cx);
        true
    }

    pub(super) fn take_pending(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }
    pub(super) fn finish_frame(
        &mut self,
        fallback: &FocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(scope) = self
            .enter
            .take()
            .filter(|id| self.scopes.contains_key(id) && self.allows(*id))
        {
            if self.entries.iter().any(|entry| {
                entry.tab_stop
                    && entry.handle.is_focused(window)
                    && self.eligible(entry.node)
                    && self.within(entry.node, scope)
            }) {
                return;
            }
            let target = self
                .entries
                .iter()
                .find(|entry| {
                    entry.tab_stop && self.eligible(entry.node) && self.within(entry.node, scope)
                })
                .map(|entry| entry.handle.clone())
                .unwrap_or_else(|| self.scopes[&scope].handle.clone());
            window.focus(&target, cx);
        } else if self.finish_navigation(window, cx) {
            // Destination focus was restored after its controls painted.
        } else if !self
            .entries
            .iter()
            .any(|entry| entry.handle.is_focused(window) && self.eligible(entry.node))
        {
            let target = self
                .active
                .and_then(|id| self.handle(id))
                .unwrap_or_else(|| fallback.clone());
            window.focus(&target, cx);
        }
    }
    pub(super) fn finish_navigation(&mut self, window: &mut Window, cx: &mut App) -> bool {
        if self.enter.is_some() {
            return false;
        }
        let mut focused = false;
        for (page, remembered) in std::mem::take(&mut self.navigation_enter).into_iter().rev() {
            if !self.allows(page) {
                continue;
            }
            if focused {
                continue;
            }
            // Respect a user/native focus choice made while the destination enters.
            if self.entries.iter().any(|entry| {
                entry.handle.is_focused(window)
                    && self.eligible(entry.node)
                    && self.within(entry.node, page)
            }) {
                focused = true;
                continue;
            }
            let target = remembered
                .as_ref()
                .and_then(|handle| handle.upgrade())
                .filter(|handle| {
                    self.entries.iter().any(|entry| {
                        &entry.handle == handle
                            && self.eligible(entry.node)
                            && self.within(entry.node, page)
                    })
                })
                .or_else(|| {
                    self.entries
                        .iter()
                        .find(|entry| {
                            entry.tab_stop
                                && self.eligible(entry.node)
                                && self.within(entry.node, page)
                        })
                        .map(|entry| entry.handle.clone())
                });
            if let Some(target) = target {
                window.focus(&target, cx);
                focused = true;
            } else {
                // At offset 1 the incoming controls can be fully clipped on the
                // first paint. Retry on a subsequent actual paint, never poll.
                self.navigation_enter.push((page, remembered));
            }
        }
        focused
    }
    pub(super) fn navigation_pending(&self) -> bool {
        !self.navigation_enter.is_empty()
    }
    pub(super) fn traverse(&self, reverse: bool, window: &mut Window, cx: &mut App) {
        let Some(scope) = self.active else {
            if reverse {
                window.focus_prev(cx);
            } else {
                window.focus_next(cx);
            }
            return;
        };
        let entries = self
            .entries
            .iter()
            .filter(|entry| entry.tab_stop && self.eligible(entry.node))
            .collect::<Vec<_>>();
        if entries.is_empty() {
            window.focus(&self.scopes[&scope].handle, cx);
            return;
        }
        let current = entries
            .iter()
            .position(|entry| entry.handle.is_focused(window));
        let next = if reverse {
            current.map_or(entries.len() - 1, |i| {
                (i + entries.len() - 1) % entries.len()
            })
        } else {
            current.map_or(0, |i| (i + 1) % entries.len())
        };
        window.focus(&entries[next].handle, cx);
    }
}
