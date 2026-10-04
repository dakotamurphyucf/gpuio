//! Checked adapters to the pinned reader. Hosts still own installation, quotas,
//! source/event fencing and UI failure reporting.
use crate::gpui::{
    self, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
};
use crate::{
    Error, Highlight, MAX_CODE_BYTES, MAX_RUNS, PluginRegistration, PrepareContext, Profile,
    RenderContext, contain,
};
use gpui_base::text::{
    InlineElement, InlineRenderContext, MarkdownExtensions, MarkdownNode, MarkdownParseContext,
    MarkdownPlugin, MarkdownPresentation, PreparedMarkdown, markdown_ast,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// Bounds generated public strings during one parse. Arbitrary opaque native
/// allocations still need honest author declarations and host reservations.
pub const MAX_GENERATED_BYTES: usize = 1_048_576;
#[derive(Default)]
struct State {
    error: Option<Error>,
    generated_bytes: usize,
}
#[derive(Clone, Default)]
pub struct Failures(Arc<Mutex<State>>);
impl Failures {
    /// First failure only; adapters never silently convert a failed hook into a
    /// successful declined match. Hosts check this before accepting preparation.
    pub fn check(&self) -> Result<(), Error> {
        self.0
            .lock()
            .map_err(|_| Error::Panicked)?
            .error
            .map_or(Ok(()), Err)
    }
    fn generated_bytes(&self) -> Result<usize, Error> {
        self.0
            .lock()
            .map(|state| state.generated_bytes)
            .map_err(|_| Error::Panicked)
    }
    fn record(&self, error: Error) {
        if let Ok(mut state) = self.0.lock() {
            state.error.get_or_insert(error);
        }
    }
    fn run<T>(&self, f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        self.check()?;
        let result = contain(f);
        if let Err(error) = &result {
            self.record(*error);
        }
        result
    }
    fn charge(&self, bytes: usize) -> Result<(), Error> {
        let mut state = self.0.lock().map_err(|_| Error::Panicked)?;
        let next = state
            .generated_bytes
            .checked_add(bytes)
            .filter(|n| *n <= MAX_GENERATED_BYTES)
            .ok_or(Error::LimitExceeded)?;
        state.generated_bytes = next;
        Ok(())
    }
}

pub struct AdaptedExtensions {
    pub extensions: MarkdownExtensions,
    pub failures: Failures,
}
struct Adapter {
    registration: PluginRegistration,
    failures: Failures,
    cancelled: Arc<AtomicBool>,
    render: Option<RenderContext>,
}
impl MarkdownPlugin for Adapter {
    fn name(&self) -> &str {
        self.registration.name()
    }
    fn is_block(&self) -> bool {
        self.registration.is_block()
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        self.failures
            .run(|| {
                let cancelled = || self.cancelled.load(Ordering::Acquire);
                let work = PrepareContext::new(&cancelled);
                work.check()?;
                let result = self.registration.plugin().parse(node, cx, &work)?;
                work.check()?;
                if let Some(node) = &result {
                    if node.name() != self.name() {
                        return Err(Error::InvalidPlugin);
                    }
                    let mut bytes = 0usize;
                    for text in [
                        node.as_text(),
                        node.as_markdown(),
                        node.accessibility_name(),
                    ] {
                        if text.len() > MAX_CODE_BYTES {
                            return Err(Error::LimitExceeded);
                        }
                        bytes = bytes.checked_add(text.len()).ok_or(Error::LimitExceeded)?;
                    }
                    self.failures.charge(bytes)?;
                }
                Ok(result)
            })
            .ok()
            .flatten()
    }
    fn presentation(&self, node: &MarkdownNode) -> MarkdownPresentation {
        self.failures.run(|| {
            let presentation = self.registration.plugin().presentation(node)?;
            if matches!(&presentation, MarkdownPresentation::Text(text) if text.len() > MAX_CODE_BYTES) { return Err(Error::LimitExceeded); }
            Ok(presentation)
        }).unwrap_or_else(|error| {
            if let Some(render) = &self.render { render.report_failure(error); }
            MarkdownPresentation::Opaque
        })
    }
    fn render(
        &self,
        node: &MarkdownNode,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> impl IntoElement {
        self.failures
            .run(|| {
                self.registration.plugin().render(
                    node,
                    self.render.as_ref().ok_or(Error::Render)?,
                    window,
                    cx,
                )
            })
            .unwrap_or_else(|error| {
                if let Some(render) = &self.render {
                    render.report_failure(error);
                }
                gpui::div()
                    .id("document-extension-error")
                    .role(gpui::Role::Alert)
                    .child("Document extension unavailable")
                    .into_any_element()
            })
    }
    fn render_inline(
        &self,
        node: &MarkdownNode,
        inline: &InlineRenderContext,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Option<InlineElement> {
        match self.failures.run(|| {
            self.registration.plugin().render_inline(
                node,
                inline,
                self.render.as_ref().ok_or(Error::Render)?,
                window,
                cx,
            )
        }) {
            Ok(element) => element,
            Err(error) => {
                if let Some(render) = &self.render {
                    render.report_failure(error);
                }
                Some(InlineElement::new(
                    gpui::div()
                        .id("document-extension-error")
                        .role(gpui::Role::Alert)
                        .child("Document extension unavailable"),
                ))
            }
        }
    }
}
impl Profile {
    /// Append profile plugins after host safety adapters. The host supplies the
    /// stable, positive parser epoch; rebuilding render closures must preserve it.
    /// Passing None is for worker parsing only; UI rendering needs a live context.
    pub fn extensions(
        &self,
        base: MarkdownExtensions,
        parser_epoch: u64,
        cancelled: Arc<AtomicBool>,
        render: Option<RenderContext>,
    ) -> Result<AdaptedExtensions, Error> {
        if parser_epoch == 0 {
            return Err(Error::InvalidProperties);
        }
        let failures = Failures::default();
        let mut extensions = base;
        for registration in self.plugins() {
            extensions = extensions.plugin(Adapter {
                registration: registration.clone(),
                failures: failures.clone(),
                cancelled: cancelled.clone(),
                render: render.clone(),
            });
        }
        Ok(AdaptedExtensions {
            extensions: extensions.parser_revision(parser_epoch),
            failures,
        })
    }
}

/// Worker request with no native UI capabilities. Host image/literal-syntax
/// adapters go in base; no implicit image fetching is introduced here.
pub struct Preparation<'a> {
    pub source: &'a str,
    pub base: MarkdownExtensions,
    pub parser_epoch: u64,
    pub dark: bool,
    pub cancelled: Arc<AtomicBool>,
}
/// Prepared results share a single profile and interpretation. Do not install
/// document and highlighter output from different requests.
pub struct Prepared {
    pub document: PreparedMarkdown,
    pub profile: Arc<Profile>,
    pub parser_epoch: u64,
    pub timings: PreparationTimings,
    /// Sum of validated public strings produced by plugins during this parse.
    pub generated_bytes: usize,
    /// Absent key uses the host highlighter; an empty present value means plain.
    pub highlights: BTreeMap<(String, String), Vec<Highlight>>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct PreparationTimings {
    pub parse: std::time::Duration,
    pub highlight: std::time::Duration,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationStage {
    Parse,
    Highlight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparationFailure {
    pub stage: PreparationStage,
    pub error: Error,
}
impl PreparationFailure {
    fn parse(error: Error) -> Self {
        Self {
            stage: PreparationStage::Parse,
            error,
        }
    }
    fn highlight(error: Error) -> Self {
        Self {
            stage: PreparationStage::Highlight,
            error,
        }
    }
}
pub fn prepare_markdown(
    profile: Arc<Profile>,
    request: Preparation<'_>,
) -> Result<Prepared, PreparationFailure> {
    prepare(profile, request, PreparedMarkdown::parse)
}

/// HTML uses the host's image adapter for every image. Markdown AST plugins do
/// not intercept HTML; profile code/table rendering and highlighting still apply.
pub fn prepare_html(
    profile: Arc<Profile>,
    request: Preparation<'_>,
    image: impl FnMut(&gpui_base::text::HtmlImage) -> MarkdownNode,
) -> Result<Prepared, PreparationFailure> {
    prepare(profile, request, |source, extensions| {
        PreparedMarkdown::parse_html(source, extensions, image)
    })
}

fn prepare(
    profile: Arc<Profile>,
    request: Preparation<'_>,
    parse: impl FnOnce(&str, MarkdownExtensions) -> Result<PreparedMarkdown, gpui::SharedString>,
) -> Result<Prepared, PreparationFailure> {
    let started = std::time::Instant::now();
    let cancelled = || request.cancelled.load(Ordering::Acquire);
    let work = PrepareContext::new(&cancelled);
    work.check().map_err(PreparationFailure::parse)?;
    if request.source.len() > MAX_CODE_BYTES {
        return Err(PreparationFailure::parse(Error::LimitExceeded));
    }
    let adapted = profile
        .extensions(
            request.base,
            request.parser_epoch,
            request.cancelled.clone(),
            None,
        )
        .map_err(PreparationFailure::parse)?;
    // Base creates a bounded AST and bounded displayed-text projection; plugin
    // errors are additionally checked so a failed matcher cannot erase content.
    let parsed = contain(|| parse(request.source, adapted.extensions).map_err(|_| Error::Parse));
    adapted
        .failures
        .check()
        .map_err(PreparationFailure::parse)?;
    work.check().map_err(PreparationFailure::parse)?;
    let document = parsed.map_err(PreparationFailure::parse)?;
    let parse_time = started.elapsed();
    let started = std::time::Instant::now();
    let mut highlights = BTreeMap::new();
    let mut count = 0usize;
    for block in document.code_blocks() {
        work.check().map_err(PreparationFailure::highlight)?;
        let language = block.lang().map(|s| s.to_string());
        let text = block.code();
        let code = crate::Code::new(&text, language.as_deref(), request.dark)
            .map_err(PreparationFailure::highlight)?;
        if let Some(styles) = profile
            .highlight(&code, &work)
            .map_err(PreparationFailure::highlight)?
        {
            count = count
                .checked_add(styles.len())
                .filter(|n| *n <= MAX_RUNS)
                .ok_or_else(|| PreparationFailure::highlight(Error::LimitExceeded))?;
            highlights.insert(
                (language.unwrap_or_else(|| "txt".into()), text.to_string()),
                styles,
            );
        }
    }
    work.check().map_err(PreparationFailure::highlight)?;
    Ok(Prepared {
        document,
        profile,
        parser_epoch: request.parser_epoch,
        generated_bytes: adapted
            .failures
            .generated_bytes()
            .map_err(PreparationFailure::parse)?,
        timings: PreparationTimings {
            parse: parse_time,
            highlight: started.elapsed(),
        },
        highlights,
    })
}
