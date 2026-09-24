//! Native select popup: application values remain in ChoiceConfig. The trigger
//! keeps keyboard focus; options use active-descendant semantics and no Tab stops.
use super::choice::Route;
use gpui::{Context, Div, FocusHandle, Stateful, Window, canvas, deferred, div, prelude::*, px};
use gpuio_protocol::v1::*;
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Default)]
pub(super) struct State {
    pub(super) popup: Rc<RefCell<super::choice_popup::State>>,
    search: super::typeahead::Search,
}
impl State {
    fn open(&mut self, config: &ChoiceConfig) {
        self.search.clear();
        self.popup.borrow_mut().open(config);
    }
    fn confirm(&mut self, config: &ChoiceConfig, route: &Route) {
        let mut popup = self.popup.borrow_mut();
        if let Some(id) = &popup.navigation.active
            && config.can_select(id)
        {
            route.select(id);
        }
        popup.open = false;
    }
}

pub(super) struct Render<'a> {
    pub priority: usize,
    pub config: &'a Arc<ChoiceConfig>,
    pub appearance: Arc<ChoiceAppearance>,
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
        priority,
        config,
        appearance,
        state,
        focus,
        route,
        pointer,
        selected_style,
    } = render;
    let owner = cx.entity_id();
    let popup_state = state.borrow().popup.clone();
    popup_state.borrow_mut().reconcile(
        config,
        &appearance,
        focus.is_focused(window),
        window.viewport_size(),
    );
    let open = popup_state.borrow().open;
    let value = config
        .selected
        .as_ref()
        .and_then(|id| config.items.iter().find(|item| &item.id == id))
        .map(|item| item.label.clone());
    let trigger = popup_state.borrow().trigger.clone();
    base = base
        .aria_expanded(open)
        .aria_value(value.clone().unwrap_or_default())
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .child(gpui::SharedString::from(
                    value.unwrap_or_else(|| config.label.clone()),
                )),
        )
        .child(
            canvas(
                |_, _, _| (),
                |bounds, _, window, _| {
                    let mut path = gpui::PathBuilder::stroke(px(1.5));
                    path.move_to(bounds.origin + gpui::point(px(2.), px(4.)));
                    path.line_to(bounds.origin + gpui::point(px(6.), px(8.)));
                    path.line_to(bounds.origin + gpui::point(px(10.), px(4.)));
                    if let Ok(path) = path.build() {
                        window.paint_path(path, window.text_style().color);
                    }
                },
            )
            .w(px(12.))
            .h(px(12.))
            .flex_shrink_0(),
        )
        .child(
            canvas(move |bounds, _, _| trigger.set(bounds), |_, _, _, _| {})
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
        );
    if let Some(route) = route {
        let key_state = state.clone();
        let key_config = config.clone();
        base = base.on_key_down(move |event, _, cx| {
            let key = event.keystroke.key.as_str();
            let mut state = key_state.borrow_mut();
            if key == "tab" {
                state.popup.borrow_mut().open = false;
                cx.notify(owner);
                return; // Normal window traversal, including Shift-Tab.
            }
            let modifiers = event.keystroke.modifiers;
            if modifiers.control || modifiers.platform || modifiers.function {
                return;
            }
            // Printable platform text includes Shift and Option-modified Unicode.
            // Space retains the control's activation behavior.
            let text = event
                .keystroke
                .key_char
                .as_deref()
                .or_else(|| (key.chars().count() == 1 && !modifiers.alt).then_some(key));
            if let Some(text) = text.filter(|text| {
                *text != " " && !text.is_empty() && !text.chars().any(char::is_control)
            }) {
                if !state.popup.borrow().open {
                    state.open(&key_config);
                }
                let active = state.popup.borrow().navigation.active.clone();
                if let Some(id) = state.search.advance(
                    &key_config,
                    active.as_deref(),
                    text,
                    std::time::Instant::now(),
                ) {
                    state.popup.borrow_mut().navigation.active = Some(id);
                    state.popup.borrow_mut().reveal(&key_config);
                }
                cx.notify(owner);
                cx.stop_propagation();
                return;
            }
            if modifiers.modified() {
                return;
            }
            if key == "escape" && state.popup.borrow().open {
                state.popup.borrow_mut().open = false;
                cx.notify(owner);
                cx.stop_propagation();
            } else if matches!(key, "up" | "down" | "home" | "end") {
                state.search.clear();
                if state.popup.borrow().open {
                    state
                        .popup
                        .borrow_mut()
                        .navigation
                        .navigate(&key_config, key);
                    state.popup.borrow_mut().reveal(&key_config);
                } else {
                    state.open(&key_config);
                    if key == "home" || key == "end" {
                        state
                            .popup
                            .borrow_mut()
                            .navigation
                            .navigate(&key_config, key);
                        state.popup.borrow_mut().reveal(&key_config);
                    }
                }
                cx.notify(owner);
                cx.stop_propagation();
            }
        });
        let click_state = state.clone();
        let click_config = config.clone();
        let click_route = route.clone();
        base = base.on_click(move |event, _, cx| {
            let keyboard = matches!(event, gpui::ClickEvent::Keyboard(_));
            if !pointer && !keyboard {
                return;
            }
            let mut state = click_state.borrow_mut();
            if state.popup.borrow().open {
                if keyboard {
                    state.confirm(&click_config, &click_route);
                } else {
                    state.popup.borrow_mut().open = false;
                }
            } else {
                state.open(&click_config);
            }
            cx.notify(owner);
            cx.stop_propagation();
        });
        let action_state = state.clone();
        let action_config = config.clone();
        let action_focus = focus.clone();
        let action_gate = route.gate.clone();
        let action_node = route.node;
        base = base.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
            if !action_gate.borrow().allows(action_node) {
                return;
            }
            window.focus(&action_focus, cx);
            let mut state = action_state.borrow_mut();
            if state.popup.borrow().open {
                state.popup.borrow_mut().open = false;
            } else {
                state.open(&action_config);
            }
            cx.notify(owner);
        });
        if !pointer {
            base = base.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                window.prevent_default()
            });
        }
        if open {
            route
                .gate
                .borrow_mut()
                .surface(route.node, popup_state.borrow().popup_bounds.clone());
            let popup = super::choice_popup::element(
                super::choice_popup::Render {
                    config,
                    appearance,
                    state: popup_state.clone(),
                    choose: Rc::new(move |id, _, _| route.select(id)),
                    owner,
                    pointer,
                    selected_style,
                },
                window,
            );
            base = base.child(
                deferred(super::popup::Surface {
                    placement: Placement::default(),
                    trigger: popup_state.borrow().trigger.clone(),
                    content: popup.into_any_element(),
                })
                .with_priority(priority),
            );
        }
    }
    base
}
