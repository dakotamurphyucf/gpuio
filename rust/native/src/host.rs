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
#[path = "chart_view.rs"]
pub(crate) mod chart_view;
#[path = "container_query_view.rs"]
mod container_query;
#[cfg(feature = "native-tests")]
#[path = "container_query_test.rs"]
pub(super) mod container_query_test;
#[path = "editor_frame_view.rs"]
mod editor_frame_view;
#[path = "extension_view.rs"]
mod extension_view;
#[path = "input_content_view.rs"]
mod input_content_view;
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
#[path = "avatar_slot.rs"]
mod avatar_slot;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "button_view_test.rs"]
mod button_view_test;
#[path = "calendar_view.rs"]
pub(super) mod calendar_view;
#[path = "carousel_view.rs"]
mod carousel;
#[path = "carousel_track_view.rs"]
mod carousel_track;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "carousel_track_view_test.rs"]
mod carousel_track_view_test;
#[path = "checkable.rs"]
mod checkable;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "checkable_navigation_view_test.rs"]
mod checkable_navigation_view_test;
#[path = "choice.rs"]
mod choice;
#[path = "choice_picker_host.rs"]
mod choice_picker_host;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "choice_picker_host_test.rs"]
mod choice_picker_host_test;
#[path = "choice_picker_layout.rs"]
mod choice_picker_layout;
#[path = "choice_picker_semantics.rs"]
mod choice_picker_semantics;
#[path = "choice_picker_view.rs"]
mod choice_picker_view;
#[path = "choice_popup.rs"]
mod choice_popup;
#[path = "color_input_view.rs"]
pub(super) mod color_input_view;
#[path = "combobox.rs"]
mod combobox;
#[path = "command.rs"]
mod command;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "control_appearance_view_test.rs"]
mod control_appearance_view_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "control_labels_view_test.rs"]
mod control_labels_view_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "disclosure_view_test.rs"]
mod disclosure_view_test;
#[path = "document_view.rs"]
pub(crate) mod document_view;
#[path = "drag_drop.rs"]
mod drag_drop;
#[path = "editor.rs"]
mod editor;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "editor_privacy_test.rs"]
mod editor_privacy_test;
#[cfg(feature = "native-tests")]
#[path = "editor_test.rs"]
pub(super) mod editor_test;
#[path = "focus.rs"]
mod focus;
#[path = "highlight_view.rs"]
pub(crate) mod highlight;
#[path = "highlight_style.rs"]
mod highlight_style;
#[path = "hover_observation.rs"]
mod hover_observation;
#[path = "image_corners.rs"]
mod image_corners;
#[path = "image_view.rs"]
pub(crate) mod image_view;
#[path = "input_region.rs"]
mod input_region;
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
#[path = "node_actions.rs"]
mod node_actions;
#[path = "node_content.rs"]
mod node_content;
#[path = "node_focus.rs"]
mod node_focus;
#[path = "node_presentation.rs"]
mod node_presentation;
#[path = "node_style.rs"]
mod node_style;
#[path = "number_frame_view.rs"]
mod number_frame_view;
#[path = "number_input_view.rs"]
pub(super) mod number_input_view;
#[path = "otp_input_view.rs"]
pub(super) mod otp_input_view;
#[path = "overlay.rs"]
mod overlay;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "overlay_backdrop_test.rs"]
mod overlay_backdrop_test;
#[path = "overlay_entry.rs"]
mod overlay_entry;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "overlay_motion_test.rs"]
mod overlay_motion_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "pagination_view_test.rs"]
mod pagination_view_test;
#[path = "palette.rs"]
mod palette;
#[path = "pointer.rs"]
mod pointer;
#[path = "popover_semantics.rs"]
mod popover_semantics;
#[path = "sheet_geometry.rs"]
mod sheet_geometry;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "sheet_insets_test.rs"]
mod sheet_insets_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "sidebar_labels_view_test.rs"]
mod sidebar_labels_view_test;
#[path = "window_regions.rs"]
pub(crate) mod window_regions;

#[path = "list_input_view.rs"]
mod list_input;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "list_input_host_test.rs"]
mod list_input_host_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "placement_geometry_test.rs"]
mod placement_geometry_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "popover_semantics_test.rs"]
mod popover_semantics_test;
#[path = "popup.rs"]
mod popup;
#[path = "progress.rs"]
mod progress;
#[path = "radio.rs"]
mod radio;
#[path = "rating.rs"]
mod rating;
#[path = "reveal_view.rs"]
mod reveal;
#[path = "scroll.rs"]
mod scroll;
#[cfg(feature = "native-tests")]
#[path = "scroll_test.rs"]
pub(super) mod scroll_test;
#[path = "scrollbar_host.rs"]
mod scrollbar_host;
#[path = "select.rs"]
mod select;
#[path = "slider_view.rs"]
mod slider_view;
#[path = "split_view.rs"]
mod split_view;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "stepper_view_test.rs"]
mod stepper_view_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "tab_frame_test.rs"]
mod tab_frame_test;
#[path = "tab_menu.rs"]
mod tab_menu;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "tab_menu_test.rs"]
mod tab_menu_test;
#[path = "tab_motion.rs"]
mod tab_motion;
#[path = "tab_presentation.rs"]
mod tab_presentation;
#[path = "tab_viewport.rs"]
mod tab_viewport;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "tab_viewport_test.rs"]
mod tab_viewport_test;
#[path = "toast.rs"]
mod toast;
#[path = "toast_clock.rs"]
mod toast_clock;
#[path = "tooltip.rs"]
mod tooltip;
#[path = "tooltip_motion.rs"]
mod tooltip_motion;
#[path = "tree_drag.rs"]
mod tree_drag;
#[path = "tree_input_view.rs"]
mod tree_input;
#[path = "tree_typeahead.rs"]
mod tree_typeahead;
#[path = "typeahead.rs"]
mod typeahead;
struct FlowPlacement<'a> {
    node: NodeId,
    parent: Option<&'a gpui::StyleRefinement>,
}

struct ButtonState {
    focus: gpui::FocusHandle,
}
#[derive(Clone, Copy)]
struct Interaction {
    pointer: bool,
    selectable: Option<bool>,
    selection_color: Option<gpui::Hsla>,
    link_content: bool,
    passive_disabled: bool,
    table_header: bool,
}
impl Default for Interaction {
    fn default() -> Self {
        Self {
            pointer: true,
            selectable: None,
            selection_color: None,
            link_content: false,
            passive_disabled: false,
            table_header: false,
        }
    }
}

