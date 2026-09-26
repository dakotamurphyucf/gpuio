//! Native table delegate over the admitted retained tree. No host-language calls.
use super::{Interaction, SharedSession, View, apply_styles};
use crate::{
    list_index::Index,
    transport::Transport,
    tree::{Node, Tree},
};
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, WeakEntity, Window, div, prelude::*,
    px,
};
use gpui_base::StyledExt;
use gpuio_protocol::{HandlerId, NodeId, WindowId, list::Viewport, table as wire};
use gpuio_table_adapter::{
    Sizable, Size,
    table::{
        Column, ColumnFixed, ColumnGroup, ColumnSort, DataTable, RowKey, Selection, TableDelegate,
        TableEvent, TableState,
    },
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
struct Route {
    window: WindowId,
    node: NodeId,
    handler: HandlerId,
    revision: i64,
    schema: i64,
    query: i64,
    session: SharedSession,
    gate: super::focus::Shared,
    transport: Arc<Transport>,
}
impl Route {
    fn live(&self) -> bool {
        let session = self.session.borrow();
        session
            .tree(self.window)
            .and_then(|tree| tree.get(self.node))
            .is_some_and(|node| {
                node.handler == Some(self.handler)
                    && node.table.as_ref().is_some_and(|config| {
                        config.schema_revision == self.schema
                            && config.query_generation == self.query
                    })
            })
    }
    fn enabled(&self) -> bool {
        self.live()
            && self.gate.borrow().allows(self.node)
            && self
                .session
                .borrow()
                .tree(self.window)
                .and_then(|tree| tree.get(self.node))
                .and_then(|node| node.table.as_ref())
                .is_some_and(|config| !config.disabled)
    }
    fn pointer_allowed(&self, inherited: bool) -> bool {
        if !self.enabled() {
            return false;
        }
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.window) else {
            return false;
        };
        let mut current = Some(self.node);
        while let Some(id) = current {
            let Some(node) = tree.get(id) else {
                return false;
            };
            for style in node.style.iter().rev() {
                if let gpuio_protocol::v1::Style::Fields(fields) = style {
                    for field in fields.iter().rev() {
                        if let gpuio_protocol::v1::Field::PointerEvents(enabled) = field {
                            return *enabled;
                        }
                    }
                }
            }
            current = node.parent;
        }
        inherited
    }
    fn emit(&self, request: wire::Request) {
        if !self.enabled() {
            return;
        }
        let event = self.session.borrow().table_input(
            self.window,
            self.node,
            self.handler,
            self.revision,
            wire::Input {
                schema_revision: self.schema,
                query_generation: self.query,
                request,
            },
        );
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.window)
        {
            self.transport.fault(self.window);
        }
    }
}
struct Delegate {
    owner: WeakEntity<View>,
    route: Route,
    config: Arc<wire::Config>,
    schema: wire::Schema,
    index: Arc<Index>,
    rows: BTreeMap<i64, Arc<[NodeId]>>,
    handles: BTreeMap<i64, FocusHandle>,
    interaction: Interaction,
    styles: Arc<[gpuio_protocol::v1::Style]>,
    #[cfg(feature = "native-tests")]
    rendered: BTreeSet<(i64, String)>,
}
impl TableDelegate for Delegate {
    fn table_event(&self, event: &TableEvent, _: &mut Context<TableState<Self>>) {
        self.route.emit(event_request(event));
    }

