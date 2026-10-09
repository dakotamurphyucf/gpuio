//! Render labeled palette sections with the original slot focus and edit owner.
use super::*;
impl ColorInput {
    pub(super) fn palette(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let p = &self.presentation;
        if p.sections.is_empty() {
            return self.palette_row(0, self.model.config().palette.len(), false, window, cx);
        }
        let mut root = div().flex().flex_col().gap(px(p.section_gap as f32));
        let mut start = 0;
        for (index, section) in p.sections.iter().enumerate() {
            root = root.child(
                div()
                    .id(("palette-section", index))
                    .flex()
                    .flex_col()
                    .gap(px(p.swatch_gap as f32))
                    .role(Role::Group)
                    .aria_label(section.label.clone())
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(section.label.clone()),
                    )
                    .child(self.palette_row(
                        start,
                        section.count as usize,
                        section.featured,
                        window,
                        cx,
                    )),
            );
            start += section.count as usize;
        }
        root
    }
    fn palette_row(
        &self,
        start: usize,
        count: usize,
        featured: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let config = self.model.config();
        let snapshot = self.model.snapshot();
        let p = &self.presentation;
        let enabled = !config.disabled && self.access(false) == Access::Allowed;
        let editable = enabled && !config.read_only;
        let mut palette = div().flex().flex_wrap().gap(px(p.swatch_gap as f32));
        for (index, entry) in config.palette.iter().enumerate().skip(start).take(count) {
            let preview_entry = entry.clone();
            let handler = self.route.handler;
            let value = Value::Color(entry.color);
            let allowed = editable && config.allows(value);
            let focus = self.palette_focus[index]
                .clone()
                .tab_stop(enabled && config.allows(value));
            let selected = snapshot.value == value;
            let mut swatch = div()
                .id(("swatch", index))
                .relative()
                .size(px(if featured {
                    p.featured_size
                } else {
                    p.swatch_size
                } as f32))
                .rounded(px(p.swatch_radius as f32))
                .overflow_hidden()
                .border(px(p.outline_width as f32))
                .border_color(if selected || focus.is_focused(window) {
                    p.selected_border
                        .map_or(window.text_style().color, |c| rgba(c as u32).into())
                } else if self.palette_preview == Some((index, entry.color)) {
                    p.hover_border
                        .map_or(window.text_style().color, |c| rgba(c as u32).into())
                } else {
                    transparent_black()
                })
                .child(swatch(value, p.swatch_radius as f32))
                .role(Role::RadioButton)
                .aria_label(entry.label.clone())
                .aria_selected(selected)
                .aria_toggled(if selected {
                    Toggled::True
                } else {
                    Toggled::False
                })
                .when(enabled && config.allows(value), |this| {
                    this.track_focus(&focus)
                })
                .child(self.record(
                    &focus,
                    4 + index as u16,
                    enabled && config.allows(value),
                    false,
                ))
                .on_hover(cx.listener(move |s, hovered, _, cx| {
                    s.hover_palette(index, &preview_entry, handler, *hovered, cx);
                }));
            if allowed {
                swatch = swatch.cursor_pointer().on_click(cx.listener(
                    move |s, event: &ClickEvent, w, cx| {
                        s.choose(
                            value,
                            c::Source::Palette,
                            !matches!(event, ClickEvent::Keyboard(_)),
                            w,
                            cx,
                        );
                        cx.stop_propagation();
                    },
                ));
            }
            palette = palette.child(crate::semantics::State {
                identity: None,
                busy: false,
                element: swatch,
                metadata: None,
                live: None,
                hidden: false,
                disabled: !enabled || !config.allows(value),
                read_only: config.read_only,
                modal: false,
            });
        }
        palette
    }
}
