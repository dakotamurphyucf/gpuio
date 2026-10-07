//! Real GPUI prepaint bindings; not OS accessibility or VoiceOver acceptance.
use super::markdown_options_test::draw;
use super::*;
use gpui::{TestAppContext, VisualTestContext, accesskit};
use gpui_base::text::{MarkdownExtensions, PreparedText, RenderedSemanticAttachment};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Documents {
    text: Entity<TextViewState>,
    scrollable: bool,
    max_lines: Option<usize>,
    clicks: Arc<AtomicUsize>,
    extensions: MarkdownExtensions,
}

impl Render for Documents {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        div().w(px(440.)).h(px(200.)).child(
            TextView::new(&self.text)
                .scrollable(self.scrollable)
                .when_some(self.max_lines, |view, lines| view.max_lines(lines))
                .markdown_extensions(self.extensions.clone())
                .code_block_actions(move |_, _, _| {
                    let clicks = clicks.clone();
                    div()
                        .id("native-code-control")
                        .role(accesskit::Role::Button)
                        .aria_label("Inspect code")
                        .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
                            clicks.fetch_add(1, Ordering::Relaxed);
                        })
                }),
        )
    }
}

fn attachments(
    text: &Entity<TextViewState>,
    cx: &mut VisualTestContext,
) -> Vec<RenderedSemanticAttachment> {
    cx.update(|window, cx| text.read(cx).rendered_semantic_attachments(window))
}

fn assert_text_positions(
    text: &Entity<TextViewState>,
    cx: &mut VisualTestContext,
) -> Vec<accesskit::NodeId> {
    let tree = cx.a11y_tree().unwrap();
    let runs: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == accesskit::Role::TextRun)
        .collect();
    assert!(!runs.is_empty());
    cx.update(|window, cx| {
        let state = text.read(cx);
        let projection = state.rendered_text().unwrap();
        for (id, node) in &runs {
            let count = node.character_lengths().len();
            let position = |index| accesskit::TextPosition {
                node: *id,
                character_index: index,
            };
            let start = state
                .rendered_accessible_position(window, position(0))
                .expect("published start maps to its prepared owner");
            let end = state
                .rendered_accessible_position(window, position(count))
                .expect("published end maps to its prepared owner");
            let selection = projection.selection(&start, &end).unwrap();
            assert_eq!(projection.selected_text(&selection), node.value());
            for character in 0..=count {
                let logical = state
                    .rendered_accessible_position(window, position(character))
                    .unwrap();
                let native = state
                    .rendered_accessible_text_position(window, &logical)
                    .unwrap();
                assert_eq!(
                    state
                        .rendered_accessible_position(window, native)
                        .unwrap()
                        .content_position(),
                    logical.content_position()
                );
            }
            assert!(
                state
                    .rendered_accessible_position(window, position(count + 1))
                    .is_none()
            );
            assert!(
                state
                    .rendered_accessible_position(window, position(usize::MAX))
                    .is_none()
            );
        }
    });
    runs.iter().map(|(id, _)| *id).collect()
}

fn descendants(tree: &accesskit::TreeUpdate, root: accesskit::NodeId) -> Vec<accesskit::NodeId> {
    let mut pending = vec![root];
    let mut result = Vec::new();
    while let Some(id) = pending.pop() {
        let node = &tree
            .nodes
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .unwrap()
            .1;
        pending.extend(node.children().iter().copied());
        result.push(id);
    }
    result
}

fn subtree_text(tree: &accesskit::TreeUpdate, root: accesskit::NodeId) -> String {
    let nodes: std::collections::HashMap<_, _> =
        tree.nodes.iter().map(|(id, node)| (*id, node)).collect();
    let mut result = String::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let node = nodes[&id];
        if node.role() == accesskit::Role::TextRun {
            result.push_str(node.value().unwrap());
        } else {
            pending.extend(node.children().iter().rev().copied());
        }
    }
    result
}

fn assert_complete_text(text: &Entity<TextViewState>, cx: &mut VisualTestContext) {
    let expected = text.read_with(cx, |state, _| {
        let projection = state.rendered_text().unwrap();
        projection
            .accessible_parts()
            .iter()
            .map(|part| projection.accessible_part_text(part.id()).unwrap())
            .collect::<String>()
    });
    let tree = cx.a11y_tree().unwrap();
    let document = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == accesskit::Role::Document)
        .unwrap()
        .0;
    assert_eq!(subtree_text(&tree, document), expected);
}