    fn copy_selection(&self, selected: &Selection, _: &App) -> Option<String> {
        use gpuio_table_adapter::table::clipboard::{MAX_COPY_BYTES, tsv};
        if !self.route.enabled() {
            return None;
        }
        let session = self.route.session.borrow();
        let tree = session.tree(self.route.window)?;
        if tree.get(self.route.node)?.list_index.as_ref()?.revision() != self.index.revision() {
            return None;
        }
        let cell = |row: RowKey, column: &str| -> Option<&str> {
            self.index.position(row.0 as i64)?;
            let column_index = self
                .config
                .schema
                .columns
                .iter()
                .position(|col| col.id == column)?;
            let id = self.rows.get(&(row.0 as i64))?.get(column_index)?;
            let metadata = tree.get(*id)?.table_cell.as_ref()?;
            (metadata.column == column).then_some(metadata.copy_text.as_str())
        };
        match selected {
            Selection::Empty => None,
            Selection::Cell { row, column } => {
                let text = cell(*row, column)?;
                (text.len() <= MAX_COPY_BYTES).then(|| text.to_owned())
            }
            Selection::Row(row) => {
                // Display order may optimistically differ from admitted schema order.
                let cells = self
                    .schema
                    .columns
                    .iter()
                    .map(|col| cell(*row, &col.id))
                    .collect::<Option<Vec<_>>>()?;
                tsv([cells])
            }
            Selection::Column(column) => {
                // A column is all logical rows, never just the viewport subset.
                if self.index.len() > self.rows.len() {
                    return None;
                }
                let cells = (0..self.index.len())
                    .map(|index| Some([cell(RowKey(self.index.id(index)? as u64), column)?]))
                    .collect::<Option<Vec<_>>>()?;
                tsv(cells)
            }
        }
    }

