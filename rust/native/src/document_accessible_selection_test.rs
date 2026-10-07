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
                        assert!(!window.accepts_document_selection(id, &selection));
                        if mode == 11 {
                            return;
                        }
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
fn completed_document_scopes_validate_endpoints_without_requiring_a_selection() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, _| SelectionClaim { mode: 0 });
    cx.simulate_a11y_active(true);
    // The cleared-selection case (8) must still authorize a new selection.
    // An unclaimed Document (11) must not inherit last frame's authorization.
    for mode in [0, 8, 11, 0, 2, 3, 4, 5, 7, 9, 10, 0] {
        view.update(cx, |view, cx| {
            view.mode = mode;
            cx.notify();
        });
        draw(cx);
        let tree = cx.a11y_tree().unwrap();
        let document = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Selection fixture"))
            .unwrap()
            .0;
        let run = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == accesskit::Role::TextRun)
            .unwrap()
            .0;
        cx.update(|window, _| {
            for (anchor, focus) in [(0, 2), (2, 0), (1, 1)] {
                let selection = accesskit::TextSelection {
                    anchor: accesskit::TextPosition {
                        node: run,
                        character_index: anchor,
                    },
                    focus: accesskit::TextPosition {
                        node: run,
                        character_index: focus,
                    },
                };
                assert_eq!(
                    window.accepts_document_selection(document, &selection),
                    mode == 0 || mode == 8,
                    "mode {mode}"
                );
                assert!(!window.accepts_document_selection(run, &selection));
                for bad in [
                    accesskit::TextPosition {
                        node: run,
                        character_index: 3,
                    },
                    accesskit::TextPosition {
                        node: accesskit::NodeId(u64::MAX),
                        character_index: 0,
                    },
                ] {
                    assert!(!window.accepts_document_selection(
                        document,
                        &accesskit::TextSelection {
                            anchor: bad,
                            ..selection
                        }
                    ));
                    assert!(!window.accepts_document_selection(
                        document,
                        &accesskit::TextSelection {
                            focus: bad,
                            ..selection
                        }
                    ));
                }
            }
        });
    }
    let tree = cx.a11y_tree().unwrap();
    let (document, node) = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Selection fixture"))
        .unwrap();
    let selection = node.text_selection().unwrap();
    cx.simulate_a11y_active(false);
    cx.update(|window, _| assert!(!window.accepts_document_selection(*document, selection)));
    draw(cx);
    cx.update(|window, _| assert!(!window.accepts_document_selection(*document, selection)));
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

struct GuardedDocuments {
    first: Entity<TextViewState>,
    second: Entity<TextViewState>,
    allowed: Arc<std::sync::atomic::AtomicBool>,
    mounted: bool,
}
impl Render for GuardedDocuments {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let allowed = self.allowed.clone();
        div()
            .size_full()
            .child(gpui_base::TextSelectionLayer)
            .when(self.mounted, |view| {
                view.child(
                    TextView::new(&self.first)
                        .link_focus_guard(move |_| allowed.load(Ordering::Relaxed)),
                )
            })
            .child(TextView::new(&self.second))
    }
}

fn native_range(
    text: &Entity<TextViewState>,
    cx: &mut VisualTestContext,
    anchor: usize,
    head: usize,
) -> accesskit::TextSelection {
    cx.update(|window, cx| {
        let state = text.read(cx);
        let projection = state.rendered_text().unwrap();
        accesskit::TextSelection {
            anchor: state
                .rendered_accessible_text_position(window, &projection.position(anchor).unwrap())
                .unwrap(),
            focus: state
                .rendered_accessible_text_position(window, &projection.position(head).unwrap())
                .unwrap(),
        }
    })
}

