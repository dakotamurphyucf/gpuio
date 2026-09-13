//! A one-shot, native-managed command chooser. The query is not an application
//! editor: native edit commands restore their original document target on close.
use super::{Interaction, View, command::Route};
use gpui::{
    App, AppContext, Bounds, Context, Entity, EntityInputHandler, Focusable, Pixels, Subscription,
    UniformListScrollHandle, Window, canvas, deferred, div, prelude::*, px, rgba,
};
use gpui_base::input::InputState;
use gpuio_protocol::{NodeId, v1::*};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
struct Row {
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
    semantic_config: Rc<RefCell<Arc<PaletteConfig>>>,
    pub(super) closed: bool,
    editor: Option<NodeId>,
    rows: Vec<Row>,
    selected: Option<String>,
    revealed: Option<Reveal>,
    scroll: UniformListScrollHandle,
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
            self.scroll.0.borrow().base_handle.offset(),
            self.row_bounds.borrow().len(),
        )
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn row_bounds(&self, command: &str) -> Bounds<Pixels> {
        self.row_bounds.borrow()[command]
    }
    fn composing(&self, cx: &App) -> bool {
        self.query.read(cx).bridge_composition().is_some()
    }
}
fn matches_query(label: &str, id: &str, query: &str) -> bool {
    let label = label.to_lowercase();
    let id = id.to_lowercase();
    query
        .split_whitespace()
        .all(|term| label.contains(term) || id.contains(term))
}
impl View {
    pub(super) fn sync_palettes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                    nodes.push((id, config.clone()));
                }
                stack.extend(node.children.iter().copied());
            }
            nodes
        };
        for (id, config) in nodes {
            if let Some(state) = self.palettes.get_mut(&id) {
                if state.config != config {
                    state.query.update(cx, |query, cx| {
                        query.set_placeholder(config.placeholder.clone(), window, cx)
                    });
                    *state.semantic_config.borrow_mut() = config.clone();
                    state.config = config;
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
            let placeholder = config.placeholder.clone();
            query.update(cx, |state, cx| {
                state.set_placeholder(placeholder, window, cx);
                state.set_submit_on_enter(false, cx);
                state.set_context_menu_enabled(false);
                state.set_bridge_decorator(Rc::new(move |element, state, _, cx| {
                    let input = cx.weak_entity();
                    let gate = gate.clone();
                    let focus_gate = gate.clone();
                    let focus = state.focus_handle(cx);
                    element
                        .role(gpui::Role::EditableComboBox)
                        .aria_label(semantic_labels.borrow().label.clone())
                        .aria_placeholder(semantic_labels.borrow().placeholder.clone())
                        .aria_value(state.value())
                        .aria_expanded(true)
                        .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                            if focus_gate.borrow().allows(id) && focus_gate.borrow().visible(id) {
                                window.focus(&focus, cx);
                            }
                        })
                        .on_a11y_action(
                            gpui::AccessibleAction::SetValue,
                            move |data, window, cx| {
                                if !gate.borrow().allows(id) || !gate.borrow().visible(id) {
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
            let subscription = cx.observe_in(&query, window, |_, _, _, cx| cx.notify());
            self.palettes.insert(
                id,
                State {
                    query,
                    config,
                    semantic_config,
                    closed: false,
                    editor,
                    rows: vec![],
                    selected: None,
                    revealed: None,
                    scroll: Default::default(),
                    bounds: Default::default(),
                    row_bounds: Default::default(),
                    _subscription: subscription,
                },
            );
        }
    }
    fn refresh_palette(&mut self, id: NodeId, cx: &App) -> String {
        let Some(state) = self.palettes.get(&id) else {
            return String::new();
        };
        let query = state.query.read(cx).value().to_lowercase();
        let rows = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return query;
            };
            let Some(config) = tree.get(id).and_then(|node| node.palette.as_ref()) else {
                return query;
            };
            config
                .commands
                .iter()
                .filter_map(|command| {
                    let (scope, command) = tree.command(id, command)?;
                    matches_query(&command.label, &command.id, &query).then(|| Row {
                        route: Route::new(tree, scope, command, CommandSource::Palette(id)),
                        enabled: self.palette_available(id, command, cx),
                    })
                })
                .collect::<Vec<_>>()
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
        state.rows = rows;
        query
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
            .filter(|(id, state)| !state.closed && !self.focus.borrow().visible(**id))
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
        if !self.focus.borrow().top_overlay(id) {
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
        if !self.focus.borrow().visible(id) {
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
            let index = rows
                .iter()
                .position(|row| row.route.config.id == *selected)
                .unwrap();
            let reveal = Reveal {
                command: selected.clone(),
                query: query_text,
                row_height: appearance.row_height,
                max_rows: appearance.max_visible_rows,
                index,
                viewport: window.viewport_size(),
            };
            if state.revealed.as_ref() != Some(&reveal) {
                state
                    .scroll
                    .scroll_to_item(index, gpui::ScrollStrategy::Nearest);
                state.revealed = Some(reveal);
            }
        }
        state.row_bounds.borrow_mut().clear();
        let bounds = state.bounds.clone();
        let row_bounds = state.row_bounds.clone();
        let query = state.query.clone();
        let selected = state.selected.clone();
        let scroll = state.scroll.clone();
        let scope_focus = self
            .focus
            .borrow()
            .handle(id)
            .expect("mounted palette scope");
        let query_focus = query.read(cx).focus_handle(cx).tab_stop(true);
        let gate = self.focus.clone();
        let record_focus = query_focus.clone();
        let owner = cx.weak_entity();
        let style = appearance.clone();
        let count = rows.len();
        let list = gpui::uniform_list(("palette-results", id.slot()), count, move |range, _, _| {
            range
                .map(|index| {
                    let row = rows[index].clone();
                    let active = Some(&row.route.config.id) == selected.as_ref();
                    let mut item = div()
                        .id(gpui::SharedString::from(row.route.config.id.clone()))
                        .h(px(style.row_height as f32))
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
                        .child(gpui::SharedString::from(row.route.config.label.clone()));
                    if row.enabled {
                        let ax_owner = owner.clone();
                        let ax_route = row.route.clone();
                        item = item.on_a11y_action(
                            gpui::AccessibleAction::Click,
                            move |_, window, cx| {
                                let _ = ax_owner.update(cx, |view, cx| {
                                    view.select_palette(id, &ax_route, window, cx)
                                });
                            },
                        );
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
                        element: item,
                        disabled: !row.enabled,
                        read_only: false,
                        modal: false,
                    }
                    .into_any_element()
                })
                .collect()
        })
        .track_scroll(&scroll)
        .h(px(
            (count.min(appearance.max_visible_rows as usize).max(1) as f64 * appearance.row_height)
                as f32,
        )
        .min((window.viewport_size().height - px(120.)).max(px(1.))))
        .w_full();
        let owner = cx.weak_entity();
        let escape_owner = owner.clone();
        let enter_owner = owner.clone();
        let key_owner = owner.clone();
        let width = px(appearance.popup_width as f32)
            .min((window.viewport_size().width - px(32.)).max(px(1.)));
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
        panel = panel
            .child(query.clone())
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
            panel = panel.child(empty);
        } else {
            panel = panel.child(
                div()
                    .id("palette-list")
                    .role(gpui::Role::ListBox)
                    .aria_label("Commands")
                    .child(list),
            );
        }
        panel = panel
            .child(
                canvas(
                    move |rect, _, _| bounds.set(rect),
                    move |_, _, window, _| {
                        gate.borrow_mut().record(
                            id,
                            record_focus.clone(),
                            true,
                            record_focus.is_focused(window),
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
        let bounds = self.palettes[&id].bounds.clone();
        let outside = owner;
        let panel = crate::semantics::State {
            element: panel,
            disabled: false,
            read_only: false,
            modal: true,
        };
        let backdrop = div()
            .w(window.viewport_size().width)
            .h(window.viewport_size().height)
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
