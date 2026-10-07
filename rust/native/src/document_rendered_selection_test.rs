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
            let frozen_copy = copy(&state, &app);
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
            // Logical paint edges exclude newly owned appended bytes, while
            // whole Copy retains the original structural separator.
            assert_eq!(copy(&state, &app), frozen_copy);
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
        matches!(node, gpui_base::text::markdown_ast::Node::Blockquote(_)).then(|| {
            MarkdownNode::new("block", ())
                .text("copy alternative")
                .markdown("> custom block")
        })
    }
    fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
        MarkdownPresentation::Text(if self.0 { "" } else { "actual glyphs" }.into())
    }
}

#[test]
fn declared_block_text_selection_addresses_glyphs_and_preserves_whole_copy_contract() {
    let source = "Before\n\n> custom block\n\nAfter";
    let extensions = MarkdownExtensions::default().plugin(DifferentGlyphs(false));
    let prepared = PreparedText::parse(source, extensions.clone()).unwrap();
    assert_eq!(prepared.plain_text(), "Before\ncopy alternative\nAfter\n");
    assert_eq!(
        prepared.rendered_text().text(),
        "Before\nactual glyphs\nAfter\n"
    );
    let (mut app, state) = mount(prepared);
    let start = "Before\nactual ".len();
    apply(&state, &mut app, start + 6, start);
    assert_eq!(copy(&state, &app), "glyphs");
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), "glyphs");
    apply(&state, &mut app, 7, 20);
    assert_eq!(copy(&state, &app), "> custom block");
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Plain, cx)
    });
    assert_eq!(copy(&state, &app), "actual glyphs");
    state.update(&mut app, |s, cx| s.select_all(cx));
    assert_eq!(copy(&state, &app), "Before\ncopy alternative\nAfter\n");
    state.read_with(&app, |s, _| {
        let range = s.rendered_selection().unwrap();
        assert_eq!(
            s.rendered_text().unwrap().selected_text(&range),
            Some("Before\nactual glyphs\nAfter\n")
        );
    });
    for suffix in [" appended", " appended again"] {
        state.update(&mut app, |s, cx| {
            s.set_prepared(
                PreparedText::parse(&format!("{source}{suffix}"), extensions.clone()).unwrap(),
                Some(source.len()),
                cx,
            )
        });
        assert_eq!(copy(&state, &app), "Before\ncopy alternative\nAfter\n");
        state.read_with(&app, |s, _| {
            let range = s.rendered_selection().unwrap();
            assert_eq!(
                s.rendered_text().unwrap().selected_text(&range),
                Some("Before\nactual glyphs\nAfter")
            );
        });
    }
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), source);
    // Explicit selection after frozen All leaves its old Copy snapshot behind.
    apply(&state, &mut app, start + 6, start);
    assert_eq!(copy(&state, &app), "glyphs");
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Plain, cx)
    });
    assert_eq!(copy(&state, &app), "glyphs");
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
fn empty_declared_block_glyphs_do_not_fabricate_selectable_alternative_characters() {
    let prepared = PreparedText::parse(
        "Before\n\n> custom block\n\nAfter",
        MarkdownExtensions::default().plugin(DifferentGlyphs(true)),
    )
    .unwrap();
    assert_eq!(prepared.rendered_text().text(), "Before\nAfter\n");
    assert_eq!(prepared.plain_text(), "Before\ncopy alternative\nAfter\n");
    let (mut app, state) = mount(prepared);
    apply(&state, &mut app, 7, 12);
    assert_eq!(copy(&state, &app), "After");
    apply(&state, &mut app, 7, 7);
    assert_eq!(copy(&state, &app), "");
    state.update(&mut app, |s, cx| s.select_all(cx));
    assert_eq!(copy(&state, &app), "Before\ncopy alternative\nAfter\n");
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
    let current = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let start = current.text().find("changed glyphs").unwrap();
    apply(&state, &mut app, start, start + "changed glyphs".len());
    assert_eq!(copy(&state, &app), "changed glyphs");
}

struct EmptyGlyphLargeCopy(usize);
impl MarkdownPlugin for EmptyGlyphLargeCopy {
    fn name(&self) -> &str {
        "large-copy"
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
            .then(|| MarkdownNode::new("large-copy", ()).text("x".repeat(self.0)))
    }
    fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
        MarkdownPresentation::Text("".into())
    }
}

