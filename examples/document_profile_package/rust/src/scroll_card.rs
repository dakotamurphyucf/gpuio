//! A bounded native viewport with keyboard-accessible scroll alternatives.
use super::{button, sdk};
use sdk::gpui::{self, IntoElement, ParentElement, Styled, prelude::*};

pub(super) struct ScrollCard;
impl sdk::Plugin for ScrollCard {
    fn name(&self) -> &'static str {
        "example.scroll_card"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &sdk::markdown_ast::Node,
        context: &sdk::MarkdownParseContext<'_>,
        _: &sdk::PrepareContext<'_>,
    ) -> Result<Option<sdk::MarkdownNode>, sdk::Error> {
        let sdk::markdown_ast::Node::Code(code) = node else {
            return Ok(None);
        };
        if code.lang.as_deref() != Some("review-scroll") {
            return Ok(None);
        }
        let offset = context.offset()
            + code
                .position
                .as_ref()
                .map_or(0, |position| position.start.offset);
        Ok(Some(
            sdk::MarkdownNode::new(self.name(), offset)
                .text("Scrollable review checklist")
                .markdown(context.node_source(node).unwrap_or("").to_owned()),
        ))
    }
    fn presentation(&self, _: &sdk::MarkdownNode) -> Result<sdk::MarkdownPresentation, sdk::Error> {
        Ok(sdk::MarkdownPresentation::NonText)
    }
    fn render(
        &self,
        node: &sdk::MarkdownNode,
        context: &sdk::RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, sdk::Error> {
        let occurrence = *node.data::<usize>().ok_or(sdk::Error::InvalidPlugin)?;
        Ok(ScrollCardElement {
            occurrence,
            context: context.clone(),
        }
        .into_any_element())
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

#[derive(gpui::IntoElement)]
struct ScrollCardElement {
    occurrence: usize,
    context: sdk::RenderContext,
}

fn scroll_button(
    label: &'static str,
    end: bool,
    scroll: &gpui::ScrollHandle,
    context: &sdk::RenderContext,
) -> gpui::AnyElement {
    let scroll = scroll.clone();
    let accessible_scroll = scroll.clone();
    let pointer = context.events().clone();
    let keyboard = pointer.clone();
    let change = move |window: &mut gpui::Window| {
        if end {
            scroll.scroll_to_bottom();
        } else {
            scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
        }
        window.refresh();
        Ok(())
    };
    sdk::base::Button::new(label)
        .px_2()
        .py_1()
        .border_1()
        .rounded_md()
        .bg(gpui::rgb(0x243349))
        .text_color(gpui::rgb(0xffffff))
        .aria_label(label)
        .child(label)
        .on_click(move |event, window, _| {
            let _ = match event {
                gpui::ClickEvent::Keyboard(_) => pointer.guard(|| change(window)),
                gpui::ClickEvent::Mouse(_) | gpui::ClickEvent::Touch(_) => {
                    pointer.guard_pointer(|| change(window))
                }
            };
        })
        .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, _| {
            let _ = keyboard.guard(|| {
                if end {
                    accessible_scroll.scroll_to_bottom();
                } else {
                    accessible_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
                }
                window.refresh();
                Ok(())
            });
        })
        .into_any_element()
}

impl gpui::RenderOnce for ScrollCardElement {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        // RenderOnce executes in the document's element namespace. Distinguish
        // parsed occurrences and source resets; retain no worker/Window handle.
        let key: gpui::SharedString = format!(
            "review-scroll-{}-{}",
            self.context.source().generation(),
            self.occurrence
        )
        .into();
        let scroll = window
            .use_keyed_state(key.clone(), cx, |_, _| gpui::ScrollHandle::new())
            .read(cx)
            .clone();
        gpui::div()
            .id(key)
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_1()
            .rounded_md()
            .child(
                gpui::div()
                    .flex()
                    .gap_2()
                    .child(scroll_button("Show review start", false, &scroll, &self.context))
                    .child(scroll_button("Show review end", true, &scroll, &self.context)),
            )
            .child(
                gpui::div()
                    .id("review-viewport")
                    .role(gpui::accesskit::Role::Group)
                    .aria_label("Review checklist viewport")
                    .h(gpui::px(150.))
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child("Review starts here")
                            .children([
                                "01 · Confirm the requested behavior and the evidence needed to verify it.",
                                "02 · Review the implementation, ownership and cancellation boundaries.",
                                "03 · Check the results, then open the review when you are ready.",
                            ].map(|text| gpui::div().min_h(gpui::px(70.)).flex_none().child(text)))
                            .child(button("Open scroll review", 4, &self.context)),
                    ),
            )
    }
}
