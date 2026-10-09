//! Reader images cannot trigger ambient network or filesystem access. A safe
//! parser always intercepts them; renderers receive already-decoded assets only.
use gpui::{IntoElement, ParentElement, Styled, StyledImage, div, img, prelude::*, px};
use gpui_base::text::{
    InlineElement, InlineRenderContext, MarkdownExtensions, MarkdownNode, MarkdownParseContext,
    MarkdownPlugin, MarkdownPresentation, markdown_ast,
};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Default)]
pub struct Images(pub BTreeMap<String, Arc<gpui::RenderImage>>);

/// Called only by bounded HTML preparation; the native HTML URL renderer never
/// receives these nodes. Preserve dimensions and the surrounding link marks.
pub fn html_image(image: &gpui_base::text::HtmlImage) -> MarkdownNode {
    let alt = image.alt.as_ref().map_or("", |s| s.as_ref());
    let alt = if alt.trim().is_empty() { "" } else { alt };
    MarkdownNode::new("gpuio-image", image.clone())
        .text(alt.to_owned())
        .accessibility_label(alt.to_owned())
}
fn image_url(node: &MarkdownNode) -> Option<&str> {
    node.data::<String>().map(String::as_str).or_else(|| {
        node.data::<gpui_base::text::HtmlImage>()
            .map(|image| image.url.as_ref())
    })
}

