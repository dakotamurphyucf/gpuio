//! Independent static profile; imports only the public pinned document SDK.
use gpuio_document_sdk as sdk;
use sdk::gpui::{self, IntoElement, ParentElement, Styled, prelude::*};
use std::sync::Arc;
mod scroll_card;
pub const NAME: &str = "example.document";
pub const FINGERPRINT: &str = "985f53077246b78111f091454a0d6f07c770cab3c3cb9218c114e4936f52a26d";
pub fn factory() -> Arc<dyn sdk::Factory> {
    Arc::new(Factory)
}
struct Factory;
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: NAME,
            version: 1,
            fingerprint: FINGERPRINT,
            sdk_version: sdk::SDK_VERSION,
            gpui_revision: sdk::GPUI_REVISION,
            base_revision: sdk::BASE_REVISION,
            max_properties: 1,
            max_event: 1,
            max_retained_bytes: 65536,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        if bytes.len() == 1 && bytes[0] <= 1 {
            Ok(())
        } else {
            Err(sdk::Error::InvalidProperties)
        }
    }
    fn configure(
        &self,
        bytes: &[u8],
        cx: &sdk::PrepareContext<'_>,
    ) -> Result<sdk::Profile, sdk::Error> {
        cx.check()?;
        self.validate_properties(bytes)?;
        let profile = Arc::new(Profile(bytes[0]));
        let configured = sdk::Profile::new()
            .with_actions(profile.clone())
            .with_highlighter(profile.clone())
            .with_plugin(profile)?;
        configured
            .with_plugin(Arc::new(Card))?
            .with_plugin(Arc::new(scroll_card::ScrollCard))
    }
}
struct Profile(u8);
fn button(label: &'static str, value: u8, context: &sdk::RenderContext) -> gpui::AnyElement {
    let pointer = context.events().clone();
    let keyboard = pointer.clone();
    sdk::base::Button::new(label)
        .px_2()
        .py_1()
        .border_1()
        .rounded_md()
        .bg(gpui::rgb(0x4558c9))
        .text_color(gpui::rgb(0xffffff))
        .aria_label(label)
        .child(label)
        .on_click(move |event, _, _| {
            let emit = || pointer.emit(vec![value]);
            let _ = match event {
                gpui::ClickEvent::Keyboard(_) => pointer.guard(emit),
                gpui::ClickEvent::Mouse(_) | gpui::ClickEvent::Touch(_) => {
                    pointer.guard_pointer(emit)
                }
            };
        })
        .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
            let _ = keyboard.guard(|| keyboard.emit(vec![value]));
        })
        .into_any_element()
}
impl sdk::ActionRenderer for Profile {
    fn code(
        &self,
        _: &sdk::CodeBlock,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<Option<gpui::AnyElement>, sdk::Error> {
        Ok(Some(button("Inspect highlighted code", 1, context)))
    }
    fn table(
        &self,
        _: &sdk::TableData,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<Option<gpui::AnyElement>, sdk::Error> {
        Ok(Some(button("Summarize native table", 2, context)))
    }
}
impl sdk::Highlighter for Profile {
    fn highlight(
        &self,
        code: &sdk::Code<'_>,
        _: &sdk::PrepareContext<'_>,
    ) -> Result<Vec<sdk::Highlight>, sdk::Error> {
        let mut highlight = sdk::Highlight::new(
            0..code.text().len(),
            if self.0 == 0 {
                [110, 125, 240]
            } else {
                [205, 130, 65]
            },
        );
        highlight.weight = 600;
        Ok(if code.text().is_empty() {
            vec![]
        } else {
            vec![highlight]
        })
    }
}
impl sdk::Plugin for Profile {
    fn name(&self) -> &'static str {
        "example.badge"
    }
    fn is_block(&self) -> bool {
        false
    }
    fn parse(
        &self,
        node: &sdk::markdown_ast::Node,
        cx: &sdk::MarkdownParseContext<'_>,
        _: &sdk::PrepareContext<'_>,
    ) -> Result<Option<sdk::MarkdownNode>, sdk::Error> {
        Ok(match node {
            sdk::markdown_ast::Node::InlineCode(code) if code.value == "review" => Some(
                sdk::MarkdownNode::new("example.badge", ())
                    .text("Review details")
                    .markdown(cx.node_source(node).unwrap_or("").to_owned()),
            ),
            _ => None,
        })
    }
    fn presentation(&self, _: &sdk::MarkdownNode) -> Result<sdk::MarkdownPresentation, sdk::Error> {
        Ok(sdk::MarkdownPresentation::NonText)
    }
    fn render(
        &self,
        _: &sdk::MarkdownNode,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, sdk::Error> {
        Ok(button("Review details", 3, context))
    }
    fn render_inline(
        &self,
        node: &sdk::MarkdownNode,
        _: &sdk::InlineRenderContext,
        context: &sdk::RenderContext,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Result<Option<sdk::InlineElement>, sdk::Error> {
        Ok(Some(sdk::InlineElement::new(
            self.render(node, context, window, cx)?,
        )))
    }
}

struct Card;
impl sdk::Plugin for Card {
    fn name(&self) -> &'static str {
        "example.card"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &sdk::markdown_ast::Node,
        cx: &sdk::MarkdownParseContext<'_>,
        _: &sdk::PrepareContext<'_>,
    ) -> Result<Option<sdk::MarkdownNode>, sdk::Error> {
        Ok(match node {
            sdk::markdown_ast::Node::Code(code) if code.lang.as_deref() == Some("review-card") => {
                Some(
                    sdk::MarkdownNode::new("example.card", ())
                        .text("Open review card")
                        .markdown(cx.node_source(node).unwrap_or("").to_owned()),
                )
            }
            _ => None,
        })
    }
    fn presentation(&self, _: &sdk::MarkdownNode) -> Result<sdk::MarkdownPresentation, sdk::Error> {
        Ok(sdk::MarkdownPresentation::NonText)
    }
    fn render(
        &self,
        _: &sdk::MarkdownNode,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, sdk::Error> {
        Ok(button("Open review card", 4, context))
    }
    fn render_inline(
        &self,
        _: &sdk::MarkdownNode,
        _: &sdk::InlineRenderContext,
        _: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<Option<sdk::InlineElement>, sdk::Error> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdk::Factory as _;
    use std::sync::atomic::AtomicBool;
    #[test]
    fn schema_properties_are_exact_and_configuration_checks_cancellation() {
        let factory = Factory;
        for byte in 0..=255u8 {
            assert_eq!(factory.validate_properties(&[byte]).is_ok(), byte <= 1);
        }
        for bytes in [vec![], vec![0, 0]] {
            assert_eq!(
                factory.validate_properties(&bytes),
                Err(sdk::Error::InvalidProperties)
            );
        }
        assert!(matches!(
            factory.configure(&[0], &sdk::PrepareContext::new(&|| true)),
            Err(sdk::Error::Cancelled)
        ));
        assert_eq!(factory.descriptor().max_event, 1);
    }
    #[test]
    fn real_preparation_installs_plugins_and_changes_highlight_palette() {
        let source = "Before `review` after.\n\n```review-card\nCard\n```\n\n```review-scroll\nChecklist\n```\n\n```ocaml\nlet answer = 42\n```\n";
        let mut colors = Vec::new();
        for accent in [0, 1] {
            let profile = Factory
                .configure(&[accent], &sdk::PrepareContext::new(&|| false))
                .unwrap();
            assert_eq!(profile.plugins().len(), 3);
            let prepared = sdk::prepare_markdown(
                Arc::new(profile),
                sdk::Preparation {
                    source,
                    base: sdk::base::text::MarkdownExtensions::default(),
                    parser_epoch: 7,
                    dark: true,
                    cancelled: Arc::new(AtomicBool::new(false)),
                },
            )
            .unwrap();
            assert_eq!(prepared.document.source().as_ref(), source);
            assert_eq!(prepared.parser_epoch, 7);
            assert_eq!(prepared.document.displayed_text().opaque_nodes(), 0);
            assert_eq!(prepared.document.code_blocks().len(), 1);
            assert_eq!(prepared.highlights.len(), 1);
            let runs = prepared.highlights.values().next().unwrap();
            assert_eq!(runs[0].weight, 600);
            colors.push(runs[0].foreground);
        }
        assert_ne!(colors[0], colors[1]);
    }
}
