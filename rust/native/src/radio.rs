//! Radio groups use one native focus target; OCaml owns selection.
use super::choice::{Route, State};
use gpui::{Context, Div, FocusHandle, Stateful, Window, div, prelude::*, px};
use gpuio_protocol::v1::*;
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(super) struct Parts {
    pub prefix: Option<gpui::AnyElement>,
    pub label: Option<gpui::AnyElement>,
    pub suffix: Option<gpui::AnyElement>,
}
pub(super) struct Render<'a> {
    pub tabs: bool,
    pub tab_content: Option<&'a gpuio_protocol::tab_content::Config>,
    pub viewport: Option<super::tab_viewport::Binding>,
    pub motion: Option<super::tab_motion::Owner>,
    pub parts: Vec<Parts>,
    pub trailing: Option<gpui::AnyElement>,
    pub gate: super::focus::Shared,
    pub node: gpuio_protocol::NodeId,
    pub tab_appearance: Option<&'a gpuio_protocol::tab_appearance::Config>,
    pub labels: Vec<Option<gpui::AnyElement>>,
    pub disabled: bool,
    pub appearance: &'a gpuio_protocol::control_appearance::Config,
    pub config: &'a Arc<ChoiceConfig>,
    pub state: Rc<RefCell<State>>,
    pub focus: FocusHandle,
    pub route: Option<Route>,
    pub pointer: bool,
    pub selected_style: Option<gpui::StyleRefinement>,
}

fn option_semantics(
    option: Stateful<Div>,
    tabs: bool,
    selected: bool,
    index: usize,
    total: usize,
) -> Stateful<Div> {
    let option = option
        .aria_selected(selected)
        .aria_position_in_set(index + 1)
        .aria_size_of_set(total);
    if tabs {
        option.role(gpui::Role::Tab)
    } else {
        // The pinned primitive reports both states: assistive technologies
        // differ in whether they read radio selection or its toggled value.
        option
            .role(gpui::Role::RadioButton)
            .aria_toggled(if selected {
                gpui::accesskit::Toggled::True
            } else {
                gpui::accesskit::Toggled::False
            })
    }
}

