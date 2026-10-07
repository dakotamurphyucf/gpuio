use gpui::{Corners, prelude::*};
use std::{
    ops::Range,
    rc::Rc,
    sync::{Arc, Mutex},
};

use gpui::{
    App, BorderStyle, Bounds, ClickEvent, CursorStyle, Edges, Element, ElementId, GlobalElementId,
    Half, HighlightStyle, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId,
    MouseButton, MouseClickEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    SharedString, StyledText, TextLayout, TextRun, TextStyle, Window, point, px, quad,
};

use crate::{
    GlobalState, TextSelection,
    input::Selection,
    text::TextViewMultiClickKind,
    text::node::LinkMark,
    text::selection::word_range_at,
    text::state::LineSpan,
    text::text_view::{LinkClickHandlerFn, handle_link_click},
};

/// The style applied to one range of inline text.
///
/// A [`HighlightStyle`] carries no font family, so the family an inline code
/// span is set in rides beside it; `None` keeps the family of the enclosing
/// text style.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct InlineHighlight {
    pub(super) style: HighlightStyle,
    pub(super) font_family: Option<SharedString>,
    pub(super) font_size_scale: Option<f32>,
}

impl InlineHighlight {
    /// Layers `other` over `self`, the way [`HighlightStyle::highlight`] does.
    fn highlight(mut self, other: &InlineHighlight) -> Self {
        self.style = self.style.highlight(other.style);
        if other.font_family.is_some() {
            self.font_family = other.font_family.clone();
        }
        if other.font_size_scale.is_some() {
            self.font_size_scale = other.font_size_scale;
        }
        self
    }
}

impl From<HighlightStyle> for InlineHighlight {
    fn from(style: HighlightStyle) -> Self {
        Self {
            style,
            font_family: None,
            font_size_scale: None,
        }
    }
}

/// Merges two highlight lists over one text into non-overlapping ranges,
/// cutting at every endpoint of every input range. Same sweep as
/// [`gpui::combine_highlights`], for [`InlineHighlight`] payloads.
pub(super) fn combine_highlights(
    a: impl IntoIterator<Item = (Range<usize>, InlineHighlight)>,
    b: impl IntoIterator<Item = (Range<usize>, InlineHighlight)>,
) -> Vec<(Range<usize>, InlineHighlight)> {
    let mut endpoints = Vec::new();
    let mut highlights = Vec::new();
    for (range, highlight) in a.into_iter().chain(b) {
        if !range.is_empty() {
            let id = highlights.len();
            endpoints.push((range.start, id, true));
            endpoints.push((range.end, id, false));
            highlights.push(highlight);
        }
    }
    endpoints.sort_unstable_by_key(|(position, _, _)| *position);

    let mut combined = Vec::new();
    let mut active: Vec<usize> = Vec::new();
    let mut ix = 0;
    for (position, id, is_start) in endpoints {
        if position > ix && !active.is_empty() {
            let style = active.iter().fold(InlineHighlight::default(), |acc, id| {
                acc.highlight(&highlights[*id])
            });
            combined.push((ix..position, style));
        }
        ix = position;
        if is_start {
            active.push(id);
        } else {
            active.retain(|active_id| *active_id != id);
        }
    }
    combined
}

/// Builds the [`TextRun`]s for `text_len` bytes of inline text: each
/// highlight refines `default_style` over its range, and a highlight that
/// names a font family shapes its run in that family.
pub(super) fn text_runs(
    text_len: usize,
    default_style: &TextStyle,
    highlights: &[(Range<usize>, InlineHighlight)],
) -> Vec<TextRun> {
    let mut runs = Vec::with_capacity(highlights.len() * 2 + 1);
    let mut ix = 0;
    for (range, highlight) in highlights {
        if ix < range.start {
            runs.push(default_style.clone().to_run(range.start - ix));
        }
        let mut run = default_style
            .clone()
            .highlight(highlight.style)
            .to_run(range.len());
        if let Some(family) = &highlight.font_family {
            run.font.family = family.clone();
        }
        runs.push(run);
        ix = range.end;
    }
    if ix < text_len {
        runs.push(default_style.to_run(text_len - ix));
    }
    runs
}

/// Splits text into contiguous ranges sharing one font size. GPUI runs can
/// vary the font but not its size, so each range needs its own shaped line.
pub(super) fn text_size_ranges(
    text_len: usize,
    highlights: &[(Range<usize>, InlineHighlight)],
) -> Vec<(Range<usize>, f32)> {
    let mut ranges: Vec<(Range<usize>, f32)> = Vec::new();
    let mut push = |range: Range<usize>, scale: f32| {
        if range.is_empty() {
            return;
        }
        if let Some((last, last_scale)) = ranges.last_mut()
            && *last_scale == scale
            && last.end == range.start
        {
            last.end = range.end;
        } else {
            ranges.push((range, scale));
        }
    };
    let mut cursor = 0;
    for (range, highlight) in highlights {
        push(cursor..range.start, 1.);
        push(range.clone(), highlight.font_size_scale.unwrap_or(1.));
        cursor = range.end;
    }
    push(cursor..text_len, 1.);
    ranges
}

/// A inline element used to render a inline text and support selectable.
///
/// All text in TextView (including the CodeBlock) used this for text rendering.
#[derive(Clone, Copy, PartialEq, Eq)]
enum InlineInteraction {
    Text,
    AtomicObject,
}

pub(super) struct Inline {
    id: ElementId,
    text: SharedString,
    links: Rc<Vec<(Range<usize>, LinkMark)>>,
    highlights: Vec<(Range<usize>, InlineHighlight)>,
    styled_text: StyledText,
    /// The resolved style from a parent deferred layout, when there is one.
    text_style: Option<TextStyle>,
    paint_origin: Option<Point<Pixels>>,
    selection_bounds: Option<Bounds<Pixels>>,
    selection_source: Option<(Arc<Mutex<InlineState>>, Range<usize>)>,
    range_backgrounds: Option<(Rc<dyn crate::input::RangeBackgrounds>, Range<usize>)>,
    style_backgrounds: Vec<crate::input::RangeBackground>,
    interaction: InlineInteraction,
    semantic_sink: Option<(super::inline_semantics::Collector, usize)>,
    suppress_semantics: bool,
    link_click_handler: Option<Arc<LinkClickHandlerFn>>,

    state: Arc<Mutex<InlineState>>,
}

pub(super) struct InlinePrepaint {
    hitbox: Hitbox,
    semantics: Vec<gpui::AnyElement>,
}

