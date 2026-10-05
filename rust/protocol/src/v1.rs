pub use crate::command::*;
pub use crate::file_dialog::*;
pub use crate::image::*;
pub use crate::menu::{MenuConfig, MenuDefinition, MenuItem, MenuPresentation};
pub use crate::palette::*;
pub use crate::pointer::*;
pub use crate::progress::*;
pub use crate::toast::*;
use crate::{HandlerId, NodeId, WindowId};
use binprot::macros::BinProtWrite;

// Unpublished exact epoch 3 adds MenuButton observation/placement, split coordination,
// and Link loading beyond epoch 2 button presentation.
pub const VERSION: i64 = 3;
pub const CAP_TREE: i64 = 1;
pub const CAP_NATIVE_STYLES: i64 = 2;
pub const CAP_FRAME_EVENTS: i64 = 4;
pub const CAP_EDITOR: i64 = 8;
pub const CAP_CONTROLS: i64 = 16;
pub const CAP_CHOICES: i64 = 32;
pub const CAP_SELECT: i64 = 64;
pub const CAP_CHOICE_APPEARANCE: i64 = 128;
pub const CAP_COMBOBOX: i64 = 256;
pub const CAP_FOCUS_SCOPES: i64 = 512;
pub const CAP_OVERLAYS: i64 = 1024;
pub const CAP_PLACEMENT: i64 = 2048;
pub const CAP_TOOLTIPS: i64 = 4096;
pub const CAP_COMMANDS: i64 = 8192;
pub const CAP_MENUS: i64 = 16384;
pub const CAP_TOASTS: i64 = 131072;
pub const CAP_PROGRESS: i64 = 65536;
pub const CAP_PALETTE: i64 = 32768;
pub const CAP_POINTER: i64 = 262144;
pub const CAP_FILE_DIALOGS: i64 = 524288;
pub const CAP_DRAG_DROP: i64 = 1048576;
pub const CAP_ASSETS: i64 = 2097152;
pub const CAP_IMAGES: i64 = 4194304;
pub const CAP_SVG: i64 = 8388608;
pub const CAP_BUTTON_ICONS: i64 = 16777216;
pub const CAP_ANIMATIONS: i64 = 33554432;
pub const CAP_VIRTUAL_LISTS: i64 = 67108864;
pub const CAP_DOCUMENTS: i64 = 134217728;
pub const CAP_WINDOWS: i64 = 268435456;
pub const CAP_EXTENSIONS: i64 = 536870912;
pub const CAP_CANVAS_RESOURCES: i64 = 1073741824;
pub const CAP_CANVAS: i64 = 2147483648;
pub const CAP_ANIMATION_PROGRAMS: i64 = 4294967296;
pub const CAP_CONTAINER_QUERIES: i64 = 8589934592;
pub const CAP_PRESENTATION: i64 = 1_i64 << 34;
pub const CAP_NUMERIC_INPUTS: i64 = 1_i64 << 35;
pub const CAP_CALENDARS: i64 = 1_i64 << 36;
pub const CAP_COLOR_INPUTS: i64 = 1_i64 << 37;
pub const CAP_NAVIGATION_COMPONENTS: i64 = 1_i64 << 38;
pub const CAP_MANAGED_TREES: i64 = 1_i64 << 39;
pub const CAP_MANAGED_TABLES: i64 = 1_i64 << 40;
pub const CAP_DESKTOP: i64 = 1_i64 << 41;
pub const CAP_OS_NOTIFICATIONS: i64 = 1_i64 << 42;
pub const CAP_CHARTS: i64 = 1_i64 << 43;
/// Extended cursor values and start-ellipsis styles.
pub const CAP_STYLE_VALUES: i64 = 1_i64 << 44;
pub const CAP_INPUT_REGIONS: i64 = 1_i64 << 45;
pub const CAP_POINTER_OCCLUSION: i64 = 1_i64 << 46;
/// Atomic foreground runs on ordinary text, including selectable text.
pub const CAP_STYLED_TEXT: i64 = 1_i64 << 47;
/// Composed passive-content links with one native action/focus owner.
pub const CAP_LINKS: i64 = 1_i64 << 48;
/// Native solid/dashed border refinements, including state layers.
pub const CAP_BORDER_STYLES: i64 = 1_i64 << 49;
pub const CAP_ASPECT_RATIO: i64 = 1_i64 << 50;
pub const CAP_OPACITY_FACTOR: i64 = 1_i64 << 51;
pub const CAP_COMMAND_BINDINGS: i64 = 1_i64 << 52;
pub const CAP_NUMBER_INPUT_DRAFT: i64 = 1_i64 << 53;
/// Inherited native input/focus gating with discoverable disabled semantics.
pub const CAP_DISABLED_SUBTREES: i64 = 1_i64 << 54;
/// Atomic signed-line/span grid placement for both axes.
pub const CAP_GRID_LOCATION: i64 = 1_i64 << 55;
/// One bounded passive retained fallback child on an avatar.
pub const CAP_AVATAR_FALLBACK: i64 = 1_i64 << 56;
pub const CAP_RATING_APPEARANCE: i64 = 1_i64 << 57;
pub const CAP_SPINNER: i64 = 1_i64 << 58;
pub const CAP_PROGRESS_PRESENTATION: i64 = 1_i64 << 59;
pub const CAP_CONTROL_APPEARANCE: i64 = 1_i64 << 60;
pub const CAP_CONTROL_LABELS: i64 = 1_i64 << 61;
/// Standalone radios, semantic radio groups and per-checkable Tab policy.
pub const CAP_CHECKABLE_NAVIGATION: i64 = 1_i64 << 62;
pub const CAPABILITIES: i64 = CAP_CHECKABLE_NAVIGATION
    | CAP_CONTROL_LABELS
    | CAP_CONTROL_APPEARANCE
    | CAP_PROGRESS_PRESENTATION
    | CAP_SPINNER
    | CAP_RATING_APPEARANCE
    | CAP_AVATAR_FALLBACK
    | CAP_GRID_LOCATION
    | CAP_DISABLED_SUBTREES
    | CAP_NUMBER_INPUT_DRAFT
    | CAP_COMMAND_BINDINGS
    | CAP_OPACITY_FACTOR
    | CAP_ASPECT_RATIO
    | CAP_BORDER_STYLES
    | CAP_LINKS
    | CAP_STYLED_TEXT
    | CAP_POINTER_OCCLUSION
    | CAP_INPUT_REGIONS
    | CAP_STYLE_VALUES
    | CAP_CHARTS
    | CAP_OS_NOTIFICATIONS
    | CAP_DESKTOP
    | CAP_MANAGED_TABLES
    | CAP_MANAGED_TREES
    | CAP_NAVIGATION_COMPONENTS
    | CAP_COLOR_INPUTS
    | CAP_CALENDARS
    | CAP_NUMERIC_INPUTS
    | CAP_PRESENTATION
    | CAP_CONTAINER_QUERIES
    | CAP_ANIMATION_PROGRAMS
    | CAP_CANVAS
    | CAP_CANVAS_RESOURCES
    | CAP_EXTENSIONS
    | CAP_WINDOWS
    | CAP_DOCUMENTS
    | CAP_VIRTUAL_LISTS
    | CAP_ANIMATIONS
    | CAP_BUTTON_ICONS
    | CAP_SVG
    | CAP_IMAGES
    | CAP_ASSETS
    | CAP_TREE
    | CAP_NATIVE_STYLES
    | CAP_FRAME_EVENTS
    | CAP_EDITOR
    | CAP_CONTROLS
    | CAP_CHOICES
    | CAP_SELECT
    | CAP_CHOICE_APPEARANCE
    | CAP_COMBOBOX
    | CAP_FOCUS_SCOPES
    | CAP_OVERLAYS
    | CAP_PLACEMENT
    | CAP_TOOLTIPS
    | CAP_COMMANDS
    | CAP_MENUS
    | CAP_PALETTE
    | CAP_PROGRESS
    | CAP_TOASTS
    | CAP_POINTER
    | CAP_FILE_DIALOGS
    | CAP_DRAG_DROP;
