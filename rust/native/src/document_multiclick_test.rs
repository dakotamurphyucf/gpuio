//! Real native selection dispatch on TestPlatform; not macOS AX acceptance.
use super::*;
use gpui_base::{
    TextView, TextViewState,
    text::{MarkdownExtensions, MarkdownNode, MarkdownPlugin, PreparedText},
};

struct Atom;
impl MarkdownPlugin for Atom {
    fn name(&self) -> &str {
        "atom"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::InlineCode(_))
            .then(|| MarkdownNode::new("atom", ()).text("OBJECT"))
    }
}
struct Scene {
    text: Entity<TextViewState>,
    extensions: MarkdownExtensions,
    width: f32,
}
impl Render for Scene {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(self.width))
            .child(gpui_base::TextSelectionLayer)
            .child(TextView::new(&self.text).markdown_extensions(self.extensions.clone()))
    }
}

fn click(cx: &mut VisualTestContext, value: &str, occurrence: usize, count: usize) {
    let tree = cx.a11y_tree().unwrap();
    let mut candidates: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.value() == Some(value) || node.label() == Some(value))
        .filter_map(|(_, node)| node.bounds())
        .collect();
    candidates.sort_by(|a, b| a.y0.total_cmp(&b.y0).then(a.x0.total_cmp(&b.x0)));
    let bounds = candidates.get(occurrence).unwrap_or_else(|| {
        panic!(
            "missing {value:?}: {:?}",
            tree.nodes
                .iter()
                .map(|(_, node)| (node.role(), node.value(), node.label()))
                .collect::<Vec<_>>()
        )
    });
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    let position = gpui::point(
        px((bounds.x0 / scale) as f32) + px(2.),
        px(((bounds.y0 + bounds.y1) / (2. * scale)) as f32),
    );
    cx.simulate_mouse_move(position, None, Default::default());
    cx.simulate_event(gpui::MouseDownEvent {
        button: gpui::MouseButton::Left,
        position,
        click_count: count,
        modifiers: Default::default(),
        first_mouse: false,
    });
    cx.simulate_mouse_up(position, gpui::MouseButton::Left, Default::default());
    draw(cx);
}

fn assert_selection(
    text: &Entity<TextViewState>,
    cx: &mut VisualTestContext,
    range: std::ops::Range<usize>,
    expected: &str,
) {
    text.read_with(cx, |state, _| {
        let selected = state
            .rendered_selection()
            .expect("multi-click logical range");
        assert_eq!(selected.bytes(), range);
        assert!(!selected.is_backward());
        assert_eq!(state.selected_text(), expected);
        assert!(state.requested_rendered_selection().is_none());
    });
    assert_eq!(
        cx.update(gpui_base::TextSelection::selected_text),
        expected.trim()
    );
}

fn visual_line_text(cx: &mut VisualTestContext, hit: &str) -> String {
    let tree = cx.a11y_tree().unwrap();
    let hit = tree
        .nodes
        .iter()
        .find(|(_, node)| node.value() == Some(hit))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    let y = (hit.y0 + hit.y1) / 2.;
    let mut fragments: Vec<_> = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let value = node.value()?;
            let bounds = node.bounds()?;
            (bounds.y0 <= y && y < bounds.y1).then_some((bounds.x0, value))
        })
        .collect();
    fragments.sort_by(|a, b| a.0.total_cmp(&b.0));
    fragments.into_iter().map(|(_, text)| text).collect()
}

#[test]
fn multiclick_combines_styled_fragments_and_atomic_line_ends() {
    for (source, extensions, hit, word, word_range, line) in [
        (
            "before ex`am`ple after",
            MarkdownExtensions::default(),
            "am",
            "example",
            7..14,
            "before example after",
        ),
        (
            "`head` ex**am**ple `tail`",
            MarkdownExtensions::default().plugin(Atom),
            " example ",
            " ",
            6..7,
            "OBJECT example OBJECT",
        ),
    ] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (view, cx) = app.add_window_view(|_, cx| Scene {
            text: cx.new(|cx| {
                let mut state = TextViewState::externally_prepared(cx);
                state.set_prepared(
                    PreparedText::parse(source, extensions.clone()).unwrap(),
                    None,
                    cx,
                );
                state
            }),
            extensions: extensions.clone(),
            width: 450.,
        });
        cx.simulate_a11y_active(true);
        draw(cx);
        let text = view.read_with(cx, |view, _| view.text.clone());
        click(cx, hit, 0, 2);
        assert_selection(&text, cx, word_range, word);
        click(cx, hit, 0, 3);
        assert_selection(&text, cx, 0..line.len(), line);
        if source.starts_with('`') {
            // The same alternative appears twice: actual owner identity selects the tail.
            click(cx, "OBJECT", 1, 2);
            assert_selection(&text, cx, line.len() - 6..line.len(), "OBJECT");
        }
        view.update(cx, |view, cx| {
            view.width = 110.;
            cx.notify();
        });
        draw(cx);
        assert!(text.read_with(cx, |state, _| state.rendered_selection().is_some()));
        if source.starts_with("before") {
            view.update(cx, |view, cx| {
                view.width = 70.;
                cx.notify();
            });
            draw(cx);
            // This width can wrap within a styled word. The independent visual
            // labels define the actual row, not an assumed word boundary.
            let visible = visual_line_text(cx, "am");
            assert!(!visible.trim().is_empty() && visible.trim() != line);
            click(cx, "am", 0, 3);
            text.read_with(cx, |state, _| {
                let range = state
                    .rendered_selection()
                    .expect("wrapped visual-line range");
                assert!(range.bytes().start > 0 && range.bytes().end < line.len());
                assert_eq!(state.selected_text(), visible);
            });
            assert_eq!(
                cx.update(gpui_base::TextSelection::selected_text),
                visible.trim()
            );
        }
        text.update(cx, |state, cx| {
            state.set_prepared(
                PreparedText::parse(source, extensions.clone()).unwrap(),
                None,
                cx,
            );
        });
        assert!(text.read_with(cx, |state, _| state.rendered_selection().is_none()));
    }
}

#[test]
fn multiclick_keeps_combining_and_joined_graphemes_whole() {
    for cluster in ["e\u{301}", "👨‍👩‍👧‍👦"] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let source = format!("{cluster}\n\n{cluster}");
        let (view, cx) = app.add_window_view(|_, cx| Scene {
            text: cx.new(|cx| {
                let mut state = TextViewState::externally_prepared(cx);
                state.set_prepared(
                    PreparedText::parse(&source, Default::default()).unwrap(),
                    None,
                    cx,
                );
                state
            }),
            extensions: Default::default(),
            width: 450.,
        });
        cx.simulate_a11y_active(true);
        draw(cx);
        let text = view.read_with(cx, |view, _| view.text.clone());
        click(cx, cluster, 1, 2);
        assert_selection(&text, cx, cluster.len() + 1..2 * cluster.len() + 1, cluster);
    }
}
