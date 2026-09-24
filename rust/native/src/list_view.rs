//! Layout-owned variable-height lists. Row construction is exclusively native;
//! layout observations request the next bounded OCaml active set asynchronously.
use super::{Interaction, SharedSession, View, apply_styles, color};
use crate::{
    list_index::Index,
    list_state,
    transport::Transport,
    tree::{ListAction, Node},
};
use gpui::{
    App, Bounds, Context, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window, div, prelude::*, px,
};
use gpui_base::StyledExt;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    list::{Config, Row, Viewport},
    v1::{Field, Kind, Style},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
};

pub(super) struct State {
    pub(super) native: list_state::State,
    config: Arc<Config>,
    rows: Arc<[Row]>,
    mapping: Arc<BTreeMap<i64, NodeId>>,
    handles: BTreeMap<i64, gpui::FocusHandle>,
    bound: Option<i64>,
    extra_pins: BTreeSet<i64>,
    width: Option<Pixels>,
    pub(super) observed: Option<Viewport>,
    observed_revision: Option<i64>,
}
impl State {
    fn new(node: &Node) -> Self {
        let config = node.list_config.clone().expect("validated list config");
        Self {
            native: list_state::State::from_index(
                (*config).clone(),
                node.list_index.clone().expect("validated index"),
            )
            .expect("validated list"),
            config,
            rows: Arc::from([]),
            mapping: Arc::default(),
            handles: BTreeMap::new(),
            bound: None,
            extra_pins: BTreeSet::new(),
            width: None,
            observed: None,
            observed_revision: None,
        }
    }
    fn bind(&mut self, row: Option<i64>) {
        if self.bound == row {
            return;
        }
        let handle = self.native.handle();
        let offset = handle.logical_scroll_top();
        let following = handle.is_following_tail();
        if let Some(old) = self.bound.and_then(|id| self.native.index().position(id)) {
            handle.splice_focusable(old..old + 1, [None]);
        }
        if let Some(id) = row
            && let Some(index) = self.native.index().position(id)
        {
            handle.splice_focusable(index..index + 1, [self.handles.get(&id).cloned()]);
        }
        if !following {
            handle.scroll_to(offset);
        }
        self.bound = row;
    }
    fn update(&mut self, node: &Node, dirty: &BTreeSet<NodeId>, cx: &mut App) {
        let index = node.list_index.as_ref().expect("validated list index");
        if self.native.index().revision() != index.revision() {
            self.bind(None);
        }
        self.native
            .replace_index(index.clone())
            .expect("validated source revision");
        let config = node.list_config.as_ref().expect("validated config");
        if self
            .native
            .configure((**config).clone())
            .expect("validated config")
        {
            self.bound = None;
            self.width = None;
            self.config = config.clone();
        }
        if !Arc::ptr_eq(&self.rows, &node.list_rows) {
            self.rows = node.list_rows.clone();
            self.mapping = Arc::new(self.rows.iter().map(|row| (row.id, row.node)).collect());
            self.handles.retain(|id, _| self.mapping.contains_key(id));
            for row in self.rows.iter() {
                self.handles
                    .entry(row.id)
                    .or_insert_with(|| cx.focus_handle());
            }
        }
        // Dirty ancestors contain the wrapper even when only its text changed.
        // This covers simple retained lists as well as managed descriptions.
        for row in self.rows.iter().filter(|row| dirty.contains(&row.node)) {
            self.native.invalidate_rows(&[row.id]);
        }
    }
    pub(super) fn focused(&self, window: &Window, cx: &App) -> Vec<i64> {
        self.handles
            .iter()
            .filter_map(|(id, handle)| handle.contains_focused(window, cx).then_some(*id))
            .collect()
    }
}

