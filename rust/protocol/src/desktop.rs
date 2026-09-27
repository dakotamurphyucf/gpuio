//! Owned application desktop requests. Paths preserve Unix bytes. These types
//! carry no OS handles and perform no registration, activation or file I/O.
use crate::file_path::FilePath;
use binprot::macros::BinProtWrite;

pub const MAX_IDENTIFIER_BYTES: usize = 128;
pub const MAX_NAME_BYTES: usize = 256;
pub const MAX_SCHEME_BYTES: usize = 64;
pub const MAX_SCHEMES: usize = 16;
pub const MAX_LINK_BYTES: usize = 16_384;
pub const MAX_LINKS: usize = 64;
pub const MAX_LINK_BATCH_BYTES: usize = 262_144;

pub fn valid_scheme(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_SCHEME_BYTES
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"+-.".contains(&c))
}

pub fn valid_identifier(s: &str) -> bool {
    s.len() <= MAX_IDENTIFIER_BYTES
        && s.contains('.')
        && s.split('.').all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_lowercase()
                && !part.ends_with('-')
                && part
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        })
}

pub fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_NAME_BYTES
        && s.bytes().all(|c| c >= 32 && c != 127)
        && s.bytes().any(|c| c != b' ')
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Identity {
    pub identifier: String,
    pub name: String,
    pub schemes: Vec<String>,
}
impl Identity {
    pub fn is_valid(&self) -> bool {
        valid_identifier(&self.identifier)
            && valid_name(&self.name)
            && self.schemes.len() <= MAX_SCHEMES
            && self
                .schemes
                .iter()
                .enumerate()
                .all(|(i, s)| valid_scheme(s) && !self.schemes[..i].contains(s))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Capabilities {
    pub incoming_links: bool,
    pub runtime_registration: bool,
    pub application_activation: bool,
    pub file_reveal: bool,
    pub file_open: bool,
    pub document_metadata: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    InvalidRequest,
    NotReady,
    AlreadyConfigured,
    Unsupported,
    Unavailable,
    Denied,
    Busy,
    Closed,
    NativeFailure,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Configure(Identity),
    Capabilities,
    TakeLinks,
    Activate(bool),
    RevealFile(FilePath),
    OpenFile(FilePath),
    RegisterScheme(String),
}
impl Request {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Configure(identity) => identity.is_valid(),
            Self::RegisterScheme(scheme) => valid_scheme(scheme),
            Self::Capabilities
            | Self::TakeLinks
            | Self::Activate(_)
            | Self::RevealFile(_)
            | Self::OpenFile(_) => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct LinkBatch {
    pub links: Vec<String>,
    /// Saturating count of rejected incoming links since the previous take.
    pub dropped: i64,
}
impl LinkBatch {
    pub fn is_valid(&self) -> bool {
        self.dropped >= 0
            && self.links.len() <= MAX_LINKS
            && self.links.iter().all(|s| s.len() <= MAX_LINK_BYTES)
            && self.links.iter().map(String::len).sum::<usize>() <= MAX_LINK_BATCH_BYTES
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Response {
    Configured,
    Capabilities(Capabilities),
    Links(LinkBatch),
    /// The platform accepted the request, not a presentation/completion receipt.
    Requested,
    Registered,
    Failed(Error),
}
impl Response {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Links(batch) => batch.is_valid(),
            _ => true,
        }
    }
}
