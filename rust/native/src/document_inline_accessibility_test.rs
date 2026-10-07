//! Readable atomic parts belong to native objects/links, not partial selections.
use super::*;

#[derive(Default)]
struct InlineAlternative(Arc<AtomicUsize>);
impl gpui_base::text::MarkdownPlugin for InlineAlternative {
    fn name(&self) -> &str {
        "accessible-inline-alternative"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<gpui_base::text::MarkdownNode> {
        use gpui_base::text::markdown_ast::Node;
        let value = match node {
            Node::InlineCode(_) => "atom λ\r\n🙂",
            Node::Image(_) => "",
            _ => return None,
        };
        Some(gpui_base::text::MarkdownNode::new(self.name(), ()).text(value))
    }
    fn render(
        &self,
        _: &gpui_base::text::MarkdownNode,
        _: &mut Window,
        _: &mut App,
    ) -> impl IntoElement {
        let clicks = self.0.clone();
        div()
            .id("inline-native-control")
            .role(accesskit::Role::Button)
            .aria_label("Inspect inline object")
            .w(px(42.))
            .h(px(22.))
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
                clicks.fetch_add(1, Ordering::Relaxed);
            })
    }
}

#[test]
fn native_inline_alternatives_preserve_complete_reading_order() {
    assert_realized_with_extensions(
        "Before `atom` after",
        MarkdownExtensions::default().plugin(InlineAlternative::default()),
    );
}

#[test]
fn native_linked_inline_alternative_belongs_to_the_existing_link() {
    let tree = assert_realized_with_extensions(
        "[Before `atom` after](test:object)",
        MarkdownExtensions::default().plugin(InlineAlternative::default()),
    );
    let links: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == accesskit::Role::Link)
        .collect();
    assert_eq!(links.len(), 1);
    assert_eq!(subtree_text(&tree, links[0].0), "Before atom λ\r\n🙂 after");
}

#[test]
fn native_linked_empty_objects_have_distinct_readable_edges() {
    let tree = assert_realized_with_extensions(
        "[![](one)![](two)](test:objects)",
        MarkdownExtensions::default().plugin(InlineAlternative::default()),
    );
    let links: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == accesskit::Role::Link)
        .collect();
    assert_eq!(links.len(), 1);
    assert_eq!(subtree_text(&tree, links[0].0), "\u{fffc}\u{fffc}");
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, node)| node.role() == accesskit::Role::Button)
            .count(),
        2
    );
}

#[test]
fn inline_atomic_reading_keeps_native_actions_and_retires_old_coordinates() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let clicks = Arc::new(AtomicUsize::new(0));
    let extensions = MarkdownExtensions::default().plugin(InlineAlternative(clicks.clone()));
    let source = "Before `atom` and [![](one)![](two)](test:objects) after";
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse(source, extensions.clone()).unwrap(),
                None,
                cx,
            );
            state
        }),
        scrollable: false,
        max_lines: None,
        clicks: Arc::default(),
        extensions: extensions.clone(),
    });
    let text = view.read_with(cx, |view, _| view.text.clone());
    cx.simulate_a11y_active(true);
    draw(cx);
    let mut previous_buttons = None;
    let mut original_runs = Vec::new();
    for phase in 0..3 {
        assert_complete_text(&text, cx);
        let tree = cx.a11y_tree().unwrap();
        let buttons: std::collections::BTreeSet<_> = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.label() == Some("Inspect inline object"))
            .map(|(id, node)| {
                assert_eq!(node.role(), accesskit::Role::Button);
                assert!(!node.is_hidden());
                *id
            })
            .collect();
        assert_eq!(buttons.len(), 3);
        if let Some(previous) = &previous_buttons {
            assert_eq!(&buttons, previous);
        }
        previous_buttons = Some(buttons.clone());
        for button in buttons {
            cx.simulate_a11y_action(accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_node: button,
                target_tree: accesskit::TreeId::ROOT,
                data: None,
            });
        }
        draw(cx); // Platform action requests are delivered through the event queue.
        assert_eq!(clicks.load(Ordering::Relaxed), (phase + 1) * 3);
        let mut first_lines = 0;
        let mut last_lines = 0;
        let mut replacements = 0;
        for (id, node) in &tree.nodes {
            if node.role() != accesskit::Role::TextRun {
                continue;
            }
            let count = node.character_lengths().len();
            let validity: Vec<_> = match node.value().unwrap() {
                "atom λ\r\n" => {
                    first_lines += 1;
                    (0..=count).map(|i| i == 0).collect()
                }
                "🙂" => {
                    last_lines += 1;
                    (0..=count).map(|i| i == count).collect()
                }
                "\u{fffc}" => {
                    replacements += 1;
                    assert_eq!(count, 1);
                    vec![true, true]
                }
                _ => continue,
            };
            assert!(
                node.bounds().is_none(),
                "arbitrary widget alternatives have no fabricated glyph bounds"
            );
            cx.update(|window, cx| {
                for (index, valid) in validity.into_iter().enumerate() {
                    assert_eq!(
                        text.read(cx)
                            .rendered_accessible_position(
                                window,
                                accesskit::TextPosition {
                                    node: *id,
                                    character_index: index
                                }
                            )
                            .is_some(),
                        valid
                    );
                }
            });
            if phase == 0 {
                original_runs.push(*id);
            }
        }
        assert_eq!((first_lines, last_lines, replacements), (1, 1, 2));
        if phase == 0 {
            view.update(cx, |_, cx| cx.notify());
        } else if phase == 1 {
            text.update(cx, |state, cx| {
                state.set_prepared(
                    PreparedText::parse(source, extensions.clone()).unwrap(),
                    None,
                    cx,
                )
            });
            cx.update(|window, cx| {
                for id in &original_runs {
                    assert!(
                        text.read(cx)
                            .rendered_accessible_position(
                                window,
                                accesskit::TextPosition {
                                    node: *id,
                                    character_index: 0
                                }
                            )
                            .is_none()
                    );
                }
            });
        } else {
            assert!(
                original_runs
                    .iter()
                    .all(|id| !tree.nodes.iter().any(|(new, _)| id == new))
            );
        }
        draw(cx);
    }
}