/// Partition rendered text in reading order. Adjacent style runs of the same
/// link remain one accessible action; ordinary text is not duplicated beside it.
pub(super) fn accessible_runs(
    text: &str,
    links: &[(Range<usize>, LinkMark)],
) -> Vec<(Range<usize>, Option<LinkMark>)> {
    let mut links = links.to_vec();
    links.sort_by_key(|(range, _)| range.start);
    let mut runs: Vec<(Range<usize>, Option<LinkMark>)> = Vec::new();
    let mut cursor = 0;
    for (range, link) in links {
        if range.start < cursor
            || range.is_empty()
            || !text.is_char_boundary(range.start)
            || !text.is_char_boundary(range.end)
        {
            continue;
        }
        if range.start > cursor {
            runs.push((cursor..range.start, None));
        }
        cursor = range.end;
        if let Some((previous, Some(previous_link))) = runs.last_mut()
            && previous.end == range.start
            && *previous_link == link
        {
            previous.end = range.end;
        } else {
            runs.push((range, Some(link)));
        }
    }
    if cursor < text.len() {
        runs.push((cursor..text.len(), None));
    }
    runs
}

/// The inline text state, used RefCell to keep the selection state.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct InlineState {
    hovered_index: Option<usize>,
    /// The text that actually rendering, matched with selection.
    pub(super) text: SharedString,
    pub(super) selection: Option<Selection>,
    pub(super) displayed_fragment: Option<super::DisplayedFragment>,
    pub(super) rendered_fragment: Option<super::rendered_text::RenderedFragment>,
}

impl InlineState {
    /// Save actually rendered text for selected text to use.
    pub(crate) fn set_text(&mut self, text: SharedString) {
        self.text = text;
    }
}

