//! Production owner and asynchronous bridge for measured flat panel groups.
use super::*;
use crate::split_group_widget as widget;

impl View {
    pub(super) fn sync_split_groups(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.split_groups.retain(|id, state| {
            let Some((tree, mount)) =
                tree.and_then(|tree| tree.get(*id)?.split_group.as_ref().map(|m| (tree, m)))
            else {
                state.borrow_mut().close(window, cx);
                return false;
            };
            let policy = widget::Policy {
                visible: self.focus.borrow().paint_visible(tree, *id),
                enabled: self.focus.borrow().allows(*id),
                pointer: pointer_enabled(tree, *id),
            };
            state
                .borrow_mut()
                .reconcile(mount.config.clone(), policy, window, cx)
                .expect("admitted split group");
            true
        });
        let Some(tree) = tree else {
            return;
        };
        for id in dirty {
            if let Some(mount) = tree.get(*id).and_then(|n| n.split_group.as_ref())
                && !self.split_groups.contains_key(id)
            {
                self.split_groups.insert(
                    *id,
                    widget::State::new(mount.config.clone(), window, cx)
                        .expect("admitted split group"),
                );
            }
        }
    }
    pub(super) fn begin_split_group_paint(&self) {
        let session = self.session.borrow();
        for (id, state) in &self.split_groups {
            state.borrow_mut().begin_frame();
            if let Some(node) = session.tree(self.id).and_then(|tree| tree.get(*id)) {
                for panel in node.children.iter() {
                    self.focus.borrow_mut().track_card_clipped(*panel, true);
                }
            }
        }
    }
    pub(super) fn finish_split_group_paint(&self, window: &mut Window, cx: &mut App) {
        for state in self.split_groups.values() {
            state.borrow_mut().finish_frame(window, cx);
            if state.borrow().contains_focused(window, cx)
                && let Some(focused) = window.focused(cx)
                && !self.focus.borrow().can_focus(&focused, window)
            {
                window.blur(cx);
            }
        }
    }
    pub(super) fn cancel_group_drags(&self, window: &mut Window) -> bool {
        let mut cancelled = false;
        for state in self.split_groups.values() {
            let mut state = state.borrow_mut();
            cancelled |= state.is_dragging();
            state.cancel(window);
        }
        cancelled
    }
    pub(super) fn close_split_groups(&mut self, window: &mut Window, cx: &mut App) {
        for state in self.split_groups.values() {
            state.borrow_mut().close(window, cx);
        }
        self.split_groups.clear();
    }
    pub(super) fn split_group_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mount = node.split_group.as_ref().expect("admitted group");
        let state = self
            .split_groups
            .get(&node.id)
            .expect("synchronized group")
            .clone();
        state
            .borrow_mut()
            .reconcile(
                mount.config.clone(),
                widget::Policy {
                    visible: self.focus.borrow().paint_visible(tree, node.id),
                    enabled: self.focus.borrow().allows(node.id),
                    pointer: interaction.pointer && pointer_enabled(tree, node.id),
                },
                window,
                cx,
            )
            .expect("admitted group");
        // Build every retained child, including hidden panels/grips. Widget
        // measurement chooses paint, while native editor/asset owners survive.
        let contents = node
            .children
            .iter()
            .map(|id| {
                let panel = tree.get(*id).expect("admitted panel slot");
                let content = tree.get(panel.children[0]).expect("admitted content slot");
                let grip = tree.get(panel.children[1]).expect("admitted grip slot");
                widget::Content {
                    panel: self.element(tree, content.children[0], interaction, window, cx),
                    handle: grip.children.first().map(|id| {
                        self.element(
                            tree,
                            *id,
                            Interaction {
                                pointer: false,
                                selectable: Some(false),
                                ..interaction
                            },
                            window,
                            cx,
                        )
                    }),
                }
            })
            .collect();
        let session = self.session.clone();
        let transport = self.transport.clone();
        let gate = self.focus.clone();
        let window_id = self.id;
        let id = node.id;
        let handler = node.handler;
        let revision = tree.revision();
        let config = mount.config.clone();
        let focus = self.focus.clone();
        let focus_config = config.clone();
        let visibility = self.focus.clone();
        let panel_ids: Vec<_> = config
            .panels
            .iter()
            .zip(node.children.iter())
            .map(|(p, id)| (p.id.clone(), *id))
            .collect();
        let panel_focus = self.focus.clone();
        let panel_clips = panel_ids.clone();
        let element = widget::element(widget::Render {
            state,
            contents,
            appearance: mount.appearance.clone(),
            observe: Rc::new(move |snapshot, _, _| {
                let allowed = match snapshot.source {
                    gpuio_protocol::split_group::Source::Request(_) => session
                        .borrow()
                        .tree(window_id)
                        .is_some_and(|tree| gate.borrow().paint_visible(tree, id)),
                    _ => gate.borrow().allows(id),
                };
                if !allowed {
                    return;
                }
                let event = handler.and_then(|handler| {
                    session
                        .borrow()
                        .split_group_resized(window_id, id, handler, revision, &config, snapshot)
                });
                if let Some(event) = event
                    && !transport.input(event)
                    && session.borrow_mut().overload(window_id)
                {
                    transport.fault(window_id);
                }
            }),
            decorate_panel: Rc::new(move |id, panel| {
                let node = panel_clips
                    .iter()
                    .find(|(key, _)| key == id)
                    .expect("admitted panel")
                    .1;
                highlight_style::Frame::clip(panel, node, &panel_focus).into_any_element()
            }),
            record_visibility: Rc::new(move |id, shown, _, _| {
                if let Some((_, node)) = panel_ids.iter().find(|(key, _)| key == id) {
                    visibility.borrow_mut().track_card_clipped(*node, !shown);
                }
            }),
            record_focus: Rc::new(move |after, handle, bounds, w, _| {
                if let Some(index) = focus_config.panels.iter().position(|p| p.id == after) {
                    focus.borrow_mut().record_part(
                        id,
                        index as u16 + 1,
                        focus::Target {
                            handle: handle.clone(),
                            tab_stop: true,
                            bounds,
                        },
                        handle.is_focused(w),
                    );
                }
            }),
        })
        .expect("admitted group presentation");
        highlight_style::Frame::clip(
            div()
                .id("split-group-clip")
                .size_full()
                .overflow_hidden()
                .child(element),
            id,
            &self.focus,
        )
        .into_any_element()
    }
}