#[test]
fn custom_glyph_projection_keeps_independent_copy_admission_bounds() {
    let max = gpui_base::text::RenderedText::max_text_bytes();
    let accepted = PreparedText::parse(
        "> custom",
        MarkdownExtensions::default().plugin(EmptyGlyphLargeCopy(max - 1)),
    )
    .unwrap();
    assert_eq!(accepted.plain_text().len(), max);
    assert_eq!(accepted.rendered_text().text(), "");
    assert!(
        PreparedText::parse(
            "> custom",
            MarkdownExtensions::default().plugin(EmptyGlyphLargeCopy(max))
        )
        .is_err()
    );
    assert!(
        PreparedText::parse(
            "> first\n\n> second",
            MarkdownExtensions::default().plugin(EmptyGlyphLargeCopy(max / 2))
        )
        .is_err()
    );
}

#[test]
fn frozen_whole_copy_keeps_its_scope_when_declared_glyphs_are_empty() {
    let source = "> custom block";
    let extensions = MarkdownExtensions::default().plugin(DifferentGlyphs(true));
    let (mut app, state) = mount(PreparedText::parse(source, extensions.clone()).unwrap());
    state.update(&mut app, |s, cx| s.select_all(cx));
    state.update(&mut app, |s, cx| {
        s.set_prepared(
            PreparedText::parse(&format!("{source}\n\nNew"), extensions).unwrap(),
            Some(source.len()),
            cx,
        )
    });
    assert_eq!(copy(&state, &app), "copy alternative\n");
    state.read_with(&app, |s, _| {
        assert!(s.has_local_selection());
        assert_eq!(s.rendered_selection().unwrap().bytes(), 0..0);
        assert_eq!(s.rendered_text().unwrap().text(), "New\n");
    });
    apply(&state, &mut app, 0, 0);
    assert_eq!(copy(&state, &app), "");
    assert!(!state.read_with(&app, |s, _| s.has_local_selection()));
}

struct EmptyAtomic;
impl MarkdownPlugin for EmptyAtomic {
    fn name(&self) -> &str {
        "empty-atomic"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        if let gpui_base::text::markdown_ast::Node::Image(code) = node {
            Some(
                MarkdownNode::new("empty-atomic", ())
                    .text("")
                    .markdown(format!("![]({})", code.url)),
            )
        } else {
            None
        }
    }
}
fn empty_atomic(source: &str) -> PreparedText {
    PreparedText::parse(source, MarkdownExtensions::default().plugin(EmptyAtomic)).unwrap()
}

#[test]
fn empty_atomic_edges_select_the_actual_owner_without_copy_placeholders() {
    let (mut app, state) = mount(empty_atomic("A![](one)![](two)Z"));
    let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let objects = text
        .parts()
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_atomic())
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    assert_eq!(objects.len(), 2);
    assert_eq!(text.text(), "AZ\n");
    let first = text.selection_for_part(objects[0]).unwrap();
    let second = text.selection_for_part(objects[1]).unwrap();
    assert_eq!(first.bytes(), 1..1);
    assert_eq!(second.bytes(), 1..1);
    assert!(!second.is_collapsed());
    assert_ne!(
        first.anchor().content_position(),
        second.anchor().content_position()
    );
    assert_eq!(
        first.head().content_position(),
        second.anchor().content_position()
    );
    let selected = state.read_with(&app, |s, _| {
        s.prepare_rendered_selection(second.head(), second.anchor())
            .unwrap()
    });
    state
        .update(&mut app, |s, cx| s.apply_rendered_selection(selected, cx))
        .unwrap();
    assert_eq!(copy(&state, &app), "");
    state.read_with(&app, |s, _| {
        assert!(s.has_local_selection());
        assert!(s.rendered_selection().unwrap().is_backward());
        assert!(!s.rendered_selection().unwrap().is_collapsed());
        assert_eq!(
            s.rendered_text().unwrap().selected_fragment_ranges(),
            vec![1..1]
        );
    });
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), "![](two)");
    let collapsed = state.read_with(&app, |s, _| {
        s.prepare_rendered_selection(second.anchor(), second.anchor())
            .unwrap()
    });
    state
        .update(&mut app, |s, cx| s.apply_rendered_selection(collapsed, cx))
        .unwrap();
    assert_eq!(copy(&state, &app), "");
    assert!(!state.read_with(&app, |s, _| s.has_local_selection()));
}

