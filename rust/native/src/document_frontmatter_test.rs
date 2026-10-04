//! Native text/layout/accessibility on TestPlatform; no physical desktop claim.
use super::*;
use gpui_base::text::{PreparedMarkdown, SelectionFormat};

const SOURCE: &str = "---\nname: Native 世界\nsummary: >-\n  Readable text that wraps naturally inside a narrow native document.\nnotes: |-\n  first line\n  last line\n---\n\nBody text\n";

fn extensions() -> gpui_base::text::MarkdownExtensions {
    crate::document_markdown::extensions_with_options(
        Default::default(),
        gpuio_protocol::document::MarkdownOptions {
            frontmatter: gpuio_protocol::document::Frontmatter::DescriptionList,
            mdx: false,
        },
    )
}
fn prepare(source: &str) -> PreparedMarkdown {
    PreparedMarkdown::parse(source, extensions()).unwrap()
}
struct Metadata {
    extensions: gpui_base::text::MarkdownExtensions,
    text: Entity<TextViewState>,
    width: f32,
    limit: Option<usize>,
    selection_format: SelectionFormat,
}
impl Render for Metadata {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(self.width))
            .child(gpui_base::TextSelectionLayer)
            .child(
                TextView::new(&self.text)
                    .markdown_extensions(self.extensions.clone())
                    .selection_format(self.selection_format)
                    .scrollable(false)
                    .when_some(self.limit, |view, limit| view.max_lines(limit)),
            )
    }
}
fn scene(cx: &mut Context<Metadata>) -> Metadata {
    Metadata {
        extensions: extensions(),
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(prepare(SOURCE), None, cx);
            state
        }),
        width: 480.,
        limit: None,
        selection_format: SelectionFormat::Plain,
    }
}
fn text_bounds(cx: &mut VisualTestContext, value: &str) -> gpui::Bounds<gpui::Pixels> {
    let tree = cx.a11y_tree().unwrap();
    let rect = tree
        .nodes
        .iter()
        .find(|(_, n)| n.value() == Some(value))
        .unwrap_or_else(|| panic!("missing text {value:?}"))
        .1
        .bounds()
        .unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    gpui::Bounds::from_corners(
        gpui::point(px((rect.x0 / scale) as f32), px((rect.y0 / scale) as f32)),
        gpui::point(px((rect.x1 / scale) as f32), px((rect.y1 / scale) as f32)),
    )
}
#[test]
fn frontmatter_descriptions_layout_semantics_copy_and_reflow() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |v, _| v.text.clone());
    let tree = cx.a11y_tree().unwrap();
    for (role, count) in [
        (gpui::Role::DescriptionList, 1),
        (gpui::Role::Term, 3),
        (gpui::Role::Definition, 3),
    ] {
        assert_eq!(
            tree.nodes.iter().filter(|(_, n)| n.role() == role).count(),
            count
        );
    }
    let label = text_bounds(cx, "name:");
    let value = text_bounds(cx, "Native 世界");
    assert!(label.right() < value.left());
    assert_eq!(label.top(), value.top());
    assert!(text_bounds(cx, "summary:").top() > label.bottom());
    text.update(cx, |text, cx| text.select_all(cx));
    let plain = text.read_with(cx, |text, _| text.selected_text());
    assert!(
        plain.contains("name: Native 世界\nsummary: Readable text"),
        "{plain:?}"
    );
    assert!(plain.contains("notes: first line\nlast line"));
    assert_eq!(
        cx.update(gpui_base::TextSelection::selected_text),
        plain.trim()
    );
    assert_eq!(
        cx.update(|w, cx| gpui_base::TextSelection::selected_text_limited(
            w,
            plain.trim().len(),
            cx
        )),
        Ok(plain.trim().to_owned()),
        "bounded collection preserves the renderer's plain-text normalization"
    );
    view.update(cx, |v, cx| {
        v.selection_format = SelectionFormat::Source;
        cx.notify();
    });
    draw(cx);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), SOURCE);
    assert_eq!(
        cx.update(|w, cx| gpui_base::TextSelection::selected_text_limited(w, SOURCE.len(), cx)),
        Ok(SOURCE.to_owned()),
        "bounded collection preserves source-format copy callbacks"
    );
    assert_eq!(
        cx.update(|w, cx| gpui_base::TextSelection::selected_text_limited(w, SOURCE.len() - 1, cx)),
        Err(gpui_base::TextSelectionCopyLimitExceeded)
    );
    let before = text.read_with(cx, |t, _| t.displayed_text().unwrap());
    let height = text.read_with(cx, |t, _| t.bounds().size.height);
    view.update(cx, |v, cx| {
        v.width = 230.;
        cx.notify();
    });
    draw(cx);
    assert!(text.read_with(cx, |t, _| t.bounds().size.height) > height);
    assert!(Arc::ptr_eq(
        &before,
        &text.read_with(cx, |t, _| t.displayed_text().unwrap())
    ));
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), SOURCE);
    let label = text_bounds(cx, "name:");
    let value = text_bounds(cx, "Native 世界");
    assert!(label.right() < value.left());
    assert!(value.right() <= px(231.));
}
#[test]
fn frontmatter_descriptions_partial_selection_survives_append_but_not_metadata_replacement() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |v, _| v.text.clone());
    let bounds = text_bounds(cx, "Native 世界");
    let start = gpui::point(bounds.left() + px(1.), bounds.top() + px(8.));
    let end = gpui::point(bounds.right() - px(1.), bounds.top() + px(8.));
    cx.simulate_mouse_move(start, None, Default::default());
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
    cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
    draw(cx);
    let selected = text.read_with(cx, |t, _| t.selected_text());
    assert_eq!(selected, "Native 世界\n");
    view.update(cx, |v, cx| {
        v.selection_format = SelectionFormat::Source;
        cx.notify();
    });
    draw(cx);
    assert_eq!(text.read_with(cx, |t, _| t.selected_text()), "Native 世界");
    text.update(cx, |t, cx| {
        t.set_prepared(
            prepare(&format!("{SOURCE}\nAppended")),
            Some(SOURCE.len()),
            cx,
        )
    });
    draw(cx);
    assert_eq!(text.read_with(cx, |t, _| t.selected_text()), "Native 世界");
    text.update(cx, |t, cx| {
        t.set_prepared(
            prepare(&SOURCE.replace("Native 世界", "Replacement")),
            Some(0),
            cx,
        )
    });
    draw(cx);
    assert!(!text.read_with(cx, |t, _| t.has_local_selection()));
    assert!(
        cx.update(gpui_base::TextSelection::selected_text)
            .is_empty()
    );
}
#[test]
fn frontmatter_descriptions_obey_preview_clipping() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    view.update(cx, |v, cx| {
        v.limit = Some(2);
        cx.notify();
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |v, _| v.text.clone());
    assert!(text.read_with(cx, |t, _| t.is_clamped()));
    let tree = cx.a11y_tree().unwrap();
    let mut cursor = tree
        .nodes
        .iter()
        .find(|(_, n)| n.value() == Some("first line\nlast line"))
        .unwrap()
        .0;
    let mut hidden = false;
    loop {
        let node = &tree.nodes.iter().find(|(id, _)| *id == cursor).unwrap().1;
        hidden |= node.is_hidden();
        let Some((parent, _)) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.children().contains(&cursor))
        else {
            break;
        };
        cursor = *parent;
    }
    assert!(hidden, "clipped metadata must have hidden semantics");
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    assert!(!text.read_with(cx, |t, _| t.is_clamped()));
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.value() == Some("first line\nlast line"))
            .unwrap()
            .1
            .is_hidden()
    );
}

