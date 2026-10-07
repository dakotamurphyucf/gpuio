//! Native request/Copy state; OS authorization and AX publication are separate.
use gpui::{AppContext, Entity, TestAppContext};
use gpui_base::text::{
    MarkdownExtensions, MarkdownNode, MarkdownPlugin, MarkdownPresentation, PreparedText,
    RenderedSelectionError as Error, RenderedSelectionRequest, SelectionFormat, TextViewState,
};

fn mount(prepared: PreparedText) -> (TestAppContext, Entity<TextViewState>) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let state = app.new(|cx| TextViewState::externally_prepared(cx).selectable(true));
    state.update(&mut app, |state, cx| state.set_prepared(prepared, None, cx));
    (app, state)
}
fn parse(source: &str) -> PreparedText {
    PreparedText::parse(source, MarkdownExtensions::default()).unwrap()
}
fn request(
    state: &Entity<TextViewState>,
    app: &TestAppContext,
    anchor: usize,
    head: usize,
) -> RenderedSelectionRequest {
    state.read_with(app, |state, _| {
        let text = state.rendered_text().unwrap();
        state
            .prepare_rendered_selection(
                &text.position(anchor).unwrap(),
                &text.position(head).unwrap(),
            )
            .unwrap()
    })
}
fn apply(state: &Entity<TextViewState>, app: &mut TestAppContext, anchor: usize, head: usize) {
    let request = request(state, app, anchor, head);
    state
        .update(app, |state, cx| state.apply_rendered_selection(request, cx))
        .unwrap();
}
fn copy(state: &Entity<TextViewState>, app: &TestAppContext) -> String {
    state.read_with(app, |state, _| state.selected_text())
}

