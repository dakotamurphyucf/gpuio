//! Real GPUI prepaint bindings; not OS accessibility or VoiceOver acceptance.
use super::markdown_options_test::draw;
use super::*;
use gpui::{TestAppContext, VisualTestContext, accesskit};
use gpui_base::text::{MarkdownExtensions, PreparedText, RenderedSemanticAttachment};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Documents {
    text: Entity<TextViewState>,
    scrollable: bool,
    clicks: Arc<AtomicUsize>,
    extensions: MarkdownExtensions,
}

impl Render for Documents {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        div().w(px(440.)).h(px(200.)).child(
            TextView::new(&self.text)
                .scrollable(self.scrollable)
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
        clicks: clicks.clone(),
        extensions: Default::default(),
    });
    let text = view.read_with(cx, |view, _| view.text.clone());
    cx.simulate_a11y_active(true);
    draw(cx);
    let before = attachments(&text, cx);
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
        clicks: Arc::default(),
        extensions: extensions.clone(),
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let text = view.read_with(cx, |view, _| view.text.clone());
    let actual = attachments(&text, cx);
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
