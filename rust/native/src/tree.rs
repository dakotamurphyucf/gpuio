#[path = "table_tree.rs"]
mod table;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn allows_children(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Container
            | Kind::Input
            | Kind::Textarea
            | Kind::NumberInput
            | Kind::Calendar
            | Kind::TabPanel
            | Kind::Panel
            | Kind::Disclosure
            | Kind::Accordion
            | Kind::NavigationStack
            | Kind::Carousel
            | Kind::CarouselTrack
            | Kind::CarouselTrackGroup
            | Kind::SplitPane
            | Kind::SplitGroup
            | Kind::VirtualList
            | Kind::Animated
            | Kind::AnimationProgram
            | Kind::ContainerQuery
            | Kind::Button
            | Kind::Checkbox
            | Kind::Switch
            | Kind::Radio
            | Kind::RadioGroup
            | Kind::TabBar
            | Kind::Select
            | Kind::ChoicePicker
            | Kind::Link
            | Kind::Avatar
            | Kind::Progress
            | Kind::CommandButton
            | Kind::FocusScope
            | Kind::Tooltip
            | Kind::HoverCard
            | Kind::CommandScope
            | Kind::CommandPalette
            | Kind::Menu
            | Kind::Toast
            | Kind::ToastStack
            | Kind::PointerArea
            | Kind::InputRegion
            | Kind::HighlightScope
            | Kind::DragSource
            | Kind::DropTarget
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct SliderMount {
    pub config: Arc<gpuio_protocol::slider::Config>,
    pub initial: gpuio_protocol::slider::Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NumberInputMount {
    pub config: Arc<gpuio_protocol::number_input::Config>,
    pub initial: gpuio_protocol::number_input::Value,
    pub initial_draft: Option<Arc<str>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OtpInputMount {
    pub config: Arc<gpuio_protocol::otp_input::Config>,
    pub initial: Arc<str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorInputMount {
    pub config: Arc<gpuio_protocol::color_input::Config>,
    pub initial: gpuio_protocol::color_value::Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarMount {
    pub config: Arc<gpuio_protocol::calendar_input::Config>,
    pub initial: gpuio_protocol::calendar::Selection,
    pub initial_month: gpuio_protocol::calendar::Month,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SplitGroupMount {
    pub config: Arc<gpuio_protocol::split_group::Config>,
    pub appearance: Arc<gpuio_protocol::split_group_appearance::Config>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub kind: Kind,
    pub text: Arc<str>,
    pub text_spans: Arc<[gpuio_protocol::text_content::Span]>,
    pub text_shimmer: Option<Arc<gpuio_protocol::text_shimmer::Config>>,
    pub editor: Option<Arc<EditorConfig>>,
    pub editor_privacy: EditorPrivacy,
    pub editor_content_hint: Option<gpuio_protocol::input_content_hint::Hint>,
    pub editor_format: Option<Arc<gpuio_protocol::input_format::Config>>,
    pub editor_clear_on_escape: bool,
    pub editor_searchable: bool,
    pub textarea_layout: Option<gpuio_protocol::text_area_layout::Config>,
    pub(crate) editor_validation: Option<Arc<crate::input_validation::Policy>>,
    pub editor_frame: Option<Arc<gpuio_protocol::editor_frame::Config>>,
    pub editor_frame_activation_revision: i64,
    pub control: Option<Control>,
    pub link: Option<Arc<gpuio_protocol::link::Config>>,
    pub choice: Option<Arc<ChoiceConfig>>,
    pub choice_picker: Option<Arc<gpuio_protocol::choice_picker::Presentation>>,
    pub focus_scope: Option<FocusScopeConfig>,
    pub overlay_backdrop: Option<i64>,
    pub overlay_motion: bool,
    pub tooltip_motion: bool,
    pub window_region: Option<Arc<gpuio_protocol::window_region::Region>>,
    pub placement_geometry: Option<gpuio_protocol::placement_geometry::Config>,
    pub sheet_insets: Option<gpuio_protocol::sheet_insets::Insets>,
    pub overlay: Option<Arc<OverlayConfig>>,
    pub tooltip: Option<Arc<TooltipConfig>>,
    pub commands: Option<Arc<[Arc<CommandConfig>]>>,
    pub command_ref: Option<Arc<str>>,
    pub menu: Option<Arc<MenuConfig>>,
    pub palette: Option<Arc<PaletteConfig>>,
    pub palette_observed: bool,
    pub palette_options: Option<Arc<gpuio_protocol::palette_options::Config>>,
    pub palette_layout: Option<Arc<gpuio_protocol::palette_layout::Config>>,
    pub progress: Option<Arc<ProgressConfig>>,
    pub progress_presentation: Option<Arc<gpuio_protocol::progress_presentation::Config>>,
    pub loading: Option<Arc<gpuio_protocol::loading::Config>>,
    pub spinner: Option<Arc<gpuio_protocol::spinner::Config>>,
    pub image: Option<Arc<ImageConfig>>,
    pub avatar: Option<Arc<gpuio_protocol::avatar::Config>>,
    pub rating: Option<Arc<gpuio_protocol::rating::Config>>,
    pub tab_order: Option<gpuio_protocol::checkable::TabOrder>,
    pub split_button: Option<Arc<gpuio_protocol::split_button::Config>>,
    pub button_presentation: Option<gpuio_protocol::button::Config>,
    pub button_activation_revision: i64,
    pub tab_appearance: Option<Arc<gpuio_protocol::tab_appearance::Config>>,
    pub tab_content: Option<Arc<gpuio_protocol::tab_content::Config>>,
    pub tab_viewport: Option<Arc<gpuio_protocol::tab_viewport::Config>>,
    pub tab_motion: Option<Arc<gpuio_protocol::tab_motion::Config>>,
    pub tab_trailing: bool,
    pub choice_menu: bool,
    pub control_appearance: Option<Arc<gpuio_protocol::control_appearance::Config>>,
    pub rating_appearance: Option<Arc<gpuio_protocol::rating::Appearance>>,
    pub slider: Option<SliderMount>,
    pub number_input: Option<NumberInputMount>,
    pub number_step_mode: gpuio_protocol::number_input::StepMode,
    pub number_presentation: Option<Arc<gpuio_protocol::number_presentation::Config>>,
    pub otp_input: Option<OtpInputMount>,
    pub popover: bool,
    pub reveal: Option<gpuio_protocol::reveal::Config>,
    pub calendar_appearance: Option<Arc<gpuio_protocol::calendar_presentation::Appearance>>,
    pub calendar_content: Option<Arc<gpuio_protocol::calendar_content::Config>>,
    pub color_presentation: Option<Arc<gpuio_protocol::color_presentation::Presentation>>,
    pub slider_appearance: Option<Arc<gpuio_protocol::slider_presentation::Appearance>>,
    pub otp_appearance: Option<Arc<gpuio_protocol::otp_presentation::Appearance>>,
    pub calendar: Option<CalendarMount>,
    pub color_input: Option<ColorInputMount>,
    pub extension: Option<Arc<gpuio_protocol::extension::Config>>,
    pub extension_command: Option<Arc<gpuio_protocol::extension::Command>>,
    pub split: Option<Arc<gpuio_protocol::split::Config>>,
    pub split_group: Option<SplitGroupMount>,
    pub document: Option<Arc<gpuio_protocol::document::Config>>,
    pub document_diff: Option<Arc<gpuio_protocol::document_diff::Config>>,
    pub document_diff_epoch: i64,
    pub document_selection_markdown: bool,
    pub document_markdown_options: gpuio_protocol::document::MarkdownOptions,
    pub document_text_style: Option<Arc<gpuio_protocol::document_style::Config>>,
    pub document_profile: Option<Arc<gpuio_protocol::document_profile::Config>>,
    pub document_actions: Option<Arc<gpuio_protocol::document_actions::Config>>,
    pub document_preview: Option<Arc<gpuio_protocol::document_preview::Config>>,
    pub canvas: Option<Arc<gpuio_protocol::canvas_view::Config>>,
    pub chart: Option<Arc<gpuio_protocol::chart_view::Config>>,
    pub animation: Option<Arc<gpuio_protocol::animation::Config>>,
    pub animation_program: Option<Arc<gpuio_protocol::animation_program::Config>>,
    pub navigation_stack: Option<gpuio_protocol::navigation_stack::Config>,
    pub carousel: Option<Arc<gpuio_protocol::carousel::Config>>,
    pub carousel_track: Option<Arc<gpuio_protocol::carousel_track::Config>>,
    pub carousel_track_motion: Option<gpuio_protocol::carousel_track::Motion>,
    pub table: Option<Arc<gpuio_protocol::table::Config>>,
    pub table_cell: Option<Arc<gpuio_protocol::table::Cell>>,
    pub table_header: Option<Arc<gpuio_protocol::table_header::Target>>,
    pub table_header_style: Arc<[Style]>,
    pub table_row_style: Arc<[Style]>,
    pub table_behavior: Option<Arc<gpuio_protocol::table::Behavior>>,
    pub table_appearance: Option<Arc<gpuio_protocol::table::Appearance>>,
    pub table_serial: i64,
    pub tree_input: bool,
    pub list_input: Option<gpuio_protocol::list_input::Config>,
    pub list_input_generation: i64,
    pub tree_moves: bool,
    pub container_query: Option<Arc<gpuio_protocol::container_query::Config>>,
    pub accessibility: Option<Arc<gpuio_protocol::accessibility::Config>>,
    pub list_axis: gpuio_protocol::list::Axis,
    pub list_config: Option<Arc<gpuio_protocol::list::Config>>,
    pub list_order: Option<Arc<gpuio_protocol::list::Order>>,
    pub list_index: Option<Arc<crate::list_index::Index>>,
    pub list_rows: Arc<[gpuio_protocol::list::Row]>,
    pub toast: Option<Arc<ToastConfig>>,
    pub toast_stack: Option<Arc<ToastStackConfig>>,
    pub toast_placement: Option<gpuio_protocol::toast_placement::Placement>,
    pub toast_layering: Option<gpuio_protocol::toast_layering::Layering>,
    pub toast_motion: Option<gpuio_protocol::toast_motion::Config>,
    pub scrollbar: Option<Arc<gpuio_protocol::scrollbar::Config>>,
    pub drag_source: Option<Arc<gpuio_protocol::drag_drop::Source>>,
    pub drop_target: Option<Arc<gpuio_protocol::drag_drop::Target>>,
    pub pointer: Option<Arc<PointerConfig>>,
    pub input_region: Option<Arc<gpuio_protocol::input::Config>>,
    pub highlight_scope: Option<Arc<gpuio_protocol::highlight::Config>>,
    pub command_binding: Option<Arc<gpuio_protocol::command_binding::Config>>,
    pub placement: Option<Placement>,
    pub combobox_filter: Option<ComboboxFilter>,
    pub choice_appearance: Option<Arc<ChoiceAppearance>>,
    pub style: Arc<[Style]>,
    pub handler: Option<HandlerId>,
    pub hover_handler: Option<HandlerId>,
    pub calendar_viewport_handler: Option<HandlerId>,
    pub children: Arc<[NodeId]>,
    pub parent: Option<NodeId>,
}

impl Node {
    fn payload_bytes(&self) -> usize {
        self.table
            .as_ref()
            .map_or(0, |config| config.retained_bytes())
            + self
                .table_cell
                .as_ref()
                .map_or(0, |cell| cell.retained_bytes())
            + self
                .carousel_track
                .as_ref()
                .map_or(0, |config| config.retained_bytes()
                // Current/previous native paint contexts may retain replaced curves.
                + 2 * (gpuio_protocol::animation::MAX_LINEAR_STOPS * 16 + 2 * std::mem::size_of::<usize>()))
            + self.carousel_track_motion.as_ref().map_or(0, |motion| motion.easing.heap_bytes())
            + self
                .carousel
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .color_input
                .as_ref()
                .map_or(0, |color| 32 + color.config.retained_bytes())
            + self
                .calendar
                .as_ref()
                .map_or(0, |calendar| 160 + calendar.config.retained_bytes())
            + self
                .otp_input
                .as_ref()
                .map_or(0, |s| s.config.retained_bytes() + s.initial.len())
            + self.number_presentation.as_ref().map_or(0, |config| {
                crate::number_presentation::retained_bytes(config)
            })
            + self.number_input.as_ref().map_or(0, |s| {
                s.config.retained_bytes() + s.initial_draft.as_ref().map_or(0, |draft| draft.len())
            })
            + self.slider.as_ref().map_or(0, |s| {
                s.config.retained_bytes() + crate::slider_state::INTERACTION_RESERVED_BYTES
            })
            + self.choice_picker.as_ref().map_or(0, |p| {
                crate::choice_picker_admission::retained_bytes(p)
                    + crate::choice_picker_admission::owner_reserved_bytes(p)
            })
            + self.rating.as_ref().map_or(0, |c| c.retained_bytes())
            + self
                .tab_appearance
                .as_ref()
                .map_or(0, |config| crate::tab_appearance::retained_bytes(config))
            + self.tab_motion.as_ref().map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::tab_motion::Config>()
                    + gpuio_protocol::tab_motion::Config::OWNER_RESERVED_BYTES
            })
            + self.tab_content.as_ref().map_or(0, |c| c.retained_bytes())
            + self.tab_viewport.as_ref().map_or(0, |c| {
                c.retained_bytes()
                    + gpuio_protocol::tab_viewport::Config::owner_reserved_bytes(
                        self.choice.as_ref().map_or(0, |c| c.items.len()),
                    )
            })
            + self.control_appearance.as_ref().map_or(0, |appearance| {
                crate::control_appearance::retained_bytes(appearance)
            })
            + self.reveal.map_or(0, |_| {
                crate::reveal_motion::RESERVED_BYTES
                    + std::mem::size_of::<gpuio_protocol::reveal::Config>()
            })
            + self.slider_appearance.as_ref().map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::slider_presentation::Appearance>()
            })
            + self.calendar_appearance.as_ref().map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::calendar_presentation::Appearance>()
            })
            + self
                .calendar_content
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .color_presentation
                .as_ref()
                .map_or(0, |p| p.retained_bytes())
            + self.otp_appearance.as_ref().map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::otp_presentation::Appearance>()
            })
            + self.rating_appearance.as_ref().map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::rating::Appearance>()
            })
            + self.avatar.as_ref().map_or(0, |c| c.retained_bytes())
            + self.loading.as_ref().map_or(0, |c| c.retained_bytes())
            + self.spinner.as_ref().map_or(0, |c| {
                c.retained_bytes() + crate::spinner_clock::RESERVED_BYTES
            })
            + self.navigation_stack.map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::navigation_stack::Config>()
            })
            + self
                .editor_frame
                .as_ref()
                .map_or(0, |frame| frame.retained_bytes())
            + self
                .editor_format
                .as_ref()
                .map_or(0, |config| crate::input_format::retained_bytes(config))
            + self
                .editor_validation
                .as_ref()
                .map_or(0, |policy| policy.retained_bytes())
            + self.textarea_layout.map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::text_area_layout::Config>()
            })
            + self.text.len()
            + self
                .accessibility
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .container_query
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .animation_program
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .canvas
                .as_ref()
                .map_or(0, |config| config.label.len() + 256)
            + self
                .chart
                .as_ref()
                .map_or(0, |config| config.label.len() + 512)
            + self.extension.as_ref().map_or(0, |config| {
                256 + config.schema.name.len()
                    + config.schema.fingerprint.len()
                    + config.label.len()
                    + config.properties.0.len()
                    + config
                        .command
                        .as_ref()
                        .map_or(0, |command| command.payload.0.len())
            })
            + self
                .extension_command
                .as_ref()
                .map_or(0, |command| command.payload.0.len() + 32)
            + self
                .split
                .as_ref()
                .map_or(0, |config| config.label.len() + 128)
            + self.split_group.as_ref().map_or(0, |mount| {
                // Conservative native owner/focus/frame reservation per panel,
                // plus retained descriptions and variable-size handle paint.
                8192 + mount.config.panels.len() * 4096
                    + mount.config.label.len()
                    + mount
                        .config
                        .panels
                        .iter()
                        .map(|p| p.id.len() + p.label.len())
                        .sum::<usize>()
                    + mount.config.resize.as_ref().map_or(0, |r| r.id.len())
                    + crate::split_group_appearance::retained_bytes(&mount.appearance)
            })
            + self.list_order.as_ref().map_or(0, |order| {
                // Admission units include expanded indexing and GPUI measurement
                // metadata, not merely the compact serialized runs. This is a
                // conservative quota, not a measurement of allocator RSS.
                order
                    .runs
                    .iter()
                    .map(|run| run.count as usize)
                    .sum::<usize>()
                    * 192
                    + std::mem::size_of_val(order.runs.as_slice())
            })
            + std::mem::size_of_val(self.list_rows.as_ref())
            + self
                .drag_source
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .drop_target
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .input_region
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .document_diff
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self.document_text_style.as_ref().map_or(0, |style| crate::document_style::retained_bytes(style))
            + self.document_profile.as_ref().map_or(0, |c| c.retained_bytes())
            + self.document_actions.as_ref().map_or(0, |c| c.retained_bytes())
            + self.document_preview.as_ref().map_or(0, |_| std::mem::size_of::<gpuio_protocol::document_preview::Config>() + 256)
            + self.command_binding.as_ref().map_or(0, |config| {
                config.retained_bytes()
                    + 2 * (gpuio_protocol::command_binding::MAX_OBSERVATION_BYTES + 256)
            })
            + self
                .highlight_scope
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .pointer
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .toast
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self.toast_stack.as_ref().map_or(0, |config| {
                std::mem::size_of::<ToastStackConfig>() + config.label.len()
            })
            + self
                .toast_layering
                .map_or(0, |c| c.retained_bytes(self.children.len()))
            + self
                .toast_motion
                .map_or(0, |c| c.retained_bytes(self.children.len()))
            // Conservative BTreeMap query-to-owner registration accounting.
            + self.list_input.and_then(|config| config.query).map_or(0, |_| 128)
            + [&self.table_header_style, &self.table_row_style].iter().map(|styles| {
                if styles.is_empty() { 0 } else { 128 + std::mem::size_of_val(&***styles)
                    + styles.iter().map(crate::style::retained_bytes).sum::<usize>() }
            }).sum::<usize>()
            + self.table_header.as_ref().map_or(0, |target| target.retained_bytes() + 128)
            + self.table_behavior.as_ref().map_or(0, |config| config.retained_bytes())
            + self.table_appearance.as_ref().map_or(0, |config| config.retained_bytes())
            + self.scrollbar.as_ref().map_or(0, |c| c.retained_bytes())
            + self.window_region.as_ref().map_or(0, |_| crate::host::window_regions::RESERVED_BYTES)
            + self.toast_placement.map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::toast_placement::Placement>()
            })
            + self
                .document
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self.image.as_ref().map_or(0, |config| {
                std::mem::size_of::<ImageConfig>() + config.label.as_ref().map_or(0, String::len)
            })
            + self.animation.as_ref().map_or(0, |config| {
                std::mem::size_of::<gpuio_protocol::animation::Config>()
                    + std::mem::size_of_val(config.targets.as_slice())
                    + config
                        .initial
                        .as_ref()
                        .map_or(0, |targets| std::mem::size_of_val(targets.as_slice()))
            })
            + self.progress_presentation.as_ref().map_or(0, |config| {
                config.retained_bytes() + crate::progress_clock::RESERVED_BYTES
            })
            + self.progress.as_ref().map_or(0, |config| {
                std::mem::size_of::<ProgressConfig>() + config.label.len()
            })
            + self.commands.as_ref().map_or(0, |commands| {
                commands
                    .iter()
                    .map(|command| {
                        command.retained_bytes()
                            + std::mem::size_of::<Arc<CommandConfig>>()
                            + 2 * std::mem::size_of::<usize>()
                    })
                    .sum::<usize>()
            })
            + self.command_ref.as_ref().map_or(0, |id| id.len())
            + self.menu.as_ref().map_or(0, |menu| menu.retained_bytes())
            + self
                .palette
                .as_ref()
                .map_or(0, |palette| palette.retained_bytes())
            + self.palette_options.as_ref().map_or(0, |options| options.retained_bytes())
            + self.palette_layout.as_ref().map_or(0, |layout| layout.retained_bytes())
            + self.tooltip.as_ref().map_or(0, |config| {
                std::mem::size_of::<TooltipConfig>()
                    + config.label.len()
                    + if self.kind == Kind::Tooltip { 512 } else { 0 }
            })
            + self
                .placement
                .map_or(0, |_| std::mem::size_of::<Placement>())
            + self.overlay.as_ref().map_or(0, |config| {
                std::mem::size_of::<OverlayConfig>()
                    + config.label.len()
                    + if config.kind.is_modal() { 256 } else { 0 }
            })
            + self
                .split_button
                .as_ref()
                .map_or(0, |config| crate::split_button::retained_bytes(config))
            + self.choice_appearance.as_ref().map_or(0, |appearance| {
                crate::appearance::retained_bytes(appearance)
            })
            + self
                .choice
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .editor
                .as_ref()
                .map_or(0, |config| config.label.len() + config.placeholder.len())
            + if matches!(self.kind, Kind::Input | Kind::Textarea | Kind::Combobox) {
                EDITOR_RESERVED_BYTES
            } else {
                0
            }
            + self
                .text_shimmer
                .as_ref()
                .map_or(0, |_| crate::text_shimmer_clock::RESERVED_BYTES)
            + std::mem::size_of_val(self.text_spans.as_ref())
            + self.link.as_ref().map_or(0, |config| {
                config.label.len() + std::mem::size_of::<gpuio_protocol::link::Config>()
            })
            + std::mem::size_of_val(self.style.as_ref())
            + self
                .style
                .iter()
                .map(crate::style::retained_bytes)
                .sum::<usize>()
            + std::mem::size_of_val(self.children.as_ref())
    }
}

