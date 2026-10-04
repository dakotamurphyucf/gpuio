//! Shared hover remains in GPUI's hit-tested native style machinery.
use super::View;
use gpui::{Div, InteractiveElement, Refineable, Stateful, Styled, Window};
use gpuio_protocol::NodeId;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "split_button_view_test.rs"]
mod tests;

// GPUI keeps empty GroupHitboxes name entries after pop. One private lexical
// name bounds that global map; the stack supplies the current pair's hitbox.
// Per-element hover state still follows the full keyed native owner identity.
pub(super) const GROUP: &str = "gpuio-split-button";

impl View {
    pub(super) fn coordinate_split(
        &self,
        tree: &crate::tree::Tree,
        id: NodeId,
        mut element: Stateful<Div>,
        enabled: bool,
        window: &Window,
    ) -> Stateful<Div> {
        if !enabled || !self.focus.borrow().allows(id) {
            return element;
        }
        let mut parent = tree.get(id).and_then(|node| node.parent);
        // Eight tooltip anchors, one keyed part slot, then the split root.
        for _ in 0..10 {
            let Some(node) = parent.and_then(|id| tree.get(id)) else {
                break;
            };
            if let Some(config) = &node.split_button {
                let Ok(owners) = crate::split_button::owners(node, |id| tree.get(id)) else {
                    break;
                };
                if owners.primary != Some(id) && owners.menu != Some(id) {
                    break;
                }
                let surface = crate::appearance::refinement(&config.surface, 0);
                if owners
                    .menu
                    .is_some_and(|menu| self.split_menu_open(tree, menu, window))
                {
                    element.style().refine(&surface);
                    if owners.menu == Some(id) {
                        element
                            .style()
                            .refine(&crate::appearance::refinement(&config.menu_open, 0));
                    }
                }
                return element.group_hover(GROUP, move |_| surface);
            }
            parent = node.parent;
        }
        element
    }
}
