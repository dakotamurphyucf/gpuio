//! Owned wire data. No GPUI, OCaml runtime, or I/O scheduler dependencies.
//!
//! V1 is under development; the private foundation protocol is unrelated.
pub mod animation;
pub mod asset;
mod command;
mod decode;
pub mod document;
pub mod drag_drop;
pub mod file_dialog;
pub mod file_path;
mod id;
pub mod image;
pub mod list;
mod menu;
mod palette;
pub mod v1;

pub use decode::{DecodeError, decode};
pub use id::{HandlerId, NodeId, ResourceId, WindowId};

pub mod progress;

pub mod toast;

pub mod pointer;

pub mod window;