impl MarkdownPlugin for Images {
    fn presentation(&self, node: &MarkdownNode) -> MarkdownPresentation {
        if image_url(node).is_some_and(|url| self.0.contains_key(url)) {
            MarkdownPresentation::NonText
        } else if node.as_text().trim().is_empty() {
            MarkdownPresentation::Text("".into())
        } else {
            MarkdownPresentation::Text(format!("[Image: {}]", node.as_text()).into())
        }
    }
    fn name(&self) -> &str {
        "gpuio-image"
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let (url, alt) = match node {
            markdown_ast::Node::Image(image) => (image.url.clone(), image.alt.clone()),
            markdown_ast::Node::ImageReference(image) => (String::new(), image.alt.clone()),
            _ => return None,
        };
        let alt = if alt.trim().is_empty() {
            String::new()
        } else {
            alt
        };
        Some(
            MarkdownNode::new("gpuio-image", url)
                .text(alt.clone())
                .accessibility_label(alt)
                .markdown(cx.node_source(node).unwrap_or("").to_string()),
        )
    }
    fn render(
        &self,
        node: &MarkdownNode,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> impl IntoElement {
        if let Some(image) = image_url(node).and_then(|url| self.0.get(url)) {
            div()
                .id("document-image")
                .when(!node.as_text().trim().is_empty(), |element| {
                    element
                        .role(gpui::Role::Image)
                        .aria_label(node.as_text().to_owned())
                })
                .max_w(px(640.))
                .overflow_hidden()
                .child(
                    img(image.clone())
                        .when_some(
                            node.data::<gpui_base::text::HtmlImage>()
                                .and_then(|image| image.width),
                            |view, width| view.w(width),
                        )
                        .when_some(
                            node.data::<gpui_base::text::HtmlImage>()
                                .and_then(|image| image.height),
                            |view, height| view.h(height),
                        )
                        .max_w(px(640.))
                        .max_h(px(480.))
                        .object_fit(gpui::ObjectFit::Contain),
                )
                .into_any_element()
        } else {
            div()
                .child(format!("[Image: {}]", node.as_text()))
                .into_any_element()
        }
    }

    fn render_inline(
        &self,
        node: &MarkdownNode,
        _: &InlineRenderContext,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Option<InlineElement> {
        Some(InlineElement::new(self.render(node, window, cx)).hide_accessibility_when_linked())
    }
}
#[cfg(test)]
pub fn extensions(images: Images) -> MarkdownExtensions {
    extensions_with_options(images, Default::default())
}
pub fn extensions_with_options(
    images: Images,
    options: gpuio_protocol::document::MarkdownOptions,
) -> MarkdownExtensions {
    let mut extensions = MarkdownExtensions::default()
        .plugin(images)
        .plugin(LiteralHtml(true))
        .plugin(LiteralHtml(false));
    extensions = match options.frontmatter {
        gpuio_protocol::document::Frontmatter::Disabled => extensions,
        gpuio_protocol::document::Frontmatter::CodeBlock => extensions.frontmatter(),
        gpuio_protocol::document::Frontmatter::DescriptionList => {
            extensions.frontmatter_description_list()
        }
    };
    if options.mdx {
        extensions = extensions.mdx();
    }
    extensions
}

struct LiteralHtml(bool);
impl MarkdownPlugin for LiteralHtml {
    fn presentation(&self, node: &MarkdownNode) -> MarkdownPresentation {
        MarkdownPresentation::Text(node.as_text().to_string().into())
    }
    fn is_block(&self) -> bool {
        self.0
    }
    fn name(&self) -> &str {
        "gpuio-literal-html"
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        _: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        if let markdown_ast::Node::Html(html) = node {
            Some(
                MarkdownNode::new("gpuio-literal-html", ())
                    .text(html.value.clone())
                    .markdown(html.value.clone()),
            )
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_base::text::PreparedMarkdown;

    fn fragments(document: &PreparedMarkdown) -> Vec<String> {
        let displayed = document.displayed_text();
        assert!(
            displayed
                .fragments()
                .iter()
                .enumerate()
                .all(|(ix, fragment)| { fragment.id() as usize == ix })
        );
        assert_eq!(
            displayed.text_bytes(),
            displayed
                .fragments()
                .iter()
                .map(|f| f.text().len())
                .sum::<usize>()
        );
        displayed
            .fragments()
            .iter()
            .map(|f| f.text().to_string())
            .collect()
    }

    #[test]
    fn frontmatter_descriptions_keep_native_fragments_and_exact_source() {
        let source = "---\nname: Native 世界\nsummary: >-\n  First line\n  second line\nnotes: |-\n  # literal\n\n  last  \nempty:\n---\n\nBody";
        let document = PreparedMarkdown::parse(
            source,
            extensions_with_options(
                Default::default(),
                gpuio_protocol::document::MarkdownOptions {
                    frontmatter: gpuio_protocol::document::Frontmatter::DescriptionList,
                    mdx: false,
                },
            ),
        )
        .unwrap();
        assert_eq!(document.source().as_ref(), source);
        assert_eq!(
            fragments(&document),
            [
                "name:",
                "Native 世界",
                "summary:",
                "First line second line",
                "notes:",
                "# literal\n\nlast  ",
                "empty:",
                "Body"
            ]
        );
        assert_eq!(document.displayed_text().opaque_nodes(), 0);
        assert!(document.code_blocks().is_empty());
        assert!(
            document
                .plain_text()
                .contains("name: Native 世界\nsummary: First line second line")
        );
    }

    #[test]
    fn frontmatter_descriptions_fall_back_without_losing_unsupported_yaml() {
        for yaml in [
            "title: Hello # comment",
            "title: \"quoted\"",
            "tags: [one, two]",
            "config: {theme: dark}",
            "config:\n  theme: dark",
            "value: &anchor hello",
            "value: *anchor",
            "value: !!str hello",
            "value: a: b",
            "- name: example",
            "notes: >-\n  first\n    indented\n  last",
            "notes: |\n  text",
            "notes: |-\n    first\n  invalid indentation",
            "# only a comment",
        ] {
            let source = format!("---\n{yaml}\n---\n\nBody");
            let document = PreparedMarkdown::parse(
                &source,
                extensions_with_options(
                    Default::default(),
                    gpuio_protocol::document::MarkdownOptions {
                        frontmatter: gpuio_protocol::document::Frontmatter::DescriptionList,
                        mdx: false,
                    },
                ),
            )
            .unwrap();
            assert_eq!(document.source().as_ref(), source);
            assert_eq!(document.code_blocks().len(), 1, "{yaml}");
            assert_eq!(document.code_blocks()[0].code().as_ref(), yaml, "{yaml}");
            assert!(fragments(&document).iter().any(|f| f == yaml), "{yaml}");
        }
    }

    #[test]
    fn frontmatter_descriptions_bound_rows_without_truncation() {
        for count in [128, 129] {
            let yaml = (0..count)
                .map(|i| format!("key{i}: value{i}\n"))
                .collect::<String>();
            let source = format!("---\n{yaml}---\n\nBody");
            let document = PreparedMarkdown::parse(
                &source,
                extensions_with_options(
                    Default::default(),
                    gpuio_protocol::document::MarkdownOptions {
                        frontmatter: gpuio_protocol::document::Frontmatter::DescriptionList,
                        mdx: false,
                    },
                ),
            )
            .unwrap();
            assert_eq!(document.source().as_ref(), source);
            assert_eq!(document.code_blocks().len(), usize::from(count > 128));
            let displayed = fragments(&document);
            assert_eq!(displayed.len(), if count == 128 { 257 } else { 2 });
            assert!(document.plain_text().contains(&format!(
                "key{}: value{}",
                count - 1,
                count - 1
            )));
        }
    }

    #[test]
    fn mdx_keeps_nested_inline_and_block_children_without_evaluation() {
        let options = gpuio_protocol::document::MarkdownOptions {
            mdx: true,
            ..Default::default()
        };
        let source = "Before <Note>nested **bold** <Tag>[link](gpuio:test) {1 + 2}</Tag></Note> after.\n\n<Panel>\n\n# Heading\n\n- First\n- Second\n\n```txt\ncode\n```\n\n</Panel>\n";
        let document =
            PreparedMarkdown::parse(source, extensions_with_options(Default::default(), options))
                .unwrap();
        let text = document.plain_text();
        for expected in [
            "Before nested bold link 1 + 2 after.",
            "Heading",
            "First",
            "Second",
            "code",
        ] {
            assert!(text.contains(expected), "missing {expected:?}: {text:?}");
        }
        assert_eq!(document.source().as_ref(), source);
        assert_eq!(document.code_blocks().len(), 1);
        assert!(!text.contains("<Note>"));
        assert!(
            fragments(&document)
                .iter()
                .any(|f| f.contains("nested bold link 1 + 2"))
        );
    }

    #[test]
    fn html_reader_prepares_displayed_text_code_and_every_image_without_url_loading() {
        let source = r#"<html><head><title>hidden title</title><style>hidden style</style></head><body>
<h1>Hello &amp; 世界</h1><p>First<br>second <b>bold</b><script>hidden script</script></p>
<pre><code>  let answer = &lt;42&gt;\n</code></pre>
<p><a href="test:image"><img src="https://example.invalid/image" alt="Prism" width="180" height="NaN"></a></p>
<table><tr><td><img src="file:///unavailable" alt="Table"></td></tr></table>
<blockquote><p><img src="asset://quote" alt="Quote"></p></blockquote>
<ul><li><img src="asset://list" alt="List"></li></ul>
<template><img src="hidden" alt="hidden template"></template>
</body></html>"#;
        let mut images = Vec::new();
        let document =
            PreparedMarkdown::parse_html(source, extensions(Images::default()), |image| {
                images.push(image.clone());
                html_image(image)
            })
            .unwrap();
        assert_eq!(document.source().as_ref(), source);
        let text = document.plain_text();
        assert!(
            text.contains("Hello & 世界") && text.contains("First\nsecond"),
            "{text:?}"
        );
        assert!(!text.contains("hidden"), "{text:?}");
        assert_eq!(images.len(), 4);
        assert_eq!(images[0].url.as_ref(), "https://example.invalid/image");
        assert!(images[0].width.is_some() && images[0].height.is_none());
        assert_eq!(images[0].link.as_ref().unwrap().url.as_ref(), "test:image");
        assert_eq!(
            document.code_blocks()[0].code().as_ref(),
            "  let answer = <42>\\n"
        );
        assert!(
            document.block_count() > 4,
            "HTML wrappers must not defeat virtualization"
        );
        let text = fragments(&document).join("|");
        for alt in ["Prism", "Table", "Quote", "List"] {
            assert!(text.contains(&format!("[Image: {alt}]")), "{text}");
        }
    }

    #[test]
    fn html_reader_bounds_input_structure_and_invalid_dimensions() {
        for source in [
            "x".repeat(16385),
            "<div>".repeat(40),
            "<p>x</p>\n".repeat(257),
            "<span>x</span>\n".repeat(2200),
        ] {
            assert!(
                PreparedMarkdown::parse_html(&source, extensions(Images::default()), html_image)
                    .is_err()
            );
        }
        for dimension in ["NaN", "inf", "-1", "16385", "101%"] {
            let mut called = false;
            PreparedMarkdown::parse_html(
                &format!("<img src='asset://test' width='{dimension}' height='{dimension}'>"),
                extensions(Images::default()),
                |image| {
                    called = true;
                    assert!(
                        image.width.is_none() && image.height.is_none(),
                        "{dimension}"
                    );
                    html_image(image)
                },
            )
            .unwrap();
            assert!(called);
        }
    }

    #[test]
    fn decorative_image_alternatives_do_not_become_raw_source_or_placeholder_text() {
        for image in ["![](asset://missing)", "![   ](asset://missing)"] {
            let prepared = PreparedMarkdown::parse(
                &format!("before {image} after"),
                extensions(Images::default()),
            )
            .unwrap();
            assert_eq!(fragments(&prepared).join(""), "before  after");
            assert!(!prepared.plain_text().contains("asset://missing"));
            assert!(!prepared.plain_text().contains("[Image:"));
        }
    }

    #[test]
    fn displayed_fragments_follow_structural_order_without_copy_separators() {
        let prepared = PreparedMarkdown::parse(
            "# Head **bold**\n\nhello *café* and `code`\n\n> quoted\n\n- first\n  - nested\n- last\n\n```rs\nlet λ = 1;\n```\n\n| one | two |\n|---|---|\n| α | β |\n",
            MarkdownExtensions::default(),
        ).unwrap();
        assert_eq!(
            fragments(&prepared),
            [
                "Head bold",
                "hello café and code",
                "quoted",
                "first",
                "nested",
                "last",
                "let λ = 1;",
                "one",
                "two",
                "α",
                "β",
            ]
        );
        assert_eq!(prepared.displayed_text().opaque_nodes(), 0);
        assert!(Arc::ptr_eq(
            &prepared.displayed_text(),
            &prepared.displayed_text()
        ));
    }

    #[test]
    fn displayed_fragments_exist_before_paint_and_survive_prepared_owner_drop() {
        let source = (0..200).map(|i| format!("row {i}\n\n")).collect::<String>();
        let before = PreparedMarkdown::parse(&source, MarkdownExtensions::default()).unwrap();
        let retained = before.displayed_text();
        assert_eq!(retained.fragments().len(), 200);
        assert_eq!(retained.fragments()[199].text().as_ref(), "row 199");
        let after = PreparedMarkdown::parse(&source, MarkdownExtensions::default()).unwrap();
        assert!(!Arc::ptr_eq(&retained, &after.displayed_text()));
        drop(before);
        assert_eq!(retained.fragments()[0].text().as_ref(), "row 0");
    }

    #[test]
    fn native_placeholders_and_literal_html_have_explicit_text_projections() {
        let source = "left ![not glyphs](asset://image) right\n\n![alone](asset://image)\n\n<a>literal</a>\n";
        let native = PreparedMarkdown::parse(source, extensions(Images::default())).unwrap();
        assert_eq!(
            fragments(&native),
            [
                "left ",
                "[Image: not glyphs]",
                " right",
                "[Image: alone]",
                "<a>",
                "literal",
                "</a>"
            ]
        );
        assert_eq!(native.displayed_text().opaque_nodes(), 0);
        let builtin = PreparedMarkdown::parse(
            "left ![not glyphs](asset://image) right",
            MarkdownExtensions::default(),
        )
        .unwrap();
        assert_eq!(fragments(&builtin), ["left ", " right"]);
        assert_eq!(builtin.displayed_text().opaque_nodes(), 1);
    }

    #[test]
    fn custom_presentation_is_explicit_bounded_and_independent_of_copy_text() {
        struct Plugin(MarkdownPresentation);
        impl MarkdownPlugin for Plugin {
            fn name(&self) -> &str {
                "projection-fixture"
            }
            fn parse(
                &self,
                node: &markdown_ast::Node,
                _: &MarkdownParseContext<'_>,
            ) -> Option<MarkdownNode> {
                matches!(node, markdown_ast::Node::InlineCode(_))
                    .then(|| MarkdownNode::new(self.name(), ()).text("copy-only λ"))
            }
            fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
                self.0.clone()
            }
        }
        let prepare = |mode| {
            PreparedMarkdown::parse(
                "before `object` after",
                MarkdownExtensions::default().plugin(Plugin(mode)),
            )
        };
        let opaque = prepare(MarkdownPresentation::Opaque).unwrap();
        assert_eq!(opaque.displayed_text().opaque_nodes(), 1);
        assert_eq!(fragments(&opaque), ["before ", " after"]);
        let graphic = prepare(MarkdownPresentation::NonText).unwrap();
        assert_eq!(graphic.displayed_text().opaque_nodes(), 0);
        assert_eq!(fragments(&graphic), ["before ", " after"]);
        let text = prepare(MarkdownPresentation::Text("shown 🙂".into())).unwrap();
        assert_eq!(fragments(&text), ["before ", "shown 🙂", " after"]);
        assert!(text.plain_text().contains("copy-only λ"));
        assert!(!text.plain_text().contains("shown 🙂"));
        assert!(prepare(MarkdownPresentation::Text("x".repeat(65537).into())).is_err());
    }

    #[test]
    fn prepared_backgrounds_validate_count_utf8_ranges_and_radius() {
        use gpui_base::{
            input::{RangeBackground, RangeBackgroundError, RangeBackgrounds},
            text::TextBackgrounds,
        };
        use std::rc::Rc;
        struct Layers(Vec<RangeBackground>);
        impl RangeBackgrounds for Layers {
            fn ranges(&self) -> &[RangeBackground] {
                &self.0
            }
        }
        let prepared = PreparedMarkdown::parse("α🙂", MarkdownExtensions::default()).unwrap();
        let source = prepared.displayed_text();
        assert!(matches!(
            TextBackgrounds::new(source.clone(), vec![]),
            Err(RangeBackgroundError::InvalidRange)
        ));
        let layer = |bytes, radius, count| -> Option<Rc<dyn RangeBackgrounds>> {
            Some(Rc::new(Layers(vec![
                RangeBackground {
                    bytes,
                    color: gpui::red(),
                    radius: px(radius)
                };
                count
            ])))
        };
        assert!(TextBackgrounds::new(source.clone(), vec![layer(0..6, 8., 1)]).is_ok());
        for range in [1..2, 2..3, 2..7, 0..0] {
            assert!(matches!(
                TextBackgrounds::new(source.clone(), vec![layer(range, 0., 1)]),
                Err(RangeBackgroundError::InvalidRange)
            ));
        }
        for radius in [f32::NAN, f32::INFINITY, -1., 65.] {
            assert!(matches!(
                TextBackgrounds::new(source.clone(), vec![layer(0..2, radius, 1)]),
                Err(RangeBackgroundError::InvalidRadius)
            ));
        }
        assert!(matches!(
            TextBackgrounds::new(source, vec![layer(0..2, 0., 32769)]),
            Err(RangeBackgroundError::Limit)
        ));
    }
    #[test]
    fn reference_images_resolve_and_html_is_literal() {
        let text =
            "![reference][target]\n\n[target]: asset://demo\n\n<img src=\"file:///ignored\">\n";
        let document =
            gpui_base::text::PreparedMarkdown::parse(text, extensions(Images::default())).unwrap();
        assert!(document.plain_text().contains("reference"));
        assert!(document.plain_text().contains("<img src="));
        assert_eq!(document.source().as_ref(), text);
    }
}