impl View {
    /// Snapshot native resources immediately before transaction validation. This
    /// closes the race between an older viewport event and new native focus.
    pub(super) fn list_pins(
        &self,
        window: &Window,
        cx: &App,
    ) -> Vec<gpuio_protocol::list::Retained> {
        let mut pins: BTreeMap<NodeId, BTreeSet<i64>> = self
            .lists
            .iter()
            .map(|(id, state)| {
                (
                    *id,
                    state.borrow().focused(window, cx).into_iter().collect(),
                )
            })
            .collect();
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return vec![];
        };
        let sensitive = self
            .editors
            .iter()
            .filter_map(|(id, editor)| {
                (editor.is_composing(cx) || editor.focus_handle(cx).is_focused(window))
                    .then_some(*id)
            })
            .chain(self.selections.iter().filter_map(|(id, selection)| {
                let selection = selection.borrow();
                (selection.is_dragging() || selection.focus.is_focused(window)).then_some(*id)
            }));
        for id in sensitive {
            let mut child = id;
            while let Some(parent) = tree.get(child).and_then(|node| node.parent) {
                let node = tree.get(parent).expect("validated parent");
                if node.kind == Kind::VirtualList
                    && let Some(row) = node.list_rows.iter().find(|row| row.node == child)
                {
                    pins.entry(parent).or_default().insert(row.id);
                }
                child = parent;
            }
        }
        pins.into_iter()
            .filter_map(|(node, rows)| {
                (!rows.is_empty()).then_some(gpuio_protocol::list::Retained {
                    node,
                    rows: rows.into_iter().collect(),
                })
            })
            .collect()
    }
    pub(super) fn refresh_list_pins(&mut self, window: &Window, cx: &App) {
        for state in self.lists.values() {
            state.borrow_mut().extra_pins.clear();
        }
        for pins in self.list_pins(window, cx) {
            if let Some(state) = self.lists.get(&pins.node) {
                state.borrow_mut().extra_pins.extend(pins.rows);
            }
        }
    }

    pub(super) fn sync_lists(&mut self, dirty: &[NodeId], cx: &mut Context<Self>) {
        let shared = self.session.clone();
        let session = shared.borrow();
        let Some(tree) = session.tree(self.id) else {
            self.lists.clear();
            return;
        };
        self.lists.retain(|id, _| {
            tree.get(*id)
                .is_some_and(|node| node.kind == Kind::VirtualList)
        });
        let dirty: BTreeSet<_> = dirty.iter().copied().collect();
        for id in &dirty {
            if let Some(node) = tree.get(*id)
                && node.kind == Kind::VirtualList
            {
                self.lists
                    .entry(*id)
                    .or_insert_with(|| Rc::new(RefCell::new(State::new(node))))
                    .borrow_mut()
                    .update(node, &dirty, cx);
            }
        }
    }
    pub(super) fn list_actions(&mut self, actions: &[ListAction]) {
        for action in actions {
            match action {
                ListAction::Invalidate(id, rows) => {
                    if let Some(state) = self.lists.get(id) {
                        state.borrow().native.invalidate_rows(rows);
                    }
                }
                ListAction::Scroll(id, request) => {
                    if let Some(state) = self.lists.get(id) {
                        state
                            .borrow_mut()
                            .native
                            .scroll(*request)
                            .expect("validated list command");
                    }
                }
            }
        }
    }
    pub(super) fn list_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &Node,
        mut interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // Direct test hosts can render without going through the transport loop.
        let state = self
            .lists
            .entry(node.id)
            .or_insert_with(|| {
                let mut state = State::new(node);
                state.update(node, &BTreeSet::new(), cx);
                Rc::new(RefCell::new(state))
            })
            .clone();
        for style in node.style.iter() {
            if let Style::Fields(fields) = style {
                for field in fields {
                    match field {
                        Field::PointerEvents(value) => interaction.pointer = *value,
                        Field::UserSelect(value) => interaction.selectable = *value,
                        Field::SelectionColor(value) => interaction.selection_color = color(value),
                        _ => (),
                    }
                }
            }
        }
        let (handle, index, mapping, handles, config) = {
            let mut state = state.borrow_mut();
            let focused = state.focused(window, cx).first().copied();
            state.bind(focused);
            (
                state.native.handle().clone(),
                state.native.shared_index(),
                state.mapping.clone(),
                state.handles.clone(),
                state.config.clone(),
            )
        };
        // Preserve native state for materialized rows through deferred layout.
        // Managed descriptions bound these subtrees; simple descriptions retain
        // every supplied row by contract.
        let mut pending: Vec<_> = node.children.iter().copied().collect();
        while let Some(id) = pending.pop() {
            self.visited.insert(id);
            if let Some(child) = tree.get(id) {
                pending.extend(child.children.iter().copied());
            }
        }
        self.visited.insert(node.id);
        let rendered = Rc::new(RefCell::new(BTreeSet::new()));
        let seen = rendered.clone();
        let overflow = Rc::new(Cell::new(false));
        let overflow_render = overflow.clone();
        let weak = cx.entity().downgrade();
        let shared = self.session.clone();
        let window_id = self.id;
        let list_id = node.id;
        let generation = index.revision();
        let cap = config.max_active as usize;
        let estimated = config.estimated_height;
        let render_index = index.clone();
        let list = gpui::list(handle.clone(), move |ix, window, cx| {
            // Never borrow ListState here: GPUI is holding its layout borrow.
            if seen.borrow().len() < cap {
                seen.borrow_mut().insert(ix);
            } else if !seen.borrow().contains(&ix) {
                overflow_render.set(true);
            }
            let placeholder = || {
                div()
                    .w_full()
                    .h(px(estimated as f32))
                    .flex_shrink_0()
                    .into_any_element()
            };
            let Some(id) = render_index.id(ix) else {
                return placeholder();
            };
            let Some(node) = mapping.get(&id).copied() else {
                return placeholder();
            };
            weak.update(cx, |view, cx| {
                let session = shared.borrow();
                let Some(tree) = session.tree(window_id) else {
                    return placeholder();
                };
                if tree
                    .get(list_id)
                    .and_then(|node| node.list_index.as_ref())
                    .is_none_or(|index| index.revision() != generation)
                    || tree.get(node).is_none()
                {
                    return placeholder();
                }
                let child = view.element(tree, node, interaction, window, cx);
                div()
                    .id(("list-row", id as u64))
                    .w_full()
                    .min_h(px(1.))
                    .track_focus(&handles[&id])
                    .child(child)
                    .into_any_element()
            })
            .unwrap_or_else(|_| placeholder())
        })
        .size_full();
        let frame = Frame {
            element: list,
            state,
            rendered,
            overflow,
            route: Route {
                window: self.id,
                node: node.id,
                handler: node.handler,
                revision: tree.revision(),
                session: self.session.clone(),
                transport: self.transport.clone(),
            },
        };
        let identity = ((node.id.generation() as u64) << 32) | node.id.slot() as u64;
        let (mut root, states) = apply_styles(
            div()
                .id(("virtual-list", identity))
                .relative()
                .overflow_hidden(),
            &node.style,
            interaction,
            false,
        );
        if let Some(style) = states[1].clone() {
            root = root.hover(move |r| r.refine_style(&style));
        }
        root = root.child(frame);
        if config.scrollbar {
            root = root
                .child(gpui_base::Scrollbar::vertical(&handle).id(("list-scrollbar", identity)));
        }
        root.into_any_element()
    }
}
struct Route {
    window: WindowId,
    node: NodeId,
    handler: Option<HandlerId>,
    revision: i64,
    session: SharedSession,
    transport: Arc<Transport>,
}
struct Frame {
    element: gpui::List,
    state: Rc<RefCell<State>>,
    rendered: Rc<RefCell<BTreeSet<usize>>>,
    overflow: Rc<Cell<bool>>,
    route: Route,
}
impl IntoElement for Frame {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Frame {
    type RequestLayoutState = <gpui::List as Element>::RequestLayoutState;
    type PrepaintState = <gpui::List as Element>::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let result = self
            .element
            .prepaint(id, inspector, bounds, layout, window, cx);
        let mut state = self.state.borrow_mut();
        let handle = state.native.handle().clone();
        if state.width != Some(bounds.size.width) {
            handle
                .clone()
                .with_uniform_item_height(px(state.config.estimated_height as f32));
            state.width = Some(bounds.size.width);
        }
        let index: Arc<Index> = state.native.shared_index();
        let offset = handle.logical_scroll_top();
        let visible_first = offset.item_ix.min(index.len());
        let mut visible_last = visible_first;
        while visible_last < index.len() {
            let Some(row) = handle.bounds_for_item(visible_last) else {
                break;
            };
            if row.top() >= bounds.bottom() {
                break;
            }
            visible_last += 1;
        }
        let pinned: Vec<_> = state
            .focused(window, cx)
            .into_iter()
            .chain(state.extra_pins.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let max_active = state.config.max_active as usize;
        let mut requested = Vec::new();
        let mut unique: BTreeSet<_> = pinned.iter().copied().collect();
        // Reserve pins, then visible content, then overscan. A small budget must
        // not let pre-viewport overdraw starve rows that are actually visible.
        for ix in (visible_first..visible_last).chain(self.rendered.borrow().iter().copied()) {
            if unique.len() >= max_active {
                break;
            }
            if let Some(id) = index.id(ix)
                && unique.insert(id)
            {
                requested.push(id);
            }
        }
        let budget_exhausted = self.overflow.get()
            || visible_last.saturating_sub(visible_first) + pinned.len() > max_active;
        let at_end = index.is_empty()
            || handle
                .is_scrolled_to_end()
                .unwrap_or_else(|| handle.max_offset_for_scrollbar().y == px(0.));
        let viewport = Viewport {
            order_revision: index.revision(),
            visible_first: visible_first as i64,
            visible_last: visible_last as i64,
            requested,
            pinned,
            anchor: index
                .id(offset.item_ix)
                .map(|id| (id, f32::from(offset.offset_in_item) as f64)),
            following_tail: handle.is_following_tail(),
            at_start: offset.item_ix == 0 && offset.offset_in_item <= px(0.),
            at_end,
            budget_exhausted,
        };
        if state.observed.as_ref() != Some(&viewport)
            || state.observed_revision != Some(self.route.revision)
        {
            state.observed = Some(viewport.clone());
            state.observed_revision = Some(self.route.revision);
            if let Some(handler) = self.route.handler {
                let event = self.route.session.borrow().list_viewport(
                    self.route.window,
                    self.route.node,
                    handler,
                    self.route.revision,
                    viewport,
                );
                if let Some(event) = event
                    && !self.route.transport.input(event)
                    && self.route.session.borrow_mut().overload(self.route.window)
                {
                    self.route.transport.fault(self.route.window);
                }
            }
        }
        result
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }
}