#[derive(Clone, Debug, Default)]
struct Slot {
    generation: u32,
    node: Option<Node>,
}

#[derive(Debug)]
pub struct Tree {
    binding_owners: BTreeSet<NodeId>,
    list_query_owners: BTreeMap<NodeId, NodeId>,
    window: WindowId,
    revision: i64,
    root: Option<NodeId>,
    slots: Vec<Slot>,
    node_count: usize,
    extension_count: usize,
    canvas_count: usize,
    chart_count: usize,
    retained_bytes: usize,
}

#[derive(Debug, PartialEq)]
pub struct Applied {
    pub revision: i64,
    pub touched_records: usize,
    pub validated_nodes: usize,
    pub dirty: Vec<NodeId>,
    pub lists: Vec<ListAction>,
    pub tables: Vec<(NodeId, gpuio_protocol::table::Command)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ListAction {
    Invalidate(NodeId, Vec<i64>),
    Scroll(NodeId, gpuio_protocol::list::ScrollRequest),
}

impl Tree {
    pub fn new(window: WindowId) -> Self {
        Self {
            window,
            binding_owners: BTreeSet::new(),
            list_query_owners: BTreeMap::new(),
            revision: 0,
            root: None,
            slots: Vec::new(),
            node_count: 0,
            extension_count: 0,
            canvas_count: 0,
            chart_count: 0,
            retained_bytes: 0,
        }
    }

    pub fn binding_owners(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.binding_owners.iter().copied()
    }

    /// Exact query incarnation to input owner. Keyboard routing need not scan
    /// unrelated lists or traverse application rows on every keystroke.
    pub fn list_query_owner(&self, query: NodeId) -> Option<NodeId> {
        self.list_query_owners.get(&query).copied()
    }

    pub fn revision(&self) -> i64 {
        self.revision
    }
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }
    pub fn len(&self) -> usize {
        self.node_count
    }
    pub fn is_empty(&self) -> bool {
        self.node_count == 0
    }

    /// The only toggle in either supported disclosure header shape. Custom
    /// content is independent; no arbitrary descendant search can steal a link.
    pub fn disclosure_trigger(&self, id: NodeId) -> Option<NodeId> {
        let disclosure = self.get(id)?;
        if disclosure.kind != Kind::Disclosure {
            return None;
        }
        let header = self.get(*disclosure.children.first()?)?;
        let trigger = match header.kind {
            Kind::Button => header,
            Kind::Container => self.get(*header.children.last()?)?,
            _ => return None,
        };
        (trigger.kind == Kind::Button).then_some(trigger.id)
    }

    /// Only a direct button anchor owns a popover's trigger semantics. Do not
    /// search arbitrary custom anchor subtrees for a supposed activation owner.
    pub fn popover_for_trigger(&self, id: NodeId) -> Option<&Node> {
        let trigger = self.get(id)?;
        if !matches!(trigger.kind, Kind::Button | Kind::CommandButton) {
            return None;
        }
        let parent = self.get(trigger.parent?)?;
        (parent.popover && parent.children.first() == Some(&id)).then_some(parent)
    }

    pub fn disclosure_for_trigger(&self, id: NodeId) -> Option<&Node> {
        let parent = self.get(self.get(id)?.parent?)?;
        let disclosure = if parent.kind == Kind::Container {
            self.get(parent.parent?)?
        } else {
            parent
        };
        (self.disclosure_trigger(disclosure.id) == Some(id)).then_some(disclosure)
    }

