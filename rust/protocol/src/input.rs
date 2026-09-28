//! Validated general input observations. Host envelopes/adapters are not enabled
//! by this module alone. Existing captured pointer gestures remain separate.
use crate::{
    pointer::{PointerButton, PointerModifiers},
    v1::CommandConfig,
};
use binprot::macros::BinProtWrite;

pub const MAX_CONFIG_BYTES: usize = 8192;
pub const MAX_EVENT_BYTES: usize = 1024;
pub const MAX_SUBSCRIPTIONS: usize = 13;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Kind {
    Click,
    AuxiliaryClick,
    MouseDown,
    MouseUp,
    MouseMove,
    MouseEnter,
    MouseLeave,
    MouseDownOutside,
    KeyDown,
    KeyUp,
    Focus,
    Blur,
    Scroll,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Phase {
    Capture,
    Bubble,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Policy {
    Observe,
    StopPropagation,
    PreventDefault,
    PreventAndStop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Subscription {
    pub kind: Kind,
    pub phase: Phase,
    pub policy: Policy,
}
impl Subscription {
    pub fn is_valid(&self) -> bool {
        match self.kind {
            Kind::MouseDown
            | Kind::MouseUp
            | Kind::MouseMove
            | Kind::KeyDown
            | Kind::KeyUp
            | Kind::Scroll => true,
            Kind::Click
            | Kind::AuxiliaryClick
            | Kind::MouseEnter
            | Kind::MouseLeave
            | Kind::MouseDownOutside
            | Kind::Focus
            | Kind::Blur => self.phase == Phase::Bubble && self.policy == Policy::Observe,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Focus {
    None,
    Click,
    Tab,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub disabled: bool,
    pub focus: Focus,
    pub subscriptions: Vec<Subscription>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        CommandConfig::valid_text(&self.label, 4096)
            && !self.subscriptions.is_empty()
            && self.subscriptions.len() <= MAX_SUBSCRIPTIONS
            && self.subscriptions.iter().all(Subscription::is_valid)
            && self.subscriptions.windows(2).all(|w| w[0].kind < w[1].kind)
            && (self.focus != Focus::None
                || self
                    .subscriptions
                    .iter()
                    .all(|s| !matches!(s.kind, Kind::Focus | Kind::Blur)))
    }
    pub fn subscription(&self, kind: Kind) -> Option<Subscription> {
        self.subscriptions.iter().find(|s| s.kind == kind).copied()
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.label.len()
            + self.subscriptions.len() * std::mem::size_of::<Subscription>()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}
impl Position {
    pub fn is_valid(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Location {
    pub window: Position,
    pub local: Position,
    pub modifiers: PointerModifiers,
}
impl Location {
    pub fn is_valid(&self) -> bool {
        self.window.is_valid() && self.local.is_valid()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Mouse {
    pub location: Location,
    pub button: PointerButton,
    pub click_count: i64,
}
impl Mouse {
    pub fn is_valid(&self) -> bool {
        self.location.is_valid() && (1..=u32::MAX as i64).contains(&self.click_count)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Motion {
    pub location: Location,
    pub pressed_button: Option<PointerButton>,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Key {
    pub key: String,
    pub character: Option<String>,
    pub modifiers: PointerModifiers,
}
impl Key {
    pub fn is_valid(&self) -> bool {
        fn valid(s: &str) -> bool {
            !s.is_empty() && s.len() <= 256 && !s.contains('\0')
        }
        valid(&self.key) && self.character.as_ref().is_none_or(|s| valid(s))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum Delta {
    Pixels(Position),
    Lines(Position),
}
impl Delta {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Pixels(p) | Self::Lines(p) => p.is_valid(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Scroll {
    pub location: Location,
    pub delta: Delta,
    pub phase: TouchPhase,
}
impl Scroll {
    pub fn is_valid(&self) -> bool {
        self.location.is_valid() && self.delta.is_valid()
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Event {
    Click(Mouse),
    AuxiliaryClick(Mouse),
    MouseDown(Mouse),
    MouseUp(Mouse),
    MouseMove(Motion),
    MouseEnter,
    MouseLeave,
    MouseDownOutside(Mouse),
    KeyDown(Key, bool),
    KeyUp(Key),
    Focus,
    Blur,
    Scroll(Scroll),
}
impl Event {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Click(_) => Kind::Click,
            Self::AuxiliaryClick(_) => Kind::AuxiliaryClick,
            Self::MouseDown(_) => Kind::MouseDown,
            Self::MouseUp(_) => Kind::MouseUp,
            Self::MouseMove(_) => Kind::MouseMove,
            Self::MouseEnter => Kind::MouseEnter,
            Self::MouseLeave => Kind::MouseLeave,
            Self::MouseDownOutside(_) => Kind::MouseDownOutside,
            Self::KeyDown(..) => Kind::KeyDown,
            Self::KeyUp(_) => Kind::KeyUp,
            Self::Focus => Kind::Focus,
            Self::Blur => Kind::Blur,
            Self::Scroll(_) => Kind::Scroll,
        }
    }
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Click(mouse) => mouse.is_valid() && mouse.button == PointerButton::Left,
            Self::AuxiliaryClick(mouse) => mouse.is_valid() && mouse.button != PointerButton::Left,
            Self::MouseDown(mouse) | Self::MouseUp(mouse) | Self::MouseDownOutside(mouse) => {
                mouse.is_valid()
            }
            Self::MouseMove(motion) => motion.location.is_valid(),
            Self::KeyDown(key, _) | Self::KeyUp(key) => key.is_valid(),
            Self::Scroll(scroll) => scroll.is_valid(),
            Self::MouseEnter | Self::MouseLeave | Self::Focus | Self::Blur => true,
        }
    }
    pub fn payload_bytes(&self) -> usize {
        match self {
            Self::KeyDown(key, _) | Self::KeyUp(key) => {
                key.key.len() + key.character.as_ref().map_or(0, String::len)
            }
            _ => 0,
        }
    }
}
