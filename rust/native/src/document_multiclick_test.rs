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
    selection_format: gpui_base::text::SelectionFormat,
}
impl Render for Scene {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(self.width))
            .child(gpui_base::TextSelectionLayer)
            .child(
                TextView::new(&self.text)
                    .markdown_extensions(self.extensions.clone())
                    .selection_format(self.selection_format),
            )
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
            selection_format: gpui_base::text::SelectionFormat::Plain,
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
            selection_format: gpui_base::text::SelectionFormat::Plain,
        });
        cx.simulate_a11y_active(true);
        draw(cx);
        let text = view.read_with(cx, |view, _| view.text.clone());
        click(cx, cluster, 1, 2);
        assert_selection(&text, cx, cluster.len() + 1..2 * cluster.len() + 1, cluster);
    }
}

#[test]
fn obsolete_frame_multiclick_cannot_restore_replaced_text() {
    for (atomic, click_count) in [false, true]
        .into_iter()
        .flat_map(|atomic| (1..=3).map(move |count| (atomic, count)))
    {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let extensions = if atomic {
            MarkdownExtensions::default().plugin(Atom)
        } else {
            MarkdownExtensions::default()
        };
        let source = if atomic { "`old`" } else { "Obsolete text" };
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
            selection_format: gpui_base::text::SelectionFormat::Plain,
        });
        cx.simulate_a11y_active(true);
        draw(cx);
        let text = view.read_with(cx, |view, _| view.text.clone());
        let bounds = text.read_with(cx, |state, _| state.bounds());
        let point = bounds.origin + gpui::point(px(2.), px(10.));
        cx.simulate_mouse_move(point, None, Default::default());
        // Keep replacement and dispatch inside one update: no repaint can
        // refresh the old hitbox or its installed callback between them.
        cx.update(|window, cx| {
            text.update(cx, |state, cx| {
                state.set_prepared(
                    PreparedText::parse("Current content", extensions.clone()).unwrap(),
                    None,
                    cx,
                )
            });
            window.dispatch_event(
                gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                    button: gpui::MouseButton::Left,
                    position: point,
                    click_count,
                    modifiers: Default::default(),
                    first_mouse: false,
                }),
                cx,
            );
        });
        cx.simulate_mouse_up(point, gpui::MouseButton::Left, Default::default());
        assert!(
            text.read_with(cx, |state, _| state.selected_text().is_empty()),
            "old frame restored selection; atomic={atomic}"
        );
        draw(cx);
        assert!(
            cx.update(gpui_base::TextSelection::selected_text)
                .is_empty()
        );
        click(cx, "Current content", 0, 2);
        assert_selection(&text, cx, 0..7, "Current");
    }
}

struct DeclaredBlock;
impl MarkdownPlugin for DeclaredBlock {
    fn name(&self) -> &str {
        "declared-block"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::Blockquote(_)).then(|| {
            MarkdownNode::new("declared-block", ())
                .text("copy alternative")
                .markdown("> custom source")
        })
    }
    fn presentation(&self, _: &MarkdownNode) -> gpui_base::text::MarkdownPresentation {
        gpui_base::text::MarkdownPresentation::Text("actual glyphs".into())
    }
}

