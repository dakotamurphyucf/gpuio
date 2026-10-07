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

#[test]
fn accessible_coordinates_round_trip_unicode_and_unpainted_structure() {
    let source = "# A😀\n\nRepeat e\u{301} 👨‍👩‍👧‍👦 **世界** [עברית](test:link).\n\n| a | b |\n| - | - |\n| العربية | e\u{301} |\n\n```txt\nline one\nline two\n```\n";
    let prepared = PreparedText::parse(source, MarkdownExtensions::default()).unwrap();
    let text = prepared.rendered_text();
    let accessible = text
        .accessible_parts()
        .iter()
        .map(|part| text.accessible_part_text(part.id()).unwrap())
        .collect::<String>();
    assert_eq!(accessible, text.text());
    for byte in 0..=text.text().len() {
        if let Some(position) = text.position(byte) {
            let utf16 = text.text()[..byte].encode_utf16().count();
            assert_eq!(text.accessible_utf16_offset(&position), Some(utf16));
            assert_eq!(
                text.position_from_accessible_utf16(utf16)
                    .unwrap()
                    .content_position(),
                position.content_position()
            );
            let (part, character) = text.accessible_coordinates(&position).unwrap();
            assert_eq!(
                text.accessible_position(part, character)
                    .unwrap()
                    .content_position(),
                position.content_position()
            );
        }
    }
    let emoji = accessible.find('😀').unwrap();
    let surrogate_middle = accessible[..emoji].encode_utf16().count() + 1;
    assert!(
        text.position_from_accessible_utf16(surrogate_middle)
            .is_none()
    );
    assert!(text.position_from_accessible_utf16(usize::MAX).is_none());
    for part in text.accessible_parts() {
        assert_eq!(
            part.character_lengths().map(usize::from).sum::<usize>(),
            text.accessible_part_text(part.id()).unwrap().len()
        );
        assert!(text.accessible_position(part.id(), usize::MAX).is_none());
    }
    let other = PreparedText::parse(source, MarkdownExtensions::default())
        .unwrap()
        .rendered_text();
    assert!(
        other
            .accessible_position(text.accessible_parts()[0].id(), 0)
            .is_none()
    );
    assert!(
        other
            .accessible_part_text(text.accessible_parts()[0].id())
            .is_none()
    );
    assert!(
        other
            .accessible_coordinates(&text.position(0).unwrap())
            .is_none()
    );
    assert!(text.retained_units() <= gpui_base::text::RenderedText::max_preparation_units());
}

#[test]
fn accessible_atomic_text_is_readable_but_internal_selection_is_rejected() {
    let prepared = PreparedText::parse(
        "Before `object` after.",
        MarkdownExtensions::default().plugin(AtomicAlternative),
    )
    .unwrap();
    let text = prepared.rendered_text();
    let part = text
        .accessible_parts()
        .iter()
        .find(|part| text.accessible_part_text(part.id()) == Some("World\r\n世界"))
        .unwrap();
    assert_eq!(part.character_count(), 8);
    assert_eq!(
        part.character_lengths().collect::<Vec<_>>(),
        [1, 1, 1, 1, 1, 2, 3, 3]
    );
    assert_eq!(part.utf16_range().len(), 9);
    let start = text.accessible_position(part.id(), 0).unwrap();
    let end = text.accessible_position(part.id(), 8).unwrap();
    assert_eq!(
        text.selected_text(&text.selection(&end, &start).unwrap()),
        Some("World\r\n世界")
    );
    for character in 1..8 {
        assert!(text.accessible_position(part.id(), character).is_none());
    }
    for offset in part.utf16_range().start + 1..part.utf16_range().end {
        assert!(text.position_from_accessible_utf16(offset).is_none());
    }
}

#[test]
fn accessible_empty_document_retains_a_checked_caret_without_fabricated_text() {
    let text = PreparedText::parse("", MarkdownExtensions::default())
        .unwrap()
        .rendered_text();
    assert_eq!(text.accessible_parts().len(), 1);
    let part = &text.accessible_parts()[0];
    assert_eq!(text.accessible_part_text(part.id()), Some(""));
    assert_eq!(part.character_count(), 0);
    let position = text.accessible_position(part.id(), 0).unwrap();
    assert_eq!(text.offset(&position), Some(0));
    assert_eq!(text.accessible_utf16_offset(&position), Some(0));
    assert!(text.position_from_accessible_utf16(1).is_none());
    assert!(text.retained_units() <= gpui_base::text::RenderedText::max_preparation_units());
}

