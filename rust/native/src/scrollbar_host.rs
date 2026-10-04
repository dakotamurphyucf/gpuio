//! Shared viewport presentation; scroll offsets remain in their existing owners.
use super::*;
use crate::scrollbar_widget as widget;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Owner {
    Viewport,
    TableHorizontal,
    TableVertical,
}
impl Owner {
    fn name(self) -> &'static str {
        match self {
            Self::Viewport => "viewport-scrollbar",
            Self::TableHorizontal => "table-horizontal-scrollbar",
            Self::TableVertical => "table-vertical-scrollbar",
        }
    }
    fn eligible(self, node: &crate::tree::Node) -> bool {
        use gpuio_protocol::scrollbar::Axis;
        let Some(config) = node.scrollbar.as_ref() else {
            return false;
        };
        match self {
            Self::Viewport => {
                node.table.is_none()
                    && node
                        .list_config
                        .as_ref()
                        .map_or_else(|| scroll::declared(&node.style), |c| c.scrollbar)
            }
            Self::TableHorizontal => {
                node.table.as_ref().is_some_and(|t| t.scrollbar)
                    && matches!(config.axis, Axis::Both | Axis::Horizontal)
            }
            Self::TableVertical => {
                node.table.as_ref().is_some_and(|t| t.scrollbar)
                    && matches!(config.axis, Axis::Both | Axis::Vertical)
            }
        }
    }
    fn config(self, node: &crate::tree::Node) -> Option<Arc<gpuio_protocol::scrollbar::Config>> {
        if !self.eligible(node) {
            return None;
        }
        let config = node.scrollbar.as_ref()?;
        let axis = match self {
            Self::Viewport => return Some(config.clone()),
            Self::TableHorizontal => gpuio_protocol::scrollbar::Axis::Horizontal,
            Self::TableVertical => gpuio_protocol::scrollbar::Axis::Vertical,
        };
        if config.axis == axis {
            return Some(config.clone());
        }
        let mut config = (**config).clone();
        config.axis = axis;
        Some(Arc::new(config))
    }
}
pub(super) struct Mount {
    pub kind: Owner,
    pub handle: Rc<dyn gpui_base::ScrollbarHandle>,
}

impl View {
    pub(super) fn cancel_scrollbar_drags(&self, window: &mut Window, cx: &mut App) -> bool {
        let mut cancelled = false;
        for state in self.scrollbars.values() {
            let mut state = state.borrow_mut();
            if state.is_dragging() {
                state.cancel(window, cx);
                cancelled = true;
            }
        }
        cancelled
    }
    pub(super) fn sync_scrollbars(&mut self, window: &mut Window, cx: &mut App) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        self.scrollbars.retain(|(id, kind), state| {
            let Some((tree, config)) =
                tree.and_then(|tree| Some((tree, kind.config(tree.get(*id)?)?)))
            else {
                state.borrow_mut().close(window, cx);
                return false;
            };
            state
                .borrow_mut()
                .reconcile(
                    config.clone(),
                    widget::Policy {
                        visible: self.focus.borrow().paint_visible(tree, *id),
                        enabled: self.focus.borrow().allows(*id),
                        pointer: pointer_enabled(tree, *id),
                    },
                    window,
                    cx,
                )
                .expect("admitted scrollbar");
            true
        });
    }
    pub(super) fn begin_scrollbar_paint(&self) {
        for state in self.scrollbars.values() {
            state.borrow_mut().begin_frame();
        }
    }
    pub(super) fn finish_scrollbar_paint(&self, window: &mut Window, cx: &mut App) {
        for state in self.scrollbars.values() {
            state.borrow_mut().finish_frame(window, cx);
        }
    }
    pub(super) fn close_scrollbars(&mut self, window: &mut Window, cx: &mut App) {
        for state in self.scrollbars.values() {
            state.borrow_mut().close(window, cx);
        }
        self.scrollbars.clear();
    }
    pub(super) fn scrollbar_owner(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        mount: Mount,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<widget::Shared> {
        let config = mount.kind.config(node)?;
        let kind = mount.kind;
        let id = node.id;
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        let state = if let Some(state) = self.scrollbars.get(&(id, kind)) {
            state.clone()
        } else {
            let state = widget::State::new(
                (kind.name(), identity).into(),
                config.clone(),
                mount.handle,
                if kind == Owner::Viewport {
                    widget::Viewport::Handle
                } else {
                    widget::Viewport::Layout
                },
                window,
                cx,
            )
            .expect("admitted scrollbar and native metrics");
            let gate = self.focus.clone();
            let session = self.session.clone();
            let window_id = self.id;
            let focus = self.focus.clone();
            state.borrow_mut().set_hooks(widget::Hooks {
                allowed: Rc::new(move |pointer| {
                    let session = session.borrow();
                    session.tree(window_id).is_some_and(|tree| {
                        tree.get(id).is_some_and(|n| kind.eligible(n))
                            && gate.borrow().allows(id)
                            && (!pointer || pointer_enabled(tree, id))
                    })
                }),
                record_focus: Rc::new(move |axis, handle, bounds, window| {
                    let part = match axis {
                        crate::scrollbar_geometry::Axis::Horizontal => u16::MAX - 1,
                        crate::scrollbar_geometry::Axis::Vertical => u16::MAX,
                    };
                    focus.borrow_mut().record_part(
                        id,
                        part,
                        focus::Target {
                            handle: handle.clone(),
                            tab_stop: true,
                            bounds,
                        },
                        handle.is_focused(window),
                    );
                }),
            });
            self.scrollbars.insert((id, kind), state.clone());
            state
        };
        state
            .borrow_mut()
            .reconcile(
                config.clone(),
                widget::Policy {
                    visible: self.focus.borrow().paint_visible(tree, id),
                    enabled: self.focus.borrow().allows(id),
                    pointer: interaction.pointer && pointer_enabled(tree, id),
                },
                window,
                cx,
            )
            .expect("admitted scrollbar");
        Some(state)
    }
}
