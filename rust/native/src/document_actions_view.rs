//! Per-picture native buttons. Captured content is immutable; callbacks enqueue.
use super::*;
use gpuio_protocol::document_actions::{
    Block, Config as ActionsConfig, Event as ActionEvent, SourceRange,
};

#[derive(Clone)]
pub(super) struct Owner {
    view: WeakEntity<Presentation>,
    interpretation: Arc<()>,
    revision: Option<(i64, i64)>,
    config: Option<Arc<ActionsConfig>>,
    markdown: bool,
    foreground: gpui::Hsla,
    accent: gpui::Hsla,
}
impl Presentation {
    pub(super) fn action_owner(&self, cx: &Context<Self>) -> Owner {
        Owner {
            view: cx.weak_entity(),
            interpretation: self.interpretation.clone(),
            revision: self.installed.as_ref().map(|s| (s.generation, s.revision)),
            config: self.actions_config.clone(),
            foreground: self.text_style.foreground(),
            accent: self.text_style.link(),
            markdown: matches!(self.config.mode, gpuio_protocol::document::Mode::Markdown),
        }
    }
    pub(super) fn refresh_actions(
        &mut self,
        config: Option<Arc<ActionsConfig>>,
        cx: &mut Context<Self>,
    ) {
        if self.actions_config != config {
            self.actions_config = config;
            if let Some(text) = &self.markdown {
                text.update(cx, |text, cx| text.invalidate_inline_layout(cx));
            }
            self.invalidate_row(cx);
            cx.notify();
        }
    }
}
impl Owner {
    fn epoch(&self) -> i64 {
        self.config.as_ref().map_or(0, |c| c.epoch)
    }
    pub(super) fn invoke(
        &self,
        action: Option<&str>,
        block: &Block,
        source_range: Option<SourceRange>,
        input: &gpui::ClickEvent,
        cx: &mut App,
    ) {
        let _ = self.view.update(cx, |view, cx| {
            if (action.is_some() && !block.is_valid())
                || !view.matches_interpretation(&self.interpretation)
                || view.actions_config.as_ref().map_or(0, |c| c.epoch) != self.epoch()
                || !view.allows_link_focus(self.revision, &cx.entity(), cx)
            {
                return;
            }
            let Some(root) = view.root.upgrade() else {
                return;
            };
            let delivery = root.read_with(cx, |root, _| {
                let session = root.session.borrow();
                let tree = session.tree(root.id)?;
                let node = tree.get(view.node)?;
                if node.document_actions.as_ref().map_or(0, |c| c.epoch) != self.epoch() {
                    return None;
                }
                if let Some(action) = action {
                    let config = node.document_actions.as_ref()?;
                    if !config.allows(action, block) {
                        return None;
                    }
                    let installed = view.installed.as_ref()?;
                    let event = ActionEvent {
                        config_epoch: self.epoch(),
                        action: action.to_owned(),
                        source_generation: installed.generation,
                        source_revision: installed.revision,
                        source_range,
                        block: block.clone(),
                        activation: link_activation(input),
                    };
                    if !event.is_valid() {
                        return None;
                    }
                    Some(Some(Event::DocumentAction(
                        root.id,
                        view.node,
                        node.handler?,
                        tree.revision(),
                        view.config.source?,
                        event,
                    )))
                } else {
                    let copy = node.document_actions.as_ref().is_none_or(|c| match block {
                        Block::Code(..) => c.copy_code,
                        Block::Table(..) => c.copy_table,
                    });
                    copy.then_some(None)
                }
            });
            match delivery {
                None => (),
                Some(None) => {
                    let text = match block {
                        Block::Code(_, code) => code,
                        Block::Table(_, _, markdown) => markdown,
                    };
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()));
                }
                Some(Some(event)) => {
                    root.update(cx, |root, _| {
                        if !root.transport.input(event)
                            && root.session.borrow_mut().overload(root.id)
                        {
                            root.transport.fault(root.id);
                        }
                    });
                }
            }
        });
    }
}
fn row(owner: &Owner, block: Block, source_range: Option<SourceRange>) -> gpui::AnyElement {
    let code = matches!(block, Block::Code(..));
    let copy = owner
        .config
        .as_ref()
        .is_none_or(|c| if code { c.copy_code } else { c.copy_table });
    let actions = owner
        .config
        .as_ref()
        .map(|c| if code { &c.code } else { &c.table });
    let valid = block.is_valid();
    let block = Arc::new(block);
    let source_range = if owner.markdown { source_range } else { None };
    let button = |id: gpui::SharedString,
                  label: gpui::SharedString,
                  action: Option<String>,
                  enabled: bool| {
        let valid = action.is_none() || valid;
        let owner = owner.clone();
        let pointer_owner = owner.clone();
        let pointer_block = block.clone();
        let block = block.clone();
        let pointer_action = action.clone();
        let span = source_range.clone();
        let pointer_span = span.clone();
        let foreground = owner.foreground;
        let accent = owner.accent;
        gpui_base::Button::new(id)
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(foreground.opacity(0.25))
            .text_color(foreground)
            .text_sm()
            .cursor_pointer()
            .hover(move |style| style.bg(foreground.opacity(0.08)))
            .focus(move |style| style.border_color(accent))
            .styles(|styles| styles.disabled(|style| style.opacity(0.45)))
            .aria_label(label.clone())
            .child(label)
            .disabled(!enabled || !valid)
            .on_click(move |event, _, cx| {
                pointer_owner.invoke(
                    pointer_action.as_deref(),
                    &pointer_block,
                    pointer_span.clone(),
                    event,
                    cx,
                )
            })
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                if enabled && valid {
                    owner.invoke(
                        action.as_deref(),
                        &block,
                        span.clone(),
                        &gpui::ClickEvent::default(),
                        cx,
                    );
                }
                cx.stop_propagation();
            })
    };
    let mut row = div()
        .flex()
        .flex_wrap()
        .w_full()
        .min_w_0()
        .justify_end()
        .gap(px(8.));
    if copy {
        let label = if code { "Copy code" } else { "Copy table" };
        row = row.child(button(
            (if code { "copy-code" } else { "copy-table" }).into(),
            label.into(),
            None,
            true,
        ));
    }
    for action in actions.into_iter().flatten() {
        row = row.child(button(
            format!("gpuio-document-action-{}", action.id).into(),
            action.label.clone().into(),
            Some(action.id.clone()),
            action.enabled,
        ));
    }
    row.into_any_element()
}
pub(super) fn code(owner: &Owner, block: &gpui_base::text::CodeBlock) -> gpui::AnyElement {
    row(
        owner,
        Block::Code(
            block.lang().map(|s| s.to_string()),
            block.code().to_string(),
        ),
        block.span.map(|s| SourceRange {
            start_byte: s.start as i64,
            end_byte: s.end as i64,
        }),
    )
}
pub(super) fn table(owner: &Owner, table: &gpui_base::text::TableData) -> gpui::AnyElement {
    // Default native Copy needs only Markdown. Avoid cloning every cell on
    // each frame unless a configured custom action can consume the snapshot.
    let custom = owner.config.as_ref().is_some_and(|c| !c.table.is_empty());
    row(
        owner,
        Block::Table(
            if custom {
                table.headers.clone()
            } else {
                Vec::new()
            },
            if custom {
                table.rows.clone()
            } else {
                Vec::new()
            },
            table.markdown.clone(),
        ),
        table.span.as_ref().map(|s| SourceRange {
            start_byte: s.start as i64,
            end_byte: s.end as i64,
        }),
    )
}
