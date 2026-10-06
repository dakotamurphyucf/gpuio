//! Structural palette slots: interactive chrome and passive measured row content.
use super::*;
pub(super) type RowContent =
    Rc<dyn Fn(usize, bool, &mut Window, &mut App) -> Option<gpui::AnyElement>>;

impl View {
    pub(in crate::host) fn hidden_palette_content(&self) -> Vec<NodeId> {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return vec![];
        };
        let mut hidden = vec![];
        for (id, state) in &self.palettes {
            let Some(node) = tree.get(*id) else {
                continue;
            };
            if node.children.is_empty() {
                continue;
            }
            if !state.rows.is_empty() || state.loading {
                hidden.push(node.children[2]);
            }
            let matched = state
                .rows
                .iter()
                .map(|row| row.route.config.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            for (index, command) in state.config.commands.iter().enumerate() {
                if !matched.contains(command.as_str()) {
                    hidden.push(node.children[3 + index]);
                }
            }
        }
        hidden
    }

    pub(super) fn palette_slot(
        &mut self,
        node: &crate::tree::Node,
        index: usize,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let shared = self.session.clone();
        let session = shared.borrow();
        let tree = session.tree(self.id)?;
        let slot = tree.get(*node.children.get(index)?)?;
        let content = *slot.children.first()?;
        Some(self.element(tree, content, interaction, window, cx))
    }

    pub(super) fn palette_row_content(
        &self,
        node: &crate::tree::Node,
        interaction: Interaction,
        cx: &Context<Self>,
    ) -> Option<RowContent> {
        // Row callbacks use the declared command index, independent of filtering.
        if node.children.is_empty() {
            return None;
        }
        let expected = node.palette.clone()?;
        let slots = node.children.clone();
        let id = node.id;
        let wid = self.id;
        let shared = self.session.clone();
        let weak = cx.weak_entity();
        Some(Rc::new(move |index, disabled, window, cx| {
            weak.update(cx, |view, cx| {
                let session = shared.borrow();
                let tree = session.tree(wid)?;
                let current = tree.get(id)?;
                if current
                    .palette
                    .as_ref()
                    .is_none_or(|config| !Arc::ptr_eq(config, &expected))
                    || !Arc::ptr_eq(&current.children, &slots)
                    || view.palettes.get(&id)?.closed
                {
                    return None;
                }
                let slot = tree.get(*slots.get(3 + index)?)?;
                let content = *slot.children.first()?;
                if !view.focus.borrow().visible(content) {
                    return None;
                }
                Some(view.control_label(tree, content, interaction, disabled, window, cx))
            })
            .ok()
            .flatten()
        }))
    }

    pub(super) fn palette_owns_keys(&self, id: NodeId, window: &Window, cx: &App) -> bool {
        self.palettes
            .get(&id)
            .is_some_and(|state| state.query.read(cx).focus_handle(cx).is_focused(window))
            || self
                .focus
                .borrow()
                .handle(id)
                .is_some_and(|focus| focus.is_focused(window))
    }

    pub(super) fn palette_child_escape(
        &mut self,
        id: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.palette_owns_keys(id, window, cx)
            && self.focus.borrow().top_overlay(id)
            && self.palettes.get(&id).is_some_and(|state| !state.closed)
        {
            self.close_palette(id, PaletteDismissal::Escape, window, cx);
            cx.stop_propagation();
        }
    }
}