#[test]
fn semantic_attachments_preserve_real_subtrees_actions_and_stable_native_ids() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let source = "# Heading\n\n[open](test:target)\n\n```txt\ncode\n```\n";
    let clicks = Arc::new(AtomicUsize::new(0));
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut text = TextViewState::externally_prepared(cx);
            text.set_prepared(
                PreparedText::parse(source, Default::default()).unwrap(),
                None,
                cx,
            );
            text
        }),
        scrollable: false,
        max_lines: None,
        clicks: clicks.clone(),
        extensions: Default::default(),
    });
    let text = view.read_with(cx, |view, _| view.text.clone());
    cx.simulate_a11y_active(true);
    draw(cx);
    let before = attachments(&text, cx);
    let old_runs = assert_text_positions(&text, cx);
    assert_eq!(before.len(), 3);
    let tree = cx.a11y_tree().unwrap();
    let document = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == accesskit::Role::Document)
        .unwrap();
    for item in &before {
        assert!(document.1.children().contains(&item.node()));
        assert!(text.read_with(cx, |state, _| {
            state
                .rendered_text()
                .unwrap()
                .semantic_node(item.owner())
                .is_some()
        }));
    }
    assert!(
        tree.nodes
            .iter()
            .any(|(_, node)| node.role() == accesskit::Role::Heading)
    );
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, node)| node.role() == accesskit::Role::Link)
            .count(),
        1
    );
    let button = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Inspect code"))
        .unwrap()
        .0;
    assert!(descendants(&tree, before[2].node()).contains(&button));
    assert!(!descendants(&tree, before[0].node()).contains(&button));
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_node: button,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert_eq!(clicks.load(Ordering::Relaxed), 1);

    let retired = cx.update(|window, cx| {
        text.update(cx, |state, cx| {
            state.set_prepared(
                PreparedText::parse(source, Default::default()).unwrap(),
                None,
                cx,
            );
        });
        text.read(cx)
            .rendered_semantic_attachments(window)
            .is_empty()
    });
    assert!(
        retired,
        "old frame cannot bind a new preparation before redraw"
    );
    draw(cx);
    let after = attachments(&text, cx);
    let new_runs = assert_text_positions(&text, cx);
    assert!(
        old_runs.iter().all(|id| !new_runs.contains(id)),
        "equal-text replacement retires run IDs, not native controls"
    );
    cx.update(|window, cx| {
        for node in &old_runs {
            assert!(
                text.read(cx)
                    .rendered_accessible_position(
                        window,
                        accesskit::TextPosition {
                            node: *node,
                            character_index: 0
                        }
                    )
                    .is_none()
            );
        }
    });
    assert_eq!(
        before.iter().map(|item| item.node()).collect::<Vec<_>>(),
        after.iter().map(|item| item.node()).collect::<Vec<_>>()
    );
    for (old, new) in before.iter().zip(after.iter()) {
        assert_ne!(old.owner(), new.owner());
    }
    let tree = cx.a11y_tree().unwrap();
    assert_eq!(
        tree.nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Inspect code"))
            .unwrap()
            .0,
        button
    );

    let mut other_app = cx.cx.clone();
    let (other, other_cx) = other_app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut text = TextViewState::externally_prepared(cx);
            text.set_prepared(
                PreparedText::parse(source, Default::default()).unwrap(),
                None,
                cx,
            );
            text
        }),
        scrollable: false,
        max_lines: None,
        clicks: Arc::default(),
        extensions: Default::default(),
    });
    other_cx.simulate_a11y_active(true);
    draw(other_cx);
    assert!(
        attachments(&text, other_cx).is_empty(),
        "another window cannot claim these attachments"
    );
    let other_text = other.read_with(other_cx, |view, _| view.text.clone());
    let other_attachments = attachments(&other_text, other_cx);
    assert_eq!(other_attachments.len(), 3);
    assert_text_positions(&other_text, other_cx);
    other_cx.update(|window, cx| {
        assert!(
            text.read(cx)
                .rendered_accessible_position(
                    window,
                    accesskit::TextPosition {
                        node: new_runs[0],
                        character_index: 0
                    }
                )
                .is_none()
        );
    });
    assert!(attachments(&other_text, cx).is_empty());
    assert_eq!(attachments(&text, cx), after);
}

