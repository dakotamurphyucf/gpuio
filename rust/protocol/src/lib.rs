//! Owned wire data. No GPUI, OCaml runtime, or I/O scheduler dependencies.
//!
//! V1 is under development; the private foundation protocol is unrelated.
mod command;
mod decode;
pub mod drag_drop;
pub mod file_dialog;
pub mod file_path;
mod id;
mod menu;
mod palette;
pub mod v1;

pub use decode::{DecodeError, decode};
pub use id::{HandlerId, NodeId, ResourceId, WindowId};

pub mod progress;

pub mod toast;

pub mod pointer;
