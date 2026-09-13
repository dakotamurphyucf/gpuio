//! Bounded option popup shared by choice controls. The owning control supplies
//! activation; this module owns highlight, geometry, virtualized rows and semantics.
use super::choice;
use gpui::{
    App, Bounds, Div, EntityId, Pixels, Stateful, UniformListScrollHandle, Window, canvas, div,
    prelude::*, px, rgba,
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
    pub(super) navigation: choice::State,
    scroll: UniformListScrollHandle,
    active_index: Option<usize>,
    geometry: Option<(f64, i64, gpui::Size<Pixels>)>,
    #[cfg(feature = "native-tests")]
    pub(super) rendered_options: Rc<Cell<usize>>,
    pub(super) trigger: Rc<Cell<Bounds<Pixels>>>,
    #[cfg(feature = "native-tests")]
    pub(super) popup_bounds: Rc<Cell<Bounds<Pixels>>>,
    #[cfg(feature = "native-tests")]
    pub(super) option_probes:
        Rc<RefCell<std::collections::BTreeMap<String, super::native_test::Probe>>>,
}
impl State {
    pub(super) fn open(&mut self, config: &ChoiceConfig) {
        self.open = !config.disabled;
        self.navigation.reconcile(config, false);
        self.reveal(config);
    }
    pub(super) fn reveal(&mut self, config: &ChoiceConfig) {
        self.active_index = config
            .items
            .iter()
            .position(|item| Some(&item.id) == self.navigation.active.as_ref());
        if let Some(index) = self.active_index {
            self.scroll
                .scroll_to_item(index, gpui::ScrollStrategy::Nearest);
        }
    }
    pub(super) fn reconcile(
        &mut self,
        config: &ChoiceConfig,
        appearance: &ChoiceAppearance,
        focused: bool,
        viewport: gpui::Size<Pixels>,
    ) {
        if config.disabled || !focused {
            self.open = false;
        }
        let open = self.open;
        self.navigation.reconcile(config, open);
        let index = config
            .items
            .iter()
            .position(|item| Some(&item.id) == self.navigation.active.as_ref());
        let geometry = (appearance.row_height, appearance.max_visible_rows, viewport);
        if open && (index != self.active_index || self.geometry != Some(geometry)) {
            self.reveal(config);
        }
        self.geometry = Some(geometry);
    }
}

pub(super) type Choose = Rc<dyn Fn(&str, &mut Window, &mut App)>;

pub(super) struct Render<'a> {
    pub config: &'a Arc<ChoiceConfig>,
    pub appearance: Arc<ChoiceAppearance>,
    pub state: Rc<RefCell<State>>,
    pub choose: Choose,
    pub owner: EntityId,
    pub pointer: bool,
    pub selected_style: Option<gpui::StyleRefinement>,
}

pub(super) fn element(render: Render<'_>, window: &Window) -> Stateful<Div> {
    let Render {
        config,
        appearance,
        state,
        choose,
        owner,
        pointer,
        selected_style,
    } = render;
    let dark = matches!(
        window.appearance(),
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
    );
    let background = rgba(if dark { 0x242424ff } else { 0xffffffff });
    let foreground = rgba(if dark { 0xf0f0f0ff } else { 0x202020ff });
    let active_color = rgba(if dark { 0x385477ff } else { 0xdbeaffff });
    #[cfg(feature = "native-tests")]
    state.borrow().option_probes.borrow_mut().clear();
    let options = config.clone();
    let option_state = state.clone();
    let scroll = state.borrow().scroll.clone();
    let count = config.items.len();
    #[cfg(feature = "native-tests")]
    let rendered = state.borrow().rendered_options.clone();
    let option_appearance = appearance.clone();
    let list_height = px((count.clamp(1, appearance.max_visible_rows as usize) as f64
        * appearance.row_height) as f32)
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
                    .h(px(option_appearance.row_height as f32))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .role(gpui::Role::ListBoxOption)
                    .aria_label(item.label.clone())
                    .aria_selected(selected);
                if pointer && !item.disabled {
                    row = row.cursor_pointer();
                }
                gpui::Refineable::refine(
                    row.style(),
                    &crate::appearance::refinement(&option_appearance.option_style, 0),
                );
                if selected && let Some(style) = &selected_style {
                    gpui::Refineable::refine(row.style(), style);
                }
                if selected {
                    gpui::Refineable::refine(
                        row.style(),
                        &crate::appearance::refinement(&option_appearance.option_style, 7),
                    );
                }
                if active.as_ref() == Some(&item.id) && !item.disabled {
                    row = row.bg(active_color).aria_active_descendant();
                    gpui::Refineable::refine(
                        row.style(),
                        &crate::appearance::refinement(&option_appearance.option_style, 1),
                    );
                }
                row = row.child(gpui::SharedString::from(item.label.clone()));
                if item.disabled {
                    row = row.opacity(0.5);
                    gpui::Refineable::refine(
                        row.style(),
                        &crate::appearance::refinement(&option_appearance.option_style, 6),
                    );
                } else {
                    if pointer {
                        let hovered =
                            crate::appearance::refinement(&option_appearance.option_style, 2);
                        let pressed =
                            crate::appearance::refinement(&option_appearance.option_style, 3);
                        row = row.hover(move |_| hovered).active(move |_| pressed);
                    }
                    let state = option_state.clone();
                    let id = item.id.clone();
                    let choose = choose.clone();
                    let action_state = state.clone();
                    let action_id = id.clone();
                    let action_choose = choose.clone();
                    row =
                        row.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                            action_choose(&action_id, window, cx);
                            action_state.borrow_mut().open = false;
                            cx.notify(owner);
                        });
                    if pointer {
                        row = row
                            .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_click(move |_, window, cx| {
                                choose(&id, window, cx);
                                state.borrow_mut().open = false;
                                cx.notify(owner);
                                cx.stop_propagation();
                            });
                    }
                }
                if !pointer || item.disabled {
                    row.style().mouse_cursor = None;
                }
                #[cfg(feature = "native-tests")]
                {
                    let probes = option_state.borrow().option_probes.clone();
                    let id = item.id.clone();
                    row = row.child(
                        canvas(
                            |bounds, _, _| bounds,
                            move |_, bounds, window, _| {
                                probes.borrow_mut().insert(
                                    id,
                                    super::native_test::Probe {
                                        bounds,
                                        color: window.text_style().color,
                                    },
                                );
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    );
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
        .min(px(appearance.popup_width as f32));
    let height = (window.viewport_size().height - px(16.)).max(px(1.));
    let mut popup = div()
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
            let mut empty = div()
                .id("choice-empty")
                .h(list_height)
                .px(px(8.))
                .flex()
                .items_center()
                .role(gpui::Role::Label)
                .aria_label(appearance.empty_label.clone())
                .child(gpui::SharedString::from(appearance.empty_label.clone()));
            gpui::Refineable::refine(
                empty.style(),
                &crate::appearance::refinement(&appearance.empty_style, 0),
            );
            empty.into_any_element()
        } else {
            list.into_any_element()
        });
    gpui::Refineable::refine(
        popup.style(),
        &crate::appearance::refinement(&appearance.popup_style, 0),
    );
    if pointer {
        let hovered = crate::appearance::refinement(&appearance.popup_style, 2);
        popup = popup.hover(move |_| hovered);
    } else {
        popup.style().mouse_cursor = None;
    }
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
    popup
}
