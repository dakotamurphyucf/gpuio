//! Native rendered line relationships; OS range queries are separately tested.
use super::*;

fn line_text(tree: &accesskit::TreeUpdate, mut id: accesskit::NodeId) -> String {
    let nodes: std::collections::HashMap<_, _> =
        tree.nodes.iter().map(|(id, node)| (*id, node)).collect();
    let mut seen = std::collections::HashSet::new();
    while let Some(previous) = nodes[&id].previous_on_line() {
        assert!(seen.insert(id), "cyclic previous-line relationship");
        assert_eq!(nodes[&previous].next_on_line(), Some(id));
        id = previous;
    }
    seen.clear();
    let mut text = String::new();
    loop {
        assert!(seen.insert(id), "cyclic next-line relationship");
        let node = nodes[&id];
        assert_eq!(node.role(), accesskit::Role::TextRun);
        text.push_str(node.value().unwrap());
        match node.next_on_line() {
            Some(next) => {
                assert_eq!(nodes[&next].previous_on_line(), Some(id));
                id = next;
            }
            None => return text,
        }
    }
}

#[test]
fn rendered_visual_line_crosses_styling_links_and_unicode_runs() {
    let tree = assert_realized_complete("left **bold** 世界 🙂 [right](test:line)");
    for (id, node) in &tree.nodes {
        if node.role() == accesskit::Role::TextRun && node.bounds().is_some() {
            assert_eq!(line_text(&tree, *id).trim_end(), "left bold 世界 🙂 right");
        }
    }
    assert!(tree.nodes.iter().any(|(_, n)| n.next_on_line().is_some()));
}

#[test]
fn rendered_visual_lines_keep_hard_breaks_and_table_cells_separate() {
    let tree = assert_realized_complete(
        "one **bold**\n\ntwo *italic*\n\n| Left | Right |\n| --- | --- |\n| A **cell** | B *cell* |",
    );
    let expected = [
        "one bold",
        "two italic",
        "Left",
        "Right",
        "A cell",
        "B cell",
    ];
    let mut found = std::collections::HashSet::new();
    for (id, node) in &tree.nodes {
        if node.role() == accesskit::Role::TextRun && node.bounds().is_some() {
            let text = line_text(&tree, *id);
            let text = text.trim_end();
            assert!(expected.contains(&text), "unexpected visual line: {text:?}");
            found.insert(text.to_owned());
        }
    }
    assert_eq!(found.len(), expected.len());
}

struct Reflow {
    text: Entity<TextViewState>,
    width: f32,
}

impl Render for Reflow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(self.width)).child(TextView::new(&self.text))
    }
}

#[test]
fn rendered_visual_lines_reflow_without_stale_links_or_lost_text() {
    let source = "alpha **beta** gamma [delta](test:line) 世界 🙂 ".repeat(8);
    let expected = "alpha beta gamma delta 世界 🙂 ".repeat(8);
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| Reflow {
        text: cx.new(|cx| {
            let mut text = TextViewState::externally_prepared(cx);
            text.set_prepared(
                PreparedText::parse(&source, MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            );
            text
        }),
        width: 640.,
    });
    cx.simulate_a11y_active(true);
    let mut counts = Vec::new();
    for width in [640., 180., 640.] {
        view.update(cx, |view, cx| {
            view.width = width;
            cx.notify();
        });
        draw(cx);
        let tree = cx.a11y_tree().unwrap();
        let nodes: std::collections::HashMap<_, _> =
            tree.nodes.iter().map(|(id, node)| (*id, node)).collect();
        let document = tree
            .nodes
            .iter()
            .find(|(_, n)| n.role() == accesskit::Role::Document)
            .unwrap()
            .0;
        let mut pending = vec![document];
        let mut lines = Vec::new();
        while let Some(id) = pending.pop() {
            let node = nodes[&id];
            if node.role() == accesskit::Role::TextRun {
                // Every run must belong to a bidirectional chain of current IDs.
                let line = line_text(&tree, id);
                if node.previous_on_line().is_none() {
                    lines.push(line);
                }
            }
            pending.extend(node.children().iter().rev());
        }
        assert_eq!(lines.concat().trim_end(), expected.trim_end());
        counts.push(lines.len());
    }
    assert!(counts[1] > counts[0], "narrow layout must wrap: {counts:?}");
    assert_eq!(counts[0], counts[2], "return to the original line topology");
}

struct LineScope {
    checks: Arc<AtomicUsize>,
}

impl Render for LineScope {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let foreign = Rc::new(std::cell::Cell::new(None));
        let sibling = foreign.clone();
        let checks = self.checks.clone();
        let text_run = || {
            let mut node = accesskit::Node::new(accesskit::Role::TextRun);
            node.set_value("x");
            node.set_character_lengths(vec![1]);
            node
        };
        div()
            .size_full()
            .child(
                div()
                    .id("foreign-line-owner")
                    .role(accesskit::Role::Group)
                    .a11y_synthetic_children(move |builder| {
                        let id = builder.synthetic_node_id("foreign-run");
                        assert!(builder.push_child(id, text_run()));
                        sibling.set(Some(id));
                    }),
            )
            .child(
                div()
                    .id("line-owner")
                    .role(accesskit::Role::Group)
                    .a11y_synthetic_children(move |builder| {
                        let a = builder.synthetic_node_id("a");
                        let b = builder.synthetic_node_id("b");
                        let control = builder.synthetic_node_id("control");
                        assert!(builder.push_child(a, text_run()));
                        assert!(builder.push_child(b, text_run()));
                        assert!(
                            builder
                                .push_child(control, accesskit::Node::new(accesskit::Role::Button))
                        );
                        let snapshot = |builder: &gpui::A11ySubtreeBuilder<'_>| {
                            let mut links = Vec::new();
                            builder.visit_descendants(|id, node| {
                                links.push((id, node.previous_on_line(), node.next_on_line()))
                            });
                            links.sort();
                            links
                        };
                        assert!(builder.set_text_run_lines(&[vec![a, b]]));
                        let linked = snapshot(builder);
                        assert!(linked.contains(&(a, None, Some(b))));
                        assert!(linked.contains(&(b, Some(a), None)));
                        for invalid in [
                            vec![vec![a, b], vec![a]],
                            vec![vec![a, b, foreign.get().unwrap()]],
                            vec![vec![a, b, control]],
                            vec![vec![a]],
                            vec![vec![]],
                        ] {
                            assert!(!builder.set_text_run_lines(&invalid));
                            assert_eq!(snapshot(builder), linked, "rejection must be atomic");
                        }
                        assert!(builder.set_text_run_lines(&[vec![a], vec![b]]));
                        assert!(
                            snapshot(builder)
                                .iter()
                                .all(|(_, before, after)| before.is_none() && after.is_none())
                        );
                        checks.fetch_add(1, Ordering::Relaxed);
                    }),
            )
    }
}

#[test]
fn rendered_visual_line_builder_preserves_scope_and_rejects_partial_edits() {
    let mut app = TestAppContext::single();
    let checks = Arc::new(AtomicUsize::new(0));
    let (_, cx) = app.add_window_view(|_, _| LineScope {
        checks: checks.clone(),
    });
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    assert!(checks.load(Ordering::Relaxed) > 0);
}