#[test]
fn accessible_request_requires_current_paint_interaction_and_host_authorization() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| {
        let make = |cx: &mut Context<TextViewState>| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("first λ🙂 last", MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            );
            state
        };
        GuardedDocuments {
            first: cx.new(make),
            second: cx.new(make),
            allowed: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            mounted: true,
        }
    });
    let (first, second, allowed) = view.read_with(cx, |view, _| {
        (
            view.first.clone(),
            view.second.clone(),
            view.allowed.clone(),
        )
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    for (anchor, head) in [(6, 12), (12, 6), (6, 6)] {
        let range = native_range(&first, cx, anchor, head);
        cx.update(|window, cx| {
            let state = first.read(cx);
            assert!(
                state
                    .prepare_accessible_selection(&range, window, cx)
                    .is_some()
            );
            assert!(
                second
                    .read(cx)
                    .prepare_accessible_selection(&range, window, cx)
                    .is_none()
            );
        });
        let foreign = native_range(&second, cx, anchor, head);
        cx.update(|window, cx| {
            assert!(
                first
                    .read(cx)
                    .prepare_accessible_selection(
                        &accesskit::TextSelection {
                            focus: foreign.focus,
                            ..range
                        },
                        window,
                        cx
                    )
                    .is_none()
            );
            let request = first
                .read(cx)
                .prepare_accessible_selection(&range, window, cx)
                .unwrap();
            first
                .update(cx, |state, cx| state.apply_rendered_selection(request, cx))
                .unwrap();
            // Applying or clearing invalidates the painted interaction stamp,
            // even when text and run IDs have not changed.
            assert!(
                first
                    .read(cx)
                    .prepare_accessible_selection(&range, window, cx)
                    .is_none()
            );
        });
        draw(cx);
        assert_eq!(
            first.read_with(cx, |state, _| state.selected_text()),
            if anchor == head { "" } else { "λ🙂" }
        );
    }
    let range = native_range(&first, cx, 6, 12);
    cx.update(|window, cx| {
        first.update(cx, |state, cx| state.clear_selection(cx));
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&range, window, cx)
                .is_none()
        )
    });
    draw(cx);
    allowed.store(false, Ordering::Relaxed);
    cx.update(|window, cx| {
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&range, window, cx)
                .is_none()
        )
    });
    allowed.store(true, Ordering::Relaxed);
    cx.update(|window, cx| {
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&range, window, cx)
                .is_some()
        )
    });
    cx.update(|window, cx| {
        first.update(cx, |state, cx| {
            state.set_selectable(false, cx);
            state.set_selectable(true, cx);
        });
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&range, window, cx)
                .is_none()
        )
    });
    draw(cx);
    // Equal text still has a different preparation/owner identity.
    cx.update(|window, cx| {
        first.update(cx, |state, cx| {
            state.set_prepared(
                PreparedText::parse("first λ🙂 last", MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            )
        });
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&range, window, cx)
                .is_none()
        )
    });
    draw(cx);
    let current = native_range(&first, cx, 6, 12);
    cx.update(|window, cx| {
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&current, window, cx)
                .is_some()
        )
    });
    let mut other_app = cx.cx.clone();
    let (_, other_cx) = other_app.add_window_view(|_, _| SelectionClaim { mode: 0 });
    other_cx.simulate_a11y_active(true);
    draw(other_cx);
    other_cx.update(|window, cx| {
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&current, window, cx)
                .is_none()
        );
    });
    view.update(cx, |view, cx| {
        view.mounted = false;
        cx.notify();
    });
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            first
                .read(cx)
                .prepare_accessible_selection(&current, window, cx)
                .is_none()
        )
    });
}

#[test]
fn accessibility_action_replaces_window_selection_and_preserves_a_caret() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| {
        let make = |cx: &mut Context<TextViewState>| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("first λ🙂 last", MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            );
            state
        };
        GuardedDocuments {
            first: cx.new(make),
            second: cx.new(make),
            allowed: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            mounted: true,
        }
    });
    let (first, second, allowed) = view.read_with(cx, |view, _| {
        (
            view.first.clone(),
            view.second.clone(),
            view.allowed.clone(),
        )
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    first.update(cx, |state, cx| state.select_all(cx));
    second.update(cx, |state, cx| state.select_all(cx));
    draw(cx);
    let range = native_range(&first, cx, 6, 12);
    let document = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| {
            node.role() == accesskit::Role::Document
                && node
                    .text_selection()
                    .is_some_and(|s| s.anchor.node == range.anchor.node)
        })
        .unwrap()
        .0;
    let before = cx.update(gpui_base::TextSelection::selected_text);
    let send = |cx: &mut VisualTestContext, range| {
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::SetTextSelection,
            target_node: document,
            target_tree: accesskit::TreeId::ROOT,
            data: Some(accesskit::ActionData::SetTextSelection(range)),
        });
        draw(cx);
    };
    let mut invalid = range;
    invalid.focus.character_index = usize::MAX;
    send(cx, invalid);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), before);
    allowed.store(false, Ordering::Relaxed);
    send(cx, range);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), before);
    allowed.store(true, Ordering::Relaxed);
    send(cx, range);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), "λ🙂");
    assert_eq!(second.read_with(cx, |state, _| state.selected_text()), "");
    cx.update(|window, cx| assert!(first.read(cx).focus_handle().is_focused(window)));
    let backwards = native_range(&first, cx, 12, 6);
    send(cx, backwards);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), "λ🙂");
    let caret = native_range(&first, cx, 6, 6);
    send(cx, caret);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), "");
    assert!(first.read_with(cx, |state, _| {
        state.rendered_selection().unwrap().is_collapsed()
    }));
    draw(cx);
    assert!(first.read_with(cx, |state, _| {
        state.rendered_selection().unwrap().is_collapsed()
    }));
    cx.update(gpui_base::TextSelection::clear);
    draw(cx);
    assert!(first.read_with(cx, |state, _| state.rendered_selection().is_none()));
}

