#[path = "animation_view.rs"]
mod animation;
#[path = "animation_program_view.rs"]
mod animation_program;
#[cfg(feature = "native-tests")]
#[path = "animation_program_test.rs"]
pub(super) mod animation_program_test;
#[cfg(feature = "native-tests")]
#[path = "animation_test.rs"]
pub(super) mod animation_test;
#[path = "canvas_view.rs"]
pub(crate) mod canvas_view;
#[path = "container_query_view.rs"]
mod container_query;
#[cfg(feature = "native-tests")]
#[path = "container_query_test.rs"]
pub(super) mod container_query_test;
#[path = "extension_view.rs"]
mod extension_view;
#[cfg(feature = "native-tests")]
#[path = "presentation_test.rs"]
pub(super) mod presentation_test;
use crate::{session::Session, transport::Transport};
use gpui::Focusable;
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

#[path = "window_host.rs"]
mod window_host;
#[cfg(target_os = "macos")]
#[path = "window_macos.rs"]
mod window_macos;

type SharedSession = Rc<RefCell<Session>>;
#[path = "avatar.rs"]
mod avatar;
#[path = "calendar_view.rs"]
pub(super) mod calendar_view;
#[path = "carousel_view.rs"]
mod carousel;
#[path = "choice.rs"]
mod choice;
#[path = "choice_popup.rs"]
mod choice_popup;
#[path = "color_input_view.rs"]
pub(super) mod color_input_view;
#[path = "combobox.rs"]
mod combobox;
#[path = "command.rs"]
mod command;
#[path = "document_view.rs"]
pub(crate) mod document_view;
#[path = "drag_drop.rs"]
mod drag_drop;
#[path = "editor.rs"]
mod editor;
#[cfg(feature = "native-tests")]
#[path = "editor_test.rs"]
pub(super) mod editor_test;
#[path = "focus.rs"]
mod focus;
#[path = "image_corners.rs"]
mod image_corners;
#[path = "image_view.rs"]
pub(crate) mod image_view;
#[cfg(feature = "native-tests")]
#[path = "list_test.rs"]
pub(super) mod list_test;
#[path = "list_view.rs"]
mod list_view;
#[path = "loading.rs"]
mod loading;
#[path = "menu.rs"]
mod menu;
#[path = "menu_platform.rs"]
mod menu_platform;
#[path = "navigation_view.rs"]
mod navigation;
#[path = "number_input_view.rs"]
pub(super) mod number_input_view;
#[path = "otp_input_view.rs"]
pub(super) mod otp_input_view;
#[path = "overlay.rs"]
mod overlay;
#[path = "palette.rs"]
mod palette;
#[path = "pointer.rs"]
mod pointer;
#[path = "popup.rs"]
mod popup;
#[path = "progress.rs"]
mod progress;
#[path = "radio.rs"]
mod radio;
#[path = "rating.rs"]
mod rating;
#[path = "scroll.rs"]
mod scroll;
#[cfg(feature = "native-tests")]
#[path = "scroll_test.rs"]
pub(super) mod scroll_test;
#[path = "select.rs"]
mod select;
#[path = "slider_view.rs"]
mod slider_view;
#[path = "split_view.rs"]
mod split_view;
#[path = "toast.rs"]
mod toast;
#[path = "toast_clock.rs"]
mod toast_clock;
#[path = "tooltip.rs"]
mod tooltip;
#[path = "tree_drag.rs"]
mod tree_drag;
#[path = "tree_input_view.rs"]
mod tree_input;
#[path = "tree_typeahead.rs"]
mod tree_typeahead;
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

#[path = "table_view.rs"]
pub(super) mod table_view;