#[test]
fn semantic_attachments_exclude_rows_only_prepainted_for_measurement() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let extensions = MarkdownExtensions::default().block_renderer("refresh", |_, _, _| div());
    let source = (0..200)
        .map(|index| format!("Paragraph {index}\n\n"))
        .collect::<String>();
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut text = TextViewState::externally_prepared(cx);
            text.set_prepared(
                PreparedText::parse(&source, extensions.clone()).unwrap(),
                None,
                cx,
            );
            text
        }),
        scrollable: true,
        max_lines: None,
        clicks: Arc::default(),
        extensions: extensions.clone(),
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |view, _| view.text.clone());
    let actual = attachments(&text, cx);
    assert_text_positions(&text, cx);
    assert!(!actual.is_empty());
    assert!(
        actual.len() < 200,
        "virtualized rows are not all native attachments"
    );
    assert_eq!(
        text.read_with(cx, |state, _| state
            .rendered_text()
            .unwrap()
            .semantic_children(None)
            .count()),
        200
    );
    let tree = cx.a11y_tree().unwrap();
    cx.update(|window, cx| {
        let state = text.read(cx);
        let projection = state.rendered_text().unwrap();
        let mut logical_only = 0;
        for part in projection.accessible_parts() {
            for character in [0, part.character_count()] {
                let logical = projection
                    .accessible_position(part.id(), character)
                    .unwrap();
                let native = state
                    .rendered_accessible_text_position(window, &logical)
                    .expect("unrealized positions remain readable");
                let node = &tree
                    .nodes
                    .iter()
                    .find(|(id, _)| *id == native.node)
                    .expect("position names a real tree node")
                    .1;
                if node.bounds().is_none() {
                    logical_only += 1;
                }
            }
        }
        assert!(
            logical_only > 0,
            "unrealized text must not claim invented layout bounds"
        );
    });
    assert_complete_text(&text, cx);
    for item in &actual {
        assert!(tree.nodes.iter().any(|(node, _)| *node == item.node()));
    }
    let stable_nodes = actual.iter().map(|item| item.node()).collect::<Vec<_>>();
    let mut previous = actual;
    for refresh_resources in [false, true] {
        let retired = cx.update(|window, cx| {
            let refreshed = extensions
                .clone()
                .block_renderer("refresh", |_, _, _| div());
            if refresh_resources {
                view.update(cx, |view, cx| {
                    view.extensions = refreshed.clone();
                    cx.notify();
                });
            }
            text.update(cx, |state, cx| {
                if refresh_resources {
                    state.set_markdown_extensions(Arc::new(refreshed), cx);
                } else {
                    state.set_prepared(
                        PreparedText::parse(&source, extensions.clone()).unwrap(),
                        Some(source.len()),
                        cx,
                    );
                }
            });
            text.read(cx)
                .rendered_semantic_attachments(window)
                .is_empty()
        });
        assert!(retired);
        draw(cx);
        let current = attachments(&text, cx);
        assert_text_positions(&text, cx);
        assert_complete_text(&text, cx);
        assert_eq!(
            current.iter().map(|item| item.node()).collect::<Vec<_>>(),
            stable_nodes,
            "refresh_resources={refresh_resources}"
        );
        for (old, new) in previous.iter().zip(&current) {
            assert_ne!(old.owner(), new.owner());
            assert!(text.read_with(cx, |state, _| {
                state
                    .rendered_text()
                    .unwrap()
                    .semantic_node(new.owner())
                    .is_some()
            }));
        }
        previous = current;
    }
    cx.simulate_a11y_active(false);
    view.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert!(attachments(&text, cx).is_empty());
    cx.simulate_a11y_active(true);
    view.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert_eq!(attachments(&text, cx), previous);
}

