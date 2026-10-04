//! Native trigger and deferred popup rendering. Rich rows render accepted child
//! nodes on demand; no layout callback crosses into OCaml.
use super::*;
use crate::choice_picker_rows::{Cursor, Navigation, Projection, Row};
use gpui::{Div, Stateful, canvas, deferred};
use gpuio_protocol::choice_picker::{
    Checkmark, Collection, Config, Event as PickerEvent, OpenReason, OpenState, Query, Selection,
};

pub(super) struct Rows {
    pub(super) list: crate::choice_picker_list::State,
    cursor: Cursor,
    query: String,
    config: Arc<Config>,
    geometry: (f64, f64),
    layout_cache: super::choice_picker_layout::Cache,
    #[cfg(all(test, feature = "native-image-tests"))]
    pub(super) rendered: std::collections::BTreeSet<usize>,
}
impl Rows {
    pub(super) fn reconcile(&mut self, config: Arc<Config>, query: String, geometry: (f64, f64)) {
        if !Arc::ptr_eq(&self.config, &config) || self.query != query {
            let projection =
                Arc::new(Projection::new(config.clone(), &query).expect("validated native query"));
            self.cursor.reconcile(&projection, false);
            self.list.replace(projection);
            self.config = config;
            self.query = query;
        }
        if self.geometry != geometry {
            let anchor = self.list.handle().logical_scroll_top();
            self.list = crate::choice_picker_list::State::new(
                self.list.projection().clone(),
                geometry.0 as f32,
                geometry.1 as f32,
            )
            .expect("validated geometry");
            self.list.handle().scroll_to(anchor);
            self.geometry = geometry;
        }
    }
    pub(super) fn invalidate(&self) {
        self.list
            .handle()
            .remeasure_items(0..self.list.projection().len());
    }
}
pub(super) struct Render<'a> {
    pub tree: &'a crate::tree::Tree,
    pub node: &'a crate::tree::Node,
    pub interaction: Interaction,
    pub priority: usize,
}
#[derive(Clone)]
pub(super) enum Action {
    Toggle,
    Activate(String),
    Clear,
    Key(String),
    Tab(bool),
    Close(OpenReason),
}
fn label(config: &Config) -> String {
    match &config.selected {
        Selection::Multiple(ids) if !ids.is_empty() => format!("{} selected", ids.len()),
        Selection::Single(Some(id)) => {
            let find = |items: &[gpuio_protocol::choice_picker::Item]| {
                items
                    .iter()
                    .find(|item| item.id == *id)
                    .map(|item| item.label.clone())
            };
            match &config.options {
                Collection::Flat(items) => find(items),
                Collection::Grouped(groups) => groups.iter().find_map(|g| find(&g.items)),
            }
            .unwrap_or_default()
        }
        Selection::Single(None) | Selection::Multiple(_) => config.placeholder.clone(),
    }
}
impl View {
    /// A nested picker in an interactive footer owns its own keyboard gestures.
    fn picker_owns_focus(&self, node: NodeId, window: &Window, cx: &App) -> bool {
        let Some(mut focused) = self.focus.borrow().focused_node(window, cx) else {
            return false;
        };
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return false;
        };
        loop {
            let Some(current) = tree.get(focused) else {
                return false;
            };
            if current.kind == Kind::ChoicePicker {
                return focused == node;
            }
            let Some(parent) = current.parent else {
                return false;
            };
            focused = parent;
        }
    }
    fn picker_query(&self, node: NodeId, window: &Window, cx: &App) -> Option<Query> {
        let child = self.pickers.get(&node)?.query?;
        Some(Query {
            node: child,
            snapshot: self.editors.get(&child)?.snapshot(window, cx),
        })
    }
    pub(super) fn picker_query_accessibility_owner(
        &self,
        node: NodeId,
        window: &Window,
        cx: &App,
    ) -> Option<gpui::FocusHandle> {
        let query = self.pickers.get(&node)?.query?;
        let editor = self.editors.get(&query)?;
        let focus = editor.focus_handle(cx);
        (focus.is_focused(window) && !editor.is_composing(cx)).then_some(focus)
    }

    fn picker_rows(&mut self, node: NodeId, window: &Window, cx: &App) {
        let query = self
            .picker_query(node, window, cx)
            .map_or_else(String::new, |q| q.snapshot.text);
        let owner = self.pickers.get_mut(&node).expect("mounted picker");
        let config = owner.state.config().clone();
        let p = &owner.presentation;
        let geometry = (p.estimated_row_height, p.overscan);
        if let Some(rows) = &mut owner.render {
            rows.reconcile(config, query, geometry);
        } else {
            let projection =
                Arc::new(Projection::new(config.clone(), &query).expect("validated native query"));
            let mut cursor = Cursor::default();
            cursor.reconcile(&projection, true);
            let list = crate::choice_picker_list::State::new(
                projection,
                geometry.0 as f32,
                geometry.1 as f32,
            )
            .expect("validated geometry");
            owner.render = Some(Rows {
                list,
                cursor,
                query,
                config,
                geometry,
                layout_cache: Default::default(),
                #[cfg(all(test, feature = "native-image-tests"))]
                rendered: Default::default(),
            });
        }
    }
    pub(super) fn picker_action(
        &mut self,
        route: &choice::Route,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.sync_choice_pickers(&[], window, cx);
        if !route.gate.borrow().allows(route.node)
            || !self.session.borrow().tree(self.id).is_some_and(|tree| {
                tree.accepts_handler(route.node, route.handler) && route.revision <= tree.revision()
            })
            || !self.pickers.contains_key(&route.node)
        {
            return false;
        }
        if matches!(&action, Action::Key(_) | Action::Tab(_))
            && (!self.picker_owns_focus(route.node, window, cx)
                || self.focused_input_composing(window, cx))
        {
            return false;
        }
        if matches!(&action, Action::Key(key) if key == "enter" || key == "space")
            && self.pickers[&route.node].clear_focus.is_focused(window)
        {
            return self.picker_action(route, Action::Clear, window, cx);
        }
        // The interactive footer owns its own editing and activation keys.
        // Its dispatch path includes this picker, so capture listeners must not
        // treat every descendant key as an option-navigation gesture.
        if let Action::Key(key) = &action
            && key != "escape"
            && !self
                .buttons
                .get(&route.node)
                .is_some_and(|button| button.focus.is_focused(window))
            && !self.pickers[&route.node]
                .query
                .and_then(|child| self.editors.get(&child))
                .is_some_and(|editor| editor.focus_handle(cx).is_focused(window))
        {
            return false;
        }
        self.picker_rows(route.node, window, cx);
        let query = self.picker_query(route.node, window, cx);
        if query
            .as_ref()
            .is_some_and(|q| q.snapshot.composition.is_some())
            && !matches!(
                &action,
                Action::Close(OpenReason::OutsidePointer | OpenReason::FocusLeft)
            )
        {
            return false;
        }
        let owns_focus = self.picker_owns_focus(route.node, window, cx);
        let toggling = matches!(&action, Action::Toggle);
        let owner = self.pickers.get_mut(&route.node).expect("live owner");
        if owner.presentation.config.disabled {
            return false;
        }
        if matches!(&action, Action::Toggle)
            && let Some(button) = self.buttons.get(&route.node)
        {
            window.focus(&button.focus, cx);
        }
        let restore_focus = !matches!(
            &action,
            Action::Close(OpenReason::OutsidePointer | OpenReason::FocusLeft)
        );
        let before = owner.state.is_open();
        if let Action::Tab(reverse) = action {
            if !before {
                return false;
            }
            self.focus.borrow().traverse_anchored(
                route.node,
                &[
                    self.buttons[&route.node].focus.clone(),
                    owner.clear_focus.clone(),
                ],
                reverse,
                window,
                cx,
            );
            return true;
        }
        let events = match action {
            Action::Tab(_) => unreachable!("Tab handled before picker state mutation"),
            Action::Toggle => owner.state.request_open(!before, OpenReason::Trigger),
            Action::Close(reason) => {
                if !before {
                    return false;
                }
                owner.state.request_open(false, reason)
            }
            Action::Clear => owner.state.clear(false, query.as_ref()),
            Action::Activate(id) => {
                if owner
                    .render
                    .as_ref()
                    .unwrap()
                    .list
                    .projection()
                    .item_row(&id)
                    .is_none()
                {
                    return false;
                }
                owner.state.activate(&id, false, query.as_ref())
            }
            Action::Key(key) => match key.as_str() {
                "escape" if before => owner.state.request_open(false, OpenReason::Escape),
                "enter" | "space" if !before => {
                    owner.state.request_open(true, OpenReason::Keyboard)
                }
                "enter" if before => {
                    let Some(id) = owner
                        .render
                        .as_ref()
                        .unwrap()
                        .cursor
                        .active_id()
                        .map(str::to_owned)
                    else {
                        return false;
                    };
                    owner.state.activate(&id, false, query.as_ref())
                }
                "up" | "down" | "home" | "end" => {
                    if query.is_some() && matches!(key.as_str(), "home" | "end") {
                        return false;
                    }
                    if !before {
                        owner.state.request_open(true, OpenReason::Keyboard)
                    } else {
                        let rows = owner.render.as_mut().unwrap();
                        let navigation = match key.as_str() {
                            "up" => Navigation::Previous,
                            "down" => Navigation::Next,
                            "home" => Navigation::First,
                            _ => Navigation::Last,
                        };
                        rows.cursor.navigate(rows.list.projection(), navigation);
                        rows.list.reveal(&rows.cursor);
                        vec![]
                    }
                }
                _ => return false,
            },
        };
        let after = owner.state.is_open();
        if matches!(
            owner.presentation.config.open_state,
            OpenState::Controlled(_)
        ) && let Some(open) = events.iter().find_map(|event| match event {
            PickerEvent::OpenRequested(open, _) => Some(*open),
            _ => None,
        }) {
            // Programmatic visibility never steals focus. Only a live gesture's
            // eventual acceptance may complete its ordinary managed handoff.
            owner.pending_focus = if restore_focus && (owns_focus || toggling) {
                window.focused(cx).map(|source| {
                    super::choice_picker_host::PendingFocus::new(open, &source, window, cx)
                })
            } else {
                None
            };
        }
        for event in events {
            let event = self.session.borrow().choice_picker_event(
                self.id,
                route.node,
                route.handler,
                route.revision,
                event,
            );
            if let Some(event) = event
                && !self.transport.input(event)
                && self.session.borrow_mut().overload(self.id)
            {
                self.transport.fault(self.id);
            }
        }
        self.sync_choice_pickers(&[], window, cx);
        if !before && after {
            if let Some(child) = self.pickers[&route.node].query
                && let Some(editor) = self.editors.get_mut(&child)
            {
                editor.command(&EditorCommand::Focus, window, cx);
            }
        } else if before
            && !after
            && restore_focus
            && let Some(button) = self.buttons.get(&route.node)
            && self.focus.borrow().allows(route.node)
        {
            window.focus(&button.focus, cx);
        }
        cx.notify();
        true
    }
    pub(super) fn picker_element(
        &mut self,
        mut base: Stateful<Div>,
        render: Render<'_>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Render {
            tree,
            node,
            interaction,
            priority,
        } = render;
        self.visited.insert(node.id);
        self.picker_rows(node.id, window, cx);
        #[cfg(all(test, feature = "native-image-tests"))]
        self.pickers
            .get_mut(&node.id)
            .unwrap()
            .render
            .as_mut()
            .unwrap()
            .rendered
            .clear();
        let owner = &self.pickers[&node.id];
        let p = owner.presentation.clone();
        let open = owner.state.is_open();
        let trigger_bounds = owner.trigger.clone();
        let painted = owner.painted.clone();
        let popup_bounds = owner.popup.clone();
        let rows = owner.render.as_ref().unwrap();
        let projection = rows.list.projection().clone();
        let handle = rows.list.handle().clone();
        let layout_cache = rows.layout_cache.clone();
        let active = rows.cursor.active_id().map(str::to_owned);
        let query_focus = self.picker_query_accessibility_owner(node.id, window, cx);
        let slots = owner.children.clone();
        let route = choice::Route {
            window: self.id,
            node: node.id,
            handler: node.handler.expect("picker observer"),
            revision: tree.revision(),
            session: self.session.clone(),
            gate: self.focus.clone(),
            transport: self.transport.clone(),
        };
        let focus = self.buttons[&node.id].focus.clone();
        if self.pickers[&node.id]
            .focus_subscription
            .as_ref()
            .is_none_or(|(previous, handler, _)| *previous != focus || *handler != route.handler)
        {
            let focus_route = route.clone();
            let subscription = cx.on_focus_out(&focus, window, move |view, _, window, cx| {
                view.picker_action(
                    &focus_route,
                    Action::Close(OpenReason::FocusLeft),
                    window,
                    cx,
                );
            });
            self.pickers.get_mut(&node.id).unwrap().focus_subscription =
                Some((focus, route.handler, subscription));
        }
        let trigger_content = if let Some(id) = slots.trigger {
            self.control_label(tree, id, interaction, p.config.disabled, window, cx)
        } else {
            div()
                .child(gpui::SharedString::from(label(&p.config)))
                .overflow_hidden()
                .into_any_element()
        };
        let click_route = route.clone();
        let mut trigger = div()
            .id("picker-trigger")
            .flex()
            .items_center()
            .justify_between()
            .flex_1()
            .min_w_0()
            .child(trigger_content)
            .child(" ▾");
        if interaction.pointer && !p.config.disabled {
            trigger = trigger.on_click(cx.listener(move |view, _, window, cx| {
                view.picker_action(&click_route, Action::Toggle, window, cx);
                cx.stop_propagation();
            }));
        }
        base = base
            .aria_label(p.config.label.clone())
            .aria_value(label(&p.config))
            .aria_expanded(open)
            .child(trigger)
            .child(
                canvas(
                    move |bounds, _, _| trigger_bounds.set(bounds),
                    move |bounds, _, window, _| {
                        let visible = bounds.intersect(&window.content_mask().bounds);
                        painted.set(visible.size.width > px(0.) && visible.size.height > px(0.));
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        if p.config.clearable {
            let clear_route = route.clone();
            let clear_focus = self.pickers[&node.id].clear_focus.clone();
            let tab_stop = node.tab_order.is_none_or(|order| order.tab_stop) && !p.config.disabled;
            let clear_record = clear_focus.clone();
            let clear_gate = self.focus.clone();
            let clear_node = node.id;
            let trigger_record = self.buttons[&node.id].focus.clone();
            let trigger_bounds = self.pickers[&node.id].trigger.clone();
            let mut clear = div()
                .id("picker-clear")
                .role(gpui::Role::Button)
                .aria_label("Clear selection")
                .child("×")
                .px(px(4.))
                .rounded(px(3.))
                .focus_visible(|style| style.bg(rgba(0x386ac840)))
                .child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let mut gate = clear_gate.borrow_mut();
                            // The host's parent record paints after child controls.
                            // Establish trigger-before-clear order for its managed
                            // traversal too (e.g. clipping or a modal focus scope).
                            gate.record(
                                clear_node,
                                trigger_record.clone(),
                                tab_stop,
                                trigger_record.is_focused(window),
                                trigger_bounds.get(),
                            );
                            gate.record_part(
                                clear_node,
                                1,
                                super::focus::Target {
                                    handle: clear_record.clone(),
                                    tab_stop,
                                    bounds,
                                },
                                clear_record.is_focused(window),
                            );
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                );
            if !p.config.disabled {
                let handle = clear_focus.clone();
                let gate = self.focus.clone();
                clear = clear
                    .track_focus(&clear_focus.tab_stop(tab_stop))
                    .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                        if gate.borrow().can_focus(&handle, window) {
                            gate.borrow().request_reveal();
                            window.focus(&handle, cx);
                        }
                    });
            }
            if !p.config.disabled {
                let route = route.clone();
                let owner = cx.entity().downgrade();
                clear =
                    clear.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                        let _ = owner.update(cx, |view, cx| {
                            view.picker_action(&route, Action::Clear, window, cx);
                        });
                    });
            }
            if interaction.pointer && !p.config.disabled {
                clear = clear
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                        window.prevent_default()
                    })
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.picker_action(&clear_route, Action::Clear, window, cx);
                        cx.stop_propagation();
                    }));
            }
            base = base.child(super::choice_picker_semantics::clear(
                clear,
                p.config.disabled,
            ));
        }
        let tab_route = route.clone();
        base = base.capture_key_down(cx.listener(
            move |view, event: &gpui::KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                if event.keystroke.key == "tab"
                    && !modifiers.control
                    && !modifiers.platform
                    && !modifiers.alt
                    && view.picker_action(&tab_route, Action::Tab(modifiers.shift), window, cx)
                {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            },
        ));
        let key_route = route.clone();
        base = base.capture_key_down(cx.listener(
            move |view, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                if view.picker_action(
                    &key_route,
                    Action::Key(event.keystroke.key.clone()),
                    window,
                    cx,
                ) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            },
        ));
        // Matched editor bindings dispatch actions before raw key listeners.
        // Keep IME ownership: picker_action declines while composing.
        let forward_route = route.clone();
        base = base.capture_action(cx.listener(
            move |view, _: &gpui_base::input::IndentInline, window, cx| {
                if view
                    .pickers
                    .get(&forward_route.node)
                    .and_then(|owner| owner.query)
                    .and_then(|query| view.editors.get(&query))
                    .is_some_and(|editor| editor.focus_handle(cx).is_focused(window))
                    && view.picker_action(&forward_route, Action::Tab(false), window, cx)
                {
                    cx.stop_propagation();
                }
            },
        ));
        let reverse_route = route.clone();
        base = base.capture_action(cx.listener(
            move |view, _: &gpui_base::input::OutdentInline, window, cx| {
                if view
                    .pickers
                    .get(&reverse_route.node)
                    .and_then(|owner| owner.query)
                    .and_then(|query| view.editors.get(&query))
                    .is_some_and(|editor| editor.focus_handle(cx).is_focused(window))
                    && view.picker_action(&reverse_route, Action::Tab(true), window, cx)
                {
                    cx.stop_propagation();
                }
            },
        ));
        let enter_route = route.clone();
        base = base.capture_action(cx.listener(
            move |view, event: &gpui_base::input::Enter, window, cx| {
                if !event.secondary
                    && !event.shift
                    && view.picker_action(&enter_route, Action::Key("enter".into()), window, cx)
                {
                    cx.stop_propagation();
                }
            },
        ));
        let escape_route = route.clone();
        base = base.capture_action(cx.listener(
            move |view, _: &gpui_base::input::Escape, window, cx| {
                if view.picker_action(&escape_route, Action::Key("escape".into()), window, cx) {
                    cx.stop_propagation();
                }
            },
        ));
        if !p.config.disabled {
            let ax_route = route.clone();
            let ax_owner = cx.entity().downgrade();
            base = base.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                let _ = ax_owner.update(cx, |view, cx| {
                    view.picker_action(&ax_route, Action::Toggle, window, cx);
                });
            });
        }
        if !open {
            return base;
        }
        self.focus
            .borrow_mut()
            .surface(node.id, popup_bounds.clone());
        let weak = cx.entity().downgrade();
        let shared = self.session.clone();
        let popup_owner = node.id;
        let wid = self.id;
        let expected = p.clone();
        let row_slots = slots.clone();
        let row_route = route.clone();
        let count = projection.len();
        let item_count = projection.item_count();
        let groups = super::choice_picker_semantics::Groups::new(projection.clone());
        let row_groups = groups.clone();
        let list = gpui::list(handle.clone(), move |index, window, cx| {
            // GPUI owns ListState's borrow here. Use only the immutable projection.
            weak.update(cx, |view, cx| {
                #[cfg(all(test, feature = "native-image-tests"))]
                view.pickers
                    .get_mut(&popup_owner)
                    .unwrap()
                    .render
                    .as_mut()
                    .unwrap()
                    .rendered
                    .insert(index);
                let session = shared.borrow();
                let Some(tree) = session.tree(wid) else {
                    return div().into_any_element();
                };
                if tree
                    .get(popup_owner)
                    .and_then(|n| n.choice_picker.as_ref())
                    .is_none_or(|p| !Arc::ptr_eq(p, &expected))
                {
                    return div().into_any_element();
                }
                let Some(row) = projection.row(index) else {
                    return div().into_any_element();
                };
                let mut element = div()
                    .id(super::choice_picker_semantics::row_id(row.key()))
                    .w_full()
                    .px(px(8.))
                    .py(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(6.));
                match row {
                    Row::Header(group) => {
                        gpui::Refineable::refine(
                            element.style(),
                            &crate::appearance::refinement(&expected.header_style, 0),
                        );
                        element = element.child(div().min_w_0().flex_1().child(
                            if let Some(id) = row_slots.groups.get(&group.id) {
                                view.control_label(tree, *id, interaction, false, window, cx)
                            } else {
                                div()
                                    .child(gpui::SharedString::from(group.label.clone()))
                                    .into_any_element()
                            },
                        ));
                    }
                    Row::Item(item) => {
                        element = super::choice_picker_semantics::membership(
                            element,
                            projection.membership(index).expect("option membership"),
                        );
                        let selected = projection.is_selected(index);
                        let focused = active.as_deref() == Some(&item.id);
                        element = element
                            .role(gpui::Role::ListBoxOption)
                            .aria_label(item.label.clone())
                            .aria_selected(selected);
                        gpui::Refineable::refine(
                            element.style(),
                            &crate::appearance::refinement(&expected.option_style, 0),
                        );
                        if selected {
                            gpui::Refineable::refine(
                                element.style(),
                                &crate::appearance::refinement(&expected.option_style, 7),
                            );
                        }
                        if focused {
                            element = element.bg(rgba(0x386ac840));
                            element = if let Some(owner) = &query_focus {
                                element.aria_active_descendant_for(owner)
                            } else {
                                element.aria_active_descendant()
                            };
                            gpui::Refineable::refine(
                                element.style(),
                                &crate::appearance::refinement(&expected.option_style, 1),
                            );
                        }
                        let custom = row_slots.options.get(&item.id).copied();
                        if custom.is_none_or(|(_, mark)| mark == Checkmark::Native) {
                            element = element.child(
                                div().w(px(14.)).flex_shrink_0().child(if selected {
                                    "✓"
                                } else {
                                    ""
                                }),
                            );
                        }
                        element = element.child(div().min_w_0().flex_1().child(
                            if let Some((id, _)) = custom {
                                view.control_label(tree, id, interaction, item.disabled, window, cx)
                            } else {
                                div()
                                    .child(gpui::SharedString::from(item.label.clone()))
                                    .into_any_element()
                            },
                        ));
                        if !item.disabled {
                            let route = row_route.clone();
                            let selected = item.id.clone();
                            let owner = cx.entity().downgrade();
                            element = element.on_a11y_action(
                                gpui::AccessibleAction::Click,
                                move |_, window, cx| {
                                    let _ = owner.update(cx, |view, cx| {
                                        view.picker_action(
                                            &route,
                                            Action::Activate(selected.clone()),
                                            window,
                                            cx,
                                        );
                                    });
                                },
                            );
                        }
                        if item.disabled {
                            element = element.opacity(0.5);
                            gpui::Refineable::refine(
                                element.style(),
                                &crate::appearance::refinement(&expected.option_style, 6),
                            );
                        } else if interaction.pointer {
                            let route = row_route.clone();
                            let id = item.id.clone();
                            let hovered = crate::appearance::refinement(&expected.option_style, 2);
                            let pressed = crate::appearance::refinement(&expected.option_style, 3);
                            element = element
                                .hover(move |_| hovered)
                                .active(move |_| pressed)
                                .cursor_pointer()
                                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                                    window.prevent_default()
                                })
                                .on_click(cx.listener(move |view, _, window, cx| {
                                    view.picker_action(
                                        &route,
                                        Action::Activate(id.clone()),
                                        window,
                                        cx,
                                    );
                                    cx.stop_propagation();
                                }));
                        }
                    }
                }
                match row {
                    Row::Item(item) => super::choice_picker_semantics::option(
                        element,
                        projection.is_selected(index),
                        item.disabled,
                    )
                    .groups(row_groups.clone())
                    .row(index)
                    .into_any_element(),
                    Row::Header(_) => super::choice_picker_semantics::header(element)
                        .groups(row_groups.clone())
                        .row(index)
                        .into_any_element(),
                }
            })
            .unwrap_or_else(|_| div().into_any_element())
        })
        .w_full()
        .min_h_0()
        .flex_shrink_1()
        .h(px((count.max(1) as f64 * p.estimated_row_height)
            .min(p.max_height)
            .min(f64::from(crate::window_frame::content_bounds(window).size.height) - 96.)
            .max(1.) as f32));
        let list = super::choice_picker_layout::observe(list, handle, layout_cache);
        let dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        let mut popup = div()
            .id("picker-popup")
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(4.))
            .occlude()
            .bg(rgba(if dark { 0x242424ff } else { 0xffffffff }))
            .text_color(rgba(if dark { 0xf0f0f0ff } else { 0x202020ff }))
            .border_1()
            .border_color(rgba(0x80808080))
            .rounded(px(6.))
            .w(px(p
                .popup_width
                .min(f64::from(crate::window_frame::content_bounds(window).size.width) - 16.)
                .max(1.) as f32))
            .max_h(px(p.max_height as f32).min(
                (crate::window_frame::content_bounds(window).size.height - px(16.)).max(px(1.)),
            ))
            .overflow_hidden();
        gpui::Refineable::refine(
            popup.style(),
            &crate::appearance::refinement(&p.popup_style, 0),
        );
        if interaction.pointer {
            let hovered = crate::appearance::refinement(&p.popup_style, 2);
            popup = popup.hover(move |_| hovered);
        }
        if let Some(query) = slots.query {
            popup = popup.child(div().flex_shrink_0().child(self.element(
                tree,
                query,
                interaction,
                window,
                cx,
            )));
        }
        if item_count == 0 {
            let mut empty = div().p(px(8.));
            gpui::Refineable::refine(
                empty.style(),
                &crate::appearance::refinement(&p.empty_style, 0),
            );
            popup = popup.child(empty.child(if let Some(id) = slots.empty {
                self.control_label(tree, id, interaction, false, window, cx)
            } else {
                div()
                    .child(gpui::SharedString::from(p.empty_label.clone()))
                    .into_any_element()
            }));
        } else {
            popup = popup.child(
                super::choice_picker_semantics::listbox(
                    div()
                        .id("picker-options")
                        .flex()
                        .flex_col()
                        .min_h_0()
                        .flex_shrink_1()
                        .role(gpui::Role::ListBox)
                        .aria_label(p.config.label.clone())
                        .child(list),
                    matches!(p.config.selected, Selection::Multiple(_)),
                )
                .groups(groups),
            );
        }
        if let Some(footer) = slots.footer {
            popup = popup.child(div().flex_shrink_0().child(self.element(
                tree,
                footer,
                interaction,
                window,
                cx,
            )));
        }
        let outside = route.clone();
        let bounds = self.pickers[&node.id].trigger.clone();
        popup = popup
            .on_mouse_down_out(cx.listener(
                move |view, event: &gpui::MouseDownEvent, window, cx| {
                    if !bounds.get().contains(&event.position) {
                        view.picker_action(
                            &outside,
                            Action::Close(OpenReason::OutsidePointer),
                            window,
                            cx,
                        );
                    }
                },
            ))
            .child(
                canvas(
                    move |bounds, _, _| popup_bounds.set(bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        base.child(
            deferred(super::popup::Surface {
                geometry: None,
                trigger: self.pickers[&node.id].trigger.clone(),
                placement: Placement::default(),
                content: popup.into_any_element(),
            })
            .with_priority(priority),
        )
    }
}