struct View {
    id: WindowId,
    window_title: String,
    session: SharedSession,
    transport: Arc<Transport>,
    images: BTreeMap<NodeId, image_view::State>,
    documents: BTreeMap<NodeId, document_view::State>,
    extensions: BTreeMap<NodeId, extension_view::State>,
    canvases: BTreeMap<NodeId, Rc<RefCell<canvas_view::State>>>,
    canvas_budget: Rc<RefCell<crate::canvas_paint::FrameBudget>>,
    splits: BTreeMap<NodeId, split_view::State>,
    split_activation: Option<gpui::Subscription>,
    buttons: BTreeMap<NodeId, Rc<ButtonState>>,
    selections: BTreeMap<NodeId, Rc<RefCell<crate::selection::State>>>,
    editors: BTreeMap<NodeId, editor::Instance>,
    root_focus: Option<gpui::FocusHandle>,
    focus: focus::Shared,
    selects: BTreeMap<NodeId, Rc<RefCell<select::State>>>,
    radios: BTreeMap<NodeId, Rc<RefCell<choice::State>>>,
    ratings: BTreeMap<NodeId, Rc<RefCell<rating::State>>>,
    sliders: BTreeMap<NodeId, slider_view::Shared>,
    numbers: BTreeMap<NodeId, number_input_view::Instance>,
    otps: BTreeMap<NodeId, otp_input_view::Instance>,
    calendars: BTreeMap<NodeId, calendar_view::Instance>,
    color_inputs: BTreeMap<NodeId, color_input_view::Instance>,
    tooltips: BTreeMap<NodeId, tooltip::State>,
    tooltip_last_closed: Option<std::time::Instant>,
    command_subscription: Option<gpui::Subscription>,
    menus: BTreeMap<NodeId, Rc<RefCell<menu::State>>>,
    menu_activation: Option<gpui::Subscription>,
    palettes: BTreeMap<NodeId, palette::State>,
    pointer_capture: pointer::Shared,
    pointer_activation: Option<gpui::Subscription>,
    toasts: BTreeMap<NodeId, toast::State>,
    toast_stacks: BTreeMap<NodeId, toast::Stack>,
    visited: std::collections::BTreeSet<NodeId>,
    #[cfg(feature = "native-tests")]
    probes: Rc<RefCell<BTreeMap<NodeId, native_test::Probe>>>,
    #[cfg(feature = "native-tests")]
    progress_probes: BTreeMap<NodeId, progress::Probe>,
    #[cfg(feature = "native-tests")]
    loading_probes: BTreeMap<NodeId, loading::Probe>,
    scrolls: BTreeMap<NodeId, Rc<scroll::State>>,
    lists: BTreeMap<NodeId, Rc<RefCell<list_view::State>>>,
    tables: BTreeMap<NodeId, Rc<RefCell<table_view::State>>>,
    tree_drag: std::rc::Weak<tree_drag::Lease>,
    animations: BTreeMap<NodeId, Rc<RefCell<animation::State>>>,
    navigation: BTreeMap<NodeId, Rc<RefCell<navigation::State>>>,
    carousels: BTreeMap<NodeId, Rc<RefCell<carousel::State>>>,
    carousel_activation: Option<gpui::Subscription>,
    animation_programs: BTreeMap<NodeId, Rc<RefCell<animation_program::State>>>,
    container_queries: BTreeMap<NodeId, container_query::State>,
    #[cfg(feature = "native-tests")]
    render_count: u64,
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

// Pointer-event inheritance uses the nearest explicit field, matching View rendering.
fn pointer_enabled(tree: &crate::tree::Tree, mut id: NodeId) -> bool {
    loop {
        let Some(node) = tree.get(id) else {
            return false;
        };
        for style in node.style.iter().rev() {
            if let Style::Fields(fields) = style {
                for field in fields.iter().rev() {
                    if let Field::PointerEvents(enabled) = field {
                        return *enabled;
                    }
                }
            }
        }
        let Some(parent) = node.parent else {
            return true;
        };
        id = parent;
    }
}
impl View {
    fn new(id: WindowId, session: SharedSession, transport: Arc<Transport>) -> Self {
        Self {
            id,
            window_title: String::new(),
            focus: focus::Manager::new(id, session.clone()),
            session,
            transport,
            images: BTreeMap::new(),
            documents: BTreeMap::new(),
            extensions: BTreeMap::new(),
            canvases: BTreeMap::new(),
            canvas_budget: Default::default(),
            splits: BTreeMap::new(),
            split_activation: None,
            buttons: BTreeMap::new(),
            selections: BTreeMap::new(),
            editors: BTreeMap::new(),
            root_focus: None,
            radios: BTreeMap::new(),
            ratings: BTreeMap::new(),
            sliders: BTreeMap::new(),
            numbers: BTreeMap::new(),
            otps: BTreeMap::new(),
            calendars: BTreeMap::new(),
            color_inputs: BTreeMap::new(),
            tooltips: BTreeMap::new(),
            tooltip_last_closed: None,
            command_subscription: None,
            menus: BTreeMap::new(),
            menu_activation: None,
            palettes: BTreeMap::new(),
            pointer_capture: Default::default(),
            pointer_activation: None,
            toasts: BTreeMap::new(),
            toast_stacks: BTreeMap::new(),
            selects: BTreeMap::new(),
            visited: Default::default(),
            #[cfg(feature = "native-tests")]
            probes: Default::default(),
            #[cfg(feature = "native-tests")]
            progress_probes: Default::default(),
            #[cfg(feature = "native-tests")]
            loading_probes: Default::default(),
            scrolls: Default::default(),
            lists: Default::default(),
            tables: Default::default(),
            tree_drag: Default::default(),
            animations: Default::default(),
            navigation: Default::default(),
            carousels: Default::default(),
            carousel_activation: None,
            animation_programs: Default::default(),
            container_queries: Default::default(),
            #[cfg(feature = "native-tests")]
            render_count: 0,
        }
    }
    fn update_editors(&mut self, dirty: &[NodeId], window: &mut Window, cx: &mut Context<Self>) {
        self.install_command_interceptor(window, cx);
        self.install_pointer_observer(window, cx);
        self.install_menu_observers(window, cx);
        self.sync_container_queries(dirty);
        self.sync_navigation(dirty);
        self.sync_lists(dirty, cx);
        self.sync_tables(dirty, window, cx);
        self.sync_images(dirty, window, cx);
        self.sync_documents(dirty, window, cx);
        self.sync_extensions(dirty, window, cx);
        self.sync_animations(dirty, cx);
        self.sync_programs(dirty, cx);
        self.sync_palettes(window, cx);
        self.sync_toasts(cx);
        self.sync_tooltips(window, cx);
        self.sync_carousels(window, cx);
        self.sync_canvases(dirty, window, cx);
        self.sync_splits(window, cx);
        self.sync_sliders(dirty, window, cx);
        self.sync_numbers(dirty, window, cx);
        self.sync_otps(dirty, window, cx);
        self.sync_calendars(dirty, window, cx);
        self.sync_color_inputs(dirty, window, cx);
        // An unselected query branch is hidden even before the first layout.
        // Do not count time waiting for its first visible paint as active motion.
        self.suspend_hidden_animations();
        self.suspend_hidden_programs();
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.editors.clear();
                self.scrolls.clear();
                return;
            };
            self.editors.retain(|id, _| tree.get(*id).is_some());
            self.scrolls.retain(|id, _| {
                tree.get(*id)
                    .is_some_and(|node| node.overlay.is_some() || scroll::declared(&node.style))
            });
            dirty
                .iter()
                .filter_map(|id| tree.get(*id))
                .filter(|node| node.editor.is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        for node in nodes {
            if let Some(editor) = self.editors.get_mut(&node.id) {
                editor.configure(
                    node.editor.as_ref().expect("validated editor"),
                    &node.accessibility,
                    window,
                    cx,
                );
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
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let element = self.element_body(tree, id, interaction, window, cx);
        let inert = tree
            .get(id)
            .is_some_and(|node| crate::style::inert(&node.style));
        if inert {
            crate::semantics::Inert(element).into_any_element()
        } else {
            element
        }
    }
    fn element_body(
        &mut self,
        tree: &crate::tree::Tree,
        id: NodeId,
        mut interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let node = tree.get(id).expect("validated retained node");
        if node.kind == Kind::CanvasView {
            return self.canvas_element(node, interaction, window, cx);
        }
        if node.kind == Kind::Extension {
            return self.extension_element(tree, node, interaction, window, cx);
        }
        if node.kind == Kind::DocumentView {
            return self.document_element(tree, node, interaction, window, cx);
        }
        if node.table.is_some() {
            return self.table_element(tree, node, interaction, window, cx);
        }
        if node.kind == Kind::VirtualList {
            return self.list_element(tree, node, interaction, window, cx);
        }
        if node.kind == Kind::ToastStack {
            return self.toast_stack_element(tree, node, interaction, window, cx);
        }
        if node.kind == Kind::Toast {
            return self.toast_element(tree, node, interaction, window, cx);
        }
        if node.kind == Kind::CommandPalette {
            return self.palette_element(node, interaction, window, cx);
        }
        if node.kind == Kind::Menu {
            return self.menu_element(tree, node, interaction, window, cx);
        }
        if matches!(node.kind, Kind::Tooltip | Kind::HoverCard) {
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
        // Native controls inside a tree/table row own pointer input. The input widget
        // may focus on mouse-down without consuming mouse-up; block the row's
        // ancestor click hitbox while preserving wheel propagation to the list.
        if (node.editor.is_some()
            || node.control.is_some()
            || node.choice.is_some()
            || node.rating.is_some()
            || node.slider.is_some()
            || node.number_input.is_some()
            || node.otp_input.is_some()
            || node.calendar.is_some()
            || node.color_input.is_some()
            || (node.kind == Kind::Text && interaction.selectable))
            && tree_input::within_input_collection(tree, id)
        {
            element = element.block_mouse_except_scroll();
        }

        if node.kind == Kind::Text && !interaction.selectable && !node.text.is_empty() {
            element = element
                .role(gpui::Role::Label)
                .aria_label(node.text.clone());
        }
        if let Some(description) = tree.tooltip_description(id) {
            element = element.aria_description(description.to_owned());
        }
        if matches!(
            node.kind,
            Kind::Container
                | Kind::Animated
                | Kind::AnimationProgram
                | Kind::ContainerQuery
                | Kind::TabPanel
                | Kind::Panel
                | Kind::Disclosure
                | Kind::Accordion
                | Kind::NavigationStack
                | Kind::Carousel
                | Kind::FocusScope
                | Kind::CommandScope
                | Kind::RadioGroup
                | Kind::Rating
                | Kind::TabBar
                | Kind::PointerArea
                | Kind::DragSource
                | Kind::DropTarget
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
        if node.container_query.is_some() || node.navigation_stack.is_some() {
            element = element.size_full();
        }
        if let Some(config) = &node.drag_source {
            element = element
                .role(gpui::Role::Group)
                .aria_label(config.label().to_owned());
        }
        if let Some(config) = &node.drop_target {
            element = element
                .role(gpui::Role::Group)
                .aria_label(config.label().to_owned());
        }
        if let Some(config) = &node.pointer {
            element = element
                .role(gpui::Role::Group)
                .aria_label(config.label.clone());
        }
        let mut image_corners = None;
        if let Some(config) = &node.image {
            let (image, corners) = self.image_element(tree, node, config, element, window, cx);
            element = image;
            image_corners = Some(corners);
        }
        if node.rating.is_some() {
            element = element
                .flex()
                .flex_row()
                .items_center()
                .flex_nowrap()
                .border_1()
                .border_color(gpui::transparent_black())
                .rounded(px(4.))
                .text_color(rgba(0xe8ad36ff))
                .focus(|style| style.border_color(rgba(0x6688ffff)));
        }
        if node.color_input.is_some() {
            element = element
                .w(px(296.))
                .min_w(px(0.))
                .text_size(px(13.))
                .overflow_hidden();
        }
        if node.calendar.is_some() {
            element = element
                .w(px(296.))
                .min_w(px(0.))
                .text_size(px(13.))
                .overflow_hidden();
        }
        if let Some(otp) = &node.otp_input {
            element = element
                .min_w(px(40.))
                .w(px(otp.config.policy.length() as f32 * 37. - 5.))
                .h(px(40.));
        }
        if node.number_input.is_some() {
            element = element.min_w(px(80.)).w(px(180.)).min_h(px(40.));
        }
        if let Some(slider) = &node.slider {
            element = element
                .relative()
                .min_w(px(24.))
                .min_h(px(24.))
                .text_color(rgba(0x6688ffff));
            element = match slider.config.axis {
                gpuio_protocol::slider::Axis::Horizontal => element.w(px(180.)).h(px(24.)),
                gpuio_protocol::slider::Axis::Vertical => element.w(px(24.)).h(px(180.)),
            };
        }
        if let Some(config) = &node.avatar {
            element = element
                .w(px(32.))
                .h(px(32.))
                .rounded(px(999.))
                .overflow_hidden()
                .bg(rgba(0x71809630))
                .text_color(rgba(0x718096ff))
                .text_size(px(12.));
            if let Some(label) = &config.label {
                element = element.role(gpui::Role::Image).aria_label(label.clone());
            }
            if config.source.is_none() {
                element = element.child(avatar::fallback(config.fallback.clone().into()));
            }
        }
        if let Some(config) = &node.loading {
            let corners = image_corners::Shared::default();
            image_corners = Some(corners.clone());
            let spinner = config.kind == gpuio_protocol::loading::Kind::Spinner;
            element = element
                .w(px(if spinner { 20. } else { 160. }))
                .h(px(if spinner { 20. } else { 16. }))
                .rounded(px(4.))
                .overflow_hidden()
                .text_color(rgba(0x8b98abff))
                .role(gpui::Role::ProgressIndicator)
                .aria_label(config.label.clone())
                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default()
                });
            if self.focus.borrow().visible(id) {
                element = element.child(loading::indicator(
                    config,
                    identity,
                    cx.reduce_motion(),
                    corners,
                    #[cfg(feature = "native-tests")]
                    self.loading_probes.entry(id).or_default().clone(),
                ));
            }
        }
        if let Some(config) = &node.progress {
            element = element
                .w(px(200.))
                .h(px(8.))
                .rounded(px(4.))
                .overflow_hidden()
                .bg(rgba(0x80808040))
                .text_color(rgba(0x4d8cffff))
                .role(gpui::Role::ProgressIndicator)
                .aria_label(config.label.clone())
                .aria_min_numeric_value(0.)
                .aria_max_numeric_value(100.)
                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default();
                });
            if let Some(fraction) = config.fraction {
                element = element.aria_numeric_value(fraction * 100.);
            }
            if self.focus.borrow().visible(id) {
                element = element.child(progress::indicator(
                    config.fraction,
                    identity,
                    cx.reduce_motion(),
                    #[cfg(feature = "native-tests")]
                    self.progress_probes.entry(id).or_default().clone(),
                ));
            }
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
            || node
                .drag_source
                .as_ref()
                .is_some_and(|config| config.disabled())
            || node
                .drop_target
                .as_ref()
                .is_some_and(|config| config.disabled())
            || node.pointer.as_ref().is_some_and(|config| config.disabled)
            || node.choice.as_ref().is_some_and(|config| config.disabled)
            || node
                .number_input
                .as_ref()
                .is_some_and(|n| n.config.disabled)
            || node.otp_input.as_ref().is_some_and(|n| n.config.disabled)
            || node.calendar.as_ref().is_some_and(|n| n.config.disabled)
            || node.color_input.as_ref().is_some_and(|n| n.config.disabled)
            || node.rating.as_ref().is_some_and(|config| config.disabled)
            || node
                .slider
                .as_ref()
                .is_some_and(|slider| slider.config.disabled)
            || node.control.is_some_and(Control::disabled)
            || node.editor.as_ref().is_some_and(|config| config.disabled);
        let checked = command
            .as_ref()
            .is_some_and(|route| route.config.checked == Some(true))
            || matches!(
                node.control,
                Some(Control::Checkbox(CheckState::Checked, _) | Control::Switch(true, _))
            );
        let indeterminate = node.loading.is_some()
            || node
                .progress
                .as_ref()
                .is_some_and(|config| config.fraction.is_none())
            || matches!(
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
                | Kind::Rating
                | Kind::TabBar
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
                    Kind::Rating => gpui::Role::Slider,
                    Kind::TabBar => gpui::Role::TabList,
                    Kind::Select => gpui::Role::ComboBox,
                    _ => gpui::Role::Button,
                })
                .aria_label(accessible_name.clone());
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
            if interaction.pointer
                && !disabled
                && !node.rating.as_ref().is_some_and(|config| config.read_only)
            {
                element = element.cursor_pointer();
            }
        }
        if node.kind == Kind::TabBar {
            element = element.flex_row();
        }
        if matches!(
            node.kind,
            Kind::TabPanel | Kind::Panel | Kind::NavigationStack | Kind::Carousel
        ) {
            element = element
                .role(if node.kind == Kind::TabPanel {
                    gpui::Role::TabPanel
                } else {
                    gpui::Role::Region
                })
                .aria_label(accessible_name);
        }
        if matches!(node.kind, Kind::Disclosure | Kind::Accordion) {
            element = element.role(gpui::Role::Group);
        }
        if let Some(parent) = tree.disclosure_for_trigger(id) {
            let expanded = self.focus.borrow().visible(parent.children[1]);
            let gate = self.focus.clone();
            element = element
                .aria_expanded(expanded)
                .on_key_down(move |event, window, cx| {
                    let modifiers = event.keystroke.modifiers;
                    if !modifiers.control
                        && !modifiers.alt
                        && !modifiers.platform
                        && !modifiers.shift
                        && gate
                            .borrow()
                            .disclosure_key(id, &event.keystroke.key, window, cx)
                    {
                        cx.stop_propagation();
                    }
                });
        }
        if node.carousel.is_some() {
            element = self.carousel_element(element, node, cx);
        }
        let animation = self.animation_frame(node, window, cx);
        let program = self.program_frame(node, window, cx);
        let styles = animation
            .as_ref()
            .map(|(state, _)| state.borrow().styles.clone())
            .or_else(|| {
                program
                    .as_ref()
                    .map(|(state, _)| state.borrow().styles.clone())
            })
            .unwrap_or_else(|| node.style.clone());
        let (styled, mut states) = apply_styles(element, &styles, interaction, disabled);
        if let Some(config) = &node.overlay {
            for state in states.iter_mut().flatten() {
                overlay::constrain_state_style(config.kind, state);
            }
        }
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
            } else if let Some(color) = self.color_inputs.get(&id) {
                if color.focused(window, cx) {
                    element.style().refine(&style);
                }
            } else if let Some(calendar) = self.calendars.get(&id) {
                if calendar.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else if let Some(otp) = self.otps.get(&id) {
                if otp.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else if let Some(number) = self.numbers.get(&id) {
                if number.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else if let Some(slider) = self.sliders.get(&id) {
                if slider
                    .borrow()
                    .focus
                    .iter()
                    .any(|(_, focus)| focus.is_focused(window))
                {
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
            if node.pointer.is_some() {
                if self.pointer_capture.borrow().is_active(id) {
                    element.style().refine(&style);
                }
            } else {
                element = element.active(move |_| style);
            }
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
        if let Some((_, sample)) = &animation {
            animation::apply(element.style(), &sample.values);
        }
        if let Some((_, sample)) = &program {
            animation::apply(element.style(), &sample.frame.values);
        }
        let scrolling = if scroll::declared(&node.style)
            || element.style().overflow.x == Some(gpui::Overflow::Scroll)
            || element.style().overflow.y == Some(gpui::Overflow::Scroll)
        {
            let state = self.scrolls.entry(id).or_default().clone();
            element = scroll::attach(element, &state);
            Some(state)
        } else {
            None
        };
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
        } else if let Some(color) = self.color_inputs.get(&id) {
            self.visited.insert(id);
            element = color.element(element, interaction.pointer, cx);
        } else if let Some(calendar) = self.calendars.get(&id) {
            self.visited.insert(id);
            element = calendar.element(element, interaction.pointer, cx);
        } else if let Some(otp) = self.otps.get(&id) {
            self.visited.insert(id);
            element = otp.element(element, interaction.pointer, cx);
        } else if let Some(number) = self.numbers.get(&id) {
            self.visited.insert(id);
            element = number.element(element, interaction.pointer, cx);
        } else if node.slider.is_some() {
            self.visited.insert(id);
            if let Some(state) = self.sliders.get(&id) {
                element = slider_view::element(element, state.clone(), interaction.pointer, window);
            }
        } else if let Some(config) = &node.rating {
            let state = self.ratings.entry(id).or_default().clone();
            let route = node
                .handler
                .filter(|_| !disabled && !config.read_only)
                .map(|handler| choice::Route {
                    window: self.id,
                    node: id,
                    handler,
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                });
            element = rating::element(
                element,
                rating::Render {
                    config,
                    state,
                    focus: self.buttons[&id].focus.clone(),
                    route,
                    pointer: interaction.pointer,
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
                        tabs: node.kind == Kind::TabBar,
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
        } else if matches!(node.kind, Kind::Button | Kind::CommandButton)
            && !node.children.is_empty()
        {
            let [leading, trailing] = node.children.as_ref() else {
                unreachable!("validated button icon slots")
            };
            if tree
                .get(*leading)
                .is_some_and(|slot| !slot.children.is_empty())
            {
                element = element.child(self.element(tree, *leading, interaction, window, cx));
            }
            if !label.is_empty() {
                element = element.child(gpui::SharedString::from(label));
            }
            if tree
                .get(*trailing)
                .is_some_and(|slot| !slot.children.is_empty())
            {
                element = element.child(self.element(tree, *trailing, interaction, window, cx));
            }
        } else if !label.is_empty()
            && !matches!(
                node.kind,
                Kind::TabPanel | Kind::Panel | Kind::NavigationStack | Kind::Carousel
            )
        {
            element = element.child(gpui::SharedString::from(label));
        }
        for child in node.children.iter() {
            if tree
                .get(*child)
                .and_then(|node| node.overlay.as_ref())
                .is_some_and(|config| config.kind == OverlayKind::Popover)
                && let Some(anchor) = self.focus.borrow().anchor(*child)
            {
                element = element.child(
                    canvas(move |bounds, _, _| anchor.set(bounds), |_, _, _, _| {})
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                );
            }
        }
        if node.navigation_stack.is_some() {
            element = element.child(self.navigation_element(tree, node, interaction, window, cx));
        } else if node.container_query.is_some() {
            element = element.child(self.container_query_element(tree, node, interaction, cx));
        } else if node.split.is_some() {
            element = element.child(self.split_element(tree, node, interaction, window, cx));
        } else {
            element = element.children(
                node.children
                    .iter()
                    .filter(|_| !matches!(node.kind, Kind::Button | Kind::CommandButton))
                    .map(|id| self.element(tree, *id, interaction, window, cx))
                    .collect::<Vec<_>>(),
            );
        }

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
            && node.rating.is_none()
            && node.slider.is_none()
            && node.number_input.is_none()
            && node.otp_input.is_none()
            && node.calendar.is_none()
            && node.color_input.is_none()
            && node.overlay.is_none()
            && node.pointer.is_none()
            && node.image.is_none()
            && node.animation.is_none()
            && node.animation_program.is_none()
            && node.split.is_none()
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
            .or_else(|| self.numbers.get(&id).map(|number| number.focus_handle(cx)))
            .or_else(|| self.otps.get(&id).map(|otp| otp.focus_handle(cx)))
            .or_else(|| {
                self.calendars
                    .get(&id)
                    .map(|calendar| calendar.focus_handle(cx))
            })
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
                scrolling.as_ref(),
                window,
            );
        }
        if let Some(config) = &node.drag_source
            && interaction.pointer
            && !disabled
        {
            element = drag_drop::source(
                element,
                choice::Route {
                    window: self.id,
                    node: id,
                    handler: node.handler.expect("validated drag/drop"),
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                },
                config.clone(),
                cx,
            );
        }
        if let Some((state, sample)) = animation {
            element = element.child(animation::paint(&state, sample));
        }
        if let Some((state, sample)) = program {
            element = element.child(animation_program::paint(&state, sample));
        }
        if let Some(corners) = image_corners {
            let image = image_corners::Rounded::capture(element, corners);
            return match &scrolling {
                Some(state) => self.finish_element(
                    scroll::Frame::new(image, state),
                    node,
                    tree.revision(),
                    disabled,
                ),
                None => self.finish_element(image, node, tree.revision(), disabled),
            };
        }
        match scrolling {
            Some(state) => self.finish_element(
                scroll::Frame::new(element, &state),
                node,
                tree.revision(),
                disabled,
            ),
            None => self.finish_element(element, node, tree.revision(), disabled),
        }
    }
    fn finish_element<E: gpui::Element>(
        &self,
        element: E,
        node: &crate::tree::Node,
        revision: i64,
        disabled: bool,
    ) -> gpui::AnyElement {
        let id = node.id;
        // The managed list row wrapper owns this metadata and the row focus
        // handle. Rendering it again on the description would duplicate AX rows.
        let tree_row_metadata = node
            .parent
            .is_some_and(|parent| self.lists.contains_key(&parent))
            && node.accessibility.as_ref().is_some_and(|metadata| {
                matches!(
                    metadata.role,
                    Some(gpuio_protocol::accessibility::Role::TreeItem(_))
                )
            });
        let element = crate::semantics::State {
            hidden: !self.focus.borrow().visible(node.id),
            metadata: if node.editor.is_none()
                && !tree_row_metadata
                && node.number_input.is_none()
                && node.otp_input.is_none()
                && node.calendar.is_none()
                && node.color_input.is_none()
            {
                node.accessibility.clone()
            } else {
                None
            },
            live: None,
            element,
            disabled,
            read_only: node
                .number_input
                .as_ref()
                .is_some_and(|n| n.config.read_only)
                || node.otp_input.as_ref().is_some_and(|n| n.config.read_only)
                || node.rating.as_ref().is_some_and(|config| config.read_only)
                || node
                    .slider
                    .as_ref()
                    .is_some_and(|slider| slider.config.read_only),
            modal: false,
        };
        if node.slider.is_some()
            && let Some(state) = self.sliders.get(&id)
        {
            return slider_view::Region {
                element,
                state: state.clone(),
            }
            .into_any_element();
        }
        if node.drop_target.is_some() {
            return drag_drop::Region {
                element,
                route: choice::Route {
                    window: self.id,
                    node: id,
                    handler: node.handler.expect("validated drag/drop"),
                    revision,
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                },
            }
            .into_any_element();
        }
        if let Some(config) = &node.pointer {
            pointer::Region {
                element,
                capture: self.pointer_capture.clone(),
                config: config.clone(),
                route: choice::Route {
                    window: self.id,
                    node: id,
                    handler: node.handler.expect("validated pointer region"),
                    revision,
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                },
            }
            .into_any_element()
        } else {
            element.into_any_element()
        }
    }
}
impl Render for View {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(feature = "native-tests")]
        {
            self.render_count += 1;
        }
        self.refresh_list_pins(window, cx);
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
        let program_begin = cx.entity().downgrade();
        let canvas_budget = self.canvas_budget.clone();
        let canvases: Vec<_> = self.canvases.values().map(Rc::downgrade).collect();
        let drag_window = self.id;
        let mut root = drag_drop::root(div(), self.id, cx)
            .capture_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape"
                    && (view.cancel_color_inputs(
                        gpuio_protocol::color_input::CancelReason::Escape,
                        window,
                        cx,
                    ) | view.cancel_tree_drag(window, cx)
                        | view.cancel_carousel_drags(window, cx)
                        | view.cancel_split_drag(window, cx)
                        | view.cancel_slider_drags(
                            gpuio_protocol::slider::CancelReason::Escape,
                            window,
                        ))
                {
                    cx.stop_propagation();
                }
            }))
            .on_action(
                cx.listener(|view, action: &menu_platform::Invoke, window, cx| {
                    view.platform_menu_action(action, window, cx)
                }),
            )
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
                    move |_, _, window, cx| {
                        begin_focus.borrow_mut().begin_frame();
                        let _ = program_begin.update(cx, |view, _| {
                            view.begin_carousel_paint();
                            view.begin_program_paint();
                            view.begin_query_paint();
                        });
                        *canvas_budget.borrow_mut() = Default::default();
                        for state in &canvases {
                            if let Some(state) = state.upgrade() {
                                state.borrow_mut().flush_canvas_frame(window, cx);
                            }
                        }
                        drag_drop::install_cleanup(drag_window, window, cx);
                    },
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
        for (id, state) in &mut self.documents {
            if !self.visited.contains(id) && !state.retained(window, cx) {
                state.presentation = None;
            }
        }
        self.hide_unvisited_extensions();
        self.hide_unvisited_canvases(window);
        self.hide_unvisited_sliders(window, cx);
        self.hide_unvisited_numbers(window, cx);
        self.hide_unvisited_otps(window, cx);
        self.hide_unvisited_calendars(window, cx);
        self.hide_unvisited_color_inputs(window, cx);
        self.buttons.retain(|id, _| self.visited.contains(id));
        self.radios.retain(|id, _| self.visited.contains(id));
        self.ratings.retain(|id, _| self.visited.contains(id));
        self.selects.retain(|id, _| self.visited.contains(id));
        self.menus.retain(|id, _| self.visited.contains(id));
        self.sync_platform_menus(window, cx);
        #[cfg(feature = "native-tests")]
        self.progress_probes.retain(|id, _| {
            session
                .tree(self.id)
                .and_then(|tree| tree.get(*id))
                .is_some_and(|node| node.progress.is_some())
        });
        #[cfg(feature = "native-tests")]
        self.loading_probes.retain(|id, _| {
            session
                .tree(self.id)
                .and_then(|tree| tree.get(*id))
                .is_some_and(|node| node.loading.is_some())
        });
        self.selections.retain(|id, _| self.visited.contains(id));
        // GPUI routes key events along the focused element's ancestry. Keep a
        // non-tab-stop fallback so Tab also works before the first click and
        // after a focused control is disabled or removed.
        let has_focus =
            self.palettes.values().any(|state| {
                !state.closed && state.query.read(cx).focus_handle(cx).is_focused(window)
            }) || self
                .menus
                .values()
                .any(|state| state.borrow().focus.is_focused(window))
                || self
                    .toasts
                    .values()
                    .any(|state| state.close_focus.is_focused(window))
                || self
                    .splits
                    .values()
                    .any(|state| state.focus.is_focused(window))
                || self
                    .tables
                    .values()
                    .any(|state| state.borrow().focused(window, cx))
                || self.lists.values().any(|state| {
                    let state = state.borrow();
                    state.tree_focus.is_some() && state.owns_tree_focus(window)
                })
                || self.focus.borrow().contains_focus(window)
                || self
                    .carousels
                    .values()
                    .any(|state| state.borrow().focused(window))
                || self
                    .numbers
                    .values()
                    .any(|number| number.focus_handle(cx).is_focused(window))
                || self
                    .otps
                    .values()
                    .any(|otp| otp.focus_handle(cx).is_focused(window))
                || self
                    .calendars
                    .values()
                    .any(|calendar| calendar.focus_handle(cx).is_focused(window))
                || self
                    .color_inputs
                    .values()
                    .any(|state| state.focused(window, cx))
                || self.sliders.values().any(|state| {
                    state
                        .borrow()
                        .focus
                        .iter()
                        .any(|(_, focus)| focus.is_focused(window))
                })
                || self
                    .documents
                    .values()
                    .any(|state| state.focused(window, cx))
                || self
                    .extensions
                    .values()
                    .any(|state| state.focus.contains_focused(window, cx))
                || self
                    .canvases
                    .values()
                    .any(|state| state.borrow().canvas_focused(window))
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
        let program_finish = cx.entity().downgrade();
        let navigation_focus = self.focus.clone();
        root.child(
            canvas(
                |_, _, _| (),
                move |_, _, window, cx| {
                    // Deferred popups paint after the root tree. Sweep only once
                    // the complete effect cycle has painted; never request a frame.
                    if program_finish
                        .update(cx, |view, _| {
                            !view.animation_programs.is_empty()
                                || !view.container_queries.is_empty()
                                || !view.carousels.is_empty()
                        })
                        .unwrap_or(false)
                    {
                        window.defer(cx, move |window, cx| {
                            let _ = program_finish.update(cx, |view, cx| {
                                view.finish_query_paint(window, cx);
                                view.finish_program_paint();
                                view.schedule_carousels(window, cx);
                            });
                        });
                    }
                    if navigation_focus.borrow().navigation_pending() {
                        window.defer(cx, move |window, cx| {
                            navigation_focus.borrow_mut().finish_navigation(window, cx);
                        });
                    }
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
    let platform = gpui_platform::current_platform(false);
    let application = gpui::Application::with_platform(platform.clone());
    let desktop = crate::desktop_host::install(&application, transport.clone());
    let reopen_transport = transport.clone();
    application.on_reopen(move |_| window_host::control(&reopen_transport, Event::ReopenRequested));
    application.run(move |cx: &mut App| {
        // GPUI on_quit is cleanup-only on both platforms. AppKit's separate
        // applicationShouldTerminate hook supplies the asynchronous decision.
        #[cfg(target_os="macos")]
        window_macos::install(cx,transport.clone());
        window_host::control(&transport,Event::WindowCapabilities(window_host::capabilities()));
        gpui_base::init(cx);
        gpuio_table_adapter::init(cx);
        crate::image_host::init(cx);
        let motion = crate::motion_preference::init(cx);
        // GPUI defaults to last-window exit on Linux. Our explicit lifecycle
        // policy must control background applications consistently on both OSes.
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let session = Rc::new(RefCell::new(Session::default()));
        crate::motion_preference::bind_clocks(&session.borrow().motion(), cx);
        let mut windows: BTreeMap<WindowId, WindowHandle<View>> = BTreeMap::new();
        let dialogs = crate::file_dialog::Dialogs::default();
        let quit_dialogs = dialogs.clone();
        let quit_motion = motion.clone();
        let quit_desktop = desktop.clone();
        cx.on_app_quit(move |cx| {
            quit_desktop.borrow_mut().close();
            quit_motion.borrow_mut().take();
            drag_drop::shutdown(cx);
            quit_dialogs.finish_before_quit();
            std::future::ready(())
        })
        .detach();
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
                    desktop.borrow_mut().close();
                    motion.borrow_mut().take();
                    dialogs.clear().wait().await;
                    crate::canvas_host::shutdown(cx).await;
                    crate::image_host::shutdown(cx).await;
                    crate::document_host::shutdown(cx).await;
                    if !stopping.replace(true) {
                        cx.update(stop_application);
                    }
                    return;
                }
                // Bound work per wake; yield to native input/paint between batches.
                for _ in 0..crate::mailbox::MAX_COMMANDS {
                    let message = transport.mailbox.lock().expect("mailbox poisoned").pop();
                    let Some(message) = message else {
                        break;
                    };
                    let message = match message {
                        Message::Open(correlation,id,title,width,height) => Message::OpenConfigured(correlation,id,gpuio_protocol::window::Config{title,width,height,focus:true,chrome:gpuio_protocol::window::Chrome::Standard,resizable:true}),
                        message=>message,
                    };
                    match message {
                        Message::Hello(version, caps) => {
                            failed(&transport, 0, session.borrow_mut().hello(version, caps))
                        }
                        Message::Open(..)=>unreachable!("normalized above"),
                        Message::OpenConfigured(correlation,id,config)=> {
                            let gpuio_protocol::window::Config {title,width,height,focus,chrome,resizable}=config;
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
                                        focus,
                                        is_resizable:resizable,
                                        titlebar: (chrome==gpuio_protocol::window::Chrome::Standard).then(|| gpui::TitlebarOptions {
                                            title: Some(title.clone().into()),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        window.set_window_title(&title);
                                        cx.new(|cx| {
                                            let mut view=View::new(id, session.clone(), transport.clone());
                                            view.window_title=title.clone();
                                            window_host::watch(&view,window,cx);
                                            view
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
                                    if focus {cx.update(|cx| cx.activate(true));}
                                }
                                Err(_) => transport
                                    .respond(Event::Failed(correlation, ErrorCode::NativeFailure)),
                            }
                        }
                        Message::Apply(tx) => {
                            let pins = windows.get(&tx.window).and_then(|handle|
                                handle.update(cx, |view, window, cx| view.list_pins(window,cx)).ok()).unwrap_or_default();
                            let result = session.borrow_mut().apply_guarded(&tx, &pins);
                            match result {
                                Ok(applied) => {
                                    transport.respond(Event::Accepted(tx.window, tx.revision));
                                    if let Some(window) = windows.get(&tx.window) {
                                        let _ = window.update(cx, |view, window, cx| {
                                            view.update_editors(&applied.dirty, window, cx);
                                            view.list_actions(&applied.lists, window, cx);
                                            view.table_actions(&applied.tables, window, cx);
                                            cx.notify();
                                        });
                                    }
                                }
                                Err(crate::tree::ApplyFailure::Retained(rows)) =>
                                    transport.respond(Event::ListRetained(tx.window, tx.revision, rows)),
                                Err(crate::tree::ApplyFailure::Rejected(error)) => transport.respond(Event::Rejected(
                                    tx.window, tx.revision, error,
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
                        Message::ColorInputCommand(correlation, id, node, command) => {
                            use gpuio_protocol::color_input::{Error, Response};
                            let result = windows.get(&id).and_then(|handle| handle.update(cx, |view, window, cx| {
                                view.color_inputs.get(&node).map(|input| input.command(&command, window, cx)).unwrap_or(Response::Failed(Error::StaleColorInput))
                            }).ok()).unwrap_or(Response::Failed(Error::Closed));
                            transport.respond(Event::ColorInputResult(correlation, id, node, result));
                        }
                        Message::CalendarCommand(correlation, id, node, command) => {
                            use gpuio_protocol::calendar_input::{Error, Response};
                            let result = windows.get(&id).and_then(|handle| handle.update(cx, |view, window, cx| {
                                view.calendars.get(&node).map(|input| input.command(&command, window, cx)).unwrap_or(Response::Failed(Error::StaleInput))
                            }).ok()).unwrap_or(Response::Failed(Error::Closed));
                            transport.respond(Event::CalendarResult(correlation, id, node, result));
                        }
                        Message::OtpInputCommand(correlation, id, node, command) => {
                            use gpuio_protocol::otp_input::{Error, Response};
                            let result = windows.get(&id).and_then(|handle| handle.update(cx, |view, window, cx| {
                                view.otps.get(&node).map(|input| input.command(&command, window, cx)).unwrap_or(Response::Failed(Error::StaleInput))
                            }).ok()).unwrap_or(Response::Failed(Error::Closed));
                            transport.respond(Event::OtpInputResult(correlation, id, node, result));
                        }
                        Message::NumberInputCommand(correlation, id, node, command) => {
                            use gpuio_protocol::number_input::{Error, Response};
                            let result = windows.get(&id).and_then(|handle| handle.update(cx, |view, window, cx| {
                                let result = view.numbers.get(&node).map(|number| number.command(&command, window, cx)).unwrap_or(Response::Failed(Error::StaleInput));
                                cx.notify();
                                result
                            }).ok()).unwrap_or(Response::Failed(Error::Closed));
                            transport.respond(Event::NumberInputResult(correlation, id, node, result));
                        }
                        Message::SliderCommand(correlation, id, node, command) => {
                            use gpuio_protocol::slider::{Error, Response};
                            let result = windows.get(&id)
                                .and_then(|handle| handle.update(cx, |view, window, cx| {
                                    view.slider_command(node, command, window, cx)
                                }).ok())
                                .unwrap_or(Response::Failed(Error::Closed));
                            transport.respond(Event::SliderResult(correlation, id, node, result));
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
                        Message::FileDialog(correlation, id, config) => {
                            let presented = match windows.get(&id) {
                                Some(handle) => cx
                                    .update_window((*handle).into(), |_, window, cx| {
                                        dialogs.show(
                                            correlation,
                                            id,
                                            config,
                                            window,
                                            cx,
                                            transport.clone(),
                                        );
                                    })
                                    .is_ok(),
                                None => false,
                            };
                            if !presented {
                                transport.respond(Event::FileDialogResult(
                                    correlation,
                                    id,
                                    FileDialogResult::Failed(FileDialogError::Closed),
                                ));
                            }
                        }
                        Message::Close(correlation, id) => {
                            dialogs.close(id).wait().await;
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
                                            .update(cx, |view, window, cx| { drag_drop::cancel(view.id, gpuio_protocol::drag_drop::CancelReason::WindowClosed, window, cx); view.cancel_tree_drag(window, cx); view.close_color_inputs(window, cx); view.cancel_split_drag(window, cx); view.extensions.clear(); for state in view.canvases.values() { state.borrow_mut().close(window); } view.canvases.clear(); window.remove_window(); });
                                    }
                                }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
                        }
                        Message::WindowCommand(correlation,id,command)=>{
                            let result=match windows.get(&id) {
                                Some(handle)=>handle.update(cx,|view,window,cx| {let result=window_host::command(view,&command,window);window_host::observe(view,window);cx.notify();result}).unwrap_or(gpuio_protocol::window::Response::Failed(gpuio_protocol::window::Error::Closed)),
                                None=>gpuio_protocol::window::Response::Failed(gpuio_protocol::window::Error::Closed),
                            };
                            transport.respond(Event::WindowResponse(correlation,id,result));
                        }
                        Message::Desktop(correlation, request) => {
                            match session.borrow().check_ready() {
                                Ok(()) => {
                                    cx.update(|cx| crate::desktop_host::dispatch(&desktop, correlation, request, cx, &transport));
                                }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
                        }
                        Message::Asset(correlation, request) => {
                            let response = session.borrow_mut().asset_request(request);
                            transport.respond(Event::AssetResponse(correlation, response));
                        }
                        Message::Document(correlation, request) => {
                            let published = match &request { gpuio_protocol::document::Request::Publish(id,_) => Some(*id), _ => None };
                            let response = session.borrow_mut().document_request(request);
                            if matches!(response, gpuio_protocol::document::Response::Ack) && let Some(source) = published {
                                for handle in windows.values() {
                                    let _ = handle.update(cx,|view,_,cx| view.document_changed(source,cx));
                                }
                            }
                            transport.respond(Event::DocumentResponse(correlation, response));
                        }
                        Message::Canvas(correlation, request) => {
                            let published=match &request { gpuio_protocol::canvas_resource::Request::Publish(id,_) => Some(*id), _ => None };
                            let response=session.borrow_mut().canvas_request(request);
                            if let Some(source)=published && matches!(response,gpuio_protocol::canvas_resource::Response::Ack) {
                                for handle in windows.values() { let _=handle.update(cx,|view,window,cx|view.canvas_changed(source,window,cx)); }
                            }
                            transport.respond(Event::CanvasResponse(correlation,response));
                        }
                        Message::SetMotion(preference) => {
                            match session.borrow().check_ready() {
                                Ok(()) => cx.update(|cx| crate::motion_preference::set(preference, cx)),
                                Err(error) => transport.respond(Event::Failed(0, error)),
                            }
                        }
                        Message::Shutdown => {
                            desktop.borrow_mut().close();
                            motion.borrow_mut().take();
                            dialogs.clear().wait().await;
                            crate::canvas_host::shutdown(cx).await;
                            crate::image_host::shutdown(cx).await;
                            crate::document_host::shutdown(cx).await;
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
pub(crate) fn stop_application(cx: &mut App) {
    use cocoa::{
        appkit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventSubtype, NSEventType},
        base::{YES, nil},
        foundation::NSPoint,
    };
    drag_drop::shutdown(cx);
    crate::canvas_host::finish_before_quit(cx);
    crate::image_host::finish_before_quit(cx);
    crate::document_host::finish_before_quit(cx);
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
pub(crate) fn stop_application(cx: &mut App) {
    drag_drop::shutdown(cx);
    crate::canvas_host::finish_before_quit(cx);
    crate::image_host::finish_before_quit(cx);
    crate::document_host::finish_before_quit(cx);
    cx.quit();
}

#[cfg(feature = "native-tests")]
#[path = "native_test.rs"]
pub(crate) mod native_test;

#[cfg(feature = "native-tests")]
#[path = "control_test.rs"]
pub(crate) mod control_test;

#[cfg(all(feature = "native-tests", target_os = "macos"))]
#[path = "window_test.rs"]
pub(super) mod window_test;