pub(super) fn element<T: 'static>(
    mut base: Stateful<Div>,
    render: Render<'_>,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Stateful<Div> {
    let Render {
        tabs,
        tab_content,
        viewport,
        motion,
        parts,
        trailing,
        gate,
        node,
        tab_appearance,
        labels,
        disabled: parent_disabled,
        appearance,
        config,
        state,
        focus,
        route,
        pointer,
        selected_style,
    } = render;
    state
        .borrow_mut()
        .reconcile(config, focus.is_focused(window));
    let owner = cx.entity_id();
    let horizontal = matches!(
        base.style().flex_direction,
        Some(gpui::FlexDirection::Row | gpui::FlexDirection::RowReverse)
    );
    base = base.aria_orientation(if horizontal {
        gpui::accesskit::Orientation::Horizontal
    } else {
        gpui::accesskit::Orientation::Vertical
    });
    let accent = base.style().text.color.unwrap_or(window.text_style().color);
    if let Some(appearance) = tab_appearance {
        base = super::tab_presentation::bar(base, appearance, accent);
    }
    let motion = motion.and_then(|owner| {
        let selected = config.selected.as_ref()?;
        let variant = tab_appearance
            .map_or(gpuio_protocol::tab_appearance::Variant::Underline, |a| {
                a.variant
            });
        let disabled = parent_disabled
            || config.disabled
            || config
                .items
                .iter()
                .any(|item| &item.id == selected && item.disabled);
        super::tab_motion::Binding::new(
            owner,
            selected,
            variant,
            disabled || cx.reduce_motion() || !window.is_window_active(),
            cx.background_executor().now(),
        )
    });
    if let Some(motion) = &motion {
        base = base.child(motion.indicator(accent));
    }
    let active = state.borrow().active.clone();
    let mut labels = labels.into_iter();
    let mut parts = parts.into_iter();
    for (index, item) in config.items.iter().enumerate() {
        let selected = config.selected.as_ref() == Some(&item.id);
        let disabled = parent_disabled || config.disabled || item.disabled;
        let mut option = div()
            .id(gpui::SharedString::from(item.id.clone()))
            .flex()
            .items_center()
            .gap(px(if tabs { 8. } else { appearance.gap as f32 }))
            .p(px(6.))
            .aria_label(item.label.clone());
        option = option_semantics(option, tabs, selected, index, config.items.len());
        if active.as_ref() == Some(&item.id) && !disabled {
            option = option.aria_active_descendant();
        }
        if let Some(appearance) = tab_appearance {
            option = super::tab_presentation::target(
                option,
                appearance,
                selected && motion.is_none(),
                accent,
            );
        } else if tabs && selected && motion.is_none() {
            option = option.border_b_2().border_color(window.text_style().color);
        }
        if selected && let Some(style) = &selected_style {
            gpui::Refineable::refine(option.style(), style);
        }
        if item.disabled && !config.disabled {
            option = option.opacity(0.5);
        }
        if let Some(appearance) = tab_appearance {
            option = super::tab_presentation::styles(
                option,
                appearance,
                &item.id,
                super::tab_presentation::State {
                    selected,
                    scrolling: viewport.is_some(),
                    focused: active.as_ref() == Some(&item.id) && focus.is_focused(window),
                    disabled,
                    pointer,
                    accent,
                },
            );
        }
        if selected && let Some(motion) = &motion {
            motion.set_opacity(option.style().opacity.unwrap_or(1.));
            let normal = tab_appearance.map_or(accent, |appearance| {
                super::tab_presentation::normal_color(
                    appearance,
                    &item.id,
                    active.as_ref() == Some(&item.id) && focus.is_focused(window),
                    accent,
                )
            });
            let target = option.style().text.color.unwrap_or(accent);
            option = option.text_color(motion.color(normal, target));
        }
        if viewport.is_some() {
            option = option.flex_shrink_0();
        }
        let before = !tabs
            && appearance.label_position
                == gpuio_protocol::control_appearance::LabelPosition::Before;
        let label = labels
            .next()
            .flatten()
            .unwrap_or_else(|| gpui::SharedString::from(item.label.clone()).into_any_element());
        if let Some(content) = tab_content {
            let Parts {
                prefix,
                label,
                suffix,
            } = parts.next().expect("validated tab parts");
            let mode = content.labels[index];
            let max_width = content
                .max_width
                .filter(|_| mode != gpuio_protocol::tab_content::Label::Hidden);
            if let Some(max_width) = max_width {
                option = option
                    .min_w_0()
                    .max_w(px(max_width as f32))
                    .flex_nowrap()
                    .overflow_hidden();
            }
            if let Some(prefix) = prefix {
                option = option.child(
                    div()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .block_mouse_except_scroll()
                        .child(prefix),
                );
            }
            let label = match mode {
                gpuio_protocol::tab_content::Label::Default => {
                    Some(gpui::SharedString::from(item.label.clone()).into_any_element())
                }
                gpuio_protocol::tab_content::Label::Custom => label,
                gpuio_protocol::tab_content::Label::Hidden => None,
            };
            if let Some(label) = label {
                let mut frame = div().min_w_0().child(label);
                if max_width.is_some() {
                    frame = frame.overflow_hidden().whitespace_nowrap().text_ellipsis();
                }
                option = option.child(frame);
            }
            if let Some(suffix) = suffix {
                option = option.child(
                    div()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .block_mouse_except_scroll()
                        .child(suffix),
                );
            }
        } else if tabs {
            option = option.child(label);
        } else {
            let indicator = crate::control_paint::element(
                crate::control_geometry::Kind::Radio,
                appearance,
                selected,
                false,
                disabled,
            );
            option = if before {
                option.child(label).child(indicator)
            } else {
                option.child(indicator).child(label)
            };
        }

        if !disabled && let Some(route) = &route {
            let selected = item.id.clone();
            let route = route.clone();
            let state = state.clone();
            let focus = focus.clone();
            // Focus and selection are distinct semantic actions.
            let gate = route.gate.clone();
            let node = route.node;
            let focus_state = state.clone();
            let focus_id = selected.clone();
            let action_focus = focus.clone();
            let action_viewport = viewport.clone();
            option = option.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                if !gate.borrow().allows(node) {
                    return;
                }
                focus_state.borrow_mut().active = Some(focus_id.clone());
                if let Some(viewport) = &action_viewport {
                    viewport.navigate(&focus_id);
                }
                window.focus(&action_focus, cx);
                cx.notify(owner);
            });
            let action_route = route.clone();
            let action_id = selected.clone();
            let action_state = state.clone();
            let action_focus = focus.clone();
            let action_viewport = viewport.clone();
            option = option.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                if !action_route.gate.borrow().allows(action_route.node) {
                    return;
                }
                action_state.borrow_mut().active = Some(action_id.clone());
                if let Some(viewport) = &action_viewport {
                    viewport.navigate(&action_id);
                }
                window.focus(&action_focus, cx);
                action_route.select(&action_id);
                cx.notify(owner);
            });
            if pointer {
                if option.style().mouse_cursor.is_none() {
                    option = option.cursor_pointer();
                }
                option = option.on_click(move |event, window, cx| {
                    if (matches!(event, gpui::ClickEvent::Keyboard(_)) && !focus.is_focused(window))
                        || !route.gate.borrow().allows(route.node)
                    {
                        return;
                    }
                    state.borrow_mut().active = Some(selected.clone());
                    window.focus(&focus, cx);
                    route.select(&selected);
                    cx.notify(owner);
                    cx.stop_propagation();
                });
            }
        }
        base = base.child(crate::semantics::State {
            identity: None,
            busy: false,
            hidden: false,
            metadata: None,
            live: None,
            element: super::highlight_style::Frame::clip(
                super::tab_viewport::Measure::new(
                    option,
                    viewport.as_ref(),
                    index,
                    motion.as_ref(),
                    selected,
                ),
                node,
                &gate,
            ),
            disabled,
            read_only: false,
            modal: false,
        });
    }
    if let Some(trailing) = trailing {
        base = base.child(trailing);
    }
    if let Some(viewport) = &viewport {
        base = base.child(viewport.finish());
    }
    if let Some(route) = route {
        let key_state = state.clone();
        let key_config = config.clone();
        let key_route = route.clone();
        let key_focus = focus.clone();
        base = base
            .on_key_down(move |event, window, cx| {
                if !key_focus.is_focused(window) || event.keystroke.modifiers.modified() {
                    return;
                }
                let mut state = key_state.borrow_mut();
                if state.navigate(&key_config, &event.keystroke.key) {
                    if let Some(id) = &state.active {
                        key_route.select(id);
                        if let Some(viewport) = &viewport {
                            viewport.navigate(id);
                        }
                    }
                    cx.notify(owner);
                    cx.stop_propagation();
                }
            })
            .on_click(move |event, window, cx| {
                if focus.is_focused(window) && matches!(event, gpui::ClickEvent::Keyboard(_)) {
                    if let Some(id) = &state.borrow().active {
                        route.select(id);
                    }
                    cx.stop_propagation();
                }
            });
        if !pointer {
            base = base.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                window.prevent_default()
            });
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Element, accesskit};

    #[test]
    fn option_metadata_reports_selected_position_and_radio_toggle_independently_of_disabled() {
        for tabs in [false, true] {
            for selected in [false, true] {
                for (index, total) in [(0, 3), (2, 3), (0, 1)] {
                    let element = crate::semantics::State {
                        identity: None,
                        busy: false,
                        hidden: false,
                        metadata: None,
                        live: None,
                        disabled: true,
                        read_only: false,
                        modal: false,
                        element: option_semantics(
                            div().id("stable-option").aria_label("Choice label"),
                            tabs,
                            selected,
                            index,
                            total,
                        ),
                    };
                    let expected_role = if tabs {
                        accesskit::Role::Tab
                    } else {
                        accesskit::Role::RadioButton
                    };
                    assert_eq!(element.a11y_role(), Some(expected_role));
                    let mut node = accesskit::Node::new(expected_role);
                    element.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Choice label"));
                    assert_eq!(node.is_selected(), Some(selected));
                    assert_eq!(node.position_in_set(), Some(index + 1));
                    assert_eq!(node.size_of_set(), Some(total));
                    assert!(node.is_disabled());
                    assert_eq!(
                        node.toggled(),
                        (!tabs).then_some(if selected {
                            accesskit::Toggled::True
                        } else {
                            accesskit::Toggled::False
                        })
                    );
                }
            }
        }
    }
}
