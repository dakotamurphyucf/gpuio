//! Statically linked profile of the TestPlatform executable only.
use gpuio_document_sdk as sdk;
use sdk::gpui::{self, IntoElement, ParentElement, Styled, prelude::*};
use std::sync::Arc;
pub(crate) const NAME: &str = "test.native_document";
pub(crate) const FINGERPRINT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(crate) fn factories() -> Vec<Arc<dyn sdk::Factory>> {
    vec![Arc::new(Factory)]
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
            max_event: 16,
            max_retained_bytes: 65536,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        if bytes.len() == 1 && bytes[0] <= 9 {
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
        assert_ne!(bytes[0], 2, "fixture configuration failure");
        let profile = Arc::new(Profile(bytes[0]));
        let configured = sdk::Profile::new()
            .with_actions(profile.clone())
            .with_highlighter(profile.clone())
            .with_plugin(profile)?;
        if bytes[0] >= 5 {
            configured.with_plugin(Arc::new(Card(bytes[0])))
        } else {
            Ok(configured)
        }
    }
}
struct Profile(u8);
fn button(label: &'static str, value: u8, context: &sdk::RenderContext) -> gpui::AnyElement {
    let pointer = context.events().clone();
    let keyboard = pointer.clone();
    gpui_base::Button::new(label)
        .px_2()
        .py_1()
        .border_1()
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
        assert_ne!(self.0, 1, "fixture action renderer failure");
        Ok(Some(button("Profile code", 7, context)))
    }
    fn table(
        &self,
        _: &sdk::TableData,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<Option<gpui::AnyElement>, sdk::Error> {
        Ok(Some(button("Profile table", 8, context)))
    }
}
impl sdk::Highlighter for Profile {
    fn highlight(
        &self,
        code: &sdk::Code<'_>,
        _: &sdk::PrepareContext<'_>,
    ) -> Result<Vec<sdk::Highlight>, sdk::Error> {
        if self.0 == 3 {
            return Err(sdk::Error::Highlight);
        }
        let mut highlight = sdk::Highlight::new(0..code.text().len(), [20, 150, 80]);
        highlight.weight = 550;
        highlight.strikethrough = true;
        Ok(if code.text().is_empty() {
            vec![]
        } else {
            vec![highlight]
        })
    }
}
impl sdk::Plugin for Profile {
    fn name(&self) -> &'static str {
        "test.badge"
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
            sdk::markdown_ast::Node::InlineCode(code) if code.value == "badge" => Some(
                sdk::MarkdownNode::new("test.badge", ())
                    .text("Profile badge")
                    .markdown(cx.node_source(node).unwrap_or("").to_owned()),
            ),
            _ => None,
        })
    }
    fn presentation(
        &self,
        node: &sdk::MarkdownNode,
    ) -> Result<sdk::MarkdownPresentation, sdk::Error> {
        Ok(presentation(self.0, node))
    }
    fn render(
        &self,
        _: &sdk::MarkdownNode,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, sdk::Error> {
        assert_ne!(self.0, 4, "fixture plugin render failure");
        Ok(button("Profile badge", 9, context))
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

struct Card(u8);
impl sdk::Plugin for Card {
    fn name(&self) -> &'static str {
        "test.card"
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
            sdk::markdown_ast::Node::Code(code) if code.lang.as_deref() == Some("card") => Some(
                sdk::MarkdownNode::new("test.card", ())
                    .text(if self.0 == 8 {
                        code.value.clone()
                    } else {
                        "Profile card".into()
                    })
                    .markdown(cx.node_source(node).unwrap_or("").to_owned()),
            ),
            _ => None,
        })
    }
    fn presentation(
        &self,
        node: &sdk::MarkdownNode,
    ) -> Result<sdk::MarkdownPresentation, sdk::Error> {
        Ok(presentation(self.0, node))
    }
    fn render(
        &self,
        _: &sdk::MarkdownNode,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, sdk::Error> {
        if self.0 == 9 {
            // Deliberately independent scroll ownership: the reader may reveal
            // this block, but cannot scroll the plugin's clipped descendants.
            return Ok(gpui::div()
                .id("profile-scroll")
                .w(gpui::px(260.))
                .h(gpui::px(80.))
                .overflow_y_scroll()
                .child(
                    gpui::div()
                        .flex()
                        .flex_col()
                        .child(button("Profile scroll top", 11, context))
                        .child(gpui::div().h(gpui::px(240.)).flex_none())
                        .child(button("Profile scroll bottom", 12, context)),
                )
                .into_any_element());
        }
        Ok(button("Profile card", 10, context))
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

// Modes 6/7 exercise mounted Text/Opaque contracts; earlier modes keep controls.
fn presentation(mode: u8, node: &sdk::MarkdownNode) -> sdk::MarkdownPresentation {
    match mode {
        6 | 8 => sdk::MarkdownPresentation::Text(node.as_text().to_owned().into()),
        7 => sdk::MarkdownPresentation::Opaque,
        _ => sdk::MarkdownPresentation::NonText,
    }
}
