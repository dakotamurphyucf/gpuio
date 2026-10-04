//! Window requests use logical pixels. Acknowledgements observe compositor state;
//! they do not promise a requested resize/fullscreen transition has completed.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Chrome {
    Standard,
    Hidden,
    Custom,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Frame {
    pub shadow_size: f64,
    pub resize_hit_size: f64,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            shadow_size: 20.,
            resize_hit_size: 4.,
        }
    }
}
impl Frame {
    pub fn is_valid(&self) -> bool {
        self.shadow_size.is_finite()
            && self.resize_hit_size.is_finite()
            && (0.0..=128.0).contains(&self.shadow_size)
            && (0.5..=32.0).contains(&self.resize_hit_size)
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub focus: bool,
    pub chrome: Chrome,
    pub resizable: bool,
    pub frame: Frame,
}
pub fn valid_title(title: &str) -> bool {
    !title.is_empty() && title.len() <= 4096 && !title.contains('\0')
}
pub fn valid_size(width: f64, height: f64) -> bool {
    width.is_finite()
        && height.is_finite()
        && (1.0..=16384.0).contains(&width)
        && (1.0..=16384.0).contains(&height)
}
impl Config {
    pub fn is_valid(&self) -> bool {
        valid_title(&self.title) && valid_size(self.width, self.height) && self.frame.is_valid()
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Document {
    pub path: Option<crate::file_path::FilePath>,
    pub edited: bool,
}
pub const MAX_SELECTION_BYTES: usize = 262_144;
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Command {
    Observe,
    SetTitle(String),
    Resize(f64, f64),
    Activate,
    Zoom,
    ToggleFullscreen,
    SetEdited(bool),
    SetDocument(Document),
    Minimize,
    FocusedInput,
    HasTextSelection,
    SelectedText(i64),
    ClearTextSelection,
    EndTextSelection,
}
impl Command {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::SelectedText(maximum) => (0..=MAX_SELECTION_BYTES as i64).contains(maximum),
            Self::SetTitle(title) => valid_title(title),
            Self::Resize(w, h) => valid_size(*w, *h),
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Tiling {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Decorations {
    Server,
    Client(Tiling),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Controls {
    pub fullscreen: bool,
    pub maximize: bool,
    pub minimize: bool,
    pub window_menu: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Presentation {
    pub decorations: Decorations,
    pub controls: Controls,
    pub resizable: bool,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub content_width: f64,
    pub content_height: f64,
    pub active: bool,
    pub fullscreen: bool,
    pub maximized: bool,
    pub document: Option<Document>,
    pub presentation: Presentation,
    pub appearance: Appearance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Appearance {
    Light,
    VibrantLight,
    Dark,
    VibrantDark,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Backend {
    Macos,
    Wayland,
    X11,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Capabilities {
    pub backend: Backend,
    pub native_quit_decision: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    Closed,
    NotReady,
    Busy,
    InvalidRequest,
    NativeFailure,
    Unsupported,
    LimitExceeded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum InputKind {
    Input,
    Textarea,
    Combobox,
    Otp,
    Number,
    Color,
    CommandPalette,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Input {
    pub node: crate::NodeId,
    pub kind: InputKind,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Response {
    Observed(Snapshot),
    Failed(Error),
    FocusedInput(Option<Input>),
    SelectionPresent(bool),
    SelectedText(String),
    SelectionUpdated,
}