#[test]
fn empty_atomic_all_objects_have_ordered_edges_and_stream_without_selecting_new_objects() {
    let source = "![](one)![](two)";
    let (mut app, state) = mount(empty_atomic(source));
    let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    assert_eq!(text.text(), "");
    state.update(&mut app, |s, cx| s.select_all(cx));
    let full = state.read_with(&app, |s, _| s.rendered_selection().unwrap());
    assert_eq!(full.bytes(), 0..0);
    assert!(!full.is_collapsed());
    let old_end = full.head().content_position();
    assert_eq!(old_end.object_boundary(), 2);
    state.update(&mut app, |s, cx| {
        s.set_prepared(
            empty_atomic("![](one)![](two)![](new)"),
            Some(source.len()),
            cx,
        )
    });
    assert_eq!(copy(&state, &app), "");
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), source);
    state.read_with(&app, |s, _| {
        let new = s.rendered_text().unwrap();
        assert!(new.captured_position(old_end).is_none());
        let retained = s.rendered_selection().unwrap();
        assert!(!retained.is_collapsed());
        assert_eq!(retained.head().content_position().object_boundary(), 2);
        assert_eq!(
            new.full_selection()
                .head()
                .content_position()
                .object_boundary(),
            3
        );
        assert_eq!(new.selected_fragment_ranges(), vec![0..0, 0..0]);
    });
}

#[test]
fn empty_atomic_resource_refresh_keeps_edges_but_replacement_retires_them() {
    let extensions = MarkdownExtensions::default()
        .plugin(EmptyAtomic)
        .block_renderer("refresh", |_, _, _| gpui::div());
    let source = "A![](one)![](two)Z";
    let (mut app, state) = mount(PreparedText::parse(source, extensions.clone()).unwrap());
    let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
    let index = text
        .parts()
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_atomic())
        .nth(1)
        .unwrap()
        .0;
    let selection = text.selection_for_part(index).unwrap();
    let make_request = || {
        state.read_with(&app, |s, _| {
            s.prepare_rendered_selection(selection.anchor(), selection.head())
                .unwrap()
        })
    };
    let active = make_request();
    let queued = make_request();
    state
        .update(&mut app, |s, cx| s.apply_rendered_selection(active, cx))
        .unwrap();
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx);
        s.set_markdown_extensions(
            std::sync::Arc::new(extensions.block_renderer("refresh", |_, _, _| gpui::div())),
            cx,
        );
    });
    assert_eq!(copy(&state, &app), "![](two)");
    state.read_with(&app, |s, _| {
        let selected = s.rendered_selection().unwrap();
        assert!(!selected.is_collapsed());
        assert_eq!(selected.anchor().content_position().object_boundary(), 1);
        assert_eq!(selected.head().content_position().object_boundary(), 2);
    });
    assert_eq!(
        state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
        Err(Error::StaleRequest)
    );
    // Same empty logical text and byte offsets are insufficient: source identity changed.
    state.update(&mut app, |s, cx| {
        s.set_prepared(empty_atomic("A![](two)![](one)Z"), None, cx)
    });
    assert_eq!(copy(&state, &app), "");
    assert!(!state.read_with(&app, |s, _| s.has_local_selection()));
}