#[path = "table_view.rs"]
pub(super) mod table_view;

struct View {
    id: WindowId,
    window_title: String,
    last_presentation: Option<gpuio_protocol::window::Presentation>,
    appearance_subscription: Option<gpui::Subscription>,
    frame: Option<gpuio_protocol::window::Frame>,
    session: SharedSession,
    transport: Arc<Transport>,
    images: BTreeMap<NodeId, image_view::State>,
    avatar_fallbacks: BTreeMap<NodeId, avatar_slot::State>,
    documents: BTreeMap<NodeId, document_view::State>,
    extensions: BTreeMap<NodeId, extension_view::State>,
    canvases: BTreeMap<NodeId, Rc<RefCell<canvas_view::State>>>,
    charts: BTreeMap<NodeId, Rc<RefCell<chart_view::State>>>,
    chart_budget: Rc<RefCell<crate::chart_paint::FrameBudget>>,
    canvas_budget: Rc<RefCell<crate::canvas_paint::FrameBudget>>,
    splits: BTreeMap<NodeId, split_view::State>,
    split_groups: BTreeMap<NodeId, crate::split_group_widget::Shared>,
    split_activation: Option<gpui::Subscription>,
    buttons: BTreeMap<NodeId, Rc<ButtonState>>,
    hover_observations: BTreeMap<NodeId, Rc<hover_observation::State>>,
    input_regions: BTreeMap<NodeId, input_region::Shared>,
    window_regions: BTreeMap<NodeId, Rc<window_regions::State>>,
    highlights: BTreeMap<NodeId, highlight::Shared>,
    binding_queries: BTreeMap<NodeId, command::binding::Owner>,
    binding_rendered: std::collections::BTreeSet<NodeId>,
    highlight_documents: Rc<RefCell<Rc<()>>>,
    input_pointer_inside: Rc<std::cell::Cell<bool>>,
    selections: BTreeMap<NodeId, Rc<RefCell<crate::selection::State>>>,
    text_shimmers: BTreeMap<NodeId, crate::text_shimmer_clock::Owner>,
    text_shimmer_clock: Rc<crate::text_shimmer_clock::Clock>,
    progresses: BTreeMap<NodeId, crate::progress_clock::Owner>,
    progress_clock: Rc<crate::progress_clock::Clock>,
    progress_close: Option<gpui::Subscription>,
    spinners: BTreeMap<NodeId, crate::spinner_clock::Owner>,
    spinner_clock: Rc<crate::spinner_clock::Clock>,
    spinner_close: Option<gpui::Subscription>,
    text_shimmer_budget: crate::text_shimmer_budget::Shared,
    editors: BTreeMap<NodeId, editor::Instance>,
    input_content: input_content_view::State,
    root_focus: Option<gpui::FocusHandle>,
    focus: focus::Shared,
    selects: BTreeMap<NodeId, Rc<RefCell<select::State>>>,
    pickers: BTreeMap<NodeId, choice_picker_host::Owner>,
    radios: BTreeMap<NodeId, Rc<RefCell<choice::State>>>,
    ratings: BTreeMap<NodeId, Rc<RefCell<rating::State>>>,
    sliders: BTreeMap<NodeId, slider_view::Shared>,
    numbers: BTreeMap<NodeId, number_input_view::Instance>,
    otps: BTreeMap<NodeId, otp_input_view::Instance>,
    calendars: BTreeMap<NodeId, calendar_view::Instance>,
    color_inputs: BTreeMap<NodeId, color_input_view::Instance>,
    tooltips: BTreeMap<NodeId, tooltip::State>,
    tooltip_last_closed: Option<std::time::Instant>,
    tooltip_previous: Option<tooltip_motion::Previous>,
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
    scrollbars: BTreeMap<(NodeId, scrollbar_host::Owner), crate::scrollbar_widget::Shared>,
    tab_motions: BTreeMap<NodeId, Rc<RefCell<tab_motion::State>>>,
    tab_viewports: BTreeMap<NodeId, Rc<RefCell<tab_viewport::State>>>,
    lists: BTreeMap<NodeId, Rc<RefCell<list_view::State>>>,
    tables: BTreeMap<NodeId, Rc<RefCell<table_view::State>>>,
    tree_drag: std::rc::Weak<tree_drag::Lease>,
    animations: BTreeMap<NodeId, Rc<RefCell<animation::State>>>,
    navigation: BTreeMap<NodeId, Rc<RefCell<navigation::State>>>,
    reveals: BTreeMap<NodeId, Rc<RefCell<reveal::State>>>,
    reveal_activation: Option<gpui::Subscription>,
    carousels: BTreeMap<NodeId, Rc<RefCell<carousel::State>>>,
    carousel_activation: Option<gpui::Subscription>,
    carousel_track_activation: Option<gpui::Subscription>,
    carousel_tracks: BTreeMap<NodeId, Rc<RefCell<carousel_track::State>>>,
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

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "message_follow_test.rs"]
mod message_follow_test;
#[path = "split_button_view.rs"]
mod split_button_view;
#[path = "split_group_host.rs"]
mod split_group_host;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "split_group_host_test.rs"]
mod split_group_host_test;
#[cfg(all(test, feature = "native-image-tests"))]
#[path = "structural_table_test.rs"]
mod structural_table_test;

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
#[path = "text_shimmer_view.rs"]
mod text_shimmer_view;