#[test]
fn accessible_checkpoint_edges_match_utf16_without_linear_document_scans() {
    for length in [63, 64, 65, 127, 128, 129, 1025] {
        let source = (0..length)
            .map(|index| match index % 5 {
                0 => "😀",
                1 => "e\u{301}",
                2 => "א",
                _ => "a",
            })
            .collect::<String>();
        let text = PreparedText::parse(&source, MarkdownExtensions::default())
            .unwrap()
            .rendered_text();
        for (byte, _) in text
            .text()
            .char_indices()
            .chain(std::iter::once((text.text().len(), '\0')))
        {
            let position = text.position(byte).unwrap();
            let utf16 = text.text()[..byte].encode_utf16().count();
            assert_eq!(text.accessible_utf16_offset(&position), Some(utf16));
            assert_eq!(
                text.position_from_accessible_utf16(utf16)
                    .unwrap()
                    .content_position(),
                position.content_position()
            );
        }
        assert!(text.retained_units() <= gpui_base::text::RenderedText::max_preparation_units());
    }
}

struct AccessibleGlyphs(String);
impl gpui_base::text::MarkdownPlugin for AccessibleGlyphs {
    fn name(&self) -> &str {
        "accessible-glyphs"
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
            .then(|| MarkdownNode::new("accessible-glyphs", ()).text(self.0.clone()))
    }
    fn presentation(&self, _: &MarkdownNode) -> gpui_base::text::MarkdownPresentation {
        gpui_base::text::MarkdownPresentation::Text(self.0.clone().into())
    }
}
fn accessible_glyphs(glyphs: String) -> std::sync::Arc<gpui_base::text::RenderedText> {
    PreparedText::parse(
        "> custom",
        MarkdownExtensions::default().plugin(AccessibleGlyphs(glyphs)),
    )
    .unwrap()
    .rendered_text()
}

#[test]
fn accessible_owned_crlf_round_trips_both_units_as_one_character() {
    let text = accessible_glyphs("A\r\n😀B".into());
    let part = &text.accessible_parts()[0];
    assert_eq!(part.character_count(), 4);
    assert_eq!(part.character_lengths().collect::<Vec<_>>(), [1, 2, 4, 1]);
    for (character, byte, utf16) in [(0, 0, 0), (1, 1, 1), (2, 3, 3), (3, 7, 5), (4, 8, 6)] {
        let position = text.accessible_position(part.id(), character).unwrap();
        assert_eq!(text.offset(&position), Some(byte));
        assert_eq!(text.accessible_utf16_offset(&position), Some(utf16));
        assert_eq!(
            text.position_from_accessible_utf16(utf16)
                .unwrap()
                .content_position(),
            position.content_position()
        );
    }
    assert!(text.position_from_accessible_utf16(2).is_none());
    assert!(text.position_from_accessible_utf16(4).is_none());
}

struct AccessibleLargeAlternative(String);
impl gpui_base::text::MarkdownPlugin for AccessibleLargeAlternative {
    fn name(&self) -> &str {
        "large-alternative"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::InlineCode(_))
            .then(|| MarkdownNode::new("large-alternative", ()).text(self.0.clone()))
    }
}
fn accessible_large_alternative(text: String) -> std::sync::Arc<gpui_base::text::RenderedText> {
    PreparedText::parse(
        "`large`",
        MarkdownExtensions::default().plugin(AccessibleLargeAlternative(text)),
    )
    .unwrap()
    .rendered_text()
}

#[test]
fn accessible_large_generated_index_stays_compact_and_within_admission() {
    let bytes = 1024 * 1024;
    let ascii = accessible_large_alternative("a".repeat(bytes));
    let unicode = accessible_large_alternative(format!("{}😀", "a".repeat(bytes - 4)));
    assert_eq!(ascii.text().len(), unicode.text().len());
    // A single Unicode character must not allocate two full offsets per byte.
    assert!(unicode.retained_units() - ascii.retained_units() < bytes * 2);
    assert!(unicode.retained_units() <= gpui_base::text::RenderedText::max_preparation_units());
    let position = unicode.position(bytes).unwrap();
    assert_eq!(unicode.accessible_utf16_offset(&position), Some(bytes - 2));
    assert_eq!(
        unicode
            .position_from_accessible_utf16(bytes - 2)
            .unwrap()
            .content_position(),
        position.content_position()
    );
}