#[derive(Clone)]
struct OpaqueBlock {
    copy: &'static str,
    non_text: bool,
}
impl MarkdownPlugin for OpaqueBlock {
    fn name(&self) -> &str {
        "opaque-block"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        context: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::Blockquote(_)).then(|| {
            MarkdownNode::new("opaque-block", ())
                .text(self.copy)
                .markdown(context.node_source(node).unwrap_or_default())
        })
    }
    fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
        if self.non_text {
            MarkdownPresentation::NonText
        } else {
            MarkdownPresentation::Opaque
        }
    }
}
#[test]
fn opaque_block_atomic_requests_copy_empty_and_nonempty_owners_and_stream() {
    for non_text in [false, true] {
        for alternative in ["", "alternative"] {
            let extensions = MarkdownExtensions::default().plugin(OpaqueBlock {
                copy: alternative,
                non_text,
            });
            let source = "Before\n\n> first\n\n> second\n\nAfter";
            let (mut app, state) = mount(PreparedText::parse(source, extensions.clone()).unwrap());
            let text = state.read_with(&app, |s, _| s.rendered_text().unwrap());
            let index = text
                .parts()
                .iter()
                .enumerate()
                .filter(|(_, p)| p.is_atomic())
                .nth(1)
                .unwrap()
                .0;
            let selected = text.selection_for_part(index).unwrap();
            let request = state.read_with(&app, |s, _| {
                s.prepare_rendered_selection(selected.head(), selected.anchor())
                    .unwrap()
            });
            state
                .update(&mut app, |s, cx| s.apply_rendered_selection(request, cx))
                .unwrap();
            assert_eq!(copy(&state, &app), alternative);
            state.read_with(&app, |s, _| {
                assert!(s.has_local_selection());
                let range = s.rendered_selection().unwrap();
                assert!(!range.is_collapsed());
                assert!(range.is_backward());
                assert_eq!(
                    s.rendered_text().unwrap().selected_fragment_ranges(),
                    vec![selected.bytes()]
                );
            });
            state.update(&mut app, |s, cx| {
                s.set_selection_format(SelectionFormat::Source, cx)
            });
            assert_eq!(copy(&state, &app), "> second");
            state.update(&mut app, |s, cx| {
                s.set_prepared(
                    PreparedText::parse(&format!("{source}\n\n> third"), extensions).unwrap(),
                    Some(source.len()),
                    cx,
                )
            });
            assert_eq!(copy(&state, &app), "> second");
            assert!(state.read_with(&app, |s, _| s.rendered_selection().unwrap().is_backward()));
            state.update(&mut app, |s, cx| s.clear_selection(cx));
            assert_eq!(copy(&state, &app), "");
            assert!(!state.read_with(&app, |s, _| s.has_local_selection()));
        }
    }
}

struct ChangingBlock(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl MarkdownPlugin for ChangingBlock {
    fn name(&self) -> &str {
        "changing-block"
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
            MarkdownNode::new("changing-block", ())
                .text("Glyph")
                .markdown("> widget")
        })
    }
    fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
        if self.0.load(std::sync::atomic::Ordering::Relaxed) {
            MarkdownPresentation::Text("Glyph".into())
        } else {
            MarkdownPresentation::NonText
        }
    }
}
#[test]
fn opaque_block_atomic_resource_kind_changes_do_not_shadow_glyph_selection() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let glyphs = Arc::new(AtomicBool::new(false));
    let extensions = MarkdownExtensions::default().plugin(ChangingBlock(glyphs.clone()));
    let source = "> widget\n\nAfter";
    let (mut app, state) = mount(PreparedText::parse(source, extensions.clone()).unwrap());
    apply(&state, &mut app, 0, 5);
    glyphs.store(true, Ordering::Relaxed);
    let refreshed = extensions.block_renderer("changing-block", |_, _, _| gpui::div());
    state.update(&mut app, |s, cx| {
        s.set_markdown_extensions(Arc::new(refreshed.clone()), cx)
    });
    assert_eq!(copy(&state, &app), "Glyph");
    apply(&state, &mut app, 0, 2);
    state.update(&mut app, |s, cx| {
        s.set_selection_format(SelectionFormat::Source, cx)
    });
    assert_eq!(copy(&state, &app), "Gl");
    state.update(&mut app, |s, cx| {
        s.select_all(cx);
        s.set_prepared(
            PreparedText::parse(&format!("{source} appended"), refreshed.clone()).unwrap(),
            Some(source.len()),
            cx,
        );
    });
    apply(&state, &mut app, 0, 2);
    assert_eq!(
        copy(&state, &app),
        "Gl",
        "All transfer must not mark a glyph-owned block atomic"
    );
    glyphs.store(false, Ordering::Relaxed);
    state.update(&mut app, |s, cx| {
        s.set_markdown_extensions(
            Arc::new(refreshed.block_renderer("changing-block", |_, _, _| gpui::div())),
            cx,
        )
    });
    assert_eq!(copy(&state, &app), "");
    assert!(!state.read_with(&app, |s, _| s.has_local_selection()));
}
#[test]
fn opaque_block_atomic_append_that_changes_the_selected_occurrence_cancels_it() {
    let extensions = MarkdownExtensions::default().plugin(OpaqueBlock {
        copy: "same",
        non_text: true,
    });
    let source = "Before\n\n> initial";
    let (mut app, state) = mount(PreparedText::parse(source, extensions.clone()).unwrap());
    apply(&state, &mut app, 7, 11);
    state.update(&mut app, |s, cx| {
        s.set_prepared(
            PreparedText::parse(&format!("{source}\n> appended"), extensions).unwrap(),
            Some(source.len()),
            cx,
        )
    });
    assert_eq!(copy(&state, &app), "");
    assert!(!state.read_with(&app, |s, _| s.has_local_selection()));
}

