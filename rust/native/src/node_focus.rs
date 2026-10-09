//! Focus and control semantics preparation, outside recursive child traversal.
//! Large GPUI builders here must not remain on every ancestor's debug stack.
use super::*;

pub(super) struct Render<'a> {
    pub node: &'a crate::tree::Node,
    pub accessible_name: gpui::SharedString,
    pub input_presentation: Interaction,
    pub disabled: bool,
    pub preserve_focus: bool,
    pub button_order: Option<gpuio_protocol::checkable::TabOrder>,
    pub checked: bool,
    pub indeterminate: bool,
    pub command_checked: bool,
}

impl View {
    pub(super) fn node_focus(
        &mut self,
        tree: &crate::tree::Tree,
        render: Render<'_>,
        mut element: gpui::Stateful<gpui::Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let Render {
            node,
            accessible_name,
            input_presentation,
            disabled,
            preserve_focus,
            button_order,
            checked,
            indeterminate,
            command_checked,
        } = render;
        let id = node.id;
        if let Some(handle) = self.focus.borrow().handle(id) {
            element = element.track_focus(&handle);
        }
        if matches!(
            node.kind,
            Kind::Button
                | Kind::Link
                | Kind::CommandButton
                | Kind::Checkbox
                | Kind::Switch
                | Kind::Radio
                | Kind::RadioGroup
                | Kind::Rating
                | Kind::TabBar
                | Kind::Select
                | Kind::ChoicePicker
        ) {
            self.visited.insert(id);
            let state = self
                .buttons
                .entry(id)
                .or_insert_with(|| {
                    Rc::new(ButtonState {
                        focus: cx.focus_handle().tab_stop(true),
                    })
                })
                .clone();
            element = self.observe_button_hover(
                element,
                node,
                input_presentation.pointer && !disabled,
                window,
                cx,
            );
            let tab_stop = button_order.or(node.tab_order).map_or_else(
                || node.link.as_ref().is_none_or(|config| config.tab_stop),
                |config| config.tab_stop,
            );
            let tab_index = node.link.as_ref().map_or_else(
                || {
                    button_order
                        .or(node.tab_order)
                        .map_or(0, |config| config.index as isize)
                },
                |config| config.tab_index as isize,
            );
            if disabled || preserve_focus {
                state.focus.clone().tab_stop(false);
                if state.focus.is_focused(window) {
                    window.blur(cx);
                }
            } else {
                element = element
                    .track_focus(&state.focus.clone().tab_stop(tab_stop).tab_index(tab_index))
                    .tab_index(tab_index);
                let focus = state.focus.clone();
                let gate = self.focus.clone();
                element =
                    element.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                        if gate.borrow().can_focus(&focus, window) {
                            gate.borrow().request_reveal();
                            window.focus(&focus, cx);
                        }
                    });
            }
            element = element.aria_label(accessible_name.clone());
            if let Some(Control::Radio(checked, position, _)) = node.control {
                element = checkable::radio_semantics(element, checked, position);
            } else {
                element = element.role(match node.kind {
                    Kind::Link => gpui::Role::Link,
                    Kind::Checkbox => gpui::Role::CheckBox,
                    Kind::Switch => gpui::Role::Switch,
                    Kind::RadioGroup => gpui::Role::RadioGroup,
                    Kind::Rating => gpui::Role::Slider,
                    Kind::TabBar => gpui::Role::TabList,
                    Kind::Select | Kind::ChoicePicker => gpui::Role::ComboBox,
                    _ => gpui::Role::Button,
                });
                if command_checked || matches!(node.kind, Kind::Checkbox | Kind::Switch) {
                    element = element.aria_toggled(if indeterminate {
                        gpui::accesskit::Toggled::Mixed
                    } else if checked {
                        gpui::accesskit::Toggled::True
                    } else {
                        gpui::accesskit::Toggled::False
                    });
                }
            }
            if input_presentation.pointer
                && !disabled
                && !node.rating.as_ref().is_some_and(|config| config.read_only)
            {
                element = element.cursor_pointer();
            }
        }
        if node.kind == Kind::Link && disabled {
            element = element.block_mouse_except_scroll();
        }
        if node.kind == Kind::TabBar {
            element = element.flex_row();
        }
        if matches!(
            node.kind,
            Kind::TabPanel
                | Kind::Panel
                | Kind::NavigationStack
                | Kind::Carousel
                | Kind::CarouselTrack
        ) {
            element = element
                .role(if node.kind == Kind::TabPanel {
                    gpui::Role::TabPanel
                } else {
                    gpui::Role::Region
                })
                .aria_label(accessible_name);
        }
        if matches!(node.kind, Kind::Disclosure | Kind::Accordion) {
            element = element.role(gpui::Role::Group);
        }
        if let Some(parent) = tree.disclosure_for_trigger(id) {
            let expanded = self.focus.borrow().visible(parent.children[1]);
            let gate = self.focus.clone();
            element = element
                .aria_expanded(expanded)
                .on_key_down(move |event, window, cx| {
                    let modifiers = event.keystroke.modifiers;
                    if !modifiers.control
                        && !modifiers.alt
                        && !modifiers.platform
                        && !modifiers.shift
                        && gate
                            .borrow()
                            .disclosure_key(id, &event.keystroke.key, window, cx)
                    {
                        cx.stop_propagation();
                    }
                });
        }
        if node.carousel.is_some() {
            element = self.carousel_element(element, node, cx);
        }
        if node.carousel_track.is_some() {
            element = self.carousel_track_element(element, node, cx);
        }
        self.track_related_element(tree, node, element, cx)
    }
}
