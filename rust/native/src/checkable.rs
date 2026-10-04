//! Rich checkable presentation, separate from generic retained-tree traversal.
use super::*;

/// Project the admitted standalone value independently from activation eligibility.
/// Disabled selected controls remain selected members of their semantic set.
pub(super) fn radio_semantics(
    element: gpui::Stateful<gpui::Div>,
    checked: bool,
    position: Option<gpuio_protocol::checkable::Position>,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(gpui::Role::RadioButton)
        .aria_selected(checked)
        .aria_toggled(if checked {
            gpui::accesskit::Toggled::True
        } else {
            gpui::accesskit::Toggled::False
        });
    match position {
        Some(position) => element
            .aria_position_in_set(position.index as usize + 1)
            .aria_size_of_set(position.count as usize),
        None => element,
    }
}

pub(super) struct Render<'a> {
    pub node: &'a crate::tree::Node,
    pub interaction: Interaction,
    pub disabled: bool,
}
impl View {
    pub(super) fn checkable_element(
        &mut self,
        tree: &crate::tree::Tree,
        render: Render<'_>,
        element: gpui::Stateful<gpui::Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let Render {
            node,
            interaction,
            disabled,
        } = render;
        let appearance = node
            .control_appearance
            .as_deref()
            .unwrap_or_else(|| crate::control_appearance::default());
        let before =
            appearance.label_position == gpuio_protocol::control_appearance::LabelPosition::Before;
        let label = if let Some(child) = node.children.first() {
            Some(self.control_label(tree, *child, interaction, disabled, window, cx))
        } else if !node.text.is_empty() {
            Some(gpui::SharedString::from(node.text.clone()).into_any_element())
        } else {
            None
        };
        let indicator = crate::control_paint::element(
            if node.kind == Kind::Switch {
                crate::control_geometry::Kind::Switch
            } else if node.kind == Kind::Radio {
                crate::control_geometry::Kind::Radio
            } else {
                crate::control_geometry::Kind::Checkbox
            },
            appearance,
            matches!(
                node.control,
                Some(
                    Control::Checkbox(CheckState::Checked, _)
                        | Control::Switch(true, _)
                        | Control::Radio(true, _, _)
                )
            ),
            matches!(
                node.control,
                Some(Control::Checkbox(CheckState::Indeterminate, _))
            ),
            disabled,
        );
        match (before, label) {
            (true, Some(label)) => element.child(label).child(indicator),
            (false, Some(label)) => element.child(indicator).child(label),
            (_, None) => element.child(indicator),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Element, accesskit};
    use gpuio_protocol::checkable::Position;

    #[test]
    fn standalone_radio_semantics_preserve_disabled_selection_and_optional_membership() {
        for checked in [false, true] {
            for disabled in [false, true] {
                for position in [
                    None,
                    Some(Position { index: 0, count: 1 }),
                    Some(Position { index: 2, count: 3 }),
                ] {
                    let element = crate::semantics::State {
                        identity: None,
                        busy: false,
                        hidden: false,
                        metadata: None,
                        live: None,
                        disabled,
                        read_only: false,
                        modal: false,
                        element: radio_semantics(
                            div().id("radio").aria_label("Response depth"),
                            checked,
                            position,
                        ),
                    };
                    assert_eq!(element.a11y_role(), Some(accesskit::Role::RadioButton));
                    let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                    element.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Response depth"));
                    assert_eq!(node.is_selected(), Some(checked));
                    assert_eq!(
                        node.toggled(),
                        Some(if checked {
                            accesskit::Toggled::True
                        } else {
                            accesskit::Toggled::False
                        })
                    );
                    assert_eq!(
                        node.position_in_set(),
                        position.map(|value| value.index as usize + 1)
                    );
                    assert_eq!(
                        node.size_of_set(),
                        position.map(|value| value.count as usize)
                    );
                    assert_eq!(node.is_disabled(), disabled);
                    assert!(
                        !node.supports_action(accesskit::Action::Click),
                        "value projection must not invent an action handler"
                    );
                }
            }
        }
    }
}