impl View {
    fn new(id: WindowId, session: SharedSession, transport: Arc<Transport>) -> Self {
        Self {
            id,
            window_title: String::new(),
            last_presentation: None,
            appearance_subscription: None,
            frame: None,
            focus: focus::Manager::new(id, session.clone()),
            session,
            transport,
            images: BTreeMap::new(),
            avatar_fallbacks: BTreeMap::new(),
            documents: BTreeMap::new(),
            extensions: BTreeMap::new(),
            canvases: BTreeMap::new(),
            charts: BTreeMap::new(),
            chart_budget: Default::default(),
            canvas_budget: Default::default(),
            splits: BTreeMap::new(),
            split_groups: BTreeMap::new(),
            split_activation: None,
            buttons: BTreeMap::new(),
            hover_observations: BTreeMap::new(),
            input_regions: BTreeMap::new(),
            window_regions: BTreeMap::new(),
            highlights: BTreeMap::new(),
            binding_queries: BTreeMap::new(),
            binding_rendered: Default::default(),
            highlight_documents: Default::default(),
            input_pointer_inside: Rc::new(std::cell::Cell::new(true)),
            selections: BTreeMap::new(),
            text_shimmers: BTreeMap::new(),
            text_shimmer_clock: Default::default(),
            progresses: Default::default(),
            progress_clock: Default::default(),
            progress_close: None,
            spinners: Default::default(),
            spinner_clock: Default::default(),
            spinner_close: None,
            text_shimmer_budget: Default::default(),
            editors: BTreeMap::new(),
            input_content: input_content_view::State::default(),
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
            tooltip_previous: None,
            command_subscription: None,
            menus: BTreeMap::new(),
            menu_activation: None,
            palettes: BTreeMap::new(),
            pointer_capture: Default::default(),
            pointer_activation: None,
            toasts: BTreeMap::new(),
            toast_stacks: BTreeMap::new(),
            selects: BTreeMap::new(),
            pickers: BTreeMap::new(),
            visited: Default::default(),
            #[cfg(feature = "native-tests")]
            probes: Default::default(),
            #[cfg(feature = "native-tests")]
            progress_probes: Default::default(),
            #[cfg(feature = "native-tests")]
            loading_probes: Default::default(),
            scrolls: Default::default(),
            scrollbars: Default::default(),
            tab_motions: Default::default(),
            tab_viewports: Default::default(),
            lists: Default::default(),
            tables: Default::default(),
            tree_drag: Default::default(),
            animations: Default::default(),
            navigation: Default::default(),
            reveals: Default::default(),
            reveal_activation: None,
            carousels: Default::default(),
            carousel_tracks: Default::default(),
            carousel_track_activation: None,
            carousel_activation: None,
            animation_programs: Default::default(),
            container_queries: Default::default(),
            #[cfg(feature = "native-tests")]
            render_count: 0,
        }
    }
    fn update_editors(&mut self, dirty: &[NodeId], window: &mut Window, cx: &mut Context<Self>) {
        self.sync_binding_queries();
        self.sync_text_shimmers();
        self.sync_spinners();
        self.sync_progress(window, cx);
        self.install_command_interceptor(window, cx);
        self.install_pointer_observer(window, cx);
        self.install_menu_observers(window, cx);
        self.sync_container_queries(dirty);
        self.sync_avatar_fallbacks(dirty);
        self.sync_navigation(dirty);
        self.sync_reveals(dirty, window, cx);
        self.sync_lists(dirty, window, cx);
        self.sync_tables(dirty, window, cx);
        self.sync_images(dirty, window, cx);
        self.sync_documents(dirty, window, cx);
        self.sync_extensions(dirty, window, cx);
        self.sync_animations(dirty, cx);
        self.sync_programs(dirty, cx);
        self.sync_palettes(window, cx);
        self.sync_toasts(window, cx);
        self.sync_tooltips(window, cx);
        self.sync_carousels(window, cx);
        self.sync_carousel_tracks(dirty, window, cx);
        self.sync_canvases(dirty, window, cx);
        self.sync_charts(dirty, window, cx);
        self.sync_splits(window, cx);
        self.sync_split_groups(dirty, window, cx);
        self.sync_sliders(dirty, window, cx);
        self.sync_numbers(dirty, window, cx);
        self.sync_otps(dirty, window, cx);
        self.sync_calendars(dirty, window, cx);
        self.sync_color_inputs(dirty, window, cx);
        // An unselected query branch is hidden even before the first layout.
        // Do not count time waiting for its first visible paint as active motion.
        self.suspend_hidden_animations();
        self.suspend_hidden_carousel_tracks();
        self.suspend_hidden_programs();
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.editors.clear();
                self.scrolls.clear();
                // The session borrow must be released before native focus hooks run.
                drop(session);
                self.close_scrollbars(window, cx);
                self.tab_viewports.clear();
                self.tab_motions.clear();
                self.input_content.clear();
                return;
            };
            self.editors.retain(|id, _| tree.get(*id).is_some());
            self.tab_motions
                .retain(|id, _| tree.get(*id).is_some_and(|node| node.tab_motion.is_some()));
            self.tab_viewports.retain(|id, state| {
                if let Some(node) = tree.get(*id)
                    && let (Some(choices), Some(viewport)) = (&node.choice, &node.tab_viewport)
                {
                    // Keep request history through hidden/unvisited retained panels,
                    // and release old layout capacity immediately on data changes.
                    state.borrow_mut().begin(choices, viewport);
                    true
                } else {
                    self.scrolls.remove(id);
                    false
                }
            });
            self.scrolls.retain(|id, _| {
                tree.get(*id).is_some_and(|node| {
                    node.overlay.is_some()
                        || node.tab_viewport.is_some()
                        || scroll::declared(&node.style)
                })
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
                editor.configure(&node, window, cx);
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
        self.sync_choice_pickers(dirty, window, cx);
        self.sync_input_content(window, cx);
        self.sync_scrollbars(window, cx);
    }
    // One checkable target owns the name/actions. Hide decorative label semantics
    // without making its native animation/image subtree inert or unpainted.
    fn control_label(
        &mut self,
        tree: &crate::tree::Tree,
        id: NodeId,
        interaction: Interaction,
        disabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        crate::semantics::State::decorative(
            self.element(
                tree,
                id,
                Interaction {
                    passive_disabled: interaction.passive_disabled || disabled,
                    selectable: Some(false),
                    link_content: true,
                    ..interaction
                },
                window,
                cx,
            ),
            ("gpuio-control-label", identity).into(),
        )
        .into_any_element()
    }

    // Keep bulky child-presenter branches out of every recursive Div builder's
    // debug stack frame. Each specialized owner still renders its own children.
    fn managed_children(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if node.navigation_stack.is_some() {
            Some(self.navigation_element(tree, node, interaction, window, cx))
        } else if node.container_query.is_some() {
            Some(self.container_query_element(tree, node, interaction, cx))
        } else if node.split_group.is_some() {
            Some(self.split_group_element(tree, node, interaction, window, cx))
        } else if node.split.is_some() {
            Some(self.split_element(tree, node, interaction, window, cx))
        } else {
            None
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
        self.element_in_flow(
            tree,
            FlowPlacement {
                node: id,
                parent: None,
            },
            interaction,
            window,
            cx,
        )
    }
    fn element_in_flow(
        &mut self,
        tree: &crate::tree::Tree,
        placement: FlowPlacement<'_>,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = placement.node;
        let element = self.element_body(tree, placement, interaction, window, cx);
        let element = if interaction.table_header
            && tree.get(id).is_some_and(|node| {
                node.handler.is_some() || node.command_ref.is_some() || node.editor.is_some()
            }) {
            table_view::clip_header_control(id, element, &self.focus)
        } else {
            element
        };
        let inert = tree
            .get(id)
            .is_some_and(|node| crate::style::inert(&node.style));
        if inert {
            let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
            crate::semantics::InteractionShield::inert(element, ("gpuio-inert", identity).into())
                .into_any_element()
        } else if tree.get(id).is_some_and(|node| {
            crate::style::disabled(&node.style)
                || node.list_input.is_some_and(|config| config.disabled)
        }) {
            crate::semantics::InteractionShield::disabled(element).into_any_element()
        } else {
            element
        }
    }
    fn element_body(
        &mut self,
        tree: &crate::tree::Tree,
        placement: FlowPlacement<'_>,
        mut interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = placement.node;
        let node = tree.get(id).expect("validated retained node");
        // Retiring toast content may still paint, but its independent popup
        // surfaces must disappear and must not retain a modal focus trap.
        if (node.overlay.is_some() || node.palette.is_some())
            && self.focus.borrow().input_retired(id)
        {
            return div().into_any_element();
        }
        let track_frame = self.carousel_track_frame(tree, node);
        if node.command_binding.is_some() || node.editor.is_some() {
            self.binding_rendered.insert(id);
        }
        let highlight_scope = node
            .highlight_scope
            .as_ref()
            // An unpainted scope is retired at frame end. Preparing one known
            // to be hidden would request work, refresh, retire and repeat forever.
            // Native state visibility is committed after paint and requests one
            // new render when an interaction makes the scope visible again.
            .filter(|_| self.focus.borrow().highlight_visible(tree, id))
            .map(|_| self.prepare_highlight(tree, node, window, cx));
        if node.kind == Kind::ChartView {
            return self.chart_element(node, interaction);
        }
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
                        Field::UserSelect(v) => interaction.selectable = Some(*v),
                        Field::SelectionColor(v) => interaction.selection_color = Some(color(v)),
                        Field::AccessibleName(v) => accessible_name = v.clone().into(),
                        _ => (),
                    }
                }
            }
        }
        if let Some(config) = &node.link {
            accessible_name = config.label.clone().into();
        }
        let mut element = div().id(("gpuio-node", identity));
        if let Some(scope) = highlight_scope {
            element = element.child(highlight::marker(scope));
        }
        // Native controls inside a tree/table row or structured tab own pointer input. The input widget
        // may focus on mouse-down without consuming mouse-up; block the row's
        // ancestor click hitbox while preserving wheel propagation to the list.
        if (node.editor.is_some()
            || node.control.is_some()
            || node.link.is_some()
            || node.choice.is_some()
            || node.rating.is_some()
            || node.slider.is_some()
            || node.number_input.is_some()
            || node.otp_input.is_some()
            || node.calendar.is_some()
            || node.color_input.is_some()
            || (node.kind == Kind::Text && interaction.selectable.unwrap_or(false)))
            && tree_input::within_input_collection(tree, id)
        {
            element = element.block_mouse_except_scroll();
        }

        element = match crate::style::pointer_occlusion(&node.style) {
            1 => element.block_mouse_except_scroll(),
            2 => element.occlude(),
            _ => element,
        };

        let (presented, image_corners) =
            self.node_presentation(tree, node, interaction, element, window, cx);
        element = presented;
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
                .choice_picker
                .as_ref()
                .is_some_and(|p| p.config.disabled)
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
            || node.link.as_ref().is_some_and(|config| config.disabled)
            || node.editor.as_ref().is_some_and(|config| config.disabled)
            || node
                .carousel_track
                .as_ref()
                .is_some_and(|config| config.carousel.disabled);
        let own_disabled = disabled
            || node
                .input_region
                .as_ref()
                .is_some_and(|config| config.disabled);
        let disabled =
            own_disabled || interaction.passive_disabled || self.focus.borrow().disabled(id);
        if let Some(config) = &node.input_region {
            element = self.input_region_element(element, node, tree.revision(), config, window, cx);
        }
        let button_policy = node.button_presentation.unwrap_or_default().policy;
        let button_loading =
            button_policy.loading || node.link.as_ref().is_some_and(|config| config.loading);
        let preserve_focus = button_policy.focus == gpuio_protocol::button::Focus::Preserve;
        let button_order = node
            .button_presentation
            .and_then(|config| match config.policy.focus {
                gpuio_protocol::button::Focus::Focusable(order) => Some(order),
                gpuio_protocol::button::Focus::Preserve => None,
            });
        let input_presentation = Interaction {
            pointer: interaction.pointer && !button_loading,
            ..interaction
        };
        let checked = command
            .as_ref()
            .is_some_and(|route| route.config.checked == Some(true))
            || matches!(
                node.control,
                Some(
                    Control::Checkbox(CheckState::Checked, _)
                        | Control::Switch(true, _)
                        | Control::Radio(true, _, _)
                )
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
        element = self.node_focus(
            tree,
            node_focus::Render {
                node,
                accessible_name,
                input_presentation,
                disabled,
                preserve_focus,
                button_order,
                checked,
                indeterminate,
                command_checked: command
                    .as_ref()
                    .is_some_and(|route| route.config.checked.is_some()),
            },
            element,
            window,
            cx,
        );
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
        let factor = animation
            .as_ref()
            .and_then(|(_, sample)| animation::opacity_factor(&sample.values))
            .or_else(|| {
                program
                    .as_ref()
                    .and_then(|(_, sample)| animation::opacity_factor(&sample.frame.values))
            });
        if matches!(node.kind, Kind::Checkbox | Kind::Switch | Kind::Radio)
            && let Some(appearance) = &node.control_appearance
        {
            element = element.gap(px(appearance.gap as f32));
        }
        let (styled, selected_style) = self.node_style(
            tree,
            node_style::Render {
                node,
                styles: &styles,
                input_presentation,
                interaction,
                disabled,
                own_disabled,
                checked,
                indeterminate,
                factor,
                track_frame: track_frame.as_deref(),
            },
            element,
            window,
            cx,
        );
        element = styled;
        if let Some((_, sample)) = &animation {
            animation::apply(element.style(), &sample.values);
        }
        if let Some((_, sample)) = &program {
            animation::apply(element.style(), &sample.frame.values);
        }
        let scrolling = if node.carousel_track.is_none()
            && track_frame.is_none()
            && (scroll::declared(&node.style)
                || element.style().overflow.x == Some(gpui::Overflow::Scroll)
                || element.style().overflow.y == Some(gpui::Overflow::Scroll))
        {
            let state = self.scrolls.entry(id).or_default().clone();
            element = scroll::attach(element, &state);
            Some(state)
        } else {
            None
        };
        element = self.node_content(
            tree,
            node_content::Render {
                node,
                interaction,
                input_presentation,
                disabled,
                popup_priority,
                label,
                selected_style,
                scrolling: &scrolling,
            },
            element,
            window,
            cx,
        );
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
        if let Some(children) = self.managed_children(tree, node, interaction, window, cx) {
            element = element.child(children);
        } else {
            let child_interaction = if node.kind == Kind::Link {
                Interaction {
                    selectable: Some(false),
                    link_content: true,
                    ..interaction
                }
            } else {
                interaction
            };
            // Deferred native interaction styles are resolved by GPUI later.
            // Such a parent uses immediate reveal instead of guessing its size.
            let parent_style = if styles.iter().any(|s| matches!(s, Style::State(..))) {
                None
            } else {
                Some(&*element.style())
            };
            let children = node
                .children
                .iter()
                .filter(|_| {
                    node.editor_frame.is_none()
                        && !matches!(
                            node.kind,
                            Kind::Button
                                | Kind::CommandButton
                                // Structural slots are mounted by the picker in
                                // their trigger/popup locations, never twice.
                                | Kind::ChoicePicker
                                | Kind::NumberInput
                                | Kind::Select
                                | Kind::Calendar
                                | Kind::Avatar
                                | Kind::Checkbox
                                | Kind::Switch
                                | Kind::Radio
                                | Kind::RadioGroup
                                | Kind::TabBar
                        )
                })
                .enumerate()
                .map(|(index, id)| {
                    let body = if node.kind == Kind::Link {
                        self.control_label(tree, *id, child_interaction, disabled, window, cx)
                    } else {
                        self.element_in_flow(
                            tree,
                            FlowPlacement {
                                node: *id,
                                parent: parent_style,
                            },
                            child_interaction,
                            window,
                            cx,
                        )
                    };
                    match &track_frame {
                        Some(frame) => frame.item(body, index, *id),
                        None => body,
                    }
                })
                .collect::<Vec<_>>();
            element = element.children(children);
        }

        element = self.node_actions(
            node_actions::Render {
                node,
                revision: tree.revision(),
                command,
                interaction,
                disabled,
                preserve_focus,
                button_loading,
                button_order,
            },
            element,
            cx,
        );
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
                node,
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
        let scrollbar = scrolling.as_ref().and_then(|state| {
            self.scrollbar_owner(
                tree,
                node,
                scrollbar_host::Mount {
                    kind: scrollbar_host::Owner::Viewport,
                    handle: state.clone(),
                },
                interaction,
                window,
                cx,
            )
        });
        if let Some(corners) = image_corners {
            let image = image_corners::Rounded::capture(element, corners);
            return match &scrolling {
                Some(state) => self.finish_element(
                    scroll::Frame::new(image, state, self.focus.clone(), id)
                        .with_scrollbar(scrollbar),
                    node,
                    tree.revision(),
                    disabled,
                ),
                None => self.finish_element(image, node, tree.revision(), disabled),
            };
        }
        let reveal = self.prepare_reveal(node, placement.parent, element.style(), window, cx);
        let body = match scrolling {
            Some(state) => self.finish_element(
                scroll::Frame::with_tab_offset(
                    element,
                    &state,
                    self.focus.clone(),
                    id,
                    self.tab_viewports.get(&id),
                )
                .with_scrollbar(scrollbar),
                node,
                tree.revision(),
                disabled,
            ),
            None => self.finish_element(element, node, tree.revision(), disabled),
        };
        let body = match track_frame {
            Some(frame) => frame.track(body),
            None => body,
        };
        let body = self.wrap_carousel_track_pointer(body, node, cx);
        match reveal {
            Some(reveal) => reveal.wrap(body),
            None => body,
        }
    }
    fn finish_element<
        E: gpui::Element<PrepaintState = Option<gpui::Hitbox>> + gpui::InteractiveElement,
    >(
        &self,
        element: E,
        node: &crate::tree::Node,
        revision: i64,
        disabled: bool,
    ) -> gpui::AnyElement {
        let element = highlight_style::Frame::new(element, node, &self.focus);
        let id = node.id;
        // The managed list row wrapper owns this metadata and the row focus
        // handle. Rendering it again on the description would duplicate AX rows.
        let tree_row_metadata = node
            .parent
            .is_some_and(|parent| self.lists.contains_key(&parent))
            && node.accessibility.as_ref().is_some_and(|metadata| {
                matches!(
                    metadata.role,
                    Some(
                        gpuio_protocol::accessibility::Role::TreeItem(_)
                            | gpuio_protocol::accessibility::Role::OptionItem(_)
                    )
                )
            });
        let element = crate::semantics::State {
            identity: None,
            busy: node
                .button_presentation
                .is_some_and(|config| config.policy.loading)
                || node.link.as_ref().is_some_and(|config| config.loading)
                || node.list_input.is_some_and(|config| config.busy),
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
        let popup_expanded = if matches!(node.kind, Kind::Button | Kind::CommandButton) {
            self.session.borrow().tree(self.id).and_then(|tree| {
                tree.popover_for_trigger(id).map(|popover| {
                    popover
                        .children
                        .get(1)
                        .is_some_and(|panel| self.focus.borrow().interactive(*panel))
                })
            })
        } else {
            None
        };
        if let Some(expanded) = popup_expanded {
            return popover_semantics::Trigger { element, expanded }.into_any_element();
        }
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
        if node.input_region.is_some()
            && let Some(state) = self.input_regions.get(&id)
        {
            return input_region::Region {
                element,
                state: state.clone(),
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
        crate::window_frame::configure(self.frame, window);
        window_host::observe_presentation(self, window);
        self.sync_choice_pickers(&[], window, cx);
        self.refresh_list_pins(window, cx);
        self.sync_binding_queries();
        self.sync_text_shimmers();
        self.sync_spinners();
        self.sync_progress(window, cx);
        self.visited.clear();
        self.binding_rendered.clear();
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
        let chart_budget = self.chart_budget.clone();
        let text_shimmer_budget = self.text_shimmer_budget.clone();
        let canvases: Vec<_> = self.canvases.values().map(Rc::downgrade).collect();
        let drag_window = self.id;
        let input_pointer_inside = self.input_pointer_inside.clone();
        gpui_base::TextSelection::activate_scope(
            self.focus.borrow().active_selection_scope(),
            window,
            cx,
        );
        let mut root = drag_drop::root(div(), self.id, cx)
            .child(gpui_base::TextSelectionLayer)
            .capture_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape"
                    && (view.cancel_color_inputs(
                        gpuio_protocol::color_input::CancelReason::Escape,
                        window,
                        cx,
                    ) | view.cancel_tree_drag(window, cx)
                        | view.cancel_carousel_drags(window, cx)
                        | view.cancel_track_drags(window, cx)
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
                        begin_focus
                            .borrow_mut()
                            .begin_frame(window.content_mask().bounds);
                        let _ = program_begin.update(cx, |view, _| {
                            view.begin_document_profile_paint();
                            view.begin_picker_paint();
                            view.begin_carousel_paint();
                            view.begin_carousel_track_paint();
                            view.begin_reveal_paint();
                            view.begin_program_paint();
                            view.begin_query_paint();
                            view.begin_avatar_paint();
                            view.begin_highlight_paint();
                            view.begin_tab_motion_paint();
                            view.begin_split_group_paint();
                            view.begin_scrollbar_paint();
                            view.begin_layered_toast_paint();
                        });
                        *canvas_budget.borrow_mut() = Default::default();
                        *chart_budget.borrow_mut() = Default::default();
                        *text_shimmer_budget.borrow_mut() = Default::default();
                        for state in &canvases {
                            if let Some(state) = state.upgrade() {
                                state.borrow_mut().flush_canvas_frame(window, cx);
                            }
                        }
                        drag_drop::install_cleanup(drag_window, window, cx);
                        input_region::install_pointer_presence(input_pointer_inside, window);
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
                if let Some(presentation) = &state.presentation {
                    presentation.update(cx, |p, _| p.revoke_profile());
                }
                state.presentation = None;
            }
        }
        self.hide_unvisited_extensions();
        self.hide_unvisited_canvases(window);
        self.hide_unvisited_charts(window);
        self.hide_unvisited_sliders(window, cx);
        self.hide_unvisited_numbers(window, cx);
        self.hide_unvisited_otps(window, cx);
        self.hide_unvisited_calendars(window, cx);
        self.hide_unvisited_color_inputs(window, cx);
        self.input_regions.retain(|id, state| {
            if !self.visited.contains(id) {
                state.borrow_mut().clear();
                return false;
            }
            if !self.focus.borrow().allows(*id) || window.captured_hitbox().is_some() {
                state.borrow_mut().clear();
            }
            true
        });
        self.window_regions.retain(|id, state| {
            if !self.visited.contains(id)
                || session
                    .tree(self.id)
                    .and_then(|tree| tree.get(*id))
                    .is_none_or(|node| node.window_region.is_none())
            {
                state.cancel();
                return false;
            }
            if !self.focus.borrow().allows(*id) || window.captured_hitbox().is_some() {
                state.cancel();
            }
            true
        });
        self.retire_unvisited_hover(window, cx);
        self.buttons.retain(|id, _| self.visited.contains(id));
        self.radios.retain(|id, _| self.visited.contains(id));
        self.ratings.retain(|id, _| self.visited.contains(id));
        self.selects.retain(|id, _| self.visited.contains(id));
        self.retire_unvisited_menus(window, cx);
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
                    .split_groups
                    .values()
                    .any(|state| state.borrow().focused(window))
                || self
                    .tables
                    .values()
                    .any(|state| state.borrow().focused(window, cx))
                || self.lists.values().any(|state| {
                    let state = state.borrow();
                    (state.tree_focus.is_some() && state.owns_tree_focus(window))
                        || state
                            .list_focus
                            .as_ref()
                            .is_some_and(|focus| focus.is_focused(window))
                })
                || self.focus.borrow().contains_focus(window)
                || self
                    .scrollbars
                    .values()
                    .any(|state| state.borrow().focused(window))
                || self
                    .carousels
                    .values()
                    .any(|state| state.borrow().focused(window))
                || self
                    .carousel_tracks
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
                || self
                    .charts
                    .values()
                    .any(|state| state.borrow().chart_focused(window))
                || root_focus.is_focused(window)
                || self
                    .input_regions
                    .values()
                    .any(|state| state.borrow().focus.is_focused(window))
                || self
                    .pickers
                    .values()
                    .any(|owner| owner.clear_focus.is_focused(window))
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
        self.sync_input_content(window, cx);
        self.sync_scrollbars(window, cx);
        let id = self.id;
        let session = self.session.clone();
        let transport = self.transport.clone();
        let program_finish = cx.entity().downgrade();
        let navigation_focus = self.focus.clone();
        let reveal_focus = self.focus.clone();
        let root = root.child(
            canvas(
                |_, _, _| (),
                move |_, _, window, cx| {
                    // Deferred popups paint after the root tree. Sweep only once
                    // the complete effect cycle has painted; never request a frame.
                    if program_finish
                        .update(cx, |view, _| {
                            !view.animation_programs.is_empty()
                                || !view.container_queries.is_empty()
                                || !view.avatar_fallbacks.is_empty()
                                || !view.carousels.is_empty()
                                || !view.carousel_tracks.is_empty()
                                || !view.reveals.is_empty()
                                || !view.highlights.is_empty()
                                || !view.binding_queries.is_empty()
                                || !view.text_shimmers.is_empty()
                                || !view.spinners.is_empty()
                                || !view.progresses.is_empty()
                                || !view.pickers.is_empty()
                                || !view.tab_motions.is_empty()
                                || !view.split_groups.is_empty()
                                || !view.scrollbars.is_empty()
                                || !view.toast_stacks.is_empty()
                                // A native style can reveal the last hidden scope.
                                // Commit that sample even with no active matcher.
                                || view.focus.borrow().has_pending_highlight_styles()
                        })
                        .unwrap_or(false)
                    {
                        window.defer(cx, move |window, cx| {
                            let _ = program_finish.update(cx, |view, cx| {
                                view.finish_picker_paint(window, cx);
                                view.finish_query_paint(window, cx);
                                view.finish_avatar_paint();
                                view.finish_binding_paint(window, cx);
                                view.finish_program_paint();
                                view.finish_reveal_paint();
                                view.finish_carousel_track_paint(window, cx);
                                view.schedule_carousel_tracks(window, cx);
                                view.schedule_carousels(window, cx);
                                view.finish_highlight_paint(cx);
                                view.finish_tab_motion_paint();
                                view.finish_split_group_paint(window, cx);
                                view.finish_scrollbar_paint(window, cx);
                                view.finish_layered_toast_paint(window, cx);
                                for owner in view.progresses.values() {
                                    owner.finish_frame();
                                }
                                for owner in view.spinners.values() {
                                    owner.finish_frame();
                                }
                                for owner in view.text_shimmers.values() {
                                    owner.finish_frame();
                                }
                            });
                        });
                    }
                    if navigation_focus.borrow().navigation_pending() {
                        window.defer(cx, move |window, cx| {
                            navigation_focus.borrow_mut().finish_navigation(window, cx);
                        });
                    }
                    window.defer(cx, move |window, cx| {
                        reveal_focus.borrow_mut().finish_paint(window, cx);
                    });
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
        );
        let stroke = self
            .session
            .borrow()
            .tree(self.id)
            .and_then(|tree| {
                tree.root()
                    .and_then(|id| tree.get(id))
                    .map(|node| crate::window_frame::border_color(&node.style))
            })
            .unwrap_or_else(|| gpui::rgba(0x80808060).into());
        crate::window_frame::wrap(root.into_any_element(), self.frame, stroke, window)
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
    let notifications = crate::notification_host::State::default();
    let reopen_transport = transport.clone();
    application.on_reopen(move |_| window_host::control(&reopen_transport, Event::ReopenRequested));
    application.run(move |cx: &mut App| {
        // GPUI on_quit is cleanup-only on both platforms. AppKit's separate
        // applicationShouldTerminate hook supplies the asynchronous decision.
        #[cfg(target_os="macos")]
        window_macos::install(cx,transport.clone());
        window_host::control(&transport,Event::WindowCapabilities(window_host::capabilities()));
        gpui_base::init(cx);
        crate::font_defaults::init(cx);
        gpuio_table_adapter::init(cx);
        crate::image_host::init(cx);
        let motion = crate::motion_preference::init(cx);
        // GPUI defaults to last-window exit on Linux. Our explicit lifecycle
        // policy must control background applications consistently on both OSes.
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let session = Rc::new(RefCell::new(Session::default()));
        crate::chart_host::init(&session, transport.clone(), cx);
        crate::motion_preference::bind_clocks(&session.borrow().motion(), cx);
        let mut windows: BTreeMap<WindowId, WindowHandle<View>> = BTreeMap::new();
        let dialogs = crate::file_dialog::Dialogs::default();
        let quit_dialogs = dialogs.clone();
        let quit_motion = motion.clone();
        let quit_desktop = desktop.clone();
        let quit_notifications = notifications.clone();
        cx.on_app_quit(move |cx| {
            quit_notifications.borrow_mut().close().wait_before_quit();
            quit_desktop.borrow_mut().close().wait_before_quit();
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
                    let notification_cleanup = notifications.borrow_mut().close();
                    notification_cleanup.wait().await;
                    let desktop_cleanup = desktop.borrow_mut().close();
                    desktop_cleanup.wait().await;
                    motion.borrow_mut().take();
                    dialogs.clear().wait().await;
                    crate::chart_host::shutdown(cx).await;
                    crate::canvas_host::shutdown(cx).await;
                    crate::chart_render_host::shutdown(cx).await;
                    crate::image_host::shutdown(cx).await;
                    crate::document_host::shutdown(cx).await;
                    crate::highlight_host::shutdown(cx).await;
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
                        Message::Open(correlation,id,title,width,height) => Message::OpenConfigured(correlation,id,gpuio_protocol::window::Config{title,width,height,focus:true,chrome:gpuio_protocol::window::Chrome::Standard,resizable:true,frame:Default::default()}),
                        message=>message,
                    };
                    match message {
                        Message::Hello(version, caps) => {
                            failed(&transport, 0, session.borrow_mut().hello(version, caps))
                        }
                        Message::Open(..)=>unreachable!("normalized above"),
                        Message::OpenConfigured(correlation,id,config)=> {
                            let gpuio_protocol::window::Config {title,width,height,focus,chrome,resizable,frame}=config;
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
                                        titlebar: (chrome!=gpuio_protocol::window::Chrome::Hidden).then(|| gpui::TitlebarOptions {
                                            title: Some(title.clone().into()),
                                            appears_transparent: chrome==gpuio_protocol::window::Chrome::Custom,
                                            traffic_light_position: (chrome==gpuio_protocol::window::Chrome::Custom).then(|| gpui::point(px(9.), px(9.))),
                                        }),
                                        app_owns_titlebar_drag: chrome==gpuio_protocol::window::Chrome::Custom,
                                        window_decorations: (cfg!(target_os="linux") && chrome==gpuio_protocol::window::Chrome::Custom).then_some(gpui::WindowDecorations::Client),
                                        window_background: if cfg!(target_os="linux") && chrome==gpuio_protocol::window::Chrome::Custom {gpui::WindowBackgroundAppearance::Transparent} else {Default::default()},
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        window.set_window_title(&title);
                                        // AppKit otherwise returns the NSWindow itself for
                                        // screen-point AX queries instead of its controls.
                                        #[cfg(target_os = "macos")]
                                        gpui_base::install_window_hit_test_forwarder(window);
                                        cx.new(|cx| {
                                            let mut view=View::new(id, session.clone(), transport.clone());
                                            view.window_title=title.clone();
                                            view.frame=(chrome==gpuio_protocol::window::Chrome::Custom).then_some(frame);
                                            window_host::watch(&mut view,window,cx);
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
                                    transport.mailbox.lock().expect("mailbox poisoned")
                                        .retain_bindings(tx.window, |node, handler| {
                                            session.borrow().tree(tx.window).is_some_and(|tree|
                                                tree.get(node).is_some_and(|n| n.command_binding.is_some() && n.handler == Some(handler)))
                                        });
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
                                        if matches!(command,EditorCommand::ReadContentHintStatus) {
                                            return view.read_input_content_status(node,window,cx);
                                        }
                                        let result = match view.editors.get_mut(&node) {
                                            None => EditorResult::Failed(EditorError::StaleEditor),
                                            Some(editor) => editor.command(&command, window, cx),
                                        };
                                        if !matches!(command, EditorCommand::ReadViewport | EditorCommand::Search(gpuio_protocol::editor_search::Command::Read)) { cx.notify(); }
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
                                            .update(cx, |view, window, cx| { drag_drop::cancel(view.id, gpuio_protocol::drag_drop::CancelReason::WindowClosed, window, cx); view.cancel_tree_drag(window, cx); view.close_color_inputs(window, cx); view.close_reveals(); view.sync_otps(&[], window, cx); view.sync_numbers(&[], window, cx); view.sync_sliders(&[], window, cx); view.cancel_split_drag(window, cx); view.close_split_groups(window, cx); view.close_scrollbars(window, cx); view.extensions.clear(); for state in view.canvases.values() { state.borrow_mut().close(window); } view.canvases.clear(); for state in view.charts.values() {state.borrow_mut().close(window);} view.charts.clear(); window.remove_window(); });
                                    }
                                }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
                        }
                        Message::WindowCommand(correlation, id, command) => {
                            let result = match windows.get(&id) {
                                Some(handle) => handle.update(cx, |view, window, cx| {
                                    let result = window_host::request(view, &command, window, cx);
                                    if window_host::observes_window(&command) {
                                        window_host::observe(view, window);
                                        cx.notify();
                                    } else if matches!(command,
                                        gpuio_protocol::window::Command::ClearTextSelection
                                        | gpuio_protocol::window::Command::EndTextSelection
                                    ) {
                                        cx.notify();
                                    }
                                    result
                                }).unwrap_or(gpuio_protocol::window::Response::Failed(
                                    gpuio_protocol::window::Error::Closed,
                                )),
                                None => gpuio_protocol::window::Response::Failed(
                                    gpuio_protocol::window::Error::Closed,
                                ),
                            };
                            transport.respond(Event::WindowResponse(correlation, id, result));
                        }
                        Message::Notification(correlation, request) => {
                            match session.borrow().check_ready() {
                                Ok(()) => { crate::notification_host::dispatch(&notifications, correlation, request, &transport); }
                                Err(error) => transport.respond(Event::Failed(correlation, error)),
                            }
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
                        Message::Chart(correlation, request) => {
                            cx.update(|cx| crate::chart_host::dispatch(correlation, request, cx));
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
                            let notification_cleanup = notifications.borrow_mut().close();
                            notification_cleanup.wait().await;
                            let desktop_cleanup = desktop.borrow_mut().close();
                            desktop_cleanup.wait().await;
                            motion.borrow_mut().take();
                            dialogs.clear().wait().await;
                            crate::chart_host::shutdown(cx).await;
                    crate::canvas_host::shutdown(cx).await;
                    crate::chart_render_host::shutdown(cx).await;
                            crate::image_host::shutdown(cx).await;
                            crate::document_host::shutdown(cx).await;
                            crate::highlight_host::shutdown(cx).await;
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
    crate::chart_host::finish_before_quit(cx);
    crate::canvas_host::finish_before_quit(cx);
    crate::chart_render_host::finish_before_quit(cx);
    crate::image_host::finish_before_quit(cx);
    crate::document_host::finish_before_quit(cx);
    crate::highlight_host::finish_before_quit(cx);
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
    crate::chart_host::finish_before_quit(cx);
    crate::canvas_host::finish_before_quit(cx);
    crate::chart_render_host::finish_before_quit(cx);
    crate::image_host::finish_before_quit(cx);
    crate::document_host::finish_before_quit(cx);
    crate::highlight_host::finish_before_quit(cx);
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

// Invalidate retained list presentations as well as scheduling window redraw.
pub(crate) fn refresh_chart_window(handle: gpui::AnyWindowHandle, cx: &mut App) {
    let _ = handle.update(cx, |root, window, cx| {
        let changed = match root.downcast::<View>() {
            Ok(view) => view.update(cx, |view, cx| view.charts_changed(None, window, cx)),
            Err(_) => true,
        };
        if changed {
            window.refresh();
        }
    });
}
pub(crate) fn chart_source_changed(source: Option<gpuio_protocol::ResourceId>, cx: &mut App) {
    for handle in cx.windows() {
        let _ = handle.update(cx, |root, window, cx| {
            if let Ok(view) = root.downcast::<View>()
                && view.update(cx, |view, cx| view.charts_changed(source, window, cx))
            {
                window.refresh();
            }
        });
    }
}

#[cfg(feature = "native-image-tests")]
#[path = "styled_text_test.rs"]
pub(crate) mod styled_text_test;

#[cfg(feature = "native-image-tests")]
#[path = "link_test.rs"]
pub(crate) mod link_test;

#[cfg(feature = "native-image-tests")]
#[path = "grid_location_test.rs"]
pub(crate) mod grid_location_test;

#[cfg(feature = "native-image-tests")]
#[path = "aspect_ratio_test.rs"]
pub(crate) mod aspect_ratio_test;

#[cfg(feature = "native-image-tests")]
#[path = "border_style_test.rs"]
pub(crate) mod border_style_test;

#[cfg(feature = "native-image-tests")]
#[path = "text_shimmer_view_test.rs"]
pub(crate) mod text_shimmer_view_test;

#[cfg(feature = "native-image-tests")]
#[path = "opacity_factor_test.rs"]
pub(crate) mod opacity_factor_test;

impl Drop for View {
    fn drop(&mut self) {
        if let Ok(mut mailbox) = self.transport.mailbox.lock() {
            mailbox.retain_bindings(self.id, |_, _| false);
        }
    }
}

#[cfg(feature = "native-tests")]
#[path = "command_binding_test.rs"]
pub(super) mod command_binding_test;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "toast_placement_test.rs"]
mod toast_placement_test;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "toast_layering_test.rs"]
mod toast_layering_test;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "toast_motion_test.rs"]
mod toast_motion_test;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "scrollbar_host_test.rs"]
mod scrollbar_host_test;