#[test]
fn native_text_runs_preserve_unicode_wrapping_and_link_reading_order() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let linked = format!(
        "First `code`  \n{}",
        "Read `code` 世界 and continue reading across several wrapped lines ".repeat(8)
    );
    let source =
        format!("# Heading λ🙂\n\nPlain e\u{301} אבג English 🙂.\n\n[{linked}](test:target)\n");
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut text = TextViewState::externally_prepared(cx);
            text.set_prepared(
                PreparedText::parse(&source, Default::default()).unwrap(),
                None,
                cx,
            );
            text
        }),
        scrollable: false,
        max_lines: None,
        clicks: Arc::default(),
        extensions: Default::default(),
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let state = view.read_with(cx, |view, _| view.text.clone());
    assert_text_positions(&state, cx);
    let tree = cx.a11y_tree().unwrap();
    fn text(tree: &accesskit::TreeUpdate, id: accesskit::NodeId) -> String {
        let node = &tree
            .nodes
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .unwrap()
            .1;
        if node.role() == accesskit::Role::TextRun {
            return node.value().unwrap().to_owned();
        }
        node.children()
            .iter()
            .map(|child| text(tree, *child))
            .collect()
    }
    let runs: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == accesskit::Role::TextRun)
        .collect();
    assert!(!runs.is_empty());
    assert!(
        runs.iter()
            .any(|(_, node)| node.text_direction() == Some(accesskit::TextDirection::RightToLeft))
    );
    for (_, node) in &runs {
        assert_eq!(
            node.character_lengths()
                .iter()
                .map(|n| *n as usize)
                .sum::<usize>(),
            node.value().unwrap().len()
        );
        if let Some(bounds) = node.bounds() {
            assert!(bounds.width() >= 0. && bounds.height() > 0.);
            assert_eq!(
                node.character_positions().unwrap().len(),
                node.character_lengths().len()
            );
            assert_eq!(
                node.character_widths().unwrap().len(),
                node.character_lengths().len()
            );
            assert!(
                node.character_widths()
                    .unwrap()
                    .iter()
                    .all(|width| *width >= 0.)
            );
        }
    }
    for label in ["Heading λ🙂", "Plain e\u{301} אבג English 🙂."] {
        let (id, _) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == accesskit::Role::Label && node.value() == Some(label))
            .unwrap();
        assert_eq!(text(&tree, *id), label);
    }
    let links: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == accesskit::Role::Link)
        .collect();
    assert_eq!(links.len(), 1);
    assert_eq!(
        text(&tree, links[0].0),
        linked.replace('`', "").replace("  \n", "\n")
    );
}

#[test]
fn completed_subtree_visit_excludes_parent_and_foreign_siblings() {
    use std::{cell::RefCell, rc::Rc};
    struct Fixture(Rc<RefCell<Vec<accesskit::NodeId>>>);
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let captured = self.0.clone();
            div()
                .child(
                    div()
                        .id("before")
                        .role(accesskit::Role::Label)
                        .aria_label("Before"),
                )
                .child(
                    div()
                        .id("owner")
                        .role(accesskit::Role::Group)
                        .aria_label("Owner")
                        .child(
                            div()
                                .id("inside")
                                .role(accesskit::Role::Label)
                                .aria_label("Inside"),
                        )
                        .a11y_synthetic_children(move |builder| {
                            let mut text = accesskit::Node::new(accesskit::Role::TextRun);
                            text.set_value("text");
                            text.set_character_lengths([1, 1, 1, 1]);
                            let id = builder.synthetic_node_id("text");
                            assert!(builder.push_child(id, text));
                            let mut found = Vec::new();
                            builder.visit_descendants(|id, _| found.push(id));
                            *captured.borrow_mut() = found;
                        }),
                )
                .child(
                    div()
                        .id("after")
                        .role(accesskit::Role::Label)
                        .aria_label("After"),
                )
        }
    }
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let captured = Rc::new(RefCell::new(Vec::new()));
    let (_, cx) = app.add_window_view(|_, _| Fixture(captured.clone()));
    cx.simulate_a11y_active(true);
    draw(cx);
    let tree = cx.a11y_tree().unwrap();
    let id = |name| {
        tree.nodes
            .iter()
            .find(|(_, node)| node.label() == Some(name))
            .unwrap()
            .0
    };
    let seen = captured.borrow();
    assert_eq!(seen.len(), 2);
    assert!(seen.contains(&id("Inside")));
    assert!(!seen.contains(&id("Owner")));
    assert!(!seen.contains(&id("Before")));
    assert!(!seen.contains(&id("After")));
    assert!(seen.iter().any(|id| {
        tree.nodes
            .iter()
            .any(|(node_id, node)| node_id == id && node.role() == accesskit::Role::TextRun)
    }));
}

