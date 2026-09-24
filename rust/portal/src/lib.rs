//! Owned XDG file-chooser requests. No GPUI/display dependency: protocol and
//! cancellation behavior can be tested on either supported OS. Window parenting
//! and retaining exported Wayland handles belong to the native adapter.
mod request;
mod response;
pub use request::{choose, version};

mod motion;
pub use motion::watch_motion;