    fn appearance(&self) -> gpuio_table_adapter::Appearance {
        use gpuio_protocol::v1::{Field, Fill, Style};
        let mut appearance = gpuio_table_adapter::Appearance::default();
        // The outer host box paints the surface once, including gradients and
        // alpha. Nested native table layers must not obscure or blend it again.
        appearance.tokens.table = gpui::transparent_black();
        appearance.tokens.table_head = gpui::transparent_black();
        for style in self.styles.iter() {
            match style {
                Style::Foreground(value) => {
                    appearance.foreground = super::color(value);
                    appearance.table_head_foreground = super::color(value);
                }
                Style::Radius(value) => appearance.radius = px(*value as f32),
                Style::State(7, fields) => {
                    for field in fields {
                        if let Field::Background(Fill::Solid(value)) = field {
                            appearance.tokens.table_active = super::color(value);
                        }
                    }
                }
                Style::Fields(fields) => {
                    for field in fields {
                        match field {
                            Field::Foreground(value) => {
                                appearance.foreground = super::color(value);
                                appearance.table_head_foreground = super::color(value);
                            }
                            Field::BorderColor(value) => {
                                appearance.border = super::color(value);
                                appearance.table_row_border = super::color(value);
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
        appearance
    }
    fn pointer_enabled(&self, _: &App) -> bool {
        self.route.pointer_allowed(self.interaction.pointer)
    }

    fn input_enabled(&self, _: &App) -> bool {
        self.route.enabled()
    }
    fn columns_count(&self, _: &App) -> usize {
        self.schema.columns.len()
    }
    fn rows_count(&self, _: &App) -> usize {
        self.index.len()
    }
    fn row_key(&self, row: usize, _: &App) -> Option<RowKey> {
        self.index.id(row).map(|id| RowKey(id as u64))
    }
    fn row_index(&self, row: RowKey, _: &App) -> Option<usize> {
        self.index.position(row.0 as i64)
    }
    fn column(&self, index: usize, _: &App) -> Column {
        let col = &self.schema.columns[index];
        Column {
            key: col.id.clone().into(),
            name: col.label.clone().into(),
            width: px(col.width as f32),
            min_width: px(col.min_width as f32),
            max_width: px(col.max_width as f32),
            fixed: (col.pin == wire::Pin::Left).then_some(ColumnFixed::Left),
            align: match col.alignment {
                wire::Alignment::Left => gpui::TextAlign::Left,
                wire::Alignment::Center => gpui::TextAlign::Center,
                wire::Alignment::Right => gpui::TextAlign::Right,
            },
            sort: col.sortable.then(|| {
                self.config
                    .sort
                    .as_ref()
                    .filter(|sort| sort.column == col.id)
                    .map_or(ColumnSort::Default, |sort| match sort.direction {
                        wire::Direction::Ascending => ColumnSort::Ascending,
                        wire::Direction::Descending => ColumnSort::Descending,
                    })
            }),
            resizable: col.resizable,
            movable: col.movable,
            ..Default::default()
        }
    }
    fn group_headers(&self, _: &App) -> Option<Vec<Vec<ColumnGroup>>> {
        (!self.schema.headers.is_empty()).then(|| {
            self.schema
                .headers
                .iter()
                .map(|level| {
                    level
                        .iter()
                        .map(|group| ColumnGroup::new(group.label.clone(), group.columns.len()))
                        .collect()
                })
                .collect()
        })
    }
    fn render_tr(
        &mut self,
        row: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> gpui::Stateful<gpui::Div> {
        let key = self.index.id(row).expect("native logical row");
        let element = div().id(("table-row", key as u64));
        if let Some(handle) = self.handles.get(&key) {
            element.track_focus(handle)
        } else {
            element
        }
    }
    fn render_td(
        &mut self,
        row: usize,
        column: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let placeholder = || div().size_full().into_any_element();
        if !self.route.live() {
            return placeholder();
        }
        let Some(row) = self.index.id(row) else {
            return placeholder();
        };
        let key = &self.schema.columns[column].id;
        let Some(column) = self
            .config
            .schema
            .columns
            .iter()
            .position(|col| col.id == *key)
        else {
            return placeholder();
        };
        let Some(cell) = self
            .rows
            .get(&row)
            .and_then(|cells| cells.get(column))
            .copied()
        else {
            return placeholder();
        };
        #[cfg(feature = "native-tests")]
        self.rendered.insert((row, key.clone()));
        let route = self.route.clone();
        let order = self.index.revision();
        let interaction = self.interaction;
        self.owner
            .update(cx, |view, cx| {
                let shared = route.session.clone();
                let session = shared.borrow();
                let Some(tree) = session.tree(route.window) else {
                    return placeholder();
                };
                if tree
                    .get(route.node)
                    .and_then(|node| node.list_index.as_ref())
                    .is_none_or(|index| index.revision() != order)
                    || tree.get(cell).is_none()
                {
                    return placeholder();
                }
                div()
                    .id(("table-cell", cell.slot()))
                    .role(gpui::Role::Cell)
                    .size_full()
                    .child(view.element(tree, cell, interaction, window, cx))
                    .into_any_element()
            })
            .unwrap_or_else(|_| placeholder())
    }
    fn move_column(
        &mut self,
        from: usize,
        to: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> bool {
        let key = self.schema.columns[from].id.clone();
        let mut ids: Vec<_> = self.schema.columns.iter().map(|c| c.id.as_str()).collect();
        ids.remove(from);
        let before = ids.get(to).copied();
        let Some(schema) = self.schema.moved(&key, before) else {
            return false;
        };
        self.schema = schema;
        true
    }
}
pub(super) struct State {
    native: Entity<TableState<Delegate>>,
    pub(super) extra_pins: BTreeSet<i64>,
    pub(super) observed: Option<Viewport>,
    observed_revision: Option<i64>,
}
fn selection(value: &Selection) -> wire::Selection {
    match value {
        Selection::Empty => wire::Selection::Empty,
        Selection::Row(key) => wire::Selection::Row(key.0 as i64),
        Selection::Column(key) => wire::Selection::Column(key.to_string()),
        Selection::Cell { row, column } => wire::Selection::Cell(row.0 as i64, column.to_string()),
    }
}
fn native_selection(value: &wire::Selection) -> Selection {
    match value {
        wire::Selection::Empty => Selection::Empty,
        wire::Selection::Row(row) => Selection::Row(RowKey(*row as u64)),
        wire::Selection::Column(column) => Selection::Column(column.clone().into()),
        wire::Selection::Cell(row, column) => Selection::Cell {
            row: RowKey(*row as u64),
            column: column.clone().into(),
        },
    }
}
fn event_request(event: &TableEvent) -> wire::Request {
    use wire::{Request as R, Selection as S};
    match event {
        TableEvent::SelectRow(row) => R::Select(S::Row(row.0 as i64)),
        TableEvent::SelectColumn(column) => R::Select(S::Column(column.to_string())),
        TableEvent::SelectCell(row, column) => R::Select(S::Cell(row.0 as i64, column.to_string())),
        TableEvent::ClearSelection => R::Select(S::Empty),
        TableEvent::Copy(value) => R::Copy(selection(value)),
        TableEvent::ContextSelection(value) => R::Context(selection(value)),
        TableEvent::ActivatedRow(row) => R::Activate(row.0 as i64, None),
        TableEvent::ActivatedCell(row, column) => {
            R::Activate(row.0 as i64, Some(column.to_string()))
        }
        TableEvent::RightClickedRow(row) => {
            R::Context(row.map_or(S::Empty, |row| S::Row(row.0 as i64)))
        }
        TableEvent::RightClickedCell(row, column) => {
            R::Context(S::Cell(row.0 as i64, column.to_string()))
        }
        TableEvent::ColumnWidthsChanged(widths) => R::Resize(
            widths
                .iter()
                .map(|(id, width)| (id.to_string(), f32::from(*width) as f64))
                .collect(),
        ),
        TableEvent::MoveColumn { column, before } => {
            R::Move(column.to_string(), before.as_ref().map(ToString::to_string))
        }
        TableEvent::SortRequested(column, direction) => R::Sort(
            column.to_string(),
            match direction {
                ColumnSort::Default => None,
                ColumnSort::Ascending => Some(wire::Direction::Ascending),
                ColumnSort::Descending => Some(wire::Direction::Descending),
            },
        ),
    }
}
impl State {
    pub(super) fn owns_focus(&self, window: &Window, cx: &App) -> bool {
        self.native.focus_handle(cx).is_focused(window)
    }
    pub(super) fn command_available(
        &self,
        action: gpuio_protocol::v1::NativeCommand,
        cx: &App,
    ) -> bool {
        let native = self.native.read(cx);
        action == gpuio_protocol::v1::NativeCommand::Copy
            && native.delegate().route.enabled()
            && *native.selection() != Selection::Empty
    }
    pub(super) fn invoke_copy(&self, window: &mut Window, cx: &mut App) -> bool {
        self.native.focus_handle(cx).focus(window, cx);
        self.native.update(cx, |native, cx| native.copy(window, cx))
    }
    pub(super) fn focused(&self, window: &Window, cx: &App) -> bool {
        self.native.focus_handle(cx).contains_focused(window, cx)
    }
    pub(super) fn pins(&self, window: &Window, cx: &App) -> BTreeSet<i64> {
        let native = self.native.read(cx);
        let mut pins: BTreeSet<_> = native
            .delegate()
            .handles
            .iter()
            .filter_map(|(row, handle)| handle.contains_focused(window, cx).then_some(*row))
            .collect();
        if native.focus_handle(cx).is_focused(window) {
            match native.selection() {
                Selection::Row(row) | Selection::Cell { row, .. } => {
                    pins.insert(row.0 as i64);
                }
                _ => (),
            }
        }
        pins
    }
    fn update(&mut self, tree: &Tree, node: &Node, cx: &mut App) {
        let config = node.table.clone().expect("validated table");
        let index = node.list_index.clone().expect("validated index");
        let rows: BTreeMap<_, _> = node
            .list_rows
            .iter()
            .map(|row| {
                (
                    row.id,
                    tree.get(row.node).expect("validated row").children.clone(),
                )
            })
            .collect();
        self.native.update(cx, |state, cx| {
            let old = state.delegate();
            let reset = old.config.schema_revision != config.schema_revision
                || old.config.disabled != config.disabled
                || old.route.handler != node.handler.unwrap();
            let mapping_changed =
                old.index.revision() != index.revision() || *old.config != *config;
            let mut handles = old.handles.clone();
            handles.retain(|id, _| rows.contains_key(id));
            for row in rows.keys() {
                handles.entry(*row).or_insert_with(|| cx.focus_handle());
            }
            let update = |delegate: &mut Delegate| {
                if reset {
                    delegate.schema = config.schema.clone();
                }
                delegate.route.handler = node.handler.unwrap();
                delegate.route.revision = tree.revision();
                delegate.route.schema = config.schema_revision;
                delegate.route.query = config.query_generation;
                delegate.config = config.clone();
                delegate.index = index;
                delegate.rows = rows;
                delegate.handles = handles;
                delegate.styles = node.style.clone();
            };
            if mapping_changed || reset {
                state.update_source_with_size(Size::Size(px(config.row_height as f32)), cx, update);
            } else {
                update(state.delegate_mut());
                cx.notify();
            }
            state.row_selectable = config.selection_mode != wire::SelectionMode::Cells;
            state.cell_selectable = config.selection_mode != wire::SelectionMode::Rows;
            state.col_selectable = config.column_selection;
            if !config.allows_selection(&selection(state.selection()), |row| {
                state.delegate().index.position(row).is_some()
            }) {
                state.replace_selection(Selection::Empty, cx);
            }
            if reset {
                state.reset_columns(cx);
            }
        });
    }
}
impl View {
    fn new_table(
        &self,
        tree: &Tree,
        node: &Node,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> State {
        let config = node.table.clone().unwrap();
        let route = Route {
            window: self.id,
            node: node.id,
            handler: node.handler.unwrap(),
            revision: tree.revision(),
            schema: config.schema_revision,
            query: config.query_generation,
            session: self.session.clone(),
            gate: self.focus.clone(),
            transport: self.transport.clone(),
        };
        let owner = cx.entity().downgrade();
        let native = cx.new(|cx| {
            TableState::new(
                Delegate {
                    owner,
                    route,
                    schema: config.schema.clone(),
                    config,
                    index: node.list_index.clone().unwrap(),
                    rows: BTreeMap::new(),
                    handles: BTreeMap::new(),
                    interaction: Interaction::default(),
                    styles: node.style.clone(),
                    #[cfg(feature = "native-tests")]
                    rendered: BTreeSet::new(),
                },
                window,
                cx,
            )
        });
        let mut state = State {
            native,
            extra_pins: BTreeSet::new(),
            observed: None,
            observed_revision: None,
        };
        state.update(tree, node, cx);
        state
    }
    pub(super) fn sync_tables(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shared = self.session.clone();
        let session = shared.borrow();
        let Some(tree) = session.tree(self.id) else {
            self.tables.clear();
            return;
        };
        self.tables
            .retain(|id, _| tree.get(*id).is_some_and(|node| node.table.is_some()));
        for id in dirty {
            if let Some(node) = tree.get(*id).filter(|node| node.table.is_some()) {
                if let Some(state) = self.tables.get(id) {
                    state.borrow_mut().update(tree, node, cx);
                } else {
                    let state = self.new_table(tree, node, window, cx);
                    self.tables.insert(*id, Rc::new(RefCell::new(state)));
                }
            }
        }
    }
    pub(super) fn table_actions(
        &mut self,
        actions: &[(NodeId, wire::Command)],
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for (id, command) in actions {
            let Some(table) = self.tables.get(id) else {
                continue;
            };
            table.borrow().native.update(cx, |state, cx| {
                match &command.target {
                    wire::Target::SetSelection(value) => {
                        state.replace_selection(native_selection(value), cx);
                    }
                    wire::Target::Reveal(row, column) => {
                        if let Some(row) = state.delegate().index.position(*row) {
                            state
                                .vertical_scroll_handle
                                .scroll_to_item(row, gpui::ScrollStrategy::Nearest);
                        }
                        if let Some(column) = column.as_ref().and_then(|key| {
                            state
                                .delegate()
                                .schema
                                .columns
                                .iter()
                                .position(|column| column.id == *key)
                        }) {
                            state.scroll_to_col(column, cx);
                        }
                    }
                    wire::Target::ScrollTo(row, within) => {
                        if let Some(row) = state.delegate().index.position(*row) {
                            let mut handle = state.vertical_scroll_handle.0.borrow_mut();
                            handle.deferred_scroll_to_item = None;
                            let offset = handle.base_handle.offset();
                            handle.base_handle.set_offset(gpui::point(
                                offset.x,
                                -px((row as f64 * state.delegate().config.row_height + within)
                                    as f32),
                            ));
                        }
                    }
                    wire::Target::ScrollToColumn(column) => {
                        if let Some(index) = state
                            .delegate()
                            .schema
                            .columns
                            .iter()
                            .position(|c| c.id == *column)
                        {
                            state.scroll_to_col(index, cx);
                        }
                    }
                    wire::Target::ScrollToEnd => {
                        if !state.delegate().index.is_empty() {
                            state.vertical_scroll_handle.scroll_to_item(
                                state.delegate().index.len() - 1,
                                gpui::ScrollStrategy::Bottom,
                            );
                        }
                    }
                    wire::Target::ResetColumns => {
                        state.delegate_mut().schema = state.delegate().config.schema.clone();
                        state.reset_columns(cx);
                    }
                }
                cx.notify();
            });
        }
    }
    pub(super) fn table_element(
        &mut self,
        tree: &Tree,
        node: &Node,
        mut interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if !self.tables.contains_key(&node.id) {
            let state = self.new_table(tree, node, window, cx);
            self.tables.insert(node.id, Rc::new(RefCell::new(state)));
        }
        for style in node.style.iter() {
            if let gpuio_protocol::v1::Style::Fields(fields) = style {
                for field in fields {
                    match field {
                        gpuio_protocol::v1::Field::PointerEvents(value) => {
                            interaction.pointer = *value
                        }
                        gpuio_protocol::v1::Field::UserSelect(value) => {
                            interaction.selectable = *value
                        }
                        gpuio_protocol::v1::Field::SelectionColor(value) => {
                            interaction.selection_color = super::color(value)
                        }
                        _ => (),
                    }
                }
            }
        }
        let state = self.tables[&node.id].clone();
        let config = node.table.as_ref().unwrap();
        let native = state.borrow().native.clone();
        native.update(cx, |state, _| {
            state.delegate_mut().interaction = interaction;
        });
        let mut pending = vec![node.id];
        while let Some(id) = pending.pop() {
            self.visited.insert(id);
            if let Some(node) = tree.get(id) {
                pending.extend(node.children.iter().copied());
            }
        }
        let appearance = native.read(cx).delegate().appearance();
        let element = DataTable::new(&native)
            .bordered(false)
            .inherit_text_style(true)
            .with_size(Size::Size(px(config.row_height as f32)))
            .scrollbar_visible(config.scrollbar, config.scrollbar)
            .into_any_element();
        let frame = Frame {
            element,
            state,
            revision: tree.revision(),
        };
        let (mut root, states) = apply_styles(
            div()
                .id(("gpuio-table", node.id.slot()))
                .role(gpui::Role::Table)
                .aria_label(config.label.clone())
                .bg(gpuio_table_adapter::Appearance::default().tokens.table)
                .text_color(appearance.foreground)
                .border_1()
                .border_color(appearance.border)
                .rounded(appearance.radius)
                .size_full()
                .min_w_0()
                .min_h_0()
                .relative()
                .overflow_hidden(),
            &node.style,
            interaction,
            config.disabled,
        );
        if !config.disabled
            && native.focus_handle(cx).is_focused(window)
            && let Some(focused) = &states[0]
        {
            root = root.refine_style(focused);
        }
        if let Some(hovered) = states[1].clone() {
            root = root.hover(move |root| root.refine_style(&hovered));
        }
        if let Some(pressed) = states[2].clone() {
            root = root.active(move |root| root.refine_style(&pressed));
        }
        if config.disabled {
            root = root.opacity(0.5);
            if let Some(disabled) = &states[5] {
                root = root.refine_style(disabled);
            }
        }
        let element =
            self.finish_element(root.child(frame), node, tree.revision(), config.disabled);
        if config.disabled {
            crate::semantics::Inert(element).into_any_element()
        } else {
            element
        }
    }
}

struct Frame {
    element: gpui::AnyElement,
    state: Rc<RefCell<State>>,
    revision: i64,
}
impl IntoElement for Frame {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl gpui::Element for Frame {
    type RequestLayoutState = <gpui::AnyElement as gpui::Element>::RequestLayoutState;
    type PrepaintState = <gpui::AnyElement as gpui::Element>::PrepaintState;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        gpui::Element::request_layout(&mut self.element, id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        gpui::Element::prepaint(&mut self.element, id, inspector, bounds, layout, window, cx);
        let mut state = self.state.borrow_mut();
        let native = state.native.read(cx);
        let delegate = native.delegate();
        let route = delegate.route.clone();
        let index = delegate.index.clone();
        let config = delegate.config.clone();
        if !route.live() {
            return;
        }
        let handle = native.vertical_scroll_handle.0.borrow();
        let body = handle.base_handle.bounds();
        let visible = body
            .intersect(&bounds)
            .intersect(&window.content_mask().bounds)
            .intersect(&window.fully_visible_bounds());
        let offset = f32::from((-handle.base_handle.offset().y).max(px(0.))) as f64;
        let (first, last) =
            if index.is_empty() || visible.size.width <= px(0.) || visible.size.height <= px(0.) {
                (0, 0)
            } else {
                let top = offset + f32::from(visible.top() - body.top()) as f64;
                let bottom = top + f32::from(visible.size.height) as f64;
                (
                    (top / config.row_height).floor().max(0.) as usize,
                    (bottom / config.row_height).ceil().max(0.) as usize,
                )
            };
        let first = first.min(index.len());
        let last = last.min(index.len());
        let at_end = index.is_empty()
            || offset + f32::from(body.size.height) as f64
                >= index.len() as f64 * config.row_height - 0.5;
        drop(handle);
        let cap = config.max_active_rows as usize;
        let pins: BTreeSet<_> = state
            .pins(window, cx)
            .into_iter()
            .chain(state.extra_pins.iter().copied())
            .filter(|row| index.position(*row).is_some())
            .collect();
        let required = last.saturating_sub(first)
            + pins
                .iter()
                .filter(|row| {
                    index
                        .position(**row)
                        .is_none_or(|position| !(first..last).contains(&position))
                })
                .count();
        let mut unique = pins.clone();
        let mut requested = Vec::new();
        let overscan = (config.overscan / config.row_height).ceil() as usize;
        if last > first {
            for position in (first..last)
                .chain(first.saturating_sub(overscan)..first)
                .chain(last..last.saturating_add(overscan).min(index.len()))
            {
                if unique.len() >= cap {
                    break;
                }
                if let Some(row) = index.id(position)
                    && unique.insert(row)
                {
                    requested.push(row);
                }
            }
        }
        let anchor_position = (offset / config.row_height).floor() as usize;
        let viewport = Viewport {
            order_revision: index.revision(),
            visible_first: first as i64,
            visible_last: last as i64,
            requested,
            pinned: pins.into_iter().collect(),
            anchor: index
                .id(anchor_position)
                .map(|row| (row, offset % config.row_height)),
            following_tail: false,
            at_start: offset <= 0.,
            at_end,
            budget_exhausted: required > cap,
        };
        if state.observed.as_ref() != Some(&viewport)
            || state.observed_revision != Some(self.revision)
        {
            state.observed = Some(viewport.clone());
            state.observed_revision = Some(self.revision);
            let event = route.session.borrow().list_viewport(
                route.window,
                route.node,
                route.handler,
                self.revision,
                viewport,
            );
            if let Some(event) = event
                && !route.transport.input(event)
                && route.session.borrow_mut().overload(route.window)
            {
                route.transport.fault(route.window);
            }
        }
    }
    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        gpui::Element::paint(
            &mut self.element,
            id,
            inspector,
            bounds,
            layout,
            prepaint,
            window,
            cx,
        );
        let state = self.state.borrow();
        let native = state.native.read(cx);
        let route = &native.delegate().route;
        let visible = bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&window.fully_visible_bounds());
        if visible.size.width > px(0.)
            && visible.size.height > px(0.)
            && route.live()
            && route.gate.borrow().visible(route.node)
        {
            let focus = state.native.focus_handle(cx);
            route.gate.borrow_mut().record(
                route.node,
                focus.clone(),
                !native.delegate().config.disabled,
                focus.is_focused(window),
            );
        }
    }
}

#[cfg(feature = "native-tests")]
#[path = "table_host_test.rs"]
mod test;

#[cfg(feature = "native-tests")]
pub(crate) fn run_test() {
    test::run();
}
