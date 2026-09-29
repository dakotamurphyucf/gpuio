//! Mounted source leases and disposable native document presentations.
use super::{Interaction, View, apply_styles};
use crate::{
    document_diff_controls::Controls,
    document_diff_projection::Projection,
    document_editor, document_host,
    document_jobs::{self, Prepared},
    document_markdown,
    document_store::{Lease, Snapshot},
    tree::{Node, Tree},
};
use gpui::{
    App, Context, Entity, Focusable, IntoElement, Render, WeakEntity, Window, div, prelude::*, px,
};
use gpui_base::input::EditorState;
use gpui_base::{Editor, TextView, TextViewState};
use gpuio_protocol::{
    NodeId, ResourceId,
    document::{Config, Layout, Navigation},
    v1::*,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

pub(super) struct State {
    source: Option<ResourceId>,
    lease: Option<Lease>,
    image_sources: Vec<(String, ImageSource)>,
    images: BTreeMap<String, crate::image_host::Handle>,
    pub(super) presentation: Option<Entity<Presentation>>,
}
struct Page {
    text: String,
    end: usize,
}
fn page(snapshot: &Snapshot, start: usize) -> Page {
    let start = snapshot
        .text
        .floor_char_boundary(start.min(snapshot.text.len()));
    let limit = snapshot
        .text
        .floor_char_boundary((start + 65536).min(snapshot.text.len()));
    let text = snapshot.text.slice(start..limit).to_string();
    bounded_page(&text, start)
}
fn bounded_page(text: &str, start: usize) -> Page {
    let mut end = text.len();
    let mut line_start = 0;
    for (index, (offset, c)) in text.char_indices().filter(|(_, c)| *c == '\n').enumerate() {
        let _ = c;
        if index >= 1023 || offset - line_start > 16384 {
            end = offset.min(line_start + 16384);
            break;
        }
        line_start = offset + 1;
    }
    if end - line_start > 16384 {
        end = (line_start + 16384).min(end);
    }
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Page {
        text: text[..end].to_string(),
        end: start + end,
    }
}

pub(super) struct Presentation {
    highlight_identity: Rc<RefCell<Rc<()>>>,
    highlight: Option<super::highlight::DocumentBinding>,
    highlight_paint: Option<(
        crate::highlight_paint::Paint,
        Rc<dyn gpui_base::input::RangeBackgrounds>,
    )>,
    markdown_highlight: Option<MarkdownHighlight>,
    buttons: BTreeMap<&'static str, gpui::FocusHandle>,
    root: WeakEntity<View>,
    node: NodeId,
    lease: Lease,
    config: Arc<Config>,
    selection_color: Option<gpui::Hsla>,
    user_selectable: bool,
    snapshot: Arc<Snapshot>,
    job: Result<document_host::Handle, document_jobs::Error>,
    markdown: Option<Entity<TextViewState>>,
    editor: Entity<EditorState>,
    code_highlighter: Option<Arc<gpui_base::text::CodeBlockHighlighterFn>>,
    search: crate::document_search::Matches,
    search_index: Option<usize>,
    runs: Arc<Vec<crate::document_highlight::Run>>,
    diff: Option<crate::document_diff::Diff>,
    diff_config: Option<Arc<gpuio_protocol::document_diff::Config>>,
    diff_epoch: i64,
    diff_handler: Option<gpuio_protocol::HandlerId>,
    #[cfg(feature = "native-tests")]
    diff_more_bounds: Rc<RefCell<Option<gpui::Bounds<gpui::Pixels>>>>,
    diff_controls: Option<Controls>,
    file_buttons: BTreeMap<usize, (Option<Arc<str>>, gpui::FocusHandle)>,
    file_visible: Rc<RefCell<BTreeMap<usize, gpui::Bounds<gpui::Pixels>>>>,
    #[cfg(feature = "native-tests")]
    file_suffixes: Rc<RefCell<BTreeMap<usize, file_headers::SuffixGeometry>>>,
    projection: Option<Projection>,
    projection_charge: Option<document_jobs::Charge>,
    projection_error: Option<String>,
    projected_page: Option<Arc<str>>,
    raw_diff: bool,
    charge: Option<document_jobs::Charge>,
    collapsed: bool,
    error: Option<String>,
    ready: bool,
    previous_pages: Vec<usize>,
    source_mode: bool,
    installed_page_start: usize,
    page_start: usize,
    page_end: usize,
    source_lines: usize,
    installed: Option<Arc<Snapshot>>,
    images: document_markdown::Images,
    markdown_extensions: gpui_base::text::MarkdownExtensions,
}
#[derive(Clone)]
struct DiffAction {
    snapshot: Arc<Snapshot>,
    page: Arc<str>,
    epoch: i64,
    handler: Option<gpuio_protocol::HandlerId>,
}
struct MarkdownHighlight {
    source: Arc<gpui_base::text::DisplayedText>,
    paints: Vec<Option<crate::highlight_paint::Paint>>,
    backgrounds: Rc<gpui_base::text::TextBackgrounds>,
}
impl Presentation {
    fn markdown_highlight_source(&self, cx: &App) -> Option<Arc<gpui_base::text::DisplayedText>> {
        if self.collapsed || self.source_mode || self.installed.is_none() {
            return None;
        }
        self.markdown.as_ref()?.read(cx).displayed_text()
    }

    fn highlight_groups(
        &self,
        cx: &App,
    ) -> Result<crate::highlight_collect::DocumentGroups, crate::highlight_collect::DocumentError>
    {
        if let Some(source) = self.markdown_highlight_source(cx) {
            // Arbitrary native objects need their own declared glyph projection.
            // Do not report a partially counted result as a complete document.
            if source.opaque_nodes() != 0 {
                return Err(crate::highlight_collect::DocumentError::Unavailable);
            }
            let installed = self.installed.as_ref().unwrap();
            return Ok(source
                .fragments()
                .iter()
                .map(|fragment| {
                    vec![crate::highlight_projection::Source::document_text(
                        installed.clone(),
                        fragment.text().clone(),
                    )]
                })
                .collect());
        }
        Ok(self
            .highlight_source()?
            .map(|source| vec![vec![source]])
            .unwrap_or_default())
    }
    fn highlight_source(
        &self,
    ) -> Result<Option<crate::highlight_projection::Source>, crate::highlight_collect::DocumentError>
    {
        use crate::highlight_collect::DocumentError;
        if self.collapsed {
            return Ok(None);
        }
        let installed = self.installed.as_ref().ok_or(DocumentError::Pending)?;
        if !self.source_mode {
            return Err(DocumentError::Unavailable);
        }
        if let Some(text) = &self.projected_page {
            return Ok(Some(crate::highlight_projection::Source::document_text(
                installed.clone(),
                text.clone(),
            )));
        }
        crate::highlight_projection::Source::document_slice(
            installed.clone(),
            self.installed_page_start..self.page_end,
        )
        .map(Some)
        .map_err(|_| DocumentError::Unavailable)
    }
    fn install_highlight(&mut self, cx: &mut Context<Self>) {
        let paint = self
            .highlight_source()
            .ok()
            .flatten()
            .and_then(|source| self.highlight.as_ref()?.paint(0, &source));
        let same = match (&self.highlight_paint, &paint) {
            (Some((old, _)), Some(new)) => old.same_paint(new),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.highlight_paint = paint.map(|paint| {
                let backgrounds = paint.editor_backgrounds();
                (paint, backgrounds)
            });
        }
        let backgrounds = self
            .highlight_paint
            .as_ref()
            .map(|(_, backgrounds)| backgrounds.clone());
        self.editor.update(cx, |editor, cx| {
            // The provider is fenced to this exact installed page above. Rejecting
            // malformed metadata clears old washes rather than decorating stale text.
            let _ = editor.set_range_backgrounds(backgrounds, cx);
        });
        let source = self.markdown_highlight_source(cx).filter(|source| {
            self.highlight.is_some() && source.opaque_nodes() == 0 && !source.fragments().is_empty()
        });
        if let Some(source) = source {
            let installed = self.installed.as_ref().unwrap();
            let paints: Vec<_> = source
                .fragments()
                .iter()
                .map(|fragment| {
                    self.highlight.as_ref()?.paint(
                        fragment.id(),
                        &crate::highlight_projection::Source::document_text(
                            installed.clone(),
                            fragment.text().clone(),
                        ),
                    )
                })
                .collect();
            let same = self.markdown_highlight.as_ref().is_some_and(|old| {
                Arc::ptr_eq(&old.source, &source)
                    && old.paints.len() == paints.len()
                    && old
                        .paints
                        .iter()
                        .zip(&paints)
                        .all(|(old, new)| match (old, new) {
                            (Some(a), Some(b)) => a.same_paint(b),
                            (None, None) => true,
                            _ => false,
                        })
            });
            if paints.iter().all(Option::is_none) {
                // No washes means no extra paint owner. Individual unpainted
                // fragments are normal; any actual wash retains the complete
                // admitted Ready/source projection through its provider.
                self.markdown_highlight = None;
            } else if !same {
                let layers = paints
                    .iter()
                    .map(|paint| paint.as_ref().map(|paint| paint.editor_backgrounds()))
                    .collect();
                self.markdown_highlight =
                    gpui_base::text::TextBackgrounds::new(source.clone(), layers)
                        .ok()
                        .map(|backgrounds| MarkdownHighlight {
                            source,
                            paints,
                            backgrounds: Rc::new(backgrounds),
                        });
            }
        } else {
            self.markdown_highlight = None;
        }
        if let Some(markdown) = &self.markdown {
            let backgrounds = self
                .markdown_highlight
                .as_ref()
                .map(|highlight| highlight.backgrounds.clone());
            markdown.update(cx, |state, cx| {
                state.set_text_backgrounds(backgrounds, cx);
            });
        }
    }
    fn new(
        root: WeakEntity<View>,
        node: NodeId,
        lease: Lease,
        config: Arc<Config>,
        highlight_identity: Rc<RefCell<Rc<()>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let snapshot = lease.snapshot();
        let job = document_host::request(
            document_jobs::Request {
                observer: None,
                snapshot: snapshot.clone(),
                mode: config.mode.clone(),
                dark: config.dark,
                search: config.search.clone(),
            },
            window,
            cx,
        );
        let editor = cx.new(|cx| {
            let mut editor = EditorState::new(window, cx)
                .language("gpuio-prepared")
                .soft_wrap(false)
                .folding(false)
                .line_number(config.line_numbers);
            editor.set_readonly(true, cx);
            editor
        });
        Self {
            highlight_identity,
            highlight: None,
            highlight_paint: None,
            markdown_highlight: None,
            buttons: BTreeMap::new(),
            root,
            node,
            lease,
            collapsed: config.initially_collapsed,
            config,
            selection_color: None,
            user_selectable: true,
            snapshot,
            job,
            editor,
            markdown: None,
            code_highlighter: None,
            search: Default::default(),
            search_index: None,
            runs: Arc::default(),
            diff: None,
            diff_config: None,
            diff_epoch: 0,
            diff_handler: None,
            #[cfg(feature = "native-tests")]
            diff_more_bounds: Rc::default(),
            diff_controls: None,
            file_buttons: BTreeMap::new(),
            file_visible: Rc::default(),
            #[cfg(feature = "native-tests")]
            file_suffixes: Rc::default(),
            projection: None,
            projection_charge: None,
            projection_error: None,
            projected_page: None,
            raw_diff: false,
            charge: None,
            error: None,
            ready: false,
            previous_pages: vec![],
            source_mode: false,
            installed_page_start: 0,
            page_start: 0,
            page_end: 0,
            source_lines: 1,
            installed: None,
            images: Default::default(),
            markdown_extensions: document_markdown::extensions(Default::default()),
        }
    }
    fn primary_focus(&self, cx: &App) -> gpui::FocusHandle {
        if (self.collapsed || self.installed.is_none())
            && let Some(focus) = self.buttons.get("document-collapse")
        {
            return focus.clone();
        }
        self.markdown
            .as_ref()
            .filter(|_| !self.source_mode)
            .map_or_else(
                || self.editor.read(cx).focus_handle(cx),
                |state| state.read(cx).focus_handle().clone(),
            )
    }
    fn configure_source_semantics(&self, cx: &mut Context<Self>) {
        let presentation = cx.weak_entity();
        let label = self.config.label.clone();
        let diff_action = self.diff_action();
        self.editor.update(cx, |editor, _| {
            editor.set_bridge_decorator(Rc::new(move |element, state, _, cx| {
                let focus = state.focus_handle(cx);
                let click_owner = presentation.clone();
                let key_owner = presentation.clone();
                let click_action = diff_action.clone();
                let key_action = diff_action.clone();
                let presentation = presentation.clone();
                let key_focus = focus.clone();
                let element = element
                    .on_click(move |event, _, cx| {
                        if let (Some(action), Some(point)) = (&click_action, event.mouse_position())
                        {
                            let _ = click_owner.update(cx, |this, cx| {
                                this.observe_diff_line(action, Some(point), cx);
                            });
                        }
                    })
                    .capture_key_down(move |event, window, cx| {
                        if event.keystroke.key == "enter"
                            && event.keystroke.modifiers == gpui::Modifiers::default()
                            && key_focus.is_focused(window)
                            && let Some(action) = &key_action
                        {
                            let applied = key_owner
                                .update(cx, |this, cx| this.observe_diff_line(action, None, cx))
                                .unwrap_or(false);
                            if applied {
                                cx.stop_propagation();
                            }
                        }
                    })
                    .role(gpui::Role::MultilineTextInput)
                    .aria_label(label.clone())
                    .aria_value(state.value())
                    .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                        let _ = presentation.update(cx, |this, cx| {
                            if !this.collapsed
                                && this.installed.is_some()
                                && (this.markdown.is_none() || this.source_mode)
                                && this.root.upgrade().is_some_and(|root| {
                                    let root = root.read(cx);
                                    root.focus.borrow().allows(this.node)
                                        && root
                                            .documents
                                            .get(&this.node)
                                            .and_then(|state| state.presentation.as_ref())
                                            .is_some_and(|current| current == &cx.entity())
                                })
                            {
                                window.focus(&focus, cx);
                            }
                        });
                    });
                crate::semantics::State {
                    hidden: false,
                    metadata: None,
                    live: None,
                    element,
                    disabled: false,
                    read_only: true,
                    modal: false,
                }
                .into_any_element()
            }));
        });
    }
    fn focused(&self, window: &Window, cx: &App) -> bool {
        self.buttons.values().any(|focus| focus.is_focused(window))
            || self
                .file_buttons
                .values()
                .any(|(_, focus)| focus.is_focused(window))
            || self.editor.read(cx).focus_handle(cx).is_focused(window)
            || self
                .markdown
                .as_ref()
                .is_some_and(|state| state.read(cx).focus_handle().is_focused(window))
    }
    fn retained(&self, window: &Window, cx: &App) -> bool {
        self.focused(window, cx)
            || !self.editor.read(cx).selected_range().is_empty()
            || self.markdown.as_ref().is_some_and(|state| {
                state.read(cx).is_selecting() || state.read(cx).has_local_selection()
            })
    }
    fn invalidate_row(&self, cx: &mut Context<Self>) {
        *self.highlight_identity.borrow_mut() = Rc::new(());
        let root = self.root.clone();
        let id = self.node;
        cx.defer(move |cx| {
            let _ = root.update(cx, |root, cx| {
                root.invalidate_resource_row(id);
                cx.notify();
            });
        });
    }
    fn refresh(&mut self, config: Arc<Config>, cx: &mut Context<Self>) {
        let snapshot = self.lease.snapshot();
        let changed = !Arc::ptr_eq(&snapshot, &self.snapshot)
            || config.mode != self.config.mode
            || config.dark != self.config.dark
            || config.search != self.config.search;
        if changed {
            self.snapshot = snapshot;
            self.ready = false;
            cx.notify();
            if let Ok(job) = &self.job {
                let _ = job.update(
                    document_jobs::Request {
                        observer: None,
                        snapshot: self.snapshot.clone(),
                        mode: config.mode.clone(),
                        dark: config.dark,
                        search: config.search.clone(),
                    },
                    cx,
                );
            }
        }
        self.config = config;
    }
    fn refresh_diff(
        &mut self,
        config: Option<Arc<gpuio_protocol::document_diff::Config>>,
        epoch: i64,
        handler: Option<gpuio_protocol::HandlerId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handler_changed = self.diff_handler != handler;
        self.diff_handler = handler;
        if self.diff_epoch == epoch {
            if handler_changed && self.installed.is_some() {
                self.install_file_headers(true, window, cx);
            }
            return;
        }
        self.diff_config = config;
        self.diff_epoch = epoch;
        if self.diff_config.is_none() {
            self.diff_controls = None;
            self.raw_diff = false;
        }
        if let Some(snapshot) = self.installed.clone() {
            self.rebuild_diff(snapshot, window, cx);
        }
    }
    fn show_source(
        &mut self,
        runs: Vec<crate::document_highlight::Run>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.runs = Arc::new(runs);
        self.rebuild_diff(self.snapshot.clone(), window, cx);
    }
    fn rebuild_diff(
        &mut self,
        snapshot: Arc<Snapshot>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prepared: Result<_, document_jobs::Error> = (|| {
            let (Some(config), Some(diff)) = (&self.diff_config, &self.diff) else {
                return Ok(None);
            };
            // Conservative admission units, not RSS. Include controller copies,
            // keys, row maps, projected runs, old/new page text and editor buffers.
            // Hold the previous charge until replacement has finished.
            let bytes = 4096
                + 4 * config.retained_bytes()
                + 128 * config.collapse.keys().len()
                + 2 * self
                    .diff_controls
                    .as_ref()
                    .map_or(0, Controls::retained_bytes)
                + 8 * snapshot.text.len()
                + 256 * diff.lines.len()
                + 2 * self.runs.len() * std::mem::size_of::<crate::document_highlight::Run>()
                + 4 * 65536;
            let charge = self.job.as_ref().map_err(|error| *error)?.reserve(bytes)?;
            let mut controls = match &self.diff_controls {
                Some(controls) => {
                    let mut next = controls.clone();
                    next.configure((**config).clone(), snapshot.generation)
                        .map_err(|_| document_jobs::Error::Parse)?;
                    next
                }
                None => Controls::new((**config).clone(), snapshot.generation)
                    .map_err(|_| document_jobs::Error::Parse)?,
            };
            controls.install(diff);
            let projection = Projection::new(
                &snapshot.text.to_string(),
                diff,
                |_, file| controls.is_collapsed(file),
                controls.limit(),
            )
            .ok_or(document_jobs::Error::Parse)?;
            Ok(Some((projection, charge, controls)))
        })();
        self.projection_error = None;
        let (next, charge) = match prepared {
            Ok(Some((projection, charge, controls))) => {
                self.diff_controls = Some(controls);
                (Some(projection), Some(charge))
            }
            Ok(None) => {
                self.diff_controls = None;
                (None, None)
            }
            Err(error) => {
                self.projection_error = Some(format!(
                    "Diff controls unavailable ({error:?}); showing source."
                ));
                // Preserve managed values and their old admission while source
                // fallback is visible. A failed replacement is not a reset.
                (None, self.projection_charge.take())
            }
        };
        let selection = self.editor.read(cx).bridge_selection();
        let mapped = if !self.raw_diff
            && self
                .installed
                .as_ref()
                .is_some_and(|old| old.generation == snapshot.generation)
        {
            self.projection
                .as_ref()
                .zip(next.as_ref())
                .and_then(|(old, next)| {
                    old.remap_selection(
                        next,
                        (
                            self.installed_page_start + selection.0,
                            self.installed_page_start + selection.1,
                        ),
                    )
                })
        } else {
            None
        };
        let page_origin = self
            .projection
            .as_ref()
            .zip(next.as_ref())
            .filter(|_| {
                self.installed
                    .as_ref()
                    .is_some_and(|old| old.generation == snapshot.generation)
            })
            .and_then(|(old, next)| {
                next.display_caret(old.source_caret(self.installed_page_start)?)
            });
        let projection_changed = self.projection.is_some() || next.is_some();
        self.projection = next;
        if projection_changed && !self.raw_diff {
            // Keep the same visible origin across appends. Move only when the
            // old origin disappeared or the surviving selection precedes it.
            self.page_start = page_origin.unwrap_or(0);
            if let Some((a, b)) = mapped {
                self.page_start = self.page_start.min(a.min(b));
            }
            self.previous_pages.clear();
        }
        self.show_page_for(
            snapshot,
            mapped,
            projection_changed && !self.raw_diff,
            window,
            cx,
        );
        self.projection_charge = charge;
    }
    fn active_projection(&self) -> Option<&Projection> {
        self.projection.as_ref().filter(|_| !self.raw_diff)
    }
    fn display_bytes(&self) -> usize {
        self.active_projection().map_or_else(
            || {
                self.installed
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.text.len())
            },
            |projection| projection.text().len(),
        )
    }
    fn show_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(snapshot) = self.installed.clone() {
            self.show_page_for(snapshot, None, false, window, cx);
        }
    }
    fn show_page_for(
        &mut self,
        snapshot: Arc<Snapshot>,
        mapped_selection: Option<(usize, usize)>,
        projection_changed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let projection = self.active_projection();
        let (page, origin, final_line, candidates, projected_runs) = if let Some(projection) =
            projection
        {
            let text = projection.text();
            let start = text.floor_char_boundary(self.page_start.min(text.len()));
            let limit = text.floor_char_boundary((start + 65536).min(text.len()));
            let page = bounded_page(&text[start..limit], start);
            let origin = text[..start].bytes().filter(|b| *b == b'\n').count();
            let final_line = origin
                + page.text.bytes().filter(|b| *b == b'\n').count()
                + usize::from(
                    page.end == text.len() && !page.text.is_empty() && !page.text.ends_with('\n'),
                );
            let hunks = projection.hunks(self.diff.as_ref().expect("projected diff"));
            let runs = projection
                .highlights(&self.runs)
                .expect("prepared diff runs");
            (page, origin, final_line, hunks, Some(runs))
        } else {
            let page = page(&snapshot, self.page_start);
            let origin = snapshot.text.byte_to_line_idx(
                self.page_start.min(snapshot.text.len()),
                ropey::LineType::LF,
            );
            let final_line = snapshot
                .text
                .byte_to_line_idx(page.end, ropey::LineType::LF)
                + usize::from(
                    page.end == snapshot.text.len()
                        && !page.text.is_empty()
                        && !page.text.ends_with('\n'),
                );
            let hunks = self
                .diff
                .as_ref()
                .map_or_else(Vec::new, |diff| diff.hunks.clone());
            (page, origin, final_line, hunks, None)
        };
        self.page_start = page.end - page.text.len();
        self.page_end = page.end;
        self.projected_page = projected_runs
            .as_ref()
            .map(|_| Arc::from(page.text.as_str()));
        let word_diff = self
            .diff_config
            .as_ref()
            .is_none_or(|config| config.word_diff);
        let runs = Arc::new(
            projected_runs
                .as_deref()
                .unwrap_or(&self.runs)
                .iter()
                .cloned()
                .filter_map(|mut run| {
                    run.bytes =
                        run.bytes.start.max(self.page_start)..run.bytes.end.min(self.page_end);
                    if run.bytes.is_empty() {
                        return None;
                    }
                    run.bytes = run.bytes.start - self.page_start..run.bytes.end - self.page_start;
                    if self.diff.is_some() && !word_diff {
                        run.diff_emphasis = false;
                    }
                    Some(run)
                })
                .collect::<Vec<_>>(),
        );
        let folds: Vec<_> = candidates
            .iter()
            .filter(|hunk| {
                hunk.start >= origin && hunk.end <= final_line && hunk.end > hunk.start + 2
            })
            .map(|hunk| gpui_base::input::FoldRange::new(hunk.start - origin, hunk.end - origin))
            .collect();
        self.source_lines = page.text.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let old = self.installed.as_ref();
        self.editor.update(cx, |state, cx| {
            let mut selection = state.bridge_selection();
            let selected_end = selection.0.max(selection.1);
            let same_selection = mapped_selection.is_some_and(|(a, b)| {
                a == self.page_start + selection.0 && b == self.page_start + selection.1
            });
            let preserve = (!projection_changed || same_selection)
                && self.page_start == self.installed_page_start
                && old.is_some_and(|old| old.generation == snapshot.generation)
                && selected_end <= page.text.len()
                && selected_end <= state.value().len()
                && state.value().get(..selected_end) == page.text.get(..selected_end);
            if !preserve {
                selection = (0, 0);
            }
            if let Some((anchor, caret)) = mapped_selection
                && anchor.min(caret) >= self.page_start
                && anchor.max(caret) <= self.page_end
            {
                selection = (anchor - self.page_start, caret - self.page_start);
            }
            let offset = state.scroll_offset();
            state.bridge_replace_all(page.text.into(), selection, false, window, cx);
            if preserve {
                state.set_scroll_offset(offset, cx);
            }
            state.set_folding(self.diff.is_some(), window, cx);
            state.set_highlighter_factory(
                Rc::new(move |_| {
                    Some(Box::new(document_editor::Highlights(
                        runs.clone(),
                        folds.clone(),
                    )))
                }),
                cx,
            );
            state.set_line_number(self.config.line_numbers, window, cx);
            state.set_line_number_offset(origin, cx);
            state.set_editor_style(editor_style(self.config.dark, self.selection_color));
            state.set_search_query(self.config.search.clone(), false, cx);
        });
        self.source_mode = true;
        self.installed_page_start = self.page_start;
        let same_generation = self
            .installed
            .as_ref()
            .is_some_and(|old| old.generation == snapshot.generation);
        self.installed = Some(snapshot);
        self.install_file_headers(same_generation, window, cx);
        self.invalidate_row(cx);
    }
    fn accept_ready(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ready = match &self.job {
            Ok(job) => job.take_ready(),
            Err(error) if !self.ready => Some(Err(*error)),
            Err(_) => None,
        };
        let Some(ready) = ready else {
            return;
        };
        if self
            .installed
            .as_ref()
            .is_some_and(|old| old.generation != self.snapshot.generation)
        {
            self.page_start = 0;
            self.previous_pages.clear();
            self.source_mode = false;
            self.raw_diff = false;
            self.collapsed = self.config.initially_collapsed;
        }
        self.ready = true;
        self.error = None;
        self.diff = None;
        self.code_highlighter = None;
        self.runs = Arc::default();
        self.charge = None;
        match ready {
            Ok(ready) => {
                self.search = ready.search;
                self.search_index = None;
                if !matches!(&ready.prepared, Prepared::Markdown { .. }) {
                    self.markdown = None;
                }
                match ready.prepared {
                    Prepared::Markdown { document, code } => {
                        self.projection = None;
                        self.projection_charge = None;
                        self.projection_error = None;
                        self.projected_page = None;
                        self.diff_controls = None;
                        self.raw_diff = false;
                        let showing_source = self.source_mode && self.markdown.is_some();
                        let state = self
                            .markdown
                            .get_or_insert_with(|| cx.new(TextViewState::externally_prepared));
                        let prefix = self
                            .installed
                            .as_ref()
                            .filter(|old| old.generation == self.snapshot.generation)
                            .map(|old| {
                                if old.revision + 1 == self.snapshot.revision {
                                    self.snapshot.unchanged_bytes
                                } else {
                                    0
                                }
                            });
                        state.update(cx, |state, cx| {
                            state.set_prepared(*document, prefix, cx);
                            state.set_markdown_extensions(
                                Arc::new(self.markdown_extensions.clone()),
                                cx,
                            );
                        });
                        self.code_highlighter = Some(Arc::new(move |block| {
                            let key = (
                                block
                                    .lang()
                                    .map_or_else(|| "txt".to_string(), |s| s.to_string()),
                                block.code().to_string(),
                            );
                            code.get(&key).map_or_else(Vec::new, |runs| {
                                runs.iter()
                                    .map(|run| (run.bytes.clone(), document_editor::style(run)))
                                    .collect()
                            })
                        }));
                        if showing_source {
                            self.show_page_for(self.snapshot.clone(), None, false, window, cx);
                        } else {
                            self.source_mode = false;
                            self.file_buttons.clear();
                            self.file_visible.borrow_mut().clear();
                            #[cfg(feature = "native-tests")]
                            self.file_suffixes.borrow_mut().clear();
                            self.editor.update(cx, |state, cx| {
                                let _ = state.set_row_adornments(None, None, cx);
                                state.bridge_replace_all("".into(), (0, 0), false, window, cx)
                            });
                            self.installed = Some(self.snapshot.clone());
                        }
                    }
                    Prepared::Source(error) => {
                        self.error = Some(format!(
                            "Rich view unavailable ({error:?}); showing source."
                        ));
                        self.show_source(vec![], window, cx);
                    }
                    Prepared::Code(runs) => self.show_source(runs, window, cx),
                    Prepared::Diff { document, runs } => {
                        self.diff = Some(document);
                        self.show_source(runs, window, cx);
                    }
                }
                self.charge = Some(ready.charge);
            }
            Err(error) => {
                self.markdown = None;
                self.error = Some(format!(
                    "Rich view unavailable ({error:?}); showing source."
                ));
                self.show_source(vec![], window, cx);
            }
        }
        self.invalidate_row(cx);
    }
    fn next_match(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.ranges.is_empty() {
            return;
        }
        let len = self.search.ranges.len();
        let index = match self.search_index {
            None => {
                if forward {
                    0
                } else {
                    len - 1
                }
            }
            Some(i) => {
                if forward {
                    (i + 1) % len
                } else {
                    (i + len - 1) % len
                }
            }
        };
        let range = self.search.ranges[index].clone();
        self.search_index = Some(index);
        if self.projection.is_some() {
            self.raw_diff = true;
            self.previous_pages.clear();
        } else {
            self.previous_pages.push(self.page_start);
        }
        self.page_start = range.start;
        self.show_page(window, cx);
        self.editor.update(cx, |state, cx| {
            state.bridge_select(0, range.len(), cx);
            window.focus(&state.focus_handle(cx), cx);
        });
        self.invalidate_row(cx);
        cx.notify();
    }
    fn diff_action(&self) -> Option<DiffAction> {
        if self.collapsed || self.raw_diff || self.active_projection().is_none() {
            return None;
        }
        Some(DiffAction {
            snapshot: self.installed.clone()?,
            page: self.projected_page.clone()?,
            epoch: self.diff_epoch,
            handler: self.diff_handler,
        })
    }
    fn accepts_diff_action(&self, action: &DiffAction, cx: &Context<Self>) -> bool {
        if self.collapsed
            || self.raw_diff
            || self.active_projection().is_none()
            || self.diff_epoch != action.epoch
            || self.diff_handler != action.handler
            || !self
                .installed
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &action.snapshot))
            || !self
                .projected_page
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &action.page))
        {
            return false;
        }
        let Some(root) = self.root.upgrade() else {
            return false;
        };
        let root = root.read(cx);
        let session = root.session.borrow();
        let Some(tree) = session.tree(root.id) else {
            return false;
        };
        let Some(node) = tree.get(self.node) else {
            return false;
        };
        let Some(source) = self.config.source else {
            return false;
        };
        session.accepts_input(root.id)
            && root.focus.borrow().allows(self.node)
            && root
                .documents
                .get(&self.node)
                .and_then(|state| state.presentation.as_ref())
                == Some(&cx.entity())
            && node.handler == action.handler
            && node.document_diff_epoch == action.epoch
            && node.document_diff == self.diff_config
            && node
                .document
                .as_ref()
                .is_some_and(|config| config.source == Some(source))
            && session
                .document(source)
                .is_ok_and(|lease| lease.snapshot().generation == action.snapshot.generation)
    }
    fn emit_diff(
        &self,
        action: &DiffAction,
        observation: gpuio_protocol::document_diff::Observation,
        cx: &mut Context<Self>,
    ) {
        let (Some(handler), Some(source)) = (action.handler, self.config.source) else {
            return;
        };
        let event = gpuio_protocol::document_diff::Event {
            config_epoch: action.epoch,
            source_revision: action.snapshot.revision,
            source_generation: action.snapshot.generation,
            observation,
        };
        let _ = self.root.update(cx, |root, _| {
            let event = root
                .session
                .borrow()
                .document_diff_event(root.id, self.node, handler, source, event);
            if let Some(event) = event
                && !root.transport.input(event)
                && root.session.borrow_mut().overload(root.id)
            {
                root.transport.fault(root.id);
            }
        });
    }
    fn observe_diff_line(
        &self,
        action: &DiffAction,
        pointer: Option<gpui::Point<gpui::Pixels>>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.accepts_diff_action(action, cx) {
            return false;
        }
        let (anchor, caret) = self.editor.read(cx).bridge_selection();
        if pointer.is_some() && anchor != caret {
            return false;
        }
        let projection = self.active_projection().unwrap();
        let display_caret = self.installed_page_start + caret;
        let row = projection
            .rows()
            .iter()
            .find(|row| row.display.contains(&display_caret))
            .or_else(|| {
                (display_caret == projection.text().len() && !projection.text().ends_with('\n'))
                    .then(|| projection.rows().last())
                    .flatten()
            });
        let Some(row) = row else {
            return false;
        };
        if let Some(point) = pointer {
            let end =
                row.display.end - usize::from(projection.text()[..row.display.end].ends_with('\n'));
            let bytes = row.display.start.max(self.installed_page_start) - self.installed_page_start
                ..end
                    .min(self.page_end)
                    .saturating_sub(self.installed_page_start);
            let editor = self.editor.read(cx);
            if !editor.input_bounds().contains(&point)
                || !editor
                    .range_to_bounds(&bytes)
                    .is_some_and(|bounds| bounds.contains(&point))
            {
                return false;
            }
        }
        let line = &self.diff.as_ref().unwrap().lines[row.source_line];
        use crate::document_diff::Kind;
        if !matches!(
            line.kind,
            Kind::Context | Kind::Added | Kind::Removed | Kind::Meta
        ) {
            return false;
        }
        let Some(index) = line.file else {
            return false;
        };
        let file = &self.diff.as_ref().unwrap().files[index];
        use gpuio_protocol::document_diff::{File, FileKey, Line, Observation};
        let payload = Line {
            file: File {
                index: index as i64,
                key: file
                    .path()
                    .map_or(FileKey::Unnamed, |path| FileKey::Path(path.to_owned())),
                before_path: file.before_path.as_deref().map(str::to_owned),
                after_path: file.after_path.as_deref().map(str::to_owned),
            },
            before: line.before.map(|value| value as i64),
            after: line.after.map(|value| value as i64),
            start_byte: line.content.start as i64,
            end_byte: line.content.end as i64,
            text: action.snapshot.text.slice(line.content.clone()).to_string(),
        };
        if !payload.is_valid() {
            return false;
        }
        self.emit_diff(action, Observation::Line(payload), cx);
        true
    }
    fn show_more_diff(&mut self, action: &DiffAction, window: &mut Window, cx: &mut Context<Self>) {
        if !self.accepts_diff_action(action, cx) {
            return;
        }
        let Some(observation) = self
            .diff_controls
            .as_mut()
            .and_then(|controls| controls.show_more(self.projection.as_ref()?))
        else {
            return;
        };
        let applied = matches!(
            &observation,
            gpuio_protocol::document_diff::Observation::ShowMore {
                applied_limit: Some(_),
                ..
            }
        );
        self.emit_diff(action, observation, cx);
        if applied {
            let was_focused = self
                .buttons
                .get("document-diff-more")
                .is_some_and(|focus| focus.is_focused(window));
            self.rebuild_diff(action.snapshot.clone(), window, cx);
            if was_focused && !self.has_more_diff() {
                window.focus(&self.editor.read(cx).focus_handle(cx), cx);
            }
            cx.notify();
        }
    }
    fn has_more_diff(&self) -> bool {
        !self.collapsed
            && self
                .active_projection()
                .is_some_and(|projection| projection.hidden_body_lines() > 0)
    }
    fn navigation_at_caret(&self, cx: &App) -> Option<Navigation> {
        let caret = self.installed_page_start + self.editor.read(cx).bridge_selection().1;
        let caret = match self.active_projection() {
            Some(projection) => projection.source_caret(caret)?,
            None => caret,
        };
        if let Some(diff) = &self.diff {
            diff.lines
                .iter()
                .find(|line| line.bytes.contains(&caret))
                .and_then(|line| line.navigation())
        } else if self.config.path.is_some()
            && matches!(self.config.mode, gpuio_protocol::document::Mode::Code(_))
        {
            let snapshot = self.installed.as_ref()?;
            let line = snapshot
                .text
                .byte_to_line_idx(caret.min(snapshot.text.len()), ropey::LineType::LF)
                + 1;
            Some(Navigation::Line(
                self.config.path.clone(),
                gpuio_protocol::document::Side::After,
                line as i64,
            ))
        } else {
            None
        }
    }
    fn navigate(&self, navigation: Navigation, cx: &mut Context<Self>) {
        if !navigation.is_valid() {
            return;
        }
        let source = self.config.source;
        let Some(installed) = &self.installed else {
            return;
        };
        let generation = installed.generation;
        let node = self.node;
        let presentation = cx.entity();
        let _ = self.root.update(cx, |root, _| {
            let event = {
                let session = root.session.borrow();
                let Some(tree) = session.tree(root.id) else {
                    return;
                };
                let Some(current) = tree.get(node) else {
                    return;
                };
                if !root.focus.borrow().allows(node) {
                    return;
                }
                if root
                    .documents
                    .get(&node)
                    .and_then(|state| state.presentation.as_ref())
                    != Some(&presentation)
                {
                    return;
                }
                if current
                    .document
                    .as_ref()
                    .is_none_or(|config| config.source != source)
                {
                    return;
                }
                if self.lease.snapshot().generation != generation {
                    return;
                }
                let Some(handler) = current.handler else {
                    return;
                };
                Event::DocumentNavigation(
                    root.id,
                    node,
                    handler,
                    tree.revision(),
                    source.expect("mounted source"),
                    generation,
                    navigation,
                )
            };
            if !root.transport.input(event) && root.session.borrow_mut().overload(root.id) {
                root.transport.fault(root.id);
            }
        });
    }
}
impl Presentation {
    // Native document controls are not protocol buttons. Handle accessibility
    // activation directly: GPUI's fallback click at the un-clipped center can
    // target an unrelated control when a retained row is outside its viewport.
    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui_base::Button {
        let action = Rc::new(action);
        let pointer_action = action.clone();
        let owner = cx.weak_entity();
        gpui_base::Button::new(id)
            .aria_label(label)
            .track_focus(&self.buttons[id])
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                pointer_action(this, window, cx);
            }))
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                let _ = owner.update(cx, |this, cx| action(this, window, cx));
                cx.stop_propagation();
            })
    }
}
impl Render for Presentation {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.refresh(self.config.clone(), cx);
        self.accept_ready(window, cx);
        if !self.has_more_diff()
            && self
                .buttons
                .get("document-diff-more")
                .is_some_and(|focus| focus.is_focused(window))
        {
            // Controlled acceptance, collapse and raw-source transitions can
            // remove the footer outside its own activation callback.
            window.focus(&self.primary_focus(cx), cx);
        }
        self.install_highlight(cx);
        let weak = cx.weak_entity();
        for name in [
            "document-collapse",
            "document-copy",
            "document-previous",
            "document-next",
            "document-search-previous",
            "document-search-next",
            "document-location",
            "document-rendered",
            "document-diff-view",
            "document-diff-more",
        ] {
            self.buttons
                .entry(name)
                .or_insert_with(|| cx.focus_handle());
        }
        let mut toolbar = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .text_size(px(11.))
            .text_color(gpui::rgb(if self.config.dark {
                0x969dad
            } else {
                0x656a76
            }))
            .child(div().flex_1().child(self.config.label.clone()))
            .when(
                !self.collapsed && !self.ready && self.installed.is_none(),
                |toolbar| toolbar.child("Updating…"),
            )
            .child(self.button(
                "document-collapse",
                if self.collapsed { "Expand" } else { "Collapse" },
                |this, _, cx| {
                    this.collapsed = !this.collapsed;
                    this.invalidate_row(cx);
                    cx.notify();
                },
                cx,
            ))
            .child(self.button(
                "document-copy",
                "Copy source",
                |this, _, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                        this.snapshot.text.to_string(),
                    ));
                },
                cx,
            ));
        if self.source_mode && self.markdown.is_some() {
            toolbar = toolbar.child(self.button(
                "document-rendered",
                "Rendered view",
                |this, _, cx| {
                    this.source_mode = false;
                    this.invalidate_row(cx);
                    cx.notify();
                },
                cx,
            ));
        }
        if self.raw_diff && self.projection.is_some() {
            toolbar = toolbar.child(self.button(
                "document-diff-view",
                "Diff view",
                |this, window, cx| {
                    this.raw_diff = false;
                    this.page_start = 0;
                    this.previous_pages.clear();
                    this.show_page_for(this.installed.clone().unwrap(), None, true, window, cx);
                    cx.notify();
                },
                cx,
            ));
        }
        if !self.config.search.is_empty() {
            toolbar = toolbar
                .child(format!(
                    "{} matches{}",
                    self.search.total,
                    if self.search.total > crate::document_search::MAX_MATCHES {
                        " (first 4096 navigable)"
                    } else {
                        ""
                    }
                ))
                .child(self.button(
                    "document-search-previous",
                    "Previous match",
                    |this, window, cx| this.next_match(false, window, cx),
                    cx,
                ))
                .child(self.button(
                    "document-search-next",
                    "Next match",
                    |this, window, cx| this.next_match(true, window, cx),
                    cx,
                ));
        }
        if self.page_start > 0 {
            toolbar = toolbar.child(self.button(
                "document-previous",
                "Previous source page",
                |this, window, cx| {
                    this.page_start = this.previous_pages.pop().unwrap_or(0);
                    this.show_page(window, cx);
                    cx.notify();
                },
                cx,
            ));
        }
        if (self.markdown.is_none() || self.source_mode)
            && self.ready
            && self.page_end < self.display_bytes()
        {
            toolbar = toolbar.child(self.button(
                "document-next",
                "Next source page",
                |this, window, cx| {
                    this.previous_pages.push(this.page_start);
                    this.page_start = this.page_end;
                    this.show_page(window, cx);
                    cx.notify();
                },
                cx,
            ));
        }
        let navigation = self.navigation_at_caret(cx);
        let line_action = self.diff_action();
        if navigation.is_some() || line_action.is_some() {
            toolbar = toolbar.child(self.button(
                "document-location",
                "Go to line",
                move |this, _, cx| {
                    if line_action
                        .as_ref()
                        .is_none_or(|action| this.observe_diff_line(action, None, cx))
                        && let Some(navigation) = this.navigation_at_caret(cx)
                    {
                        this.navigate(navigation, cx);
                    }
                },
                cx,
            ));
        }
        let mut order = vec![
            self.buttons["document-collapse"].clone(),
            self.buttons["document-copy"].clone(),
        ];
        if !self.config.search.is_empty() {
            order.extend([
                self.buttons["document-search-previous"].clone(),
                self.buttons["document-search-next"].clone(),
            ]);
        }
        if self.page_start > 0 {
            order.push(self.buttons["document-previous"].clone());
        }
        if (self.markdown.is_none() || self.source_mode)
            && self.ready
            && self.page_end < self.display_bytes()
        {
            order.push(self.buttons["document-next"].clone());
        }
        if self.source_mode && self.markdown.is_some() {
            order.push(self.buttons["document-rendered"].clone());
        }
        if self.raw_diff && self.projection.is_some() {
            order.push(self.buttons["document-diff-view"].clone());
        }
        if navigation.is_some() || self.diff_action().is_some() {
            order.push(self.buttons["document-location"].clone());
        }
        let header_slot = order.len();
        let header_handles: Vec<_> = if self.collapsed || self.active_projection().is_none() {
            Vec::new()
        } else {
            self.file_buttons
                .iter()
                .map(|(index, (_, focus))| (*index, focus.clone()))
                .collect()
        };
        let header_visible = self.file_visible.clone();
        if !self.collapsed && self.installed.is_some() {
            order.push(self.primary_focus(cx));
        }
        if self.has_more_diff() {
            order.push(self.buttons["document-diff-more"].clone());
        }
        let root_view = self.root.clone();
        let node = self.node;
        let mut root = div()
            .id("document-presentation")
            .role(gpui::Role::Group)
            .aria_label(self.config.label.clone())
            .flex()
            .flex_col()
            .w_full()
            .gap(px(6.))
            .child(toolbar)
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key != "tab" {
                    return;
                }
                let backward = event.keystroke.modifiers.shift;
                let mut order = order.clone();
                order.splice(
                    header_slot..header_slot,
                    header_handles
                        .iter()
                        .filter(|(index, _)| header_visible.borrow().contains_key(index))
                        .map(|(_, focus)| focus.clone()),
                );
                let index = order.iter().position(|focus| focus.is_focused(window));
                let next = index.and_then(|i| {
                    if backward {
                        i.checked_sub(1)
                    } else {
                        (i + 1 < order.len()).then_some(i + 1)
                    }
                });
                if let Some(index) = next {
                    window.focus(&order[index], cx);
                } else {
                    let _ = root_view.update(cx, |root, cx| {
                        // Global traversal stores the document's primary focus; briefly
                        // restore it to identify this composite before leaving it.
                        if let Some(p) = root
                            .documents
                            .get(&node)
                            .and_then(|state| state.presentation.as_ref())
                        {
                            window.focus(&p.read(cx).primary_focus(cx), cx);
                        }
                        root.focus.borrow().traverse(backward, window, cx);
                    });
                }
                cx.stop_propagation();
            });
        if self.collapsed {
            return root.into_any_element();
        }
        for error in [&self.error, &self.projection_error].into_iter().flatten() {
            root = root.child(error.clone());
        }
        // Keep the last installed document geometrically stable while a newer
        // parse is pending. Before the first result, the toolbar carries the
        // loading notice; do not paint a dummy source editor that will collapse
        // when an initially empty Markdown document becomes ready.
        if !self.ready && self.installed.is_none() {
            return root.into_any_element();
        }
        let height = match self.config.layout {
            Layout::Viewport(height) => height as f32,
            Layout::Flow => ((self.source_lines + 1) as f32 * window.line_height().as_f32() + 16.)
                .clamp(60., 360.),
        };
        if let Some(state) = self.markdown.as_ref().filter(|_| !self.source_mode) {
            let installed_revision = self
                .installed
                .as_ref()
                .map(|snapshot| (snapshot.generation, snapshot.revision));
            let highlighter = self
                .code_highlighter
                .clone()
                .expect("prepared Markdown highlighter");
            let text = TextView::new(state)
                .style(markdown_style(self.config.dark, self.selection_color))
                .selectable(self.user_selectable)
                .scrollable(matches!(self.config.layout, Layout::Viewport(_)))
                .markdown_extensions(self.markdown_extensions.clone())
                .table_actions(|table, _, _| {
                    let markdown = table.markdown.clone();
                    let accessible_markdown = markdown.clone();
                    gpui_base::Button::new("copy-table")
                        .aria_label("Copy table")
                        .child("Copy table")
                        .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                accessible_markdown.clone(),
                            ));
                            cx.stop_propagation();
                        })
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(markdown.clone()))
                        })
                })
                .code_block_actions(|block, _, _| {
                    let code = block.code();
                    let accessible_code = code.clone();
                    gpui_base::Button::new("copy-code")
                        .aria_label("Copy code")
                        .child("Copy code")
                        .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                accessible_code.to_string(),
                            ));
                            cx.stop_propagation();
                        })
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(code.to_string()))
                        })
                })
                .code_block_highlighter_shared(highlighter)
                .on_link_click(move |url, _, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        if !this.collapsed
                            && !this.source_mode
                            && this.markdown.is_some()
                            && this
                                .installed
                                .as_ref()
                                .map(|snapshot| (snapshot.generation, snapshot.revision))
                                == installed_revision
                        {
                            this.navigate(Navigation::Link(url.to_string()), cx)
                        }
                    });
                });
            let content = div().w_full().child(text);
            root = root.child(if matches!(self.config.layout, Layout::Viewport(_)) {
                content.h(px(height))
            } else {
                content
            });
        } else {
            self.configure_source_semantics(cx);
            if self.installed.is_some()
                && (self.page_start > 0 || self.page_end < self.display_bytes())
            {
                root = root.child(format!(
                    "{} bytes {}–{} of {}",
                    if self.active_projection().is_some() {
                        "Visible diff"
                    } else {
                        "Source"
                    },
                    self.page_start,
                    self.page_end,
                    self.display_bytes()
                ));
            }
            root = root.child(
                div()
                    .w_full()
                    .h(px(height))
                    .font_family(if cfg!(target_os = "macos") {
                        "Menlo"
                    } else {
                        "DejaVu Sans Mono"
                    })
                    .child(Editor::new(&self.editor)),
            );
        }
        if self.has_more_diff()
            && let Some(action) = self.diff_action()
        {
            let hidden = self.active_projection().unwrap().hidden_body_lines();
            let button = self.button(
                "document-diff-more",
                "Show more diff lines",
                move |this, window, cx| this.show_more_diff(&action, window, cx),
                cx,
            );
            #[cfg(feature = "native-tests")]
            let button = {
                use gpui_base::ElementExt as _;
                let bounds = self.diff_more_bounds.clone();
                button.on_prepaint(move |value, _, _| *bounds.borrow_mut() = Some(value))
            };
            root = root.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(button)
                    .child(format!("{hidden} more lines")),
            );
        }
        root.into_any_element()
    }
}
impl State {
    pub(super) fn highlight_groups(
        &self,
        cx: &App,
    ) -> Result<crate::highlight_collect::DocumentGroups, crate::highlight_collect::DocumentError>
    {
        let presentation = self
            .presentation
            .as_ref()
            .ok_or(crate::highlight_collect::DocumentError::Pending)?;
        presentation.read(cx).highlight_groups(cx)
    }
    pub(super) fn retained(&self, window: &Window, cx: &App) -> bool {
        self.presentation
            .as_ref()
            .is_some_and(|p| p.read(cx).retained(window, cx))
    }
    pub(super) fn focused(&self, window: &Window, cx: &App) -> bool {
        self.presentation
            .as_ref()
            .is_some_and(|p| p.read(cx).focused(window, cx))
    }
}
impl View {
    pub(super) fn sync_documents(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            self.documents.clear();
            return;
        };
        self.documents
            .retain(|id, _| tree.get(*id).is_some_and(|node| node.document.is_some()));
        for id in dirty {
            let Some(config) = tree.get(*id).and_then(|node| node.document.as_ref()) else {
                continue;
            };
            if !self
                .documents
                .get(id)
                .is_some_and(|state| state.source == config.source)
            {
                self.documents.remove(id);
                let lease = config.source.and_then(|id| session.document(id).ok());
                self.documents.insert(
                    *id,
                    State {
                        source: config.source,
                        lease,
                        image_sources: vec![],
                        images: BTreeMap::new(),
                        presentation: None,
                    },
                );
            }
            let state = self.documents.get_mut(id).unwrap();
            if state.image_sources != config.images {
                state.images.clear();
                state.image_sources.clone_from(&config.images);
            }
            state
                .images
                .retain(|url, _| config.images.iter().any(|(current, _)| current == url));
            for (url, source) in &config.images {
                if state.images.contains_key(url) {
                    continue;
                }
                if let ImageSource::Reference(id) = source
                    && let Ok(lease) = session.acquire_image(*id)
                    && let Ok(handle) = crate::image_host::request(lease, window, cx)
                {
                    state.images.insert(url.clone(), handle);
                }
            }
            if let Some(presentation) = &state.presentation {
                presentation.update(cx, |state, cx| {
                    state.refresh(config.clone(), cx);
                    let node = tree.get(*id).unwrap();
                    state.refresh_diff(
                        node.document_diff.clone(),
                        node.document_diff_epoch,
                        node.handler,
                        window,
                        cx,
                    );
                    cx.notify();
                });
            }
        }
    }
    pub(super) fn document_changed(&mut self, source: ResourceId, cx: &mut Context<Self>) {
        let ids: Vec<_> = self
            .documents
            .iter()
            .filter_map(|(id, state)| (state.source == Some(source)).then_some(*id))
            .collect();
        if ids.is_empty() {
            return;
        }
        for id in ids {
            self.invalidate_resource_row(id);
        }
        cx.notify();
    }
    pub(super) fn invalidate_resource_row(&self, id: NodeId) {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return;
        };
        let mut child = id;
        while let Some(parent) = tree.get(child).and_then(|node| node.parent) {
            let node = tree.get(parent).unwrap();
            if let Some(list) = self.lists.get(&parent)
                && let Some(row) = node.list_rows.iter().find(|row| row.node == child)
            {
                list.borrow().native.invalidate_rows(&[row.id]);
            }
            child = parent;
        }
    }
    pub(super) fn document_element(
        &mut self,
        tree: &Tree,
        node: &Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.visited.insert(node.id);
        let identity = (node.id.generation() as u64) << 32 | node.id.slot() as u64;
        let root = div().id(("gpuio-document", identity)).w_full();
        let (mut root, states) = apply_styles(root, &node.style, interaction, false);
        let [focused, hover, pressed, _, _, _, _] = states;
        if let Some(style) = focused
            && self
                .documents
                .get(&node.id)
                .is_some_and(|state| state.focused(window, cx))
        {
            gpui::Refineable::refine(root.style(), &style);
        }
        if let Some(style) = hover {
            root = root.hover(move |_| style);
        }
        if let Some(style) = pressed {
            root = root.active(move |_| style);
        }
        let user_selectable = node
            .style
            .iter()
            .filter_map(|style| match style {
                Style::Fields(fields) => fields.iter().rev().find_map(|field| match field {
                    Field::UserSelect(value) => Some(*value),
                    _ => None,
                }),
                _ => None,
            })
            .next_back()
            .or(interaction.selectable)
            .unwrap_or(true);
        let selection_color = node
            .style
            .iter()
            .filter_map(|style| match style {
                Style::Fields(fields) => fields.iter().rev().find_map(|field| match field {
                    Field::SelectionColor(value) => Some(super::color(value)),
                    _ => None,
                }),
                _ => None,
            })
            .next_back()
            .or(interaction.selection_color);
        let highlight = self.document_highlight(tree, node.id);
        let highlight_identity = self.highlight_documents.clone();
        let Some(state) = self.documents.get_mut(&node.id) else {
            return super::highlight_style::Frame::new(
                root.child("Document unavailable"),
                node,
                &self.focus,
            )
            .into_any_element();
        };
        let Some(lease) = &state.lease else {
            return super::highlight_style::Frame::new(
                root.child("Document released or from another application"),
                node,
                &self.focus,
            )
            .into_any_element();
        };
        let config = node.document.clone().unwrap();
        let presentation = state.presentation.get_or_insert_with(|| {
            let root = cx.weak_entity();
            cx.new(|cx| {
                Presentation::new(
                    root,
                    node.id,
                    lease.clone(),
                    config.clone(),
                    highlight_identity,
                    window,
                    cx,
                )
            })
        });
        let images = state
            .images
            .iter()
            .filter_map(|(url, handle)| {
                crate::image_host::image(handle, window, cx)
                    .ok()
                    .flatten()
                    .map(|image| (url.clone(), image))
            })
            .collect();
        presentation.update(cx, |state, cx| {
            state.highlight = highlight;
            if state.user_selectable != user_selectable {
                state.user_selectable = user_selectable;
                state.editor.update(cx, |editor, cx| {
                    editor.set_user_selectable(user_selectable, cx);
                });
                if let Some(markdown) = &state.markdown {
                    markdown.update(cx, |markdown, cx| {
                        markdown.set_selectable(user_selectable, cx);
                    });
                }
                cx.notify();
            }
            if state.selection_color != selection_color {
                state.selection_color = selection_color;
                state.editor.update(cx, |editor, _| {
                    editor.set_editor_style(editor_style(state.config.dark, selection_color));
                });
                cx.notify();
            }
            let images: BTreeMap<String, Arc<gpui::RenderImage>> = images;
            let changed = images.len() != state.images.0.len()
                || images.iter().any(|(url, image)| {
                    state
                        .images
                        .0
                        .get(url)
                        .is_none_or(|old| !Arc::ptr_eq(old, image))
                });
            state.images = document_markdown::Images(images);
            if changed {
                state.markdown_extensions = document_markdown::extensions(
                    document_markdown::Images(state.images.0.clone()),
                );
                if let Some(markdown) = &state.markdown {
                    markdown.update(cx, |markdown, cx| {
                        markdown.set_markdown_extensions(
                            Arc::new(state.markdown_extensions.clone()),
                            cx,
                        );
                    });
                }
                state.invalidate_row(cx);
                cx.notify();
            }
            state.refresh(config, cx);
            state.refresh_diff(
                node.document_diff.clone(),
                node.document_diff_epoch,
                node.handler,
                window,
                cx,
            );
            state.install_highlight(cx);
        });
        let presentation = presentation.clone();
        let weak = presentation.downgrade();
        let manager = self.focus.clone();
        let id = node.id;
        let root = root.relative().child(presentation).child(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    if bounds.size.width > px(0.)
                        && bounds.size.height > px(0.)
                        && bounds.intersects(&window.content_mask().bounds)
                        && let Some(p) = weak.upgrade()
                    {
                        let handle = p.read(cx).primary_focus(cx);
                        let focused = handle.is_focused(window);
                        manager.borrow_mut().record(id, handle, true, focused);
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
        gpui_base::text_selection_scope(
            self.focus.borrow().selection_scope(node.id),
            super::highlight_style::Frame::new(root, node, &self.focus),
        )
        .into_any_element()
    }
}

#[cfg(feature = "native-tests")]
#[path = "document_test.rs"]
pub(crate) mod test;

#[cfg(feature = "native-image-tests")]
#[path = "highlight_document_test.rs"]
pub(crate) mod highlight_test;

fn markdown_style(dark: bool, selection: Option<gpui::Hsla>) -> gpui_base::TextViewStyle {
    let (foreground, background, border, link) = if dark {
        (0xe6e7ed, 0x1b1e26, 0x2b2f39, 0x93c5fd)
    } else {
        (0x262832, 0xf3f3f1, 0xe0e1df, 0x1d4ed8)
    };
    let style = gpui_base::TextViewStyle::default()
        .with_dark(dark)
        .with_foreground(gpui::rgb(foreground).into())
        .with_link(gpui::rgb(link).into())
        .with_code_background(gpui::rgb(background).into())
        // Base's default inline-code style already has an explicit light
        // background, so the code-background fallback alone cannot theme it.
        .with_inline_code(gpui::HighlightStyle {
            background_color: Some(gpui::rgb(background).into()),
            ..Default::default()
        })
        .with_border(gpui::rgb(border).into());
    if let Some(selection) = selection {
        style.with_selection(selection)
    } else {
        style
    }
}
fn editor_style(dark: bool, selection: Option<gpui::Hsla>) -> gpui_base::input::InputEditorStyle {
    let style = markdown_style(dark, selection);
    let mut editor = gpui_base::input::InputEditorStyle {
        foreground: style.foreground(),
        background: style.code_background(),
        border: style.border(),
        ..Default::default()
    };
    if let Some(selection) = selection {
        editor.selection = selection;
    }
    editor
}

#[path = "document_file_headers.rs"]
mod file_headers;
