//! Static rich-document profile attachment and stamped asynchronous observations.
use crate::extension::{MAX_MESSAGE, MAX_PROPERTIES, Payload, Schema};
use binprot::macros::BinProtWrite;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Instance {
    pub schema: Schema,
    pub generation: i64,
    pub properties: Payload,
}
impl Instance {
    pub fn is_valid(&self) -> bool {
        self.schema.is_valid() && self.generation > 0 && self.properties.0.len() <= MAX_PROPERTIES
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.schema.name.len()
            + self.schema.fingerprint.len()
            + self.properties.0.len()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub epoch: i64,
    pub instance: Option<Instance>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.epoch > 0 && self.instance.as_ref().is_none_or(Instance::is_valid)
    }
    pub fn retained_bytes(&self) -> usize {
        256 + std::mem::size_of::<Self>()
            + self.instance.as_ref().map_or(0, Instance::retained_bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Stage {
    Configure,
    Parse,
    Highlight,
    Render,
    Input,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    InvalidSchema,
    IncompatibleSdk,
    DuplicateProfile,
    UnknownProfile,
    IncompatibleSchema,
    InvalidProperties,
    InvalidPlugin,
    DuplicatePlugin,
    InvalidHighlight,
    InvalidSource,
    Parse,
    Render,
    Highlight,
    LimitExceeded,
    Cancelled,
    Panicked,
    Closed,
    InvalidEvent,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Signal {
    Data(Payload),
    Failed(Stage, Error),
}
impl Signal {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Data(bytes) => bytes.0.len() <= MAX_MESSAGE,
            Self::Failed(..) => true,
        }
    }
    pub fn payload_bytes(&self) -> usize {
        128 + match self {
            Self::Data(bytes) => bytes.0.len(),
            Self::Failed(..) => 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Event {
    pub config_epoch: i64,
    pub instance_generation: i64,
    pub source_revision: i64,
    pub source_generation: i64,
    pub signal: Signal,
}
impl Event {
    pub fn is_valid(&self) -> bool {
        self.config_epoch > 0
            && self.instance_generation > 0
            && self.source_revision > 0
            && self.source_generation > 0
            && self.signal.is_valid()
    }
    pub fn payload_bytes(&self) -> usize {
        128 + self.signal.payload_bytes()
    }
}
