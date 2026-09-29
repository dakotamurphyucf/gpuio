//! Read-only text selection. Offsets are UTF-8 byte boundaries; arrow movement
//! uses extended graphemes. This owns no OCaml callbacks or mutable documents.
use crate::text_projection::{self, Mapping};
use gpui::{
    App, ClipboardItem, EntityId, FocusHandle, MouseButton, SharedString, StyledText, div,
    prelude::*,
};
use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}
impl Selection {
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
    pub fn clamp(&mut self, text: &str) {
        fn boundary(text: &str, offset: usize) -> usize {
            let mut offset = offset.min(text.len());
            while !text.is_char_boundary(offset) {
                offset -= 1;
            }
            offset
        }
        self.anchor = boundary(text, self.anchor);
        self.head = boundary(text, self.head);
    }
    pub fn move_to(&mut self, index: usize, extend: bool) {
        self.head = index;
        if !extend {
            self.anchor = index;
        }
    }
    pub fn horizontal(&mut self, text: &str, right: bool, extend: bool) {
        let range = self.range();
        let index = if !extend && !range.is_empty() {
            if right { range.end } else { range.start }
        } else if right {
            text.grapheme_indices(true)
                .map(|(i, _)| i)
                .find(|i| *i > self.head)
                .unwrap_or(text.len())
        } else {
            text.grapheme_indices(true)
                .map(|(i, _)| i)
                .take_while(|i| *i < self.head)
                .last()
                .unwrap_or(0)
        };
        self.move_to(index, extend);
    }
    pub fn word(&mut self, text: &str, index: usize) {
        if let Some((start, word)) = text
            .split_word_bound_indices()
            .find(|(start, word)| *start <= index && index < *start + word.len())
        {
            self.anchor = start;
            self.head = start + word.len();
        } else {
            self.move_to(text.len(), false);
        }
    }
}
pub struct State {
    pub text: Arc<str>,
    pub selection: Selection,
    dragging: bool,
    local_gesture: bool,
    geometric: bool,
    pub focus: FocusHandle,
    participant: gpui_base::TextSelectionHandle,
    run: Option<gpui_base::TextSelectionRun>,
    mapping: Vec<Mapping>,
    paint: crate::highlight_paint::SharedCache,
}
impl State {
    pub(crate) fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn new(text: Arc<str>, owner: EntityId, cx: &mut App) -> Rc<RefCell<Self>> {
        let participant = gpui_base::TextSelectionHandle::new("", cx);
        let state = Rc::new(RefCell::new(Self {
            text,
            selection: Selection::default(),
            dragging: false,
            local_gesture: false,
            geometric: false,
            focus: cx.focus_handle().tab_stop(true),
            participant: participant.clone(),
            run: None,
            mapping: Vec::new(),
            paint: Default::default(),
        }));
        let weak = Rc::downgrade(&state);
        participant.clear_with(
            move |cx| {
                if let Some(state) = weak.upgrade() {
                    let mut state = state.borrow_mut();
                    state.selection = Selection::default();
                    state.dragging = false;
                    state.local_gesture = false;
                    state.geometric = false;
                    cx.notify(owner);
                }
            },
            cx,
        );
        let weak = Rc::downgrade(&state);
        participant.copy_with(
            move |cx| {
                let Some(state) = weak.upgrade() else {
                    return String::new();
                };
                let mut state = state.borrow_mut();
                state.project(cx);
                state.text[state.selection.range()].to_owned()
            },
            cx,
        );
        let weak = Rc::downgrade(&state);
        participant.focus_with(
            move |window, cx| {
                if let Some(state) = weak.upgrade() {
                    window.focus(&state.borrow().focus, cx);
                }
            },
            cx,
        );
        let weak = Rc::downgrade(&state);
        participant
            .subscribe(
                move |_, cx| {
                    if weak.upgrade().is_some() {
                        cx.notify(owner);
                    }
                },
                cx,
            )
            .detach();
        state
    }

    pub fn update(&mut self, text: Arc<str>) {
        self.selection.clamp(&text);
        self.text = text;
    }

    pub(crate) fn has_geometry(&self, cx: &App) -> bool {
        self.participant.snapshot(cx).is_some()
    }

