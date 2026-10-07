//! Directed partial/intermediate ranges on TestPlatform, not OS AX acceptance.
use super::*;
use gpui_base::{TextView, TextViewState, text::PreparedText};

const LABEL: &str = "Repeated 世界 content";

struct Documents {
    documents: Vec<Entity<TextViewState>>,
}

impl Render for Documents {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(450.))
            .child(gpui_base::TextSelectionLayer)
            .children(
                self.documents
                    .iter()
                    .map(|text| TextView::new(text).scrollable(false)),
            )
    }
}

fn points(cx: &mut VisualTestContext) -> (gpui::Point<gpui::Pixels>, gpui::Point<gpui::Pixels>) {
    let tree = cx.a11y_tree().unwrap();
    let mut bounds: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.value() == Some(LABEL))
        .map(|(_, node)| node.bounds().unwrap())
        .collect();
    assert_eq!(bounds.len(), 3);
    bounds.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    assert!(
        bounds[0].y1 <= bounds[1].y0 && bounds[1].y1 <= bounds[2].y0,
        "ordered rows: {bounds:?}"
    );
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    // Paragraph semantics include the allocated line width. Target the actual
    // glyphs near the start, not the center of that wider layout rectangle.
    let inside_text = |bounds: gpui::accesskit::Rect| {
        gpui::point(
            px((bounds.x0 / scale) as f32) + px(45.),
            px(((bounds.y0 + bounds.y1) / (2. * scale)) as f32),
        )
    };
    (inside_text(bounds[0]), inside_text(bounds[2]))
}

fn drag(
    cx: &mut VisualTestContext,
    start: gpui::Point<gpui::Pixels>,
    end: gpui::Point<gpui::Pixels>,
) {
    cx.simulate_mouse_move(start, None, Default::default());
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
    cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
    draw(cx);
}

fn ranges(
    documents: &[Entity<TextViewState>],
    cx: &VisualTestContext,
    backward: bool,
) -> Vec<std::ops::Range<usize>> {
    documents
        .iter()
        .enumerate()
        .map(|(index, text)| {
            text.read_with(cx, |state, cx| {
                let selection = state.rendered_selection().unwrap_or_else(|| panic!("directed native range for row {index}; text={:?}, copy={:?}, captured={:?}", state.rendered_text().map(|text|text.text().to_owned()), state.selected_text(), state.captured_rendered_pointer_selection(cx)));
                assert_eq!(selection.is_backward(), backward);
                assert!(state.requested_rendered_selection().is_none());
                assert_eq!(
                    state.selected_text(),
                    state
                        .rendered_text()
                        .unwrap()
                        .selected_text(&selection)
                        .unwrap()
                );
                selection.bytes()
            })
        })
        .collect()
}

#[test]
fn repeated_documents_keep_partial_endpoints_full_middle_direction_and_retirement() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        documents: (0..3)
            .map(|_| {
                cx.new(|cx| {
                    let mut state = TextViewState::externally_prepared(cx);
                    state.set_prepared(
                        PreparedText::parse(LABEL, Default::default()).unwrap(),
                        None,
                        cx,
                    );
                    state
                })
            })
            .collect(),
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let documents = view.read_with(cx, |view, _| view.documents.clone());
    let (start, end) = points(cx);
    drag(cx, start, end);
    let forward = ranges(&documents, cx, false);
    let len = LABEL.len() + 1; // Native paragraph separator.
    assert!(forward[0].start > 0);
    assert_eq!(forward[0].end, len);
    assert_eq!(forward[1], 0..len);
    assert_eq!(forward[2].start, 0);
    assert!(forward[2].end > 0 && forward[2].end < LABEL.len());
    let copy = cx.update(gpui_base::TextSelection::selected_text);
    drag(cx, end, start);
    assert_eq!(ranges(&documents, cx, true), forward);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), copy);

    // Equal labels must remain bound to their actual owner, including after reorder.
    view.update(cx, |view, cx| {
        view.documents.reverse();
        cx.notify();
    });
    draw(cx);
    let (start, end) = points(cx);
    drag(cx, start, end);
    let reordered = view.read_with(cx, |view, _| view.documents.clone());
    assert_eq!(ranges(&reordered, cx, false), forward);

    // Replacing the intermediate participant also invalidates the whole window range.
    documents[1].update(cx, |state, cx| {
        state.set_prepared(
            PreparedText::parse("replacement", Default::default()).unwrap(),
            None,
            cx,
        );
    });
    draw(cx);
    assert!(
        cx.update(gpui_base::TextSelection::selected_text)
            .is_empty()
    );
    for text in &documents {
        assert!(text.read_with(cx, |state, _| state.rendered_selection().is_none()));
    }
}
