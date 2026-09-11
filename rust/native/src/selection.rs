//! Read-only text selection. Offsets are UTF-8 byte boundaries; arrow movement
//! uses extended graphemes. This owns no OCaml callbacks or mutable documents.
use gpui::{
    App, ClipboardItem, EntityId, FocusHandle, HighlightStyle, MouseButton, SharedString,
    StyledText, div, prelude::*,
};
use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default, Debug, PartialEq, Eq)]
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
    pub focus: FocusHandle,
}
impl State {
    pub fn new(text: Arc<str>, cx: &App) -> Self {
        Self {
            text,
            selection: Selection::default(),
            dragging: false,
            focus: cx.focus_handle().tab_stop(true),
        }
    }
    pub fn update(&mut self, text: Arc<str>) {
        self.selection.clamp(&text);
        self.text = text;
    }
}

pub fn element(
    state: Rc<RefCell<State>>,
    color: gpui::Hsla,
    pointer: bool,
    owner: EntityId,
) -> gpui::AnyElement {
    let (text, range, focus) = {
        let state = state.borrow();
        (
            state.text.clone(),
            state.selection.range(),
            state.focus.clone(),
        )
    };
    let highlights = if range.is_empty() {
        vec![]
    } else {
        vec![(
            range,
            HighlightStyle {
                background_color: Some(color),
                ..Default::default()
            },
        )]
    };
    let text = StyledText::new(SharedString::from(text)).with_highlights(highlights);
    let layout = text.layout().clone();
    let mut element = div()
        .id("selectable-text")
        .track_focus(&focus)
        .tab_index(0)
        .role(gpui::Role::Label)
        .child(text);
    if pointer {
        element = element.cursor_text();
        let down_state = state.clone();
        let down_layout = layout.clone();
        element = element.on_mouse_down(MouseButton::Left, move |event, window, cx| {
            let index = down_layout
                .index_for_position(event.position)
                .unwrap_or_else(|index| index);
            let mut state = down_state.borrow_mut();
            let text = state.text.clone();
            if event.click_count >= 3 {
                state.selection.anchor = 0;
                state.selection.head = text.len();
            } else if event.click_count == 2 {
                state.selection.word(&text, index);
            } else {
                state.selection.move_to(index, event.modifiers.shift);
            }
            state.dragging = true;
            window.focus(&state.focus, cx);
            cx.notify(owner);
            cx.stop_propagation();
        });
        let move_state = state.clone();
        element = element.on_mouse_move(move |event, _, cx| {
            let mut state = move_state.borrow_mut();
            if state.dragging {
                state.selection.head = layout
                    .index_for_position(event.position)
                    .unwrap_or_else(|index| index);
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
        .on_key_down(move |event, _, cx| {
            let mut state = state.borrow_mut();
            let text = state.text.clone();
            let key = event.keystroke.key.as_str();
            let modifiers = event.keystroke.modifiers;
            if modifiers.secondary() && key == "c" {
                let range = state.selection.range();
                if !range.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text[range].to_owned()));
                }
            } else if modifiers.secondary() && key == "a" {
                state.selection.anchor = 0;
                state.selection.head = text.len();
            } else {
                match key {
                    "left" => state.selection.horizontal(&text, false, modifiers.shift),
                    "right" => state.selection.horizontal(&text, true, modifiers.shift),
                    "home" => state.selection.move_to(0, modifiers.shift),
                    "end" => state.selection.move_to(text.len(), modifiers.shift),
                    _ => return,
                }
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
