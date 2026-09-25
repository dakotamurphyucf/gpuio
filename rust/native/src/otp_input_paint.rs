//! Shared painted text geometry for selection, pointer hits and platform IME.
//! Accepted ASCII is segmented. During composition the bounded Unicode draft is
//! shaped continuously, so combining characters are not split between cells.
use super::*;

struct Line {
    shaped: ShapedLine,
    origin: Point<Pixels>,
}
pub(super) struct Layout {
    pub(super) bounds: Bounds<Pixels>,
    text: String,
    masked: bool,
    composing: bool,
    positions: Vec<(usize, Pixels)>,
    lines: Vec<Line>,
    cells: Vec<Bounds<Pixels>>,
    selection: Option<Bounds<Pixels>>,
    caret: Option<Bounds<Pixels>>,
    scroll: Pixels,
    color: Hsla,
    focused: bool,
}
impl Layout {
    pub(super) fn matches(&self, model: &State) -> bool {
        self.text == model.editor().draft()
            && self.masked == model.config().masked
            && self.composing == model.editor().is_composing()
    }
    fn x(&self, offset: usize) -> Pixels {
        self.positions
            .iter()
            .find(|(byte, _)| *byte == offset)
            .map_or(self.bounds.left(), |(_, x)| *x)
    }
    pub(super) fn range_bounds(&self, range: Range<usize>) -> Bounds<Pixels> {
        let start = self
            .x(range.start)
            .clamp(self.bounds.left(), self.bounds.right());
        let end = self
            .x(range.end)
            .clamp(self.bounds.left(), self.bounds.right());
        Bounds::new(
            point(start.min(end), self.bounds.top()),
            size((end - start).abs().max(px(1.)), self.bounds.size.height),
        )
    }
    pub(super) fn byte_for_point(&self, point: Point<Pixels>) -> usize {
        self.positions
            .iter()
            .min_by(|(_, a), (_, b)| {
                ((*a - point.x).abs())
                    .partial_cmp(&((*b - point.x).abs()))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map_or(0, |(byte, _)| *byte)
    }
}
fn shape(text: SharedString, mark: Option<Range<usize>>, window: &Window) -> ShapedLine {
    let style = window.text_style();
    let run = TextRun {
        len: text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let runs = if let Some(mark) = mark {
        vec![
            TextRun {
                len: mark.start,
                ..run.clone()
            },
            TextRun {
                len: mark.end - mark.start,
                underline: Some(UnderlineStyle {
                    color: Some(style.color),
                    thickness: px(1.),
                    wavy: false,
                }),
                ..run.clone()
            },
            TextRun {
                len: text.len() - mark.end,
                ..run
            },
        ]
        .into_iter()
        .filter(|run| run.len > 0)
        .collect()
    } else {
        vec![run]
    };
    window.text_system().shape_line(
        text,
        style.font_size.to_pixels(window.rem_size()),
        &runs,
        None,
    )
}
fn build(input: &Input, bounds: Bounds<Pixels>, window: &Window) -> Layout {
    let model = &input.model;
    let editor = model.editor();
    let text = editor.draft();
    let height = bounds.size.height;
    let font = window.text_style().font_size.to_pixels(window.rem_size());
    let cell = (font * 2.).max(px(28.));
    let gap = px(5.);
    let inset = px(5.);
    let y = bounds.top() + (height - window.line_height()).max(px(0.)) / 2.;
    let mut positions = Vec::new();
    let mut lines = Vec::new();
    let composing = editor.is_composing();
    let masked = model.config().masked;
    if composing {
        let display: SharedString = if masked {
            "•".repeat(text.chars().count()).into()
        } else {
            text.to_owned().into()
        };
        let display_byte = |offset: usize| {
            if masked {
                text[..offset].chars().count() * "•".len()
            } else {
                offset
            }
        };
        let marked = editor
            .marked()
            .map(|range| display_byte(range.start)..display_byte(range.end));
        let shaped = shape(display, marked, window);
        for byte in text
            .char_indices()
            .map(|(byte, _)| byte)
            .chain(std::iter::once(text.len()))
        {
            positions.push((
                byte,
                bounds.left() + inset + shaped.x_for_index(display_byte(byte)),
            ));
        }
        lines.push(Line {
            shaped,
            origin: point(bounds.left() + inset, y),
        });
    } else {
        for (index, ch) in text.chars().enumerate() {
            let shaped = shape(
                if masked {
                    "•".into()
                } else {
                    ch.to_string().into()
                },
                None,
                window,
            );
            let left = bounds.left() + (cell + gap) * index;
            lines.push(Line {
                origin: point(left + (cell - shaped.width) / 2., y),
                shaped,
            });
            positions.push((index, left + inset));
        }
        let end = if text.len() == model.config().policy.length() {
            bounds.left() + (cell + gap) * text.len() - gap - inset
        } else {
            bounds.left() + (cell + gap) * text.len() + inset
        };
        positions.push((text.len(), end));
    }
    let caret_x = positions
        .iter()
        .find(|(byte, _)| *byte == editor.selection().head)
        .map_or(bounds.left(), |(_, x)| *x);
    let mut scroll = input.layout.as_ref().map_or(px(0.), |layout| layout.scroll);
    if caret_x - scroll > bounds.right() - inset {
        scroll = (caret_x - bounds.right() + inset).max(px(0.));
    }
    if caret_x - scroll < bounds.left() + inset {
        scroll = (caret_x - bounds.left() - inset).max(px(0.));
    }
    for (_, x) in &mut positions {
        *x -= scroll;
    }
    for line in &mut lines {
        line.origin.x -= scroll;
    }
    let cells = if composing {
        vec![bounds]
    } else {
        (0..model.config().policy.length())
            .map(|index| {
                Bounds::new(
                    point(
                        bounds.left() + (cell + gap) * index - scroll,
                        bounds.top() + px(1.),
                    ),
                    size(cell, (height - px(2.)).max(px(0.))),
                )
            })
            .collect()
    };
    let focused = input.focus.is_focused(window);
    let mut layout = Layout {
        bounds,
        text: text.into(),
        masked,
        composing,
        positions,
        lines,
        cells,
        selection: None,
        caret: None,
        scroll,
        color: window.text_style().color,
        focused,
    };
    let range = editor.selection().range();
    if range.is_empty() {
        if focused && !model.config().disabled {
            layout.caret = Some(Bounds::new(
                point(layout.x(range.start), bounds.top() + px(8.)),
                size(px(1.5), (height - px(16.)).max(px(1.))),
            ));
        }
    } else {
        layout.selection = Some(layout.range_bounds(range));
    }
    layout
}

pub(super) struct Field {
    pub(super) input: Entity<Input>,
}
impl IntoElement for Field {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Field {
    type RequestLayoutState = ();
    type PrepaintState = Option<Layout>;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = gpui::Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Layout> {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.input.update(cx, |state, _| {
            if let Some(capture) = state.capture {
                if window.captured_hitbox() == Some(capture)
                    && state.pointer
                    && state.access() == Access::Allowed
                    && !state.model.config().disabled
                {
                    window.capture_pointer(hitbox.id);
                    state.capture = Some(hitbox.id);
                } else {
                    state.stop_drag(window);
                }
            }
            state.hitbox = Some(hitbox.id);
        });
        Some(build(self.input.read(cx), bounds, window))
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        layout: &mut Option<Layout>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(layout) = layout.take() else {
            return;
        };
        let moved = self.input.downgrade();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if !phase.capture() {
                return;
            }
            let _ = moved.update(cx, |state, cx| {
                if state.dragging {
                    state.mouse_move(event, window, cx);
                    cx.stop_propagation();
                }
            });
        });
        let released = self.input.downgrade();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if !phase.capture() || event.button != MouseButton::Left {
                return;
            }
            let _ = released.update(cx, |state, cx| {
                if state.dragging {
                    state.mouse_up(event, window, cx);
                    cx.stop_propagation();
                }
            });
        });
        let input = self.input.read(cx);
        if input.access() == Access::Allowed && !input.model.config().disabled {
            window.handle_input(
                &input.focus,
                ElementInputHandler::new(bounds, self.input.clone()),
                cx,
            );
        }
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for cell in &layout.cells {
                let mut quad = fill(*cell, layout.color.opacity(0.04));
                quad.corner_radii = px(6.).into();
                quad.border_widths = px(1.).into();
                quad.border_color = if layout.focused {
                    rgba(0x6688ffff).into()
                } else {
                    layout.color.opacity(0.25)
                };
                window.paint_quad(quad);
            }
            if let Some(selection) = layout.selection {
                window.paint_quad(fill(selection, rgba(0x6688ff40)));
            }
            for line in &layout.lines {
                let _ = line.shaped.paint(
                    line.origin,
                    window.line_height(),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
            if let Some(caret) = layout.caret {
                window.paint_quad(fill(caret, layout.color));
            }
        });
        self.input.update(cx, |input, _| {
            input.layout = Some(layout);
        });
        let input = self.input.read(cx);
        if input.autofocus && input.access() == Access::Allowed && !input.model.config().disabled {
            self.input.update(cx, |s, _| s.autofocus = false);
            let weak = self.input.downgrade();
            window.defer(cx, move |window, cx| {
                let _ = weak.update(cx, |s, cx| {
                    if s.access() == Access::Allowed && !s.model.config().disabled {
                        window.focus(&s.focus, cx);
                    }
                });
            });
        }
    }
}
