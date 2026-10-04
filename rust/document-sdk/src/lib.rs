//! Statically linked rich-document authoring contracts. Trusted Rust, not a sandbox.
//! Host/protocol attachment is separate from constructing these definitions.
/// Native primitives from the exact Base revision used by the reader.
pub use gpui_base as base;
pub use gpui_base::text::{
    CodeBlock, InlineElement, InlineRenderContext, MarkdownNode, MarkdownParseContext,
    MarkdownPresentation, TableData, markdown_ast,
};
pub use gpuio_extension_sdk::{EventLease, EventSink, gpui};
use std::panic::{AssertUnwindSafe, catch_unwind};

mod highlight;
mod preparation;
mod profile;
mod registry;
pub use highlight::{Code, Highlight, Highlighter};
pub use preparation::{
    AdaptedExtensions, Failures, MAX_GENERATED_BYTES, Preparation, PreparationFailure,
    PreparationStage, PreparationTimings, Prepared, prepare_html, prepare_markdown,
};
pub use profile::{ActionRenderer, Plugin, PluginRegistration, Profile, RenderContext, Source};
pub use registry::{Binding, Descriptor, Factory, Registry};

pub const SDK_VERSION: u32 = 1;
pub const GPUI_REVISION: &str = gpuio_extension_sdk::GPUI_REVISION;
pub const BASE_REVISION: &str = "84f57fdfcb4910623fb0bb7f795b077e249f9271";
pub const MAX_PROFILES: usize = 64;
pub const MAX_PLUGINS: usize = 32;
pub const MAX_PROPERTIES: usize = 65_536;
pub const MAX_EVENT: usize = 16_384;
pub const MAX_CODE_BYTES: usize = 65_536;
pub const MAX_RETAINED_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_RUNS: usize = 32_768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// Contains ordinary unwinding Rust panics, not aborts or unsafe memory corruption.
pub fn contain<T>(f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(Err(Error::Panicked))
}

/// Worker capability: no Window/App, I/O handles or event delivery. Authors check
/// this between bounded units of work; arbitrary Rust cannot be forcibly stopped.
pub struct PrepareContext<'a> {
    cancelled: &'a (dyn Fn() -> bool + Sync),
}
impl<'a> PrepareContext<'a> {
    pub fn new(cancelled: &'a (dyn Fn() -> bool + Sync)) -> Self {
        Self { cancelled }
    }
    pub fn check(&self) -> Result<(), Error> {
        contain(|| {
            if (self.cancelled)() {
                Err(Error::Cancelled)
            } else {
                Ok(())
            }
        })
    }
}

fn qualified_name(name: &str) -> bool {
    name.len() <= 128
        && name.split('.').count() >= 2
        && name.split('.').all(|segment| {
            segment
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_lowercase)
                && segment
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        })
}