#[test]
fn frontmatter_descriptions_search_matches_painted_label_and_value_fragments() {
    use gpui_base::{
        input::{RangeBackground, RangeBackgrounds},
        text::TextBackgrounds,
    };
    struct Ranges(Vec<RangeBackground>);
    impl RangeBackgrounds for Ranges {
        fn ranges(&self) -> &[RangeBackground] {
            &self.0
        }
    }
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    draw(cx);
    let text = view.read_with(cx, |v, _| v.text.clone());
    let source = text.read_with(cx, |t, _| t.displayed_text().unwrap());
    for (query, count, rgba) in [
        ("name", 1, 0x771133ff),
        ("Native", 1, 0x116633ff),
        ("Body", 1, 0x335511ff),
        ("name: Native", 0, 0x223377ff),
    ] {
        let matcher = crate::highlight_search::Matcher::new(&gpuio_protocol::highlight::Query {
            text: query.into(),
            case_sensitive: true,
            whole_word: false,
        })
        .unwrap();
        let mut budget = Default::default();
        let mut total = 0;
        let color: gpui::Hsla = gpui::rgba(rgba).into();
        let layers = source
            .fragments()
            .iter()
            .map(|fragment| {
                let matches = matcher
                    .find([fragment.text().as_ref()], &mut budget, || false)
                    .unwrap();
                total += matches.total;
                (!matches.ranges.is_empty()).then(|| {
                    Rc::new(Ranges(
                        matches
                            .ranges
                            .into_iter()
                            .map(|bytes| RangeBackground {
                                bytes,
                                color,
                                radius: px(0.),
                            })
                            .collect(),
                    )) as Rc<dyn RangeBackgrounds>
                })
            })
            .collect();
        assert_eq!(total, count);
        let layers = Rc::new(TextBackgrounds::new(source.clone(), layers).unwrap());
        text.update(cx, |text, cx| {
            assert!(text.set_text_backgrounds(Some(layers), cx))
        });
        draw(cx);
        cx.update(|window, _| {
            assert_eq!(
                window
                    .painted_quads()
                    .iter()
                    .any(|quad| quad.background == gpui::Background::from(color)),
                count > 0,
                "painted search result for {query:?}"
            )
        });
    }
}
