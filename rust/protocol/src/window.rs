//! Window requests use logical pixels. Acknowledgements observe compositor state;
//! they do not promise a requested resize/fullscreen transition has completed.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Chrome {
    Standard,
    Hidden,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub focus: bool,
    pub chrome: Chrome,
    pub resizable: bool,
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
        valid_title(&self.title) && valid_size(self.width, self.height)
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Command {
    Observe,
    SetTitle(String),
    Resize(f64, f64),
    Activate,
    Zoom,
    ToggleFullscreen,
    SetEdited(bool),
}
impl Command {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::SetTitle(title) => valid_title(title),
            Self::Resize(w, h) => valid_size(*w, *h),
            _ => true,
        }
    }
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
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Response {
    Observed(Snapshot),
    Failed(Error),
}