#[test]
fn custom_block_glyph_multiclick_addresses_the_correct_repeated_owner() {
    let extensions = MarkdownExtensions::default().plugin(DeclaredBlock);
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| Scene {
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("> first\n\n> second", extensions.clone()).unwrap(),
                None,
                cx,
            );
            state
        }),
        extensions,
        width: 450.,
        selection_format: gpui_base::text::SelectionFormat::Plain,
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |view, _| view.text.clone());
    click(cx, "actual glyphs", 1, 2);
    assert_selection(&text, cx, 14..20, "actual");
    click(cx, "actual glyphs", 1, 3);
    assert_selection(&text, cx, 14..27, "actual glyphs");
    view.update(cx, |scene, cx| {
        scene.selection_format = gpui_base::text::SelectionFormat::Source;
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        cx.update(gpui_base::TextSelection::selected_text),
        "> custom source"
    );
    view.update(cx, |scene, cx| {
        scene.selection_format = gpui_base::text::SelectionFormat::Plain;
        scene.text.update(cx, |state, cx| state.select_all(cx));
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        cx.update(gpui_base::TextSelection::selected_text),
        "copy alternative\ncopy alternative"
    );
    text.read_with(cx, |state, _| {
        let range = state.rendered_selection().unwrap();
        assert_eq!(
            state.rendered_text().unwrap().selected_text(&range),
            Some("actual glyphs\nactual glyphs\n")
        );
    });
}

struct EmptyChip;
impl MarkdownPlugin for EmptyChip {
    fn name(&self) -> &str {
        "empty-chip"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        if let gpui_base::text::markdown_ast::Node::Image(image) = node {
            Some(
                MarkdownNode::new("empty-chip", ())
                    .text("")
                    .markdown(format!("![]({})", image.url))
                    .accessibility_label("chip"),
            )
        } else {
            None
        }
    }
    fn presentation(&self, _: &MarkdownNode) -> gpui_base::text::MarkdownPresentation {
        gpui_base::text::MarkdownPresentation::Text("CHIP".into())
    }
}

#[test]
fn empty_atomic_native_multiclick_and_drag_keep_repeated_object_identity() {
    let extensions = MarkdownExtensions::default().plugin(EmptyChip);
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| Scene {
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("A![](one)![](two)Z", extensions.clone()).unwrap(),
                None,
                cx,
            );
            state
        }),
        extensions,
        width: 450.,
        selection_format: gpui_base::text::SelectionFormat::Source,
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |view, _| view.text.clone());
    click(cx, "chip", 1, 2);
    assert_selection(&text, cx, 1..1, "![](two)");
    text.read_with(cx, |state, _| {
        assert!(!state.rendered_selection().unwrap().is_collapsed())
    });
    click(cx, "chip", 1, 3);
    assert_selection(&text, cx, 0..2, "A![](one)![](two)Z");
    // Both physical directions through the second visual chip select its edges.
    for backward in [false, true] {
        let tree = cx.a11y_tree().unwrap();
        let mut bounds = tree
            .nodes
            .iter()
            .filter(|(_, n)| n.label() == Some("chip"))
            .filter_map(|(_, n)| n.bounds())
            .collect::<Vec<_>>();
        bounds.sort_by(|a, b| a.x0.total_cmp(&b.x0));
        let b = bounds[1];
        let scale = cx.update(|w, _| f64::from(w.scale_factor()));
        let left = gpui::point(
            px((b.x0 / scale) as f32 + 1.),
            px(((b.y0 + b.y1) / (2. * scale)) as f32),
        );
        let right = gpui::point(px((b.x1 / scale) as f32 - 1.), left.y);
        let (start, end) = if backward {
            (right, left)
        } else {
            (left, right)
        };
        cx.simulate_mouse_move(start, None, Default::default());
        cx.simulate_event(gpui::MouseDownEvent {
            button: gpui::MouseButton::Left,
            position: start,
            click_count: 1,
            modifiers: Default::default(),
            first_mouse: false,
        });
        cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
        cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
        draw(cx);
        text.read_with(cx, |s, _| {
            let r = s.rendered_selection().expect("atomic pointer range");
            assert!(!r.is_collapsed());
            assert_eq!(r.is_backward(), backward);
            assert_eq!(r.bytes(), 1..1);
            assert_eq!(s.selected_text(), "![](two)");
        });
        assert_eq!(
            cx.update(gpui_base::TextSelection::selected_text),
            "![](two)"
        );
    }
}

#[test]
fn empty_atomic_paragraph_does_not_join_the_following_text_paragraph() {
    let extensions = MarkdownExtensions::default().plugin(EmptyChip);
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| Scene {
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("![](one)\n\nAfter", extensions.clone()).unwrap(),
                None,
                cx,
            );
            state
        }),
        extensions,
        width: 450.,
        selection_format: gpui_base::text::SelectionFormat::Source,
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |v, _| v.text.clone());
    click(cx, "After", 0, 3);
    assert_selection(&text, cx, 0..5, "After");
    assert_eq!(
        text.read_with(cx, |s, _| s
            .rendered_selection()
            .unwrap()
            .anchor()
            .content_position()
            .object_boundary()),
        1
    );
}