pub const EDITOR_HISTORY_BYTES: usize = 2 * 1024 * 1024;
pub const EDITOR_RESERVED_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 1_048_576;
pub const MAX_TEXT_BYTES: usize = 262_144;
pub const MAX_OPERATIONS: usize = 4096;
pub const MAX_STYLE_FIELDS: usize = 128;
pub const MAX_NODES: usize = 100_000;
pub const MAX_DEPTH: usize = 128;
pub const MAX_WINDOWS: usize = 32;
pub const MAX_RETAINED_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SESSION_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Kind {
    Container,
    Text,
    Button,
    Input,
    Textarea,
    Checkbox,
    Switch,
    RadioGroup,
    Select,
    Combobox,
    FocusScope,
    Tooltip,
    CommandScope,
    CommandButton,
    Menu,
    CommandPalette,
    Progress,
    Toast,
    ToastStack,
    PointerArea,
    DragSource,
    DropTarget,
    Image,
    Icon,
    Animated,
    VirtualList,
    DocumentView,
    TabBar,
    TabPanel,
    SplitPane,
    Extension,
    CanvasView,
    AnimationProgram,
    ContainerQuery,
    Loading,
    Avatar,
    Rating,
    Slider,
    NumberInput,
    OtpInput,
    Calendar,
    ColorInput,
    Panel,
    Disclosure,
    Accordion,
    NavigationStack,
    HoverCard,
    Carousel,
    ChartView,
    InputRegion,
    HighlightScope,
    Link,
    Radio,
    ChoicePicker,
    CarouselTrack,
    CarouselTrackGroup,
    SplitGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct FocusScopeConfig {
    pub trap: bool,
    pub auto_focus: bool,
    pub restore_focus: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ComboboxFilter {
    Substring,
    Unfiltered,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct ChoiceItem {
    pub id: String,
    pub label: String,
    pub disabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct ChoiceConfig {
    pub label: String,
    pub items: Vec<ChoiceItem>,
    pub selected: Option<String>,
    pub disabled: bool,
}
impl ChoiceConfig {
    pub fn is_valid(&self) -> bool {
        fn text(value: &str, max: usize) -> bool {
            !value.is_empty() && value.len() <= max && !value.contains('\0')
        }
        if !text(&self.label, 1024) || self.items.len() > 4096 {
            return false;
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut bytes = 0;
        for item in &self.items {
            bytes += item.id.len() + item.label.len();
            if !text(&item.id, 256)
                || !text(&item.label, 4096)
                || bytes > MAX_TEXT_BYTES
                || !ids.insert(item.id.as_str())
            {
                return false;
            }
        }
        self.selected
            .as_ref()
            .is_none_or(|selected| ids.contains(selected.as_str()))
    }
    pub fn can_select(&self, id: &str) -> bool {
        !self.disabled
            && self
                .items
                .iter()
                .any(|item| item.id == id && !item.disabled)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.label.len()
            + self.selected.as_ref().map_or(0, String::len)
            + self
                .items
                .iter()
                .map(|item| std::mem::size_of::<ChoiceItem>() + item.id.len() + item.label.len())
                .sum::<usize>()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum CheckState {
    Unchecked,
    Checked,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Control {
    Button(bool),
    Checkbox(CheckState, bool),
    Switch(bool, bool),
    Radio(bool, Option<crate::checkable::Position>, bool),
}
impl Control {
    pub fn disabled(self) -> bool {
        match self {
            Self::Button(disabled)
            | Self::Checkbox(_, disabled)
            | Self::Switch(_, disabled)
            | Self::Radio(_, _, disabled) => disabled,
        }
    }
    pub fn kind(self) -> Kind {
        match self {
            Self::Button(_) => Kind::Button,
            Self::Checkbox(..) => Kind::Checkbox,
            Self::Switch(..) => Kind::Switch,
            Self::Radio(..) => Kind::Radio,
        }
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Length {
    Px(f64),
    Percent(f64),
    Auto,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Color {
    Rgba(i64),
    Token(i64),
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Fill {
    Solid(Color),
    LinearGradient(f64, Color, f64, Color, f64),
    /// Explicit interpolation: 0=sRGB, 1=Oklab. Legacy tag1 remains sRGB.
    LinearGradientIn(i64, f64, Color, f64, Color, f64),
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Shadow {
    pub color: Color,
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur: f64,
    pub spread: f64,
    pub inset: bool,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Field {
    Display(i64),
    Visibility(i64),
    Direction(i64),
    Wrap(i64),
    Grow(f64),
    Shrink(f64),
    Basis(Length),
    AlignItems(i64),
    AlignSelf(i64),
    AlignContent(i64),
    JustifyContent(i64),
    RowGap(Length),
    ColumnGap(Length),
    GridColumns(i64),
    GridRows(i64),
    GridColumnMinimum(i64),
    GridRowMinimum(i64),
    Width(Length),
    Height(Length),
    MinWidth(Length),
    MinHeight(Length),
    MaxWidth(Length),
    MaxHeight(Length),
    PaddingTop(Length),
    PaddingRight(Length),
    PaddingBottom(Length),
    PaddingLeft(Length),
    MarginTop(Length),
    MarginRight(Length),
    MarginBottom(Length),
    MarginLeft(Length),
    Position(i64),
    Top(Length),
    Right(Length),
    Bottom(Length),
    Left(Length),
    Background(Fill),
    Foreground(Color),
    Opacity(f64),
    BorderTopWidth(f64),
    BorderRightWidth(f64),
    BorderBottomWidth(f64),
    BorderLeftWidth(f64),
    TopLeftRadius(f64),
    TopRightRadius(f64),
    BottomLeftRadius(f64),
    BottomRightRadius(f64),
    BorderColor(Color),
    Shadows(Vec<Shadow>),
    FontSize(f64),
    FontFamily(String),
    FontWeight(i64),
    TextAlign(i64),
    LineHeight(Length),
    WhiteSpace(i64),
    TextOverflow(i64),
    LineClamp(i64),
    TextDecoration(i64),
    OverflowX(i64),
    OverflowY(i64),
    Cursor(i64),
    PointerEvents(bool),
    UserSelect(bool),
    SelectionColor(Color),
    AccessibleName(String),
    Inert(bool),
    PointerOcclusion(i64),
    BorderStyle(i64),
    AspectRatio(f64),
    Disabled(bool),
    GridLocation(crate::grid_location::Location),
}

/// Initial portable refinements; adding tags requires explicit schema review.
/// Replacing a node's style list also clears absent prior refinements.
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Style {
    Width(Length),
    Height(Length),
    MinWidth(Length),
    MinHeight(Length),
    MaxWidth(Length),
    MaxHeight(Length),
    Padding(f64),
    Gap(f64),
    Grow(f64),
    Shrink(f64),
    Direction(i64),
    Background(Color),
    Foreground(Color),
    FontSize(f64),
    Radius(f64),
    Opacity(f64),
    HoverBackground(Color),
    PressedBackground(Color),
    FocusBackground(Color),
    Fields(Vec<Field>),
    State(i64, Vec<Field>),
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct EditorSelection {
    pub anchor: i64,
    pub head: i64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub enum EditorPrivacy {
    #[default]
    Plain,
    PasswordHidden,
    PasswordRevealed,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct EditorConfig {
    pub label: String,
    pub placeholder: String,
    pub read_only: bool,
    pub disabled: bool,
    pub submit_on_enter: bool,
    pub auto_focus: bool,
    pub min_rows: i64,
    pub max_rows: i64,
}
impl EditorConfig {
    pub fn is_valid(&self) -> bool {
        !self.label.is_empty()
            && self.label.len() <= 1024
            && !self.label.contains('\0')
            && self.placeholder.len() <= 4096
            && !self.placeholder.contains('\0')
            && (1..=256).contains(&self.min_rows)
            && (self.min_rows..=256).contains(&self.max_rows)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct EditorSnapshot {
    pub revision: i64,
    pub text: String,
    pub selection: EditorSelection,
    pub composition: Option<EditorSelection>,
    pub focused: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum EditorSelectionPolicy {
    Start,
    End,
    Preserve,
    Select(EditorSelection),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum EditorUndoPolicy {
    Record,
    Reset,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum EditorCommand {
    Replace(String, EditorSelectionPolicy, EditorUndoPolicy, Option<i64>),
    Select(EditorSelection),
    Focus,
    Undo,
    Redo,
    Submit,
    ReadSnapshot,
    ReadContentHintStatus,
    ReadViewport,
    ScrollViewport(crate::editor_viewport::Offset),
    Search(crate::editor_search::Command),
    ReadRangeBounds(i64, EditorSelection),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum EditorError {
    NotMounted,
    Closed,
    StaleEditor,
    StaleRevision,
    Composing,
    InvalidSelection,
    LimitExceeded,
    Busy,
    NativeFailure,
    InvalidText,
    FocusBlocked,
    SearchUnavailable,
    StaleSearch,
    NotEditable,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum EditorResult {
    Applied(EditorSnapshot),
    Failed(EditorError),
    ContentHintStatus(crate::input_content_hint::Status),
    Viewport(Option<crate::editor_viewport::Snapshot>),
    ViewportScrollAccepted,
    SearchObserved(crate::editor_search::Snapshot),
    SearchReplaced(EditorSnapshot, crate::editor_search::Snapshot, i64),
    RangeBounds(Option<crate::editor_geometry::Snapshot>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum EditorEventKind {
    Changed,
    Submitted,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct ChoiceAppearance {
    pub popup_width: f64,
    pub row_height: f64,
    pub max_visible_rows: i64,
    pub empty_label: String,
    pub popup_style: Vec<Style>,
    pub option_style: Vec<Style>,
    pub empty_style: Vec<Style>,
}
impl Default for ChoiceAppearance {
    fn default() -> Self {
        Self {
            popup_width: 320.,
            row_height: 32.,
            max_visible_rows: 8,
            empty_label: "No options".into(),
            popup_style: vec![],
            option_style: vec![],
            empty_style: vec![],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Align {
    Start,
    Center,
    End,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Placement {
    pub side: Side,
    pub align: Align,
    pub offset: f64,
}
impl Default for Placement {
    fn default() -> Self {
        Self {
            side: Side::Bottom,
            align: Align::Start,
            offset: 0.,
        }
    }
}
impl Placement {
    pub fn is_valid(&self) -> bool {
        self.offset.is_finite() && (-16384.0..=16384.0).contains(&self.offset)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum TooltipOpenState {
    Managed(bool),
    Controlled(bool),
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct TooltipConfig {
    pub label: String,
    pub width: f64,
    pub open_state: TooltipOpenState,
    pub disabled: bool,
    pub hoverable: bool,
    pub show_delay_ns: i64,
    pub hide_delay_ns: i64,
    pub skip_delay_ns: i64,
}
impl TooltipConfig {
    pub fn is_valid(&self) -> bool {
        !self.label.trim().is_empty()
            && self.label.len() <= 4096
            && !self.label.contains('\0')
            && self.width.is_finite()
            && (1.0..=16384.0).contains(&self.width)
            && [self.show_delay_ns, self.hide_delay_ns, self.skip_delay_ns]
                .into_iter()
                .all(|delay| (0..=60_000_000_000).contains(&delay))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum OverlayKind {
    Dialog,
    Popover,
    SheetLeft,
    SheetRight,
    SheetTop,
    SheetBottom,
    AlertDialog,
}
impl OverlayKind {
    pub fn is_modal(self) -> bool {
        match self {
            Self::Dialog
            | Self::SheetLeft
            | Self::SheetRight
            | Self::SheetTop
            | Self::SheetBottom
            | Self::AlertDialog => true,
            Self::Popover => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Dismissal {
    Escape,
    OutsidePointer,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct OverlayConfig {
    pub kind: OverlayKind,
    pub label: String,
    pub width: f64,
    pub dismiss_on_escape: bool,
    pub dismiss_on_outside_pointer: bool,
}
impl OverlayConfig {
    pub fn is_valid(&self) -> bool {
        !self.label.trim().is_empty()
            && self.label.len() <= 4096
            && !self.label.contains('\0')
            && self.width.is_finite()
            && (1.0..=16384.0).contains(&self.width)
            && (self.kind != OverlayKind::AlertDialog || !self.dismiss_on_outside_pointer)
    }
    pub fn allows(&self, reason: Dismissal) -> bool {
        match reason {
            Dismissal::Escape => self.dismiss_on_escape,
            Dismissal::OutsidePointer => self.dismiss_on_outside_pointer,
        }
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Op {
    Create(NodeId, Kind, String, Option<HandlerId>),
    Remove(NodeId),
    SetText(NodeId, String),
    SetStyle(NodeId, Vec<Style>),
    Bind(NodeId, Option<HandlerId>),
    Splice(NodeId, i64, i64, Vec<NodeId>),
    SetRoot(Option<NodeId>),
    SetEditor(NodeId, EditorConfig),
    SetControl(NodeId, Control),
    SetChoice(NodeId, ChoiceConfig),
    SetChoiceAppearance(NodeId, ChoiceAppearance),
    SetComboboxFilter(NodeId, ComboboxFilter),
    SetFocusScope(NodeId, FocusScopeConfig),
    SetOverlay(NodeId, Option<OverlayConfig>),
    SetPlacement(NodeId, Option<Placement>),
    SetTooltip(NodeId, TooltipConfig),
    SetCommands(NodeId, Vec<CommandConfig>),
    SetCommandRef(NodeId, String),
    SetMenu(NodeId, MenuConfig),
    SetPalette(NodeId, PaletteConfig),
    SetProgress(NodeId, ProgressConfig),
    SetToast(NodeId, ToastConfig),
    SetToastStack(NodeId, ToastStackConfig),
    SetPointer(NodeId, PointerConfig),
    SetDragSource(NodeId, crate::drag_drop::Source),
    SetDropTarget(NodeId, crate::drag_drop::Target),
    SetImage(NodeId, ImageConfig),
    SetAnimation(NodeId, crate::animation::Config),
    SetListConfig(NodeId, crate::list::Config),
    SetListOrder(NodeId, crate::list::Order),
    SetListRows(NodeId, Vec<crate::list::Row>),
    InvalidateListRows(NodeId, Vec<i64>),
    ScrollList(NodeId, crate::list::ScrollRequest),
    SetDocument(NodeId, crate::document::Config),
    SetSplit(NodeId, crate::split::Config),
    SetExtension(NodeId, crate::extension::Config),
    SetCanvas(NodeId, crate::canvas_view::Config),
    SetAnimationProgram(NodeId, crate::animation_program::Config),
    SetContainerQuery(NodeId, crate::container_query::Config),
    SetAccessibility(NodeId, Option<crate::accessibility::Config>),
    SetLoading(NodeId, crate::loading::Config),
    SetAvatar(NodeId, crate::avatar::Config),
    SetRating(NodeId, crate::rating::Config),
    SetSlider(NodeId, crate::slider::Config, crate::slider::Value),
    SetNumberInput(
        NodeId,
        crate::number_input::Config,
        crate::number_input::Value,
    ),
    SetOtpInput(NodeId, crate::otp_input::Config, String),
    SetCalendar(
        NodeId,
        Box<crate::calendar_input::Config>,
        crate::calendar::Selection,
        crate::calendar::Month,
    ),
    SetColorInput(
        NodeId,
        Box<crate::color_input::Config>,
        crate::color_value::Value,
    ),
    SetNavigationStack(NodeId, crate::navigation_stack::Config),
    SetCarousel(NodeId, crate::carousel::Config),
    SetTreeInput(NodeId, bool),
    SetTreeMoves(NodeId, bool),
    SetTable(NodeId, crate::table::Config),
    SetTableCell(NodeId, crate::table::Cell),
    TableCommand(NodeId, crate::table::Command),
    SetChart(NodeId, crate::chart_view::Config),
    SetInputRegion(NodeId, crate::input::Config),
    SetHighlightScope(NodeId, crate::highlight::Config),
    SetDocumentDiff(NodeId, i64, Option<crate::document_diff::Config>),
    SetStyledText(NodeId, crate::text_content::Content),
    SetLink(NodeId, crate::link::Config),
    SetTextShimmer(NodeId, Option<crate::text_shimmer::Config>),
    SetCommandBinding(NodeId, Option<crate::command_binding::Config>),
    SetNumberInputDraft(NodeId, Option<String>),
    SetRatingAppearance(NodeId, Option<crate::rating::Appearance>),
    SetSpinner(NodeId, crate::spinner::Config),
    SetProgressPresentation(NodeId, crate::progress_presentation::Config),
    SetControlAppearance(NodeId, Option<crate::control_appearance::Config>),
    SetTabOrder(NodeId, Option<crate::checkable::TabOrder>),
    SetButtonPresentation(NodeId, Option<crate::button::Config>),
    SetSplitButton(NodeId, Option<crate::split_button::Config>),
    SetHoverObserver(NodeId, Option<HandlerId>),
    SetChoicePicker(NodeId, Box<crate::choice_picker::Presentation>),
    SetEditorPrivacy(NodeId, EditorPrivacy),
    SetEditorFrame(NodeId, Option<crate::editor_frame::Config>),
    SetEditorContentHint(NodeId, Option<crate::input_content_hint::Hint>),
    SetEditorFormat(NodeId, Option<crate::input_format::Config>),
    SetEditorValidation(NodeId, Option<crate::input_validation::Rule>),
    SetTextAreaLayout(NodeId, Option<crate::text_area_layout::Config>),
    SetEditorClearOnEscape(NodeId, bool),
    SetEditorSearchable(NodeId, bool),
    SetOtpAppearance(NodeId, Option<crate::otp_presentation::Appearance>),
    SetNumberPresentation(NodeId, Option<crate::number_presentation::Config>),
    SetNumberStepMode(NodeId, crate::number_input::StepMode),
    SetSliderAppearance(NodeId, Option<crate::slider_presentation::Appearance>),
    SetReveal(NodeId, Option<crate::reveal::Config>),
    SetCalendarAppearance(NodeId, Option<crate::calendar_presentation::Appearance>),
    SetColorPresentation(NodeId, Option<crate::color_presentation::Presentation>),
    SetPopover(NodeId, bool),
    SetCalendarContent(NodeId, Option<crate::calendar_content::Config>),
    SetOverlayBackdrop(NodeId, Option<i64>),
    SetOverlayMotion(NodeId, bool),
    SetTooltipMotion(NodeId, bool),
    SetPlacementGeometry(NodeId, Option<crate::placement_geometry::Config>),
    SetSheetInsets(NodeId, Option<crate::sheet_insets::Insets>),
    SetCalendarViewportObserver(NodeId, Option<HandlerId>),
    SetCarouselTrack(NodeId, crate::carousel_track::Config),
    SetCarouselTrackMotion(NodeId, Option<crate::carousel_track::Motion>),
    SetTabAppearance(NodeId, Option<crate::tab_appearance::Config>),
    SetTabContent(NodeId, Option<crate::tab_content::Config>),
    SetTabViewport(NodeId, Option<crate::tab_viewport::Config>),
    SetTabTrailing(NodeId, bool),
    SetChoiceMenu(NodeId, bool),
    SetTabMotion(NodeId, Option<crate::tab_motion::Config>),
    SetSplitGroup(
        NodeId,
        crate::split_group::Config,
        crate::split_group_appearance::Config,
    ),
    SetToastPlacement(NodeId, Option<crate::toast_placement::Placement>),
    SetToastLayering(NodeId, Option<crate::toast_layering::Layering>),
    SetToastMotion(NodeId, Option<crate::toast_motion::Config>),
    SetScrollbar(NodeId, Option<Box<crate::scrollbar::Config>>),
    SetListAxis(NodeId, crate::list::Axis),
    SetListInput(NodeId, Option<crate::list_input::Config>),
    SetTableBehavior(NodeId, Option<crate::table::Behavior>),
    SetTableAppearance(NodeId, Option<crate::table::Appearance>),
    SetTableHeader(NodeId, Option<crate::table_header::Target>),
    SetTableHeaderStyle(NodeId, Vec<Style>),
    SetTableRowStyle(NodeId, Vec<Style>),
    SetDocumentSelectionFormat(NodeId, bool),
    SetDocumentPreview(NodeId, crate::document_preview::Config),
    SetDocumentTextStyle(NodeId, Option<crate::document_style::Config>),
    SetDocumentMarkdownOptions(NodeId, crate::document::MarkdownOptions),
    SetDocumentActions(NodeId, crate::document_actions::Config),
    SetDocumentProfile(NodeId, crate::document_profile::Config),
    SetWindowRegion(NodeId, Option<crate::window_region::Region>),
    CreateTableText(NodeId, crate::table::Cell),
    SetTableText(NodeId, crate::table::Cell),
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Transaction {
    pub window: WindowId,
    pub base: i64,
    pub revision: i64,
    pub operations: Vec<Op>,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Message {
    Hello(i64, i64),                       // exact version, required capability mask
    Open(i64, WindowId, String, f64, f64), // correlation, id, title, logical size
    Close(i64, WindowId),
    Apply(Transaction),
    RequestFrame(i64, WindowId),
    Shutdown,
    EditorCommand(i64, WindowId, NodeId, EditorCommand),
    FileDialog(i64, WindowId, FileDialogConfig),
    Asset(i64, crate::asset::Request),
    SetMotion(crate::animation::Preference),
    Document(i64, crate::document::Request),
    WindowCommand(i64, WindowId, crate::window::Command),
    OpenConfigured(i64, WindowId, crate::window::Config),
    Canvas(i64, crate::canvas_resource::Request),
    SliderCommand(i64, WindowId, NodeId, crate::slider::Command),
    NumberInputCommand(i64, WindowId, NodeId, crate::number_input::Command),
    OtpInputCommand(i64, WindowId, NodeId, crate::otp_input::Command),
    CalendarCommand(i64, WindowId, NodeId, crate::calendar_input::Command),
    ColorInputCommand(i64, WindowId, NodeId, crate::color_input::Command),
    Desktop(i64, crate::desktop::Request),
    Notification(i64, crate::notification::Request),
    Chart(i64, crate::chart_resource::Request),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ErrorCode {
    UnsupportedVersion,
    UnsupportedCapability,
    Malformed,
    LimitExceeded,
    NotReady,
    StaleHandle,
    InvalidRevision,
    InvalidTree,
    Busy,
    Closed,
    Overloaded,
    NativeFailure,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Event {
    Welcome(i64, i64),
    Opened(i64, WindowId),
    Closed(i64, WindowId),
    Accepted(WindowId, i64),
    Rejected(WindowId, i64, ErrorCode),
    Rendered(WindowId, i64),
    FrameRequested(i64, WindowId, i64),
    Press(WindowId, NodeId, HandlerId, i64),
    Failed(i64, ErrorCode),
    Stopped,
    Overloaded(WindowId),
    EditorEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        EditorEventKind,
        EditorSnapshot,
    ),
    EditorResult(i64, WindowId, NodeId, EditorResult),
    Choice(WindowId, NodeId, HandlerId, i64, String),
    ComboboxSelected(WindowId, NodeId, HandlerId, i64, String, EditorSnapshot),
    OverlayDismissed(WindowId, NodeId, HandlerId, i64, Dismissal),
    TooltipOpenChanged(WindowId, NodeId, HandlerId, i64, bool),
    CommandInvoked(WindowId, NodeId, HandlerId, i64, String, i64, CommandSource),
    PaletteDismissed(WindowId, NodeId, HandlerId, i64, PaletteDismissal),
    ToastDismissed(WindowId, NodeId, HandlerId, i64, ToastDismissal),
    PointerEvent(WindowId, NodeId, HandlerId, i64, PointerSample),
    FileDialogResult(i64, WindowId, FileDialogResult),
    DragSourceEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::drag_drop::SourceSample,
    ),
    DropTargetEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::drag_drop::TargetSample,
    ),
    AssetResponse(i64, crate::asset::Response),
    ImageState(WindowId, NodeId, HandlerId, i64, ImageState),
    AnimationEndpoint(WindowId, NodeId, HandlerId, i64, crate::animation::Endpoint),
    ListViewport(WindowId, NodeId, HandlerId, i64, crate::list::Viewport),
    ListRetained(WindowId, i64, Vec<crate::list::Retained>),
    DocumentResponse(i64, crate::document::Response),
    DocumentNavigation(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::ResourceId,
        i64,
        crate::document::Navigation,
    ),
    CloseRequested(WindowId),
    QuitRequested,
    ReopenRequested,
    WindowChanged(WindowId, crate::window::Snapshot),
    WindowResponse(i64, WindowId, crate::window::Response),
    WindowCapabilities(crate::window::Capabilities),
    SplitResized(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        i64,
        crate::split::Snapshot,
    ),
    ExtensionEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        i64,
        crate::extension::Signal,
    ),
    CanvasResponse(i64, crate::canvas_resource::Response),
    CanvasEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        Option<crate::ResourceId>,
        i64,
        i64,
        crate::canvas_view::Observation,
    ),
    AnimationProgramEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        Vec<crate::animation_program::Signal>,
    ),
    ContainerSelected(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::container_query::Snapshot,
    ),
    RatingRequested(WindowId, NodeId, HandlerId, i64, crate::rating::Request),
    SliderEvent(WindowId, NodeId, HandlerId, i64, crate::slider::Event),
    SliderResult(i64, WindowId, NodeId, crate::slider::Response),
    NumberInputEvent(WindowId, NodeId, HandlerId, i64, crate::number_input::Event),
    NumberInputResult(i64, WindowId, NodeId, crate::number_input::Response),
    OtpInputEvent(WindowId, NodeId, HandlerId, i64, crate::otp_input::Event),
    OtpInputResult(i64, WindowId, NodeId, crate::otp_input::Response),
    CalendarEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::calendar_input::Event,
    ),
    CalendarResult(i64, WindowId, NodeId, crate::calendar_input::Response),
    ColorInputEvent(WindowId, NodeId, HandlerId, i64, crate::color_input::Event),
    ColorInputResult(i64, WindowId, NodeId, crate::color_input::Response),
    CarouselRequested(WindowId, NodeId, HandlerId, i64, crate::carousel::Request),
    TreeInput(WindowId, NodeId, HandlerId, i64, crate::tree_input::Request),
    TableInput(WindowId, NodeId, HandlerId, i64, crate::table::Input),
    DesktopResponse(i64, crate::desktop::Response),
    DesktopPending,
    NotificationResponse(i64, crate::notification::Response),
    NotificationPending,
    ChartResponse(i64, crate::chart_resource::Response),
    ChartEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        Option<crate::ResourceId>,
        i64,
        i64,
        crate::chart_view::Observation,
    ),
    InputObserved(WindowId, NodeId, HandlerId, i64, crate::input::Event),
    HighlightObserved(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::highlight::Observation,
    ),
    DocumentDiffEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::ResourceId,
        crate::document_diff::Event,
    ),
    CommandBindingObserved(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::command_binding::Observation,
    ),
    MenuOpenChanged(WindowId, NodeId, HandlerId, i64, bool),
    HoverChanged(WindowId, NodeId, HandlerId, i64, bool),
    ChoicePickerEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::choice_picker::Event,
    ),
    EditorSearchObserved(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::editor_search::Snapshot,
    ),
    CalendarViewportChanged(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::calendar_viewport::Observation,
    ),
    CarouselTrackRequested(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::carousel_track::Request,
    ),
    SplitGroupResized(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        i64,
        crate::split_group::Snapshot,
    ),
    ListInput(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        i64,
        crate::list_input::Request,
    ),
    TableColumnsObserved(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::table::ColumnViewport,
    ),
    DocumentPreviewObserved(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::ResourceId,
        crate::document_preview::Event,
    ),
    DocumentAction(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::ResourceId,
        crate::document_actions::Event,
    ),
    DocumentProfileEvent(
        WindowId,
        NodeId,
        HandlerId,
        i64,
        crate::ResourceId,
        crate::document_profile::Event,
    ),
}
