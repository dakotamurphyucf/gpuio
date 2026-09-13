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
}
pub(super) struct Manager {
    window: WindowId,
    session: SharedSession,
    scopes: BTreeMap<NodeId, Scope>,
    entries: Vec<Entry>,
    surfaces: BTreeMap<NodeId, Rc<Cell<Bounds<Pixels>>>>,
    seen: BTreeSet<NodeId>,
    active: Option<NodeId>,
    hidden: BTreeSet<NodeId>,
    order: u64,
    enter: Option<NodeId>,
    pending: bool,
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
            order: 0,
            enter: None,
            pending: false,
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
            if self.hidden.contains(&id) {
                return true;
            }
            cursor = tree.get(id).and_then(|node| node.parent);
        }
        false
    }
    pub(super) fn set_hidden(&mut self, hidden: BTreeSet<NodeId>) {
        if self.hidden != hidden {
            self.hidden = hidden;
            self.pending = true;
        }
    }
    pub(super) fn allows(&self, node: NodeId) -> bool {
        !self.hidden(node) && self.active.is_none_or(|scope| self.within(node, scope))
    }
    pub(super) fn blocks_pointer(&self, node: NodeId) -> bool {
        self.hidden(node)
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
        if item.control.is_some_and(Control::disabled)
            || item.editor.as_ref().is_some_and(|config| config.disabled)
            || item.choice.as_ref().is_some_and(|config| config.disabled)
        {
            return false;
        }
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let Some(item) = tree.get(id) else {
                return false;
            };
            for style in item.style.iter() {
                if let Style::Fields(fields) = style
                    && fields
                        .iter()
                        .any(|field| matches!(field, Field::Display(3) | Field::Visibility(1)))
                {
                    return false;
                }
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
        self.surfaces.insert(node, bounds);
    }
    pub(super) fn surface_contains(&self, scope: NodeId, position: gpui::Point<Pixels>) -> bool {
        self.surfaces
            .iter()
            .any(|(node, bounds)| self.within(*node, scope) && bounds.get().contains(&position))
    }
    pub(super) fn anchor(&self, node: NodeId) -> Option<Rc<Cell<Bounds<Pixels>>>> {
        self.scopes.get(&node).map(|scope| scope.anchor.clone())
    }
    pub(super) fn layer(&self, node: NodeId) -> usize {
        self.scopes
            .iter()
            .filter(|(id, _)| self.within(node, **id))
            .map(|(_, scope)| scope.order as usize * 4)
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
        let had_scopes = !self.scopes.is_empty();
        let configs = {
            let session = self.session.borrow();
            let mut result = Vec::new();
            if let Some(tree) = session.tree(self.window) {
                let mut stack = tree.root().into_iter().collect::<Vec<_>>();
                while let Some(id) = stack.pop() {
                    if self.hidden.contains(&id) {
                        continue;
                    }
                    let node = tree.get(id).expect("validated node");
                    let config = node.focus_scope.or_else(|| {
                        node.tooltip.as_ref().map(|_| FocusScopeConfig {
                            trap: false,
                            auto_focus: false,
                            restore_focus: false,
                        })
                    });
                    if let Some(config) = config {
                        result.push((id, config, node.overlay.as_ref().map(|config| config.kind)));
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
        if let Some(handle) = restore
            && self
                .entries
                .iter()
                .any(|entry| entry.handle == handle && self.eligible(entry.node))
        {
            window.focus(&handle, cx);
        }
        if let Some(id) = enter.filter(|id| self.allows(*id)) {
            self.enter = Some(id);
            window.focus(&self.scopes[&id].handle, cx);
        }
        self.pending = had_scopes || !self.scopes.is_empty();
    }
    pub(super) fn begin_frame(&mut self) {
        self.entries.clear();
        self.seen.clear();
    }
    pub(super) fn record(&mut self, node: NodeId, handle: FocusHandle, tab_stop: bool) {
        if self.seen.insert(node) {
            self.entries.push(Entry {
                node,
                handle,
                tab_stop,
            });
        }
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
