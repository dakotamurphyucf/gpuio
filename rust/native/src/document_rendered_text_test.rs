//! Prepared rendered-selection text; no AX or desktop acceptance claim.
use gpui_base::text::{MarkdownExtensions, MarkdownNode, PreparedText};

#[test]
fn rendered_text_matches_native_copy_structure_before_any_layout() {
    let source = "# Heading\n\nA **bold** [link](https://example.test) and `code` 世界.\n\n- one\n- two\n\n> quoted\n\n| A | B |\n| - | - |\n| X | Y |\n\n```txt\ntail\n```\n";
    let prepared = PreparedText::parse(source, MarkdownExtensions::default()).unwrap();
    let rendered = prepared.rendered_text();
    assert_eq!(rendered.text(), prepared.plain_text());
    assert_eq!(
        rendered.text(),
        "Heading\nA bold link and code 世界.\none\ntwo\nquoted\n\nA B\nX Y\n\ntail\n"
    );
    let mut cursor = 0;
    for part in rendered.parts() {
        let bytes = part.bytes();
        assert_eq!(bytes.start, cursor);
        assert!(bytes.end > bytes.start);
        cursor = bytes.end;
    }
    assert_eq!(cursor, rendered.text().len());
    assert!(rendered.selected_fragment_ranges().is_empty());
}

#[test]
fn rendered_positions_are_scalar_checked_and_bound_to_exact_preparation() {
    let source = "Repeated 👨‍👩‍👧‍👦 e\u{301} 世界\n\nRepeated 👨‍👩‍👧‍👦 e\u{301} 世界\n";
    let prepared = PreparedText::parse(source, MarkdownExtensions::default()).unwrap();
    let rendered = prepared.rendered_text();
    let second = PreparedText::parse(source, MarkdownExtensions::default())
        .unwrap()
        .rendered_text();
    for byte in 0..=rendered.text().len() + 1 {
        let position = rendered.position(byte);
        assert_eq!(position.is_some(), rendered.text().is_char_boundary(byte));
        if let Some(position) = position {
            assert_eq!(rendered.offset(&position), Some(byte));
            let captured = position.content_position();
            assert_eq!(
                rendered.offset(&rendered.captured_position(captured).unwrap()),
                Some(byte)
            );
            assert!(second.captured_position(captured).is_none());
            assert_eq!(
                second.offset(&position),
                None,
                "equal text is a different owner"
            );
        }
    }
    // Text identity can outlive the single-use prepared AST without retaining
    // its mutable owners. No original source or substring search is involved.
    let position = rendered.position(0).unwrap();
    drop(prepared);
    assert_eq!(rendered.offset(&position), Some(0));
    assert!(rendered.selected_fragment_ranges().is_empty());
}

struct AtomicAlternative;
impl gpui_base::text::MarkdownPlugin for AtomicAlternative {
    fn name(&self) -> &str {
        "fixture"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::InlineCode(_))
            .then(|| MarkdownNode::new("fixture", ()).text("World\r\n世界"))
    }
}

#[test]
fn rendered_projection_preserves_atomic_alternatives_and_crlf_boundaries() {
    let prepared = PreparedText::parse(
        "Before `object` after.",
        MarkdownExtensions::default().plugin(AtomicAlternative),
    )
    .unwrap();
    let rendered = prepared.rendered_text();
    assert_eq!(rendered.text(), prepared.plain_text());
    assert_eq!(rendered.text(), "Before World\r\n世界 after.\n");
    let atomic = rendered
        .parts()
        .iter()
        .find(|part| part.is_atomic())
        .unwrap();
    assert_eq!(&rendered.text()[atomic.bytes()], "World\r\n世界");
    let newline = rendered.text().find('\n').unwrap();
    assert!(
        rendered.position(newline).is_none(),
        "CRLF is one native break"
    );
    assert!(rendered.position(newline - 1).is_some());
    assert!(rendered.position(newline + 1).is_some());
    let revision = rendered.position(0).unwrap().content_position().revision();
    assert!(
        rendered
            .captured_position(revision.position(newline))
            .is_none()
    );
    let unicode = rendered.text().find('世').unwrap();
    assert!(
        rendered
            .captured_position(revision.position(unicode + 1))
            .is_none()
    );
    assert!(
        rendered
            .captured_position(revision.position(usize::MAX))
            .is_none()
    );
}

#[test]
fn rendered_html_preserves_the_native_custom_block_copy_separators() {
    let prepared = PreparedText::parse_html(
        "<p>Before <img src='test'/> after.</p>",
        MarkdownExtensions::default(),
        |_| MarkdownNode::new("fixture", ()).text("World\r\n世界"),
    )
    .unwrap();
    assert_eq!(prepared.rendered_text().text(), prepared.plain_text());
    assert_eq!(
        prepared.rendered_text().text(),
        "Before \nWorld\r\n世界\n after.\n"
    );
}

#[test]
fn rendered_projection_keeps_all_virtual_blocks_and_empty_document_endpoint() {
    let source = (0..200)
        .map(|i| format!("Paragraph {i} 世界\n\n"))
        .collect::<String>();
    let prepared = PreparedText::parse(&source, MarkdownExtensions::default()).unwrap();
    assert_eq!(prepared.block_count(), 200);
    assert_eq!(prepared.rendered_text().text(), prepared.plain_text());
    assert!(
        prepared
            .rendered_text()
            .text()
            .ends_with("Paragraph 199 世界\n")
    );
    let empty = PreparedText::parse("", MarkdownExtensions::default())
        .unwrap()
        .rendered_text();
    assert_eq!(empty.text(), "");
    assert!(empty.parts().is_empty());
    assert_eq!(empty.offset(&empty.position(0).unwrap()), Some(0));
    assert!(empty.position(1).is_none());
}

#[test]
fn rendered_alternatives_are_bounded_even_when_they_are_not_searchable_glyphs() {
    let prepared =
        PreparedText::parse_html("<img src='test'/>", MarkdownExtensions::default(), |_| {
            MarkdownNode::new("fixture", ())
                .text("x".repeat(gpui_base::text::RenderedText::max_text_bytes() + 1))
        });
    assert!(
        prepared.is_err(),
        "copy/AX alternatives need their own bound"
    );
}

#[test]
fn rendered_projection_retires_when_switching_to_unbounded_parser_updates() {
    use gpui::{AppContext, TestAppContext};
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let state = app.new(|cx| gpui_base::TextViewState::markdown("", cx));
    app.run_until_parked();
    let prepared = PreparedText::parse("Old 世界", MarkdownExtensions::default()).unwrap();
    let old = prepared.rendered_text();
    let position = old.position(0).unwrap();
    state.update(&mut app, |state, cx| state.set_prepared(prepared, None, cx));
    assert!(std::sync::Arc::ptr_eq(
        &old,
        &state.read_with(&app, |state, _| state.rendered_text().unwrap())
    ));
    state.update(&mut app, |state, cx| {
        state.set_text("Replacement 日本語", cx)
    });
    for _ in 0..200 {
        app.run_until_parked();
        if state.read_with(&app, |state, _| state.rendered_text().is_none()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(state.read_with(&app, |state, _| state.rendered_text().is_none()));
    state.update(&mut app, |state, cx| state.select_all(cx));
    assert_eq!(
        state.read_with(&app, |state, _| state.selected_text()),
        "Replacement 日本語\n"
    );
    assert_eq!(old.offset(&position), Some(0));
    assert_eq!(old.text(), "Old 世界\n");
}
