//! Layout-owned variable-extent lists. Row construction is exclusively native;
//! layout observations request the next bounded OCaml active set asynchronously.
use super::{Interaction, SharedSession, View, apply_styles, color};
use crate::{
    list_index::Index,
    list_state,
    transport::Transport,
    tree::{ListAction, Node},
};
use gpui::{
    Along, App, Axis, Bounds, Context, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, div, prelude::*, px,
};
use gpui_base::StyledExt;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    list::{Config, Row, ScrollTarget, Viewport},
    v1::{Field, Kind, Style},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
};

struct PendingFocus {
    row: i64,
    serial: i64,
    handler: HandlerId,
    _subscriptions: [gpui::Subscription; 2],
}

fn native_axis(axis: gpuio_protocol::list::Axis) -> Axis {
    match axis {
        gpuio_protocol::list::Axis::Vertical => Axis::Vertical,
        gpuio_protocol::list::Axis::Horizontal => Axis::Horizontal,
    }
}

fn row_is_revealed(row: Bounds<Pixels>, viewport: Bounds<Pixels>, axis: Axis) -> bool {
    // An item larger than its viewport is revealed from its leading edge.
    let start = row.origin.along(axis);
    start >= viewport.origin.along(axis) - px(0.5)
        && start + row.size.along(axis).min(viewport.size.along(axis))
            <= viewport.origin.along(axis) + viewport.size.along(axis) + px(0.5)
}

