//! Owned OS notification values; no native handles, permission prompts or I/O.
use binprot::macros::BinProtWrite;
pub const MAX_TAG_BYTES: usize = 128;
pub const MAX_ACTION_ID_BYTES: usize = 64;
pub const MAX_LABEL_BYTES: usize = 128;
pub const MAX_TITLE_BYTES: usize = 256;
pub const MAX_BODY_BYTES: usize = 8192;
pub const MAX_ACTIONS: usize = 4;
pub const MAX_LIVE: usize = 128;
pub const MAX_PENDING: usize = 16;
pub const MAX_EVENTS: usize = MAX_LIVE + 1;
fn text(s: &str, limit: usize, multiline: bool, allow_empty: bool) -> bool {
    s.len() <= limit
        && (allow_empty || s.bytes().any(|c| c != b' '))
        && s.bytes()
            .all(|c| (c >= 32 && c != 127) || (multiline && b"\r\n\t".contains(&c)))
}
pub fn valid_tag(s: &str) -> bool {
    text(s, MAX_TAG_BYTES, false, false)
}
pub fn valid_action_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_ACTION_ID_BYTES
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Action {
    pub id: String,
    pub label: String,
}
impl Action {
    pub fn is_valid(&self) -> bool {
        valid_action_id(&self.id) && text(&self.label, MAX_LABEL_BYTES, false, false)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Sound {
    Silent,
    Default,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Content {
    pub title: String,
    pub body: String,
    pub actions: Vec<Action>,
    pub sound: Sound,
}
impl Content {
    pub fn is_valid(&self) -> bool {
        text(&self.title, MAX_TITLE_BYTES, false, false)
            && text(&self.body, MAX_BODY_BYTES, true, true)
            && self.actions.len() <= MAX_ACTIONS
            && self
                .actions
                .iter()
                .enumerate()
                .all(|(i, a)| a.is_valid() && !self.actions[..i].iter().any(|old| old.id == a.id))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub struct Receipt {
    pub id: i64,
    pub tag: String,
}
impl Receipt {
    pub fn is_valid(&self) -> bool {
        self.id > 0 && valid_tag(&self.tag)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Authorization {
    NotDetermined,
    Denied,
    Authorized,
    Provisional,
    NotRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Capabilities {
    pub body: bool,
    pub actions: bool,
    pub activation: bool,
    pub replacement: bool,
    pub dismissal: bool,
    pub permission_request: bool,
    pub sound: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    InvalidRequest,
    NotReady,
    Unsupported,
    Unavailable,
    Denied,
    Busy,
    Closed,
    Stale,
    NativeFailure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ClosedReason {
    Expired,
    User,
    Platform,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Event {
    Activated(Receipt),
    Action(Receipt, String),
    Closed(Receipt, ClosedReason),
    Failed(Error),
}
impl Event {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Activated(r) | Self::Closed(r, _) => r.is_valid(),
            Self::Action(r, id) => r.is_valid() && valid_action_id(id),
            Self::Failed(_) => true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Capabilities,
    Authorization,
    RequestAuthorization,
    Post(String, Content),
    Replace(Receipt, Content),
    Dismiss(Receipt),
    TakeEvents,
    Close,
}
impl Request {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Post(tag, c) => valid_tag(tag) && c.is_valid(),
            Self::Replace(r, c) => r.is_valid() && c.is_valid(),
            Self::Dismiss(r) => r.is_valid(),
            Self::Capabilities
            | Self::Authorization
            | Self::RequestAuthorization
            | Self::TakeEvents
            | Self::Close => true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Response {
    Capabilities(Capabilities),
    Authorization(Authorization),
    Posted(Receipt),
    Replaced,
    DismissRequested,
    Events(Vec<Event>),
    Closed,
    Failed(Error),
}
impl Response {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Posted(r) => r.is_valid(),
            Self::Events(e) => e.len() <= MAX_EVENTS && e.iter().all(Event::is_valid),
            Self::Capabilities(_)
            | Self::Authorization(_)
            | Self::Replaced
            | Self::DismissRequested
            | Self::Closed
            | Self::Failed(_) => true,
        }
    }
}
