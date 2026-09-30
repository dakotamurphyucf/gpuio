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

// Resolve base fields in declaration order, as native styling does. Inertness
// controls input rather than whether text exists in a search projection.
fn search_style_hidden(styles: &[Style]) -> bool {
    let (mut display_none, mut visibility_hidden) = (false, false);
    for style in styles {
        if let Style::Fields(fields) = style {
            for field in fields {
                match field {
                    Field::Display(value) => display_none = *value == 3,
                    Field::Visibility(value) => visibility_hidden = *value == 1,
                    _ => (),
                }
            }
        }
    }
    display_none || visibility_hidden
}

#[cfg(test)]
mod search_visibility_tests {
    use super::*;

    #[test]
    fn base_visibility_resolves_overrides_and_keeps_inert_text() {
        let styles = [
            Style::Fields(vec![Field::Display(3), Field::Visibility(1)]),
            Style::Fields(vec![
                Field::Display(1),
                Field::Visibility(0),
                Field::Inert(true),
            ]),
        ];
        assert!(!search_style_hidden(&styles));
        assert!(search_style_hidden(&styles[..1]));
        assert!(search_style_hidden(&[Style::Fields(vec![Field::Display(
            3
        )])]));
        assert!(search_style_hidden(&[Style::Fields(vec![
            Field::Visibility(1)
        ])]));
    }