pub(super) struct State {
    pub(super) native: list_state::State,
    config: Arc<Config>,
    rows: Arc<[Row]>,
    mapping: Arc<BTreeMap<i64, NodeId>>,
    handles: BTreeMap<i64, gpui::FocusHandle>,
    pub(super) tree_focus: Option<gpui::FocusHandle>,
    pub(super) list_focus: Option<gpui::FocusHandle>,
    list_cursor: Option<(i64, i64)>,
    pub(super) tree_typeahead: super::tree_typeahead::Clock,
    pub(super) tree_typeahead_activation: Option<gpui::Subscription>,
    pending_focus: Option<PendingFocus>,
    pending_reveal: Option<i64>,
    bound: Option<i64>,
    extra_pins: BTreeSet<i64>,
    width: Option<Pixels>,
    height: Option<Pixels>,
    layout_bounds: Option<Bounds<Pixels>>,
    layout_cache: super::measured_list_layout::Cache,
    pub(super) observed: Option<Viewport>,
    observed_revision: Option<i64>,
}
impl State {
    #[cfg(feature = "native-tests")]
    pub(super) fn resource_counts(&self) -> (usize, usize) {
        (self.mapping.len(), self.handles.len())
    }
    fn new(node: &Node) -> Self {
        let config = node.list_config.clone().expect("validated list config");
        Self {
            native: list_state::State::from_index_for_axis(
                (*config).clone(),
                node.list_index.clone().expect("validated index"),
                native_axis(node.list_axis),
            )
            .expect("validated list"),
            config,
            rows: Arc::from([]),
            mapping: Arc::default(),
            handles: BTreeMap::new(),
            tree_focus: None,
            list_focus: None,
            list_cursor: None,
            tree_typeahead: Default::default(),
            tree_typeahead_activation: None,
            pending_focus: None,
            pending_reveal: None,
            bound: None,
            extra_pins: BTreeSet::new(),
            width: None,
            height: None,
            layout_bounds: None,
            layout_cache: Default::default(),
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
        if node.list_input.is_some_and(|config| !config.disabled) {
            self.list_focus.get_or_insert_with(|| cx.focus_handle());
        } else {
            self.list_focus = None;
        }
        if node.tree_input {
            self.tree_focus.get_or_insert_with(|| cx.focus_handle());
        } else {
            self.tree_focus = None;
            self.tree_typeahead_activation = None;
        }
        self.tree_typeahead
            .sync(node.tree_input.then_some(node.handler).flatten());
        let index = node.list_index.as_ref().expect("validated list index");
        if self
            .pending_reveal
            .is_some_and(|row| index.position(row).is_none())
        {
            self.pending_reveal = None;
        }
        if self.pending_focus.as_ref().is_some_and(|pending| {
            !node.tree_input
                || node.handler != Some(pending.handler)
                || index.position(pending.row).is_none()
        }) {
            self.pending_focus = None;
            self.pending_reveal = None;
        }
        let reordered = self.native.index().revision() != index.revision();
        if reordered {
            self.bind(None);
        }
        self.native
            .replace_index(index.clone())
            .expect("validated source revision");
        if reordered && let Some(pending) = &self.pending_focus {
            let position = index.position(pending.row).expect("live focus target");
            self.native.handle().scroll_to_reveal_item(position);
        }
        let config = node.list_config.as_ref().expect("validated config");
        if self
            .native
            .configure_for_axis((**config).clone(), native_axis(node.list_axis))
            .expect("validated config")
        {
            self.bound = None;
            self.width = None;
            self.height = None;
            self.layout_bounds = None;
            self.observed = None;
            self.config = config.clone();
        }
        let cursor = node
            .list_input
            .filter(|config| !config.disabled)
            .and_then(|config| config.cursor.map(|row| (config.generation, row)));
        if self.list_cursor != cursor {
            self.pending_reveal = cursor.map(|(_, row)| row);
            if let Some((_, row)) = cursor
                && let Some(position) = index.position(row)
            {
                self.native.handle().scroll_to_reveal_item(position);
            }
            self.list_cursor = cursor;
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
    #[cfg(feature = "native-tests")]
    pub(super) fn pending_focus_row(&self) -> Option<i64> {
        self.pending_focus.as_ref().map(|pending| pending.row)
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn pending_reveal_row(&self) -> Option<i64> {
        self.pending_reveal
    }

    pub(super) fn owns_tree_focus(&self, window: &Window) -> bool {
        self.tree_focus
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window))
            || self
                .handles
                .values()
                .any(|handle| handle.is_focused(window))
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
        for (id, state) in &self.tables {
            pins.entry(*id)
                .or_default()
                .extend(state.borrow().pins(window, cx));
        }
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
            .chain(self.otps.iter().filter_map(|(id, otp)| {
                (otp.is_composing(cx) || otp.focus_handle(cx).is_focused(window)).then_some(*id)
            }))
            .chain(
                self.color_inputs
                    .iter()
                    .filter_map(|(id, input)| input.retained(window, cx).then_some(*id)),
            )
            .chain(self.calendars.iter().filter_map(|(id, calendar)| {
                calendar.focus_handle(cx).is_focused(window).then_some(*id)
            }))
            .chain(self.numbers.iter().filter_map(|(id, number)| {
                (number.is_composing(cx) || number.focus_handle(cx).is_focused(window))
                    .then_some(*id)
            }))
            .chain(
                self.documents
                    .iter()
                    .filter_map(|(id, state)| state.retained(window, cx).then_some(*id)),
            )
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
        for state in self.tables.values() {
            state.borrow_mut().extra_pins.clear();
        }
        for pins in self.list_pins(window, cx) {
            if let Some(state) = self.tables.get(&pins.node) {
                state
                    .borrow_mut()
                    .extra_pins
                    .extend(pins.rows.iter().copied());
            }
            if let Some(state) = self.lists.get(&pins.node) {
                state.borrow_mut().extra_pins.extend(pins.rows);
            }
        }
    }

    pub(super) fn sync_lists(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shared = self.session.clone();
        let session = shared.borrow();
        let Some(tree) = session.tree(self.id) else {
            self.lists.clear();
            return;
        };
        self.lists.retain(|id, state| {
            let retained = tree
                .get(*id)
                .is_some_and(|node| node.kind == Kind::VirtualList && node.table.is_none());
            if !retained
                && state
                    .borrow()
                    .list_focus
                    .as_ref()
                    .is_some_and(|focus| focus.is_focused(window))
            {
                window.blur(cx);
            }
            retained
        });
        let dirty: BTreeSet<_> = dirty.iter().copied().collect();
        for id in &dirty {
            if let Some(node) = tree.get(*id)
                && node.kind == Kind::VirtualList
                && node.table.is_none()
            {
                let state = self
                    .lists
                    .entry(*id)
                    .or_insert_with(|| Rc::new(RefCell::new(State::new(node))));
                let mut state = state.borrow_mut();
                // Configuring a list replaces its measurement/scroll handle.
                // Retire the old presentation first so capture and drag hooks
                // finish on that handle, and the next render binds the new one.
                if (state.native.handle().axis() != native_axis(node.list_axis)
                    || node
                        .list_config
                        .as_ref()
                        .is_some_and(|config| config.as_ref() != state.config.as_ref()))
                    && let Some(scrollbar) = self
                        .scrollbars
                        .remove(&(*id, super::scrollbar_host::Owner::Viewport))
                {
                    scrollbar.borrow_mut().close(window, cx);
                }
                if (node.list_input.is_none_or(|config| config.disabled)
                    || !self.focus.borrow().allows(*id))
                    && state
                        .list_focus
                        .as_ref()
                        .is_some_and(|focus| focus.is_focused(window))
                {
                    window.blur(cx);
                }
                state.update(node, &dirty, cx);
                // Retirement is transaction-driven even if an inert/occluded
                // tree will not paint again before it is restored.
                if !self.focus.borrow().allows(*id)
                    || state.pending_focus.as_ref().is_some_and(|pending| {
                        state.mapping.get(&pending.row).is_some_and(|row| {
                            !self.focus.borrow().allows(*row)
                                || !tree.get(*row).is_some_and(|node| {
                                    matches!(node.accessibility.as_ref().and_then(|value| value.role),
                                        Some(gpuio_protocol::accessibility::Role::TreeItem(item)) if !item.disabled)
                                })
                        })
                    })
                {
                    state.pending_focus = None;
                    state.pending_reveal = None;
                }
            }
        }
    }
    pub(super) fn list_actions(
        &mut self,
        actions: &[ListAction],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for action in actions {
            match action {
                ListAction::Invalidate(id, rows) => {
                    if let Some(state) = self.lists.get(id) {
                        state.borrow().native.invalidate_rows(rows);
                    }
                }
                ListAction::Scroll(id, request) => {
                    if let Some(state) = self.lists.get(id) {
                        let fresh = state
                            .borrow_mut()
                            .native
                            .scroll(*request)
                            .expect("validated list command");
                        if fresh {
                            let mut current = state.borrow_mut();
                            current.pending_focus = None;
                            current.pending_reveal = match request.target {
                                ScrollTarget::Reveal(row) | ScrollTarget::FocusTreeRow(row) => {
                                    Some(row)
                                }
                                ScrollTarget::Offset(..) | ScrollTarget::End => None,
                            };
                            drop(current);
                            if let ScrollTarget::FocusTreeRow(row) = request.target {
                                self.start_tree_focus(*id, row, request.serial, window, cx);
                            }
                        }
                    }
                }
            }
        }
    }
    fn start_tree_focus(
        &mut self,
        id: NodeId,
        row: i64,
        serial: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !window.is_window_active() || !self.focus.borrow().allows(id) {
            return;
        }
        let handler = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return;
            };
            let Some(node) = tree.get(id).filter(|node| node.tree_input) else {
                return;
            };
            if let Some(binding) = node.list_rows.iter().find(|binding| binding.id == row) {
                let enabled = tree.get(binding.node).is_some_and(|node| {
                    matches!(node.accessibility.as_ref().and_then(|value| value.role),
                        Some(gpuio_protocol::accessibility::Role::TreeItem(item)) if !item.disabled)
                });
                if !enabled || !self.focus.borrow().allows(binding.node) {
                    return;
                }
            }
            node.handler.expect("validated tree handler")
        };
        let state = self.lists[&id].clone();
        let Some(focus) = state.borrow().tree_focus.clone() else {
            return;
        };
        // Release the prior row pin before requesting the destination, allowing
        // a one-row budget to make progress. Never activate an inactive window.
        focus.focus(window, cx);
        let blur = cx.on_blur(&focus, window, move |view, _, _| {
            view.cancel_tree_focus(id, serial);
        });
        let activation = cx.observe_window_activation(window, move |view, window, _| {
            if !window.is_window_active() {
                view.cancel_tree_focus(id, serial);
            }
        });
        state.borrow_mut().pending_focus = Some(PendingFocus {
            row,
            serial,
            handler,
            _subscriptions: [blur, activation],
        });
    }

