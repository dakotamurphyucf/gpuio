//! Editable choices retain the native editor's query/caret/IME ownership.
//! Application selection and query replacement remain separate operations.
use super::{choice::Route, choice_popup};
use gpui::{Context, Div, Entity, Focusable, Stateful, Window, canvas, deferred, prelude::*};
use gpui_base::input::InputState;
use gpuio_protocol::v1::*;
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Default)]
pub(super) struct State {
    pub(super) popup: Rc<RefCell<choice_popup::State>>,
    query: Option<gpui::SharedString>,
    composing: bool,
    cached: Option<(Arc<ChoiceConfig>, String, ComboboxFilter, Arc<ChoiceConfig>)>,
}
impl State {
    pub(super) fn replaced(&mut self, text: &str) {
        self.query = Some(text.to_owned().into());
        self.popup.borrow_mut().open = false;
    }

    fn options(
        &mut self,
        config: &Arc<ChoiceConfig>,
        query: &str,
        filter: ComboboxFilter,
    ) -> Arc<ChoiceConfig> {
        if filter == ComboboxFilter::Unfiltered || query.is_empty() {
            self.cached = None;
            return config.clone();
        }
        if let Some((source, previous, mode, result)) = &self.cached
            && Arc::ptr_eq(source, config)
            && previous == query
            && *mode == filter
        {
            return result.clone();
        }
        let folded = query.to_lowercase();
        let result = Arc::new(ChoiceConfig {
            items: config
                .items
                .iter()
                .filter(|item| item.label.to_lowercase().contains(&folded))
                .cloned()
                .collect(),
            label: config.label.clone(),
            selected: config.selected.clone(),
            disabled: config.disabled,
        });
        self.cached = Some((config.clone(), query.to_owned(), filter, result.clone()));
        result
    }
}

