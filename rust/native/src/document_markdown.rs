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
