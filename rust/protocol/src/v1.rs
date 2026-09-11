use crate::{HandlerId, NodeId, WindowId};
use binprot::macros::BinProtWrite;

pub const VERSION: i64 = 1;
pub const CAP_TREE: i64 = 1;
pub const CAP_NATIVE_STYLES: i64 = 2;
pub const CAP_FRAME_EVENTS: i64 = 4;
pub const CAPABILITIES: i64 = CAP_TREE | CAP_NATIVE_STYLES | CAP_FRAME_EVENTS;
pub const MAX_MESSAGE_BYTES: usize = 1_048_576;
pub const MAX_TEXT_BYTES: usize = 262_144;
pub const MAX_OPERATIONS: usize = 4096;
pub const MAX_STYLE_FIELDS: usize = 128;
pub const MAX_NODES: usize = 100_000;
pub const MAX_DEPTH: usize = 128;
pub const MAX_WINDOWS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Kind {
    Container,
    Text,
    Button,
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
}
