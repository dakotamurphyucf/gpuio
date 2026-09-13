use crate::{session::Session, transport::Transport};
use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowHandle, WindowOptions, canvas, div,
    prelude::*, px, rgba, size,
};
use gpuio_protocol::{
    NodeId, WindowId,
    v1::{self, *},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};

type SharedSession = Rc<RefCell<Session>>;
#[path = "choice.rs"]
mod choice;
#[path = "choice_popup.rs"]
mod choice_popup;
#[path = "combobox.rs"]
mod combobox;
#[path = "command.rs"]
mod command;
#[path = "editor.rs"]
mod editor;
#[cfg(feature = "native-tests")]
#[path = "editor_test.rs"]
pub(super) mod editor_test;
#[path = "focus.rs"]
mod focus;
#[path = "overlay.rs"]
mod overlay;
#[path = "popup.rs"]
mod popup;
#[path = "radio.rs"]
mod radio;
#[path = "select.rs"]
mod select;
#[path = "tooltip.rs"]
mod tooltip;
#[path = "typeahead.rs"]
mod typeahead;
struct ButtonState {
    focus: gpui::FocusHandle,
}
#[derive(Clone, Copy)]
struct Interaction {
    pointer: bool,
    selectable: bool,
    selection_color: gpui::Hsla,
}
impl Default for Interaction {
    fn default() -> Self {
        Self {
            pointer: true,
            selectable: false,
            selection_color: rgba(0x386ac880).into(),
        }
    }
}

struct View {
    id: WindowId,
    session: SharedSession,
    transport: Arc<Transport>,
    buttons: BTreeMap<NodeId, Rc<ButtonState>>,
    selections: BTreeMap<NodeId, Rc<RefCell<crate::selection::State>>>,
    editors: BTreeMap<NodeId, editor::Instance>,
    root_focus: Option<gpui::FocusHandle>,
    focus: focus::Shared,
    selects: BTreeMap<NodeId, Rc<RefCell<select::State>>>,
    radios: BTreeMap<NodeId, Rc<RefCell<choice::State>>>,
    tooltips: BTreeMap<NodeId, tooltip::State>,
    tooltip_last_closed: Option<std::time::Instant>,
    command_subscription: Option<gpui::Subscription>,
    visited: std::collections::BTreeSet<NodeId>,
    #[cfg(feature = "native-tests")]
    probes: Rc<RefCell<BTreeMap<NodeId, native_test::Probe>>>,
}
fn emit_press(
    session: &SharedSession,
    gate: &focus::Shared,
    transport: &Transport,
    window: WindowId,
    node: NodeId,
    handler: gpuio_protocol::HandlerId,
    revision: i64,
) {
    if !gate.borrow().allows(node) {
        return;
    }
    let event = session.borrow().press(window, node, handler, revision);
    if let Some(event) = event
        && !transport.input(event)
        && session.borrow_mut().overload(window)
    {
        transport.fault(window);
    }
}
fn color(value: &Color) -> gpui::Hsla {
    // Named token resolution will be supplied by the typed theme adapter (OCH-8).
    match value {
        Color::Rgba(value) => rgba(*value as u32).into(),
        Color::Token(_) => unreachable!("unresolved theme token passed validation"),
    }
}
fn length(value: &v1::Length) -> gpui::Length {
    match value {
        v1::Length::Auto => gpui::Length::Auto,
        v1::Length::Px(v) => px(*v as f32).into(),
        v1::Length::Percent(v) => gpui::relative(*v as f32 / 100.).into(),
    }
}