    /// Explicitly admitted outer group, never inferred from a user key/label.
    pub(crate) fn carousel_track_control(&self, control: NodeId) -> Option<NodeId> {
        let node = self.get(control)?;
        if node.kind != Kind::Button {
            return None;
        }
        let controls = self.get(node.parent?)?;
        let group = self.get(controls.parent?)?;
        (group.kind == Kind::CarouselTrackGroup && group.children.get(1) == Some(&controls.id))
            .then(|| group.children[0])
    }
    pub(crate) fn carousel_track_item(&self, item: NodeId) -> Option<(usize, usize)> {
        let track = self.get(self.get(item)?.parent?)?;
        let viewport = self.get(track.parent?)?;
        let config = viewport.carousel_track.as_ref()?;
        Some((
            track.children.iter().position(|id| *id == item)?,
            config.carousel.ids.len(),
        ))
    }
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.slots
            .get(id.slot())
            .filter(|s| s.generation == id.generation())?
            .node
            .as_ref()
    }

    /// Nearest enabled tooltip whose anchor subtree contains this node.
    pub fn tooltip_description(&self, node: NodeId) -> Option<&str> {
        let mut child = node;
        while let Some(parent) = self.get(child)?.parent {
            let node = self.get(parent)?;
            if let Some(config) = &node.tooltip
                && node.kind == Kind::Tooltip
                && !config.disabled
                && node.children.first() == Some(&child)
            {
                return Some(&config.label);
            }
            child = parent;
        }
        None
    }

    /// Resolve the nearest matching registry entry, preserving shadowing even
    /// when that entry is disabled.
    pub fn command(&self, node: NodeId, command: &str) -> Option<(NodeId, &Arc<CommandConfig>)> {
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let node = self.get(id)?;
            if let Some(commands) = &node.commands
                && let Some(command) = commands.iter().find(|entry| entry.id == command)
            {
                return Some((id, command));
            }
            cursor = node.parent;
        }
        None
    }

    /// Enumerate effective declarations at a native tree position, nearest scope
    /// first and preserving declaration order within each scope. Shadow IDs
    /// before callers filter by shortcut, enabled state or input policy: an inner
    /// definition with no usable shortcut must still hide the outer definition.
    /// The iterator borrows the tree; it retains no command snapshots or owners.
    fn command_steps_from(
        &self,
        node: NodeId,
    ) -> impl Iterator<Item = (NodeId, Option<&Arc<CommandConfig>>)> {
        std::iter::successors(Some(node), |id| self.get(*id)?.parent).flat_map(|id| {
            std::iter::once((id, None)).chain(
                self.get(id)
                    .and_then(|node| node.commands.as_ref())
                    .into_iter()
                    .flat_map(move |commands| {
                        commands.iter().map(move |command| (id, Some(command)))
                    }),
            )
        })
    }

    pub fn commands_from(
        &self,
        node: NodeId,
    ) -> impl Iterator<Item = (NodeId, &Arc<CommandConfig>)> {
        let mut seen = BTreeSet::new();
        self.command_steps_from(node)
            .filter_map(|(id, command)| command.map(|command| (id, command)))
            .filter(move |(_, command)| seen.insert(command.id.as_str()))
    }

    /// Count ancestors and shadowed declarations as work too. A result is whole
    /// or absent; exhausting the budget must never look like a missing command.
    pub fn bounded_commands_from(
        &self,
        node: NodeId,
        remaining: &mut usize,
    ) -> Option<Vec<(NodeId, &Arc<CommandConfig>)>> {
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for (id, command) in self.command_steps_from(node) {
            *remaining = remaining.checked_sub(1)?;
            if let Some(command) = command
                && seen.insert(command.id.as_str())
            {
                result.push((id, command));
            }
        }
        Some(result)
    }

    pub fn accepts_handler(&self, node: NodeId, handler: HandlerId) -> bool {
        self.get(node)
            .is_some_and(|node| node.handler == Some(handler))
    }

    /// Validate a touched-record overlay before publishing any mutation. Text and
    /// style-only edits do not traverse unrelated nodes. Structural validation
    /// walks the final tree iteratively, bounded by MAX_NODES and MAX_DEPTH.
    pub fn apply(&mut self, tx: &Transaction) -> Result<Applied, ErrorCode> {
        self.apply_with_budget(tx, MAX_RETAINED_BYTES)
    }

    /// Payload budget excludes fixed-size slots, independently bounded by MAX_NODES.
    /// A session may lower the budget to enforce its aggregate memory ceiling.
    pub fn apply_with_budget(
        &mut self,
        tx: &Transaction,
        budget: usize,
    ) -> Result<Applied, ErrorCode> {
        self.apply_guarded(tx, budget, &[])
            .map_err(ApplyFailure::rejection)
    }

    pub fn apply_guarded(
        &mut self,
        tx: &Transaction,
        budget: usize,
        pins: &[gpuio_protocol::list::Retained],
    ) -> Result<Applied, ApplyFailure> {
        self.apply_with_admission(tx, budget, pins, |_| Ok(()))
    }

    pub(crate) fn apply_with_admission(
        &mut self,
        tx: &Transaction,
        budget: usize,
        pins: &[gpuio_protocol::list::Retained],
        admit: impl FnOnce(&[ProgramChange]) -> Result<(), ErrorCode>,
    ) -> Result<Applied, ApplyFailure> {
        if tx.window != self.window {
            return Err(ErrorCode::StaleHandle.into());
        }
        if tx.base != self.revision || self.revision.checked_add(1) != Some(tx.revision) {
            return Err(ErrorCode::InvalidRevision.into());
        }
        if tx.operations.len() > MAX_OPERATIONS {
            return Err(ErrorCode::LimitExceeded.into());
        }
        let mut plan = Plan {
            original: self,
            changes: BTreeMap::new(),
            root: self.root,
            slot_count: self.slots.len(),
            node_count: self.node_count,
            extension_count: self.extension_count,
            canvas_count: self.canvas_count,
            chart_count: self.chart_count,
            retained_bytes: self.retained_bytes,
            budget: budget.min(MAX_RETAINED_BYTES),
            structural: false,
            lists: Vec::new(),
            tables: Vec::new(),
        };
        let mut extension_updates = BTreeSet::new();
        let mut canvas_updates = BTreeSet::new();
        let mut chart_updates = BTreeSet::new();
        let mut program_updates = BTreeSet::new();
        let mut query_updates = BTreeSet::new();
        let mut table_updates = BTreeSet::new();
        for op in &tx.operations {
            if let Op::SetTable(id, _) = op
                && !table_updates.insert(*id)
            {
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetContainerQuery(id, _) = op
                && !query_updates.insert(*id)
            {
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetAnimationProgram(id, _) = op
                && !program_updates.insert(*id)
            {
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetExtension(id, _) = op
                && !extension_updates.insert(*id)
            {
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetCanvas(id, _) = op
                && !canvas_updates.insert(*id)
            {
                // Each command must reach the mounted state; a later config in
                // this transaction cannot silently replace an earlier one.
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetChart(id, _) = op
                && !chart_updates.insert(*id)
            {
                // One configuration per chart in an atomic transaction.
                return Err(ErrorCode::InvalidTree.into());
            }
            plan.operation(op)?;
        }
        // A viewport-only update must also validate its unchanged carousel owner.
        // Include it in dirty output so owner scheduling sees the admitted snapshot.
        let carousel_parents = plan
            .changes
            .values()
            .filter_map(|slot| {
                let parent = slot.node.as_ref()?.parent?;
                let owner = plan.node(parent).ok()?;
                if owner.carousel.is_none()
                    && owner.carousel_track.is_none()
                    && owner.kind != Kind::CarouselTrackGroup
                {
                    return None;
                }
                Some(parent)
            })
            .collect::<BTreeSet<_>>();
        for parent in carousel_parents {
            plan.node_mut(parent)?;
        }
        for slot in plan.changes.values() {
            if let Some(node) = &slot.node {
                plan.validate_list(node)?;
                plan.validate_track_group(node)?;
                if node.reveal.is_some_and(|config| {
                    (!config.expanded && !crate::style::display_none(&node.style))
                        || (!config.expanded && !config.retain && !node.children.is_empty())
                }) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node
                    .editor_content_hint
                    .is_some_and(|hint| hint.is_password())
                    && node.editor_privacy == EditorPrivacy::Plain
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if let Some(config) = &node.text_shimmer {
                    if node.kind != Kind::Text || !config.is_valid() {
                        return Err(ErrorCode::InvalidTree.into());
                    }
                    if node.text.len() > gpuio_protocol::text_shimmer::MAX_TEXT_BYTES {
                        return Err(ErrorCode::LimitExceeded.into());
                    }
                }
                if node.choice_appearance.is_some()
                    && !matches!(
                        node.kind,
                        Kind::Select | Kind::Combobox | Kind::Menu | Kind::CommandPalette
                    )
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.choice.is_some()
                    && !matches!(
                        node.kind,
                        Kind::RadioGroup | Kind::TabBar | Kind::Select | Kind::Combobox
                    )
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node
                    .control
                    .is_some_and(|control| control.kind() != node.kind)
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.kind == Kind::Combobox {
                    let choices = node.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
                    let editor = node.editor.as_ref().ok_or(ErrorCode::InvalidTree)?;
                    if node.combobox_filter.is_none()
                        || !choices.is_valid()
                        || choices.label != editor.label
                        || choices.disabled != editor.disabled
                        || editor.read_only
                        || editor.submit_on_enter
                    {
                        return Err(ErrorCode::InvalidTree.into());
                    }
                } else if node.combobox_filter.is_some() {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::DragSource) != node.drag_source.is_some()
                    || (node.kind == Kind::DropTarget) != node.drop_target.is_some()
                    || ((node.drag_source.is_some() || node.drop_target.is_some())
                        && (node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.document_markdown_options != Default::default()
                    && node.document.as_ref().is_none_or(|config| {
                        !matches!(config.mode, gpuio_protocol::document::Mode::Markdown)
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.document_text_style.is_some()
                    && node.document.as_ref().is_none_or(|config| {
                        !matches!(
                            config.mode,
                            gpuio_protocol::document::Mode::Markdown
                                | gpuio_protocol::document::Mode::Html
                        )
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node
                    .document_profile
                    .as_ref()
                    .is_some_and(|config| config.instance.is_some())
                    && (node.handler.is_none()
                        || node.document.as_ref().is_none_or(|config| {
                            !matches!(
                                config.mode,
                                gpuio_protocol::document::Mode::Markdown
                                    | gpuio_protocol::document::Mode::Html
                            )
                        }))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.document_actions.as_ref().is_some_and(|actions| {
                    actions.enabled()
                        && (node.document.as_ref().is_none_or(|config| {
                            !matches!(
                                config.mode,
                                gpuio_protocol::document::Mode::Markdown
                                    | gpuio_protocol::document::Mode::Html
                            )
                        }) || (actions.observe && node.handler.is_none()))
                }) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.document_preview.as_ref().is_some_and(|preview| {
                    preview.enabled()
                        && (node.document.as_ref().is_none_or(|config| {
                            !matches!(
                                config.mode,
                                gpuio_protocol::document::Mode::Markdown
                                    | gpuio_protocol::document::Mode::Html
                            ) || config.layout != gpuio_protocol::document::Layout::Flow
                        }) || (preview.observe && node.handler.is_none()))
                }) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.document_diff.is_some()
                    && (node.document_diff_epoch <= 0
                        || node.document.as_ref().is_none_or(|config| {
                            config.mode != gpuio_protocol::document::Mode::Diff
                        }))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Link) != node.link.is_some()
                    || node.link.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || (!config.disabled && !config.loading && node.handler.is_none())
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.command_binding.as_ref().is_some_and(|config| {
                    node.kind != Kind::Container || node.handler.is_none() || !config.is_valid()
                        || !node.text.is_empty() || node.control.is_some() || node.choice.is_some()
                        || matches!(config.context, gpuio_protocol::command_binding::Context::Editor(window, _) if window != self.window)
                }) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if let Some(old) = self.get(node.id)
                    && old.command_binding != node.command_binding
                    && old.command_binding.is_some()
                    && old.handler == node.handler
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::HighlightScope) != node.highlight_scope.is_some()
                    || node.highlight_scope.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if let Some(old) = self.get(node.id)
                    && old.highlight_scope != node.highlight_scope
                    && old.highlight_scope.is_some()
                    && old.handler.is_some()
                    && old.handler == node.handler
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::InputRegion) != node.input_region.is_some()
                    || node.input_region.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if let Some(old) = self.get(node.id)
                    && old.input_region != node.input_region
                    && old.input_region.is_some()
                    && old.handler == node.handler
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::PointerArea) != node.pointer.is_some()
                    || node.pointer.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Toast) != node.toast.is_some()
                    || node.toast.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.toast_placement.is_some()
                    || node.toast_layering.is_some()
                    || node.toast_motion.is_some())
                    && node.kind != Kind::ToastStack
                    || (node.kind == Kind::ToastStack) != node.toast_stack.is_some()
                    || node.toast_stack.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_some()
                            || !node.text.is_empty()
                            || node.children.len() > MAX_TOASTS
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::ContainerQuery) != node.container_query.is_some()
                    || node.container_query.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || node.children.len() != config.branches.len()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::AnimationProgram) != node.animation_program.is_some()
                    || node.animation_program.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Animated) != node.animation.is_some()
                    || node.animation.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (matches!(node.kind, Kind::Image | Kind::Icon)
                    || node.avatar.as_ref().is_some_and(|c| c.source.is_some())
                    || node.spinner.as_ref().is_some_and(|c| c.source.is_some()))
                    != node.image.is_some()
                    || node.image.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || (node.kind != Kind::Avatar && !node.children.is_empty())
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Slider) != node.slider.is_some()
                    || node.slider.as_ref().is_some_and(|slider| {
                        !slider.config.is_valid()
                            || !slider.initial.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::ColorInput) != node.color_input.is_some()
                    || node.color_input.as_ref().is_some_and(|color| {
                        !color.config.is_valid()
                            || node
                                .color_presentation
                                .as_ref()
                                .is_some_and(|p| !p.fits(color.config.palette.len()))
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Calendar) != node.calendar.is_some()
                    || node.calendar.as_ref().is_some_and(|calendar| {
                        !calendar.config.is_valid()
                            || !calendar.initial.fits(calendar.config.mode)
                            || !node.text.is_empty()
                            || (node.calendar_content.is_none() && !node.children.is_empty())
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::OtpInput) != node.otp_input.is_some()
                    || node.otp_input.as_ref().is_some_and(|input| {
                        !input.config.is_valid()
                            || !input.config.policy.canonical(&input.initial)
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::NumberInput) != node.number_input.is_some()
                    || node.number_input.as_ref().is_some_and(|number_input| {
                        !number_input.config.is_valid()
                            || !number_input.initial.is_valid()
                            || number_input.initial_draft.as_ref().is_some_and(|draft| {
                                !gpuio_protocol::number_input::valid_text(draft)
                            })
                            || !node.text.is_empty()
                            || (node.number_presentation.is_none() && !node.children.is_empty())
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Rating) != node.rating.is_some()
                    || node.rating.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || (!config.disabled && !config.read_only && node.handler.is_none())
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Avatar) != node.avatar.is_some()
                    || node.avatar.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !config.matches_image(node.image.as_deref())
                            || !node.text.is_empty()
                            || node.children.len() > 1
                            || node.control.is_some()
                            || node.choice.is_some()
                            || (config.source.is_none() && node.handler.is_some())
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Extension) != node.extension.is_some()
                    || node.extension.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::SplitGroup) != node.split_group.is_some()
                    || node.split_group.as_ref().is_some_and(|m| {
                        !m.config.is_valid()
                            || !node.text.is_empty()
                            || node.children.len() != m.config.panels.len()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::SplitPane) != node.split.is_some()
                    || node
                        .split
                        .as_ref()
                        .is_some_and(|config| !config.is_valid() || node.children.len() != 2)
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::CanvasView) != node.canvas.is_some()
                    || node.canvas.as_ref().is_some_and(|config| {
                        !config.is_valid() || !node.text.is_empty() || !node.children.is_empty()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::ChartView) != node.chart.is_some()
                    || node.chart.as_ref().is_some_and(|config| {
                        !config.is_valid() || !node.text.is_empty() || !node.children.is_empty()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::DocumentView) != node.document.is_some()
                    || node.document.as_ref().is_some_and(|config| {
                        !config.is_valid() || !node.text.is_empty() || !node.children.is_empty()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.spinner.as_ref().is_some_and(|config| {
                    node.kind != Kind::Loading
                        || !config.is_valid()
                        || node.loading.as_ref().is_none_or(|loading| {
                            loading.kind != gpuio_protocol::loading::Kind::Spinner
                                || loading.label != config.label
                                || loading.animated != config.animated
                                || loading.period_ms != config.period_ms
                        })
                        || node.image.as_deref() != config.image().as_ref()
                }) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Loading) != node.loading.is_some()
                    || node.loading.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || (node.handler.is_some() && node.image.is_none())
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Progress) != node.progress_presentation.is_some()
                    || node.progress_presentation.as_ref().is_some_and(|config| {
                        !config.is_valid() || node.progress.as_deref() != Some(&config.progress)
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Progress) != node.progress.is_some()
                    || node.progress.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_some()
                            || !node.text.is_empty()
                            || (!node.children.is_empty()
                                && node.progress_presentation.as_ref().is_none_or(|config| {
                                    config.shape
                                        != gpuio_protocol::progress_presentation::Shape::Circle
                                }))
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.palette_layout.as_ref().is_some_and(|layout| {
                    node.palette
                        .as_ref()
                        .is_none_or(|config| !layout.fits(config, node.palette_options.as_deref()))
                }) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.palette_options.as_ref().is_some_and(|options| {
                    node.palette
                        .as_ref()
                        .is_none_or(|config| !options.fits(config))
                }) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::CommandPalette) != node.palette.is_some()
                    || node.palette.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Menu) != node.menu.is_some()
                    || node.menu.as_ref().is_some_and(|menu| {
                        !menu.is_valid()
                            || (node.handler.is_some()
                                && !matches!(
                                    menu.presentation,
                                    MenuPresentation::Button
                                        | MenuPresentation::Context
                                        | MenuPresentation::PlatformContext
                                ))
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.children.len() < usize::from(menu.presentation.is_context())
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node
                    .menu
                    .as_ref()
                    .is_some_and(|menu| menu.presentation == MenuPresentation::EditorContext)
                {
                    let child = node
                        .children
                        .first()
                        .and_then(|id| plan.node(*id).ok())
                        .ok_or(ErrorCode::InvalidTree)?;
                    if !matches!(child.kind, Kind::Input | Kind::Textarea) {
                        return Err(ErrorCode::InvalidTree.into());
                    }
                }
                if (node.kind == Kind::CommandScope) != node.commands.is_some()
                    || (node.kind == Kind::CommandButton) != node.command_ref.is_some()
                    || node.commands.as_ref().is_some_and(|commands| {
                        !CommandConfig::registry_entries_are_valid(commands.iter().map(Arc::as_ref))
                            || node.handler.is_none()
                    })
                    || (node.kind == Kind::CommandButton
                        && (node.handler.is_some() || node.control.is_some()))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::FocusScope) != node.focus_scope.is_some() {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if matches!(node.kind, Kind::Tooltip | Kind::HoverCard) != node.tooltip.is_some()
                    || node
                        .tooltip
                        .as_ref()
                        .is_some_and(|config| !config.is_valid())
                    || (matches!(node.kind, Kind::Tooltip | Kind::HoverCard)
                        && node.children.len() != 2)
                    || (node.kind == Kind::HoverCard
                        && node
                            .tooltip
                            .as_ref()
                            .is_some_and(|config| !config.hoverable || config.skip_delay_ns != 0))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.placement.is_some()
                    && node.overlay.is_none()
                    && node.tooltip.is_none()
                    && !node
                        .menu
                        .as_ref()
                        .is_some_and(|menu| menu.presentation == MenuPresentation::Button)
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.sheet_insets.is_some()
                    && !node.overlay.as_ref().is_some_and(|config| {
                        matches!(
                            config.kind,
                            OverlayKind::SheetLeft
                                | OverlayKind::SheetRight
                                | OverlayKind::SheetTop
                                | OverlayKind::SheetBottom
                        )
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.placement_geometry.is_some()
                    && !node
                        .overlay
                        .as_ref()
                        .is_some_and(|config| config.kind == OverlayKind::Popover)
                    && node.tooltip.is_none()
                    && !node
                        .menu
                        .as_ref()
                        .is_some_and(|menu| menu.presentation == MenuPresentation::Button)
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.overlay_backdrop.is_some() || node.overlay_motion)
                    && node
                        .overlay
                        .as_ref()
                        .is_none_or(|config| !config.kind.is_modal())
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if let Some(config) = &node.overlay
                    && (node.kind != Kind::FocusScope
                        || !config.is_valid()
                        || node.handler.is_none()
                        || (config.kind.is_modal()
                            && !node.focus_scope.is_some_and(|scope| scope.trap))
                        || (config.kind == OverlayKind::Popover && Some(node.id) == plan.root))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.tree_moves && !node.tree_input {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.tree_input
                    && (node.kind != Kind::VirtualList
                        || node.handler.is_none()
                        || !node.accessibility.as_ref().is_some_and(|metadata| {
                            matches!(
                                metadata.role,
                                Some(gpuio_protocol::accessibility::Role::Tree(_))
                            )
                        }))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::CarouselTrack) != node.carousel_track.is_some()
                    || node.carousel_track.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || node.text.is_empty()
                            || node.text.len() > 4096
                            || node.text.contains('\0')
                            || node.children.len() != 1
                            || node.children.first().is_none_or(|child| {
                                plan.node(*child).map_or(true, |track| {
                                    track.kind != Kind::Container
                                        || !track.text.is_empty()
                                        || track.handler.is_some()
                                        || track.children.len() != config.carousel.ids.len()
                                        || track.children.iter().any(|item| {
                                            plan.node(*item)
                                                .map_or(true, |item| item.kind != Kind::Panel)
                                        })
                                })
                            })
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Carousel) != node.carousel.is_some()
                    || node.carousel.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || node.text.is_empty()
                            || node.text.len() > 4096
                            || node.text.contains('\0')
                            || !(1..=2).contains(&node.children.len())
                            || node.children.first().is_none_or(|child| {
                                plan.node(*child).map_or(true, |viewport| {
                                    viewport.kind != Kind::NavigationStack
                                        || viewport.children.len() != config.ids.len()
                                        || viewport.navigation_stack.is_none_or(|presentation| {
                                            presentation.selected != config.selected
                                        })
                                })
                            })
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::NavigationStack) != node.navigation_stack.is_some()
                    || node.navigation_stack.is_some_and(|config| {
                        !config.valid_children(node.children.len())
                            || node.handler.is_some()
                            || node.text.is_empty()
                            || node.text.len() > 4096
                            || node.text.contains('\0')
                            || node.children.iter().enumerate().any(|(index, child)| {
                                plan.node(*child).map_or(true, |child| {
                                    child.kind != Kind::Panel
                                        || (!config.retain
                                            && config.selected != Some(index as i64)
                                            && !child.children.is_empty())
                                })
                            })
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.kind == Kind::Panel
                    && (node.text.is_empty() || node.text.len() > 4096 || node.text.contains('\0'))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                match node.kind {
                    Kind::Input | Kind::Textarea | Kind::Combobox => {
                        let config = node.editor.as_ref().ok_or(ErrorCode::InvalidTree)?;
                        if node.handler.is_none()
                            || !config.is_valid()
                            || node.text.contains('\0')
                            || (matches!(node.kind, Kind::Input | Kind::Combobox)
                                && (config.min_rows != 1
                                    || config.max_rows != 1
                                    || node.text.contains(['\r', '\n'])))
                        {
                            return Err(ErrorCode::InvalidTree.into());
                        }
                    }
                    Kind::Container
                    | Kind::FocusScope
                    | Kind::Tooltip
                    | Kind::HoverCard
                    | Kind::Toast
                    | Kind::ToastStack
                    | Kind::PointerArea
                    | Kind::InputRegion
                    | Kind::HighlightScope
                    | Kind::DragSource
                    | Kind::DropTarget
                    | Kind::CommandScope
                    | Kind::CommandButton
                    | Kind::Menu
                    | Kind::CommandPalette
                    | Kind::Progress
                    | Kind::Loading
                    | Kind::Image
                    | Kind::Avatar
                    | Kind::Rating
                    | Kind::Slider
                    | Kind::NumberInput
                    | Kind::OtpInput
                    | Kind::Calendar
                    | Kind::ColorInput
                    | Kind::TabPanel
                    | Kind::Panel
                    | Kind::Disclosure
                    | Kind::Accordion
                    | Kind::NavigationStack
                    | Kind::Carousel
                    | Kind::CarouselTrack
                    | Kind::CarouselTrackGroup
                    | Kind::SplitPane
                    | Kind::SplitGroup
                    | Kind::Extension
                    | Kind::CanvasView
                    | Kind::ChartView
                    | Kind::DocumentView
                    | Kind::Icon
                    | Kind::Animated
                    | Kind::AnimationProgram
                    | Kind::ContainerQuery
                    | Kind::VirtualList
                    | Kind::Text
                    | Kind::Link
                    | Kind::Button => {
                        if node.editor.is_some() {
                            return Err(ErrorCode::InvalidTree.into());
                        }
                    }
                    Kind::Checkbox | Kind::Switch | Kind::Radio => {
                        let control = node.control.ok_or(ErrorCode::InvalidTree)?;
                        if node.editor.is_some()
                            || (!control.disabled()
                                && !matches!(control, Control::Radio(true, _, _))
                                && node.handler.is_none())
                            || node.text.contains('\0')
                        {
                            return Err(ErrorCode::InvalidTree.into());
                        }
                    }
                    Kind::ChoicePicker => {
                        let p = node.choice_picker.as_ref().ok_or(ErrorCode::InvalidTree)?;
                        if node.editor.is_some() || node.handler.is_none() || !node.text.is_empty()
                        {
                            return Err(ErrorCode::InvalidTree.into());
                        }
                        crate::choice_picker_admission::validate(p)?;
                    }
                    Kind::RadioGroup | Kind::TabBar | Kind::Select => {
                        let config = node.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
                        if (node.kind == Kind::Select
                            && !node.choice_menu
                            && !node.children.is_empty())
                            || !config.is_valid()
                            || node.editor.is_some()
                            || (!config.disabled && node.handler.is_none())
                        {
                            return Err(ErrorCode::InvalidTree.into());
                        }
                    }
                }
            }
        }
        let validated_nodes = if plan.structural {
            plan.validate_structure()?
        } else {
            0
        };
        // Structure validation establishes final parent links. Revalidate a
        // changed row's parent or a registered query owner, without walking
        // unrelated siblings or expanding the logical list order.
        let mut list_owners = BTreeSet::new();
        let mut list_query_owners = self.list_query_owners.clone();
        for (index, slot) in &plan.changes {
            let before = self.slots.get(*index).and_then(|slot| slot.node.as_ref());
            for node in slot.node.iter().chain(before) {
                list_owners.insert(node.id);
                list_owners.extend(node.parent);
                list_owners.extend(self.list_query_owners.get(&node.id).copied());
            }
            if let Some(query) = before
                .and_then(|node| node.list_input)
                .and_then(|config| config.query)
            {
                list_query_owners.remove(&query);
            }
        }
        // Remove every previous registration before adding new ones: an atomic
        // ownership transfer must not depend on node/operation order.
        for slot in plan.changes.values() {
            if let Some(node) = &slot.node
                && let Some(query) = node.list_input.and_then(|config| config.query)
                && list_query_owners.insert(query, node.id).is_some()
            {
                return Err(ErrorCode::InvalidTree.into());
            }
        }
        for owner in list_owners {
            if plan.node(owner).is_ok_and(|node| node.list_input.is_some()) {
                plan.node_mut(owner)?;
                plan.validate_list_input(plan.node(owner)?)?;
            }
        }
        let mut dirty = BTreeSet::new();
        for slot in plan.changes.values() {
            let mut id = slot.node.as_ref().map(|n| n.id);
            while let Some(current) = id {
                if !dirty.insert(current) {
                    break;
                }
                id = plan.node(current)?.parent;
            }
        }
        for id in &dirty {
            let node = plan.node(*id)?;
            if let Some(presentation) = &node.choice_picker {
                crate::choice_picker_admission::children(presentation, &node.children, |id| {
                    plan.node(id).ok()
                })?;
            }
            if plan.node(*id)?.split_button.is_some() {
                crate::split_button::owners(plan.node(*id)?, |id| plan.node(id).ok())?;
            }
            if node.popover {
                if node.kind != Kind::Container || !(1..=2).contains(&node.children.len()) {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if let Some(panel) = node.children.get(1)
                    && !plan
                        .node(*panel)?
                        .overlay
                        .as_ref()
                        .is_some_and(|c| c.kind == OverlayKind::Popover)
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
            }
            plan.validate_editor_frame(node)?;
            plan.validate_number_presentation(node)?;
            plan.validate_calendar_content(node)?;
            plan.validate_button_icons(*id)?;
            plan.validate_passive_content(*id)?;
            plan.validate_table(plan.node(*id)?)?;
        }
        if let Some(root) = plan.root {
            dirty.insert(root);
        }
        for (node, command) in plan.tables.clone() {
            plan.validate_table_command(node, &command)?;
        }
        let touched_records = plan.changes.len();
        for action in &plan.lists {
            plan.validate_list_action(action)?;
        }
        let retained: Vec<_> = pins
            .iter()
            .filter_map(|pin| {
                let node = plan.node(pin.node).ok()?;
                let index = node.list_index.as_ref()?;
                let mounted: BTreeSet<_> = node.list_rows.iter().map(|row| row.id).collect();
                let rows: Vec<_> = pin
                    .rows
                    .iter()
                    .copied()
                    .filter(|id| index.position(*id).is_some() && !mounted.contains(id))
                    .collect();
                (!rows.is_empty()).then_some(gpuio_protocol::list::Retained {
                    node: pin.node,
                    rows,
                })
            })
            .collect();
        if retained
            .iter()
            .map(|notice| notice.rows.len())
            .sum::<usize>()
            > gpuio_protocol::list::MAX_ACTIVE_ROWS
        {
            return Err(ErrorCode::LimitExceeded.into());
        }
        if !retained.is_empty() {
            return Err(ApplyFailure::Retained(retained));
        }
        let mut programs = Vec::new();
        for (index, slot) in &plan.changes {
            let before = self.slots.get(*index).and_then(|s| s.node.as_ref());
            let after = slot.node.as_ref();
            if before.and_then(|n| n.animation_program.as_ref())
                == after.and_then(|n| n.animation_program.as_ref())
                && before.map(|n| n.id) == after.map(|n| n.id)
            {
                continue;
            }
            if let Some(node) = before.filter(|n| n.animation_program.is_some()) {
                programs.push((node.id, None));
            }
            if let Some(node) = after
                && let Some(config) = &node.animation_program
            {
                programs.push((node.id, Some(config.clone())));
            }
        }
        let mut binding_owners = self.binding_owners.clone();
        for (index, slot) in &plan.changes {
            if let Some(before) = self.slots.get(*index).and_then(|s| s.node.as_ref()) {
                binding_owners.remove(&before.id);
            }
            if let Some(after) = slot
                .node
                .as_ref()
                .filter(|node| node.command_binding.is_some())
            {
                binding_owners.insert(after.id);
            }
        }
        if binding_owners.len() > gpuio_protocol::command_binding::MAX_OBSERVERS {
            return Err(ErrorCode::LimitExceeded.into());
        }
        let Plan {
            changes,
            root,
            slot_count,
            node_count,
            extension_count,
            canvas_count,
            chart_count,
            retained_bytes,
            lists,
            tables,
            ..
        } = plan;
        // All validation has succeeded. Reserve before mutating semantic state.
        self.slots
            .try_reserve(slot_count - self.slots.len())
            .map_err(|_| ErrorCode::LimitExceeded)?;
        admit(&programs)?;
        self.slots.resize_with(slot_count, Slot::default);
        for (index, slot) in changes {
            self.slots[index] = slot;
        }
        self.binding_owners = binding_owners;
        self.list_query_owners = list_query_owners;
        self.root = root;
        self.node_count = node_count;
        self.extension_count = extension_count;
        self.canvas_count = canvas_count;
        self.chart_count = chart_count;
        self.retained_bytes = retained_bytes;
        self.revision = tx.revision;
        Ok(Applied {
            revision: tx.revision,
            touched_records,
            validated_nodes,
            dirty: dirty.into_iter().collect(),
            lists,
            tables,
        })
    }
}

pub(crate) type ProgramChange = (
    NodeId,
    Option<Arc<gpuio_protocol::animation_program::Config>>,
);

/// Invalid transactions and stale row-eviction attempts have different retry semantics.
#[derive(Debug, PartialEq, Eq)]
pub enum ApplyFailure {
    Rejected(ErrorCode),
    Retained(Vec<gpuio_protocol::list::Retained>),
}
impl From<ErrorCode> for ApplyFailure {
    fn from(error: ErrorCode) -> Self {
        Self::Rejected(error)
    }
}
impl ApplyFailure {
    // Existing unguarded callers pass no pins, so Retained is unreachable there.
    pub(crate) fn rejection(self) -> ErrorCode {
        match self {
            Self::Rejected(error) => error,
            Self::Retained(_) => ErrorCode::InvalidTree,
        }
    }
}

struct Plan<'a> {
    original: &'a Tree,
    changes: BTreeMap<usize, Slot>,
    root: Option<NodeId>,
    slot_count: usize,
    node_count: usize,
    extension_count: usize,
    canvas_count: usize,
    chart_count: usize,
    retained_bytes: usize,
    budget: usize,
    structural: bool,
    lists: Vec<ListAction>,
    tables: Vec<(NodeId, gpuio_protocol::table::Command)>,
}

impl Plan<'_> {
    fn list_contains(index: &crate::list_index::Index, mut ids: impl Iterator<Item = i64>) -> bool {
        ids.all(|id| index.position(id).is_some())
    }

    fn validate_list(&self, node: &Node) -> Result<(), ErrorCode> {
        if node.kind != Kind::VirtualList {
            return if node.list_config.is_none()
                && node.list_order.is_none()
                && node.list_index.is_none()
                && node.list_rows.is_empty()
            {
                Ok(())
            } else {
                Err(ErrorCode::InvalidTree)
            };
        }
        let config = node.list_config.as_ref().ok_or(ErrorCode::InvalidTree)?;
        let order = node.list_order.as_ref().ok_or(ErrorCode::InvalidTree)?;
        let index = node.list_index.as_ref().ok_or(ErrorCode::InvalidTree)?;
        if (node.list_axis == gpuio_protocol::list::Axis::Horizontal
            && (node.tree_input || node.table.is_some()))
            || !config.is_valid()
            || !order.is_valid()
            || !node.text.is_empty()
            || (config.managed && node.list_rows.len() > config.max_active as usize)
            || (!config.managed && node.list_rows.len() != index.len())
            || (node.table.is_none() && node.list_rows.len() != node.children.len())
            || !Self::list_contains(index, node.list_rows.iter().map(|row| row.id))
        {
            return Err(ErrorCode::InvalidTree);
        }
        let mut ids = BTreeSet::new();
        let mut children = BTreeSet::new();
        for row in node.list_rows.iter() {
            if !ids.insert(row.id) || !children.insert(row.node) {
                return Err(ErrorCode::InvalidTree);
            }
        }
        if node.table.is_some() {
            let mut headers = 0;
            for child in node.children.iter() {
                if self.node(*child)?.table_header.is_some() {
                    headers += 1;
                    if !children.insert(*child)
                        || headers > gpuio_protocol::table_header::MAX_HEADERS
                    {
                        return Err(ErrorCode::InvalidTree);
                    }
                }
            }
        }
        if children != node.children.iter().copied().collect() {
            return Err(ErrorCode::InvalidTree);
        }
        Ok(())
    }

    fn validate_list_input(&self, node: &Node) -> Result<(), ErrorCode> {
        let Some(config) = node.list_input else {
            return Ok(());
        };
        use gpuio_protocol::accessibility::Role;
        if node.kind != Kind::VirtualList
            || node.handler.is_none()
            || node.tree_input
            || node.table.is_some()
            || !config.is_valid()
            || !node
                .accessibility
                .as_ref()
                .is_some_and(|metadata| matches!(metadata.role, Some(Role::ListBox(_))))
        {
            return Err(ErrorCode::InvalidTree);
        }
        if let Some(cursor) = config.cursor {
            if node
                .list_index
                .as_ref()
                .is_none_or(|index| index.position(cursor).is_none())
            {
                return Err(ErrorCode::InvalidTree);
            }
            if let Some(row) = node.list_rows.iter().find(|row| row.id == cursor) {
                let item = self.node(row.node)?;
                if !item.accessibility.as_ref().is_some_and(|metadata|
                    matches!(metadata.role, Some(Role::OptionItem(item)) if !item.disabled))
                { return Err(ErrorCode::InvalidTree); }
            }
        }
        if let Some(query) = config.query {
            let query = self.node(query)?;
            if query.kind != Kind::Input || node.parent.is_none() || query.parent != node.parent {
                return Err(ErrorCode::InvalidTree);
            }
        }
        Ok(())
    }

    fn validate_list_action(&self, action: &ListAction) -> Result<(), ErrorCode> {
        let id = match action {
            ListAction::Invalidate(id, _) | ListAction::Scroll(id, _) => *id,
        };
        let node = self.node(id)?;
        if node.kind != Kind::VirtualList {
            return Err(ErrorCode::InvalidTree);
        }
        let index = node.list_index.as_ref().ok_or(ErrorCode::InvalidTree)?;
        match action {
            ListAction::Invalidate(_, ids) => {
                if ids.len() > gpuio_protocol::list::MAX_LOGICAL_ROWS
                    || !Self::list_contains(index, ids.iter().copied())
                {
                    return Err(ErrorCode::InvalidTree);
                }
            }
            ListAction::Scroll(_, request) => {
                if node.table.is_some() {
                    return Err(ErrorCode::InvalidTree);
                }
                use gpuio_protocol::list::ScrollTarget;
                if request.serial < 1 {
                    return Err(ErrorCode::InvalidTree);
                }
                let row = match request.target {
                    ScrollTarget::Offset(row, offset) => {
                        if !offset.is_finite() || !(0.0..=1_000_000.0).contains(&offset) {
                            return Err(ErrorCode::InvalidTree);
                        }
                        Some(row)
                    }
                    ScrollTarget::Reveal(row) => Some(row),
                    ScrollTarget::FocusTreeRow(row) => {
                        if !node.tree_input {
                            return Err(ErrorCode::InvalidTree);
                        }
                        Some(row)
                    }
                    ScrollTarget::End => None,
                };
                if !Self::list_contains(index, row.into_iter()) {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        Ok(())
    }

    // The link root is the only activation/focus owner in composed content.
    // Run for dirty ancestors too: Bind/SetStyle can invalidate a descendant
    // without changing the structural edges.
    fn validate_passive_content(&self, id: NodeId) -> Result<(), ErrorCode> {
        let root = self.node(id)?;
        let rich_button = matches!(root.kind, Kind::Button | Kind::CommandButton)
            && root
                .button_presentation
                .is_some_and(|config| config.content == gpuio_protocol::button::Content::Rich);
        if !rich_button
            && root.menu.is_none()
            && root.palette.is_none()
            && !root.choice_menu
            && root.calendar_content.is_none()
            && root.number_presentation.is_none()
            && !matches!(
                root.kind,
                Kind::Link
                    | Kind::Avatar
                    | Kind::Checkbox
                    | Kind::Switch
                    | Kind::Radio
                    | Kind::RadioGroup
                    | Kind::TabBar
                    | Kind::SplitGroup
            )
        {
            return Ok(());
        }
        if matches!(root.kind, Kind::Checkbox | Kind::Switch | Kind::Radio)
            && !root.children.is_empty()
            && (root.children.len() != 1
                // Match Core.String.strip / Char.is_whitespace exactly. Rust's
                // Unicode trim rejects names admitted by the OCaml constructor.
                || root.text.bytes().all(|byte| matches!(byte, b'\t'..=b'\r' | b' '))
                || root.text.len() > 1024
                || root.text.contains('\0'))
        {
            return Err(ErrorCode::InvalidTree);
        }
        if rich_button
            && (root.children.len() != 1
                || (root.kind == Kind::Button
                    && (root.text.is_empty()
                        || root
                            .text
                            .bytes()
                            .all(|byte| matches!(byte, b'\t'..=b'\r' | b' '))
                        || root.text.len() > 1024
                        || root.text.contains('\0'))))
        {
            return Err(ErrorCode::InvalidTree);
        }
        let structural = |node: &Node| {
            node.kind == Kind::Container
                && node.text.is_empty()
                && node.style.is_empty()
                && node.handler.is_none()
                && node.hover_handler.is_none()
                && node.accessibility.is_none()
                && node.focus_scope.is_none()
                && node.highlight_scope.is_none()
                && node.command_binding.is_none()
                && node.drag_source.is_none()
                && node.drop_target.is_none()
                && node.pointer.is_none()
                && node.input_region.is_none()
                && node.window_region.is_none()
                && node.placement.is_none()
                && node.placement_geometry.is_none()
                && node.split_button.is_none()
                && node.split.is_none()
                && node.split_group.is_none()
                && !node.popover
                && node.table_cell.is_none()
                && node.reveal.is_none()
                && node.choice_appearance.is_none()
        };
        if let Some(palette) = &root.palette
            && !root.children.is_empty()
        {
            if root.children.len() != 3 + palette.commands.len() {
                return Err(ErrorCode::InvalidTree);
            }
            for id in root.children.iter() {
                let slot = self.node(*id)?;
                if !structural(slot) || slot.children.len() > 1 {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        let menu_children = if let Some(menu) = &root.menu {
            let slots = &root.children[usize::from(menu.presentation.is_context())..];
            if !slots.is_empty() {
                let items = menu.items_preorder();
                if menu.presentation == MenuPresentation::PlatformBar || slots.len() != items.len()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                for (slot, item) in slots.iter().zip(items) {
                    let slot = self.node(*slot)?;
                    if !structural(slot)
                        || slot.children.len() > 1
                        || (matches!(item, MenuItem::Separator) && !slot.children.is_empty())
                    {
                        return Err(ErrorCode::InvalidTree);
                    }
                    if menu.presentation == MenuPresentation::PlatformContext
                        && let Some(icon) = slot.children.first()
                    {
                        let icon = self.node(*icon)?;
                        if icon.kind != Kind::Icon
                            || !icon.children.is_empty()
                            || !icon.text.is_empty()
                            || !icon
                                .image
                                .as_ref()
                                .is_some_and(|image| image.label.is_none())
                        {
                            return Err(ErrorCode::InvalidTree);
                        }
                    }
                }
            }
            Some(slots)
        } else {
            None
        };
        if root.choice_menu && !root.children.is_empty() {
            let config = root.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
            if root.children.len() != config.items.len() {
                return Err(ErrorCode::InvalidTree);
            }
            for slot in root.children.iter() {
                let slot = self.node(*slot)?;
                if !structural(slot) || slot.children.len() > 1 {
                    return Err(ErrorCode::InvalidTree);
                }
                if let Some(icon) = slot.children.first() {
                    let icon = self.node(*icon)?;
                    if icon.kind != Kind::Icon
                        || !icon.children.is_empty()
                        || !icon.text.is_empty()
                        || !icon
                            .image
                            .as_ref()
                            .is_some_and(|image| image.label.is_none())
                    {
                        return Err(ErrorCode::InvalidTree);
                    }
                }
            }
        }
        let logical_children = if let Some(slots) = menu_children {
            slots
        } else if root.tab_trailing {
            let config = root.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
            if root.children.len() != config.items.len() + 1 {
                return Err(ErrorCode::InvalidTree);
            }
            let trailing = self.node(*root.children.last().ok_or(ErrorCode::InvalidTree)?)?;
            if !structural(trailing) || trailing.children.len() > 1 {
                return Err(ErrorCode::InvalidTree);
            }
            &root.children[..config.items.len()]
        } else {
            &root.children[..]
        };
        if root.tab_trailing || root.tab_content.is_some() || root.palette.is_some() {
            // Bound all parts, including interactive subtrees and wrappers.
            let mut all: Vec<_> = root.children.iter().map(|id| (*id, 1)).collect();
            let mut count = 0;
            while let Some((id, depth)) = all.pop() {
                count += 1;
                if count > 4096 || depth > 128 {
                    return Err(ErrorCode::LimitExceeded);
                }
                all.extend(self.node(id)?.children.iter().map(|id| (*id, depth + 1)));
            }
        }
        if root.tab_content.is_none()
            && matches!(root.kind, Kind::RadioGroup | Kind::TabBar)
            && !logical_children.is_empty()
        {
            let config = root.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
            if logical_children.len() != config.items.len() {
                return Err(ErrorCode::InvalidTree);
            }
            for slot in logical_children.iter() {
                let slot = self.node(*slot)?;
                if slot.kind != Kind::Container || !slot.text.is_empty() || slot.children.len() > 1
                {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        let skip = if root.palette.is_some() {
            3
        } else if root.number_presentation.is_some() {
            2
        } else {
            0
        };
        let mut pending: Vec<_> = if let Some(mount) = &root.split_group {
            if root.children.len() != mount.config.panels.len() {
                return Err(ErrorCode::InvalidTree);
            }
            let mut grips = Vec::new();
            for panel in root.children.iter() {
                let panel = self.node(*panel)?;
                if !structural(panel) || panel.children.len() != 2 {
                    return Err(ErrorCode::InvalidTree);
                }
                let content = self.node(panel.children[0])?;
                let grip = self.node(panel.children[1])?;
                if !structural(content)
                    || content.children.len() != 1
                    || !structural(grip)
                    || grip.children.len() > 1
                {
                    return Err(ErrorCode::InvalidTree);
                }
                grips.extend(grip.children.iter().map(|id| (*id, 3)));
            }
            grips
        } else if let Some(content) = &root.tab_content {
            let config = root.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
            if logical_children.len() != config.items.len()
                || content.labels.len() != config.items.len()
                || config.items.iter().any(|item| {
                    item.label
                        .bytes()
                        .all(|b| matches!(b, b'\t'..=b'\r' | b' '))
                })
            {
                return Err(ErrorCode::InvalidTree);
            }
            let mut labels = Vec::new();
            for (slot, mode) in logical_children.iter().zip(&content.labels) {
                let slot = self.node(*slot)?;
                if !structural(slot) || slot.children.len() != 3 {
                    return Err(ErrorCode::InvalidTree);
                }
                for (index, part) in slot.children.iter().enumerate() {
                    let part = self.node(*part)?;
                    if !structural(part) || part.children.len() > 1 {
                        return Err(ErrorCode::InvalidTree);
                    }
                    if index == 1 {
                        let custom = *mode == gpuio_protocol::tab_content::Label::Custom;
                        if part.children.len() != usize::from(custom) {
                            return Err(ErrorCode::InvalidTree);
                        }
                        labels.extend(part.children.iter().map(|id| (*id, 3)));
                    }
                }
            }
            labels
        } else {
            logical_children
                .iter()
                .skip(skip)
                .map(|id| (*id, 1))
                .collect()
        };
        let mut count = 0;
        while let Some((id, depth)) = pending.pop() {
            count += 1;
            if count > 4096 || depth > 128 {
                return Err(ErrorCode::LimitExceeded);
            }
            let node = self.node(id)?;
            if (!matches!(
                node.kind,
                Kind::Container
                    | Kind::Text
                    | Kind::Image
                    | Kind::Icon
                    | Kind::Avatar
                    | Kind::Loading
                    | Kind::Animated
                    | Kind::AnimationProgram
            ) && !((rich_button || root.calendar_content.is_some())
                && node.kind == Kind::Progress))
                || node.handler.is_some()
                || node.hover_handler.is_some()
                || node.window_region.is_some()
                || node.style.iter().any(|style| {
                    let fields: &[Field] = match style {
                        Style::Fields(fields) | Style::State(_, fields) => fields,
                        _ => &[],
                    };
                    fields.iter().any(|field| {
                        matches!(
                            field,
                            Field::UserSelect(true)
                                | Field::Inert(true)
                                | Field::Disabled(true)
                                | Field::OverflowX(3)
                                | Field::OverflowY(3)
                                | Field::PointerOcclusion(1 | 2)
                        )
                    })
                })
            {
                return Err(ErrorCode::InvalidTree);
            }
            pending.extend(node.children.iter().map(|id| (*id, depth + 1)));
        }
        Ok(())
    }

    // Buttons retain one action/focus target. Their optional children represent
    // two fixed decorative icon slots, never nested controls or callbacks.
    // Run for dirty ancestors too: Bind/SetImage can invalidate a slot without
    // changing the structural edges.
    fn validate_button_icons(&self, id: NodeId) -> Result<(), ErrorCode> {
        let node = self.node(id)?;
        if !matches!(node.kind, Kind::Button | Kind::CommandButton)
            || node.children.is_empty()
            || node
                .button_presentation
                .is_some_and(|config| config.content == gpuio_protocol::button::Content::Rich)
        {
            return Ok(());
        }
        if node.children.len() != 2 {
            return Err(ErrorCode::InvalidTree);
        }
        for slot in node.children.iter() {
            let slot = self.node(*slot)?;
            if slot.kind != Kind::Container
                || slot.handler.is_some()
                || slot.window_region.is_some()
                || !slot.text.is_empty()
                || slot.children.len() > 1
            {
                return Err(ErrorCode::InvalidTree);
            }
            if let Some(icon) = slot.children.first() {
                let icon = self.node(*icon)?;
                if icon.kind != Kind::Icon
                    || icon.handler.is_some()
                    || !icon
                        .image
                        .as_ref()
                        .is_some_and(|image| image.label.is_none())
                {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        Ok(())
    }
    fn slot(&self, index: usize) -> Option<&Slot> {
        self.changes
            .get(&index)
            .or_else(|| self.original.slots.get(index))
    }

    fn node(&self, id: NodeId) -> Result<&Node, ErrorCode> {
        self.slot(id.slot())
            .filter(|s| s.generation == id.generation())
            .and_then(|s| s.node.as_ref())
            .ok_or(ErrorCode::StaleHandle)
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, ErrorCode> {
        self.node(id)?;
        let slot = self.slot(id.slot()).expect("validated slot").clone();
        Ok(self
            .changes
            .entry(id.slot())
            .or_insert(slot)
            .node
            .as_mut()
            .expect("validated node"))
    }

    fn operation(&mut self, op: &Op) -> Result<(), ErrorCode> {
        if matches!(op, Op::InvalidateListRows(..) | Op::ScrollList(..)) {
            return self.operation_inner(op);
        }
        let target = match op {
            Op::Create(id, ..)
            | Op::CreateTableText(id, ..)
            | Op::SetTableText(id, ..)
            | Op::Remove(id)
            | Op::SetText(id, ..)
            | Op::SetStyledText(id, ..)
            | Op::SetTextShimmer(id, ..)
            | Op::SetLink(id, ..)
            | Op::SetStyle(id, ..)
            | Op::SetEditor(id, ..)
            | Op::SetEditorPrivacy(id, ..)
            | Op::SetEditorFrame(id, ..)
            | Op::SetEditorContentHint(id, ..)
            | Op::SetEditorFormat(id, ..)
            | Op::SetEditorValidation(id, ..)
            | Op::SetTextAreaLayout(id, ..)
            | Op::SetEditorClearOnEscape(id, ..)
            | Op::SetEditorSearchable(id, ..)
            | Op::SetNumberStepMode(id, ..)
            | Op::SetNumberPresentation(id, ..)
            | Op::SetReveal(id, ..)
            | Op::SetCalendarAppearance(id, ..)
            | Op::SetCalendarContent(id, ..)
            | Op::SetColorPresentation(id, ..)
            | Op::SetSliderAppearance(id, ..)
            | Op::SetOtpAppearance(id, ..)
            | Op::SetControl(id, ..)
            | Op::SetChoice(id, ..)
            | Op::SetFocusScope(id, ..)
            | Op::SetOverlayBackdrop(id, ..)
            | Op::SetOverlayMotion(id, ..)
            | Op::SetTooltipMotion(id, ..)
            | Op::SetPlacementGeometry(id, ..)
            | Op::SetWindowRegion(id, ..)
            | Op::SetSheetInsets(id, ..)
            | Op::SetOverlay(id, ..)
            | Op::SetPlacement(id, ..)
            | Op::SetTooltip(id, ..)
            | Op::SetCommands(id, ..)
            | Op::SetCommandRef(id, ..)
            | Op::SetMenu(id, ..)
            | Op::SetPaletteObserved(id, ..)
            | Op::SetPalette(id, ..)
            | Op::SetPaletteOptions(id, ..)
            | Op::SetPaletteLayout(id, ..)
            | Op::SetAnimation(id, ..)
            | Op::SetAnimationProgram(id, ..)
            | Op::SetNavigationStack(id, ..)
            | Op::SetTreeInput(id, ..)
            | Op::SetListInput(id, ..)
            | Op::SetTableBehavior(id, ..)
            | Op::SetTableAppearance(id, ..)
            | Op::SetTable(id, ..)
            | Op::SetTableCell(id, ..)
            | Op::SetTableHeader(id, ..)
            | Op::SetTableHeaderStyle(id, ..)
            | Op::SetTableRowStyle(id, ..)
            | Op::TableCommand(id, ..)
            | Op::SetTreeMoves(id, ..)
            | Op::SetCarousel(id, ..)
            | Op::SetCarouselTrack(id, ..)
            | Op::SetTabAppearance(id, ..)
            | Op::SetTabContent(id, ..)
            | Op::SetTabViewport(id, ..)
            | Op::SetTabMotion(id, ..)
            | Op::SetTabTrailing(id, ..)
            | Op::SetChoiceMenu(id, ..)
            | Op::SetCarouselTrackMotion(id, ..)
            | Op::SetContainerQuery(id, ..)
            | Op::SetAccessibility(id, ..)
            | Op::SetLoading(id, ..)
            | Op::SetProgressPresentation(id, ..)
            | Op::SetSpinner(id, ..)
            | Op::SetAvatar(id, ..)
            | Op::SetRating(id, ..)
            | Op::SetRatingAppearance(id, ..)
            | Op::SetTabOrder(id, ..)
            | Op::SetCalendarViewportObserver(id, ..)
            | Op::SetHoverObserver(id, ..)
            | Op::SetChoicePicker(id, ..)
            | Op::SetSplitButton(id, ..)
            | Op::SetButtonPresentation(id, ..)
            | Op::SetControlAppearance(id, ..)
            | Op::SetSlider(id, ..)
            | Op::SetNumberInput(id, ..)
            | Op::SetOtpInput(id, ..)
            | Op::SetColorInput(id, ..)
            | Op::SetCalendar(id, ..)
            | Op::SetListAxis(id, ..)
            | Op::SetListConfig(id, ..)
            | Op::SetListOrder(id, ..)
            | Op::SetListRows(id, ..)
            | Op::InvalidateListRows(id, ..)
            | Op::ScrollList(id, ..)
            | Op::SetImage(id, ..)
            | Op::SetCanvas(id, ..)
            | Op::SetChart(id, ..)
            | Op::SetDocument(id, ..)
            | Op::SetDocumentDiff(id, ..)
            | Op::SetDocumentSelectionFormat(id, ..)
            | Op::SetDocumentPreview(id, ..)
            | Op::SetDocumentProfile(id, ..)
            | Op::SetDocumentActions(id, ..)
            | Op::SetDocumentMarkdownOptions(id, ..)
            | Op::SetDocumentTextStyle(id, ..)
            | Op::SetExtension(id, ..)
            | Op::SetSplit(id, ..)
            | Op::SetSplitGroup(id, ..)
            | Op::SetProgress(id, ..)
            | Op::SetToast(id, ..)
            | Op::SetToastPlacement(id, ..)
            | Op::SetToastLayering(id, ..)
            | Op::SetToastMotion(id, ..)
            | Op::SetScrollbar(id, ..)
            | Op::SetToastStack(id, ..)
            | Op::SetDragSource(id, ..)
            | Op::SetDropTarget(id, ..)
            | Op::SetPointer(id, ..)
            | Op::SetInputRegion(id, ..)
            | Op::SetHighlightScope(id, ..)
            | Op::SetCommandBinding(id, ..)
            | Op::SetNumberInputDraft(id, ..)
            | Op::SetComboboxFilter(id, ..)
            | Op::SetChoiceAppearance(id, ..)
            | Op::Bind(id, ..)
            | Op::Splice(id, ..) => Some(*id),
            Op::SetRoot(_) => None,
            Op::SetPopover(id, _) => Some(*id),
        };
        let bytes = |plan: &Self| {
            target
                .and_then(|id| plan.slot(id.slot()))
                .and_then(|slot| slot.node.as_ref())
                .map_or(0, Node::payload_bytes)
        };
        let before = bytes(self);
        self.operation_inner(op)?;
        self.retained_bytes = self
            .retained_bytes
            .checked_sub(before)
            .and_then(|remaining| remaining.checked_add(bytes(self)))
            .filter(|total| *total <= self.budget)
            .ok_or(ErrorCode::LimitExceeded)?;
        Ok(())
    }

    fn operation_inner(&mut self, op: &Op) -> Result<(), ErrorCode> {
        match op {
            Op::CreateTableText(id, cell) => {
                if !cell.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                // A bounded single-cell operation, not an unbounded nested batch.
                // Reuse ordinary allocation/generation checks; the outer operation
                // accounts for both display and copy payloads before publication.
                self.operation_inner(&Op::Create(*id, Kind::Text, cell.copy_text.clone(), None))?;
                self.node_mut(*id)?.table_cell = Some(Arc::new(cell.clone()));
            }
            Op::SetTableText(id, cell) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Text
                    || !cell.is_valid()
                    || node
                        .table_cell
                        .as_ref()
                        .is_none_or(|old| old.column != cell.column)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.text = Arc::from(cell.copy_text.as_str());
                node.table_cell = Some(Arc::new(cell.clone()));
            }
            Op::Create(id, kind, text, handler) => {
                validate_text(text)?;
                if id.slot() > self.slot_count || id.slot() >= MAX_NODES {
                    return Err(ErrorCode::LimitExceeded);
                }
                let old_generation = if id.slot() == self.slot_count {
                    0
                } else {
                    let old = self.slot(id.slot()).ok_or(ErrorCode::StaleHandle)?;
                    if old.node.is_some() {
                        return Err(ErrorCode::InvalidTree);
                    }
                    old.generation
                };
                if old_generation.checked_add(1) != Some(id.generation()) {
                    return Err(ErrorCode::StaleHandle);
                }
                if id.slot() == self.slot_count {
                    self.slot_count += 1;
                }
                if *kind == Kind::CanvasView {
                    if self.canvas_count == 128 {
                        return Err(ErrorCode::LimitExceeded);
                    }
                    self.canvas_count += 1;
                }
                if *kind == Kind::ChartView {
                    if self.chart_count == 128 {
                        return Err(ErrorCode::LimitExceeded);
                    }
                    self.chart_count += 1;
                }
                if *kind == Kind::Extension {
                    if self.extension_count == 256 {
                        return Err(ErrorCode::LimitExceeded);
                    }
                    self.extension_count += 1;
                }
                self.node_count += 1;
                self.changes.insert(
                    id.slot(),
                    Slot {
                        generation: id.generation(),
                        node: Some(Node {
                            id: *id,
                            kind: *kind,
                            text: Arc::from(text.as_str()),
                            text_spans: Arc::from([]),
                            text_shimmer: None,
                            editor: None,
                            editor_privacy: EditorPrivacy::Plain,
                            editor_content_hint: None,
                            editor_format: None,
                            textarea_layout: None,
                            editor_clear_on_escape: false,
                            editor_searchable: false,
                            editor_validation: None,
                            editor_frame: None,
                            editor_frame_activation_revision: 0,
                            control: None,
                            link: None,
                            choice: None,
                            choice_picker: None,
                            choice_appearance: None,
                            combobox_filter: None,
                            focus_scope: None,
                            overlay_backdrop: None,
                            overlay_motion: false,
                            tooltip_motion: false,
                            window_region: None,
                            placement_geometry: None,
                            sheet_insets: None,
                            overlay: None,
                            tooltip: None,
                            commands: None,
                            command_ref: None,
                            menu: None,
                            palette: None,
                            palette_observed: false,
                            palette_options: None,
                            palette_layout: None,
                            progress: None,
                            progress_presentation: None,
                            loading: None,
                            spinner: None,
                            image: None,
                            avatar: None,
                            rating: None,
                            rating_appearance: None,
                            tab_order: None,
                            split_button: None,
                            button_presentation: None,
                            button_activation_revision: 0,
                            tab_appearance: None,
                            tab_content: None,
                            tab_viewport: None,
                            tab_motion: None,
                            tab_trailing: false,
                            choice_menu: false,
                            control_appearance: None,
                            slider: None,
                            number_input: None,
                            number_step_mode: Default::default(),
                            number_presentation: None,
                            otp_input: None,
                            popover: false,
                            reveal: None,
                            slider_appearance: None,
                            calendar_appearance: None,
                            calendar_content: None,
                            color_presentation: None,
                            otp_appearance: None,
                            calendar: None,
                            color_input: None,
                            extension: None,
                            extension_command: None,
                            split: None,
                            split_group: None,
                            document: None,
                            document_diff: None,
                            document_diff_epoch: 0,
                            document_selection_markdown: false,
                            document_text_style: None,
                            document_markdown_options: Default::default(),
                            document_profile: None,
                            document_actions: None,
                            document_preview: None,
                            canvas: None,
                            chart: None,
                            animation: None,
                            animation_program: None,
                            navigation_stack: None,
                            carousel: None,
                            carousel_track: None,
                            carousel_track_motion: None,
                            table: None,
                            table_cell: None,
                            table_header: None,
                            table_header_style: Arc::from([]),
                            table_row_style: Arc::from([]),
                            table_serial: 0,
                            tree_input: false,
                            list_input: None,
                            table_behavior: None,
                            table_appearance: None,
                            list_input_generation: 0,
                            tree_moves: false,
                            container_query: None,
                            accessibility: None,
                            list_axis: Default::default(),
                            list_config: None,
                            list_order: None,
                            list_index: None,
                            list_rows: Arc::from([]),
                            toast: None,
                            toast_stack: None,
                            toast_placement: None,
                            toast_layering: None,
                            toast_motion: None,
                            scrollbar: None,
                            drag_source: None,
                            drop_target: None,
                            pointer: None,
                            input_region: None,
                            highlight_scope: None,
                            command_binding: None,
                            placement: None,
                            style: Arc::from([]),
                            handler: *handler,
                            hover_handler: None,
                            calendar_viewport_handler: None,
                            children: Arc::from([]),
                            parent: None,
                        }),
                    },
                );
                self.structural = true;
            }
            Op::Remove(id) => {
                if self.node(*id)?.kind == Kind::CanvasView {
                    self.canvas_count -= 1;
                }
                if self.node(*id)?.kind == Kind::ChartView {
                    self.chart_count -= 1;
                }
                if self.node(*id)?.kind == Kind::Extension {
                    self.extension_count -= 1;
                }
                self.changes.insert(
                    id.slot(),
                    Slot {
                        generation: id.generation(),
                        node: None,
                    },
                );
                self.node_count -= 1;
                self.structural = true;
            }
            Op::SetText(id, text) => {
                if matches!(
                    self.node(*id)?.kind,
                    Kind::Input | Kind::Textarea | Kind::Combobox
                ) {
                    // Native editor contents change only through explicit commands.
                    return Err(ErrorCode::InvalidTree);
                }
                validate_text(text)?;
                let node = self.node_mut(*id)?;
                node.text = Arc::from(text.as_str());
                node.text_spans = Arc::from([]);
            }
            Op::SetStyledText(id, content) => {
                if self.node(*id)?.kind != Kind::Text || !content.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.text = Arc::from(content.text.as_str());
                node.text_spans = Arc::from(content.spans.as_slice());
            }
            Op::SetTextShimmer(id, config) => {
                if self.node(*id)?.kind != Kind::Text
                    || config.is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.text_shimmer = config.map(Arc::new);
            }
            Op::SetLink(id, config) => {
                if self.node(*id)?.kind != Kind::Link || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let changed = self.node(*id)?.link.as_ref().is_some_and(|previous| {
                    previous.loading != config.loading || previous.disabled != config.disabled
                });
                let activation_revision = self.original.revision + 1;
                let node = self.node_mut(*id)?;
                if changed {
                    node.button_activation_revision = activation_revision;
                }
                node.link = Some(Arc::new(config.clone()));
            }
            Op::SetEditorSearchable(id, enabled) => {
                if self.node(*id)?.kind != Kind::Textarea {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.editor_searchable = *enabled;
            }
            Op::SetEditorClearOnEscape(id, enabled) => {
                if !matches!(self.node(*id)?.kind, Kind::Input | Kind::Textarea) {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.editor_clear_on_escape = *enabled;
            }
            Op::SetTextAreaLayout(id, config) => {
                if self.node(*id)?.kind != Kind::Textarea
                    || config.as_ref().is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.textarea_layout = *config;
            }
            Op::SetEditorValidation(id, rule) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Input {
                    return Err(ErrorCode::InvalidTree);
                }
                if node.editor_validation.as_ref().map(|policy| policy.rule()) != rule.as_ref() {
                    let policy = rule
                        .clone()
                        .map(crate::input_validation::Policy::new)
                        .transpose()
                        .map_err(|error| match error {
                            gpuio_protocol::input_validation::Error::TooComplex => {
                                ErrorCode::LimitExceeded
                            }
                            _ => ErrorCode::InvalidTree,
                        })?
                        .map(Arc::new);
                    self.node_mut(*id)?.editor_validation = policy;
                }
            }
            Op::SetEditorFormat(id, config) => {
                if self.node(*id)?.kind != Kind::Input
                    || config.as_ref().is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.editor_format = config.clone().map(Arc::new);
            }
            Op::SetEditorContentHint(id, hint) => {
                if !matches!(self.node(*id)?.kind, Kind::Input | Kind::Textarea) {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.editor_content_hint = *hint;
            }
            Op::SetEditorFrame(id, config) => {
                if !matches!(self.node(*id)?.kind, Kind::Input | Kind::Textarea)
                    || config.as_ref().is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let changed = self.node(*id)?.editor_frame.as_deref() != config.as_ref();
                let revision = self.original.revision + 1;
                let node = self.node_mut(*id)?;
                if changed {
                    node.editor_frame_activation_revision = revision;
                }
                node.editor_frame = config.clone().map(Arc::new);
            }
            Op::SetEditorPrivacy(id, privacy) => {
                if self.node(*id)?.kind != Kind::Input {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.editor_privacy = *privacy;
            }
            Op::SetEditor(id, config) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::Input | Kind::Textarea | Kind::Combobox
                ) || !config.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let changed = self.node(*id)?.editor.as_ref().is_some_and(|old| {
                    old.read_only != config.read_only || old.disabled != config.disabled
                });
                let revision = self.original.revision + 1;
                let node = self.node_mut(*id)?;
                if changed {
                    node.editor_frame_activation_revision = revision;
                }
                node.editor = Some(Arc::new(config.clone()));
            }
            Op::SetToast(id, config) => {
                if self.node(*id)?.kind != Kind::Toast || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.toast = Some(Arc::new(config.clone()));
            }
            Op::SetDragSource(id, config) => {
                if self.node(*id)?.kind != Kind::DragSource {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.drag_source = Some(Arc::new(config.clone()));
            }
            Op::SetDropTarget(id, config) => {
                if self.node(*id)?.kind != Kind::DropTarget {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.drop_target = Some(Arc::new(config.clone()));
            }
            Op::SetCommandBinding(id, config) => {
                if self.node(*id)?.kind != Kind::Container
                    || config.as_ref().is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.command_binding = config.clone().map(Arc::new);
            }
            Op::SetHighlightScope(id, config) => {
                if self.node(*id)?.kind != Kind::HighlightScope || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.highlight_scope = Some(Arc::new(config.clone()));
            }
            Op::SetInputRegion(id, config) => {
                if self.node(*id)?.kind != Kind::InputRegion || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.input_region = Some(Arc::new(config.clone()));
            }
            Op::SetPointer(id, config) => {
                if self.node(*id)?.kind != Kind::PointerArea || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.pointer = Some(Arc::new(config.clone()));
            }
            Op::SetScrollbar(id, config) => {
                if !matches!(self.node(*id)?.kind, Kind::Container | Kind::VirtualList)
                    || config.as_ref().is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.scrollbar = config.clone().map(Arc::from);
            }
            Op::SetToastMotion(id, config) => {
                if self.node(*id)?.kind != Kind::ToastStack || config.is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.toast_motion = *config;
            }
            Op::SetToastLayering(id, config) => {
                if self.node(*id)?.kind != Kind::ToastStack || config.is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.toast_layering = *config;
            }
            Op::SetToastPlacement(id, placement) => {
                if self.node_mut(*id)?.kind != Kind::ToastStack
                    || placement.is_some_and(|p| !p.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.toast_placement = *placement;
            }
            Op::SetToastStack(id, config) => {
                if self.node(*id)?.kind != Kind::ToastStack || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.toast_stack = Some(Arc::new(config.clone()));
            }
            Op::SetTable(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::VirtualList
                    || !config.is_valid()
                    || node.table.as_ref().is_some_and(|old| {
                        config.schema_revision < old.schema_revision
                            || config.query_generation < old.query_generation
                            || ((config.schema != old.schema || config.sort != old.sort)
                                && config.schema_revision <= old.schema_revision)
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.table = Some(Arc::new(config.clone()));
            }
            Op::SetTableHeaderStyle(id, styles) | Op::SetTableRowStyle(id, styles) => {
                let row = matches!(op, Op::SetTableRowStyle(..));
                let expected = if row {
                    Kind::Container
                } else {
                    Kind::VirtualList
                };
                if self.node(*id)?.kind != expected
                    || !gpuio_protocol::table_presentation::valid_scope(styles, row)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                validate_style(styles)?;
                if row {
                    self.node_mut(*id)?.table_row_style = Arc::from(styles.clone());
                } else {
                    self.node_mut(*id)?.table_header_style = Arc::from(styles.clone());
                }
            }
            Op::SetTableHeader(id, target) => {
                if self.node(*id)?.kind != Kind::Container
                    || target.as_ref().is_some_and(|target| !target.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.table_header = target.clone().map(Arc::new);
            }
            Op::SetTableCell(id, cell) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Container
                    || !cell.is_valid()
                    || node
                        .table_cell
                        .as_ref()
                        .is_some_and(|old| old.column != cell.column)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.table_cell = Some(Arc::new(cell.clone()));
            }
            Op::SetTableAppearance(id, appearance) => {
                if self.node(*id)?.kind != Kind::VirtualList
                    || appearance.as_ref().is_some_and(|a| !a.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.table_appearance = appearance.clone().map(Arc::new);
            }
            Op::SetTableBehavior(id, behavior) => {
                if self.node(*id)?.kind != Kind::VirtualList
                    || behavior.as_ref().is_some_and(|b| !b.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.table_behavior = behavior.clone().map(Arc::new);
            }
            Op::TableCommand(id, command) => {
                self.tables.push((*id, command.clone()));
            }
            Op::SetListInput(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::VirtualList
                    || config.is_some_and(|config| {
                        !config.is_valid()
                            || match node.list_input {
                                Some(old) => !config.can_replace(old),
                                None => config.generation <= node.list_input_generation,
                            }
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                if let Some(config) = config {
                    node.list_input_generation = config.generation;
                }
                node.list_input = *config;
            }
            Op::SetTreeInput(id, enabled) => {
                if self.node(*id)?.kind != Kind::VirtualList {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tree_input = *enabled;
            }
            Op::SetTreeMoves(id, enabled) => {
                if self.node(*id)?.kind != Kind::VirtualList {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tree_moves = *enabled;
            }
            Op::SetCarouselTrackMotion(id, motion) => {
                if self.node(*id)?.kind != Kind::CarouselTrack
                    || motion.as_ref().is_some_and(|motion| !motion.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.carousel_track_motion = motion.clone();
            }
            Op::SetCarouselTrack(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::CarouselTrack
                    || !config.is_valid()
                    || node
                        .carousel_track
                        .as_ref()
                        .is_some_and(|old| !config.can_replace(old))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.carousel_track = Some(Arc::new(config.clone()));
            }
            Op::SetCarousel(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Carousel
                    || !config.is_valid()
                    || node
                        .carousel
                        .as_ref()
                        .is_some_and(|old| !config.can_replace(old))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.carousel = Some(Arc::new(config.clone()));
            }
            Op::SetNavigationStack(id, config) => {
                if self.node(*id)?.kind != Kind::NavigationStack || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.navigation_stack = Some(*config);
            }
            Op::SetContainerQuery(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::ContainerQuery
                    || !config.is_valid()
                    || node.container_query.as_ref().is_some_and(|old| {
                        config != old.as_ref() && config.generation <= old.generation
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.container_query = Some(Arc::new(config.clone()));
            }
            Op::SetAccessibility(id, config) => {
                let kind = self.node(*id)?.kind;
                if config
                    .as_ref()
                    .is_some_and(|c| !c.is_valid() || !c.supports(kind))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.accessibility = config.clone().map(Arc::new);
            }
            Op::SetAnimationProgram(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::AnimationProgram
                    || !config.is_valid()
                    || node.animation_program.as_ref().is_some_and(|old| {
                        (config != old.as_ref() && config.generation <= old.generation)
                            || (config.program == old.program && config.restart < old.restart)
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.animation_program = Some(Arc::new(config.clone()));
            }
            Op::SetAnimation(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Animated
                    || !config.is_valid()
                    || node.animation.as_ref().is_some_and(|old| {
                        config != old.as_ref() && config.generation <= old.generation
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.animation = Some(Arc::new(config.clone()));
                self.structural = true;
            }
            Op::SetListAxis(id, axis) => {
                if self.node(*id)?.kind != Kind::VirtualList {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.list_axis = *axis;
            }
            Op::SetListConfig(id, config) => {
                if self.node(*id)?.kind != Kind::VirtualList || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.list_config = Some(Arc::new(config.clone()));
            }
            Op::SetListOrder(id, order) => {
                let node = self.node(*id)?;
                if node.kind != Kind::VirtualList
                    || !order.is_valid()
                    || node
                        .list_order
                        .as_ref()
                        .is_some_and(|old| order != old.as_ref() && order.revision <= old.revision)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if node.list_order.as_deref() == Some(order) {
                    return Ok(());
                }
                let old_bytes = node.list_order.as_ref().map_or(0, |old| {
                    old.runs.iter().map(|run| run.count as usize).sum::<usize>() * 192
                        + std::mem::size_of_val(old.runs.as_slice())
                });
                let new_bytes = order
                    .runs
                    .iter()
                    .map(|run| run.count as usize)
                    .sum::<usize>()
                    * 192
                    + std::mem::size_of_val(order.runs.as_slice());
                if self.retained_bytes - old_bytes + new_bytes > self.budget {
                    return Err(ErrorCode::LimitExceeded);
                }
                let index =
                    crate::list_index::Index::new(order).map_err(|_| ErrorCode::InvalidTree)?;
                let node = self.node_mut(*id)?;
                node.list_order = Some(Arc::new(order.clone()));
                node.list_index = Some(Arc::new(index));
            }
            Op::SetListRows(id, rows) => {
                if self.node(*id)?.kind != Kind::VirtualList || rows.len() > MAX_NODES {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.list_rows = Arc::from(rows.clone());
            }
            Op::InvalidateListRows(id, rows) => {
                self.lists.push(ListAction::Invalidate(*id, rows.clone()));
            }
            Op::ScrollList(id, request) => {
                self.lists.push(ListAction::Scroll(*id, *request));
            }
            Op::SetSlider(id, config, initial) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Slider
                    || !config.is_valid()
                    || !initial.is_valid()
                    || node
                        .slider
                        .as_ref()
                        .is_some_and(|old| !old.initial.same_mode(*initial))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.slider = Some(SliderMount {
                    config: Arc::new(config.clone()),
                    initial: *initial,
                });
            }
            Op::SetColorInput(id, config, initial) => {
                let node = self.node(*id)?;
                if node.kind != Kind::ColorInput
                    || !config.is_valid()
                    || (node.color_input.is_none() && !config.allows(*initial))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let initial = node
                    .color_input
                    .as_ref()
                    .map_or(*initial, |old| old.initial);
                self.node_mut(*id)?.color_input = Some(ColorInputMount {
                    config: Arc::new(config.as_ref().clone()),
                    initial,
                });
            }
            Op::SetCalendar(id, config, initial, initial_month) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Calendar
                    || !config.is_valid()
                    || !initial.fits(config.mode)
                    || node
                        .calendar
                        .as_ref()
                        .is_some_and(|old| old.config.mode != config.mode)
                    || (node.calendar.is_none()
                        && !config.constraints.allows_selection(*initial, config.mode))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let (initial, initial_month) = node
                    .calendar
                    .as_ref()
                    .map_or((*initial, *initial_month), |old| {
                        (old.initial, old.initial_month)
                    });
                self.node_mut(*id)?.calendar = Some(CalendarMount {
                    config: Arc::new(config.as_ref().clone()),
                    initial,
                    initial_month,
                });
            }
            Op::SetNumberStepMode(id, mode) => {
                if self.node(*id)?.kind != Kind::NumberInput {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.number_step_mode = *mode;
            }
            Op::SetNumberPresentation(id, config) => {
                if self.node(*id)?.kind != Kind::NumberInput {
                    return Err(ErrorCode::InvalidTree);
                }
                if let Some(config) = config {
                    crate::number_presentation::validate(config)?;
                }
                self.node_mut(*id)?.number_presentation = config.clone().map(Arc::new);
            }
            Op::SetPopover(id, enabled) => {
                if self.node(*id)?.kind != Kind::Container {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.popover = *enabled;
            }
            Op::SetReveal(id, config) => {
                if self.node(*id)?.kind != Kind::Panel || config.is_some_and(|c| !c.is_valid()) {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.reveal = *config;
            }
            Op::SetCalendarAppearance(id, appearance) => {
                if self.node(*id)?.kind != Kind::Calendar
                    || appearance.as_ref().is_some_and(|a| !a.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.calendar_appearance = appearance.clone().map(Arc::new);
            }
            Op::SetCalendarContent(id, content) => {
                if self.node(*id)?.kind != Kind::Calendar
                    || content.as_ref().is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.calendar_content = content.clone().map(Arc::new);
            }
            Op::SetColorPresentation(id, presentation) => {
                if self.node(*id)?.kind != Kind::ColorInput
                    || presentation.as_ref().is_some_and(|p| !p.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.color_presentation = presentation.clone().map(Arc::new);
            }
            Op::SetSliderAppearance(id, appearance) => {
                if self.node(*id)?.kind != Kind::Slider
                    || appearance.as_ref().is_some_and(|a| !a.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.slider_appearance = appearance.clone().map(Arc::new);
            }
            Op::SetOtpAppearance(id, appearance) => {
                if self.node(*id)?.kind != Kind::OtpInput
                    || appearance.as_ref().is_some_and(|a| !a.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.otp_appearance = appearance.clone().map(Arc::new);
            }
            Op::SetOtpInput(id, config, initial) => {
                let node = self.node(*id)?;
                if node.kind != Kind::OtpInput
                    || !config.is_valid()
                    || !config.policy.canonical(initial)
                    || node
                        .otp_input
                        .as_ref()
                        .is_some_and(|old| old.config.policy != config.policy)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                // Rerendered seeds never replace an existing placement's original seed.
                let initial = node
                    .otp_input
                    .as_ref()
                    .map_or_else(|| Arc::from(initial.as_str()), |old| old.initial.clone());
                self.node_mut(*id)?.otp_input = Some(OtpInputMount {
                    config: Arc::new(config.clone()),
                    initial,
                });
            }
            Op::SetNumberInput(id, config, initial) => {
                if self.node(*id)?.kind != Kind::NumberInput
                    || !config.is_valid()
                    || !initial.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let initial_draft = self
                    .node(*id)?
                    .number_input
                    .as_ref()
                    .and_then(|mount| mount.initial_draft.clone());
                self.node_mut(*id)?.number_input = Some(NumberInputMount {
                    config: Arc::new(config.clone()),
                    initial: *initial,
                    initial_draft,
                });
            }
            Op::SetNumberInputDraft(id, draft) => {
                if draft
                    .as_ref()
                    .is_some_and(|draft| !gpuio_protocol::number_input::valid_text(draft))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let mount = self
                    .node_mut(*id)?
                    .number_input
                    .as_mut()
                    .ok_or(ErrorCode::InvalidTree)?;
                mount.initial_draft = draft.as_deref().map(Arc::from);
            }
            Op::SetCalendarViewportObserver(id, handler) => {
                if self.node(*id)?.kind != Kind::Calendar {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.calendar_viewport_handler = *handler;
            }
            Op::SetHoverObserver(id, handler) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::Button | Kind::CommandButton | Kind::Link
                ) {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.hover_handler = *handler;
            }
            Op::SetSplitButton(id, config) => {
                if self.node(*id)?.kind != Kind::Container {
                    return Err(ErrorCode::InvalidTree);
                }
                if let Some(config) = config {
                    crate::split_button::validate(config)?;
                }
                self.node_mut(*id)?.split_button =
                    config.as_ref().map(|config| Arc::new(config.clone()));
            }
            Op::SetButtonPresentation(id, config) => {
                let node = self.node(*id)?;
                if !matches!(node.kind, Kind::Button | Kind::CommandButton)
                    || config.is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let changed = node
                    .button_presentation
                    .is_some_and(|config| config.policy.loading)
                    != config.is_some_and(|config| config.policy.loading);
                let activation_revision = self.original.revision + 1;
                let node = self.node_mut(*id)?;
                if changed {
                    node.button_activation_revision = activation_revision;
                }
                node.button_presentation = *config;
            }
            Op::SetTabOrder(id, config) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::Checkbox | Kind::Switch | Kind::Radio | Kind::RadioGroup
                ) || config.is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tab_order = *config;
            }
            Op::SetChoiceMenu(id, enabled) => {
                if self.node(*id)?.kind != Kind::Select {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.choice_menu = *enabled;
            }
            Op::SetTabTrailing(id, enabled) => {
                if self.node(*id)?.kind != Kind::TabBar {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tab_trailing = *enabled;
            }
            Op::SetTabMotion(id, config) => {
                if self.node(*id)?.kind != Kind::TabBar {
                    return Err(ErrorCode::InvalidTree);
                }
                if config.is_some_and(|c| !c.is_valid()) {
                    return Err(ErrorCode::Malformed);
                }
                self.node_mut(*id)?.tab_motion = config.map(Arc::new);
            }
            Op::SetTabViewport(id, config) => {
                if self.node(*id)?.kind != Kind::TabBar {
                    return Err(ErrorCode::InvalidTree);
                }
                if config.as_ref().is_some_and(|c| !c.is_valid()) {
                    return Err(ErrorCode::Malformed);
                }
                self.node_mut(*id)?.tab_viewport = config.clone().map(Arc::new);
            }
            Op::SetTabContent(id, config) => {
                if self.node(*id)?.kind != Kind::TabBar {
                    return Err(ErrorCode::InvalidTree);
                }
                if config.as_ref().is_some_and(|c| !c.is_valid()) {
                    return Err(ErrorCode::Malformed);
                }
                self.node_mut(*id)?.tab_content = config.clone().map(Arc::new);
            }
            Op::SetTabAppearance(id, config) => {
                if self.node(*id)?.kind != Kind::TabBar {
                    return Err(ErrorCode::InvalidTree);
                }
                if let Some(config) = config {
                    crate::tab_appearance::validate(config)?;
                }
                self.node_mut(*id)?.tab_appearance = config.clone().map(Arc::new);
            }
            Op::SetControlAppearance(id, appearance) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::Checkbox | Kind::Switch | Kind::Radio | Kind::RadioGroup
                ) {
                    return Err(ErrorCode::InvalidTree);
                }
                if let Some(appearance) = appearance {
                    crate::control_appearance::validate(appearance)?;
                }
                self.node_mut(*id)?.control_appearance = appearance.clone().map(Arc::new);
            }
            Op::SetRatingAppearance(id, appearance) => {
                if self.node(*id)?.kind != Kind::Rating || appearance.is_some_and(|a| !a.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.rating_appearance = appearance.map(Arc::new);
            }
            Op::SetRating(id, config) => {
                if self.node(*id)?.kind != Kind::Rating || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.rating = Some(Arc::new(config.clone()));
            }
            Op::SetAvatar(id, config) => {
                if self.node(*id)?.kind != Kind::Avatar || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.image = config.image().map(Arc::new);
                node.avatar = Some(Arc::new(config.clone()));
            }
            Op::SetImage(id, config) => {
                if !matches!(self.node(*id)?.kind, Kind::Image | Kind::Icon) || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.image = Some(Arc::new(config.clone()));
            }
            Op::SetExtension(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Extension
                    || !config.is_valid()
                    || node.extension.as_ref().is_some_and(|old| {
                        config.schema != old.schema || config.generation < old.generation
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let same_generation = node
                    .extension
                    .as_ref()
                    .is_some_and(|old| old.generation == config.generation);
                if same_generation
                    && let (Some(old), Some(next)) = (&node.extension_command, &config.command)
                    && (next.sequence < old.sequence
                        || (next.sequence == old.sequence && next != old.as_ref()))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                if !same_generation {
                    node.extension_command = None;
                }
                if let Some(command) = &config.command {
                    node.extension_command = Some(Arc::new(command.clone()));
                }
                node.extension = Some(Arc::new(config.clone()));
            }
            Op::SetSplitGroup(id, config, appearance) => {
                let node = self.node(*id)?;
                if node.kind != Kind::SplitGroup
                    || !config.is_valid()
                    || node
                        .split_group
                        .as_ref()
                        .is_some_and(|old| config.reset_generation < old.config.reset_generation)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                crate::split_group_appearance::validate(appearance)?;
                self.node_mut(*id)?.split_group = Some(SplitGroupMount {
                    config: Arc::new(config.clone()),
                    appearance: Arc::new(appearance.clone()),
                });
            }
            Op::SetSplit(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::SplitPane
                    || !config.is_valid()
                    || node
                        .split
                        .as_ref()
                        .is_some_and(|old| config.reset_generation < old.reset_generation)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.split = Some(Arc::new(config.clone()));
            }
            Op::SetCanvas(id, config) => {
                if self.node(*id)?.kind != Kind::CanvasView || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.canvas = Some(Arc::new(config.clone()));
            }
            Op::SetChart(id, config) => {
                if self.node(*id)?.kind != Kind::ChartView || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.chart = Some(Arc::new(config.clone()));
            }
            Op::SetDocumentDiff(id, epoch, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::DocumentView
                    || *epoch <= node.document_diff_epoch
                    || *epoch <= 0
                    || config.as_ref().is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.document_diff = config.clone().map(Arc::new);
                node.document_diff_epoch = *epoch;
            }
            Op::SetDocument(id, config) => {
                if self.node(*id)?.kind != Kind::DocumentView || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.document = Some(Arc::new(config.clone()));
            }
            Op::SetDocumentMarkdownOptions(id, options) => {
                if self.node(*id)?.kind != Kind::DocumentView {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.document_markdown_options = *options;
            }
            Op::SetDocumentTextStyle(id, config) => {
                if self.node(*id)?.kind != Kind::DocumentView {
                    return Err(ErrorCode::InvalidTree);
                }
                if let Some(config) = config {
                    crate::document_style::validate(config)?;
                }
                self.node_mut(*id)?.document_text_style = config.clone().map(Arc::new);
            }
            Op::SetDocumentProfile(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::DocumentView
                    || !config.is_valid()
                    || config.epoch <= node.document_profile.as_ref().map_or(0, |old| old.epoch)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if let (Some(old), Some(next)) = (
                    node.document_profile
                        .as_ref()
                        .and_then(|old| old.instance.as_ref()),
                    config.instance.as_ref(),
                ) && old.schema == next.schema
                    && next.generation < old.generation
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.document_profile = Some(Arc::new(config.clone()));
            }
            Op::SetDocumentActions(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::DocumentView
                    || !config.is_valid()
                    || config.epoch <= node.document_actions.as_ref().map_or(0, |old| old.epoch)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.document_actions = Some(Arc::new(config.clone()));
            }
            Op::SetDocumentPreview(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::DocumentView
                    || !config.is_valid()
                    || config.epoch
                        <= node
                            .document_preview
                            .as_ref()
                            .map_or(0, |previous| previous.epoch)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.document_preview = Some(Arc::new(config.clone()));
            }
            Op::SetDocumentSelectionFormat(id, markdown) => {
                if self.node(*id)?.kind != Kind::DocumentView {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.document_selection_markdown = *markdown;
            }
            Op::SetSpinner(id, config) => {
                if self.node(*id)?.kind != Kind::Loading || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.loading = Some(Arc::new(config.loading()));
                node.image = config.image().map(Arc::new);
                node.spinner = Some(Arc::new(config.clone()));
            }
            Op::SetLoading(id, config) => {
                if self.node(*id)?.kind != Kind::Loading || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.loading = Some(Arc::new(config.clone()));
                node.spinner = None;
                node.image = None;
            }
            Op::SetProgressPresentation(id, config) => {
                if self.node(*id)?.kind != Kind::Progress || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.progress = Some(Arc::new(config.progress.clone()));
                node.progress_presentation = Some(Arc::new(config.clone()));
            }
            Op::SetProgress(id, config) => {
                if self.node(*id)?.kind != Kind::Progress || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.progress = Some(Arc::new(config.clone()));
                node.progress_presentation =
                    Some(Arc::new(gpuio_protocol::progress_presentation::Config {
                        progress: config.clone(),
                        shape: gpuio_protocol::progress_presentation::Shape::Linear,
                        transition: gpuio_protocol::progress_presentation::Transition::Immediate,
                    }));
            }
            Op::SetPaletteObserved(id, observed) => {
                if self.node(*id)?.kind != Kind::CommandPalette {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.palette_observed = *observed;
            }
            Op::SetPaletteLayout(id, config) => {
                if self.node(*id)?.kind != Kind::CommandPalette
                    || config.as_ref().is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.palette_layout = config.clone().map(Arc::new);
            }
            Op::SetPaletteOptions(id, config) => {
                if self.node(*id)?.kind != Kind::CommandPalette
                    || config.as_ref().is_some_and(|c| !c.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.palette_options = config.clone().map(Arc::new);
            }
            Op::SetPalette(id, config) => {
                if self.node(*id)?.kind != Kind::CommandPalette || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.palette = Some(Arc::new(config.clone()));
                self.structural = true;
            }
            Op::SetMenu(id, config) => {
                if self.node(*id)?.kind != Kind::Menu || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.menu = Some(Arc::new(config.clone()));
                self.structural = true;
            }
            Op::SetCommands(id, commands) => {
                if self.node(*id)?.kind != Kind::CommandScope
                    || !CommandConfig::registry_is_valid(commands)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.commands =
                    Some(commands.iter().cloned().map(Arc::new).collect());
                self.structural = true;
            }
            Op::SetCommandRef(id, command) => {
                if self.node(*id)?.kind != Kind::CommandButton
                    || !CommandConfig::valid_text(command, 256)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let revision = self.original.revision + 1;
                let node = self.node_mut(*id)?;
                if node.command_ref.as_deref() != Some(command.as_str()) {
                    node.button_activation_revision = revision;
                }
                node.command_ref = Some(Arc::from(command.as_str()));
                self.structural = true;
            }
            Op::SetTooltip(id, config) => {
                if !matches!(self.node(*id)?.kind, Kind::Tooltip | Kind::HoverCard)
                    || !config.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tooltip = Some(Arc::new(config.clone()));
            }
            Op::SetPlacement(id, placement) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::FocusScope | Kind::Tooltip | Kind::HoverCard | Kind::Menu
                ) || placement.is_some_and(|placement| !placement.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.placement = *placement;
            }
            Op::SetWindowRegion(id, region) => {
                if self.node(*id)?.kind != Kind::Container {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.window_region = region.map(Arc::new);
            }
            Op::SetSheetInsets(id, insets) => {
                if self.node(*id)?.kind != Kind::FocusScope || insets.is_some_and(|n| !n.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.sheet_insets = *insets;
            }
            Op::SetPlacementGeometry(id, config) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::FocusScope | Kind::Tooltip | Kind::HoverCard | Kind::Menu
                ) || config.is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.placement_geometry = *config;
            }
            Op::SetTooltipMotion(id, enabled) => {
                if self.node(*id)?.kind != Kind::Tooltip {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tooltip_motion = *enabled;
            }
            Op::SetOverlayMotion(id, enabled) => {
                if self.node(*id)?.kind != Kind::FocusScope {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.overlay_motion = *enabled;
            }
            Op::SetOverlayBackdrop(id, color) => {
                if self.node(*id)?.kind != Kind::FocusScope
                    || color.is_some_and(|color| !(0..=0xffff_ffff).contains(&color))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.overlay_backdrop = *color;
            }
            Op::SetOverlay(id, config) => {
                if self.node(*id)?.kind != Kind::FocusScope
                    || config.as_ref().is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.overlay = config.clone().map(Arc::new);
            }
            Op::SetFocusScope(id, config) => {
                if self.node(*id)?.kind != Kind::FocusScope {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.focus_scope = Some(*config);
            }
            Op::SetComboboxFilter(id, filter) => {
                if self.node(*id)?.kind != Kind::Combobox {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.combobox_filter = Some(*filter);
            }
            Op::SetChoiceAppearance(id, appearance) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::Select | Kind::Combobox | Kind::Menu | Kind::CommandPalette
                ) {
                    return Err(ErrorCode::InvalidTree);
                }
                crate::appearance::validate(appearance)?;
                self.node_mut(*id)?.choice_appearance = Some(Arc::new(appearance.clone()));
            }
            Op::SetChoicePicker(id, presentation) => {
                if self.node(*id)?.kind != Kind::ChoicePicker {
                    return Err(ErrorCode::InvalidTree);
                }
                crate::choice_picker_admission::validate(presentation)?;
                self.node_mut(*id)?.choice_picker = Some(Arc::new(presentation.as_ref().clone()));
            }
            Op::SetChoice(id, config) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::RadioGroup | Kind::TabBar | Kind::Select | Kind::Combobox
                ) || !config.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.choice = Some(Arc::new(config.clone()));
            }
            Op::SetControl(id, control) => {
                if matches!(control, Control::Radio(_, Some(position), _) if !position.is_valid()) {
                    return Err(ErrorCode::InvalidTree);
                }

                if self.node(*id)?.kind != control.kind() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.control = Some(*control);
            }
            Op::SetStyle(id, style) => {
                validate_style(style)?;
                self.node_mut(*id)?.style = Arc::from(style.as_slice());
            }
            Op::Bind(id, handler) => self.node_mut(*id)?.handler = *handler,
            Op::Splice(parent, offset, remove, insert) => {
                let node = self.node(*parent)?;
                if !allows_children(node.kind) {
                    return Err(ErrorCode::InvalidTree);
                }
                let start = usize::try_from(*offset).map_err(|_| ErrorCode::InvalidTree)?;
                let count = usize::try_from(*remove).map_err(|_| ErrorCode::InvalidTree)?;
                let end = start
                    .checked_add(count)
                    .filter(|end| *end <= node.children.len())
                    .ok_or(ErrorCode::InvalidTree)?;
                let size = node.children.len() - count + insert.len();
                if size > MAX_NODES {
                    return Err(ErrorCode::LimitExceeded);
                }
                let mut children = Vec::with_capacity(size);
                children.extend_from_slice(&node.children[..start]);
                children.extend_from_slice(insert);
                children.extend_from_slice(&node.children[end..]);
                self.node_mut(*parent)?.children = children.into();
                self.structural = true;
            }
            Op::SetRoot(root) => {
                self.root = *root;
                self.structural = true;
            }
        }
        Ok(())
    }

    fn validate_calendar_content(&self, node: &Node) -> Result<(), ErrorCode> {
        let Some(config) = &node.calendar_content else {
            return Ok(());
        };
        if node.kind != Kind::Calendar
            || !config.is_valid()
            || node.children.len() != config.items.len()
        {
            return Err(ErrorCode::InvalidTree);
        }
        for id in node.children.iter() {
            let slot = self.node(*id)?;
            if slot.kind != Kind::Container
                || !slot.text.is_empty()
                || !slot.style.is_empty()
                || slot.handler.is_some()
                || slot.hover_handler.is_some()
                || slot.accessibility.is_some()
                || slot.overlay.is_some()
                || slot.placement.is_some()
                || slot.table_cell.is_some()
                || slot.popover
                || slot.children.len() != 1
            {
                return Err(ErrorCode::InvalidTree);
            }
        }
        Ok(())
    }

    fn validate_number_presentation(&self, node: &Node) -> Result<(), ErrorCode> {
        if node.number_presentation.is_none() {
            return Ok(());
        }
        if node.kind != Kind::NumberInput || node.children.len() != 4 {
            return Err(ErrorCode::InvalidTree);
        }
        for id in node.children.iter() {
            let slot = self.node(*id)?;
            if slot.kind != Kind::Container
                || !slot.text.is_empty()
                || !slot.style.is_empty()
                || slot.handler.is_some()
                || slot.hover_handler.is_some()
                || slot.accessibility.is_some()
                || slot.overlay.is_some()
                || slot.placement.is_some()
                || slot.table_cell.is_some()
                || slot.children.len() > 1
            {
                return Err(ErrorCode::InvalidTree);
            }
        }
        Ok(())
    }

    fn validate_editor_frame(&self, node: &Node) -> Result<(), ErrorCode> {
        let Some(frame) = &node.editor_frame else {
            if matches!(node.kind, Kind::Input | Kind::Textarea) && !node.children.is_empty() {
                return Err(ErrorCode::InvalidTree);
            }
            return Ok(());
        };
        if !matches!(node.kind, Kind::Input | Kind::Textarea)
            || !frame.is_valid()
            || node.children.len() != 4
            || (node.kind == Kind::Textarea && frame.clear_label.is_some())
        {
            return Err(ErrorCode::InvalidTree);
        }
        for (index, id) in node.children.iter().enumerate() {
            let slot = self.node(*id)?;
            if slot.kind != Kind::Container
                || !slot.text.is_empty()
                || !slot.style.is_empty()
                || slot.handler.is_some()
                || slot.hover_handler.is_some()
                || slot.accessibility.is_some()
                || slot.overlay.is_some()
                || slot.placement.is_some()
                || slot.table_cell.is_some()
                || slot.children.len() > 1
            {
                return Err(ErrorCode::InvalidTree);
            }
            if index == 1 && (slot.children.len() == 1) != frame.loading {
                return Err(ErrorCode::InvalidTree);
            }
            if let Some(child) = slot.children.first() {
                let child = self.node(*child)?;
                if (index == 1 && (child.kind != Kind::Loading || child.spinner.is_none()))
                    || (index == 2
                        && (child.kind != Kind::Button
                            || node.editor_privacy == EditorPrivacy::Plain))
                {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        Ok(())
    }

    fn validate_track_group(&self, node: &Node) -> Result<(), ErrorCode> {
        if node.kind == Kind::CarouselTrackGroup
            && (!node.text.is_empty()
                || node.handler.is_some()
                || !(1..=2).contains(&node.children.len())
                || self.node(node.children[0])?.kind != Kind::CarouselTrack
                || node.children.get(1).is_some_and(|id| {
                    self.node(*id).map_or(true, |controls| {
                        controls.kind != Kind::Container
                            || !controls.text.is_empty()
                            || controls.handler.is_some()
                            || controls.children.len() > 128
                            || controls.children.iter().any(|id| {
                                self.node(*id)
                                    .map_or(true, |control| control.kind != Kind::Button)
                            })
                    })
                }))
        {
            return Err(ErrorCode::InvalidTree);
        }
        Ok(())
    }

    fn validate_structure(&mut self) -> Result<usize, ErrorCode> {
        let mut stack = Vec::new();
        if let Some(root) = self.root {
            stack.push((root, None, 0));
        }
        let mut seen = BTreeSet::new();
        let mut parents = Vec::new();
        while let Some((id, parent, depth)) = stack.pop() {
            if depth > MAX_DEPTH || !seen.insert(id) {
                return Err(ErrorCode::InvalidTree);
            }
            let node = self.node(id)?;
            if parent.is_none()
                && node
                    .overlay
                    .as_ref()
                    .is_some_and(|config| config.kind == OverlayKind::Popover)
            {
                return Err(ErrorCode::InvalidTree);
            }
            if !allows_children(node.kind)
                && node.editor_frame.is_none()
                && node.number_presentation.is_none()
                && !node.children.is_empty()
            {
                return Err(ErrorCode::InvalidTree);
            }
            if node.kind == Kind::Toast
                && !parent.is_some_and(|parent| {
                    self.node(parent)
                        .is_ok_and(|node| node.kind == Kind::ToastStack)
                })
            {
                return Err(ErrorCode::InvalidTree);
            }
            if node.kind == Kind::ToastStack
                && (node.children.len() > MAX_TOASTS
                    || node
                        .children
                        .iter()
                        .any(|id| !self.node(*id).is_ok_and(|node| node.kind == Kind::Toast)))
            {
                return Err(ErrorCode::InvalidTree);
            }
            if node.kind == Kind::Disclosure
                && (node.children.len() != 2
                    || self.node(node.children[1])?.kind != Kind::Panel
                    || {
                        let header = self.node(node.children[0])?;
                        !(header.kind == Kind::Button
                            || (header.kind == Kind::Container
                                && header.children.last().is_some_and(|id| {
                                    self.node(*id).is_ok_and(|node| node.kind == Kind::Button)
                                })))
                    })
            {
                return Err(ErrorCode::InvalidTree);
            }
            if node.kind == Kind::Accordion
                && (node.children.len() > 4096
                    || node.children.iter().any(|id| {
                        !self
                            .node(*id)
                            .is_ok_and(|child| child.kind == Kind::Disclosure)
                    }))
            {
                return Err(ErrorCode::InvalidTree);
            }
            if node.parent != parent {
                parents.push((id, parent));
            }
            // Check edge count before growing traversal storage (duplicate edges
            // must not create unbounded intermediate work).
            if stack.len() + node.children.len() > MAX_NODES {
                return Err(ErrorCode::LimitExceeded);
            }
            stack.extend(
                node.children
                    .iter()
                    .map(|child| (*child, Some(id), depth + 1)),
            );
        }
        if seen.len() != self.node_count {
            return Err(ErrorCode::InvalidTree);
        }
        for (id, parent) in parents {
            self.node_mut(id)?.parent = parent;
        }
        let mut platform_menus = 0;
        for id in &seen {
            let node = self.node(*id)?;
            let mut references = node
                .menu
                .as_ref()
                .map_or_else(Vec::new, |menu| menu.command_ids());
            references.extend(node.command_ref.as_deref());
            if let Some(config) = &node.palette {
                references.extend(config.commands.iter().map(String::as_str));
            }
            if node
                .menu
                .as_ref()
                .is_some_and(|menu| menu.presentation == MenuPresentation::PlatformBar)
            {
                platform_menus += 1;
                if platform_menus > 1 {
                    return Err(ErrorCode::InvalidTree);
                }
            }
            let editor_menu = node
                .menu
                .as_ref()
                .is_some_and(|menu| menu.presentation == MenuPresentation::EditorContext);
            for command in references {
                let mut cursor = Some(*id);
                let mut found = false;
                while let Some(id) = cursor {
                    let node = self.node(id)?;
                    if let Some(entry) = node
                        .commands
                        .as_ref()
                        .and_then(|commands| commands.iter().find(|entry| entry.id == command))
                    {
                        if editor_menu && !matches!(entry.target, CommandTarget::Native(_)) {
                            return Err(ErrorCode::InvalidTree);
                        }
                        found = true;
                        break;
                    }
                    cursor = node.parent;
                }
                if !found {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        Ok(seen.len())
    }
}

fn validate_text(text: &str) -> Result<(), ErrorCode> {
    if text.len() > MAX_TEXT_BYTES {
        Err(ErrorCode::LimitExceeded)
    } else {
        Ok(())
    }
}

pub fn validate_style(style: &[Style]) -> Result<(), ErrorCode> {
    fn nonnegative(value: f64) -> bool {
        value.is_finite() && (0.0..=1_000_000.0).contains(&value)
    }
    fn length(value: &Length) -> bool {
        match value {
            Length::Auto => true,
            Length::Px(v) | Length::Percent(v) => nonnegative(*v),
        }
    }
    fn color(value: &Color) -> bool {
        match value {
            Color::Rgba(v) => (0..=u32::MAX as i64).contains(v),
            Color::Token(v) => (0..256).contains(v),
        }
    }
    if style.len() > MAX_STYLE_FIELDS {
        return Err(ErrorCode::LimitExceeded);
    }
    for field in style {
        if matches!(
            field,
            Style::Background(Color::Token(_))
                | Style::Foreground(Color::Token(_))
                | Style::HoverBackground(Color::Token(_))
                | Style::PressedBackground(Color::Token(_))
                | Style::FocusBackground(Color::Token(_))
        ) {
            // The public theme adapter resolves tokens before submitting colors.
            return Err(ErrorCode::UnsupportedCapability);
        }
        let valid = match field {
            Style::Width(v)
            | Style::Height(v)
            | Style::MinWidth(v)
            | Style::MinHeight(v)
            | Style::MaxWidth(v)
            | Style::MaxHeight(v) => length(v),
            Style::Padding(v)
            | Style::Gap(v)
            | Style::Grow(v)
            | Style::Shrink(v)
            | Style::FontSize(v)
            | Style::Radius(v) => nonnegative(*v),
            Style::Direction(v) => (0..=3).contains(v),
            Style::Background(v)
            | Style::Foreground(v)
            | Style::HoverBackground(v)
            | Style::PressedBackground(v)
            | Style::FocusBackground(v) => color(v),
            Style::Fields(fields) => {
                crate::style::validate_fields(fields)?;
                true
            }
            Style::State(state, fields) => {
                crate::style::validate_fields(fields)?;
                (1..=7).contains(state)
                    && !fields.iter().any(|field| {
                        matches!(
                            field,
                            Field::PointerOcclusion(_)
                                | Field::PointerEvents(_)
                                | Field::UserSelect(_)
                                | Field::SelectionColor(_)
                                | Field::AccessibleName(_)
                                | Field::Inert(_)
                                | Field::Disabled(_)
                        )
                    })
            }
            Style::Opacity(v) => v.is_finite() && (0.0..=1.0).contains(v),
        };
        if !valid {
            return Err(ErrorCode::Malformed);
        }
    }
    Ok(())
}