impl Inline {
    pub(super) fn new(
        id: impl Into<ElementId>,
        state: Arc<Mutex<InlineState>>,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, InlineHighlight)>,
        link_click_handler: Option<Arc<LinkClickHandlerFn>>,
    ) -> Self {
        let text = state
            .lock()
            .map(|state| state.text.clone())
            .unwrap_or_default();

        Self {
            id: id.into(),
            links: Rc::new(links),
            highlights,
            text: text.clone(),
            styled_text: StyledText::new(text),
            text_style: None,
            paint_origin: None,
            selection_bounds: None,
            selection_source: None,
            range_backgrounds: None,
            style_backgrounds: Vec::new(),
            interaction: InlineInteraction::Text,
            semantic_sink: None,
            suppress_semantics: false,
            link_click_handler,
            state,
        }
    }

    pub(super) fn semantic_sink(
        mut self,
        sink: super::inline_semantics::Collector,
        slot: usize,
    ) -> Self {
        self.semantic_sink = Some((sink, slot));
        self
    }

    pub(super) fn suppress_semantics(mut self, suppress: bool) -> Self {
        self.suppress_semantics = suppress;
        self
    }

    /// Use the resolved style captured by a deferred parent layout.
    pub(super) fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    /// Glyph presentation inside an atomic custom object. The object owns
    /// selection, links and input; it must not acquire a second text controller.
    pub(super) fn passive(mut self) -> Self {
        self.interaction = InlineInteraction::AtomicObject;
        self
    }

    /// Preserve the shared inline-flow baseline through GPUI's element-bound snapping.
    pub(super) fn paint_origin(mut self, origin: Point<Pixels>) -> Self {
        self.paint_origin = Some(origin);
        self
    }

    pub(super) fn selection_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.selection_bounds = Some(bounds);
        self
    }

    pub(super) fn selection_source(
        mut self,
        state: Arc<Mutex<InlineState>>,
        range: Range<usize>,
    ) -> Self {
        self.selection_source = Some((state, range));
        self
    }

    fn semantic_elements(
        &self,
        owner: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<gpui::AnyElement> {
        if self.suppress_semantics {
            return Vec::new();
        }
        if let Some((sink, _)) = &self.semantic_sink {
            if !sink.enabled() {
                return Vec::new();
            }
        } else if self.links.is_empty() || !window.is_a11y_active() {
            return Vec::new();
        }
        let layout = self.styled_text.layout();
        let height = layout.line_height();
        let view = GlobalState::global(cx).text_view_state().cloned();
        let mut elements = Vec::new();
        for (range, link) in accessible_runs(&self.text, &self.links) {
            let (Some(start), Some(end)) = (
                layout.position_for_index(range.start),
                layout.position_for_index(range.end),
            ) else {
                continue;
            };
            let area = if start.y == end.y {
                Bounds::from_corners(
                    point(start.x.min(end.x), start.y),
                    point(start.x.max(end.x), end.y + height),
                )
            } else {
                Bounds::from_corners(
                    point(bounds.left(), start.y),
                    point(bounds.right(), end.y + height),
                )
            };
            if let Some((sink, slot)) = &self.semantic_sink {
                sink.push(*slot, area, &self.text[range], link);
                continue;
            }
            let id: SharedString = format!(
                "semantic-{}-{}-{}",
                range.start,
                range.end,
                link.as_ref().map_or("", |link| link.url.as_ref())
            )
            .into();
            let text = self.text[range].to_owned();
            let mut element = gpui::div().id(id).w(area.size.width).h(area.size.height);
            if let Some(link) = link {
                let text = if text.trim().is_empty() {
                    link.url.to_string()
                } else {
                    text
                };
                let active = view.as_ref().is_some_and(|view| {
                    view.update(cx, |state, _| {
                        if link.source_start.is_none()
                            || state.link_navigation.active != link.source_start
                        {
                            return false;
                        }
                        let Some(owner) = owner else {
                            return false;
                        };
                        if state.link_active_owner.is_none() {
                            state.link_active_owner = Some(owner.clone());
                        }
                        state.link_active_owner.as_ref() == Some(owner)
                    })
                });
                let activation_target = view
                    .as_ref()
                    .map(|view| (view.downgrade(), link.source_start));
                let focus_target = view
                    .as_ref()
                    .filter(|_| link.source_start.is_some())
                    .map(|view| (view.downgrade(), link.clone()));
                let url = link.url;
                let metadata_url = url.clone();
                let handler = self.link_click_handler.clone();
                element = element
                    .role(gpui::Role::Link)
                    .aria_label(text)
                    .when(active, |element| element.aria_active_descendant())
                    .when_some(focus_target, |element, (view, link)| {
                        element.on_a11y_action(
                            gpui::AccessibleAction::Focus,
                            move |_, window, cx| {
                                let _ = view
                                    .update(cx, |state, cx| state.focus_link(&link, window, cx));
                            },
                        )
                    })
                    .a11y_synthetic_children(move |builder| {
                        builder.parent_node().set_url(metadata_url.to_string())
                    })
                    .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                        if !super::inline_semantics::activation_allowed(&activation_target, cx) {
                            return;
                        }
                        handle_link_click(
                            &handler,
                            url.clone(),
                            ClickEvent::Keyboard(gpui::KeyboardClickEvent {
                                bounds: area,
                                ..Default::default()
                            }),
                            window,
                            cx,
                        );
                    });
            } else {
                element = element.role(gpui::Role::Label).aria_value(text);
            }
            let mut element = element.into_any_element();
            element.prepaint_as_root(
                area.origin,
                gpui::size(
                    gpui::AvailableSpace::Definite(area.size.width),
                    gpui::AvailableSpace::Definite(area.size.height),
                ),
                window,
                cx,
            );
            elements.push(element);
        }
        elements
    }

    /// Get link at given mouse position.
    fn link_for_position(
        layout: &TextLayout,
        links: &Vec<(Range<usize>, LinkMark)>,
        position: Point<Pixels>,
    ) -> Option<LinkMark> {
        let offset = layout.index_for_position(position).ok()?;
        for (range, link) in links.iter() {
            if range.contains(&offset) {
                return Some(link.clone());
            }
        }

        None
    }

    /// Paint selected bounds for debug.
    #[allow(unused)]
    fn paint_selected_bounds(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        window.paint_quad(gpui::PaintQuad {
            bounds,
            background: gpui::hsla(0.58, 0.85, 0.62, 0.01).into(),
            corner_radii: Corners::default(),
            border_color: gpui::transparent_black(),
            border_style: BorderStyle::default(),
            border_widths: gpui::Edges::all(px(0.)),
        });
    }

    fn layout_selections(
        &self,
        text_layout: &TextLayout,
        bounds: &Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> (bool, bool, Option<Selection>) {
        let Some(text_view_state) = GlobalState::global(cx).text_view_state() else {
            return (false, false, None);
        };

        let text_view_state = text_view_state.read(cx);
        let is_selectable = text_view_state.is_selectable();
        if !is_selectable {
            return (false, false, None);
        }

        if text_view_state.is_all_selected() {
            return (is_selectable, true, Some((0..self.text.len()).into()));
        }

        if text_view_state.preserve_inline_selection {
            let selection = if let Some((source, range)) = &self.selection_source {
                source
                    .lock()
                    .ok()
                    .and_then(|state| state.selection)
                    .and_then(|selection| {
                        let start = selection.start.max(range.start);
                        let end = selection.end.min(range.end);
                        (start < end)
                            .then(|| Selection::new(start - range.start, end - range.start))
                    })
            } else {
                self.state.lock().ok().and_then(|state| state.selection)
            };
            return (true, selection.is_some(), selection);
        }

        if let Some(selection) = text_view_state.multi_click_selection() {
            if selection.kind == TextViewMultiClickKind::Line {
                return (
                    true,
                    true,
                    selection
                        .line_bounds
                        .filter(|row| row.contains(&bounds.center()))
                        .map(|_| Selection::new(0, self.text.len())),
                );
            }
            return (
                is_selectable,
                true,
                selection_for_multi_click(
                    &self.text,
                    text_layout,
                    *bounds,
                    selection.pos,
                    selection.kind,
                )
                .map(Selection::from),
            );
        }

        let Some((selection_start, selection_end)) = text_view_state.selection_points(cx) else {
            return (is_selectable, false, None);
        };
        let line_height = window.line_height();

        // Use for debug selection bounds
        // self.paint_selected_bounds(Bounds::from_corners(selection_start, selection_end), window, cx);

        // NOTE: the selection is computed purely from the geometric band
        // (`selection_start`..`selection_end`), NOT from what is currently
        // visible. Every glyph of a *painted* element is laid out (its
        // `position_for_index` is valid) even when it is scrolled out of, or
        // clipped by, an ancestor's viewport — the content mask only clips the
        // painted pixels. Because the copied text is derived from
        // `InlineState.selection`, gating the selection on `content_mask` here
        // used to drop scrolled-out-but-selected glyphs, so a selection taller
        // than the viewport (e.g. a long chat message, or a drag with
        // auto-scroll) copied only the portion that happened to be on screen.
        //
        // This does not resurrect the #2156 clipped-hit-testing behavior: a
        // selection can only START on visible text (window selection resolves
        // endpoints with hitbox hover testing against visible Inline bounds),
        // so the band's endpoints are always anchored to on-screen text.
        // Content that is merely `overflow_hidden`
        // (not scrolled) lies outside that band and is still excluded, while
        // the highlight quads painted for off-screen glyphs are clipped away by
        // GPUI's content mask as before.
        let mut selection: Option<Selection> = None;
        let mut offset = 0;
        let mut chars = self.text.chars().peekable();
        while let Some(c) = chars.next() {
            let Some(pos) = text_layout.position_for_index(offset) else {
                offset += c.len_utf8();
                continue;
            };

            let next_offset = offset + c.len_utf8();
            let mut char_width = line_height.half();
            if let Some(next_pos) = text_layout.position_for_index(next_offset) {
                if next_pos.y == pos.y {
                    char_width = next_pos.x - pos.x;
                }
            }

            let selection_pos = self
                .selection_bounds
                .map_or(pos, |bounds| point(pos.x, bounds.top()));
            let selection_height = self
                .selection_bounds
                .map_or(line_height, |bounds| bounds.size.height);
            if point_in_text_selection(
                selection_pos,
                char_width,
                selection_start,
                selection_end,
                selection_height,
            ) {
                if selection.is_none() {
                    selection = Some((offset..offset).into());
                }

                if let Some(selection) = selection.as_mut() {
                    selection.end = next_offset;
                }
            }

            offset = next_offset;
        }

        (true, true, selection)
    }

    fn text_line_bounds(
        &self,
        text_layout: &TextLayout,
        line_height: Pixels,
        mask_bounds: Bounds<Pixels>,
    ) -> Vec<Bounds<Pixels>> {
        let mut line_bounds = Vec::new();
        let mut current_line_y = None;
        let mut current_bounds: Option<Bounds<Pixels>> = None;
        let mut offset = 0;

        for c in self.text.chars() {
            let next_offset = offset + c.len_utf8();
            let Some(pos) = text_layout.position_for_index(offset) else {
                offset = next_offset;
                continue;
            };

            let mut char_width = line_height.half();
            if let Some(next_pos) = text_layout.position_for_index(next_offset) {
                if next_pos.y == pos.y {
                    char_width = next_pos.x - pos.x;
                }
            }

            let bounds = Bounds::from_corners(pos, point(pos.x + char_width, pos.y + line_height))
                .intersect(&mask_bounds);
            if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                if current_line_y == Some(pos.y) {
                    if let Some(current) = current_bounds.as_mut() {
                        *current = current.union(&bounds);
                    }
                } else {
                    if let Some(current) = current_bounds.take() {
                        line_bounds.push(current);
                    }
                    current_line_y = Some(pos.y);
                    current_bounds = Some(bounds);
                }
            }

            offset = next_offset;
        }

        if let Some(current) = current_bounds {
            line_bounds.push(current);
        }

        line_bounds
    }

    /// Paint the selection background.
    fn paint_selection(
        selection: &Selection,
        text_layout: &TextLayout,
        bounds: &Bounds<Pixels>,
        window: &mut Window,
        color: gpui::Hsla,
    ) {
        let mut start = selection.start;
        let mut end = selection.end;
        if end < start {
            std::mem::swap(&mut start, &mut end);
        }
        let Some(start_position) = text_layout.position_for_index(start) else {
            return;
        };
        let Some(end_position) = text_layout.position_for_index(end) else {
            return;
        };

        let line_height = text_layout.line_height();
        if start_position.y == end_position.y {
            window.paint_quad(quad(
                Bounds::from_corners(
                    start_position,
                    point(end_position.x, end_position.y + line_height),
                ),
                px(0.),
                color,
                Edges::default(),
                gpui::transparent_black(),
                BorderStyle::default(),
            ));
        } else {
            window.paint_quad(quad(
                Bounds::from_corners(
                    start_position,
                    point(bounds.right(), start_position.y + line_height),
                ),
                px(0.),
                color,
                Edges::default(),
                gpui::transparent_black(),
                BorderStyle::default(),
            ));

            if end_position.y > start_position.y + line_height {
                window.paint_quad(quad(
                    Bounds::from_corners(
                        point(bounds.left(), start_position.y + line_height),
                        point(bounds.right(), end_position.y),
                    ),
                    px(0.),
                    color,
                    Edges::default(),
                    gpui::transparent_black(),
                    BorderStyle::default(),
                ));
            }

            window.paint_quad(quad(
                Bounds::from_corners(
                    point(bounds.left(), end_position.y),
                    point(end_position.x, end_position.y + line_height),
                ),
                px(0.),
                color,
                Edges::default(),
                gpui::transparent_black(),
                BorderStyle::default(),
            ));
        }
    }
}