// Paint indicators with the inherited foreground, including theme and native
// state refinements. They are decorative; the focus root owns all semantics.
fn control_indicator(kind: Kind, checked: bool, indeterminate: bool) -> gpui::AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let color = window.text_style().color;
            let radius = if matches!(kind, Kind::Switch | Kind::RadioGroup) {
                9.
            } else {
                3.
            };
            let mut outline = gpui::outline(bounds, color, Default::default());
            outline.corner_radii = px(radius).into();
            window.paint_quad(outline);
            let origin = bounds.origin;
            if kind == Kind::Switch {
                let x = if checked { 15. } else { 3. };
                let mut knob = gpui::fill(
                    Bounds::new(origin + gpui::point(px(x), px(3.)), size(px(12.), px(12.))),
                    color,
                );
                knob.corner_radii = px(6.).into();
                window.paint_quad(knob);
            } else if kind == Kind::RadioGroup && checked {
                let mut dot = gpui::fill(
                    Bounds::new(origin + gpui::point(px(5.), px(5.)), size(px(8.), px(8.))),
                    color,
                );
                dot.corner_radii = px(4.).into();
                window.paint_quad(dot);
            } else if indeterminate {
                window.paint_quad(gpui::fill(
                    Bounds::new(origin + gpui::point(px(4.), px(8.)), size(px(10.), px(2.))),
                    color,
                ));
            } else if checked {
                let mut path = gpui::PathBuilder::stroke(px(2.));
                path.move_to(origin + gpui::point(px(4.), px(9.)));
                path.line_to(origin + gpui::point(px(8.), px(13.)));
                path.line_to(origin + gpui::point(px(14.), px(5.)));
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .w(px(if kind == Kind::Switch { 30. } else { 18. }))
    .h(px(18.))
    .flex_shrink_0()
    .into_any_element()
}

fn apply_styles(
    mut element: gpui::Stateful<gpui::Div>,
    styles: &[Style],
    interaction: Interaction,
    disabled: bool,
) -> (
    gpui::Stateful<gpui::Div>,
    [Option<gpui::StyleRefinement>; 7],
) {
    let mut states: [Option<gpui::StyleRefinement>; 7] = Default::default();
    for style in styles {
        match style {
            Style::Fields(fields) => {
                crate::style::refine(element.style(), fields);
            }
            Style::State(state, fields) => {
                if matches!(*state, 2 | 3) && (!interaction.pointer || disabled) {
                    continue;
                }
                let refinement = states[(*state - 1) as usize].get_or_insert_with(Default::default);
                crate::style::refine(refinement, fields);
            }
            Style::Width(v) => element.style().size.width = Some(length(v)),
            Style::Height(v) => element.style().size.height = Some(length(v)),
            Style::MinWidth(v) => element.style().min_size.width = Some(length(v)),
            Style::MinHeight(v) => element.style().min_size.height = Some(length(v)),
            Style::MaxWidth(v) => element.style().max_size.width = Some(length(v)),
            Style::MaxHeight(v) => element.style().max_size.height = Some(length(v)),
            Style::Padding(v) => element = element.p(px(*v as f32)),
            Style::Gap(v) => element = element.gap(px(*v as f32)),
            Style::Grow(v) => element.style().flex_grow = Some(*v as f32),
            Style::Shrink(v) => element.style().flex_shrink = Some(*v as f32),
            Style::Direction(v) => {
                element.style().flex_direction = Some(match v {
                    0 => gpui::FlexDirection::Row,
                    1 => gpui::FlexDirection::Column,
                    2 => gpui::FlexDirection::RowReverse,
                    _ => gpui::FlexDirection::ColumnReverse,
                })
            }
            Style::Background(v) => element = element.bg(color(v)),
            Style::Foreground(v) => element = element.text_color(color(v)),
            Style::FontSize(v) => element = element.text_size(px(*v as f32)),
            Style::Radius(v) => element = element.rounded(px(*v as f32)),
            Style::Opacity(v) => element = element.opacity(*v as f32),
            Style::HoverBackground(v) => {
                if interaction.pointer && !disabled {
                    states[1].get_or_insert_with(Default::default).background =
                        Some(color(v).into());
                }
            }
            Style::PressedBackground(v) => {
                if interaction.pointer && !disabled {
                    states[2].get_or_insert_with(Default::default).background =
                        Some(color(v).into());
                }
            }
            Style::FocusBackground(v) => {
                states[0].get_or_insert_with(Default::default).background = Some(color(v).into());
            }
        }
    }
    (element, states)
}

