use super::*;

#[derive(Clone)]
struct ClearTarget {
    node: NodeId,
    focus: gpui::WeakFocusHandle,
    revision: i64,
    policy: i64,
}

impl View {
    fn clear_target(&self, node: &crate::tree::Node, cx: &App) -> Option<ClearTarget> {
        let frame = node.editor_frame.as_ref()?;
        let config = node.editor.as_ref()?;
        if frame.loading
            || frame.clear_label.is_none()
            || config.disabled
            || config.read_only
            || !self.focus.borrow().allows(node.id)
            || !self.focus.borrow().visible(node.id)
        {
            return None;
        }
        let editor = self.editors.get(&node.id)?;
        Some(ClearTarget {
            node: node.id,
            focus: editor.focus_handle(cx).downgrade(),
            revision: editor.clear_revision(cx)?,
            policy: node.editor_frame_activation_revision,
        })
    }

    fn clear_input(&mut self, target: &ClearTarget, window: &mut Window, cx: &mut Context<Self>) {
        let current = {
            let session = self.session.borrow();
            session
                .tree(self.id)
                .and_then(|tree| tree.get(target.node))
                .and_then(|node| self.clear_target(node, cx))
        };
        let Some(current) = current else {
            return;
        };
        if current.focus != target.focus
            || current.revision != target.revision
            || current.policy != target.policy
        {
            return;
        }
        if let Some(editor) = self.editors.get(&target.node) {
            editor.clear(target.revision, window, cx);
        }
    }

    pub(super) fn editor_frame_element(
        &mut self,
        mut element: gpui::Stateful<gpui::Div>,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let editor = self.editors[&node.id].element();
        // The plain structural wrappers keep slot identity but must not allocate
        // flex space when empty. Mount only the slot's single admitted content.
        let slot = |view: &mut Self, index: usize, window: &mut Window, cx: &mut Context<Self>| {
            tree.get(node.children[index])
                .and_then(|slot| slot.children.first())
                .map(|id| view.element(tree, *id, interaction, window, cx))
        };
        element = element
            .children(slot(self, 0, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .when(node.kind == Kind::Textarea, |middle| {
                        middle.self_stretch().flex().flex_col().min_h_0()
                    })
                    .child(editor),
            )
            .children(slot(self, 1, window, cx))
            .children(slot(self, 2, window, cx));
        if let Some(target) = self.clear_target(node, cx) {
            let owner = cx.weak_entity();
            let accessible_owner = owner.clone();
            let accessible_target = target.clone();
            let mut clear = div()
                .id("gpuio-input-clear")
                .role(gpui::Role::Button)
                .aria_label(
                    node.editor_frame
                        .as_ref()
                        .unwrap()
                        .clear_label
                        .clone()
                        .unwrap(),
                )
                .flex_none()
                .px(px(5.))
                .child("×")
                .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                    let _ = accessible_owner.update(cx, |view, cx| {
                        view.clear_input(&accessible_target, window, cx)
                    });
                    cx.stop_propagation();
                });
            if interaction.pointer {
                clear = clear
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                        window.prevent_default()
                    })
                    .on_click(move |_, window, cx| {
                        let _ = owner.update(cx, |view, cx| view.clear_input(&target, window, cx));
                        cx.stop_propagation();
                    });
            }
            element = element.child(clear);
        }
        element.children(slot(self, 3, window, cx))
    }
}

#[cfg(all(test, feature = "native-tests"))]
#[path = "editor_frame_test.rs"]
mod tests;
