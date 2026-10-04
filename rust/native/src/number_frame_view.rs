//! Stable content slots around the existing numeric editor and native buttons.
use super::*;
impl View {
    pub(super) fn number_frame_parts(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> number_input_view::Presentation {
        let Some(config) = &node.number_presentation else {
            return number_input_view::Presentation::default();
        };
        let disabled = self.focus.borrow().disabled(node.id);
        let mut slot = |index: usize, decorative: bool| {
            tree.get(node.children[index])
                .and_then(|slot| slot.children.first())
                .map(|id| {
                    if decorative {
                        self.control_label(tree, *id, interaction, disabled, window, cx)
                    } else {
                        self.element(tree, *id, interaction, window, cx)
                    }
                })
        };
        let leading = slot(0, false);
        let trailing = slot(1, false);
        let hidden = node.number_input.as_ref().unwrap().config.step_controls
            == gpuio_protocol::number_input::StepControls::Hidden;
        let decrement = if hidden { None } else { slot(2, true) };
        let increment = if hidden { None } else { slot(3, true) };
        number_input_view::Presentation {
            config: Some(config.clone()),
            leading,
            trailing,
            decrement,
            increment,
        }
    }
}