    #[cfg(feature = "native-tests")]
    pub(crate) fn point(&self, byte: usize) -> gpui::Point<gpui::Pixels> {
        let run = self.run.as_ref().expect("painted text");
        let mut point = run
            .layout()
            .position_for_index(
                text_projection::displayed_offset(&self.mapping, byte).expect("mapped byte"),
            )
            .expect("visible byte");
        point.y += run.layout().line_height() / 2.;
        point
    }

    #[cfg(feature = "native-tests")]
    pub(crate) fn layout(&self) -> gpui::TextLayout {
        self.run.as_ref().expect("painted text").layout().clone()
    }

    #[cfg(feature = "native-tests")]
    pub(crate) fn displayed_text(&self) -> &str {
        self.run.as_ref().expect("painted text").text()
    }

    fn local_anchor(&self, cx: &mut App) {
        let point = self.run.as_ref().and_then(|run| {
            let mut point = run.position_for_index(text_projection::displayed_offset(
                &self.mapping,
                self.selection.anchor,
            )?)?;
            point.y += run.layout().line_height() / 2.;
            Some(point)
        });
        self.participant.set_local_anchor(point, cx);
    }

    fn project(&mut self, cx: &mut App) {
        if let Some(snapshot) = self.participant.snapshot(cx) {
            let Some(run) = &self.run else { return };
            let projected = self.participant.update_runs(std::slice::from_ref(run), cx);
            let range = projected.ranges()[0]
                .clone()
                .and_then(|range| text_projection::source_range(&self.mapping, range))
                .unwrap_or(0..0);
            let reversed = snapshot.window_points().is_some_and(|points| {
                let (a, b) = (points.anchor(), points.cursor());
                b.y < a.y || (b.y == a.y && b.x < a.x)
            });
            self.selection = if reversed {
                Selection {
                    anchor: range.end,
                    head: range.start,
                }
            } else {
                Selection {
                    anchor: range.start,
                    head: range.end,
                }
            };
            self.geometric = true;
        } else if self.geometric {
            self.selection.move_to(self.selection.head, false);
            self.geometric = false;
        }
    }
}

