//! OS presentation qualification. The only supported production backend is the
//! pinned macOS Metal renderer; unsupported backends never fabricate samples.
pub use gpui::presentation::{Counts, LatencyBounds, Limits, Outcome, Record, Snapshot};

/// Starting diagnostics can fail without changing the window or another session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartError {
    Unsupported,
    Collector(gpui::presentation::StartError),
}

/// Bounded measurements only; no window, entity or native renderer ownership.
pub struct Session(gpui::presentation::Session);

impl Session {
    /// Start on a native macOS Metal window. TestPlatform and other operating
    /// systems report Unsupported. Starting does not request or schedule a frame.
    pub fn start(window: &gpui::Window, limits: Limits) -> Result<Self, StartError> {
        if !supported(window) {
            return Err(StartError::Unsupported);
        }
        window
            .start_presentation(limits)
            .map(Self)
            .map_err(StartError::Collector)
    }

    pub fn snapshot(&self) -> Snapshot {
        self.0.snapshot()
    }

    /// Stop admission; existing callbacks can settle. A caller must impose a
    /// deadline and report remaining pending frames rather than waiting forever.
    pub fn stop(&self) {
        self.0.stop();
    }
}

#[cfg(target_os = "macos")]
fn supported(window: &gpui::Window) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    HasWindowHandle::window_handle(window)
        .is_ok_and(|handle| matches!(handle.as_raw(), RawWindowHandle::AppKit(_)))
}

#[cfg(not(target_os = "macos"))]
fn supported(_: &gpui::Window) -> bool {
    false
}