impl IntoElement for Inline {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Inline {
    type RequestLayoutState = ();
    type PrepaintState = InlinePrepaint;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        if self.suppress_semantics || self.semantic_sink.is_some() {
            return None;
        }
        Some(if self.links.is_empty() {
            gpui::accesskit::Role::Label
        } else {
            gpui::accesskit::Role::Group
        })
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        // StyledText is painted directly below, bypassing its Element wrapper.
        // Publish the same rendered text here; do not duplicate raw Markdown.
        if self.links.is_empty() {
            node.set_value(self.text.to_string());
        }
    }

    fn request_layout(
        &mut self,
        global_element_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let text_style = self
            .text_style
            .clone()
            .unwrap_or_else(|| window.text_style());
        let (source_state, source_range) = self
            .selection_source
            .as_ref()
            .map(|(state, range)| (state, range.clone()))
            .unwrap_or((&self.state, 0..self.text.len()));
        let fragment = source_state
            .lock()
            .ok()
            .and_then(|state| state.displayed_fragment.clone());
        self.range_backgrounds = fragment.and_then(|fragment| {
            if fragment.text().get(source_range.clone()) != Some(self.text.as_ref()) {
                return None;
            }
            let view = GlobalState::global(cx).text_view_state()?.read(cx);
            let layer = view.text_backgrounds.as_ref()?.layer(&fragment)?;
            Some((layer, source_range))
        });
        let mut runs = text_runs(self.text.len(), &text_style, &self.highlights);
        self.style_backgrounds.clear();
        if self.range_backgrounds.is_some() {
            // StyledText normally paints backgrounds and glyphs together. Move
            // only its backgrounds into our first pass so rounded prepared
            // washes sit above syntax/mark colors and below the glyphs.
            let mut offset = 0;
            for run in &mut runs {
                if let Some(color) = run.background_color.take() {
                    self.style_backgrounds.push(crate::input::RangeBackground {
                        bytes: offset..offset + run.len,
                        color,
                        radius: px(0.),
                    });
                }
                offset += run.len;
            }
        }

        self.styled_text = StyledText::new(self.text.clone()).with_runs(runs);
        let (layout_id, _) =
            self.styled_text
                .request_layout(global_element_id, inspector_id, window, cx);

        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let bounds = Bounds::new(self.paint_origin.unwrap_or(bounds.origin), bounds.size);
        self.styled_text
            .prepaint(id, inspector_id, bounds, &mut (), window, cx);

        // Report this element's laid-out extent so an ancestor TextView with
        // `max_lines` can snap its clip to a whole-line boundary. The state
        // stack also supplies the one-shot keyboard link reveal request.
        if let Some(text_view_state) = GlobalState::global(cx).text_view_state().cloned() {
            let state = text_view_state.read(cx);
            if self.interaction == InlineInteraction::Text
                && state.max_lines.is_some()
                && let Ok(mut line_spans) = state.line_spans.lock()
            {
                line_spans.push(LineSpan {
                    top: bounds.top(),
                    bottom: bounds.bottom(),
                    line_height: window.line_height(),
                });
            }
        }

        if self.semantic_sink.is_none()
            && let Some(view) = GlobalState::global(cx).text_view_state().cloned()
        {
            let target = {
                let state = view.read(cx);
                state.link_reveal.filter(|_| !state.link_reveal_claimed)
            };
            if let Some(target) = target {
                let layout = self.styled_text.layout();
                if let Some((range, _)) = self
                    .links
                    .iter()
                    .find(|(_, link)| link.source_start == Some(target))
                    && let Some(start) = layout.position_for_index(range.start)
                {
                    // Reveal the start of the logical link, including when its
                    // containing list/table block is taller than the viewport.
                    window.request_autoscroll(Bounds::from_corners(
                        start,
                        point(start.x + px(2.), start.y + layout.line_height()),
                    ));
                    view.update(cx, |state, _| state.link_reveal_claimed = true);
                }
            }
        }

        InlinePrepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
            semantics: self.semantic_elements(id, bounds, window, cx),
        }
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let bounds = Bounds::new(self.paint_origin.unwrap_or(bounds.origin), bounds.size);
        let current_view = window.current_view();
        for element in &mut prepaint.semantics {
            element.paint(window, cx);
        }
        let hitbox = &prepaint.hitbox;
        let text_layout = self.styled_text.layout().clone();
        if let Some((layers, source)) = &self.range_backgrounds {
            let align = self
                .text_style
                .as_ref()
                .map_or_else(|| window.text_style().text_align, |style| style.text_align);
            super::backgrounds::paint(
                &self.style_backgrounds,
                0..self.text.len(),
                &text_layout,
                align,
                window,
            );
            super::backgrounds::paint(layers.ranges(), source.clone(), &text_layout, align, window);
        }
        self.styled_text
            .paint(global_id, None, bounds, &mut (), &mut (), window, cx);