#[test]
fn logical_offscreen_document_preserves_nested_structure_and_link_ranges() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let prefix = (0..60)
        .map(|i| format!("Initial paragraph {i}\n\n"))
        .collect::<String>();
    let source = format!(
        "{prefix}## Offscreen heading\n\n- first\n  - nested [λ🙂 `code`](test:nested) text\n- last\n\n| First | Second |\n| --- | --- |\n| α | β |\n\n```txt\nline one\nline two\n```\n"
    );
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse(&source, Default::default()).unwrap(),
                None,
                cx,
            );
            state
        }),
        scrollable: true,
        max_lines: None,
        clicks: Arc::default(),
        extensions: Default::default(),
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |view, _| view.text.clone());
    assert_complete_text(&text, cx);
    assert_text_positions(&text, cx);
    let tree = cx.a11y_tree().unwrap();
    for role in [
        accesskit::Role::Heading,
        accesskit::Role::List,
        accesskit::Role::ListItem,
        accesskit::Role::Table,
        accesskit::Role::Row,
        accesskit::Role::ColumnHeader,
        accesskit::Role::Cell,
        accesskit::Role::Link,
    ] {
        let nodes: Vec<_> = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == role)
            .collect();
        assert!(!nodes.is_empty(), "missing {role:?}");
        assert!(
            nodes.iter().all(|(_, node)| node.bounds().is_none()),
            "offscreen {role:?}"
        );
    }
    let heading = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == accesskit::Role::Heading)
        .unwrap();
    assert_eq!(heading.1.level(), Some(2));
    let link = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == accesskit::Role::Link)
        .unwrap();
    assert_eq!(link.1.url(), Some("test:nested"));
    assert_eq!(subtree_text(&tree, link.0), "λ🙂 code");
    let table = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == accesskit::Role::Table)
        .unwrap();
    assert_eq!(table.1.column_count(), Some(2));
}

#[test]
fn logical_empty_document_has_a_caret_and_clamped_preview_stays_clamped() {
    for source in ["", "# Heading\n\nFirst paragraph\n\nLast paragraph"] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (view, cx) = app.add_window_view(|_, cx| Documents {
            text: cx.new(|cx| {
                let mut state = TextViewState::externally_prepared(cx);
                state.set_prepared(
                    PreparedText::parse(source, Default::default()).unwrap(),
                    None,
                    cx,
                );
                state
            }),
            scrollable: false,
            max_lines: (!source.is_empty()).then_some(1),
            clicks: Arc::default(),
            extensions: Default::default(),
        });
        cx.simulate_a11y_active(true);
        draw(cx);
        let text = view.read_with(cx, |view, _| view.text.clone());
        if source.is_empty() {
            assert_complete_text(&text, cx);
            assert_eq!(assert_text_positions(&text, cx).len(), 1);
        } else {
            let tree = cx.a11y_tree().unwrap();
            assert!(!tree.nodes.iter().any(|(_, node)| {
                node.role() == accesskit::Role::TextRun
                    && !node.is_hidden()
                    && node
                        .value()
                        .is_some_and(|value| value.contains("Last paragraph"))
            }));
        }
    }
}

struct AtomicAlternative;
impl gpui_base::text::MarkdownPlugin for AtomicAlternative {
    fn name(&self) -> &str {
        "logical-atomic"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        context: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<gpui_base::text::MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::Blockquote(_)).then(|| {
            let source = context.node_source(node).unwrap_or_default();
            gpui_base::text::MarkdownNode::new("logical-atomic", ())
                .text(if source.contains("empty") {
                    ""
                } else {
                    "atomic λ\r\nsecond 🙂"
                })
                .markdown(source)
        })
    }
    fn render(
        &self,
        _: &gpui_base::text::MarkdownNode,
        _: &mut Window,
        _: &mut App,
    ) -> impl IntoElement {
        div().w_full().h(px(40.)).child("Actual native widget")
    }
}