#[test]
fn rendered_request_keeps_direction_and_native_source_copy_before_layout() {
    let (mut app, state) = mount(parse("Before **世界** and 世界."));
    let start = "Before ".len();
    let end = start + "世界".len();
    apply(&state, &mut app, end, start);
    assert_eq!(copy(&state, &app), "世界");
    state.read_with(&app, |state, _| {
        let selection = state.requested_rendered_selection().unwrap();
        assert!(selection.is_backward());
        assert_eq!(selection.bytes(), start..end);
        assert_eq!(
            state.rendered_text().unwrap().selected_fragment_ranges(),
            vec![start..end]
        );
    });
    let queued = request(&state, &app, end, start);
    state.update(&mut app, |state, cx| {
        state.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), "**世界**");
    state
        .update(&mut app, |state, cx| {
            state.apply_rendered_selection(queued, cx)
        })
        .unwrap();
    assert_eq!(copy(&state, &app), "**世界**");
    state.update(&mut app, |state, cx| {
        state.set_selection_format(SelectionFormat::Plain, cx)
    });
    let second = "Before 世界 and ".len();
    apply(&state, &mut app, second, second + "世界".len());
    assert_eq!(copy(&state, &app), "世界");
    state.update(&mut app, |state, cx| {
        state.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(
        copy(&state, &app),
        "世界",
        "equal text must retain its actual owner"
    );
}

#[test]
fn rendered_requests_reject_other_views_epochs_and_replacements_without_mutation() {
    let (mut app, state) = mount(parse("First **世界** last"));
    let other = app.new(|cx| TextViewState::externally_prepared(cx).selectable(true));
    other.update(&mut app, |s, cx| {
        s.set_prepared(parse("First **世界** last"), None, cx)
    });
    let queued = request(&state, &app, 0, 5);
    assert_eq!(
        other.update(&mut app, |s, cx| s
            .apply_rendered_selection(queued.clone(), cx)),
        Err(Error::StaleRequest)
    );
    state
        .update(&mut app, |s, cx| {
            s.apply_rendered_selection(queued.clone(), cx)
        })
        .unwrap();
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    assert_eq!(copy(&state, &app), "First");
    let old_text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let queued = request(&state, &app, 0, 5);
    state.update(&mut app, |s, cx| {
        s.set_prepared(parse("First **世界** last"), None, cx)
    });
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    assert_eq!(copy(&state, &app), "");
    assert_eq!(
        state.read_with(&app, |s, _| s
            .prepare_rendered_selection(
                &old_text.position(0).unwrap(),
                &old_text.position(5).unwrap()
            )
            .err()),
        Some(Error::ForeignPosition)
    );
    let queued = request(&state, &app, 0, 5);
    state.update(&mut app, |s, cx| s.select_all(cx));
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    assert!(copy(&state, &app).starts_with("First 世界 last"));
    let queued = request(&state, &app, 0, 5);
    state.update(&mut app, |s, cx| s.set_selectable(false, cx));
    assert_eq!(
        state.update(&mut app, |s, cx| s
            .apply_rendered_selection(queued.clone(), cx)),
        Err(Error::NotSelectable)
    );
    state.update(&mut app, |s, cx| s.set_selectable(true, cx));
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    assert_eq!(copy(&state, &app), "");
}

struct Atomic;
impl MarkdownPlugin for Atomic {
    fn name(&self) -> &str {
        "atomic"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::InlineCode(_)).then(|| {
            MarkdownNode::new("atomic", ())
                .text("World\r\n世界")
                .markdown("`object`")
        })
    }
}

#[test]
fn rendered_requests_select_atomic_alternatives_whole_and_reject_inner_endpoints() {
    let (mut app, state) = mount(
        PreparedText::parse(
            "Before `object` after",
            MarkdownExtensions::default().plugin(Atomic),
        )
        .unwrap(),
    );
    apply(&state, &mut app, 0, 6);
    let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let part = text.parts().iter().find(|p| p.is_atomic()).unwrap().bytes();
    let invalid = state.read_with(&app, |s, _| {
        s.prepare_rendered_selection(
            &text.position(part.start).unwrap(),
            &text.position(part.start + 1).unwrap(),
        )
    });
    assert_eq!(invalid.err(), Some(Error::AtomicBoundary));
    assert_eq!(copy(&state, &app), "Before");
    apply(&state, &mut app, part.end, part.start);
    assert_eq!(copy(&state, &app), "World\r\n世界");
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), "`object`");
}

#[test]
fn rendered_requests_cover_unpainted_virtual_blocks_and_collapse_exactly() {
    let source = (0..200)
        .map(|i| format!("Paragraph {i} **世界**\n\n"))
        .collect::<String>();
    let (mut app, state) = mount(parse(&source));
    let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let start = text.text().find("Paragraph 2 ").unwrap() + 3;
    let end = text.text().find("Paragraph 198 ").unwrap() + "Paragraph 198 世界".len();
    apply(&state, &mut app, end, start);
    assert_eq!(copy(&state, &app), &text.text()[start..end]);
    assert_eq!(text.selected_fragment_ranges().len(), 197);
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    let source_selection = copy(&state, &app);
    assert!(source_selection.starts_with("agraph 2 **世界**"));
    assert!(source_selection.ends_with("Paragraph 198 **世界**"));
    assert!(!source_selection.contains("Paragraph 199"));
    apply(&state, &mut app, start, start);
    assert_eq!(copy(&state, &app), "");
    assert!(text.selected_fragment_ranges().is_empty());
    state.read_with(&app, |s, _| {
        assert!(!s.has_local_selection());
        assert_eq!(
            s.requested_rendered_selection().unwrap().bytes(),
            start..start
        );
    });
}