struct ScrollableSelectedDocument(Entity<TextViewState>);
impl Render for ScrollableSelectedDocument {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(440.))
            .h(px(200.))
            .child(gpui_base::TextSelectionLayer)
            .child(TextView::new(&self.0).scrollable(true))
    }
}

#[test]
fn accessibility_selection_realizes_and_reveals_offscreen_and_tall_block_heads() {
    for code in [false, true] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let source = if code {
            format!(
                "```txt\n{}target λ🙂 end\n```",
                "line before target\n".repeat(120)
            )
        } else {
            format!(
                "{}target λ🙂 end",
                "paragraph before target\n\n".repeat(120)
            )
        };
        let (view, cx) = app.add_window_view(|_, cx| {
            ScrollableSelectedDocument(cx.new(|cx| {
                let mut state = TextViewState::externally_prepared(cx);
                state.set_prepared(
                    PreparedText::parse(&source, MarkdownExtensions::default()).unwrap(),
                    None,
                    cx,
                );
                state
            }))
        });
        let text = view.read_with(cx, |view, _| view.0.clone());
        cx.simulate_a11y_active(true);
        draw(cx);
        let (start, end) = text.read_with(cx, |state, _| {
            let projection = state.rendered_text().unwrap();
            let start = projection.text().rfind("target λ🙂 end").unwrap();
            (start, projection.text().len())
        });
        let range = native_range(&text, cx, start, end);
        let document = cx
            .a11y_tree()
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, node)| node.role() == accesskit::Role::Document)
            .unwrap()
            .0;
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::SetTextSelection,
            target_node: document,
            target_tree: accesskit::TreeId::ROOT,
            data: Some(accesskit::ActionData::SetTextSelection(range)),
        });
        draw(cx);
        assert_eq!(
            cx.update(gpui_base::TextSelection::selected_text),
            "target λ🙂 end",
            "code={code}"
        );
        let current = native_range(&text, cx, start, start + "target λ🙂 end".len() - 1);
        let tree = cx.a11y_tree().unwrap();
        let node = &tree
            .nodes
            .iter()
            .find(|(id, _)| *id == current.focus.node)
            .unwrap()
            .1;
        let bounds = node.bounds().unwrap_or_else(|| {
            panic!(
                "code={code}: last selected glyph has no bounds: {:?}",
                node.value()
            )
        });
        let scale = cx.update(|window, _| f64::from(window.scale_factor()));
        assert!(
            bounds.y0 >= 0. && bounds.y1 <= 200. * scale + 0.1,
            "code={code}: {bounds:?}, scale={scale}"
        );
        cx.update(|window, cx| assert!(text.read(cx).focus_handle().is_focused(window)));
    }
}

#[test]
fn queued_window_clear_cannot_erase_new_native_caret() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| {
        SelectedDocument(cx.new(|cx| {
            let mut state = TextViewState::externally_prepared(cx);
            state.set_prepared(
                PreparedText::parse("abc", MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            );
            state
        }))
    });
    let text = view.read_with(cx, |view, _| view.0.clone());
    draw(cx);
    text.update(cx, |state, cx| state.select_all(cx));
    cx.update(|window, cx| {
        gpui_base::TextSelection::clear(window, cx);
        text.update(cx, |state, cx| {
            let projection = state.rendered_text().unwrap();
            let caret = projection.position(1).unwrap();
            let request = state.prepare_rendered_selection(&caret, &caret).unwrap();
            state.apply_rendered_selection(request, cx).unwrap();
        });
    });
    draw(cx);
    assert!(text.read_with(cx, |state, _| {
        state.rendered_selection().unwrap().is_collapsed()
    }));
    cx.update(gpui_base::TextSelection::clear);
    draw(cx);
    assert!(text.read_with(cx, |state, _| state.rendered_selection().is_none()));
}
