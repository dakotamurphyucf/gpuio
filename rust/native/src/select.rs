//! Native select popup: application values remain in ChoiceConfig. The trigger
//! keeps keyboard focus; options use active-descendant semantics and no Tab stops.
use super::choice::{self, Route};
use gpui::{
    Bounds, Context, Div, FocusHandle, Pixels, Stateful, UniformListScrollHandle, Window, canvas,
    deferred, div, prelude::*, px, rgba,
};
use gpuio_protocol::v1::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[derive(Default)]
pub(super) struct State {
    pub(super) open: bool,
    navigation: choice::State,
    search: super::typeahead::Search,
    scroll: UniformListScrollHandle,
    active_index: Option<usize>,
    #[cfg(feature = "native-tests")]
    pub(super) rendered_options: Rc<Cell<usize>>,
    trigger: Rc<Cell<Bounds<Pixels>>>,
    #[cfg(feature = "native-tests")]
    pub(super) popup_bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl State {
    fn open(&mut self, config: &ChoiceConfig) {
        self.search.clear();
        self.open = !config.disabled;
        self.navigation.reconcile(config, false);
        self.reveal(config);
    }
    fn reveal(&mut self, config: &ChoiceConfig) {
        self.active_index = config
            .items
            .iter()
            .position(|item| Some(&item.id) == self.navigation.active.as_ref());
        if let Some(index) = self.active_index {
            self.scroll
                .scroll_to_item(index, gpui::ScrollStrategy::Nearest);
        }
    }
    fn confirm(&mut self, config: &ChoiceConfig, route: &Route) {
        if let Some(id) = &self.navigation.active
            && config.can_select(id)
        {
            route.select(id);
        }
        self.open = false;
    }
}

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
    let owner = cx.entity_id();
    {
        let mut state = state.borrow_mut();
        if config.disabled || !focus.is_focused(window) {
            state.open = false;
        }
        let open = state.open;
        state.navigation.reconcile(config, open);
        let index = config
            .items
            .iter()
            .position(|item| Some(&item.id) == state.navigation.active.as_ref());
        if open && index != state.active_index {
            state.reveal(config);
        }
    }
    let open = state.borrow().open;
    let value = config
        .selected
        .as_ref()
        .and_then(|id| config.items.iter().find(|item| &item.id == id))
        .map(|item| item.label.clone());
    let trigger = state.borrow().trigger.clone();
    base = base
        .relative()
        .aria_expanded(open)
        .aria_value(value.clone().unwrap_or_default())
        .child(gpui::SharedString::from(
            value.unwrap_or_else(|| config.label.clone()),
        ))
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
                state.open = false;
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
                if !state.open {
                    state.open(&key_config);
                }
                let active = state.navigation.active.clone();
                if let Some(id) = state.search.advance(
                    &key_config,
                    active.as_deref(),
                    text,
                    std::time::Instant::now(),
                ) {
                    state.navigation.active = Some(id);
                    state.reveal(&key_config);
                }
                cx.notify(owner);
                cx.stop_propagation();
                return;
            }
            if modifiers.modified() {
                return;
            }
            if key == "escape" && state.open {
                state.open = false;
                cx.notify(owner);
                cx.stop_propagation();
            } else if matches!(key, "up" | "down" | "home" | "end") {
                state.search.clear();
                if state.open {
                    state.navigation.navigate(&key_config, key);
                    state.reveal(&key_config);
                } else {
                    state.open(&key_config);
                    if key == "home" || key == "end" {
                        state.navigation.navigate(&key_config, key);
                        state.reveal(&key_config);
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
            if state.open {
                if keyboard {
                    state.confirm(&click_config, &click_route);
                } else {
                    state.open = false;
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
        base = base.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
            window.focus(&action_focus, cx);
            let mut state = action_state.borrow_mut();
            if state.open {
                state.open = false;
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
            let dark = matches!(
                window.appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            );
            let background = rgba(if dark { 0x242424ff } else { 0xffffffff });
            let foreground = rgba(if dark { 0xf0f0f0ff } else { 0x202020ff });
            let active_color = rgba(if dark { 0x385477ff } else { 0xdbeaffff });
            let options = config.clone();
            let option_state = state.clone();
            let scroll = state.borrow().scroll.clone();
            let count = config.items.len();
            #[cfg(feature = "native-tests")]
            let rendered = state.borrow().rendered_options.clone();
            let list_height = px((count.clamp(1, 8) * 32) as f32)
                .min((window.viewport_size().height - px(18.)).max(px(1.)));
            let list = gpui::uniform_list("options", count, move |range, _, _| {
                #[cfg(feature = "native-tests")]
                rendered.set(range.len());
                let active = option_state.borrow().navigation.active.clone();
                range
                    .map(|index| {
                        let item = &options.items[index];
                        let selected = options.selected.as_ref() == Some(&item.id);
                        let mut row = div()
                            .id(gpui::SharedString::from(item.id.clone()))
                            .h(px(32.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .overflow_hidden()
                            .role(gpui::Role::ListBoxOption)
                            .aria_label(item.label.clone())
                            .aria_selected(selected);
                        if selected && let Some(style) = &selected_style {
                            gpui::Refineable::refine(row.style(), style);
                        }
                        if active.as_ref() == Some(&item.id) && !item.disabled {
                            row = row.bg(active_color).aria_active_descendant();
                        }
                        row = row.child(gpui::SharedString::from(item.label.clone()));
                        if item.disabled {
                            row = row.opacity(0.5);
                        } else {
                            let state = option_state.clone();
                            let id = item.id.clone();
                            let route = route.clone();
                            let action_state = state.clone();
                            let action_id = id.clone();
                            let action_route = route.clone();
                            row = row.on_a11y_action(
                                gpui::AccessibleAction::Click,
                                move |_, _, cx| {
                                    action_route.select(&action_id);
                                    action_state.borrow_mut().open = false;
                                    cx.notify(owner);
                                },
                            );
                            if pointer {
                                row = row
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                    })
                                    .on_click(move |_, _, cx| {
                                        route.select(&id);
                                        state.borrow_mut().open = false;
                                        cx.notify(owner);
                                        cx.stop_propagation();
                                    });
                            }
                        }
                        crate::semantics::State {
                            element: row,
                            disabled: item.disabled,
                            read_only: false,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .track_scroll(&scroll)
            .w_full()
            .h(list_height);
            let outside_state = state.clone();
            let trigger = state.borrow().trigger.clone();
            let width = (window.viewport_size().width - px(16.))
                .max(px(1.))
                .min(px(320.));
            let height = (window.viewport_size().height - px(16.)).max(px(1.));
            let popup = div()
                .id("choice-popup")
                .role(gpui::Role::ListBox)
                .aria_label(config.label.clone())
                .occlude()
                .bg(background)
                .text_color(foreground)
                .border_1()
                .border_color(active_color)
                .w(width)
                .max_h(height)
                .overflow_hidden()
                .on_mouse_down_out(move |event, _, cx| {
                    if !trigger.get().contains(&event.position) {
                        outside_state.borrow_mut().open = false;
                        cx.notify(owner);
                    }
                })
                .child(if count == 0 {
                    div()
                        .id("choice-empty")
                        .h(list_height)
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .role(gpui::Role::Label)
                        .aria_label("No options")
                        .child("No options")
                        .into_any_element()
                } else {
                    list.into_any_element()
                });
            #[cfg(feature = "native-tests")]
            let popup = {
                let bounds = state.borrow().popup_bounds.clone();
                popup.child(
                    canvas(move |value, _, _| bounds.set(value), |_, _, _, _| {})
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                )
            };
            base = base.child(
                deferred(super::popup::Surface {
                    trigger: state.borrow().trigger.clone(),
                    content: popup.into_any_element(),
                })
                .with_priority(gpui_base::POPUP_PRIORITY),
            );
        }
    }
    base
}
