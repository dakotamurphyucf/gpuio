//! Markdown images cannot trigger ambient network or filesystem access. A safe
//! parser always intercepts them; renderers receive already-decoded assets only.
use gpui::{IntoElement, ParentElement, Styled, StyledImage, div, img, px};
use gpui_base::text::{
    MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, markdown_ast,
};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Default)]
pub struct Images(pub BTreeMap<String, Arc<gpui::RenderImage>>);
impl MarkdownPlugin for Images {
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
        if let Some(image) = node.data::<String>().and_then(|url| self.0.get(url)) {
            div()
                .max_w(px(640.))
                .overflow_hidden()
                .child(
                    img(image.clone())
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
}
pub fn extensions(images: Images) -> MarkdownExtensions {
    MarkdownExtensions::default()
        .plugin(images)
        .plugin(LiteralHtml(true))
        .plugin(LiteralHtml(false))
}

struct LiteralHtml(bool);
impl MarkdownPlugin for LiteralHtml {
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
    fn images_and_custom_nodes_are_explicit_opaque_boundaries() {
        let source = "left ![not glyphs](asset://image) right\n\n![alone](asset://image)\n\n<a>literal</a>\n";
        let native = PreparedMarkdown::parse(source, extensions(Images::default())).unwrap();
        assert_eq!(fragments(&native), ["left ", " right", "literal"]);
        assert_eq!(native.displayed_text().opaque_nodes(), 4);
        let builtin = PreparedMarkdown::parse(
            "left ![not glyphs](asset://image) right",
            MarkdownExtensions::default(),
        )
        .unwrap();
        assert_eq!(fragments(&builtin), ["left ", " right"]);
        assert_eq!(builtin.displayed_text().opaque_nodes(), 1);
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
