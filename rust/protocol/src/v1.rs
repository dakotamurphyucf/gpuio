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

pub const VERSION: i64 = 1;
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
pub const CAPABILITIES: i64 = CAP_WINDOWS
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
}
impl Control {
    pub fn disabled(self) -> bool {
        match self {
            Self::Button(disabled) | Self::Checkbox(_, disabled) | Self::Switch(_, disabled) => {
                disabled
            }
        }
    }
    pub fn kind(self) -> Kind {
        match self {
            Self::Button(_) => Kind::Button,
            Self::Checkbox(..) => Kind::Checkbox,
            Self::Switch(..) => Kind::Switch,
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
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum EditorCommand {
    Replace(String, EditorSelectionPolicy, EditorUndoPolicy, Option<i64>),
    Select(EditorSelection),
    Focus,
    Undo,
    Redo,
    Submit,
    ReadSnapshot,
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
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum EditorResult {
    Applied(EditorSnapshot),
    Failed(EditorError),
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
}