pub fn element(
    state: Rc<RefCell<State>>,
    color: gpui::Hsla,
    pointer: bool,
    owner: EntityId,
    scope: gpui_base::TextSelectionScopeId,
    highlight: Option<(
        crate::highlight_paint::Paint,
        crate::highlight_paint::SharedCache,
    )>,
) -> gpui::AnyElement {
    let (source, focus) = {
        let s = state.borrow();
        (s.text.clone(), s.focus.clone())
    };
    let label = SharedString::from(source.clone());
    let text = StyledText::new(label.clone());
    let layout = text.layout().clone();
    let mut element = div()
        .id("selectable-text")
        .relative()
        .track_focus(&focus)
        .tab_index(0)
        .role(gpui::Role::Label)
        .aria_label(label);
    if let Some((paint, cache)) = highlight {
        element = element.child(crate::highlight_paint::underlay(
            source.clone(),
            layout.clone(),
            paint,
            cache,
        ));
    }
    let paint_state = state.clone();
    let paint_layout = layout.clone();
    element = element
        .child(
            gpui::canvas(
                |bounds, window, _| window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal),
                move |_, hitbox, window, cx| {
                    let bounds = paint_layout.bounds();
                    let participant = paint_state.borrow().participant.clone();
                    participant.register_in_paint_order(
                        gpui_base::TextSelectionRegistration::new(hitbox.clone(), bounds)
                            .with_scope(scope)
                            .with_text_bounds(if pointer { vec![bounds] } else { vec![] }),
                        window,
                        cx,
                    );
                    let mut state = paint_state.borrow_mut();
                    let displayed = paint_layout.text();
                    state.mapping = text_projection::mapping(
                        &state.text,
                        &displayed,
                        window.text_style().text_overflow.as_ref(),
                    );
                    let mut run = state.run.take().unwrap_or_else(|| {
                        gpui_base::TextSelectionRun::new("", paint_layout.clone(), bounds)
                    });
                    run.update(displayed, paint_layout.clone(), bounds);
                    state.run = Some(run.with_text_align(window.text_style().text_align));
                    state.project(cx);
                    // Cache runs for native multi-click/endpoint projection even at rest.
                    if !state.geometric {
                        participant
                            .update_runs(std::slice::from_ref(state.run.as_ref().unwrap()), cx);
                        if state.local_gesture {
                            // Local caret anchors also track scrolling when the
                            // keyboard range is empty.
                            state.local_anchor(cx);
                        }
                    }
                    crate::highlight_paint::selection(
                        &state.text,
                        &paint_layout,
                        state.selection.range(),
                        color,
                        &state.paint,
                        window,
                    );
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .child(text);
    if pointer {
        element = element.cursor_text();
        let down_state = state.clone();
        element = element.on_mouse_down(MouseButton::Left, move |event, window, cx| {
            let mut state = down_state.borrow_mut();
            let Some(run) = &state.run else { return };
            let index = if event.click_count >= 2 {
                run.index_for_position(event.position)
            } else {
                run.caret_for_position(event.position)
            };
            let Some(index) = index else { return };
            let Some(index) = text_projection::source_offset(&state.mapping, index) else {
                return;
            };
            let text = state.text.clone();
            let local = event.click_count >= 2;
            if event.click_count >= 3 {
                state.selection = Selection {
                    anchor: 0,
                    head: text.len(),
                };
            } else if event.click_count == 2 {
                state.selection.word(&text, index);
            } else {
                state.selection.move_to(index, event.modifiers.shift);
            }
            state.dragging = true;
            state.local_gesture = local;
            if local {
                gpui_base::GlobalState::suppress_text_selection(cx);
                state.participant.set_local_selection(true, cx);
                state.local_anchor(cx);
                cx.stop_propagation();
            }
            window.focus(&state.focus, cx);
            cx.notify(owner);
        });
        let move_state = state.clone();
        element = element.on_mouse_move(move |event, _, cx| {
            let mut state = move_state.borrow_mut();
            if state.dragging && state.local_gesture {
                let index = state
                    .run
                    .as_ref()
                    .and_then(|run| run.caret_for_position(event.position));
                if let Some(index) =
                    index.and_then(|index| text_projection::source_offset(&state.mapping, index))
                {
                    state.selection.head = index;
                }
                cx.notify(owner);
                cx.stop_propagation();
            }
        });
        let up_state = state.clone();
        element = element.on_mouse_up(MouseButton::Left, move |_, _, _| {
            up_state.borrow_mut().dragging = false
        });
        let out_state = state.clone();
        element = element.on_mouse_up_out(MouseButton::Left, move |_, _, _| {
            out_state.borrow_mut().dragging = false
        });
    }
    element
        .on_key_down(move |event, window, cx| {
            let key = event.keystroke.key.as_str();
            let modifiers = event.keystroke.modifiers;
            if modifiers.secondary() && key == "c" {
                let text = gpui_base::TextSelection::selected_text(window, cx);
                if !text.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            } else {
                let mut next = {
                    let mut state = state.borrow_mut();
                    state.project(cx);
                    state.selection.clone()
                };
                let text = state.borrow().text.clone();
                if modifiers.secondary() && key == "a" {
                    next = Selection {
                        anchor: 0,
                        head: text.len(),
                    };
                } else {
                    match key {
                        "left" => next.horizontal(&text, false, modifiers.shift),
                        "right" => next.horizontal(&text, true, modifiers.shift),
                        "home" => next.move_to(0, modifiers.shift),
                        "end" => next.move_to(text.len(), modifiers.shift),
                        _ => return,
                    }
                }
                gpui_base::TextSelection::clear(window, cx);
                let mut state = state.borrow_mut();
                state.selection = next;
                state.local_gesture = true;
                state
                    .participant
                    .set_local_selection(!state.selection.range().is_empty(), cx);
                state.local_anchor(cx);
            }
            cx.notify(owner);
            cx.stop_propagation();
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grapheme_selection_clamps_edits_and_preserves_direction() {
        let text = "a👨‍👩‍👧‍👦e\u{301} z";
        let mut selection = Selection::default();
        selection.horizontal(text, true, false);
        assert_eq!(selection.head, 1);
        selection.horizontal(text, true, true);
        assert_eq!(&text[selection.range()], "👨‍👩‍👧‍👦");
        selection.horizontal(text, false, false);
        assert_eq!(selection.head, 1);
        assert!(selection.range().is_empty());
        selection.anchor = text.len();
        selection.head = 2;
        selection.clamp("é");
        assert_eq!(selection, Selection { anchor: 2, head: 2 });
        selection.word("hello world", 7);
        assert_eq!(selection.range(), 6..11);
    }
}