#[test]
fn prepared_semantic_structure_preserves_nested_roles_and_table_coordinates() {
    use gpui_base::text::RenderedSemanticKind as Kind;
    let source = "# Title\n\n> Quote\n>\n> - first\n> - second\n\n| Name | Value |\n| - | - |\n| A | 世界 |\n\n```txt\ncode\n```\n";
    let prepared = PreparedText::parse(source, MarkdownExtensions::default()).unwrap();
    let text = prepared.rendered_text();
    let nodes = text.semantic_nodes();
    let kinds = nodes.iter().map(|node| node.kind()).collect::<Vec<_>>();
    assert!(kinds.contains(&&Kind::Heading { level: 1 }));
    assert!(kinds.contains(&&Kind::Blockquote));
    assert!(kinds.contains(&&Kind::List));
    assert!(kinds.contains(&&Kind::ListItem));
    assert!(kinds.contains(&&Kind::Code));
    let table = nodes
        .iter()
        .find(|node| matches!(node.kind(), Kind::Table { .. }))
        .unwrap();
    assert_eq!(
        table.kind(),
        &Kind::Table {
            rows: 2,
            columns: 2
        }
    );
    let rows = text.semantic_children(Some(table.id())).collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    for (row_index, row) in rows.iter().enumerate() {
        assert_eq!(row.kind(), &Kind::Row { index: row_index });
        assert_eq!(text.semantic_parent(row.id()), Some(table.id()));
        let cells = text.semantic_children(Some(row.id())).collect::<Vec<_>>();
        assert_eq!(cells.len(), 2);
        for (column, cell) in cells.iter().enumerate() {
            assert_eq!(
                cell.kind(),
                &Kind::Cell {
                    row: row_index,
                    column,
                    header: row_index == 0
                }
            );
            let parts = text.semantic_parts(cell.id()).unwrap();
            assert!(!parts.is_empty());
            assert_eq!(text.semantic_parent(cell.id()), Some(row.id()));
            assert!(text.semantic_children(Some(cell.id())).next().is_none());
        }
    }
    let mut end = 0;
    for root in text.semantic_children(None) {
        assert_eq!(root.parts().start, end);
        end = root.parts().end;
        assert_eq!(text.semantic_parent(root.id()), None);
    }
    assert_eq!(end, text.parts().len());
    for node in nodes {
        assert!(node.parts().end <= text.parts().len());
        for child in text.semantic_children(Some(node.id())) {
            assert!(
                node.parts().start <= child.parts().start && child.parts().end <= node.parts().end
            );
        }
    }
    assert_eq!(text.text(), prepared.plain_text());
}

#[test]
fn prepared_semantic_owners_survive_virtualization_without_retaining_the_ast() {
    let source = (0..200)
        .map(|i| format!("## Item {i}\n\n"))
        .collect::<String>();
    let prepared = PreparedText::parse(&source, MarkdownExtensions::default()).unwrap();
    let text = prepared.rendered_text();
    assert_eq!(text.semantic_children(None).count(), 200);
    let first = text.semantic_children(None).next().unwrap().id();
    let last = text.semantic_children(None).last().unwrap().id();
    let second = PreparedText::parse(&source, MarkdownExtensions::default())
        .unwrap()
        .rendered_text();
    assert!(second.semantic_node(first).is_none());
    assert!(second.semantic_parts(first).is_none());
    assert!(second.semantic_selection(first).is_none());
    assert!(second.semantic_children(Some(first)).next().is_none());
    drop(prepared);
    let parts = text.semantic_parts(last).unwrap();
    assert_eq!(&text.text()[parts[0].bytes()], "Item 199");
    assert!(text.selected_fragment_ranges().is_empty());
    assert!(text.retained_units() <= gpui_base::text::RenderedText::max_preparation_units());
}

#[test]
fn prepared_semantics_include_frontmatter_and_declared_native_block_owners() {
    use gpui_base::text::RenderedSemanticKind as Kind;
    let prepared = PreparedText::parse(
        "---\nname: Demo\nstatus: Draft\n---\n\n> custom",
        MarkdownExtensions::default()
            .frontmatter_description_list()
            .plugin(AccessibleGlyphs("Native glyphs".into())),
    )
    .unwrap();
    let text = prepared.rendered_text();
    let kinds = text
        .semantic_nodes()
        .iter()
        .map(|node| node.kind())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&&Kind::DescriptionList));
    assert_eq!(kinds.iter().filter(|kind| ***kind == Kind::Term).count(), 2);
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| ***kind == Kind::Definition)
            .count(),
        2
    );
    let custom = text
        .semantic_nodes()
        .iter()
        .find(|node| *node.kind() == Kind::NativeObject)
        .unwrap();
    let parts = text.semantic_parts(custom.id()).unwrap();
    assert_eq!(&text.text()[parts[0].bytes()], "Native glyphs");
    assert!(!parts[0].is_atomic());
    assert_eq!(text.text(), prepared.plain_text());
}