    fn cancel_tree_focus(&mut self, id: NodeId, serial: i64) {
        if let Some(state) = self.lists.get(&id) {
            let mut state = state.borrow_mut();
            if state
                .pending_focus
                .as_ref()
                .is_some_and(|pending| pending.serial == serial)
            {
                state.pending_focus = None;
                state.pending_reveal = None;
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
        let mut accessible_name = gpui::SharedString::default();
        for style in node.style.iter() {
            if let Style::Fields(fields) = style {
                for field in fields {
                    match field {
                        Field::PointerEvents(value) => interaction.pointer = *value,
                        Field::UserSelect(value) => interaction.selectable = Some(*value),
                        Field::SelectionColor(value) => {
                            interaction.selection_color = Some(color(value))
                        }
                        Field::AccessibleName(value) => accessible_name = value.clone().into(),
                        _ => (),
                    }
                }
            }
        }
        let (handle, index, mapping, handles, config) = {
            let mut state = state.borrow_mut();
            let focused = state.focused(window, cx).first().copied();
            state.bind(focused);
            // The command may precede layout or destination materialization.
            // Once present, anchor an offscreen destination by row identity;
            // old estimated prefix heights cannot reliably reveal it.
            if let Some(row) = state.pending_reveal
                && state.mapping.contains_key(&row)
                && let Some(position) = state.native.index().position(row)
                && !state
                    .layout_bounds
                    .zip(state.native.handle().bounds_for_item(position))
                    .is_some_and(|(viewport, row)| {
                        row_is_revealed(row, viewport, state.native.handle().axis())
                    })
            {
                state.native.handle().scroll_to(gpui::ListOffset {
                    item_ix: position,
                    offset_in_item: px(0.),
                });
            }
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
        let axis = handle.axis();
        let tree_input = node.tree_input;
        let list_route = super::list_input::Route::new(node);
        let list_cursor = node.list_input.and_then(|config| config.cursor);
        let list_focus = node
            .list_input
            .and_then(|config| self.list_accessibility_owner(node.id, config, window, cx));
        let owner_node = node.id;
        let tree_root = node.accessibility.as_ref().is_some_and(|metadata| {
            matches!(
                metadata.role,
                Some(
                    gpuio_protocol::accessibility::Role::Tree(_)
                        | gpuio_protocol::accessibility::Role::ListBox(_)
                )
            )
        });
        let render_index = index.clone();
        let focus_target = state
            .borrow()
            .pending_focus
            .as_ref()
            .map(|pending| pending.row);
        let focus_rendered = Rc::new(Cell::new(None));
        let render_focus = focus_rendered.clone();
        let list = gpui::list(handle.clone(), move |ix, window, cx| {
            // Never borrow ListState here: GPUI is holding its layout borrow.
            if seen.borrow().len() < cap {
                seen.borrow_mut().insert(ix);
            } else if !seen.borrow().contains(&ix) {
                overflow_render.set(true);
            }
            let placeholder = || {
                let item = div().flex_shrink_0();
                match axis {
                    Axis::Vertical => item.w_full().h(px(estimated as f32)),
                    Axis::Horizontal => item.h_full().w(px(estimated as f32)),
                }
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
                if focus_target == Some(id) {
                    render_focus.set(Some(id));
                }
                let metadata = tree.get(node).and_then(|node| {
                    node.accessibility
                        .as_ref()
                        .filter(|metadata| {
                            matches!(
                                metadata.role,
                                Some(
                                    gpuio_protocol::accessibility::Role::TreeItem(_)
                                        | gpuio_protocol::accessibility::Role::OptionItem(_)
                                )
                            )
                        })
                        .cloned()
                });
                let child = view.element(tree, node, interaction, window, cx);
                let mut row = div()
                    .id(list_route.map_or_else(
                        || gpui::ElementId::from(("list-row", id as u64)),
                        |route| route.row_element_id(id),
                    ))
                    .when(!tree_root && metadata.is_none(), |row| {
                        row.role(gpui::Role::ListItem)
                    })
                    .map(|row| match axis {
                        Axis::Vertical => row.w_full().min_h(px(1.)),
                        Axis::Horizontal => row.h_full().min_w(px(1.)),
                    })
                    .track_focus(&handles[&id])
                    .when(
                        list_route.is_some(),
                        super::list_input::preserve_owner_focus,
                    )
                    .child(child);
                if tree_input
                    && let Some(item) = metadata.as_ref().and_then(|metadata| match metadata.role {
                        Some(gpuio_protocol::accessibility::Role::TreeItem(item)) => Some(item),
                        _ => None,
                    })
                {
                    row = view.tree_row_input(
                        row,
                        owner_node,
                        Row { id, node },
                        handles[&id].clone(),
                        item,
                        cx,
                    );
                }
                if let Some(route) = list_route
                    && let Some(gpuio_protocol::accessibility::Role::OptionItem(item)) =
                        metadata.as_ref().and_then(|metadata| metadata.role)
                {
                    row = view.list_row_input(row, route, Row { id, node }, item, cx);
                    if !item.disabled
                        && list_cursor == Some(id)
                        && let Some(focus) = &list_focus
                    {
                        row = row.aria_active_descendant_for(focus).child(
                            gpui::canvas(
                                |_, _, _| (),
                                |bounds, _, window, _| {
                                    if window.is_window_active() {
                                        window.paint_quad(gpui::outline(
                                            bounds.inset(px(1.)),
                                            window.text_style().color,
                                            gpui::BorderStyle::default(),
                                        ));
                                    }
                                },
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        );
                    }
                }
                crate::semantics::State {
                    identity: None,
                    busy: false,
                    element: row,
                    metadata,
                    hidden: !view.focus.borrow().visible(node),
                    disabled: false,
                    read_only: false,
                    modal: false,
                    live: None,
                }
                .into_any_element()
            })
            .unwrap_or_else(|_| placeholder())
        })
        .size_full();
        let layout_cache = state.borrow().layout_cache.clone();
        let frame = Frame {
            element: list,
            state,
            rendered,
            overflow,
            focus: self.focus.clone(),
            focus_rendered,
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
                .role(gpui::Role::List)
                .aria_orientation(match axis {
                    Axis::Vertical => gpui::accesskit::Orientation::Vertical,
                    Axis::Horizontal => gpui::accesskit::Orientation::Horizontal,
                })
                .aria_label(accessible_name)
                .size_full()
                .min_w_0()
                .min_h_0()
                .relative()
                .overflow_hidden(),
            &node.style,
            interaction,
            false,
        );
        if let Some(style) = states[1].clone() {
            root = root.hover(move |r| r.refine_style(&style));
        }
        if node.tree_input {
            root = self.tree_root_input(root, node.id, window, cx);
        }
        if let Some(route) = list_route {
            root = self.list_root_input(root, route, cx);
        }
        // Inherited metrics can change without a row transaction. Invalidate
        // warm offscreen measurements before GPUI computes the next wheel anchor.
        root = root.child(super::measured_list_layout::observe(
            frame,
            handle.clone(),
            layout_cache,
        ));
        if config.scrollbar {
            root = if let Some(scrollbar) = self.scrollbar_owner(
                tree,
                node,
                super::scrollbar_host::Mount {
                    kind: super::scrollbar_host::Owner::Viewport,
                    handle: Rc::new(handle.clone()),
                },
                interaction,
                window,
                cx,
            ) {
                root.child(crate::scrollbar_widget::inherited_element(&scrollbar))
            } else {
                let scrollbar = match axis {
                    Axis::Vertical => gpui_base::Scrollbar::vertical(&handle),
                    Axis::Horizontal => gpui_base::Scrollbar::horizontal(&handle),
                };
                root.child(scrollbar.id(("list-scrollbar", identity)))
            };
        }
        self.finish_element(
            root,
            node,
            tree.revision(),
            node.list_input.is_some_and(|config| config.disabled),
        )
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
    focus: super::focus::Shared,
    focus_rendered: Rc<Cell<Option<i64>>>,
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
        state.layout_bounds = Some(bounds);
        if !state.owns_tree_focus(window)
            || !window.is_window_active()
            || !self.focus.borrow().allows(self.route.node)
        {
            state.tree_typeahead.clear();
        }
        let visible_bounds = bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&window.fully_visible_bounds());
        if visible_bounds.size.width <= px(0.) || visible_bounds.size.height <= px(0.) {
            state.pending_focus = None;
            state.pending_reveal = None;
            state.tree_typeahead.clear();
        }
        let handle = state.native.handle().clone();
        let axis = handle.axis();
        let cross_changed = match axis {
            Axis::Vertical => state.width != Some(bounds.size.width),
            Axis::Horizontal => state.height != Some(bounds.size.height),
        };
        if cross_changed {
            handle
                .clone()
                .with_uniform_item_extent(px(state.config.estimated_height as f32));
        }
        if state.width != Some(bounds.size.width) || state.height != Some(bounds.size.height) {
            state.width = Some(bounds.size.width);
            state.height = Some(bounds.size.height);
            state.observed = None;
        }
        let index: Arc<Index> = state.native.shared_index();
        let offset = handle.logical_scroll_top();
        let visible_first = offset.item_ix.min(index.len());
        let mut visible_last = visible_first;
        while visible_last < index.len() {
            let Some(row) = handle.bounds_for_item(visible_last) else {
                break;
            };
            if row.origin.along(axis) >= bounds.origin.along(axis) + bounds.size.along(axis) {
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
        let anchor = index
            .id(offset.item_ix)
            .map(|id| (id, f32::from(offset.offset_in_item) as f64));
        // A measured overdraw row may not invoke GPUI's renderer again. Keep a
        // bounded warm set across nearby scrolling, or a one-pixel movement
        // evicts those rows and exposes estimated placeholders on the next wheel.
        // Source/configuration/size changes clear observed; disjoint jumps discard
        // the old set. No additional row history or unbounded cache is retained.
        let mut overscan = self.rendered.borrow().clone();
        if let Some(previous) = state.observed.as_ref().filter(|previous| {
            previous.order_revision == index.revision() && state.config.overscan > 0.
        }) {
            let previous_positions: Vec<_> = previous
                .requested
                .iter()
                .filter_map(|row| index.position(*row))
                .collect();
            // Requested rows exclude focus pins. If the focused row fills the
            // viewport, it can be the only visible row and absent from that
            // vector; actual visible-range overlap must still retain neighbours.
            let overlaps = (previous.visible_first < visible_last as i64
                && (visible_first as i64) < previous.visible_last)
                || previous_positions
                    .iter()
                    .any(|position| (visible_first..visible_last).contains(position));
            if overlaps {
                overscan.extend(previous_positions);
            }
        }
        // Visible rows and focus pins are admitted first below. Spend the
        // remaining budget on the closest warm/rendered neighbours, rather than
        // accumulating rows behind a long scroll or favouring one direction.
        let mut overscan: Vec<_> = overscan.into_iter().collect();
        overscan.sort_by_key(|position| {
            if *position < visible_first {
                (visible_first - position, *position)
            } else {
                (
                    position.saturating_sub(visible_last.saturating_sub(1)),
                    *position,
                )
            }
        });
        let max_active = state.config.max_active as usize;
        let mut requested = Vec::new();
        let mut unique: BTreeSet<_> = pinned.iter().copied().collect();
        // Reserve pins, then pending navigation, visible content and overscan.
        // Partial leading rows must not starve a one-row destination budget.
        let focus_position = state
            .pending_focus
            .as_ref()
            .and_then(|pending| index.position(pending.row))
            .filter(|position| (visible_first..visible_last).contains(position));
        let reveal_position = state.pending_reveal.and_then(|row| index.position(row));
        for ix in reveal_position
            .into_iter()
            .chain(focus_position)
            .chain(visible_first..visible_last)
            .chain(overscan)
        {
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
            || visible_last.saturating_sub(visible_first) + pinned.len() > max_active
            || state
                .pending_reveal
                .is_some_and(|row| !unique.contains(&row));
        let at_end = index.is_empty()
            || handle
                .is_scrolled_to_end()
                .unwrap_or_else(|| handle.max_offset_for_scrollbar().along(axis) == px(0.));
        let viewport = Viewport {
            order_revision: index.revision(),
            visible_first: visible_first as i64,
            visible_last: visible_last as i64,
            requested,
            pinned,
            anchor,
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
        // GPUI's list updates its offset but leaves the wheel event bubbling.
        // Register outside its listener: capture snapshots the position, then
        // our bubble callback runs after the list and any nested scroll child.
        // Consume actual list movement only; boundaries and perpendicular
        // input remain available to ordinary ancestor scrollers. Reading each
        // event's starting position also handles several events in one frame.
        let owner = Rc::downgrade(&self.state);
        window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, _, _| {
            if phase == gpui::DispatchPhase::Capture
                && bounds.contains(&event.position)
                && let Some(owner) = owner.upgrade()
            {
                let mut owner = owner.borrow_mut();
                owner.pending_reveal = None;
                owner.pending_focus = None;
            }
        });
        let owner = Rc::downgrade(&self.state);
        let mut before = None;
        window.on_mouse_event(move |event: &gpui::ScrollWheelEvent, phase, _, cx| {
            if !bounds.contains(&event.position) {
                before = None;
                return;
            }
            let Some(owner) = owner.upgrade() else {
                return;
            };
            let current = owner.borrow().native.handle().logical_scroll_top();
            let current = (current.item_ix, current.offset_in_item);
            match phase {
                gpui::DispatchPhase::Capture => {
                    before = Some(current);
                    if event
                        .delta
                        .pixel_delta(px(16.))
                        .along(owner.borrow().native.handle().axis())
                        != px(0.)
                    {
                        let mut owner = owner.borrow_mut();
                        owner.pending_reveal = None;
                        owner.pending_focus = None;
                    }
                }
                gpui::DispatchPhase::Bubble => {
                    if before.take().is_some_and(|old| old != current) {
                        cx.stop_propagation();
                    }
                }
            }
        });
        // Materialized overscan is not a new Tab destination. The list owns its
        // scrolling/focus handoff; ordinary ancestor reveal cannot reveal a row
        // through this viewport. Floating descendants paint outside this scope.
        let boundary = self.focus.borrow_mut().enter_clip(super::focus::Clip {
            bounds,
            x: true,
            y: true,
        });
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
        self.focus.borrow_mut().leave_boundary(boundary);
        {
            let mut state = self.state.borrow_mut();
            if let Some(row) = state.pending_reveal
                && state.mapping.contains_key(&row)
                && let Some(position) = state.native.index().position(row)
            {
                if self.rendered.borrow().contains(&position)
                    && state
                        .native
                        .handle()
                        .bounds_for_item(position)
                        .is_some_and(|row| {
                            row_is_revealed(row, bounds, state.native.handle().axis())
                        })
                {
                    state.pending_reveal = None;
                } else {
                    // Actual measurement can invalidate the previous estimate.
                    // Retry after this frame, when anchoring sees real bounds.
                    // Missing managed rows instead wait for their observation
                    // response; they must not create a recurring idle redraw.
                    window.defer(cx, |window, _| window.refresh());
                }
            }
        }
        let focus = {
            let mut state = self.state.borrow_mut();
            let Some(pending) = &state.pending_focus else {
                return;
            };
            let live = window.is_window_active()
                && self.route.handler == Some(pending.handler)
                && self.focus.borrow().allows(self.route.node)
                && state
                    .tree_focus
                    .as_ref()
                    .is_some_and(|handle| handle.is_focused(window));
            let position = state.native.index().position(pending.row);
            if !live || position.is_none() {
                state.pending_focus = None;
                state.pending_reveal = None;
                return;
            }
            let position = position.unwrap();
            // User scrolling away while OCaml materializes rows cancels handoff.
            let visible = state
                .native
                .handle()
                .bounds_for_item(position)
                .is_some_and(|row| {
                    let visible = row
                        .intersect(&bounds)
                        .intersect(&window.content_mask().bounds)
                        .intersect(&window.fully_visible_bounds());
                    visible.size.width > px(0.) && visible.size.height > px(0.)
                });
            if !visible {
                if state.pending_reveal.is_some() {
                    return;
                }
                state.pending_focus = None;
                return;
            }
            let Some(node) = state.mapping.get(&pending.row).copied() else {
                return;
            };
            let enabled = self
                .route
                .session
                .borrow()
                .tree(self.route.window)
                .and_then(|tree| tree.get(node))
                .is_some_and(|node| {
                    matches!(node.accessibility.as_ref().and_then(|value| value.role),
                    Some(gpuio_protocol::accessibility::Role::TreeItem(item)) if !item.disabled)
                });
            if !enabled || !self.focus.borrow().allows(node) {
                state.pending_focus = None;
                state.pending_reveal = None;
                return;
            }
            if self.focus_rendered.get() != Some(pending.row) {
                return;
            }
            let focus = state.handles.get(&pending.row).cloned();
            state.pending_focus = None;
            focus
        };
        // Drop the list borrow/subscriptions before focus observers run.
        if let Some(focus) = focus {
            focus.focus(window, cx);
            // Viewport pins were observed in prepaint, before this handoff.
            // GPUI suppresses refresh during paint. Publish the new focus path,
            // outline and pin in one subsequent frame even if OCaml stays idle.
            window.defer(cx, |window, _| window.refresh());
        }
    }
}