        if self.semantic_sink.is_none()
            && GlobalState::global(cx)
                .text_view_state()
                .is_some_and(|view| view.read(cx).max_lines.is_some())
        {
            for (range, link) in accessible_runs(&self.text, &self.links) {
                let Some(link) = link else { continue };
                if let (Some(start), Some(end)) = (
                    text_layout.position_for_index(range.start),
                    text_layout.position_for_index(range.end),
                ) {
                    let area = if start.y == end.y {
                        Bounds::from_corners(
                            point(start.x.min(end.x), start.y),
                            point(start.x.max(end.x), end.y + text_layout.line_height()),
                        )
                    } else {
                        Bounds::from_corners(
                            point(bounds.left(), start.y),
                            point(bounds.right(), end.y + text_layout.line_height()),
                        )
                    };
                    super::inline_semantics::record_preview_link(&link, area, window, cx);
                }
            }
        }

        if self.interaction != InlineInteraction::Text {
            if let Some(view) = GlobalState::global(cx).text_view_state().cloned() {
                view.update(cx, |state, _| {
                    if state.is_selectable() && state.rendered_text().is_some() {
                        state.selection_adapter.register_text_endpoint(
                            text_layout.clone(),
                            bounds,
                            None,
                        );
                    }
                });
            }
            return;
        }

        if self.semantic_sink.is_none()
            && let Some(view) = GlobalState::global(cx).text_view_state()
        {
            let view = view.read(cx);
            if view.focus_handle().is_focused(window)
                && let Some(active) = view.link_navigation.active
            {
                for (range, link) in accessible_runs(&self.text, &self.links) {
                    if link.as_ref().and_then(|link| link.source_start) != Some(active) {
                        continue;
                    }
                    if let (Some(start), Some(end)) = (
                        text_layout.position_for_index(range.start),
                        text_layout.position_for_index(range.end),
                    ) {
                        let focus_bounds = if start.y == end.y {
                            Bounds::from_corners(
                                point(start.x.min(end.x), start.y),
                                point(start.x.max(end.x), end.y + text_layout.line_height()),
                            )
                        } else {
                            Bounds::from_corners(
                                point(bounds.left(), start.y),
                                point(bounds.right(), end.y + text_layout.line_height()),
                            )
                        };
                        window.paint_quad(quad(
                            focus_bounds,
                            px(2.),
                            gpui::transparent_black(),
                            Edges {
                                top: px(1.5),
                                right: px(1.5),
                                bottom: px(1.5),
                                left: px(1.5),
                            },
                            view.text_view_style.link(),
                            BorderStyle::default(),
                        ));
                    }
                }
            }
        }

        // layout selections
        let (is_selectable, is_selection, selection) =
            self.layout_selections(&text_layout, &bounds, window, cx);

        let Ok(mut state) = self.state.lock() else {
            return;
        };

        state.selection = selection;
        if let Some((source, range)) = &self.selection_source
            && let Some(selection) = selection
            && let Ok(mut source) = source.lock()
        {
            let start = range.start + selection.start;
            let end = range.start + selection.end;
            source.selection = Some(match source.selection {
                Some(previous) => Selection::new(previous.start.min(start), previous.end.max(end)),
                None => Selection::new(start, end),
            });
        }

        if is_selection || is_selectable {
            window.set_cursor_style(CursorStyle::IBeam, &hitbox);
        }

        // link cursor pointer
        let mouse_position = window.mouse_position();
        if let Some(_) = Self::link_for_position(&text_layout, &self.links, mouse_position) {
            window.set_cursor_style(CursorStyle::PointingHand, &hitbox);
        }

        if let Some(selection) = &state.selection {
            let color = GlobalState::global(cx)
                .text_view_state()
                .map(|state| state.read(cx).text_view_style.selection())
                .unwrap_or_else(|| crate::Theme::global(cx).tokens.colors.selection);
            Self::paint_selection(selection, &text_layout, &bounds, window, color);
        }

        if is_selectable {
            if let Some(text_view_state) = GlobalState::global(cx).text_view_state().cloned() {
                let text_bounds = self.text_line_bounds(
                    &text_layout,
                    text_layout.line_height(),
                    window.content_mask().bounds,
                );
                let ordinary_fragment = state.rendered_fragment.clone();
                text_view_state.update(cx, |state, _| {
                    state.selection_adapter.register_inline(text_bounds);
                    let Some(projection) = state.rendered_text() else {
                        // Unbounded legacy TextViews have no endpoint contract;
                        // do not retain an unused shaped-layout map for them.
                        return;
                    };
                    let (owner, range) = self
                        .selection_source
                        .as_ref()
                        .map(|(owner, range)| (owner, range.clone()))
                        .unwrap_or((&self.state, 0..self.text.len()));
                    // `self.state` is already locked above. Rich-flow fragments
                    // have a separate canonical owner; ordinary runs reuse it.
                    let fragment = if Arc::ptr_eq(owner, &self.state) {
                        ordinary_fragment
                    } else {
                        owner
                            .lock()
                            .ok()
                            .and_then(|owner| owner.rendered_fragment.clone())
                    };
                    let fragment = fragment
                        .and_then(|fragment| fragment.slice(range))
                        .filter(|fragment| fragment.matches(&projection, &self.text));
                    state.selection_adapter.register_text_endpoint(
                        text_layout.clone(),
                        bounds,
                        fragment,
                    );
                });
            }

            window.on_mouse_event({
                let hitbox = hitbox.clone();
                let text_layout = text_layout.clone();
                let inline_state = self.state.clone();
                let text = self.text.clone();
                let text_view_state = GlobalState::global(cx).text_view_state().cloned();
                let line_bounds = self.selection_bounds;
                move |event: &MouseDownEvent, phase, window, cx| {
                    if !phase.bubble()
                        || !hitbox.is_hovered(window)
                        || event.button != MouseButton::Left
                    {
                        return;
                    }

                    if event.click_count == 3
                        && let Some(line_bounds) = line_bounds
                    {
                        GlobalState::suppress_text_selection(cx);
                        if let Some(view) = &text_view_state {
                            view.update(cx, |state, cx| {
                                state.set_multi_click_line(line_bounds, cx)
                            });
                        }
                        cx.notify(current_view);
                        return;
                    }

                    let kind = match event.click_count {
                        2 => TextViewMultiClickKind::Word,
                        3 => TextViewMultiClickKind::Paragraph,
                        _ => return,
                    };

                    let Some(range) = selection_for_multi_click(
                        &text,
                        &text_layout,
                        hitbox.bounds,
                        event.position,
                        kind,
                    ) else {
                        return;
                    };

                    let selected_text = text[range.clone()].to_string();

                    // This renderer owns multi-click selection. Prevent the
                    // window selection layer from handling the same press.
                    GlobalState::suppress_text_selection(cx);

                    if let Ok(mut inline_state) = inline_state.lock() {
                        inline_state.selection = Some(range.into());
                    }
                    if let Some(text_view_state) = &text_view_state {
                        text_view_state.update(cx, |state, cx| {
                            state.set_multi_click_selection(
                                event.position,
                                kind,
                                selected_text,
                                cx,
                            );
                        });
                    }
                    cx.notify(current_view);
                }
            });
        }