#[test]
fn rendered_request_preserves_compatible_streamed_selection_but_retires_queued_requests() {
    let source = "Before **世界** after";
    let (mut app, state) = mount(parse(source));
    apply(&state, &mut app, 13, 7);
    let queued = request(&state, &app, 0, 6);
    let previous = state.read_with(&app, |s, _| {
        s.requested_rendered_selection().unwrap().clone()
    });
    state.update(&mut app, |s, cx| {
        s.set_prepared(
            parse(&format!("{source} and more\n\nTail")),
            Some(source.len()),
            cx,
        )
    });
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    assert_eq!(copy(&state, &app), "世界");
    state.read_with(&app, |s, _| {
        let new = s.rendered_text().unwrap();
        let selected = s.requested_rendered_selection().unwrap();
        assert!(selected.is_backward());
        assert_eq!(new.offset(selected.anchor()), Some(13));
        assert!(new.offset(previous.anchor()).is_none());
    });
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), "**世界**");
}

#[test]
fn streamed_select_all_keeps_old_glyphs_and_separator_ownership() {
    for (source, suffix, expected) in [
        ("Before **世界** after", " and more", "Before 世界 after"),
        ("Before **世界** after", "\n\nTail", "Before 世界 after\n"),
        ("> quoted", " more", "quoted"),
        ("```txt\nold", "\nnew", "old"),
        ("```txt\nold\n\n", "new", "old\n"),
    ] {
        for backward in [false, true] {
            let (mut app, state) = mount(parse(source));
            if backward {
                let len = state.read_with(&app, |s, _| s.rendered_text().unwrap().text().len());
                apply(&state, &mut app, len, 0);
            } else {
                state.update(&mut app, |s, cx| s.select_all(cx));
            }
            let queued = request(&state, &app, 0, 0);
            state.update(&mut app, |s, cx| {
                s.set_prepared(parse(&format!("{source}{suffix}")), Some(source.len()), cx)
            });
            state.read_with(&app, |s, _| {
                let selection = s.rendered_selection().expect("streamed Select All range");
                assert_eq!(
                    selection.bytes(),
                    0..expected.len(),
                    "source={source:?} suffix={suffix:?}"
                );
                assert_eq!(selection.is_backward(), backward);
                assert_eq!(
                    s.rendered_text().unwrap().selected_text(&selection),
                    Some(expected)
                );
                assert_eq!(s.requested_rendered_selection().is_some(), backward);
            });
            assert_eq!(copy(&state, &app), expected);
            assert_eq!(
                state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
                Err(Error::StaleRequest)
            );
            state.update(&mut app, |s, cx| {
                s.set_selection_format(SelectionFormat::Source, cx)
            });
            assert_eq!(copy(&state, &app), source);
        }
    }
}

#[test]
fn streamed_structural_separator_collapses_without_selecting_new_text() {
    let (mut app, state) = mount(parse("Word"));
    apply(&state, &mut app, 5, 4);
    assert_eq!(copy(&state, &app), "\n");
    state.update(&mut app, |s, cx| {
        s.set_prepared(parse("Word more"), Some(4), cx)
    });
    state.read_with(&app, |s, _| {
        assert_eq!(s.rendered_selection().unwrap().bytes(), 4..4);
        assert!(!s.has_local_selection());
        assert_eq!(s.selected_text(), "");
    });
    let (mut app, state) = mount(parse(""));
    state.update(&mut app, |s, cx| s.select_all(cx));
    state.update(&mut app, |s, cx| s.set_prepared(parse("new"), Some(0), cx));
    state.read_with(&app, |s, _| {
        assert_eq!(s.rendered_selection().unwrap().bytes(), 0..0);
        assert!(!s.has_local_selection());
        assert_eq!(s.selected_text(), "");
    });
}

struct DifferentGlyphs(bool);
impl MarkdownPlugin for DifferentGlyphs {
    fn name(&self) -> &str {
        "block"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::Blockquote(_))
            .then(|| MarkdownNode::new("block", ()).text("copy alternative"))
    }
    fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
        MarkdownPresentation::Text(if self.0 { "" } else { "actual glyphs" }.into())
    }
}