#[test]
fn prepared_semantic_links_keep_logical_identity_across_styles_and_references() {
    use gpui_base::text::RenderedSemanticKind as Kind;
    let source = "[one **bold** `code` 世界](test:same) and [again](test:same).\n\n[reference][target]\n\n[target]: test:resolved \"Title\"\n";
    let text = PreparedText::parse(source, MarkdownExtensions::default())
        .unwrap()
        .rendered_text();
    let links = text
        .semantic_nodes()
        .iter()
        .filter(|node| matches!(node.kind(), Kind::Link { .. }))
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 3);
    let labels = links
        .iter()
        .map(|node| {
            text.selected_text(&text.semantic_selection(node.id()).unwrap())
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, ["one bold code 世界", "again", "reference"]);
    assert_ne!(links[0].id(), links[1].id());
    let Kind::Link {
        source_start: first,
        url,
        ..
    } = links[0].kind()
    else {
        unreachable!()
    };
    assert_eq!(url.as_ref(), "test:same");
    let Kind::Link {
        source_start: second,
        ..
    } = links[1].kind()
    else {
        unreachable!()
    };
    assert_ne!(first, second);
    let Kind::Link { url, title, .. } = links[2].kind() else {
        unreachable!()
    };
    assert_eq!(url.as_ref(), "test:resolved");
    assert_eq!(title.as_deref(), Some("Title"));
    assert!(text.retained_units() <= gpui_base::text::RenderedText::max_preparation_units());
}

struct SemanticEmptyObject;
impl gpui_base::text::MarkdownPlugin for SemanticEmptyObject {
    fn name(&self) -> &str {
        "empty-link-object"
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        _: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::Image(_))
            .then(|| MarkdownNode::new("empty-link-object", ()).text(""))
    }
}

#[test]
fn prepared_semantic_empty_link_owns_both_atomic_objects_without_invented_copy() {
    use gpui_base::text::RenderedSemanticKind as Kind;
    let text = PreparedText::parse(
        "[![](one)![](two)](test:objects)",
        MarkdownExtensions::default().plugin(SemanticEmptyObject),
    )
    .unwrap()
    .rendered_text();
    let links = text
        .semantic_nodes()
        .iter()
        .filter(|node| matches!(node.kind(), Kind::Link { .. }))
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 1);
    let selection = text.semantic_selection(links[0].id()).unwrap();
    assert!(!selection.is_collapsed());
    assert_eq!(selection.bytes(), 0..0);
    assert_eq!(text.selected_text(&selection), Some(""));
    assert_eq!(text.accessible_utf16_offset(selection.anchor()), Some(0));
    assert_eq!(text.accessible_utf16_offset(selection.head()), Some(2));
}

#[test]
fn prepared_semantic_html_links_preserve_adjacent_equal_targets_and_unicode_ranges() {
    use gpui_base::text::RenderedSemanticKind as Kind;
    let prepared = PreparedText::parse_html(
        "<p>前<a href='test:same'>😀<b>é</b></a><a href='test:same'>終</a>後</p>",
        MarkdownExtensions::default(),
        |_| unreachable!("fixture contains no images"),
    )
    .unwrap();
    let text = prepared.rendered_text();
    let links = text
        .semantic_nodes()
        .iter()
        .filter(|node| matches!(node.kind(), Kind::Link { .. }))
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 2);
    let selections = links
        .iter()
        .map(|node| text.semantic_selection(node.id()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(text.selected_text(&selections[0]), Some("😀é"));
    assert_eq!(text.selected_text(&selections[1]), Some("終"));
    assert_eq!(
        text.accessible_utf16_offset(selections[0].anchor()),
        Some(1)
    );
    assert_eq!(text.accessible_utf16_offset(selections[0].head()), Some(5));
    assert_eq!(text.accessible_utf16_offset(selections[1].head()), Some(6));
    assert_eq!(text.text(), prepared.plain_text());
}

#[test]
fn prepared_semantic_repeated_references_share_long_target_storage() {
    use gpui_base::text::RenderedSemanticKind as Kind;
    let target = format!("test:{}", "a".repeat(8000));
    let source = format!("{}\n\n[ref]: {target}\n", "[x][ref] ".repeat(500));
    let prepared = PreparedText::parse(&source, MarkdownExtensions::default()).unwrap();
    let text = prepared.rendered_text();
    let urls = text
        .semantic_nodes()
        .iter()
        .filter_map(|node| match node.kind() {
            Kind::Link { url, .. } => Some(url),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(urls.len(), 500);
    for url in &urls {
        assert_eq!(url.as_ref(), target);
        assert_eq!(url.as_ptr(), urls[0].as_ptr());
    }
    assert!(text.retained_units() < target.len() * urls.len());
    assert!(text.retained_units() <= gpui_base::text::RenderedText::max_preparation_units());
}
