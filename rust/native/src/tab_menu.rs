//! Fresh decorative menu row elements, backed by accepted-tree icon leases.
//! Only visible rows are constructed; no OCaml callback enters native layout.
use super::*;

impl View {
    pub(super) fn tab_menu_icons(
        &self,
        node: &crate::tree::Node,
        interaction: Interaction,
        cx: &Context<Self>,
    ) -> Option<choice_popup::Decoration> {
        if !node.choice_menu || node.children.is_empty() {
            return None;
        }
        let expected = node.choice.clone()?;
        let slots = node.children.clone();
        let id = node.id;
        let wid = self.id;
        let session = self.session.clone();
        let weak = cx.entity().downgrade();
        Some(Rc::new(move |index, window, cx| {
            weak.update(cx, |view, cx| {
                let session = session.borrow();
                let tree = session.tree(wid)?;
                let current = tree.get(id)?;
                if !current.choice_menu
                    || current
                        .choice
                        .as_ref()
                        .is_none_or(|c| !Arc::ptr_eq(c, &expected))
                    || !Arc::ptr_eq(&current.children, &slots)
                {
                    return None;
                }
                let slot = tree.get(*slots.get(index)?)?;
                let icon = *slot.children.first()?;
                let disabled = expected.items.get(index)?.disabled;
                Some(view.control_label(tree, icon, interaction, disabled, window, cx))
            })
            .ok()
            .flatten()
        }))
    }
}