#[test]
fn rendered_request_owner_mapping_failure_leaves_previous_selection_intact() {
    let prepared = PreparedText::parse(
        "Before\n\n> custom block\n\nAfter",
        MarkdownExtensions::default().plugin(DifferentGlyphs(false)),
    )
    .unwrap();
    let (mut app, state) = mount(prepared);
    let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let start = text.text().find("After").unwrap();
    apply(&state, &mut app, start, start + 5);
    let queued = request(&state, &app, 0, start);
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::UnmappedOwner)
    );
    assert_eq!(copy(&state, &app), "After");
    assert_eq!(text.selected_fragment_ranges(), vec![start..start + 5]);
}

#[test]
fn rendered_request_empty_and_collapsed_ranges_do_not_activate_copy() {
    let (mut app, state) = mount(parse(""));
    apply(&state, &mut app, 0, 0);
    state.read_with(&app, |s, _| {
        assert!(!s.has_local_selection());
        assert_eq!(s.selected_text(), "");
        assert_eq!(s.requested_rendered_selection().unwrap().bytes(), 0..0);
    });
    let queued = request(&state, &app, 0, 0);
    state.update(&mut app, |s, cx| s.clear_selection(cx));
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
}

#[test]
fn rendered_request_distinguishes_empty_declared_glyphs_from_unpainted_text() {
    let prepared = PreparedText::parse(
        "Before\n\n> custom block\n\nAfter",
        MarkdownExtensions::default().plugin(DifferentGlyphs(true)),
    )
    .unwrap();
    let (mut app, state) = mount(prepared);
    apply(&state, &mut app, 0, 6);
    let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let start = text.text().find("copy alternative").unwrap();
    let queued = request(&state, &app, start, start + "copy alternative".len());
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::UnmappedOwner)
    );
    assert_eq!(copy(&state, &app), "Before");
}

struct ResourceGlyphs(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl MarkdownPlugin for ResourceGlyphs {
    fn name(&self) -> &str {
        "resource"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::Blockquote(_))
            .then(|| MarkdownNode::new("resource", ()).text("copy alternative"))
    }
    fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
        MarkdownPresentation::Text(
            if self.0.load(std::sync::atomic::Ordering::Relaxed) {
                "changed glyphs"
            } else {
                "copy alternative"
            }
            .into(),
        )
    }
}

#[test]
fn rendered_request_renderer_refresh_expires_requests_and_preserves_only_compatible_selection() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let changed = Arc::new(AtomicBool::new(false));
    let extensions = MarkdownExtensions::default().plugin(ResourceGlyphs(changed.clone()));
    let (mut app, state) =
        mount(PreparedText::parse("Before\n\n> object\n\nAfter", extensions.clone()).unwrap());
    apply(&state, &mut app, 6, 0);
    let old = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let queued = request(&state, &app, 0, 5);
    let updated = extensions.block_renderer("resource", |_, _, _| gpui::div());
    state.update(&mut app, |s, cx| {
        s.set_markdown_extensions(Arc::new(updated.clone()), cx)
    });
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    assert_eq!(copy(&state, &app), "Before");
    let text = state.read_with(&app, |s, _| {
        assert!(s.requested_rendered_selection().unwrap().is_backward());
        s.rendered_text().unwrap()
    });
    assert!(!Arc::ptr_eq(&old, &text));
    let start = text.text().find("copy alternative").unwrap();
    apply(&state, &mut app, start, start + "copy alternative".len());
    let queued = request(&state, &app, 0, 5);
    changed.store(true, Ordering::Relaxed);
    let updated = updated.block_renderer("resource", |_, _, _| gpui::div());
    state.update(&mut app, |s, cx| {
        s.set_markdown_extensions(Arc::new(updated), cx)
    });
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    assert_eq!(copy(&state, &app), "");
    assert!(state.read_with(&app, |s, _| s.requested_rendered_selection().is_none()));
    let queued = request(&state, &app, start, start + "copy alternative".len());
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::UnmappedOwner)
    );
}