pub(super) struct Render<'a> {
    pub editor: Entity<InputState>,
    pub state: Rc<RefCell<State>>,
    pub config: &'a Arc<ChoiceConfig>,
    pub appearance: Arc<ChoiceAppearance>,
    pub filter: ComboboxFilter,
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
        editor,
        state,
        config,
        appearance,
        filter,
        route,
        pointer,
        selected_style,
    } = render;
    let owner = cx.entity_id();
    let input = editor.read(cx);
    let query = input.value();
    let composing = input.bridge_composition().is_some();
    let focused = input.focus_handle(cx).is_focused(window);
    let (options, popup_state) = {
        let mut state = state.borrow_mut();
        let changed = state
            .query
            .as_deref()
            .is_some_and(|previous| previous != query.as_ref())
            || (state.composing && !composing);
        let options = state.options(config, &query, filter);
        if changed && focused && !composing {
            state.popup.borrow_mut().open(&options);
        }
        if composing {
            state.popup.borrow_mut().open = false;
        }
        state.query = Some(query.clone());
        state.composing = composing;
        let popup = state.popup.clone();
        popup
            .borrow_mut()
            .reconcile(&options, &appearance, focused, window.viewport_size());
        (options, popup)
    };
    let trigger = popup_state.borrow().trigger.clone();
    base = base.relative().child(editor.clone()).child(
        canvas(move |bounds, _, _| trigger.set(bounds), |_, _, _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
    );
    let Some(route) = route else {
        return base;
    };
    let choose_editor = editor.clone();
    let choose: choice_popup::Choose = Rc::new(move |id, window, cx| {
        let snapshot = super::editor::snapshot(choose_editor.read(cx), window, cx);
        route.select_combobox(id, snapshot);
    });
    // GPUI dispatches matched key bindings as actions before raw key listeners.
    // Capture the editor's Enter/Escape actions at the combobox boundary.
    let enter_editor = editor.clone();
    let enter_state = popup_state.clone();
    let enter_choose = choose.clone();
    base = base.capture_action(move |action: &gpui_base::input::Enter, window, cx| {
        if action.secondary || action.shift || enter_editor.read(cx).bridge_composition().is_some()
        {
            return;
        }
        if !enter_state.borrow().open {
            return;
        }
        let active = enter_state.borrow().navigation.active.clone();
        if let Some(id) = active {
            enter_choose(&id, window, cx);
        }
        enter_state.borrow_mut().open = false;
        cx.stop_propagation();
        cx.notify(owner);
    });
    let escape_editor = editor.clone();
    let escape_state = popup_state.clone();
    base = base.capture_action(move |_: &gpui_base::input::Escape, _, cx| {
        if escape_editor.read(cx).bridge_composition().is_some() || !escape_state.borrow().open {
            return;
        }
        escape_state.borrow_mut().open = false;
        cx.stop_propagation();
        cx.notify(owner);
    });
    let key_editor = editor.clone();
    let key_state = popup_state.clone();
    let key_options = options.clone();
    let key_choose = choose.clone();
    base = base.capture_key_down(move |event, window, cx| {
        // The OS/input method retains all composing key sequences, including Enter.
        if key_editor.read(cx).bridge_composition().is_some() {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if key == "tab" && !modifiers.control && !modifiers.platform && !modifiers.alt {
            key_state.borrow_mut().open = false;
            cx.notify(owner);
            return;
        }
        if modifiers.modified() {
            return;
        }
        let open = key_state.borrow().open;
        match key {
            "down" | "up" => {
                let mut state = key_state.borrow_mut();
                if open {
                    state.navigation.navigate(&key_options, key);
                    state.reveal(&key_options);
                } else {
                    state.open(&key_options);
                }
            }
            "escape" if open => key_state.borrow_mut().open = false,
            "enter" if open => {
                let active = key_state.borrow().navigation.active.clone();
                if let Some(id) = active {
                    key_choose(&id, window, cx);
                }
                key_state.borrow_mut().open = false;
            }
            _ => return,
        }
        window.prevent_default();
        cx.stop_propagation();
        cx.notify(owner);
    });
    if !pointer {
        base = base.capture_any_mouse_down(|_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        });
    }
    if popup_state.borrow().open {
        let popup = choice_popup::element(
            choice_popup::Render {
                config: &options,
                appearance,
                state: popup_state.clone(),
                choose,
                owner,
                pointer,
                selected_style,
            },
            window,
        );
        base = base.child(
            deferred(super::popup::Surface {
                trigger: popup_state.borrow().trigger.clone(),
                content: popup.into_any_element(),
            })
            .with_priority(gpui_base::POPUP_PRIORITY),
        );
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtering_preserves_identity_order_and_selection_outside_visible_results() {
        let config = Arc::new(ChoiceConfig {
            label: "Model".into(),
            selected: Some("other".into()),
            disabled: false,
            items: [
                ("other", "Other", false),
                ("one", "Éclair", true),
                ("two", "École", false),
            ]
            .into_iter()
            .map(|(id, label, disabled)| ChoiceItem {
                id: id.into(),
                label: label.into(),
                disabled,
            })
            .collect(),
        });
        let mut state = State::default();
        let filtered = state.options(&config, "é", ComboboxFilter::Substring);
        assert_eq!(
            filtered
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["one", "two"]
        );
        assert_eq!(filtered.selected.as_deref(), Some("other"));
        assert!(!filtered.can_select("one"));
        assert!(filtered.can_select("two"));
        assert!(Arc::ptr_eq(
            &filtered,
            &state.options(&config, "é", ComboboxFilter::Substring)
        ));
        let mut next = config.as_ref().clone();
        next.items.pop();
        let next = Arc::new(next);
        assert_eq!(
            state
                .options(&next, "é", ComboboxFilter::Substring)
                .items
                .len(),
            1
        );
        assert!(Arc::ptr_eq(
            &config,
            &state.options(&config, "é", ComboboxFilter::Unfiltered)
        ));
        assert!(state.cached.is_none());
        assert!(
            state
                .options(&config, "unmatched", ComboboxFilter::Substring)
                .items
                .is_empty()
        );
    }
}
