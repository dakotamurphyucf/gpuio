//! Native-managed menu surfaces. Command metadata remains in the retained tree;
//! transient navigation and focus never roundtrip through OCaml.
use super::{Interaction, View, command::Route};
use gpui::{
    App, Bounds, Context, FocusHandle, Pixels, UniformListScrollHandle, WeakFocusHandle, Window,
    canvas, deferred, div, prelude::*, px, rgba,
};
use gpuio_protocol::{NodeId, v1::*};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};

#[cfg(target_os = "macos")]
#[path = "menu_popup_macos.rs"]
mod popup;
#[cfg(target_os = "macos")]
#[path = "menu_popup_host.rs"]
mod popup_host;
#[cfg(target_os = "macos")]
#[path = "menu_popup_icon.rs"]
mod popup_icon;

#[path = "menu_command_host.rs"]
mod command_host;

#[cfg(all(target_os = "macos", feature = "native-tests"))]
#[path = "menu_popup_queue_test.rs"]
pub(crate) mod queued_test;

type Geometry = Rc<Cell<Bounds<Pixels>>>;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_menu_test.rs"]
mod editor_menu_tests;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "menu_observation_test.rs"]
mod observation_tests;
pub(super) struct State {
    #[cfg(target_os = "macos")]
    popup: Option<popup::Owner>,
    config: Arc<MenuConfig>,
    pub(super) focus: FocusHandle,
    restore: Option<FocusHandle>,
    editor_target: Option<(NodeId, WeakFocusHandle)>,
    context_position: Option<gpui::Point<Pixels>>,
    path: Vec<usize>,
    selected: Vec<Option<usize>>,
    revealed: Vec<Option<usize>>,
    geometry: Option<(f64, i64, gpui::Size<Pixels>)>,
    triggers: Vec<Geometry>,
    panels: Vec<Geometry>,
    rows: Rc<RefCell<BTreeMap<(usize, usize), Geometry>>>,
    scrolls: Vec<UniformListScrollHandle>,
    search: super::typeahead::Search,
    observed: Option<(gpuio_protocol::HandlerId, bool)>,
}
impl State {
    fn new(config: Arc<MenuConfig>, cx: &mut App) -> Self {
        let trigger_count = config.menus.len();
        Self {
            #[cfg(target_os = "macos")]
            popup: None,
            config,
            focus: cx.focus_handle(),
            restore: None,
            editor_target: None,
            context_position: None,
            path: vec![],
            selected: vec![None; 8],
            revealed: vec![None; 8],
            geometry: None,
            triggers: (0..trigger_count).map(|_| Default::default()).collect(),
            panels: Vec::new(),
            rows: Default::default(),
            scrolls: Vec::new(),
            search: Default::default(),
            observed: None,
        }
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn test_state(&self) -> (Vec<usize>, Vec<Option<usize>>) {
        (self.path.clone(), self.selected[..self.path.len()].to_vec())
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn test_panel(&self, depth: usize) -> Bounds<Pixels> {
        self.panels[depth].get()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn test_scroll(&self, depth: usize) -> (usize, usize) {
        let row_height = px(self.geometry.expect("rendered menu").0 as f32);
        let offset = self.scrolls[depth].0.borrow().base_handle.offset();
        (
            (-offset.y / row_height).floor() as usize,
            self.rows.borrow().len(),
        )
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn test_scroll_diagnostics(&self, depth: usize) -> String {
        let scroll = self.scrolls[depth].0.borrow();
        format!(
            "offset={:?}, max={:?}, item={:?}, selected={:?}, revealed={:?}",
            scroll.base_handle.offset(),
            scroll.base_handle.max_offset(),
            scroll.last_item_size,
            self.selected[depth],
            self.revealed[depth]
        )
    }
    fn menu(&self, depth: usize) -> Option<&MenuDefinition> {
        let mut menu = self.config.menus.get(*self.path.first()?)?;
        for index in self.path.iter().take(depth + 1).skip(1) {
            let MenuItem::Submenu(child) = menu.items.get(*index)? else {
                return None;
            };
            menu = child;
        }
        Some(menu)
    }
    fn prepare_levels(&mut self) {
        self.panels.resize_with(self.path.len(), Default::default);
        self.scrolls.resize_with(self.path.len(), Default::default);
    }
    fn tracking(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.popup.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    fn close(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.popup = None;
        }
        self.path.clear();
        self.editor_target = None;
        self.panels.clear();
        self.scrolls.clear();
        self.rows.borrow_mut().clear();
        self.search.clear();
    }
    fn inside(&self, position: gpui::Point<Pixels>) -> bool {
        self.path
            .first()
            .is_some_and(|index| self.triggers[*index].get().contains(&position))
            || self
                .panels
                .iter()
                .take(self.path.len())
                .any(|bounds| bounds.get().contains(&position))
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Command,
    Submenu,
    Separator,
    Label,
}
#[derive(Clone)]
struct Row {
    label: String,
    enabled: bool,
    checked: Option<bool>,
    route: Option<Route>,
    kind: RowKind,
}
struct Hit<'a> {
    id: NodeId,
    path: &'a [usize],
    index: usize,
    row: &'a Row,
    expected: &'a MenuConfig,
}
struct Panel<'a> {
    id: NodeId,
    path: &'a [usize],
    label: &'a str,
    rows: Vec<Row>,
    state: Rc<RefCell<State>>,
    appearance: Arc<ChoiceAppearance>,
    pointer: bool,
    content: Option<MenuContent>,
}
type MenuContent = Rc<dyn Fn(usize, bool, &mut Window, &mut App) -> Option<gpui::AnyElement>>;
impl View {
    fn menu_content(
        &self,
        node: &crate::tree::Node,
        path: &[usize],
        interaction: Interaction,
        rows: &[Row],
        cx: &Context<Self>,
    ) -> Option<MenuContent> {
        let expected = node.menu.clone()?;
        let skip = usize::from(expected.presentation.is_context());
        if node.children.len() == skip {
            return None;
        }
        let indices = expected.row_content_indices(path)?;
        // Reuse the panel's already resolved row labels. Resolving every command
        // again for each visible icon would multiply work by the visible rows.
        let labels: Option<Vec<_>> = (expected.presentation == MenuPresentation::PlatformContext)
            .then(|| rows.iter().map(|row| row.label.clone()).collect());
        let slots = node.children.clone();
        let path = path.to_vec();
        let id = node.id;
        let wid = self.id;
        let session = self.session.clone();
        let weak = cx.weak_entity();
        Some(Rc::new(move |index, disabled, window, cx| {
            weak.update(cx, |view, cx| {
                let session = session.borrow();
                let tree = session.tree(wid)?;
                let current = tree.get(id)?;
                if current
                    .menu
                    .as_ref()
                    .is_none_or(|menu| !Arc::ptr_eq(menu, &expected))
                    || !Arc::ptr_eq(&current.children, &slots)
                    || view.menus.get(&id)?.borrow().path.get(..path.len()) != Some(path.as_slice())
                {
                    return None;
                }
                let slot = tree.get(*slots.get(skip + indices.get(index)?)?)?;
                let content = *slot.children.first()?;
                let icon = view.control_label(tree, content, interaction, disabled, window, cx);
                if let Some(labels) = &labels {
                    let label = labels.get(index)?.clone();
                    Some(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(icon)
                            .child(label)
                            .into_any_element(),
                    )
                } else {
                    Some(icon)
                }
            })
            .ok()
            .flatten()
        }))
    }
    fn editor_menu_target(
        &self,
        tree: &crate::tree::Tree,
        id: NodeId,
        cx: &App,
    ) -> Option<(NodeId, WeakFocusHandle)> {
        let node = tree.get(id)?;
        if node.menu.as_ref()?.presentation != MenuPresentation::EditorContext {
            return None;
        }
        let target = *node.children.first()?;
        let editor = self.editors.get(&target)?;
        if !self.focus.borrow().allows(target)
            || !self.focus.borrow().visible(target)
            || editor.is_composing(cx)
        {
            return None;
        }
        Some((target, editor.focus_handle(cx).downgrade()))
    }
    pub(super) fn split_menu_open(
        &self,
        tree: &crate::tree::Tree,
        id: NodeId,
        window: &Window,
    ) -> bool {
        let Some(config) = tree.get(id).and_then(|node| node.menu.as_ref()) else {
            return false;
        };
        let Some(state) = self.menus.get(&id) else {
            return false;
        };
        let state = state.borrow();
        let focus = self.focus.borrow();
        state.config == *config
            && !state.path.is_empty()
            && state.focus.is_focused(window)
            && focus.visible(id)
            && focus.allows(id)
            && !focus.disabled(id)
            && config.menus.first().is_some_and(|menu| !menu.disabled)
    }

    /// Retire native leases when their retained owner becomes ineligible, even
    /// when an occluded window cannot paint. Do not restore focus into that owner.
    pub(super) fn retire_ineligible_menus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let retired: Vec<_> = self
            .menus
            .iter()
            .filter_map(|(id, state)| {
                let state = state.borrow();
                ((!state.path.is_empty() || state.tracking()) && !self.focus.borrow().allows(*id))
                    .then_some(*id)
            })
            .collect();
        for id in retired {
            self.close_menu(id, false, window, cx);
        }
    }

    pub(super) fn retire_unvisited_menus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let retired: Vec<_> = self
            .menus
            .keys()
            .copied()
            .filter(|id| !self.visited.contains(id))
            .collect();
        for id in retired {
            self.menus[&id].borrow_mut().close();
            // A retained node that was culled still has a subscriber. An actual
            // tree removal has none and publish drops its final observation.
            self.publish_menu_observation(id, window, cx);
        }
        self.menus.retain(|id, _| self.visited.contains(id));
    }

    fn publish_menu_observation(
        &mut self,
        id: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let event = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return;
            };
            let Some(node) = tree.get(id) else {
                return;
            };
            let Some(config) = &node.menu else {
                return;
            };
            let Some(state) = self.menus.get(&id) else {
                return;
            };
            let mut state = state.borrow_mut();
            if &state.config != config {
                return;
            }
            let observed = node
                .handler
                .map(|handler| (handler, !state.path.is_empty() || state.tracking()));
            if state.observed == observed {
                return;
            }
            state.observed = observed;
            observed.and_then(|(handler, open)| {
                session.menu_open_changed(self.id, id, handler, tree.revision(), open)
            })
        };
        if let Some(event) = event
            && !self.transport.input(event)
        {
            // Render may hold a shared Session borrow. Fault handling must run
            // after it ends, never reentrantly mutate that borrow on overflow.
            let session = self.session.clone();
            let transport = self.transport.clone();
            let window_id = self.id;
            window.defer(cx, move |_, _| {
                if session.borrow_mut().overload(window_id) {
                    transport.fault(window_id);
                }
            });
        }
    }
    fn menu_rows(
        &self,
        tree: &crate::tree::Tree,
        id: NodeId,
        menu: &MenuDefinition,
        window: &Window,
        cx: &App,
    ) -> Vec<Row> {
        menu.items
            .iter()
            .map(|item| match item {
                MenuItem::Command(command) => {
                    let route = tree.command(id, command).and_then(|(scope, config)| {
                        let route = Route::new(tree, scope, config, CommandSource::Menu(id));
                        if tree.get(id)?.menu.as_ref()?.presentation
                            == MenuPresentation::EditorContext
                        {
                            self.menus
                                .get(&id)?
                                .borrow()
                                .editor_target
                                .clone()
                                .map(|target| route.bind_editor(target))
                        } else {
                            Some(route)
                        }
                    });
                    Row {
                        label: route
                            .as_ref()
                            .map_or_else(|| command.clone(), |route| route.config.label.clone()),
                        enabled: !menu.disabled
                            && route.as_ref().is_some_and(|route| {
                                self.command_route_available(route, window, cx)
                            }),
                        checked: route.as_ref().and_then(|route| route.config.checked),
                        route,
                        kind: RowKind::Command,
                    }
                }
                MenuItem::Submenu(menu) => Row {
                    label: menu.label.clone(),
                    enabled: !menu.disabled,
                    checked: None,
                    route: None,
                    kind: RowKind::Submenu,
                },
                MenuItem::Label(label) => Row {
                    label: label.clone(),
                    enabled: false,
                    checked: None,
                    route: None,
                    kind: RowKind::Label,
                },
                MenuItem::Separator => Row {
                    label: String::new(),
                    enabled: false,
                    checked: None,
                    route: None,
                    kind: RowKind::Separator,
                },
            })
            .collect()
    }
    fn close_menu(
        &mut self,
        id: NodeId,
        restore: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.menus.get(&id) else {
            return;
        };
        let mut state = state.borrow_mut();
        let was_open = !state.path.is_empty();
        state.close();
        if was_open
            && restore
            && state.focus.is_focused(window)
            && let Some(focus) = state.restore.take()
            && self.focus.borrow().can_focus(&focus, window)
        {
            window.focus(&focus, cx);
        }
        drop(state);
        self.publish_menu_observation(id, window, cx);
        if was_open {
            cx.notify();
        }
    }
    fn open_menu(
        &mut self,
        id: NodeId,
        index: usize,
        position: Option<gpui::Point<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.focus.borrow().interactive(id) || !self.focus.borrow().allows(id) {
            return;
        }
        let Some(state) = self.menus.get(&id).cloned() else {
            return;
        };
        let current = self
            .session
            .borrow()
            .tree(self.id)
            .and_then(|tree| tree.get(id))
            .and_then(|node| node.menu.as_ref())
            .is_some_and(|config| config == &state.borrow().config);
        if !current {
            return;
        }
        #[cfg(target_os = "macos")]
        if state.borrow().config.presentation == MenuPresentation::PlatformContext {
            let _ = self.open_platform_popup(id, state.clone(), position, window, cx);
            return;
        }
        let editor_target = if state.borrow().config.presentation == MenuPresentation::EditorContext
        {
            let session = self.session.borrow();
            let Some(target) = session
                .tree(self.id)
                .and_then(|tree| self.editor_menu_target(tree, id, cx))
            else {
                return;
            };
            Some(target)
        } else {
            None
        };
        if state
            .borrow()
            .config
            .menus
            .get(index)
            .is_none_or(|menu| menu.disabled)
        {
            return;
        }
        let others: Vec<_> = self
            .menus
            .keys()
            .copied()
            .filter(|other| *other != id)
            .collect();
        for other in others {
            self.close_menu(other, false, window, cx);
        }
        let mut state = state.borrow_mut();
        if state.path.is_empty() {
            state.restore = editor_target
                .as_ref()
                .and_then(|(_, focus)| focus.upgrade())
                .or_else(|| window.focused(cx));
        }
        state.editor_target = editor_target;
        state.context_position = position;
        if let Some(position) = position {
            state.triggers[index].set(Bounds::new(position, gpui::size(px(0.), px(0.))));
        }
        state.path = vec![index];
        state.prepare_levels();
        state.selected.fill(None);
        state.revealed.fill(None);
        state.search.clear();
        window.focus(&state.focus, cx);
        drop(state);
        self.publish_menu_observation(id, window, cx);
        cx.notify();
    }
    fn activate_menu_row(&mut self, hit: Hit<'_>, window: &mut Window, cx: &mut Context<Self>) {
        let Hit {
            id,
            path,
            index,
            row,
            expected,
        } = hit;
        if !row.enabled || !self.focus.borrow().allows(id) || !self.focus.borrow().visible(id) {
            return;
        }
        let Some(state) = self.menus.get(&id).cloned() else {
            return;
        };
        {
            let mut state = state.borrow_mut();
            if state.config.as_ref() != expected || !state.path.starts_with(path) {
                return;
            }
            let depth = path.len() - 1;
            state.path.truncate(path.len());
            state.selected[depth] = Some(index);
            if row.kind == RowKind::Submenu {
                state.path.push(index);
                state.prepare_levels();
                state.selected[depth + 1] = None;
                state.search.clear();
                cx.notify();
                return;
            }
        }
        if let Some(route) = &row.route {
            // Restore the editing context before native edit actions; ordinary
            // callbacks also leave focus where it was before a context menu.
            self.close_menu(id, true, window, cx);
            self.invoke_command(route, window, cx);
        }
    }
    fn menu_key(
        &mut self,
        id: NodeId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.menus.get(&id).cloned() else {
            return;
        };
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let context = state.borrow().config.presentation.is_context();
        let open = !state.borrow().path.is_empty();
        if context
            && key == "f10"
            && modifiers.shift
            && !modifiers.control
            && !modifiers.platform
            && !modifiers.alt
        {
            self.open_menu(id, 0, None, window, cx);
            cx.stop_propagation();
            return;
        }
        if !state.borrow().focus.is_focused(window) {
            return;
        }
        if key == "tab" && open {
            self.close_menu(id, true, window, cx);
            return;
        }
        if key == "escape" && open {
            self.close_menu(id, true, window, cx);
            cx.stop_propagation();
            return;
        }
        if modifiers.control || modifiers.platform || modifiers.function {
            return;
        }
        if !open {
            if matches!(key, "enter" | "space" | "down" | "up") && !modifiers.modified() {
                self.open_menu(id, 0, None, window, cx);
                cx.stop_propagation();
            }
            return;
        }
        let (path, config, rows) = {
            let state = state.borrow();
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return;
            };
            let depth = state.path.len() - 1;
            let Some(menu) = state.menu(depth) else {
                return;
            };
            (
                state.path.clone(),
                state.config.clone(),
                self.menu_rows(tree, id, menu, window, cx),
            )
        };
        let depth = path.len() - 1;
        let selected = state.borrow().selected[depth];
        if matches!(key, "enter" | "space" | "right")
            && !modifiers.modified()
            && let Some(index) = selected
            && let Some(row) = rows.get(index)
            && (key != "right" || row.kind == RowKind::Submenu)
        {
            self.activate_menu_row(
                Hit {
                    id,
                    path: &path,
                    index,
                    row,
                    expected: &config,
                },
                window,
                cx,
            );
            cx.stop_propagation();
            return;
        }
        if key == "left" && depth > 0 && !modifiers.modified() {
            state.borrow_mut().path.pop();
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if matches!(key, "left" | "right")
            && depth == 0
            && !modifiers.modified()
            && matches!(
                config.presentation,
                MenuPresentation::Bar | MenuPresentation::PlatformBar
            )
        {
            let direction = if key == "left" {
                config.menus.len() - 1
            } else {
                1
            };
            for step in 1..=config.menus.len() {
                let next = (path[0] + direction * step) % config.menus.len();
                if !config.menus[next].disabled {
                    self.open_menu(id, next, None, window, cx);
                    break;
                }
            }
            cx.stop_propagation();
            return;
        }
        let enabled: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.enabled)
            .map(|(i, _)| i)
            .collect();
        let navigation = if !modifiers.modified() {
            let current =
                selected.and_then(|current| enabled.iter().position(|index| *index == current));
            match key {
                "home" => enabled.first().copied(),
                "end" => enabled.last().copied(),
                "down" if !enabled.is_empty() => {
                    Some(enabled[current.map_or(0, |i| (i + 1) % enabled.len())])
                }
                "up" if !enabled.is_empty() => Some(
                    enabled[current.map_or(enabled.len() - 1, |i| {
                        (i + enabled.len() - 1) % enabled.len()
                    })],
                ),
                _ => None,
            }
        } else {
            None
        };
        let text = event
            .keystroke
            .key_char
            .as_deref()
            .or_else(|| (key.chars().count() == 1 && !modifiers.alt).then_some(key));
        let navigation = navigation.or_else(|| {
            text.and_then(|text| {
                let choices = ChoiceConfig {
                    label: String::new(),
                    selected: None,
                    disabled: false,
                    items: rows
                        .iter()
                        .enumerate()
                        .map(|(index, row)| ChoiceItem {
                            id: index.to_string(),
                            label: row.label.clone(),
                            disabled: !row.enabled,
                        })
                        .collect(),
                };
                state
                    .borrow_mut()
                    .search
                    .advance(
                        &choices,
                        selected.map(|i| i.to_string()).as_deref(),
                        text,
                        std::time::Instant::now(),
                    )
                    .and_then(|id| id.parse().ok())
            })
        });
        if let Some(index) = navigation {
            let mut state = state.borrow_mut();
            state.selected[depth] = Some(index);
            state.scrolls[depth].scroll_to_item(index, gpui::ScrollStrategy::Nearest);
            cx.notify();
            cx.stop_propagation();
        } else if matches!(
            key,
            "up" | "down" | "home" | "end" | "enter" | "space" | "left" | "right"
        ) {
            cx.stop_propagation();
        }
    }
    pub(super) fn menu_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        mut interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = node.id;
        self.visited.insert(id);
        let config = node.menu.as_ref().expect("validated menu").clone();
        let visible = self.focus.borrow().visible(id);
        let state = self
            .menus
            .entry(id)
            .or_insert_with(|| Rc::new(RefCell::new(State::new(config.clone(), cx))))
            .clone();
        let current_editor = self.editor_menu_target(tree, id, cx);
        if state.borrow().config.presentation == MenuPresentation::EditorContext
            && state.borrow().editor_target.is_some()
            && (state.borrow().editor_target != current_editor || state.borrow().config != config)
        {
            self.close_menu(id, true, window, cx);
        }
        {
            let mut state = state.borrow_mut();
            if state.config != config {
                state.close();
                state.config = config.clone();
                state
                    .triggers
                    .resize_with(config.menus.len(), Default::default);
            }
            if !visible
                || (state.tracking() && state.observed.map(|(handler, _)| handler) != node.handler)
                || (!state.tracking() && !state.focus.is_focused(window))
                || !self.focus.borrow().interactive(id)
                || !self.focus.borrow().allows(id)
            {
                state.close();
            }
            state.rows.borrow_mut().clear();
        }
        self.publish_menu_observation(id, window, cx);
        for style in node.style.iter() {
            if let Style::Fields(fields) = style {
                for field in fields {
                    if let Field::PointerEvents(pointer) = field {
                        interaction.pointer = *pointer;
                    }
                }
            }
        }
        let platform =
            cfg!(target_os = "macos") && config.presentation == MenuPresentation::PlatformBar;
        if platform {
            return div().into_any_element();
        }
        let tab_stop = !config.presentation.is_context();
        let own_disabled = config.menus.is_empty() || config.menus.iter().all(|menu| menu.disabled);
        let disabled = own_disabled || self.focus.borrow().disabled(id);
        if (disabled || !visible) && state.borrow().focus.is_focused(window) {
            window.blur(cx);
        }
        let focus = state
            .borrow()
            .focus
            .clone()
            .tab_stop(tab_stop && !disabled && visible);
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        let mut base = div().id(("gpuio-menu", identity)).track_focus(&focus);
        if tab_stop && !disabled && visible {
            base = base.tab_index(0);
        }
        let (styled, states) = super::apply_styles(base, &node.style, interaction, disabled);
        base = self.coordinate_split(tree, id, styled, interaction.pointer && !disabled, window);
        let [focused, hovered, pressed, _, _, disabled_style, _] = states;
        if let Some(style) = focused {
            base = base.focus(move |_| style);
        }
        if let Some(style) = hovered {
            base = base.hover(move |_| style);
        }
        if let Some(style) = pressed {
            base = base.active(move |_| style);
        }
        if disabled {
            if own_disabled {
                base = base.opacity(0.5);
            }
            if let Some(style) = disabled_style {
                gpui::Refineable::refine(base.style(), &style);
            }
        }
        base = base.on_key_down(
            cx.listener(move |view, event, window, cx| view.menu_key(id, event, window, cx)),
        );
        let owner = cx.weak_entity();
        let action_focus = focus.clone();
        let gate = self.focus.clone();
        if !disabled {
            base = base.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                if gate.borrow().allows(id) && gate.borrow().visible(id) {
                    window.focus(&action_focus, cx);
                }
            });
        }
        if config.presentation.is_context() {
            base = base.child(self.element(tree, node.children[0], interaction, window, cx));
            if interaction.pointer {
                base = base.on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(move |view, event: &gpui::MouseDownEvent, window, cx| {
                        view.open_menu(id, 0, Some(event.position), window, cx);
                        window.prevent_default();
                        cx.stop_propagation();
                    }),
                );
            }
        } else if config.presentation == MenuPresentation::Button {
            let menu = &config.menus[0];
            base = base
                .role(gpui::Role::Button)
                .aria_label(menu.label.clone())
                .aria_expanded(!state.borrow().path.is_empty())
                .child(gpui::SharedString::from(menu.label.clone()));
            let accessible_owner = owner.clone();
            base = base.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                let _ =
                    accessible_owner.update(cx, |view, cx| view.open_menu(id, 0, None, window, cx));
            });
            if interaction.pointer && !menu.disabled {
                base = base.cursor_pointer().on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |view, _, window, cx| {
                        if view.menus[&id].borrow().path.is_empty() {
                            view.open_menu(id, 0, None, window, cx);
                        } else {
                            view.close_menu(id, true, window, cx);
                        }
                        window.prevent_default();
                        cx.stop_propagation();
                    }),
                );
            }
        } else {
            base = base
                .flex()
                .flex_row()
                .role(gpui::Role::MenuBar)
                .aria_label("Application menus");
            for (index, menu) in config.menus.iter().enumerate() {
                let bounds = state.borrow().triggers[index].clone();
                let action_owner = owner.clone();
                let mut trigger = div()
                    .id(index)
                    .px(px(10.))
                    .py(px(6.))
                    .role(gpui::Role::MenuItem)
                    .aria_label(menu.label.clone())
                    .aria_expanded(state.borrow().path.first() == Some(&index))
                    .child(gpui::SharedString::from(menu.label.clone()))
                    .child(
                        canvas(move |geometry, _, _| bounds.set(geometry), |_, _, _, _| {})
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                    );
                if !menu.disabled {
                    trigger = trigger.on_a11y_action(
                        gpui::AccessibleAction::Click,
                        move |_, window, cx| {
                            let _ = action_owner
                                .update(cx, |view, cx| view.open_menu(id, index, None, window, cx));
                        },
                    );
                    if interaction.pointer {
                        trigger = trigger.cursor_pointer().on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |view, _, window, cx| {
                                if view.menus[&id].borrow().path.first() == Some(&index) {
                                    view.close_menu(id, true, window, cx);
                                } else {
                                    view.open_menu(id, index, None, window, cx);
                                }
                                window.prevent_default();
                                cx.stop_propagation();
                            }),
                        );
                    }
                }
                base = base.child(crate::semantics::State {
                    identity: None,
                    busy: false,
                    hidden: false,
                    metadata: None,
                    live: None,
                    element: trigger,
                    disabled: menu.disabled,
                    read_only: false,
                    modal: false,
                });
            }
        }
        let root_state = state.clone();
        let manager = self.focus.clone();
        let record_focus = focus.clone();
        base = base.child(
            canvas(
                move |bounds, _, _| {
                    let state = root_state.borrow();
                    if matches!(
                        state.config.presentation,
                        MenuPresentation::Button
                            | MenuPresentation::Context
                            | MenuPresentation::EditorContext
                            | MenuPresentation::PlatformContext
                    ) && !(state.config.presentation.is_context()
                        && !state.path.is_empty()
                        && state.context_position.is_some())
                    {
                        state.triggers[0].set(bounds);
                    }
                },
                move |bounds, _, window, _| {
                    if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                        manager.borrow_mut().record(
                            id,
                            record_focus.clone(),
                            tab_stop && !disabled,
                            record_focus.is_focused(window),
                            bounds,
                        );
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
        #[cfg(feature = "native-tests")]
        {
            let probes = self.probes.clone();
            base = base.child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds, window, _| {
                        probes.borrow_mut().insert(
                            id,
                            super::native_test::Probe {
                                bounds,
                                color: window.text_style().color,
                            },
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        }
        let path = state.borrow().path.clone();
        let appearance = node
            .choice_appearance
            .clone()
            .unwrap_or_else(crate::appearance::default);
        let priority = self.focus.borrow().layer(id) + 2;
        for depth in 0..path.len() {
            let (definition, trigger) = {
                let state = state.borrow();
                let Some(menu) = state.menu(depth) else {
                    break;
                };
                let trigger = if depth == 0 {
                    state.triggers[path[0]].clone()
                } else {
                    state
                        .rows
                        .borrow_mut()
                        .entry((depth - 1, path[depth]))
                        .or_default()
                        .clone()
                };
                (menu.clone(), trigger)
            };
            let rows = self.menu_rows(tree, id, &definition, window, cx);
            let content = self.menu_content(node, &path[..=depth], interaction, &rows, cx);
            let panel = self.menu_panel(
                Panel {
                    id,
                    path: &path[..=depth],
                    label: &definition.label,
                    rows,
                    state: state.clone(),
                    appearance: appearance.clone(),
                    pointer: interaction.pointer,
                    content,
                },
                window,
                cx,
            );
            self.focus
                .borrow_mut()
                .surface(id, state.borrow().panels[depth].clone());
            base = base.child(
                deferred(super::popup::Surface {
                    geometry: node.placement_geometry.map(|geometry| {
                        if depth == 0 {
                            geometry
                        } else {
                            geometry.submenu()
                        }
                    }),
                    trigger,
                    placement: if depth == 0 {
                        node.placement.unwrap_or(Placement {
                            side: Side::Bottom,
                            align: Align::Start,
                            offset: 2.,
                        })
                    } else {
                        Placement {
                            side: Side::Right,
                            align: Align::Start,
                            offset: 2.,
                        }
                    },
                    content: panel,
                })
                .with_priority(priority + depth),
            );
        }
        crate::semantics::State {
            identity: None,
            busy: false,
            hidden: false,
            metadata: None,
            live: None,
            element: super::highlight_style::Frame::new(base, node, &self.focus),
            disabled,
            read_only: false,
            modal: false,
        }
        .into_any_element()
    }
    fn menu_panel(
        &self,
        render: Panel<'_>,
        window: &Window,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let Panel {
            id,
            path,
            label,
            rows,
            state,
            appearance,
            pointer,
            content,
        } = render;
        let depth = path.len() - 1;
        let count = rows.len();
        {
            let mut state = state.borrow_mut();
            if state.selected[depth]
                .is_none_or(|index| rows.get(index).is_none_or(|row| !row.enabled))
            {
                state.selected[depth] = rows.iter().position(|row| row.enabled);
            }
            let geometry = (
                appearance.row_height,
                appearance.max_visible_rows,
                crate::window_frame::content_bounds(window).size,
            );
            if state.geometry != Some(geometry) {
                state.geometry = Some(geometry);
                state.revealed.fill(None);
            }
            if state.selected[depth] != state.revealed[depth] {
                if let Some(index) = state.selected[depth] {
                    state.scrolls[depth].scroll_to_item(index, gpui::ScrollStrategy::Nearest);
                }
                state.revealed[depth] = state.selected[depth];
            }
        }
        let dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        let background = rgba(if dark { 0x242424ff } else { 0xffffffff });
        let foreground = rgba(if dark { 0xf0f0f0ff } else { 0x202020ff });
        let active = rgba(if dark { 0x385477ff } else { 0xdbeaffff });
        let height = px((count.clamp(1, appearance.max_visible_rows as usize) as f64
            * appearance.row_height) as f32)
        .min((crate::window_frame::content_bounds(window).size.height - px(18.)).max(px(1.)));
        let width = px(appearance.popup_width as f32)
            .min((crate::window_frame::content_bounds(window).size.width - px(16.)).max(px(1.)));
        let config = state.borrow().config.clone();
        let path = path.to_vec();
        let owner = cx.weak_entity();
        let owner_id = cx.entity_id();
        let row_state = state.clone();
        let row_appearance = appearance.clone();
        let scroll = state.borrow().scrolls[depth].clone();
        let list = gpui::uniform_list(("menu-rows", depth), count, move |range, window, cx| {
            range
                .map(|index| {
                    let row = rows[index].clone();
                    let label = content
                        .as_ref()
                        .and_then(|render| {
                            render(
                                index,
                                !row.enabled && row.kind != RowKind::Label,
                                window,
                                cx,
                            )
                        })
                        .unwrap_or_else(|| {
                            div()
                                .child(gpui::SharedString::from(row.label.clone()))
                                .into_any_element()
                        });
                    let selected = row_state.borrow().selected[depth] == Some(index);
                    let anchor = row_state
                        .borrow()
                        .rows
                        .borrow_mut()
                        .entry((depth, index))
                        .or_default()
                        .clone();
                    let mut element = div()
                        .id(index)
                        .w_full()
                        .h(px(row_appearance.row_height as f32))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .overflow_hidden();
                    gpui::Refineable::refine(
                        element.style(),
                        &crate::appearance::refinement(&row_appearance.option_style, 0),
                    );
                    if row.kind == RowKind::Separator {
                        element = element
                            .role(gpui::Role::Splitter)
                            .child(div().h(px(1.)).w_full().bg(rgba(0x80808080)));
                    } else if row.kind == RowKind::Label {
                        // Ordinary text semantics, with no command action,
                        // focus target or disabled-control announcement.
                        element = element
                            .role(gpui::Role::Label)
                            .aria_label(row.label.clone())
                            .child(label);
                    } else {
                        element = element
                            .role(if row.checked.is_some() {
                                gpui::Role::MenuItemCheckBox
                            } else {
                                gpui::Role::MenuItem
                            })
                            .aria_label(row.label.clone());
                        if let Some(checked) = row.checked {
                            element = element.aria_toggled(if checked {
                                gpui::accesskit::Toggled::True
                            } else {
                                gpui::accesskit::Toggled::False
                            });
                        }
                        if row.kind == RowKind::Submenu {
                            element = element.aria_expanded(
                                row_state.borrow().path.get(depth + 1) == Some(&index),
                            );
                        }
                        if selected && row.enabled {
                            element = element.bg(active).aria_active_descendant();
                            gpui::Refineable::refine(
                                element.style(),
                                &crate::appearance::refinement(&row_appearance.option_style, 1),
                            );
                        }
                        if row.checked == Some(true) {
                            gpui::Refineable::refine(
                                element.style(),
                                &crate::appearance::refinement(&row_appearance.option_style, 7),
                            );
                        }
                        element = element
                            .child(div().w(px(12.)).child(if row.checked == Some(true) {
                                "✓"
                            } else {
                                ""
                            }))
                            .child(div().flex_1().child(label))
                            .child(if row.kind == RowKind::Submenu {
                                "›"
                            } else {
                                ""
                            });
                        if row.enabled {
                            let accessible_owner = owner.clone();
                            let accessible_path = path.clone();
                            let accessible_row = row.clone();
                            let accessible_config = config.clone();
                            element = element.on_a11y_action(
                                gpui::AccessibleAction::Click,
                                move |_, window, cx| {
                                    let _ = accessible_owner.update(cx, |view, cx| {
                                        view.activate_menu_row(
                                            Hit {
                                                id,
                                                path: &accessible_path,
                                                index,
                                                row: &accessible_row,
                                                expected: &accessible_config,
                                            },
                                            window,
                                            cx,
                                        )
                                    });
                                },
                            );
                            if pointer {
                                let click_owner = owner.clone();
                                let click_path = path.clone();
                                let click_config = config.clone();
                                let click_row = row.clone();
                                element = element
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                    })
                                    .on_click(move |_, window, cx| {
                                        let _ = click_owner.update(cx, |view, cx| {
                                            view.activate_menu_row(
                                                Hit {
                                                    id,
                                                    path: &click_path,
                                                    index,
                                                    row: &click_row,
                                                    expected: &click_config,
                                                },
                                                window,
                                                cx,
                                            )
                                        });
                                        cx.stop_propagation();
                                    });
                                let hover_owner = owner.clone();
                                let hover_path = path.clone();
                                let hover_config = config.clone();
                                let hover_row = row.clone();
                                let hover_state = row_state.clone();
                                element = element.on_hover(move |hovered, window, cx| {
                                    if !hovered {
                                        return;
                                    }
                                    if hover_row.kind == RowKind::Submenu {
                                        let _ = hover_owner.update(cx, |view, cx| {
                                            view.activate_menu_row(
                                                Hit {
                                                    id,
                                                    path: &hover_path,
                                                    index,
                                                    row: &hover_row,
                                                    expected: &hover_config,
                                                },
                                                window,
                                                cx,
                                            )
                                        });
                                    } else {
                                        let mut state = hover_state.borrow_mut();
                                        if state.config == hover_config
                                            && state.path.starts_with(&hover_path)
                                        {
                                            state.path.truncate(depth + 1);
                                            state.selected[depth] = Some(index);
                                            cx.notify(owner_id);
                                        }
                                    }
                                });
                            }
                        } else {
                            element = element.opacity(0.5);
                            gpui::Refineable::refine(
                                element.style(),
                                &crate::appearance::refinement(&row_appearance.option_style, 6),
                            );
                        }
                    }
                    element = element.child(
                        canvas(move |bounds, _, _| anchor.set(bounds), |_, _, _, _| {})
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
                        element,
                        disabled: !row.enabled
                            && matches!(row.kind, RowKind::Command | RowKind::Submenu),
                        read_only: false,
                        modal: false,
                    }
                })
                .collect::<Vec<_>>()
        })
        .track_scroll(&scroll)
        .w_full()
        .h(height);
        let bounds = state.borrow().panels[depth].clone();
        let outside_owner = cx.weak_entity();
        let outside_state = state.clone();
        let mut panel = div()
            .id(("menu-panel", depth))
            .role(gpui::Role::Menu)
            .aria_label(label.to_owned())
            .w(width)
            .bg(background)
            .text_color(foreground)
            .border_1()
            .border_color(active)
            .occlude()
            .on_mouse_down_out(move |event, window, cx| {
                if !outside_state.borrow().inside(event.position) {
                    let _ =
                        outside_owner.update(cx, |view, cx| view.close_menu(id, false, window, cx));
                }
            })
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(if count == 0 {
                let mut empty = div()
                    .h(height)
                    .px(px(8.))
                    .child(gpui::SharedString::from(appearance.empty_label.clone()));
                gpui::Refineable::refine(
                    empty.style(),
                    &crate::appearance::refinement(&appearance.empty_style, 0),
                );
                empty.into_any_element()
            } else {
                list.into_any_element()
            })
            .child(
                canvas(move |geometry, _, _| bounds.set(geometry), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            );
        gpui::Refineable::refine(
            panel.style(),
            &crate::appearance::refinement(&appearance.popup_style, 0),
        );
        panel.into_any_element()
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "placement_menu_test.rs"]
mod placement_geometry_test;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "menu_command_test.rs"]
mod command_tests;
