//! Current-frame file controls beside original selectable diff header text.
use super::*;
use gpui_base::ElementExt as _;
impl Presentation {
    pub(super) fn install_file_headers(
        &mut self,
        same_generation: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let old = std::mem::take(&mut self.file_buttons);
        let had_focus = old.values().any(|(_, focus)| focus.is_focused(window));
        self.file_visible.borrow_mut().clear();
        let mut entries = BTreeMap::new();
        let mut next_buttons = BTreeMap::new();
        let action =
            self.installed
                .clone()
                .zip(self.projected_page.clone())
                .map(|(snapshot, page)| DiffAction {
                    snapshot,
                    page,
                    epoch: self.diff_epoch,
                    handler: self.diff_handler,
                });
        if let (Some(projection), Some(diff), Some(controls), Some(action)) = (
            self.active_projection(),
            &self.diff,
            &self.diff_controls,
            action,
        ) {
            let origin = projection.text()[..self.installed_page_start]
                .bytes()
                .filter(|b| *b == b'\n')
                .count();
            for (display_row, row) in projection.rows().iter().enumerate() {
                if row.display.start < self.installed_page_start || row.display.end > self.page_end
                {
                    continue;
                }
                let Some(index) = diff.lines[row.source_line].file else {
                    continue;
                };
                let file = &diff.files[index];
                if row.source_line != file.lines.start {
                    continue;
                }
                let path = file.after_path.clone().or(file.before_path.clone());
                let focus = old
                    .get(&index)
                    .filter(|(old_path, _)| same_generation && old_path == &path)
                    .map_or_else(|| cx.focus_handle(), |(_, focus)| focus.clone());
                let collapsed = controls.is_collapsed(file);
                let name = path.as_deref().unwrap_or("Unnamed file");
                let label = format!(
                    "{} file {name}",
                    if collapsed { "Expand" } else { "Collapse" }
                );
                let status = if file.binary {
                    "Binary"
                } else if file.before_path.is_none() && file.after_path.is_some() {
                    "Added"
                } else if file.after_path.is_none() && file.before_path.is_some() {
                    "Deleted"
                } else {
                    "Changed"
                };
                let details = format!("{status} · +{} −{}", file.added, file.removed);
                let renamed = file.before_path != file.after_path
                    && file.before_path.is_some()
                    && file.after_path.is_some();
                let caption = if renamed {
                    format!("→ {name} · {details}")
                } else {
                    details.clone()
                };
                let visible = self.file_visible.clone();
                let action = action.clone();
                let owner = cx.weak_entity();
                let focus_for_button = focus.clone();
                let dark = self.config.dark;
                let gutter = Rc::new(
                    move |slot: gpui::Size<gpui::Pixels>, _: &mut Window, _: &mut App| {
                        let click_owner = owner.clone();
                        let ax_owner = owner.clone();
                        let click_action = action.clone();
                        let ax_action = action.clone();
                        let visible = visible.clone();
                        gpui_base::Button::new(("diff-file", index))
                            .w(slot.width)
                            .h(slot.height)
                            .p_0()
                            .cursor_pointer()
                            .rounded(px(3.))
                            .hover(move |style| {
                                style.bg(gpui::rgb(if dark { 0x2b303b } else { 0xe5e8ed }))
                            })
                            .focus_visible(move |style| {
                                style.bg(gpui::rgb(if dark { 0x324563 } else { 0xd8e7ff }))
                            })
                            .track_focus(&focus_for_button)
                            .aria_label(label.clone())
                            .aria_expanded(!collapsed)
                            .child(if collapsed { "▸" } else { "▾" })
                            .on_prepaint(move |bounds, _, _| {
                                visible.borrow_mut().insert(index, bounds);
                            })
                            .on_click(move |_, window, cx| {
                                let _ = click_owner.update(cx, |this, cx| {
                                    this.toggle_diff_file(&click_action, index, window, cx)
                                });
                            })
                            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                                let _ = ax_owner.update(cx, |this, cx| {
                                    this.toggle_diff_file(&ax_action, index, window, cx)
                                });
                                cx.stop_propagation();
                            })
                            .into_any_element()
                    },
                );
                let suffix = Rc::new(
                    move |slot: gpui::Size<gpui::Pixels>, _: &mut Window, _: &mut App| {
                        div()
                            .w(slot.width)
                            .h(slot.height)
                            .pl(px(10.))
                            .text_size(px(11.))
                            .text_color(gpui::rgb(if dark { 0x969dad } else { 0x656a76 }))
                            .text_ellipsis()
                            .child(caption.clone())
                            .into_any_element()
                    },
                );
                let local_row = display_row - origin;
                entries.insert(
                    local_row,
                    gpui_base::input::RowAdornment {
                        gutter: Some(gutter),
                        suffix: Some(suffix),
                        suffix_width: px(240.),
                    },
                );
                next_buttons.insert(index, (path, focus));
            }
        }
        self.file_buttons = next_buttons;
        let begin = self.file_visible.clone();
        self.editor.update(cx, |editor, cx| {
            let set = (!entries.is_empty()).then(|| Rc::new(entries));
            editor
                .set_row_adornments(set, Some(Rc::new(move || begin.borrow_mut().clear())), cx)
                .expect("bounded installed read-only diff page");
        });
        if had_focus
            && !self
                .file_buttons
                .values()
                .any(|(_, focus)| focus.is_focused(window))
        {
            window.focus(&self.primary_focus(cx), cx);
        }
    }
    pub(super) fn toggle_diff_file(
        &mut self,
        action: &DiffAction,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.accepts_diff_action(action, cx) || !self.file_visible.borrow().contains_key(&index)
        {
            return;
        }
        let Some(observation) = self
            .diff_controls
            .as_mut()
            .and_then(|controls| controls.toggle_file(self.diff.as_ref()?, index))
        else {
            return;
        };
        let applied = matches!(
            &observation,
            gpuio_protocol::document_diff::Observation::ToggleFile { applied: true, .. }
        );
        self.emit_diff(action, observation, cx);
        if applied {
            self.rebuild_diff(action.snapshot.clone(), window, cx);
            cx.notify();
        }
    }
}
