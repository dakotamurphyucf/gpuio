//! Radio groups use one native focus target; OCaml owns selection.
use super::choice::{Route, State};
use gpui::{Context, Div, FocusHandle, Stateful, Window, div, prelude::*, px};
use gpuio_protocol::v1::*;
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(super) struct Render<'a> {
    pub config: &'a Arc<ChoiceConfig>,
    pub state: Rc<RefCell<State>>,
    pub focus: FocusHandle,
    pub route: Option<Route>,
    pub pointer: bool,
    pub selected_style: Option<gpui::StyleRefinement>,
}

pub(super) fn element<T: 'static>(
    mut base: Stateful<Div>,
    render: Render<'_>,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Stateful<Div> {
    let Render {
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
    let active = state.borrow().active.clone();
    for item in &config.items {
        let selected = config.selected.as_ref() == Some(&item.id);
        let disabled = config.disabled || item.disabled;
        let mut option = div()
            .id(gpui::SharedString::from(item.id.clone()))
            .flex()
            .items_center()
            .gap(px(8.))
            .p(px(6.))
            .role(gpui::Role::RadioButton)
            .aria_label(item.label.clone())
            .aria_toggled(if selected {
                gpui::accesskit::Toggled::True
            } else {
                gpui::accesskit::Toggled::False
            });
        if active.as_ref() == Some(&item.id) && !disabled {
            option = option.aria_active_descendant();
        }
        if selected && let Some(style) = &selected_style {
            gpui::Refineable::refine(option.style(), style);
        }
        if item.disabled && !config.disabled {
            option = option.opacity(0.5);
        }
        option = option
            .child(super::control_indicator(Kind::RadioGroup, selected, false))
            .child(gpui::SharedString::from(item.label.clone()));
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
            option = option.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                if !gate.borrow().allows(node) {
                    return;
                }
                focus_state.borrow_mut().active = Some(focus_id.clone());
                window.focus(&action_focus, cx);
                cx.notify(owner);
            });
            let action_route = route.clone();
            let action_id = selected.clone();
            let action_state = state.clone();
            let action_focus = focus.clone();
            option = option.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                if !action_route.gate.borrow().allows(action_route.node) {
                    return;
                }
                action_state.borrow_mut().active = Some(action_id.clone());
                window.focus(&action_focus, cx);
                action_route.select(&action_id);
                cx.notify(owner);
            });
            if pointer {
                option = option.cursor_pointer().on_click(move |_, window, cx| {
                    if !route.gate.borrow().allows(route.node) {
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
            live: None,
            element: option,
            disabled,
            read_only: false,
            modal: false,
        });
    }
    if let Some(route) = route {
        let key_state = state.clone();
        let key_config = config.clone();
        let key_route = route.clone();
        base = base
            .on_key_down(move |event, _, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                let mut state = key_state.borrow_mut();
                if state.navigate(&key_config, &event.keystroke.key) {
                    if let Some(id) = &state.active {
                        key_route.select(id);
                    }
                    cx.notify(owner);
                    cx.stop_propagation();
                }
            })
            .on_click(move |event, _, cx| {
                if matches!(event, gpui::ClickEvent::Keyboard(_)) {
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