        // mouse move, update hovered link
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let text_layout = text_layout.clone();
            let mut hovered_index = state.hovered_index;
            move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() || !hitbox.is_hovered(window) {
                    return;
                }

                let current = hovered_index;
                let updated = text_layout.index_for_position(event.position).ok();
                //  notify update when hovering over different links
                if current != updated {
                    hovered_index = updated;
                    cx.notify(current_view);
                }
            }
        });

        if !is_selection {
            // click to open link
            window.on_mouse_event({
                let links = self.links.clone();
                let text_layout = text_layout.clone();
                let hitbox = hitbox.clone();
                let text_view_state = GlobalState::global(cx).text_view_state().cloned();
                let link_click_handler = self.link_click_handler.clone();

                move |event: &MouseUpEvent, phase, window, cx| {
                    if !phase.bubble() || !hitbox.is_hovered(window) {
                        return;
                    }
                    if text_view_state
                        .as_ref()
                        .is_some_and(|state| state.read(cx).has_selection(cx))
                    {
                        return;
                    }

                    if let Some(link) =
                        Self::link_for_position(&text_layout, &links, event.position)
                    {
                        TextSelection::end(window, cx);
                        cx.stop_propagation();
                        let click = ClickEvent::Mouse(MouseClickEvent {
                            down: MouseDownEvent {
                                button: event.button,
                                position: event.position,
                                modifiers: event.modifiers,
                                click_count: event.click_count,
                                first_mouse: false,
                            },
                            up: event.clone(),
                        });
                        handle_link_click(&link_click_handler, link.url, click, window, cx);
                    }
                }
            });
        }
    }
}

fn selection_for_multi_click(
    text: &str,
    text_layout: &TextLayout,
    bounds: Bounds<Pixels>,
    pos: Point<Pixels>,
    kind: TextViewMultiClickKind,
) -> Option<std::ops::Range<usize>> {
    if !bounds.contains(&pos) {
        return None;
    }

    let offset = text_layout.index_for_position(pos).ok()?;

    match kind {
        TextViewMultiClickKind::Word => word_range_at(text, offset),
        // Known limitation: a paragraph maps to a single Inline run here. When a
        // paragraph embeds an inline image it is split into multiple Inline runs,
        // so triple-click only selects the run on the clicked side of the image.
        TextViewMultiClickKind::Paragraph | TextViewMultiClickKind::Line => {
            (!text.is_empty()).then_some(0..text.len())
        }
    }
}

/// Check if a `pos` is within a `bounds`, considering multi-line selections.
pub(super) fn point_in_text_selection(
    pos: Point<Pixels>,
    char_width: Pixels,
    selection_start: Point<Pixels>,
    selection_end: Point<Pixels>,
    line_height: Pixels,
) -> bool {
    let point_in_line = |point: Point<Pixels>| point.y >= pos.y && point.y < pos.y + line_height;
    let top = selection_start.y.min(selection_end.y);
    let bottom = selection_start.y.max(selection_end.y);
    let x = pos.x + char_width.half();

    // Out of the vertical bounds
    if pos.y + line_height <= top || pos.y > bottom {
        return false;
    }

    // Treat the selection as single-line when both drag points fall within the
    // same rendered line, even if their y coordinates differ inside that line.
    if point_in_line(selection_start) && point_in_line(selection_end) {
        let left = selection_start.x.min(selection_end.x);
        let right = selection_start.x.max(selection_end.x);
        return x >= left && x <= right;
    }

    let (top_point, bottom_point) = if selection_start.y < selection_end.y {
        (selection_start, selection_end)
    } else {
        (selection_end, selection_start)
    };
    let is_top_line = point_in_line(top_point);
    let is_bottom_line = point_in_line(bottom_point);

    if is_top_line {
        return x >= top_point.x;
    } else if is_bottom_line {
        return x <= bottom_point.x;
    } else {
        return true;
    }
}

/// A platform text system for tests where the `Mono` family shapes twice as
/// wide as every other family, so a measurement that ignores the family of a
/// run comes out visibly short.
#[cfg(test)]
pub(super) mod test_draw {
    use gpui::{App, Context, IntoElement, Render, Styled as _, TestApp, Window, canvas, px};
    use std::{cell::RefCell, rc::Rc};

    struct Probe {
        body: Option<Box<dyn FnOnce(&mut Window, &mut App)>>,
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let body = self.body.take();
            canvas(
                move |_, window, cx| {
                    if let Some(body) = body {
                        body(window, cx);
                    }
                },
                |_, _, _, _| {},
            )
            .w(px(1000.))
            .h(px(1000.))
        }
    }

    /// Runs `f` inside a real prepaint pass and returns its value.
    ///
    /// Measuring an inline object lays its GPUI element out through the
    /// window, which GPUI only permits while a frame is being drawn, so a test
    /// that builds a real element cannot call the measurement helpers straight
    /// from `TestAppWindow::update`.
    pub(crate) fn in_prepaint<R: 'static>(
        app: &mut TestApp,
        f: impl FnOnce(&mut Window, &mut App) -> R + 'static,
    ) -> R {
        let slot: Rc<RefCell<Option<R>>> = Rc::new(RefCell::new(None));
        let out = slot.clone();
        let mut window = app.open_window(|_, _| Probe {
            body: Some(Box::new(move |window, cx| {
                *out.borrow_mut() = Some(f(window, cx));
            })),
        });
        window.draw();
        let value = slot.borrow_mut().take();
        value.expect("prepaint probe did not run")
    }
}

