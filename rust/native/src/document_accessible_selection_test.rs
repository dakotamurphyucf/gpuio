//! Same-frame publication is distinct from authorizing OS selection mutation.
use super::*;

#[test]
fn literal_and_enabled_mdx_tags_have_one_reading_representation() {
    let source = "<Note>MDX keeps **useful** child content; expressions stay text: {1 + 2}.</Note>";
    assert_realized_with_extensions(
        source,
        crate::document_markdown::extensions(Default::default()),
    );
    assert_realized_with_extensions(
        source,
        crate::document_markdown::extensions_with_options(
            Default::default(),
            gpuio_protocol::document::MarkdownOptions {
                mdx: true,
                ..Default::default()
            },
        ),
    );
}

struct SelectedDocument(Entity<TextViewState>);
impl Render for SelectedDocument {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(gpui_base::TextSelectionLayer)
            .child(TextView::new(&self.0))
    }
}

#[test]
fn selected_terminal_code_separator_preserves_existing_whole_copy_contract() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| {
        SelectedDocument(cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("```txt\ncode\n```", MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            );
            state
        }))
    });
    let text = view.read_with(cx, |view, _| view.0.clone());
    cx.simulate_a11y_active(true);
    draw(cx);
    text.update(cx, |state, cx| state.select_all(cx));
    draw(cx);
    assert_complete_text(&text, cx);
    let tree = cx.a11y_tree().unwrap();
    let selection = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == accesskit::Role::Document)
        .unwrap()
        .1
        .text_selection()
        .expect("Select All publication");
    cx.update(|window, cx| {
        assert_eq!(gpui_base::TextSelection::selected_text(window, cx), "code");
        let state = text.read(cx);
        let projection = state.rendered_text().unwrap();
        assert_eq!(state.selected_text(), "code\n");
        assert_eq!(projection.text(), "code\n");
        let anchor = state
            .rendered_accessible_position(window, selection.anchor)
            .unwrap();
        let focus = state
            .rendered_accessible_position(window, selection.focus)
            .unwrap();
        let selected = projection.selection(&anchor, &focus).unwrap();
        assert_eq!(projection.selected_text(&selected), Some("code\n"));
    });
}

struct SelectionClaim {
    mode: usize,
}

impl Render for SelectionClaim {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mode = self.mode;
        let claim = Arc::new(std::sync::Mutex::new(None));
        let published = claim.clone();
        div()
            .id("selection-document")
            .aria_label("Selection fixture")
            .role(accesskit::Role::Document)
            .size_full()
            .a11y_synthetic_children(move |builder| {
                let run_id = builder.synthetic_node_id("run");
                let mut run = accesskit::Node::new(accesskit::Role::TextRun);
                run.set_value("aλ");
                run.set_character_lengths(vec![1, 2]);
                assert!(builder.push_child(run_id, run));
                if (2..=5).contains(&mode) || mode == 10 {
                    let role = match mode {
                        2 => accesskit::Role::Document,
                        3 => accesskit::Role::TextInput,
                        10 => accesskit::Role::Terminal,
                        _ => accesskit::Role::Group,
                    };
                    let mut scope = accesskit::Node::new(role);
                    if mode == 4 {
                        scope.set_hidden();
                    }
                    if mode == 5 {
                        scope.set_disabled();
                    }
                    builder.group_children("scope", scope, &[run_id]).unwrap();
                }
                if mode == 7 {
                    builder.parent_node().set_disabled();
                }
                if mode == 9 {
                    builder.parent_node().set_hidden();
                }
                let position = |character_index| accesskit::TextPosition {
                    node: if mode == 6 {
                        accesskit::NodeId(u64::MAX)
                    } else {
                        run_id
                    },
                    character_index,
                };
                *claim.lock().unwrap() = Some((
                    builder.parent_id(),
                    accesskit::TextSelection {
                        anchor: position(if mode == 1 { 3 } else { 2 }),
                        focus: position(0),
                    },
                ));
            })
            .child(
                gpui::canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        if !window.is_a11y_active() {
                            return;
                        }
                        let (id, selection) = (*published.lock().unwrap()).unwrap();
                        assert!(
                            window.publish_document_selection(id, (mode != 8).then_some(selection))
                        );
                    },
                )
                .size_full(),
            )
    }
}

#[test]
fn final_tree_selection_rejects_foreign_nested_hidden_and_invalid_positions() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, _| SelectionClaim { mode: 0 });
    cx.simulate_a11y_active(true);
    for mode in [0, 1, 0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0] {
        view.update(cx, |view, cx| {
            view.mode = mode;
            cx.notify();
        });
        draw(cx);
        let tree = cx.a11y_tree().unwrap();
        let (id, document) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Selection fixture"))
            .unwrap();
        if mode == 0 {
            let selection = document
                .text_selection()
                .expect("published in the same frame");
            assert_eq!(selection.anchor.character_index, 2);
            assert_eq!(selection.focus.character_index, 0);
        } else {
            assert!(
                tree.nodes.iter().all(|(_, n)| n.text_selection().is_none()),
                "mode {mode}"
            );
        }
        cx.update(|window, _| assert!(!window.publish_document_selection(*id, None)));
    }
}

#[test]
fn native_document_selection_matches_prepared_direction_in_the_painted_frame() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| Documents {
        text: cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("first λ🙂 last", MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            );
            state
        }),
        scrollable: false,
        max_lines: None,
        clicks: Arc::default(),
        extensions: MarkdownExtensions::default(),
    });
    let text = view.read_with(cx, |view, _| view.text.clone());
    cx.simulate_a11y_active(true);
    draw(cx);
    for (anchor, head) in [(6, 13), (13, 6), (6, 6)] {
        let request = text.read_with(cx, |state, _| {
            let projection = state.rendered_text().unwrap();
            state
                .prepare_rendered_selection(
                    &projection.position(anchor).unwrap(),
                    &projection.position(head).unwrap(),
                )
                .unwrap()
        });
        text.update(cx, |state, cx| state.apply_rendered_selection(request, cx))
            .unwrap();
        draw(cx);
        let tree = cx.a11y_tree().unwrap();
        let selection = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == accesskit::Role::Document)
            .unwrap()
            .1
            .text_selection()
            .expect("current native selection");
        cx.update(|window, cx| {
            let state = text.read(cx);
            let expected = state.rendered_selection().unwrap();
            assert_eq!(
                state
                    .rendered_accessible_position(window, selection.anchor)
                    .unwrap()
                    .content_position(),
                expected.anchor().content_position()
            );
            assert_eq!(
                state
                    .rendered_accessible_position(window, selection.focus)
                    .unwrap()
                    .content_position(),
                expected.head().content_position()
            );
        });
    }
    text.update(cx, |state, cx| {
        state.set_prepared(
            PreparedText::parse("replacement", MarkdownExtensions::default()).unwrap(),
            None,
            cx,
        );
    });
    draw(cx);
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .all(|(_, node)| node.text_selection().is_none())
    );
}
