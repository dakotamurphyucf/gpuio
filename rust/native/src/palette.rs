//! A one-shot, native-managed command chooser. The query is not an application
//! editor: native edit commands restore their original document target on close.
use super::{Interaction, View, command::Route};
use gpui::{
    App, AppContext, Bounds, Context, Entity, EntityInputHandler, Focusable, Pixels, Subscription,
    Window, canvas, deferred, div, prelude::*, px, rgba,
};
use gpui_base::input::InputState;
use gpuio_protocol::{NodeId, palette_options, v1::*};
#[path = "palette_content.rs"]
mod content;
#[path = "palette_list.rs"]
mod list;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
struct Row {
    declared_index: usize,
    route: Route,
    enabled: bool,
}
#[derive(PartialEq)]
struct Reveal {
    command: String,
    query: String,
    row_height: f64,
    max_rows: i64,
    index: usize,
    viewport: gpui::Size<Pixels>,
}
pub(super) struct State {
    pub(super) query: Entity<InputState>,
    config: Arc<PaletteConfig>,
    options: Option<Arc<palette_options::Config>>,
    keywords: BTreeMap<String, Vec<String>>,
    semantic_config: Rc<RefCell<Arc<PaletteConfig>>>,
    query_visible: Rc<Cell<bool>>,
    pub(super) closed: bool,
    editor: Option<NodeId>,
    rows: Vec<Row>,
    selected: Option<String>,
    snapshot: Option<gpuio_protocol::palette_state::Snapshot>,
    input_revision: Option<i64>,
    published: Option<(gpuio_protocol::HandlerId, i64)>,
    revealed: Option<Reveal>,
    scroll: list::State,
    layout_cache: super::measured_list_layout::Cache,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    row_bounds: Rc<RefCell<BTreeMap<String, Bounds<Pixels>>>>,
    _subscription: Subscription,
}
impl State {
    #[cfg(feature = "native-tests")]
    pub(super) fn probe(&self) -> (bool, Option<String>, Vec<(String, bool)>) {
        (
            self.closed,
            self.selected.clone(),
            self.rows
                .iter()
                .map(|row| (row.route.config.id.clone(), row.enabled))
                .collect(),
        )
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn geometry(&self) -> (Bounds<Pixels>, gpui::Point<Pixels>, usize) {
        (
            self.bounds.get(),
            self.scroll.handle.scroll_px_offset_for_scrollbar(),
            self.row_bounds.borrow().len(),
        )
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn row_bounds(&self, command: &str) -> Bounds<Pixels> {
        self.row_bounds.borrow()[command]
    }
    fn searchable(&self) -> bool {
        self.options
            .as_ref()
            .is_none_or(|options| options.searchable)
    }
    fn composing(&self, cx: &App) -> bool {
        self.query.read(cx).bridge_composition().is_some()
    }
}
fn keyword_index(options: Option<&palette_options::Config>) -> BTreeMap<String, Vec<String>> {
    options
        .into_iter()
        .flat_map(|options| &options.keywords)
        .map(|entry| {
            (
                entry.command.clone(),
                entry.words.iter().map(|word| word.to_lowercase()).collect(),
            )
        })
        .collect()
}
impl View {
    pub(super) fn sync_palettes(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.palettes.clear();
                return;
            };
            self.palettes.retain(|id, _| tree.get(*id).is_some());
            let mut nodes = vec![];
            let mut stack = tree.root().into_iter().collect::<Vec<_>>();
            while let Some(id) = stack.pop() {
                let node = tree.get(id).expect("validated node");
                if let Some(config) = &node.palette {
                    nodes.push((id, config.clone(), node.palette_options.clone()));
                }
                stack.extend(node.children.iter().copied());
            }
            nodes
        };
        for (id, config, options) in nodes {
            if let Some(state) = self.palettes.get_mut(&id) {
                if state.config != config {
                    state.query.update(cx, |query, cx| {
                        query.set_placeholder(config.placeholder.clone(), window, cx)
                    });
                    *state.semantic_config.borrow_mut() = config.clone();
                    state.config = config;
                }
                if state.options != options {
                    let was_searchable = state.searchable();
                    state.keywords = keyword_index(options.as_deref());
                    state.options = options;
                    let searchable = state.searchable();
                    state.query_visible.set(searchable);
                    if was_searchable != searchable {
                        let query_focus = state.query.read(cx).focus_handle(cx);
                        let scope_focus = self.focus.borrow().handle(id);
                        if !searchable && state.composing(cx) {
                            state
                                .query
                                .update(cx, |query, cx| query.unmark_text(window, cx));
                        }
                        if !state.closed
                            && self.focus.borrow().top_overlay(id)
                            && let Some(scope_focus) = scope_focus
                        {
                            if !searchable && query_focus.is_focused(window) {
                                window.focus(&scope_focus, cx);
                            } else if searchable && scope_focus.is_focused(window) {
                                window.focus(&query_focus, cx);
                            }
                        }
                    }
                }
                continue;
            }
            let editor = self.command_editor(window, cx);
            let query = cx.new(|cx| {
                InputState::new(window, cx)
                    .bridge_max_bytes(PALETTE_QUERY_BYTES)
                    .bridge_history_budget(PALETTE_HISTORY_BYTES)
            });
            let gate = self.focus.clone();
            let semantic_config = Rc::new(RefCell::new(config.clone()));
            let semantic_labels = semantic_config.clone();
            let query_visible = Rc::new(Cell::new(
                options.as_ref().is_none_or(|options| options.searchable),
            ));
            let query_actions = query_visible.clone();
            let placeholder = config.placeholder.clone();
            query.update(cx, |state, cx| {
                state.set_placeholder(placeholder, window, cx);
                state.set_submit_on_enter(false, cx);
                state.set_context_menu_enabled(false);
                state.set_bridge_decorator(Rc::new(move |element, state, _, cx| {
                    let input = cx.weak_entity();
                    let gate = gate.clone();
                    let focus_gate = gate.clone();
                    let focus_visible = query_actions.clone();
                    let value_visible = query_actions.clone();
                    let focus = state.focus_handle(cx);
                    element
                        .role(gpui::Role::EditableComboBox)
                        .aria_label(semantic_labels.borrow().label.clone())
                        .aria_placeholder(semantic_labels.borrow().placeholder.clone())
                        .aria_value(state.value())
                        .aria_expanded(true)
                        .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                            if focus_visible.get()
                                && focus_gate.borrow().allows(id)
                                && focus_gate.borrow().visible(id)
                            {
                                window.focus(&focus, cx);
                            }
                        })
                        .on_a11y_action(
                            gpui::AccessibleAction::SetValue,
                            move |data, window, cx| {
                                if !value_visible.get()
                                    || !gate.borrow().allows(id)
                                    || !gate.borrow().visible(id)
                                {
                                    return;
                                }
                                if let Some(gpui::accesskit::ActionData::Value(value)) = data
                                    && value.len() <= PALETTE_QUERY_BYTES
                                    && !value.contains(['\0', '\r', '\n'])
                                {
                                    let _ = input.update(cx, |state, cx| {
                                        if state.bridge_composition().is_none() {
                                            state.set_value(value.clone(), window, cx);
                                        }
                                    });
                                }
                            },
                        )
                        .into_any_element()
                }));
            });
            let subscription = cx.observe_in(&query, window, move |view, _, window, cx| {
                view.refresh_palette(id, cx);
                view.sync_tooltips(window, cx);
                view.suspend_hidden_animations();
                view.suspend_hidden_programs();
                cx.notify();
            });
            self.palettes.insert(
                id,
                State {
                    query,
                    config,
                    keywords: keyword_index(options.as_deref()),
                    options,
                    semantic_config,
                    query_visible,
                    closed: false,
                    editor,
                    rows: vec![],
                    selected: None,
                    snapshot: None,
                    input_revision: None,
                    published: None,
                    revealed: None,
                    scroll: Default::default(),
                    layout_cache: Default::default(),
                    bounds: Default::default(),
                    row_bounds: Default::default(),
                    _subscription: subscription,
                },
            );
        }
        // Retire old projected labels/keys at admission even when an occluded
        // window will not render. Retained data must follow its budgeted tree.
        let ids = self.palettes.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.refresh_palette(id, cx);
            if dirty.contains(&id) {
                let state = &self.palettes[&id];
                state
                    .scroll
                    .handle
                    .remeasure_items(0..state.scroll.rows().len());
            }
        }
    }
    fn refresh_palette(&mut self, id: NodeId, cx: &App) -> String {
        let Some(state) = self.palettes.get(&id) else {
            return String::new();
        };
        let query = state.query.read(cx).value().to_lowercase();
        let (rows, layout, row_height) = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return query;
            };
            let Some(node) = tree.get(id) else {
                return query;
            };
            let Some(config) = node.palette.as_ref() else {
                return query;
            };
            let rows = config
                .commands
                .iter()
                .enumerate()
                .filter_map(|(declared_index, command)| {
                    let (scope, command) = tree.command(id, command)?;
                    let search = state
                        .options
                        .as_ref()
                        .map_or(palette_options::Search::AllTerms, |options| options.search);
                    let keywords = state
                        .keywords
                        .get(&command.id)
                        .map_or(&[][..], Vec::as_slice);
                    (!state.searchable()
                        || search.matches(
                            &command.label.to_lowercase(),
                            &command.id.to_lowercase(),
                            keywords,
                            &query,
                        ))
                    .then(|| Row {
                        declared_index,
                        route: Route::new(tree, scope, command, CommandSource::Palette(id)),
                        enabled: self.palette_available(id, command, cx),
                    })
                })
                .collect::<Vec<_>>();
            (
                rows,
                node.palette_layout.clone(),
                node.choice_appearance
                    .as_ref()
                    .map_or(32., |appearance| appearance.row_height),
            )
        };
        let state = self.palettes.get_mut(&id).unwrap();
        if !rows
            .iter()
            .any(|row| row.enabled && Some(&row.route.config.id) == state.selected.as_ref())
        {
            state.selected = rows
                .iter()
                .find(|row| row.enabled)
                .map(|row| row.route.config.id.clone());
        }
        let matched = rows
            .iter()
            .map(|row| row.route.config.id.clone())
            .collect::<Vec<_>>();
        state.scroll.replace(
            list::project(layout.as_deref(), &state.config.commands, &matched),
            row_height as f32,
        );
        state.rows = rows;
        self.observe_palette(id, cx);
        query
    }
    // Pure snapshots cross the bounded asynchronous bridge. Producing one never
    // invokes OCaml or makes native query/selection dependent on an observer.
    fn observe_palette(&mut self, id: NodeId, cx: &App) {
        let Some(state) = self.palettes.get_mut(&id) else {
            return;
        };
        if state.closed {
            return;
        }
        let query = state.query.read(cx).value().to_string();
        let composing = state.composing(cx);
        let previous = state.snapshot.as_ref();
        let input_revision = state.query.read(cx).bridge_revision();
        // GPUI can coalesce notifications. The editor revision catches edits
        // restoring the old text even when no intermediate value was observed.
        let query_changed = state.input_revision != Some(input_revision)
            || previous.is_none_or(|s| s.query != query || s.composing != composing);
        let changed = query_changed
            || previous.is_none_or(|s| {
                s.selected != state.selected || s.matched_count != state.rows.len() as i64
            });
        if changed {
            let Some(sequence) = previous.map_or(Some(1), |s| s.sequence.checked_add(1)) else {
                if self.session.borrow_mut().overload(self.id) {
                    self.transport.fault(self.id);
                }
                return;
            };
            state.input_revision = Some(input_revision);
            state.snapshot = Some(gpuio_protocol::palette_state::Snapshot {
                sequence,
                query_revision: if query_changed {
                    sequence
                } else {
                    previous.unwrap().query_revision
                },
                query,
                composing,
                selected: state.selected.clone(),
                matched_count: state.rows.len() as i64,
            });
        }
        let snapshot = state.snapshot.as_ref().unwrap();
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return;
        };
        let Some(node) = tree.get(id) else {
            return;
        };
        let Some(handler) = node.handler.filter(|_| node.palette_observed) else {
            state.published = None;
            return;
        };
        if state.published == Some((handler, snapshot.sequence)) {
            return;
        }
        let event =
            session.palette_observed(self.id, id, handler, tree.revision(), snapshot.clone());
        if let Some(event) = event {
            state.published = Some((handler, snapshot.sequence));
            drop(session);
            self.publish_palette_dismissal(event);
        }
    }
    fn palette_available(&self, id: NodeId, config: &CommandConfig, cx: &App) -> bool {
        if !config.enabled {
            return false;
        }
        let CommandTarget::Native(action) = config.target else {
            return true;
        };
        let Some(editor) = self.palettes.get(&id).and_then(|state| state.editor) else {
            return false;
        };
        self.focus.borrow().allows_without(id, editor)
            && self.focus.borrow().visible(editor)
            && self
                .editors
                .get(&editor)
                .is_some_and(|editor| editor.command_available(action, cx))
    }
    fn finish_palette(
        &mut self,
        id: NodeId,
        reason: PaletteDismissal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Event> {
        if !self.focus.borrow().top_overlay(id) {
            return None;
        }
        let event = self.take_palette_dismissal(id, reason)?;
        self.sync_tooltips(window, cx);
        cx.notify();
        Some(event)
    }
    // Complete visibility-driven dismissal before paint discards the query's
    // focus ancestry. The shared scope sync can then restore its previous owner.
    pub(super) fn dismiss_hidden_palettes(&mut self) -> Vec<NodeId> {
        let hidden = self
            .palettes
            .iter()
            .filter(|(id, state)| !state.closed && !self.focus.borrow().interactive(**id))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        hidden
            .into_iter()
            .filter(|id| {
                if let Some(event) = self.take_palette_dismissal(*id, PaletteDismissal::Escape) {
                    self.publish_palette_dismissal(event);
                    true
                } else {
                    false
                }
            })
            .collect()
    }
    fn take_palette_dismissal(&mut self, id: NodeId, reason: PaletteDismissal) -> Option<Event> {
        let state = self.palettes.get(&id)?;
        if state.closed || !state.config.allows(&reason) {
            return None;
        }
        let event = {
            let session = self.session.borrow();
            let tree = session.tree(self.id)?;
            let handler = tree.get(id).and_then(|node| node.handler)?;
            session.palette_dismissed(self.id, id, handler, tree.revision(), reason)
        }?;
        let state = self.palettes.get_mut(&id).unwrap();
        state.closed = true;
        state.rows.clear();
        state.row_bounds.borrow_mut().clear();
        Some(event)
    }
    fn publish_palette_dismissal(&self, event: Event) {
        if !self.transport.input(event) && self.session.borrow_mut().overload(self.id) {
            self.transport.fault(self.id);
        }
    }
    fn close_palette(
        &mut self,
        id: NodeId,
        reason: PaletteDismissal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(event) = self.finish_palette(id, reason, window, cx) {
            self.publish_palette_dismissal(event);
        }
    }
    fn select_palette(
        &mut self,
        id: NodeId,
        route: &Route,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_palette(id, cx);
        let Some(state) = self.palettes.get(&id) else {
            return;
        };
        if state.closed
            || state.composing(cx)
            || !self.focus.borrow().top_overlay(id)
            || !self.focus.borrow().visible(id)
            || !self.palette_available(id, &route.config, cx)
        {
            return;
        }
        if !state.rows.iter().any(|row| {
            row.enabled
                && row.route.config.id == route.config.id
                && row.route.request().scope == route.request().scope
                && row.route.config.generation == route.config.generation
        }) {
            return;
        }
        if self
            .session
            .borrow()
            .command_target(self.id, route.request())
            .is_none()
        {
            return;
        }
        // Native close releases this scope before edit-target validation. The
        // dismissal callback merely removes the spent presentation from OCaml.
        if let Some(event) = self.finish_palette(
            id,
            PaletteDismissal::Selected(route.config.id.clone()),
            window,
            cx,
        ) {
            self.invoke_command(route, window, cx);
            self.publish_palette_dismissal(event);
        }
    }

    fn palette_key(
        &mut self,
        id: NodeId,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.focus.borrow().top_overlay(id) || !self.palette_owns_keys(id, window, cx) {
            return false;
        }
        if !matches!(
            key,
            "enter" | "escape" | "up" | "down" | "pageup" | "pagedown"
        ) {
            return false;
        }
        self.refresh_palette(id, cx);
        let Some(state) = self.palettes.get(&id) else {
            return false;
        };
        if state.closed {
            return false;
        }
        if state.composing(cx) {
            if key == "escape" {
                state.query.update(cx, |state, cx| {
                    state.unmark_text(window, cx);
                    cx.notify();
                });
                return true;
            }
            return false;
        }
        if key == "escape" {
            if state.searchable()
                && state.options.as_ref().is_some_and(|options| {
                    options.escape == palette_options::Escape::ClearQueryFirst
                })
                && !state.query.read(cx).value().is_empty()
            {
                state
                    .query
                    .update(cx, |query, cx| query.set_value("", window, cx));
                self.refresh_palette(id, cx);
                cx.notify();
                return true;
            }
            self.close_palette(id, PaletteDismissal::Escape, window, cx);
            return true;
        }
        if key == "enter" {
            let route = state
                .rows
                .iter()
                .find(|row| Some(&row.route.config.id) == state.selected.as_ref() && row.enabled)
                .map(|row| row.route.clone());
            if let Some(route) = route {
                self.select_palette(id, &route, window, cx);
            }
            return true;
        }
        let delta = match key {
            "up" => -1,
            "down" => 1,
            "pageup" => -8,
            "pagedown" => 8,
            _ => return false,
        };
        let state = self.palettes.get_mut(&id).unwrap();
        let enabled = state
            .rows
            .iter()
            .filter(|row| row.enabled)
            .map(|row| row.route.config.id.clone())
            .collect::<Vec<_>>();
        if !enabled.is_empty() {
            let current = enabled
                .iter()
                .position(|id| Some(id) == state.selected.as_ref())
                .unwrap_or(0) as isize;
            state.selected = Some(
                enabled[(current + delta).rem_euclid(enabled.len() as isize) as usize].clone(),
            );
            self.observe_palette(id, cx);
            cx.notify();
        }
        true
    }
    pub(super) fn palette_element(
        &mut self,
        node: &crate::tree::Node,
        mut interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = node.id;
        let Some(state) = self.palettes.get(&id) else {
            return div().into_any_element();
        };
        if state.closed || self.focus.borrow().hidden(id) {
            return div().into_any_element();
        }
        if !self.focus.borrow().interactive(id) {
            return div().into_any_element();
        }
        for style in node.style.iter() {
            if let Style::Fields(fields) = style {
                for field in fields {
                    if let Field::PointerEvents(pointer) = field {
                        interaction.pointer = *pointer;
                    }
                }
            }
        }
        let config = node.palette.as_ref().unwrap().clone();
        let query_text = self.refresh_palette(id, cx);
        let appearance = node
            .choice_appearance
            .clone()
            .unwrap_or_else(crate::appearance::default);
        let state = self.palettes.get_mut(&id).unwrap();
        let rows = state.rows.clone();
        if let Some(selected) = &state.selected {
            let index = state.scroll.command_position(selected).unwrap();
            let reveal = Reveal {
                command: selected.clone(),
                query: query_text,
                row_height: appearance.row_height,
                max_rows: appearance.max_visible_rows,
                index,
                viewport: crate::window_frame::content_bounds(window).size,
            };
            if state.revealed.as_ref() != Some(&reveal) {
                state.scroll.handle.scroll_to_reveal_item(index);
                state.revealed = Some(reveal);
            }
        }
        state.row_bounds.borrow_mut().clear();
        let bounds = state.bounds.clone();
        let row_bounds = state.row_bounds.clone();
        let query = state.query.clone();
        let searchable = state.searchable();
        let selected = state.selected.clone();
        let scroll = state.scroll.handle.clone();
        let layout_handle = scroll.clone();
        let layout_cache = state.layout_cache.clone();
        let visual_rows = state.scroll.rows();
        let scope_focus = self
            .focus
            .borrow()
            .handle(id)
            .expect("mounted palette scope");
        let query_focus = query.read(cx).focus_handle(cx).tab_stop(true);
        let gate = self.focus.clone();
        let record_focus = if searchable {
            query_focus.clone()
        } else {
            scope_focus.clone().tab_stop(true)
        };
        let owner = cx.weak_entity();
        let style = appearance.clone();
        let row_content = self.palette_row_content(node, interaction, cx);
        let rich_layout = row_content.is_some();
        let count = visual_rows.len();
        let list = gpui::list(scroll, move |index, window, cx| {
            let row = match &visual_rows[index] {
                list::Row::Command { index, .. } => rows[*index].clone(),
                list::Row::Heading { id, label } => {
                    return div()
                        .id(gpui::SharedString::from(format!("palette-group:{id}")))
                        .w_full()
                        .px(px(8.))
                        .py(px(5.))
                        .text_size(px(12.))
                        .role(gpui::Role::Label)
                        .aria_label(label.clone())
                        .child(gpui::SharedString::from(label.clone()))
                        .into_any_element();
                }
                list::Row::Separator(index) => {
                    return div()
                        .id(("palette-divider", *index))
                        .w_full()
                        .py(px(4.))
                        .child(div().w_full().h(px(1.)).bg(rgba(0x80808060)))
                        .into_any_element();
                }
            };
            let custom = row_content
                .as_ref()
                .and_then(|content| content(row.declared_index, !row.enabled, window, cx));
            let active = Some(&row.route.config.id) == selected.as_ref();
            let mut item = div()
                .id(gpui::SharedString::from(format!(
                    "palette-command:{}",
                    row.route.config.id
                )))
                .w_full()
                .min_h(px(style.row_height as f32))
                .when(custom.is_none(), |item| item.h(px(style.row_height as f32)))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .overflow_hidden()
                .role(gpui::Role::ListBoxOption)
                .aria_label(row.route.config.label.clone());
            gpui::Refineable::refine(
                item.style(),
                &crate::appearance::refinement(&style.option_style, 0),
            );
            if active && row.enabled {
                item = item.bg(rgba(0x386ac880)).aria_active_descendant();
                gpui::Refineable::refine(
                    item.style(),
                    &crate::appearance::refinement(&style.option_style, 1),
                );
            }
            if let Some(checked) = row.route.config.checked {
                item = item.aria_toggled(if checked {
                    gpui::accesskit::Toggled::True
                } else {
                    gpui::accesskit::Toggled::False
                });
                if checked {
                    gpui::Refineable::refine(
                        item.style(),
                        &crate::appearance::refinement(&style.option_style, 7),
                    );
                }
            }
            item = item
                .child(if row.route.config.checked == Some(true) {
                    "✓"
                } else {
                    ""
                })
                .child(custom.unwrap_or_else(|| {
                    div()
                        .child(gpui::SharedString::from(row.route.config.label.clone()))
                        .into_any_element()
                }));
            if row.enabled {
                let ax_owner = owner.clone();
                let ax_route = row.route.clone();
                item = item.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                    let _ = ax_owner.update(cx, |view, cx| {
                        view.select_palette(id, &ax_route, window, cx)
                    });
                });
                if interaction.pointer {
                    let click_owner = owner.clone();
                    let click_route = row.route.clone();
                    let hovered = crate::appearance::refinement(&style.option_style, 2);
                    let pressed = crate::appearance::refinement(&style.option_style, 3);
                    item = item
                        .cursor_pointer()
                        .hover(move |_| hovered)
                        .active(move |_| pressed)
                        .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_click(move |_, window, cx| {
                            let _ = click_owner.update(cx, |view, cx| {
                                view.select_palette(id, &click_route, window, cx)
                            });
                        });
                }
            } else {
                item = item.opacity(0.5);
                gpui::Refineable::refine(
                    item.style(),
                    &crate::appearance::refinement(&style.option_style, 6),
                );
            }
            let rows = row_bounds.clone();
            let command = row.route.config.id.clone();
            item = item.child(
                canvas(
                    move |bounds, _, _| {
                        rows.borrow_mut().insert(command.clone(), bounds);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
            crate::semantics::State {
                identity: None,
                busy: false,
                hidden: false,
                metadata: None,
                live: None,
                element: item,
                disabled: !row.enabled,
                read_only: false,
                modal: false,
            }
            .into_any_element()
        })
        .with_sizing_behavior(if rich_layout {
            gpui::ListSizingBehavior::Infer
        } else {
            gpui::ListSizingBehavior::Auto
        })
        .when(rich_layout, |list| {
            list.min_h(px(appearance.row_height as f32)).max_h(
                px((appearance.max_visible_rows as f64 * appearance.row_height) as f32).min(
                    (crate::window_frame::content_bounds(window).size.height - px(120.))
                        .max(px(1.)),
                ),
            )
        })
        .when(!rich_layout, |list| {
            list.h(px(
                (count.min(appearance.max_visible_rows as usize).max(1) as f64
                    * appearance.row_height) as f32,
            )
            .min((crate::window_frame::content_bounds(window).size.height - px(120.)).max(px(1.))))
        })
        .w_full();
        let list = super::measured_list_layout::observe(list, layout_handle, layout_cache);
        let owner = cx.weak_entity();
        let escape_owner = owner.clone();
        let bubble_escape_owner = owner.clone();
        let bubble_key_owner = owner.clone();
        let enter_owner = owner.clone();
        let key_owner = owner.clone();
        let width = px(appearance.popup_width as f32)
            .min((crate::window_frame::content_bounds(window).size.width - px(32.)).max(px(1.)));
        let mut panel = div()
            .id(("palette", id.slot()))
            .track_focus(&scope_focus)
            .w(width)
            .flex()
            .flex_col()
            .p(px(8.))
            .gap(px(8.))
            .bg(rgba(0x20242aff))
            .text_color(rgba(0xffffffff))
            .rounded(px(6.))
            .role(gpui::Role::Dialog)
            .aria_label(config.label.clone())
            .occlude();
        gpui::Refineable::refine(
            panel.style(),
            &crate::appearance::refinement(&appearance.popup_style, 0),
        );
        let mut hovered = crate::appearance::refinement(&appearance.popup_style, 2);
        let (styled, states) = super::apply_styles(panel, &node.style, interaction, false);
        panel = styled;
        let [focused, hover, pressed, _, _, _, _] = states;
        if let Some(style) = focused
            && (scope_focus.contains_focused(window, cx) || query_focus.is_focused(window))
        {
            gpui::Refineable::refine(panel.style(), &style);
        }
        if interaction.pointer {
            if let Some(style) = hover {
                gpui::Refineable::refine(&mut hovered, &style);
            }
            panel = panel.hover(move |_| hovered);
            if let Some(style) = pressed {
                panel = panel.active(move |_| style);
            }
        }
        if let Some(header) = self.palette_slot(node, 0, interaction, window, cx) {
            panel = panel.child(header);
        }
        if searchable {
            panel = panel.child(query.clone());
        }
        // Record the private query at its visual position so ordinary Tab
        // traversal follows header -> query -> empty/footer controls.
        panel = panel
            .child(
                canvas(
                    move |rect, _, _| bounds.set(rect),
                    move |bounds, _, window, _| {
                        gate.borrow_mut().record(
                            id,
                            record_focus.clone(),
                            true,
                            record_focus.is_focused(window),
                            bounds,
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_action(move |_: &gpui_base::input::Escape, window, cx| {
                let _ = bubble_escape_owner
                    .update(cx, |view, cx| view.palette_child_escape(id, window, cx));
            })
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    let _ = bubble_key_owner
                        .update(cx, |view, cx| view.palette_child_escape(id, window, cx));
                }
            })
            .capture_action(move |action: &gpui_base::input::Enter, window, cx| {
                if action.secondary || action.shift {
                    return;
                }
                let _ = enter_owner.update(cx, |view, cx| {
                    if view.palette_key(id, "enter", window, cx) {
                        cx.stop_propagation();
                    }
                });
            })
            .capture_action(move |_: &gpui_base::input::Escape, window, cx| {
                let _ = escape_owner.update(cx, |view, cx| {
                    if view.palette_key(id, "escape", window, cx) {
                        cx.stop_propagation();
                    }
                });
            })
            .capture_key_down(move |event, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                let _ = key_owner.update(cx, |view, cx| {
                    if view.palette_key(id, &event.keystroke.key, window, cx) {
                        window.prevent_default();
                        cx.stop_propagation();
                    }
                });
            });
        if !interaction.pointer {
            panel = panel.capture_any_mouse_down(|_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            });
        }
        if count == 0 {
            let mut empty = div()
                .h(px(appearance.row_height as f32))
                .child(gpui::SharedString::from(appearance.empty_label.clone()));
            gpui::Refineable::refine(
                empty.style(),
                &crate::appearance::refinement(&appearance.empty_style, 0),
            );
            panel = panel.child(
                self.palette_slot(node, 2, interaction, window, cx)
                    .unwrap_or_else(|| empty.into_any_element()),
            );
        } else {
            panel = panel.child(
                div()
                    .id("palette-list")
                    .role(gpui::Role::ListBox)
                    .aria_label("Commands")
                    .child(list),
            );
        }
        if let Some(footer) = self.palette_slot(node, 1, interaction, window, cx) {
            panel = panel.child(footer);
        }
        panel = panel
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
        let bounds = self.palettes[&id].bounds.clone();
        let outside = owner;
        let panel = crate::semantics::State {
            identity: None,
            busy: false,
            hidden: false,
            metadata: None,
            live: None,
            element: panel,
            disabled: false,
            read_only: false,
            modal: true,
        };
        let backdrop = div()
            .w(crate::window_frame::content_bounds(window).size.width)
            .h(crate::window_frame::content_bounds(window).size.height)
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000080))
            .occlude()
            .child(panel)
            .on_any_mouse_down(move |event, window, cx| {
                if interaction.pointer && !bounds.get().contains(&event.position) {
                    let _ = outside.update(cx, |view, cx| {
                        view.close_palette(id, PaletteDismissal::OutsidePointer, window, cx)
                    });
                }
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
        let priority = self.focus.borrow().layer(id);
        deferred(super::overlay::ViewportSurface {
            content: backdrop.into_any_element(),
        })
        .with_priority(priority)
        .into_any_element()
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "palette_options_test.rs"]
mod options_tests;