#[cfg(test)]
pub(super) mod test_fonts {
    use gpui::{
        Bounds, DevicePixels, Font, FontId, FontMetrics, FontRun, FontWeight, GlyphId, LineLayout,
        Pixels, PlatformTextSystem, RenderGlyphParams, ShapedGlyph, ShapedRun, Size,
        TextRenderingMode, point, px, size,
    };
    use std::borrow::Cow;

    pub(crate) const BODY: &str = "Body";
    pub(crate) const MONO: &str = "Mono";
    const BODY_ID: FontId = FontId(1);
    const MONO_ID: FontId = FontId(2);
    const BOLD_BODY_ID: FontId = FontId(3);
    const BOLD_MONO_ID: FontId = FontId(4);
    const UNITS_PER_EM: f32 = 1000.;

    pub(crate) struct WideMonoTextSystem;

    impl WideMonoTextSystem {
        /// Advance of one glyph in `font_id`, in em units.
        fn advance_units(font_id: FontId) -> f32 {
            match font_id {
                MONO_ID => 1000.,
                BOLD_MONO_ID => 1250.,
                BODY_ID => 500.,
                BOLD_BODY_ID => 750.,
                _ => 500.,
            }
        }

        /// Width of `text` shaped entirely in `family` at `font_size`.
        pub(crate) fn width_of(text: &str, family: &str, font_size: Pixels) -> Pixels {
            let font_id = if family == MONO { MONO_ID } else { BODY_ID };
            font_size * (Self::advance_units(font_id) / UNITS_PER_EM) * text.chars().count() as f32
        }
    }

    impl PlatformTextSystem for WideMonoTextSystem {
        fn add_fonts(&self, _fonts: Vec<Cow<'static, [u8]>>) -> anyhow::Result<()> {
            Ok(())
        }

        fn all_font_names(&self) -> Vec<String> {
            vec![BODY.into(), MONO.into()]
        }

        fn font_id(&self, descriptor: &Font) -> anyhow::Result<FontId> {
            Ok(
                match (
                    descriptor.family.as_ref() == MONO,
                    descriptor.weight == FontWeight::BOLD,
                ) {
                    (true, true) => BOLD_MONO_ID,
                    (true, false) => MONO_ID,
                    (false, true) => BOLD_BODY_ID,
                    (false, false) => BODY_ID,
                },
            )
        }

        fn font_metrics(&self, _font_id: FontId) -> FontMetrics {
            FontMetrics {
                units_per_em: UNITS_PER_EM as u32,
                ascent: 800.,
                descent: -200.,
                line_gap: 0.,
                underline_position: -100.,
                underline_thickness: 50.,
                cap_height: 700.,
                x_height: 500.,
                bounding_box: Bounds {
                    origin: point(0., -200.),
                    size: size(1000., 1000.),
                },
            }
        }

        fn typographic_bounds(
            &self,
            font_id: FontId,
            _glyph_id: GlyphId,
        ) -> anyhow::Result<Bounds<f32>> {
            Ok(Bounds {
                origin: point(0., 0.),
                size: size(Self::advance_units(font_id), 700.),
            })
        }

        fn advance(&self, font_id: FontId, _glyph_id: GlyphId) -> anyhow::Result<Size<f32>> {
            Ok(size(Self::advance_units(font_id), 0.))
        }

        fn glyph_for_char(&self, _font_id: FontId, ch: char) -> Option<GlyphId> {
            Some(GlyphId(ch as u32))
        }

        fn glyph_raster_bounds(
            &self,
            _params: &RenderGlyphParams,
        ) -> anyhow::Result<Bounds<DevicePixels>> {
            Ok(Bounds::default())
        }

        fn rasterize_glyph(
            &self,
            _params: &RenderGlyphParams,
            raster_bounds: Bounds<DevicePixels>,
        ) -> anyhow::Result<(Size<DevicePixels>, Vec<u8>)> {
            Ok((raster_bounds.size, Vec::new()))
        }

        fn layout_line(&self, text: &str, font_size: Pixels, runs: &[FontRun]) -> LineLayout {
            let mut position = px(0.);
            let mut shaped_runs = Vec::new();
            let mut run_start = 0;
            for run in runs {
                let run_text = &text[run_start..run_start + run.len];
                let advance = font_size * (Self::advance_units(run.font_id) / UNITS_PER_EM);
                let mut glyphs = Vec::new();
                for (ix, ch) in run_text.char_indices() {
                    glyphs.push(ShapedGlyph {
                        id: GlyphId(ch as u32),
                        position: point(position, px(0.)),
                        index: run_start + ix,
                        is_emoji: false,
                    });
                    position += advance;
                }
                shaped_runs.push(ShapedRun {
                    font_id: run.font_id,
                    glyphs,
                });
                run_start += run.len;
            }
            let metrics = self.font_metrics(BODY_ID);
            LineLayout {
                font_size,
                width: position,
                ascent: font_size * (metrics.ascent / UNITS_PER_EM),
                // Native backends normalize the signed font metric for shaped lines.
                descent: font_size * (-metrics.descent / UNITS_PER_EM),
                runs: shaped_runs,
                len: text.len(),
            }
        }

        fn recommended_rendering_mode(
            &self,
            _font_id: FontId,
            _font_size: Pixels,
        ) -> TextRenderingMode {
            TextRenderingMode::Grayscale
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InlineHighlight, LinkMark, accessible_runs, combine_highlights, point_in_text_selection,
        text_runs,
    };
    use gpui::{FontWeight, HighlightStyle, SharedString, TextStyle, point, px};