#[test]
fn logical_atomic_alternatives_are_readable_without_interior_selection_positions() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let extensions = MarkdownExtensions::default().plugin(AtomicAlternative);
    let prefix = (0..60)
        .map(|i| format!("Paragraph {i}\n\n"))
        .collect::<String>();
    let source = format!("{prefix}> alternative\n\nBetween\n\n> empty\n");
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse(&source, extensions.clone()).unwrap(),
                None,
                cx,
            );
            state
        }),
        scrollable: true,
        max_lines: None,
        clicks: Arc::default(),
        extensions: extensions.clone(),
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |view, _| view.text.clone());
    assert_complete_text(&text, cx);
    text.read_with(cx, |state, _| {
        let projection = state.rendered_text().unwrap();
        let first = projection.position(0).unwrap();
        // Reference the full set of intervals rather than the production index.
        // Check both directions, all legal scalar/CRLF positions, and every
        // atomic interior; object-edge slot tests follow below.
        for byte in 0..=projection.text().len() {
            if let Some(position) = projection.position(byte) {
                let inside_atomic = projection.parts().iter().any(|part| {
                    let range = part.bytes();
                    part.is_atomic() && range.start < byte && byte < range.end
                });
                assert_eq!(
                    projection.selection(&first, &position).is_err(),
                    inside_atomic
                );
                assert_eq!(
                    projection.selection(&position, &first).is_err(),
                    inside_atomic
                );
            }
        }
    });
    let tree = cx.a11y_tree().unwrap();
    for (value, start_valid, end_valid) in [
        ("atomic λ\r\n", true, false),
        ("second 🙂", false, true),
        ("\u{fffc}", true, true),
    ] {
        let (id, node) = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == accesskit::Role::TextRun && node.value() == Some(value)
            })
            .unwrap();
        assert!(node.bounds().is_none());
        let count = node.character_lengths().len();
        if value.ends_with("\r\n") {
            assert_eq!(count, value.chars().count() - 1);
        }
        cx.update(|window, cx| {
            let state = text.read(cx);
            for character_index in 0..=count {
                let position = state.rendered_accessible_position(
                    window,
                    accesskit::TextPosition {
                        node: *id,
                        character_index,
                    },
                );
                let expected = (character_index == 0 && start_valid)
                    || (character_index == count && end_valid);
                assert_eq!(position.is_some(), expected, "{value:?}: {character_index}");
                if let Some(position) = position {
                    let native = state
                        .rendered_accessible_text_position(window, &position)
                        .unwrap();
                    assert_eq!(
                        state
                            .rendered_accessible_position(window, native)
                            .unwrap()
                            .content_position(),
                        position.content_position()
                    );
                }
            }
        });
    }
}

#[test]
fn large_logical_document_keeps_complete_text_and_stable_redraw_ids() {
    for count in [250, 1000] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let prefix = (0..60)
            .map(|i| format!("Paragraph {i}\n\n"))
            .collect::<String>();
        let items = (0..count)
            .map(|i| format!("- Entry {i} λ🙂\n"))
            .collect::<String>();
        let source = format!("{prefix}{items}");
        let (view, cx) = app.add_window_view(|_, cx| Documents {
            text: cx.new(|cx| {
                let mut state = TextViewState::externally_prepared(cx);
                state.set_prepared(
                    PreparedText::parse(&source, Default::default()).unwrap(),
                    None,
                    cx,
                );
                state
            }),
            scrollable: true,
            max_lines: None,
            clicks: Arc::default(),
            extensions: Default::default(),
        });
        let text = view.read_with(cx, |view, _| view.text.clone());
        cx.simulate_a11y_active(true);
        let mut timings = Vec::new();
        let mut previous_ids = None;
        for _ in 0..3 {
            view.update(cx, |_, cx| cx.notify());
            let start = std::time::Instant::now();
            draw(cx);
            timings.push(start.elapsed().as_secs_f64() * 1000.);
            assert_complete_text(&text, cx);
            let tree = cx.a11y_tree().unwrap();
            let ids: std::collections::BTreeSet<_> = tree.nodes.iter().map(|(id, _)| *id).collect();
            assert_eq!(ids.len(), tree.nodes.len(), "unique tree identities");
            if let Some(previous) = previous_ids {
                assert_eq!(ids, previous);
            }
            previous_ids = Some(ids);
            assert!(
                attachments(&text, cx).len() < 20,
                "only visible native blocks realized"
            );
        }
        eprintln!(
            "AX_LOGICAL_PUBLICATION_DIAGNOSTIC list_items={count} nodes={} draw_ms={timings:?}",
            previous_ids.unwrap().len()
        );
    }
}
