use crate::{
    Code, Error, EventSink, Highlight, Highlighter, MAX_PLUGINS, PrepareContext, contain, gpui,
    qualified_name,
};
use crate::{
    CodeBlock, InlineElement, InlineRenderContext, MarkdownNode, MarkdownParseContext,
    MarkdownPresentation, TableData, markdown_ast,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    generation: i64,
    revision: i64,
}
impl Source {
    pub fn new(generation: i64, revision: i64) -> Result<Self, Error> {
        if generation <= 0 || revision <= 0 {
            return Err(Error::InvalidSource);
        }
        Ok(Self {
            generation,
            revision,
        })
    }
    pub fn generation(self) -> i64 {
        self.generation
    }
    pub fn revision(self) -> i64 {
        self.revision
    }
}

/// Installed-picture provenance plus a host-owned revocable event sink. The host
/// must revoke/fence source, property, visibility, modal and unmount transitions.
/// Callback authors use the sink's guard/guard_pointer helpers for panic containment.
#[derive(Clone)]
pub struct RenderContext {
    source: Source,
    events: EventSink,
    failure: Option<Arc<dyn Fn(Error) + Send + Sync>>,
}
impl RenderContext {
    pub fn new(source: Source, events: EventSink) -> Self {
        Self {
            source,
            events,
            failure: None,
        }
    }
    /// Host-only queued failure reporting; the callback must not reenter GPUI.
    pub fn with_failure_reporter(mut self, report: impl Fn(Error) + Send + Sync + 'static) -> Self {
        self.failure = Some(Arc::new(report));
        self
    }
    pub(crate) fn report_failure(&self, error: Error) {
        if let Some(report) = &self.failure {
            let _ = contain(|| {
                report(error);
                Ok(())
            });
        }
    }
    pub fn source(&self) -> Source {
        self.source
    }
    pub fn events(&self) -> &EventSink {
        &self.events
    }
}

/// Native slots below a code block or table. None preserves declarative/default
/// actions; Some replaces that slot. Include any desired Copy action explicitly.
pub trait ActionRenderer: Send + Sync + 'static {
    fn code(
        &self,
        _block: &CodeBlock,
        _context: &RenderContext,
        _window: &mut gpui::Window,
        _cx: &mut gpui::App,
    ) -> Result<Option<gpui::AnyElement>, Error> {
        Ok(None)
    }
    fn table(
        &self,
        _table: &TableData,
        _context: &RenderContext,
        _window: &mut gpui::Window,
        _cx: &mut gpui::App,
    ) -> Result<Option<gpui::AnyElement>, Error> {
        Ok(None)
    }
}

/// Worker parsing/presentation is distinct from UI rendering. All retained data
/// must be immutable/thread-safe; no callbacks into OCaml from any hook.
pub trait Plugin: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn is_block(&self) -> bool;
    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
        work: &PrepareContext<'_>,
    ) -> Result<Option<MarkdownNode>, Error>;
    /// Explicit, truthful text/search/selection contract; no implicit Text claim.
    fn presentation(&self, node: &MarkdownNode) -> Result<MarkdownPresentation, Error>;
    fn render(
        &self,
        node: &MarkdownNode,
        context: &RenderContext,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Result<gpui::AnyElement, Error>;
    fn render_inline(
        &self,
        node: &MarkdownNode,
        inline: &InlineRenderContext,
        context: &RenderContext,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Result<Option<InlineElement>, Error>;
}

/// Metadata is captured once. A stateful plugin cannot change parser identity
/// between registration and rendering by returning a different name later.
#[derive(Clone)]
pub struct PluginRegistration {
    name: &'static str,
    is_block: bool,
    plugin: Arc<dyn Plugin>,
}
impl PluginRegistration {
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn is_block(&self) -> bool {
        self.is_block
    }
    pub fn plugin(&self) -> &Arc<dyn Plugin> {
        &self.plugin
    }
}

/// Immutable configuration produced on a worker for one checked property value.
/// Host adapters still enforce aggregate parsed/retained bytes and hook boundaries.
#[derive(Clone, Default)]
pub struct Profile {
    highlighter: Option<Arc<dyn Highlighter>>,
    actions: Option<Arc<dyn ActionRenderer>>,
    plugins: Vec<PluginRegistration>,
}
impl Profile {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_highlighter(mut self, highlighter: Arc<dyn Highlighter>) -> Self {
        self.highlighter = Some(highlighter);
        self
    }
    pub fn with_actions(mut self, actions: Arc<dyn ActionRenderer>) -> Self {
        self.actions = Some(actions);
        self
    }
    pub fn with_plugin(mut self, plugin: Arc<dyn Plugin>) -> Result<Self, Error> {
        if self.plugins.len() >= MAX_PLUGINS {
            return Err(Error::LimitExceeded);
        }
        let (name, is_block) = contain(|| Ok((plugin.name(), plugin.is_block())))?;
        if !qualified_name(name) || name.starts_with("gpuio.") {
            return Err(Error::InvalidPlugin);
        }
        if self.plugins.iter().any(|p| p.name == name) {
            return Err(Error::DuplicatePlugin);
        }
        self.plugins.push(PluginRegistration {
            name,
            is_block,
            plugin,
        });
        Ok(self)
    }
    pub fn plugins(&self) -> &[PluginRegistration] {
        &self.plugins
    }
    pub fn actions(&self) -> Option<&Arc<dyn ActionRenderer>> {
        self.actions.as_ref()
    }
    /// None uses the native highlighter. Invalid results return no accepted prefix.
    pub fn highlight(
        &self,
        code: &Code<'_>,
        cx: &PrepareContext<'_>,
    ) -> Result<Option<Vec<Highlight>>, Error> {
        cx.check()?;
        self.highlighter
            .as_ref()
            .map(|h| crate::highlight::run(h.as_ref(), code, cx))
            .transpose()
    }
}