#[test]
fn rendered_select_all_preserves_terminal_separator_copy_when_stream_extends_last_block() {
    for source in ["First\n\nLast", "First\n\nLast\n"] {
        let (mut app, state) = mount(parse(source));
        state.update(&mut app, |s, cx| s.select_all(cx));
        let frozen = copy(&state, &app);
        for suffix in [" continued", " continued again"] {
            state.update(&mut app, |s, cx| {
                s.set_prepared(parse(&format!("{source}{suffix}")), Some(source.len()), cx);
            });
            assert_eq!(copy(&state, &app), frozen, "source {source:?}");
        }
        // A new partial selection retires the whole-copy snapshot.
        apply(&state, &mut app, 0, 2);
        assert_eq!(copy(&state, &app), "Fi");
    }
}

#[test]
fn opaque_block_atomic_unpainted_ranges_retire_requests_and_release_owners() {
    for alternative in ["", "same"] {
        let extensions = MarkdownExtensions::default().plugin(OpaqueBlock {
            copy: alternative,
            non_text: true,
        });
        let source = (0..200)
            .map(|i| format!("> widget {i}\n\n"))
            .collect::<String>();
        let (mut app, state) = mount(PreparedText::parse(&source, extensions.clone()).unwrap());
        let old = state.read_with(&app, |s, _| s.rendered_text().unwrap());
        let atoms = old
            .parts()
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_atomic())
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(atoms.len(), 200);
        let start = old.selection_for_part(atoms[2]).unwrap();
        let end = old.selection_for_part(atoms[198]).unwrap();
        let selected = state.read_with(&app, |s, _| {
            s.prepare_rendered_selection(end.head(), start.anchor())
                .unwrap()
        });
        state
            .update(&mut app, |s, cx| s.apply_rendered_selection(selected, cx))
            .unwrap();
        state.update(&mut app, |s, cx| {
            s.set_selection_format(SelectionFormat::Source, cx)
        });
        let frozen = copy(&state, &app);
        assert!(frozen.starts_with("> widget 2\n"));
        assert!(frozen.ends_with("> widget 198"));
        assert!(!frozen.contains("> widget 199"));
        assert_eq!(old.selected_fragment_ranges().len(), 197);
        let queued = state.read_with(&app, |s, _| {
            s.prepare_rendered_selection(start.anchor(), start.head())
                .unwrap()
        });
        state.update(&mut app, |s, cx| {
            s.set_prepared(
                PreparedText::parse(&format!("{source}> new"), extensions.clone()).unwrap(),
                Some(source.len()),
                cx,
            )
        });
        assert_eq!(
            state.update(&mut app, |s, cx| s.apply_rendered_selection(queued, cx)),
            Err(Error::StaleRequest)
        );
        assert_eq!(copy(&state, &app), frozen);
        state.update(&mut app, |s, cx| {
            s.set_prepared(
                PreparedText::parse("> replacement", extensions).unwrap(),
                None,
                cx,
            )
        });
        assert_eq!(copy(&state, &app), "");
        assert!(!state.read_with(&app, |s, _| s.has_local_selection()));
        // Retaining a projection must not retain obsolete selection owners.
        assert!(old.selected_fragment_ranges().is_empty());
    }
}