    #[test]
    fn visibility_identity_changes_only_with_actual_branch_selection() {
        let session = Rc::new(RefCell::new(crate::session::Session::default()));
        let manager = Manager::new(WindowId::from_parts(0, 1).unwrap(), session);
        let mut manager = manager.borrow_mut();
        let children = [
            NodeId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 1).unwrap(),
        ];
        let initial = manager.visibility_identity();
        manager.set_hidden(BTreeSet::new());
        manager.set_query_hidden(BTreeSet::new());
        assert!(Rc::ptr_eq(&initial, &manager.visibility_identity()));
        manager.select_query(&children, Some(children[0]));
        let selected = manager.visibility_identity();
        assert!(!Rc::ptr_eq(&initial, &selected));
        manager.select_query(&children, Some(children[0]));
        assert!(Rc::ptr_eq(&selected, &manager.visibility_identity()));
        manager.select_query(&children, Some(children[1]));
        assert!(!Rc::ptr_eq(&selected, &manager.visibility_identity()));
    }

    #[test]
    fn native_visibility_commits_final_sample_and_fences_style_identity() {
        let session = Rc::new(RefCell::new(crate::session::Session::default()));
        let window = WindowId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window, "visibility", 400., 200.)
            .unwrap();
        let parent = NodeId::from_parts(0, 1).unwrap();
        let child = NodeId::from_parts(1, 1).unwrap();
        let apply = |operations| {
            let base = session.borrow().tree(window).unwrap().revision();
            session
                .borrow_mut()
                .apply(&Transaction {
                    window,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
        };
        let styles = || {
            vec![
                Style::Fields(vec![Field::Visibility(0), Field::Inert(true)]),
                Style::State(2, vec![Field::Visibility(1)]),
            ]
        };
        apply(vec![
            Op::Create(parent, Kind::Container, "".into(), None),
            Op::SetStyle(parent, styles()),
            Op::Create(child, Kind::Text, "aaa".into(), None),
            Op::Splice(parent, 0, 0, vec![child]),
            Op::SetRoot(Some(parent)),
        ]);
        let manager = Manager::new(window, session.clone());
        let mut manager = manager.borrow_mut();
        let source = session
            .borrow()
            .tree(window)
            .unwrap()
            .get(parent)
            .unwrap()
            .style
            .clone();
        let initial = manager.visibility_identity();
        manager.highlight_style(parent, &source, true);
        manager.highlight_style(parent, &source, false);
        assert!(
            !manager.commit_highlight_styles(),
            "intermediate samples are not visibility changes"
        );
        assert!(Rc::ptr_eq(&initial, &manager.visibility_identity()));
        assert!(
            manager.highlight_visible(session.borrow().tree(window).unwrap(), child),
            "inertness is not search hiding"
        );
        manager.highlight_style(parent, &source, true);
        assert!(manager.commit_highlight_styles());
        assert!(!manager.highlight_visible(session.borrow().tree(window).unwrap(), child));
        assert!(
            !manager.commit_highlight_styles(),
            "unchanged frames never wake"
        );

        // Even value-equal replacement has a new admitted identity. Old frames
        // cannot reapply their hidden result after the new tree is committed.
        apply(vec![Op::SetStyle(parent, styles())]);
        manager.prune_highlight_styles();
        assert!(manager.highlight_styles.is_empty());
        manager.highlight_style(parent, &source, true);
        assert!(manager.highlight_styles.is_empty());
        assert!(manager.highlight_visible(session.borrow().tree(window).unwrap(), child));
        let source = session
            .borrow()
            .tree(window)
            .unwrap()
            .get(parent)
            .unwrap()
            .style
            .clone();
        manager.highlight_style(parent, &source, true);
        manager.commit_highlight_styles();
        apply(vec![
            Op::SetRoot(None),
            Op::Remove(child),
            Op::Remove(parent),
        ]);
        manager.prune_highlight_styles();
        assert!(
            manager.highlight_styles.is_empty(),
            "removed nodes retain no visibility records"
        );
    }

    #[test]
    fn panel_visibility_preserves_anchor_and_fences_replaced_children() {
        let session = Rc::new(RefCell::new(crate::session::Session::default()));
        let window = WindowId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window, "parts", 400., 200.)
            .unwrap();
        let id = |slot| NodeId::from_parts(slot, 1).unwrap();
        let apply = |operations| {
            let base = session.borrow().tree(window).unwrap().revision();
            session
                .borrow_mut()
                .apply(&Transaction {
                    window,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
        };
        apply(vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(id(0), vec![Style::State(2, vec![Field::Visibility(1)])]),
            Op::Create(id(1), Kind::Text, "anchor".into(), None),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::Create(id(3), Kind::Text, "content".into(), None),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
            Op::SetRoot(Some(id(0))),
        ]);
        let manager = Manager::new(window, session.clone());
        let mut manager = manager.borrow_mut();
        let source = session
            .borrow()
            .tree(window)
            .unwrap()
            .get(id(0))
            .unwrap()
            .style
            .clone();
        let visible = |manager: &Manager, node| {
            manager.highlight_visible(session.borrow().tree(window).unwrap(), node)
        };
        // A part can only name a direct child, never an arbitrary descendant.
        manager.highlight_part_style(id(0), Some((1, id(3))), &source, true);
        assert!(manager.highlight_styles.is_empty());
        manager.highlight_part_style(id(0), Some((1, id(2))), &source, true);
        assert!(manager.commit_highlight_styles());
        assert!(visible(&manager, id(0)));
        assert!(visible(&manager, id(1)));
        assert!(!visible(&manager, id(2)));
        assert!(!visible(&manager, id(3)));
        // Revealing the floating panel does not override its child's own style.
        apply(vec![Op::SetStyle(
            id(2),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )]);
        manager.highlight_part_style(id(0), Some((1, id(2))), &source, false);
        manager.commit_highlight_styles();
        assert!(!visible(&manager, id(3)));
        assert!(visible(&manager, id(1)));
        apply(vec![Op::SetStyle(id(2), vec![])]);
        assert!(visible(&manager, id(3)));
        manager.highlight_part_style(id(0), Some((1, id(2))), &source, true);
        manager.commit_highlight_styles();
        // Swapping child roles cannot apply an old panel sample to the anchor.
        apply(vec![Op::Splice(id(0), 0, 2, vec![id(2), id(1)])]);
        manager.prune_highlight_styles();
        assert!(manager.highlight_styles.is_empty());
        manager.highlight_part_style(id(0), Some((1, id(2))), &source, true);
        assert!(manager.highlight_styles.is_empty());
        assert!(visible(&manager, id(3)));
        apply(vec![Op::Splice(id(0), 0, 2, vec![id(1), id(2)])]);
        manager.highlight_part_style(id(0), Some((1, id(2))), &source, true);
        manager.commit_highlight_styles();
        // The owner style identity is unchanged, but the target generation is new.
        let replacement = NodeId::from_parts(2, 2).unwrap();
        apply(vec![
            Op::Splice(id(0), 1, 1, vec![]),
            Op::Remove(id(3)),
            Op::Remove(id(2)),
            Op::Create(replacement, Kind::Text, "new content".into(), None),
            Op::Splice(id(0), 1, 0, vec![replacement]),
        ]);
        manager.prune_highlight_styles();
        assert!(manager.highlight_styles.is_empty());
        manager.highlight_part_style(id(0), Some((1, id(2))), &source, true);
        assert!(manager.highlight_styles.is_empty());
        assert!(visible(&manager, replacement));
        manager.highlight_part_style(id(0), Some((1, replacement)), &source, true);
        assert!(manager.commit_highlight_styles());
        assert!(!visible(&manager, replacement));
        assert!(visible(&manager, id(1)));
    }
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
struct HighlightStyle {
    source: std::sync::Weak<[Style]>,
    /// None observes the owner; Some gates a child at its captured position.
    part: Option<(usize, NodeId)>,
    hidden: bool,
    pending: Option<bool>,
}
struct Scope {
    handle: FocusHandle,
    selection: gpui_base::TextSelectionScopeId,
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
    tab_index: i64,
    bounds: Bounds<Pixels>,
    paint_path: Vec<Boundary>,
    disclosure_path: Vec<(NodeId, NodeId)>,
    navigation_path: Vec<(NodeId, NodeId)>,
}
/// One measured native focus part. Geometry comes from the current paint and is
/// independent of the viewport mask; eligibility still uses the retained tree.
pub(super) struct Target {
    pub handle: FocusHandle,
    pub tab_stop: bool,
    pub bounds: Bounds<Pixels>,
}
#[derive(Clone)]
pub(super) struct Clip {
    pub bounds: Bounds<Pixels>,
    pub x: bool,
    pub y: bool,
}
impl Clip {
    fn intersect(&self, mut target: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
        if self.x {
            let left = target.left().max(self.bounds.left());
            let right = target.right().min(self.bounds.right());
            target.origin.x = left;
            target.size.width = right - left;
        }
        if self.y {
            let top = target.top().max(self.bounds.top());
            let bottom = target.bottom().min(self.bounds.bottom());
            target.origin.y = top;
            target.size.height = bottom - top;
        }
        (target.size.width > gpui::px(0.) && target.size.height > gpui::px(0.)).then_some(target)
    }
}
#[derive(Clone)]
enum Boundary {
    Clip(Clip),
    Scroll(NodeId, std::rc::Weak<super::scroll::State>),
}
impl Boundary {
    fn scroll_clip(state: &super::scroll::State, bounds: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
        match state.mask.get() {
            Some(mask) => Clip {
                bounds: mask,
                x: true,
                y: true,
            }
            .intersect(bounds),
            None => Some(bounds),
        }
    }
    fn project(&self, owner: NodeId, bounds: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
        match self {
            Self::Clip(clip) => clip.intersect(bounds),
            Self::Scroll(node, state) => {
                let state = state.upgrade()?;
                if *node == owner {
                    Some(bounds)
                } else {
                    Self::scroll_clip(&state, state.project(bounds).0)
                }
            }
        }
    }
    fn reveal(&self, owner: NodeId, bounds: Bounds<Pixels>) -> Option<(Bounds<Pixels>, bool)> {
        match self {
            Self::Clip(clip) => clip.intersect(bounds).map(|bounds| (bounds, false)),
            Self::Scroll(node, state) => {
                let state = state.upgrade()?;
                if *node == owner {
                    Some((bounds, false))
                } else {
                    let projected = Self::scroll_clip(&state, state.project(bounds).0)?;
                    let (_, changed) = state.reveal(bounds);
                    Some((projected, changed))
                }
            }
        }
    }
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
    visibility_identity: Rc<()>,
    highlight_styles: BTreeMap<NodeId, HighlightStyle>,
    last_command_target: Option<NodeId>,
    order: u64,
    enter: Option<NodeId>,
    pending: bool,
    navigation: BTreeMap<NodeId, Navigation>,
    navigation_enter: Vec<(NodeId, Option<WeakFocusHandle>)>,
    paint_path: Vec<Boundary>,
    last_focus: Option<WeakFocusHandle>,
    reveal_requested: Cell<bool>,
    viewport: Bounds<Pixels>,
    clipped_targets: bool,
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
            visibility_identity: Rc::new(()),
            highlight_styles: BTreeMap::new(),
            last_command_target: None,
            order: 0,
            enter: None,
            pending: false,
            navigation: BTreeMap::new(),
            navigation_enter: Vec::new(),
            paint_path: Vec::new(),
            last_focus: None,
            reveal_requested: Cell::new(false),
            viewport: Bounds::default(),
            clipped_targets: false,
        }))
    }
    pub(super) fn active_selection_scope(&self) -> gpui_base::TextSelectionScopeId {
        self.active
            .and_then(|id| self.scopes.get(&id))
            .map(|scope| scope.selection)
            .unwrap_or_default()
    }

    pub(super) fn selection_scope(&self, node: NodeId) -> gpui_base::TextSelectionScopeId {
        self.scopes
            .iter()
            .filter(|(id, scope)| scope.config.trap && self.within(node, **id))
            .max_by_key(|(_, scope)| scope.order)
            .map(|(_, scope)| scope.selection)
            .unwrap_or_default()
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
            self.visibility_identity = Rc::new(());
            self.pending = true;
        }
    }
    pub(super) fn select_query(&mut self, children: &[NodeId], selected: Option<NodeId>) {
        let mut changed = false;
        for child in children {
            if Some(*child) == selected {
                changed |= self.query_hidden.remove(child);
            } else {
                changed |= self.query_hidden.insert(*child);
            }
        }
        if changed {
            self.visibility_identity = Rc::new(());
        }
        self.pending = true;
    }
    pub(super) fn set_hidden(&mut self, hidden: BTreeSet<NodeId>) {
        if self.hidden != hidden {
            self.hidden = hidden;
            self.visibility_identity = Rc::new(());
            self.pending = true;
        }
    }
    pub(super) fn allows(&self, node: NodeId) -> bool {
        self.visible(node) && self.active.is_none_or(|scope| self.within(node, scope))
    }
    pub(super) fn visibility_identity(&self) -> Rc<()> {
        self.visibility_identity.clone()
    }
    /// Called with GPUI's computed style, independent of clipping or input gates.
    /// Weak style identity prevents old frames from overriding a later restyle.
    pub(super) fn highlight_style(
        &mut self,
        node: NodeId,
        styles: &std::sync::Arc<[Style]>,
        hidden: bool,
    ) {
        self.highlight_part_style(node, None, styles, hidden);
    }
    pub(super) fn highlight_part_style(
        &mut self,
        node: NodeId,
        part: Option<(usize, NodeId)>,
        styles: &std::sync::Arc<[Style]>,
        hidden: bool,
    ) {
        let session = self.session.borrow();
        if !session
            .tree(self.window)
            .and_then(|t| t.get(node))
            .is_some_and(|n| {
                std::sync::Arc::ptr_eq(&n.style, styles)
                    && part.is_none_or(|(index, part)| n.children.get(index) == Some(&part))
            })
        {
            return;
        }
        let previous = self
            .highlight_styles
            .get(&node)
            .filter(|entry| {
                entry.part == part && entry.source.ptr_eq(&std::sync::Arc::downgrade(styles))
            })
            .map_or_else(
                || part.is_none() && search_style_hidden(styles),
                |entry| entry.hidden,
            );
        self.highlight_styles.insert(
            node,
            HighlightStyle {
                source: std::sync::Arc::downgrade(styles),
                part,
                hidden: previous,
                pending: Some(hidden),
            },
        );
    }

    pub(super) fn has_pending_highlight_styles(&self) -> bool {
        self.highlight_styles
            .values()
            .any(|entry| entry.pending.is_some())
    }

    /// Layout can use yesterday's hover state while paint has today's hitbox.
    /// Commit only the last sample after the complete paint cycle, so intermediate
    /// samples cannot repeatedly invalidate the same stable native frame.
    pub(super) fn commit_highlight_styles(&mut self) -> bool {
        let mut changed = false;
        for entry in self.highlight_styles.values_mut() {
            if let Some(hidden) = entry.pending.take() {
                changed |= entry.hidden != hidden;
                entry.hidden = hidden;
            }
        }
        if changed {
            self.visibility_identity = Rc::new(());
        }
        changed
    }

    fn highlight_style_hidden(&self, tree: &crate::tree::Tree, node: &crate::tree::Node) -> bool {
        let own = self
            .highlight_styles
            .get(&node.id)
            .filter(|entry| {
                entry.part.is_none() && entry.source.ptr_eq(&std::sync::Arc::downgrade(&node.style))
            })
            .map_or_else(|| search_style_hidden(&node.style), |entry| entry.hidden);
        own || node
            .parent
            .and_then(|parent| tree.get(parent))
            .is_some_and(|parent| {
                self.highlight_styles.get(&parent.id).is_some_and(|entry| {
                    entry.part.is_some_and(|(index, part)| {
                        part == node.id && parent.children.get(index) == Some(&part)
                    }) && entry.hidden
                        && entry
                            .source
                            .ptr_eq(&std::sync::Arc::downgrade(&parent.style))
                })
            })
    }
    /// Visual search eligibility is independent of focus, disabled controls and
    /// modal interaction gates. Hidden popup/query/navigation branches still
    /// contribute no displayed source.
    pub(super) fn highlight_visible(&self, tree: &crate::tree::Tree, node: NodeId) -> bool {
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let Some(item) = tree.get(id) else {
                return false;
            };
            if self.hidden.contains(&id)
                || self.query_hidden.contains(&id)
                || navigation_hidden(tree, item)
                || self.highlight_style_hidden(tree, item)
            {
                return false;
            }
            cursor = item.parent;
        }
        true
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
    pub(super) fn eligible(&self, node: NodeId) -> bool {
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
        if item.input_region.as_ref().is_some_and(|config| {
            config.disabled || config.focus == gpuio_protocol::input::Focus::None
        }) || item.table.as_ref().is_some_and(|config| config.disabled)
            || item.carousel.as_ref().is_some_and(|config| config.disabled)
            || item.canvas.as_ref().is_some_and(|config| config.disabled)
            || item.chart.as_ref().is_some_and(|config| config.disabled)
            || item
                .extension
                .as_ref()
                .is_some_and(|config| config.disabled)
            || item.control.is_some_and(Control::disabled)
            || item.link.as_ref().is_some_and(|config| config.disabled)
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
            if style_hidden(item)
                || navigation_hidden(tree, item)
                || item.table.as_ref().is_some_and(|config| config.disabled)
            {
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
    fn closed_card_anchor(&self, focused: NodeId) -> Option<FocusHandle> {
        let session = self.session.borrow();
        let tree = session.tree(self.window)?;
        let mut child = focused;
        while let Some(parent) = tree.get(child)?.parent {
            let node = tree.get(parent)?;
            if node.kind == Kind::HoverCard
                && node.children.get(1) == Some(&child)
                && !self.visible(child)
            {
                let anchor = node.children[0];
                if let Some(entry) = self.entries.iter().find(|entry| {
                    entry.tab_stop && self.eligible(entry.node) && self.within(entry.node, anchor)
                }) {
                    return Some(entry.handle.clone());
                }
            }
            child = parent;
        }
        None
    }

    fn prune_highlight_styles(&mut self) {
        let session = self.session.borrow();
        let tree = session.tree(self.window);
        self.highlight_styles.retain(|id, entry| {
            tree.and_then(|tree| tree.get(*id)).is_some_and(|node| {
                entry.source.ptr_eq(&std::sync::Arc::downgrade(&node.style))
                    && entry
                        .part
                        .is_none_or(|(index, part)| node.children.get(index) == Some(&part))
            })
        });
    }
    pub(super) fn sync(&mut self, window: &mut Window, cx: &mut App) {
        self.prune_highlight_styles();
        self.sync_navigation_focus(window, cx);
        // Keep the previous painted ancestry: removed editor nodes are already
        // absent from the new tree. Select the first still-eligible outer trigger
        // only when its content region or previously focused child became ineligible.
        let previous_focused = self.focused_entry(window, cx).map(|entry| entry.node);
        let disclosure_restore = self
            .focused_entry(window, cx)
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
                        selection: gpui_base::TextSelectionScopeId::new(),
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
            && self.can_focus(handle, window)
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
        if !scope_restored
            && let Some(handle) = previous_focused.and_then(|node| self.closed_card_anchor(node))
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
            .focused_entry(window, cx)
            .is_some_and(|entry| !self.eligible(entry.node));
        if hidden_focus {
            window.blur(cx);
        }
        self.pending = had_scopes
            || !self.scopes.is_empty()
            || hidden_focus
            || !self.navigation_enter.is_empty();
    }

    fn sync_navigation_focus(&mut self, window: &Window, cx: &App) {
        let focused_path = self
            .focused_entry(window, cx)
            .map(|entry| entry.navigation_path.clone())
            .unwrap_or_default();
        if let Some(handle) = window.focused(cx) {
            for (owner, page) in &focused_path {
                if let Some(state) = self.navigation.get_mut(owner) {
                    state.remembered.insert(*page, handle.downgrade());
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
                let carousel = node
                    .parent
                    .and_then(|parent| tree.get(parent))
                    .is_some_and(|owner| owner.carousel.is_some());
                let state = self.navigation.entry(id).or_default();
                // Carousel changes never steal focus from its controls or the
                // surrounding application, including automatic advancement.
                let restore = !carousel
                    || focused_path
                        .iter()
                        .any(|(owner, page)| *owner == id && Some(*page) == state.selected);
                state
                    .remembered
                    .retain(|page, _| node.children.contains(page));
                if state.selected != selected {
                    // Supersede pending focus for older selections of this presenter.
                    self.navigation_enter
                        .retain(|(page, _)| !node.children.contains(page));
                    if visible
                        && restore
                        && let Some(page) = selected
                    {
                        self.navigation_enter
                            .push((page, state.remembered.get(&page).cloned()));
                    }
                    state.selected = selected;
                }
            }
            stack.extend(node.children.iter().rev().copied());
        }
    }
    pub(super) fn begin_frame(&mut self, viewport: Bounds<Pixels>) {
        self.entries.clear();
        self.seen.clear();
        self.paint_path.clear();
        self.viewport = viewport;
        self.clipped_targets = false;
    }
    pub(super) fn enter_scroll(&mut self, node: NodeId, state: &Rc<super::scroll::State>) -> usize {
        let depth = self.paint_path.len();
        self.paint_path
            .push(Boundary::Scroll(node, Rc::downgrade(state)));
        depth
    }
    pub(super) fn enter_clip(&mut self, clip: Clip) -> usize {
        let depth = self.paint_path.len();
        self.paint_path.push(Boundary::Clip(clip));
        depth
    }
    pub(super) fn leave_boundary(&mut self, depth: usize) {
        debug_assert_eq!(self.paint_path.len(), depth + 1);
        self.paint_path.truncate(depth);
    }
    pub(super) fn request_reveal(&self) {
        self.reveal_requested.set(true);
    }
    fn reachable(&self, target: NodeId, bounds: Bounds<Pixels>) -> bool {
        self.paint_path
            .iter()
            .rev()
            .try_fold(bounds, |bounds, boundary| boundary.project(target, bounds))
            .is_some_and(|bounds| bounds.intersects(&self.viewport))
    }
    /// Run after the complete paint, including deferred popups. A focus change
    /// reveals once; later wheel motion with unchanged focus stays user-owned.
    pub(super) fn finish_paint(&mut self, window: &mut Window, cx: &mut App) {
        let focused = window.focused(cx).map(|handle| handle.downgrade());
        if !self.reveal_requested.replace(false) && focused == self.last_focus {
            return;
        }
        self.last_focus = focused.clone();
        let Some(entry) = self
            .focused_entry(window, cx)
            .filter(|entry| self.eligible(entry.node))
        else {
            return;
        };
        // Validate the whole path before moving any owner: a stale or newly
        // clipped outer boundary must not produce a partial reveal.
        if entry
            .paint_path
            .iter()
            .rev()
            .try_fold(entry.bounds, |bounds, boundary| {
                boundary.project(entry.node, bounds)
            })
            .is_none_or(|bounds| !bounds.intersects(&self.viewport))
        {
            return;
        }
        let mut bounds = entry.bounds;
        let mut changed = false;
        for boundary in entry.paint_path.iter().rev() {
            let Some((revealed, moved)) = boundary.reveal(entry.node, bounds) else {
                break;
            };
            bounds = revealed;
            changed |= moved;
        }
        if changed {
            window.refresh();
        }
    }
    /// Exact handles win. Otherwise select the nearest recorded ancestor, so a
    /// compound extension remains one host stop without flattening its own policy.
    /// Resolve ownership before eligibility: an ineligible child must not inherit
    /// permission from an eligible outer input region.
    fn entry_for_handle(&self, handle: &FocusHandle, window: &Window) -> Option<&Entry> {
        if let Some(entry) = self.entries.iter().find(|entry| &entry.handle == handle) {
            return Some(entry);
        }
        self.entries
            .iter()
            .filter(|entry| entry.handle.contains(handle, window))
            .reduce(|nearest, candidate| {
                if nearest.handle.contains(&candidate.handle, window) {
                    candidate
                } else {
                    nearest
                }
            })
    }
    fn focused_entry(&self, window: &Window, cx: &App) -> Option<&Entry> {
        let handle = window.focused(cx)?;
        self.entry_for_handle(&handle, window)
    }
    pub(super) fn focused_node(&self, window: &Window, cx: &App) -> Option<NodeId> {
        self.focused_entry(window, cx).map(|entry| entry.node)
    }
    pub(super) fn can_focus(&self, handle: &FocusHandle, window: &Window) -> bool {
        self.entry_for_handle(handle, window)
            .is_some_and(|entry| self.eligible(entry.node))
    }
    pub(super) fn remember_command_target(&mut self, node: NodeId) {
        if self.eligible(node) {
            self.last_command_target = Some(node);
        }
    }
    pub(super) fn last_command_target(&self) -> Option<NodeId> {
        self.last_command_target.filter(|node| self.eligible(*node))
    }
    pub(super) fn record(
        &mut self,
        node: NodeId,
        handle: FocusHandle,
        tab_stop: bool,
        focused: bool,
        bounds: Bounds<Pixels>,
    ) {
        self.record_part(
            node,
            0,
            Target {
                handle,
                tab_stop,
                bounds,
            },
            focused,
        );
    }
    /// Distinct native controls can share one retained owner (e.g. range thumbs).
    /// Part identity only deduplicates paint; all eligibility stays owner-scoped.
    pub(super) fn record_part(&mut self, node: NodeId, part: u16, target: Target, focused: bool) {
        let Target {
            handle,
            tab_stop,
            bounds,
        } = target;
        if bounds.size.width <= gpui::px(0.) || bounds.size.height <= gpui::px(0.) {
            return;
        }
        if !self.reachable(node, bounds) {
            self.clipped_targets = true;
            return;
        }
        if focused
            && self
                .session
                .borrow()
                .tree(self.window)
                .and_then(|tree| tree.get(node))
                .is_some_and(|node| {
                    node.editor.is_some()
                        || node.number_input.is_some()
                        || node.otp_input.is_some()
                        || node.table.is_some()
                })
        {
            self.last_command_target = Some(node);
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
            let tab_index = self
                .session
                .borrow()
                .tree(self.window)
                .and_then(|tree| tree.get(node))
                .and_then(|node| node.link.as_ref())
                .map_or(0, |config| config.tab_index);
            self.entries.push(Entry {
                node,
                handle,
                tab_stop,
                tab_index,
                bounds,
                paint_path: self.paint_path.clone(),
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
        // One stable sort after paint keeps ties in recorded order, including
        // parts of native compound controls. Never probe focus to discover order.
        if self.entries.iter().any(|entry| entry.tab_index != 0) {
            self.entries.sort_by_key(|entry| entry.tab_index);
        }
        if let Some(scope) = self
            .enter
            .take()
            .filter(|id| self.scopes.contains_key(id) && self.allows(*id))
        {
            if self.focused_entry(window, cx).is_some_and(|entry| {
                entry.tab_stop && self.eligible(entry.node) && self.within(entry.node, scope)
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
            .focused_entry(window, cx)
            .is_some_and(|entry| self.eligible(entry.node))
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
                // A higher modal temporarily owns focus. Keep the destination
                // request while its page remains selected/visible, then restore
                // it after an accepted modal close. Hidden/removed routes retire it.
                if self.visible(page) {
                    self.navigation_enter.push((page, remembered));
                }
                continue;
            }
            if focused {
                continue;
            }
            // Respect a user/native focus choice made while the destination enters.
            if self
                .focused_entry(window, cx)
                .is_some_and(|entry| self.eligible(entry.node) && self.within(entry.node, page))
            {
                focused = true;
                continue;
            }
            let target = remembered
                .as_ref()
                .and_then(|handle| handle.upgrade())
                .filter(|handle| {
                    self.entry_for_handle(handle, window).is_some_and(|entry| {
                        self.eligible(entry.node) && self.within(entry.node, page)
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
    #[cfg(feature = "native-tests")]
    pub(super) fn navigation_test_stats(&self) -> (usize, usize, usize) {
        (
            self.navigation.len(),
            self.navigation
                .values()
                .map(|state| state.remembered.len())
                .sum(),
            self.navigation_enter.len(),
        )
    }
    pub(super) fn traverse(&self, reverse: bool, window: &mut Window, cx: &mut App) {
        self.request_reveal();
        // Inert exits still paint native focus handles. Preserve native traversal
        // for ordinary frames, but never let those ineligible handles become stops.
        if self.active.is_none()
            && !self.clipped_targets
            && self
                .entries
                .iter()
                .all(|entry| entry.tab_index == 0 && self.eligible(entry.node))
        {
            if reverse {
                window.focus_prev(cx);
            } else {
                window.focus_next(cx);
            }
            return;
        }
        let mut entries = self
            .entries
            .iter()
            // Pointer/AX focus may be on a non-stop. Keep its position as the
            // traversal anchor; only destinations must opt into Tab navigation.
            .filter(|entry| self.eligible(entry.node))
            .collect::<Vec<_>>();
        // Paint rebuilds entries even when no tree update schedules finish_frame.
        // Order the current frame here too; otherwise a cosmetic redraw silently
        // restores paint order for the next Tab key.
        if entries.iter().any(|entry| entry.tab_index != 0) {
            entries.sort_by_key(|entry| entry.tab_index);
        }
        let current = self
            .focused_entry(window, cx)
            .and_then(|owner| entries.iter().position(|entry| std::ptr::eq(*entry, owner)));
        let next = if let Some(current) = current {
            // At most one full cycle, including the anchor when it is the only
            // stop. No intermediate focus changes or unbounded all-nonstop loop.
            (1..=entries.len())
                .map(|offset| {
                    if reverse {
                        (current + entries.len() - offset) % entries.len()
                    } else {
                        (current + offset) % entries.len()
                    }
                })
                .find(|&index| entries[index].tab_stop)
        } else if reverse {
            entries.iter().rposition(|entry| entry.tab_stop)
        } else {
            entries.iter().position(|entry| entry.tab_stop)
        };
        if let Some(next) = next {
            window.focus(&entries[next].handle, cx);
        } else if let Some(scope) = self.active {
            window.focus(&self.scopes[&scope].handle, cx);
        }
    }
}
