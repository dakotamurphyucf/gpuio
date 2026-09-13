//! Owned wire data. No GPUI, OCaml runtime, or I/O scheduler dependencies.
//!
//! V1 is under development; the private foundation protocol is unrelated.
mod command;
mod decode;
mod id;
pub mod v1;

pub use decode::{DecodeError, decode};
pub use id::{HandlerId, NodeId, ResourceId, WindowId};