impl View {
    fn new(id: WindowId, session: SharedSession, transport: Arc<Transport>) -> Self {
        Self {
            id,
            focus: focus::Manager::new(id, session.clone()),
            session,
            transport,
            buttons: BTreeMap::new(),
            selections: BTreeMap::new(),
            editors: BTreeMap::new(),
            root_focus: None,
            radios: BTreeMap::new(),
            tooltips: BTreeMap::new(),
            tooltip_last_closed: None,
            command_subscription: None,
            selects: BTreeMap::new(),
            visited: Default::default(),
            #[cfg(feature = "native-tests")]
            probes: Default::default(),
        }
    }
    fn update_editors(&mut self, dirty: &[NodeId], window: &mut Window, cx: &mut Context<Self>) {
        self.install_command_interceptor(window, cx);
        self.sync_tooltips(window, cx);
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.editors.clear();
                return;
            };
            self.editors.retain(|id, _| tree.get(*id).is_some());
            dirty
                .iter()
                .filter_map(|id| tree.get(*id))
                .filter(|node| node.editor.is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        for node in nodes {
            if let Some(editor) = self.editors.get_mut(&node.id) {
                editor.configure(node.editor.as_ref().expect("validated editor"), window, cx);
            } else {
                let editor = editor::Instance::new(
                    self.id,
                    &node,
                    self.session.clone(),
                    self.focus.clone(),
                    self.transport.clone(),
                    window,
                    cx,
                );
                self.editors.insert(node.id, editor);
            }
        }
    }
    fn element(
        &mut self,
        tree: &crate::tree::Tree,
        id: NodeId,
        mut interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let node = tree.get(id).expect("validated retained node");
        if node.kind == Kind::Tooltip {
            return self.tooltip_element(tree, node, interaction, window, cx);
        }
        let popup_priority = self.focus.borrow().layer(id) + 2;
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        let command = node
            .command_ref
            .as_ref()
            .and_then(|id| tree.command(node.id, id))
            .map(|(scope, config)| {
                command::Route::new(tree, scope, config, CommandSource::Button(id))
            });
        let label = command.as_ref().map_or_else(
            || node.text.clone(),
            |route| Arc::from(route.config.label.as_str()),
        );
        let mut accessible_name = gpui::SharedString::from(label.clone());
        for style in node.style.iter() {
            if let Style::Fields(fields) = style {
                for field in fields {
                    match field {
                        Field::PointerEvents(v) => interaction.pointer = *v,
                        Field::UserSelect(v) => interaction.selectable = *v,
                        Field::SelectionColor(v) => interaction.selection_color = color(v),
                        Field::AccessibleName(v) => accessible_name = v.clone().into(),
                        _ => (),
                    }
                }
            }
        }
        let mut element = div().id(("gpuio-node", identity));
        if let Some(description) = tree.tooltip_description(id) {
            element = element.aria_description(description.to_owned());
        }
        if matches!(
            node.kind,
            Kind::Container | Kind::FocusScope | Kind::CommandScope | Kind::RadioGroup
        ) {
            element = element.flex().flex_col();
        } else if node.kind == Kind::Select {
            element = element
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .p(px(8.))
                .border_1()
                .border_color(rgba(0x80808080))
                .rounded(px(4.));
        }
        if let Some(config) = &node.overlay {
            let available = window.viewport_size();
            element = element
                .w(px(config.width as f32).min((available.width - px(32.)).max(px(1.))))
                .max_h((available.height - px(32.)).max(px(1.)))
                .overflow_y_scroll()
                .p(px(16.))
                .gap(px(8.))
                .rounded(px(8.))
                .bg(rgba(0x25272aff))
                .border_1()
                .border_color(rgba(0x80808080));
        }
        let disabled = command
            .as_ref()
            .is_some_and(|route| !self.command_available(&route.config, window, cx))
            || node.choice.as_ref().is_some_and(|config| config.disabled)
            || node.control.is_some_and(Control::disabled)
            || node.editor.as_ref().is_some_and(|config| config.disabled);
        let checked = command
            .as_ref()
            .is_some_and(|route| route.config.checked == Some(true))
            || matches!(
                node.control,
                Some(Control::Checkbox(CheckState::Checked, _) | Control::Switch(true, _))
            );
        let indeterminate = matches!(
            node.control,
            Some(Control::Checkbox(CheckState::Indeterminate, _))
        );
        if let Some(handle) = self.focus.borrow().handle(id) {
            element = element.track_focus(&handle);
        }
        if matches!(
            node.kind,
            Kind::Button
                | Kind::CommandButton
                | Kind::Checkbox
                | Kind::Switch
                | Kind::RadioGroup
                | Kind::Select
        ) {
            self.visited.insert(id);
            let state = self
                .buttons
                .entry(id)
                .or_insert_with(|| {
                    Rc::new(ButtonState {
                        focus: cx.focus_handle().tab_stop(true),
                    })
                })
                .clone();
            if disabled {
                state.focus.clone().tab_stop(false);
                if state.focus.is_focused(window) {
                    window.blur(cx);
                }
            } else {
                element = element
                    .track_focus(&state.focus.clone().tab_stop(true))
                    .tab_index(0);
                let focus = state.focus.clone();
                let gate = self.focus.clone();
                element =
                    element.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                        if gate.borrow().allows(id) {
                            window.focus(&focus, cx);
                        }
                    });
            }
            element = element
                .role(match node.kind {
                    Kind::Checkbox => gpui::Role::CheckBox,
                    Kind::Switch => gpui::Role::Switch,
                    Kind::RadioGroup => gpui::Role::RadioGroup,
                    Kind::Select => gpui::Role::ComboBox,
                    _ => gpui::Role::Button,
                })
                .aria_label(accessible_name);
            if command
                .as_ref()
                .is_some_and(|route| route.config.checked.is_some())
                || matches!(node.kind, Kind::Checkbox | Kind::Switch)
            {
                element = element.aria_toggled(if indeterminate {
                    gpui::accesskit::Toggled::Mixed
                } else if checked {
                    gpui::accesskit::Toggled::True
                } else {
                    gpui::accesskit::Toggled::False
                });
            }
            if interaction.pointer && !disabled {
                element = element.cursor_pointer();
            }
        }
        let (styled, states) = apply_styles(element, &node.style, interaction, disabled);
        element = styled;
        let [
            focused,
            hovered,
            pressed,
            checked_style,
            indeterminate_style,
            disabled_style,
            selected_style,
        ] = states;
        use gpui::Refineable;
        for style in [
            checked.then_some(checked_style).flatten(),
            indeterminate.then_some(indeterminate_style).flatten(),
        ]
        .into_iter()
        .flatten()
        {
            element.style().refine(&style);
        }
        if let Some(style) = focused {
            if let Some(editor) = self.editors.get(&id) {
                if editor.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else {
                element = element.focus(move |_| style);
            }
        }
        if let Some(style) = hovered {
            element = element.hover(move |_| style);
        }
        if let Some(style) = pressed {
            element = element.active(move |_| style);
        }
        if disabled {
            element.style().mouse_cursor = None;
            element = element.opacity(0.5);
            if let Some(style) = disabled_style {
                element.style().refine(&style);
            }
        }
        if !interaction.pointer {
            element.style().mouse_cursor = None;
        }
        if node.kind == Kind::Combobox {
            let (editor, state) = self.editors[&id]
                .combobox()
                .expect("validated combobox editor");
            let route = node
                .handler
                .filter(|_| !disabled)
                .map(|handler| choice::Route {
                    window: self.id,
                    node: id,
                    handler,
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                });
            element = combobox::element(
                element,
                combobox::Render {
                    priority: popup_priority,
                    editor,
                    state,
                    config: node.choice.as_ref().expect("validated combobox choices"),
                    appearance: node
                        .choice_appearance
                        .clone()
                        .unwrap_or_else(crate::appearance::default),
                    filter: node.combobox_filter.expect("validated combobox filter"),
                    route,
                    pointer: interaction.pointer,
                    selected_style,
                },
                window,
                cx,
            );
        } else if let Some(config) = &node.choice {
            element = element.aria_label(config.label.clone());
            let focus = self.buttons[&id].focus.clone();
            let route = node
                .handler
                .filter(|_| !disabled)
                .map(|handler| choice::Route {
                    window: self.id,
                    node: id,
                    handler,
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                });
            if node.kind == Kind::Select {
                let state = self.selects.entry(id).or_default().clone();
                element = select::element(
                    element,
                    select::Render {
                        priority: popup_priority,
                        config,
                        appearance: node
                            .choice_appearance
                            .clone()
                            .unwrap_or_else(crate::appearance::default),
                        state,
                        focus,
                        route,
                        pointer: interaction.pointer,
                        selected_style,
                    },
                    window,
                    cx,
                );
            } else {
                let state = self.radios.entry(id).or_default().clone();
                element = radio::element(
                    element,
                    radio::Render {
                        config,
                        state,
                        focus,
                        route,
                        pointer: interaction.pointer,
                        selected_style,
                    },
                    window,
                    cx,
                );
            }
        } else if let Some(editor) = self.editors.get(&id) {
            let next = self.focus.clone();
            let previous = self.focus.clone();
            element = element
                .capture_action(move |_: &gpui_base::input::IndentInline, window, cx| {
                    next.borrow().traverse(false, window, cx);
                    cx.stop_propagation();
                })
                .capture_action(move |_: &gpui_base::input::OutdentInline, window, cx| {
                    previous.borrow().traverse(true, window, cx);
                    cx.stop_propagation();
                })
                .child(editor.element());
        } else if node.kind == Kind::Text && interaction.selectable {
            self.visited.insert(id);
            let selection = self
                .selections
                .entry(id)
                .or_insert_with(|| {
                    Rc::new(RefCell::new(crate::selection::State::new(
                        node.text.clone(),
                        cx,
                    )))
                })
                .clone();
            selection.borrow_mut().update(node.text.clone());
            element = element.child(crate::selection::element(
                selection,
                interaction.selection_color,
                interaction.pointer,
                cx.entity_id(),
            ));
        } else if matches!(node.kind, Kind::Checkbox | Kind::Switch) {
            element = element.child(control_indicator(node.kind, checked, indeterminate));
            if !node.text.is_empty() {
                element = element.child(gpui::SharedString::from(node.text.clone()));
            }
        } else if !label.is_empty() {
            element = element.child(gpui::SharedString::from(label));
        }
        for child in node.children.iter() {
            if tree
                .get(*child)
                .and_then(|node| node.overlay.as_ref())
                .is_some_and(|config| config.kind == OverlayKind::Popover)
            {
                let anchor = self.focus.borrow().anchor(*child).expect("mounted scope");
                element = element.child(
                    canvas(move |bounds, _, _| anchor.set(bounds), |_, _, _, _| {})
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                );
            }
        }
        element = element.children(
            node.children
                .iter()
                .map(|id| self.element(tree, *id, interaction, window, cx))
                .collect::<Vec<_>>(),
        );
        if let Some(route) = command.filter(|_| !disabled) {
            let accessible = route.clone();
            let owner = cx.weak_entity();
            element =
                element.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                    let _ =
                        owner.update(cx, |view, cx| view.invoke_command(&accessible, window, cx));
                    cx.stop_propagation();
                });
            if !interaction.pointer {
                element = element.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default()
                });
            }
            element = element.on_click(cx.listener(move |view, event, window, cx| {
                if interaction.pointer || matches!(event, gpui::ClickEvent::Keyboard(_)) {
                    view.invoke_command(&route, window, cx);
                    cx.stop_propagation();
                }
            }));
        }
        if let Some(handler) = node.handler
            && node.commands.is_none()
            && node.editor.is_none()
            && node.choice.is_none()
            && node.overlay.is_none()
            && !disabled
        {
            let window = self.id;
            let revision = tree.revision();
            let session = self.session.clone();
            let transport = self.transport.clone();
            let gate = self.focus.clone();
            let accessible_gate = gate.clone();
            let accessible_session = session.clone();
            let accessible_transport = transport.clone();
            // GPUI's fallback accessibility Click synthesizes pointer input.
            // Route the semantic action directly so pointer policy and pointer
            // occlusion do not suppress assistive activation.
            element = element.on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                emit_press(
                    &accessible_session,
                    &accessible_gate,
                    &accessible_transport,
                    window,
                    id,
                    handler,
                    revision,
                );
                cx.stop_propagation();
            });
            if !interaction.pointer {
                element = element.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default()
                });
            }
            // GPUI synthesizes one click for native Enter/Space and accessibility
            // activation. Registering a second key handler duplicates activation.
            element = element.on_click(move |event, _, cx| {
                if !interaction.pointer && !matches!(event, gpui::ClickEvent::Keyboard(_)) {
                    return;
                }
                emit_press(&session, &gate, &transport, window, id, handler, revision);
                cx.stop_propagation();
            });
        }
        let gate = self.focus.clone();
        element = element.capture_any_mouse_down(move |_, window, cx| {
            if gate.borrow().blocks_pointer(id) {
                window.prevent_default();
                cx.stop_propagation();
            }
        });
        let handle = self
            .editors
            .get(&id)
            .map(|editor| editor.focus_handle(cx))
            .or_else(|| self.buttons.get(&id).map(|button| button.focus.clone()))
            .or_else(|| {
                self.selections
                    .get(&id)
                    .map(|state| state.borrow().focus.clone())
            })
            .or_else(|| self.focus.borrow().handle(id));
        if let Some(handle) = handle.filter(|_| !disabled) {
            let tab_stop = node.kind != Kind::FocusScope;
            let manager = self.focus.clone();
            element = element.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        if bounds.size.width > px(0.)
                            && bounds.size.height > px(0.)
                            && bounds.intersects(&window.content_mask().bounds)
                        {
                            let focused = handle.is_focused(window);
                            manager.borrow_mut().record(id, handle, tab_stop, focused);
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        }
        #[cfg(feature = "native-tests")]
        {
            let probes = self.probes.clone();
            element = element.child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds, window, _| {
                        probes.borrow_mut().insert(
                            id,
                            native_test::Probe {
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
        if let Some(config) = &node.overlay {
            return overlay::element(
                element,
                config.clone(),
                node.placement.unwrap_or_default(),
                choice::Route {
                    window: self.id,
                    node: id,
                    handler: node.handler.expect("validated overlay"),
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                },
                window,
            );
        }
        crate::semantics::State {
            element,
            disabled,
            read_only: false,
            modal: false,
        }
        .into_any_element()
    }
}
impl Render for View {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.visited.clear();
        self.focus.borrow_mut().clear_surfaces();
        let shared = self.session.clone();
        let session = shared.borrow();
        let root_focus = self
            .root_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let tab_focus = self.focus.clone();
        let begin_focus = self.focus.clone();
        let mut root = div()
            .track_focus(&root_focus)
            .size_full()
            .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
                view.command_shortcut(&event.keystroke, ShortcutPriority::NativeFirst, window, cx);
            }))
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "tab" {
                    tab_focus
                        .borrow()
                        .traverse(event.keystroke.modifiers.shift, window, cx);
                    cx.stop_propagation();
                }
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, _, _| begin_focus.borrow_mut().begin_frame(),
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        if self.focus.borrow_mut().take_pending() {
            let focus = self.focus.clone();
            let fallback = root_focus.clone();
            window.on_next_frame(move |window, cx| {
                focus.borrow_mut().finish_frame(&fallback, window, cx)
            });
        }
        let revision = if let Some(tree) = session.tree(self.id) {
            if let Some(id) = tree.root() {
                root = root.child(self.element(tree, id, Interaction::default(), window, cx));
            }
            tree.revision()
        } else {
            0
        };
        self.buttons.retain(|id, _| self.visited.contains(id));
        self.radios.retain(|id, _| self.visited.contains(id));
        self.selects.retain(|id, _| self.visited.contains(id));
        self.selections.retain(|id, _| self.visited.contains(id));
        // GPUI routes key events along the focused element's ancestry. Keep a
        // non-tab-stop fallback so Tab also works before the first click and
        // after a focused control is disabled or removed.
        let has_focus = self.focus.borrow().contains_focus(window)
            || root_focus.is_focused(window)
            || self
                .buttons
                .values()
                .any(|state| state.focus.is_focused(window))
            || self
                .selections
                .values()
                .any(|state| state.borrow().focus.is_focused(window))
            || self
                .editors
                .values()
                .any(|editor| editor.focus_handle(cx).is_focused(window));
        if !has_focus {
            window.focus(&root_focus, cx);
        }
        let id = self.id;
        let session = self.session.clone();
        let transport = self.transport.clone();
        root.child(
            canvas(
                |_, _, _| (),
                move |_, _, _, _| {
                    let events = session.borrow_mut().painted(id, revision);
                    for event in events {
                        if matches!(event, Event::FrameRequested(..)) {
                            transport.respond(event);
                        } else if !transport.input(event) && session.borrow_mut().overload(id) {
                            transport.fault(id);
                        }
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

fn failed(transport: &Transport, correlation: i64, result: Result<Event, ErrorCode>) {
    transport.respond(result.unwrap_or_else(|error| Event::Failed(correlation, error)));
}

pub fn run(transport: Arc<Transport>) {
    #[cfg(target_os = "linux")]
    if let Ok(expected) = std::env::var("GPUIO_EXPECT_BACKEND") {
        assert_eq!(gpui::guess_compositor(), expected);
    }
    let stopping = Rc::new(Cell::new(false));
    gpui_platform::application().run(move |cx: &mut App| {
        gpui_base::init(cx);
        // GPUI defaults to last-window exit on Linux. Our explicit lifecycle
        // policy must control background applications consistently on both OSes.
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let session = Rc::new(RefCell::new(Session::default()));
        let mut windows: BTreeMap<WindowId, WindowHandle<View>> = BTreeMap::new();
        let closing = stopping.clone();
        let exit_on_last_window = transport.exit_on_last_window;
        cx.on_window_closed(move |cx, _| {
            if exit_on_last_window && cx.windows().is_empty() && !closing.replace(true) {
                cx.defer(stop_application);
            }
        })
        .detach();
        let rx = transport.rx.clone();
        cx.spawn(async move |cx| {
            while rx.recv().await.is_ok() {
                if transport
                    .aborting
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    if !stopping.replace(true) {
                        cx.update(stop_application);
                    }
                    return;
                }
                loop {
                    let id = transport
                        .mailbox
                        .lock()
                        .expect("mailbox poisoned")
                        .pop_close();
                    let Some(id) = id else {
                        break;
                    };
                    if let Ok(pending) = session.borrow_mut().close(id) {
                        if let Some(correlation) = pending {
                            transport.respond(Event::Failed(correlation, ErrorCode::Closed));
                        }
                        transport
                            .mailbox
                            .lock()
                            .expect("mailbox poisoned")
                            .native_closed(id);
                        transport.wake_ocaml();
                        if let Some(window) = windows.remove(&id) {
                            let _ = window.update(cx, |_, window, _| window.remove_window());
                        }
                    }
                }
                // Bound work per wake; yield to native input/paint between batches.
                for _ in 0..crate::mailbox::MAX_COMMANDS {
                    let message = transport.mailbox.lock().expect("mailbox poisoned").pop();
                    let Some(message) = message else {
                        break;
                    };
                    match message {
                        Message::Hello(version, caps) => {
                            failed(&transport, 0, session.borrow_mut().hello(version, caps))
                        }
                        Message::Open(correlation, id, title, width, height) => {
                            let validation = if transport
                                .mailbox
                                .lock()
                                .unwrap()
                                .has_window_output(id.slot())
                            {
                                Err(ErrorCode::Busy)
                            } else {
                                session.borrow().validate_open(id, &title, width, height)
                            };
                            if let Err(error) = validation {
                                transport.respond(Event::Failed(correlation, error));
                                continue;
                            }
                            let native = cx.update(|cx| {
                                let bounds = Bounds::centered(
                                    None,
                                    size(px(width as f32), px(height as f32)),
                                    cx,
                                );
                                cx.open_window(
                                    WindowOptions {
                                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                                        titlebar: Some(gpui::TitlebarOptions {
                                            title: Some(title.clone().into()),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        let close_transport = transport.clone();
                                        window.on_window_should_close(cx, move |_, _| {
                                            close_transport
                                                .mailbox
                                                .lock()
                                                .expect("mailbox poisoned")
                                                .request_close(id);
                                            let _ = close_transport.tx.try_send(());
                                            false
                                        });
                                        cx.new(|_| {
                                            View::new(id, session.clone(), transport.clone())
                                        })
                                    },
                                )
                            });
                            match native {
                                Ok(window) => {
                                    windows.insert(id, window);
                                    failed(
                                        &transport,
                                        correlation,
                                        session.borrow_mut().open(
                                            correlation,
                                            id,
                                            &title,
                                            width,
                                            height,
                                        ),
                                    );
                                    cx.update(|cx| cx.activate(true));
                                }
                                Err(_) => transport
                                    .respond(Event::Failed(correlation, ErrorCode::NativeFailure)),
                            }
                        }
                        Message::Apply(tx) => {
                            let result = session.borrow_mut().apply(&tx);
                            match result {
                                Ok(applied) => {
                                    transport.respond(Event::Accepted(tx.window, tx.revision));
                                    if let Some(window) = windows.get(&tx.window) {
                                        let _ = window.update(cx, |view, window, cx| {
                                            view.update_editors(&applied.dirty, window, cx);
                                            cx.notify();
                                        });
                                    }
                                }
                                Err(error) => transport.respond(Event::Rejected(
                                    tx.window,
                                    tx.revision,
                                    error,
                                )),
                            }
                        }
                        Message::RequestFrame(correlation, id) => {
                            let result = session.borrow_mut().request_frame(correlation, id);
                            match result {
                                Ok(()) => {
                                    if let Some(window) = windows.get(&id) {
                                        let _ = window.update(cx, |_, _, cx| cx.notify());
                                    }
                                }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
                        }
                        Message::EditorCommand(correlation, id, node, command) => {
                            let result = match windows.get(&id) {
                                None => EditorResult::Failed(EditorError::Closed),
                                Some(handle) => handle
                                    .update(cx, |view, window, cx| {
                                        let result = match view.editors.get_mut(&node) {
                                            None => EditorResult::Failed(EditorError::StaleEditor),
                                            Some(editor) => editor.command(&command, window, cx),
                                        };
                                        cx.notify();
                                        result
                                    })
                                    .unwrap_or(EditorResult::Failed(EditorError::Closed)),
                            };
                            transport.respond(Event::EditorResult(correlation, id, node, result));
                        }
                        Message::Close(correlation, id) => {
                            let result = session.borrow_mut().close(id);
                            match result {
                                Ok(pending) => {
                                    if let Some(pending) = pending {
                                        transport
                                            .respond(Event::Failed(pending, ErrorCode::Closed));
                                    }
                                    transport.respond(Event::Closed(correlation, id));
                                    if let Some(window) = windows.remove(&id) {
                                        let _ = window
                                            .update(cx, |_, window, _| window.remove_window());
                                    }
                                }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
                        }
                        Message::Shutdown => {
                            for event in session.borrow_mut().shutdown() {
                                transport.respond(event);
                            }
                            if !stopping.replace(true) {
                                cx.update(stop_application);
                            }
                            return;
                        }
                    }
                }
                // A ready channel alone need not yield its future. Give native
                // input/paint a turn even with a continuously producing client.
                // This timer exists only after work, never as idle polling.
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(1))
                    .await;
            }
        })
        .detach();
    });
}

#[allow(deprecated)]
#[cfg(target_os = "macos")]
fn stop_application(cx: &mut App) {
    use cocoa::{
        appkit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventSubtype, NSEventType},
        base::{YES, nil},
        foundation::NSPoint,
    };
    cx.shutdown();
    // Embedded runtime must regain control instead of NSApplication.terminate.
    unsafe {
        let app = cocoa::appkit::NSApp();
        app.stop_(nil);
        let wake=cocoa::base::id::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2_(nil,NSEventType::NSApplicationDefined,NSPoint::new(0.,0.),NSEventModifierFlags::empty(),0.,0,nil,NSEventSubtype::NSWindowExposedEventType,0,0);
        app.postEvent_atStart_(wake, YES);
    }
}
#[cfg(not(target_os = "macos"))]
fn stop_application(cx: &mut App) {
    cx.quit();
}

#[cfg(feature = "native-tests")]
#[path = "native_test.rs"]
pub(crate) mod native_test;

#[cfg(feature = "native-tests")]
#[path = "control_test.rs"]
pub(crate) mod control_test;
