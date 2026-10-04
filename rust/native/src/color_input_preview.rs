//! Transient native inspection. Hover never writes the color model or editors.
use super::*;

impl ColorInput {
    fn preview_allowed(&self, index: usize, color: Rgba) -> bool {
        let config = self.model.config();
        self.shows_panel(Panel::Palette)
            && self.access(true) == Access::Allowed
            && !config.disabled
            && !config.read_only
            && self.capture.is_none()
            && config.allows(Value::Color(color))
            && config
                .palette
                .get(index)
                .is_some_and(|entry| entry.color == color)
    }

    pub(super) fn clear_palette_preview(&mut self, cx: &mut Context<Self>) {
        if self.palette_preview.take().is_some() {
            cx.notify();
        }
    }

    pub(super) fn validate_palette_preview(&mut self, cx: &mut Context<Self>) {
        if self
            .palette_preview
            .is_some_and(|(index, color)| !self.preview_allowed(index, color))
        {
            self.clear_palette_preview(cx);
        }
    }

    pub(super) fn hover_palette(
        &mut self,
        index: usize,
        entry: &c::PaletteEntry,
        handler: HandlerId,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        if self.route.handler != handler || self.model.config().palette.get(index) != Some(entry) {
            return;
        }
        // A late leave from the previous swatch must not erase the new one.
        let candidate = (index, entry.color);
        if !hovered {
            if self.palette_preview == Some(candidate) {
                self.clear_palette_preview(cx);
            }
            return;
        }
        if self.preview_allowed(index, entry.color) && self.palette_preview != Some(candidate) {
            self.palette_preview = Some(candidate);
            cx.notify();
        }
    }

    pub(super) fn displayed_color(&self) -> Value {
        self.palette_preview.map_or_else(
            || self.model.snapshot().value,
            |(_, color)| Value::Color(color),
        )
    }
}
