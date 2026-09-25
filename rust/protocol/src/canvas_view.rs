//! Small mounted-canvas configuration. Scene data lives in scoped resources.
use crate::{
    ResourceId,
    canvas::{Point, Transform},
};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Viewport {
    pub origin: Point,
    pub zoom: f64,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            origin: Point { x: 0., y: 0. },
            zoom: 1.,
        }
    }
}
impl Viewport {
    pub fn is_valid(&self) -> bool {
        self.origin.is_valid() && self.zoom.is_finite() && (0.05..=64.).contains(&self.zoom)
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Action {
    Select(Option<i64>),
    SetViewport(Viewport),
    ResetViewport,
    ResetPositions,
}
impl Action {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Select(id) => id.is_none_or(|id| id > 0),
            Self::SetViewport(viewport) => viewport.is_valid(),
            Self::ResetViewport | Self::ResetPositions => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Command {
    pub sequence: i64,
    pub action: Action,
}
impl Command {
    pub fn is_valid(&self) -> bool {
        self.sequence > 0 && self.action.is_valid()
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub source: Option<ResourceId>,
    pub label: String,
    pub initial_viewport: Viewport,
    pub minimum_zoom: f64,
    pub maximum_zoom: f64,
    pub selectable: bool,
    pub draggable: bool,
    pub pan_zoom: bool,
    pub disabled: bool,
    pub selection_color: i64,
    pub command: Option<Command>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.label.bytes().any(|byte| !matches!(byte, 9..=13 | 32))
            && self.label.len() <= 1024
            && !self.label.contains('\0')
            && self.initial_viewport.is_valid()
            && self.minimum_zoom.is_finite()
            && self.maximum_zoom.is_finite()
            && self.minimum_zoom >= 0.05
            && self.maximum_zoom <= 64.
            && self.minimum_zoom <= self.initial_viewport.zoom
            && self.initial_viewport.zoom <= self.maximum_zoom
            && (0..=0xffff_ffff).contains(&self.selection_color)
            && self.command.as_ref().is_none_or(Command::is_valid)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    WrongApplication,
    UnavailableScene,
    RenderLimit,
    UnavailableImage,
    InvalidCommand,
    NativeFailure,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Observation {
    SelectionChanged(Option<i64>),
    Activated(i64),
    Moved(i64, Transform),
    ViewportChanged(Viewport),
    CommandCompleted(i64),
    Failed(Error),
}
impl Observation {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::SelectionChanged(id) => id.is_none_or(|id| id > 0),
            Self::Activated(id) | Self::CommandCompleted(id) => *id > 0,
            Self::Moved(id, transform) => *id > 0 && transform.is_valid(),
            Self::ViewportChanged(viewport) => viewport.is_valid(),
            Self::Failed(_) => true,
        }
    }
}
