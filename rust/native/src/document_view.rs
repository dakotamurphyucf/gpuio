//! Mounted source leases and disposable native document presentations.
use super::{Interaction, View, apply_styles};
use crate::{
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
    snapshot: Arc<Snapshot>,
    job: Result<document_host::Handle, document_jobs::Error>,
    markdown: Option<Entity<TextViewState>>,
    editor: Entity<EditorState>,
    code_highlighter: Option<Arc<gpui_base::text::CodeBlockHighlighterFn>>,
    search: crate::document_search::Matches,
    search_index: Option<usize>,
    runs: Arc<Vec<crate::document_highlight::Run>>,
    diff: Option<crate::document_diff::Diff>,
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
            snapshot,
            job,
            editor,
            markdown: None,
            code_highlighter: None,
            search: Default::default(),
            search_index: None,
            runs: Arc::default(),
            diff: None,
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
        self.editor.update(cx, |editor, _| {
            editor.set_bridge_decorator(Rc::new(move |element, state, _, cx| {
                let focus = state.focus_handle(cx);
                let presentation = presentation.clone();
                let element = element
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
            if snapshot.generation != self.snapshot.generation {
                self.page_start = 0;
                self.previous_pages.clear();
                self.source_mode = false;
                self.collapsed = config.initially_collapsed;
            }
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
    fn show_source(
        &mut self,
        runs: Vec<crate::document_highlight::Run>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.runs = Arc::new(runs);
        self.show_page(window, cx);
    }
    fn show_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let page = page(&self.snapshot, self.page_start);
        self.page_end = page.end;
        let runs = Arc::new(
            self.runs
                .iter()
                .cloned()
                .filter_map(|mut run| {
                    run.bytes =
                        run.bytes.start.max(self.page_start)..run.bytes.end.min(self.page_end);
                    if run.bytes.is_empty() {
                        return None;
                    }
                    run.bytes = run.bytes.start - self.page_start..run.bytes.end - self.page_start;
                    Some(run)
                })
                .collect::<Vec<_>>(),
        );
        let origin = self
            .snapshot
            .text
            .byte_to_line_idx(self.page_start, ropey::LineType::LF);
        let final_line = self
            .snapshot
            .text
            .byte_to_line_idx(self.page_end, ropey::LineType::LF);
        let folds: Vec<_> = self.diff.as_ref().map_or_else(Vec::new, |diff| {
            diff.hunks
                .iter()
                .filter(|hunk| {
                    hunk.start >= origin && hunk.end <= final_line && hunk.end > hunk.start + 2
                })
                .map(|hunk| {
                    gpui_base::input::FoldRange::new(hunk.start - origin, hunk.end - origin)
                })
                .collect()
        });
        self.source_lines = page.text.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let old = self.installed.as_ref();
        self.editor.update(cx, |state, cx| {
            let mut selection = state.bridge_selection();
            let selected_end = selection.0.max(selection.1);
            let preserve = self.page_start == self.installed_page_start
                && old.is_some_and(|old| old.generation == self.snapshot.generation)
                && selected_end <= page.text.len()
                && selected_end <= state.value().len()
                && state.value().get(..selected_end) == page.text.get(..selected_end);
            if !preserve {
                selection = (0, 0);
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
            state.set_line_number_offset(
                self.snapshot
                    .text
                    .byte_to_line_idx(self.page_start, ropey::LineType::LF),
                cx,
            );
            state.set_editor_style(editor_style(self.config.dark));
            state.set_search_query(self.config.search.clone(), false, cx);
        });
        self.source_mode = true;
        self.installed_page_start = self.page_start;
        self.installed = Some(self.snapshot.clone());
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
                            self.show_page(window, cx);
                        } else {
                            self.source_mode = false;
                            self.editor.update(cx, |state, cx| {
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
        self.previous_pages.push(self.page_start);
        self.page_start = range.start;
        self.show_page(window, cx);
        self.editor.update(cx, |state, cx| {
            state.bridge_select(0, range.len(), cx);
            window.focus(&state.focus_handle(cx), cx);
        });
        self.invalidate_row(cx);
        cx.notify();
    }
    fn navigation_at_caret(&self, cx: &App) -> Option<Navigation> {
        let caret = self.page_start + self.editor.read(cx).bridge_selection().1;
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
            && self.page_end < self.snapshot.text.len()
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
        if let Some(navigation) = navigation.clone() {
            toolbar = toolbar.child(self.button(
                "document-location",
                "Go to line",
                move |this, _, cx| this.navigate(navigation.clone(), cx),
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
            && self.page_end < self.snapshot.text.len()
        {
            order.push(self.buttons["document-next"].clone());
        }
        if self.source_mode && self.markdown.is_some() {
            order.push(self.buttons["document-rendered"].clone());
        }
        if navigation.is_some() {
            order.push(self.buttons["document-location"].clone());
        }
        if !self.collapsed && self.installed.is_some() {
            order.push(self.primary_focus(cx));
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
        if let Some(error) = &self.error {
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
                .style(markdown_style(self.config.dark))
                .selectable(true)
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
            if let Some(installed) = self.installed.as_ref()
                && (self.page_start > 0 || self.page_end < installed.text.len())
            {
                root = root.child(format!(
                    "Source bytes {}–{} of {}",
                    self.page_start,
                    self.page_end,
                    installed.text.len()
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
        let (root, _) = apply_styles(root, &node.style, interaction, false);
        let highlight = self.document_highlight(tree, node.id);
        let highlight_identity = self.highlight_documents.clone();
        let Some(state) = self.documents.get_mut(&node.id) else {
            return root.child("Document unavailable").into_any_element();
        };
        let Some(lease) = &state.lease else {
            return root
                .child("Document released or from another application")
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
            state.install_highlight(cx);
        });
        let presentation = presentation.clone();
        let weak = presentation.downgrade();
        let manager = self.focus.clone();
        let id = node.id;
        root.relative()
            .child(presentation)
            .child(
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

fn markdown_style(dark: bool) -> gpui_base::TextViewStyle {
    let (foreground, background, border, link) = if dark {
        (0xe6e7ed, 0x1b1e26, 0x2b2f39, 0x93c5fd)
    } else {
        (0x262832, 0xf3f3f1, 0xe0e1df, 0x1d4ed8)
    };
    gpui_base::TextViewStyle::default()
        .with_dark(dark)
        .with_foreground(gpui::rgb(foreground).into())
        .with_link(gpui::rgb(link).into())
        .with_code_background(gpui::rgb(background).into())
        .with_border(gpui::rgb(border).into())
}
fn editor_style(dark: bool) -> gpui_base::input::InputEditorStyle {
    let style = markdown_style(dark);
    gpui_base::input::InputEditorStyle {
        foreground: style.foreground(),
        background: style.code_background(),
        border: style.border(),
        ..Default::default()
    }
}
