//! Native presentation switching around one color model and five retained editors.
use super::*;
use gpuio_protocol::color_presentation::{Panel, Panels, Presentation};

impl ColorInput {
    pub(super) fn shows_panel(&self, panel: Panel) -> bool {
        self.presentation.panels.shows(self.panel, panel)
    }
    fn panel_index(panel: Panel) -> usize {
        match panel {
            Panel::Palette => 0,
            Panel::Channels => 1,
        }
    }
    fn channels_focused(&self, window: &Window) -> bool {
        self.channel_focus
            .iter()
            .chain(self.editors.fields.iter().skip(1).map(|e| &e.focus))
            .any(|f| f.is_focused(window))
    }
    fn settle_channels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .model
            .snapshot()
            .interaction
            .is_some_and(|i| matches!(i.kind, c::InteractionKind::Drag(_)))
        {
            self.cancel(c::CancelReason::Interrupted, window, cx);
        }
        // Observe pending platform changes before disabling a hidden editor.
        for index in 1..self.editors.fields.len() {
            self.finish_editor(index, true, window, cx);
        }
        self.release(window);
        self.hitboxes = [None; 4];
    }
    fn restore_visible_focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.model.config().disabled || self.access(false) == Access::Blocked {
            window.blur(cx);
        } else if self.presentation.panels.is_tabbed() {
            window.focus(&self.tab_focus[Self::panel_index(self.panel)], cx);
        } else {
            window.focus(&self.editors.fields[0].focus, cx);
        }
    }
    pub(super) fn configure_presentation(
        &mut self,
        next: Arc<Presentation>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.presentation == next {
            return;
        }
        let panel = if !self.presentation.panels.is_tabbed() && next.panels.is_tabbed() {
            next.panels.initial()
        } else {
            self.panel
        };
        let hide_channels =
            self.shows_panel(Panel::Channels) && !next.panels.shows(panel, Panel::Channels);
        let hide_palette =
            self.shows_panel(Panel::Palette) && !next.panels.shows(panel, Panel::Palette);
        let move_focus = (hide_channels && self.channels_focused(window))
            || (hide_palette && self.palette_focus.iter().any(|f| f.is_focused(window)))
            || (!next.panels.is_tabbed() && self.tab_focus.iter().any(|f| f.is_focused(window)));
        if hide_channels {
            self.settle_channels(window, cx);
        }
        self.clear_palette_preview(cx);
        self.presentation = next;
        self.panel = panel;
        self.sync_editors(window, cx);
        if move_focus {
            self.restore_visible_focus(window, cx);
        }
        cx.notify();
    }
    pub(super) fn reveal_panel(
        &mut self,
        panel: Panel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.presentation.panels.is_tabbed() || self.panel == panel {
            return;
        }
        let move_focus = match panel {
            Panel::Palette => self.channels_focused(window),
            Panel::Channels => self.palette_focus.iter().any(|f| f.is_focused(window)),
        };
        if panel == Panel::Palette {
            self.settle_channels(window, cx);
        }
        self.clear_palette_preview(cx);
        self.panel = panel;
        self.sync_editors(window, cx);
        if move_focus {
            self.restore_visible_focus(window, cx);
        }
        cx.notify();
    }
    pub(super) fn activate_tab(
        &mut self,
        panel: Panel,
        handler: HandlerId,
        pointer: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.presentation.panels.is_tabbed()
            || self.route.handler != handler
            || self.model.config().disabled
            || self.access(pointer) != Access::Allowed
        {
            return;
        }
        if self.capture.is_some() {
            self.cancel(c::CancelReason::Interrupted, window, cx);
        }
        // GPUI may already have moved focus on mouse-down. Settle visible
        // editors by their pending native stamp/interaction, not current focus.
        // A configuration change, by contrast, leaves visible hex alone.
        for index in 0..self.editors.fields.len() {
            if index == 0 || self.shows_panel(Panel::Channels) {
                self.finish_editor(index, true, window, cx);
            }
        }
        if self.access(pointer) != Access::Allowed {
            return;
        }
        self.reveal_panel(panel, window, cx);
        self.clear_palette_preview(cx);
        window.focus(&self.tab_focus[Self::panel_index(panel)], cx);
        cx.notify();
    }
    pub(super) fn tab_bar(&self, window: &Window, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let Panels::Tabs {
            palette_label,
            channels_label,
            ..
        } = &self.presentation.panels
        else {
            return None;
        };
        let enabled = !self.model.config().disabled && self.access(false) == Access::Allowed;
        let handler = self.route.handler;
        let mut bar = div()
            .id("color-tabs")
            .flex()
            .gap(px(self.presentation.swatch_gap as f32))
            .role(Role::TabList)
            .aria_label(self.model.config().labels.control.clone())
            .aria_orientation(accesskit::Orientation::Horizontal);
        for (index, (panel, label)) in [
            (Panel::Palette, palette_label),
            (Panel::Channels, channels_label),
        ]
        .into_iter()
        .enumerate()
        {
            let selected = self.panel == panel;
            let focus = self.tab_focus[index].clone().tab_stop(enabled && selected);
            let mut tab = div()
                .id(("color-tab", index))
                .relative()
                .px(px(8.))
                .py(px(5.))
                .rounded(px(4.))
                .border_1()
                .border_color(if selected || focus.is_focused(window) {
                    self.presentation
                        .selected_border
                        .map_or(window.text_style().color, |c| rgba(c as u32).into())
                } else {
                    window.text_style().color.opacity(0.25)
                })
                .role(Role::Tab)
                .aria_label(label.clone())
                .aria_selected(selected)
                .child(label.clone())
                .when(enabled, |tab| tab.track_focus(&focus))
                .child(self.record(&focus, 266 + index as u16, enabled && selected, false));
            if enabled {
                tab = tab
                    .cursor_pointer()
                    .on_click(cx.listener(move |s, e: &ClickEvent, w, cx| {
                        s.activate_tab(
                            panel,
                            handler,
                            !matches!(e, ClickEvent::Keyboard(_)),
                            w,
                            cx,
                        );
                        cx.stop_propagation();
                    }))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        if e.keystroke.modifiers.modified() {
                            return;
                        }
                        let panel = match e.keystroke.key.as_str() {
                            "left" | "right" => {
                                if panel == Panel::Palette {
                                    Panel::Channels
                                } else {
                                    Panel::Palette
                                }
                            }
                            "home" => Panel::Palette,
                            "end" => Panel::Channels,
                            _ => return,
                        };
                        s.activate_tab(panel, handler, false, w, cx);
                        cx.stop_propagation();
                    }));
                // AX activation must not synthesize a pointer release into an
                // outstanding channel drag or inherit pointer-only policy.
                let click = cx.weak_entity();
                tab = tab.on_a11y_action(AccessibleAction::Click, move |_, w, cx| {
                    let _ = click.update(cx, |s, cx| s.activate_tab(panel, handler, false, w, cx));
                });
                let weak = cx.weak_entity();
                tab = tab.on_a11y_action(AccessibleAction::Focus, move |_, w, cx| {
                    let _ = weak.update(cx, |s, cx| s.activate_tab(panel, handler, false, w, cx));
                });
            }
            bar = bar.child(crate::semantics::State {
                identity: None,
                element: tab,
                metadata: None,
                live: None,
                busy: false,
                hidden: false,
                disabled: !enabled,
                read_only: false,
                modal: false,
            });
        }
        Some(bar)
    }
    pub(super) fn panel_label(&self, panel: Panel) -> Option<&str> {
        match &self.presentation.panels {
            Panels::All => None,
            Panels::Tabs {
                palette_label,
                channels_label,
                ..
            } => Some(match panel {
                Panel::Palette => palette_label,
                Panel::Channels => channels_label,
            }),
        }
    }
}