    #[test]
    fn accessible_links_partition_unicode_text_without_duplicating_style_runs() {
        let text = "Before 世界 guide after";
        let link = LinkMark {
            url: "test:guide".into(),
            ..Default::default()
        };
        let links = vec![(14..19, link.clone()), (7..14, link.clone())];
        let runs = accessible_runs(text, &links);
        assert_eq!(
            runs,
            vec![(0..7, None), (7..19, Some(link.clone())), (19..25, None)]
        );
        assert_eq!(
            runs.iter()
                .map(|(range, _)| &text[range.clone()])
                .collect::<String>(),
            text
        );
        // Invalid boundaries cannot panic or hide the remainder of the text.
        assert_eq!(
            accessible_runs(text, &[(8..10, link.clone()), (50..80, link)]),
            vec![(0..text.len(), None)]
        );
    }

    fn mono(style: HighlightStyle) -> InlineHighlight {
        InlineHighlight {
            style,
            font_family: Some(SharedString::from("Mono")),
            font_size_scale: None,
        }
    }

    #[test]
    fn text_runs_shape_a_code_highlight_in_its_font_family() {
        let style = TextStyle {
            font_family: SharedString::from("Body"),
            ..Default::default()
        };
        let highlights = vec![(4..8, mono(HighlightStyle::default()))];

        let runs = text_runs(12, &style, &highlights);

        let families = runs
            .iter()
            .map(|run| (run.len, run.font.family.as_ref()))
            .collect::<Vec<_>>();
        assert_eq!(families, vec![(4, "Body"), (4, "Mono"), (4, "Body")]);
    }

    #[test]
    fn combine_highlights_cuts_a_bold_span_at_the_code_boundary() {
        // `**bold `code`**`: the bold mark spans the code mark, so the
        // combined list carries the weight on both sides and the family on
        // the code side only.
        let bold = InlineHighlight::from(HighlightStyle {
            font_weight: Some(FontWeight::BOLD),
            ..Default::default()
        });
        let combined = combine_highlights(
            vec![(0..10, bold)],
            vec![(6..10, mono(HighlightStyle::default()))],
        );

        assert_eq!(combined.len(), 2);
        assert_eq!(combined[0].0, 0..6);
        assert_eq!(combined[0].1.style.font_weight, Some(FontWeight::BOLD));
        assert_eq!(combined[0].1.font_family, None);
        assert_eq!(combined[1].0, 6..10);
        assert_eq!(combined[1].1.style.font_weight, Some(FontWeight::BOLD));
        assert_eq!(combined[1].1.font_family.as_deref(), Some("Mono"));
    }

    #[test]
    fn test_point_in_text_selection() {
        let line_height = px(20.);
        let char_width = px(10.);
        let start = point(px(50.), px(50.));
        let end = point(px(150.), px(150.));

        // First line but haft line height, true
        // | p --------|
        // | selection |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(50.), px(40.)),
            char_width,
            start,
            end,
            line_height
        ));

        // First line in selection, true
        // | p --------|
        // | selection |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(50.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
        // First line, but left out of selection, false
        // p |-----------|
        //   | selection |
        //   |-----------|
        assert!(!point_in_text_selection(
            point(px(40.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
        // First line but right out of selection, true
        // |-----------| p
        // | selection |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(160.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));

        // Middle line in selection, true
        // |-----------|
        // |     p     |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(100.), px(70.)),
            char_width,
            start,
            end,
            line_height
        ));
        // Middle line, but left out of selection, true
        //   |-----------|
        // p | selection |
        //   |-----------|
        assert!(point_in_text_selection(
            point(px(40.), px(70.)),
            char_width,
            start,
            end,
            line_height
        ));
        // Middle line, but right out of selection, true
        // |-----------|
        // | selection | p
        // |-----------|
        assert!(point_in_text_selection(
            point(px(160.), px(70.)),
            char_width,
            start,
            end,
            line_height
        ));

        // Last line in selection, true
        // |-----------|
        // | selection |
        // |------- p -|
        assert!(point_in_text_selection(
            point(px(100.), px(140.)),
            char_width,
            start,
            end,
            line_height
        ));
        // Last line, but left out of selection, true
        //
        //   |-----------|
        //   | selection |
        // p |-----------|
        assert!(point_in_text_selection(
            point(px(40.), px(140.)),
            char_width,
            start,
            end,
            line_height
        ));
        // Last line, but right out of selection, false
        // |-----------|
        // | selection |
        // |-----------| p
        assert!(!point_in_text_selection(
            point(px(160.), px(140.)),
            char_width,
            start,
            end,
            line_height
        ));

        // Out of vertical bounds (top), false
        //       p
        // |-----------|
        // | selection |
        // |-----------|
        assert!(!point_in_text_selection(
            point(px(100.), px(20.)),
            char_width,
            start,
            end,
            line_height
        ));
        // Out of vertical bounds (bottom), false
        // |-----------|
        // | selection |
        // |-----------|
        //       p
        assert!(!point_in_text_selection(
            point(px(100.), px(160.)),
            char_width,
            start,
            end,
            line_height
        ));
    }

    #[test]
    fn test_point_in_text_selection_reversed_drag_direction() {
        let line_height = px(20.);
        let char_width = px(10.);

        // Mouse down on lower line then drag upward to x=150.
        // Top line should follow current mouse x, bottom line should keep anchor x.
        let start = point(px(80.), px(150.));
        let end = point(px(150.), px(50.));

        // On top line, selection starts from top cursor x (150), so x=140 should be excluded.
        assert!(!point_in_text_selection(
            point(px(140.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
        assert!(point_in_text_selection(
            point(px(150.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));

        // On bottom line, selection ends at anchor x (80), so x=90 should be excluded.
        assert!(point_in_text_selection(
            point(px(75.), px(140.)),
            char_width,
            start,
            end,
            line_height
        ));
        assert!(!point_in_text_selection(
            point(px(80.), px(140.)),
            char_width,
            start,
            end,
            line_height
        ));
    }

    #[test]
    fn test_point_in_text_selection_same_visual_line_with_different_y() {
        let line_height = px(20.);
        let char_width = px(10.);
        let start = point(px(100.), px(55.));
        let end = point(px(60.), px(58.));

        assert!(!point_in_text_selection(
            point(px(40.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
        assert!(point_in_text_selection(
            point(px(70.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
        assert!(!point_in_text_selection(
            point(px(110.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
    }

    #[test]
    fn test_point_in_text_selection_same_visual_line_with_reversed_y() {
        let line_height = px(20.);
        let char_width = px(10.);
        let start = point(px(60.), px(58.));
        let end = point(px(100.), px(55.));

        assert!(!point_in_text_selection(
            point(px(40.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
        assert!(point_in_text_selection(
            point(px(70.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
        assert!(!point_in_text_selection(
            point(px(110.), px(50.)),
            char_width,
            start,
            end,
            line_height
        ));
    }
}
