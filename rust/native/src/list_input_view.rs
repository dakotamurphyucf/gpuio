//! Persistent list input. OCaml owns cursor/selection; native code owns focus
//! and synchronous key routing. Painted callbacks carry their admitted epoch.
use super::View;
use gpui::{
    AccessibleAction, Context, Div, FocusHandle, Keystroke, Stateful, Window, canvas, prelude::*,
};
use gpuio_protocol::{
    HandlerId, NodeId,
    accessibility::OptionItem,
    list::{Axis, Row},
    list_input::{Config, Confirmation, Gesture, Navigation, Request},
};

#[derive(Clone, Copy, Debug)]
pub(super) struct Route {
    pub(super) owner: NodeId,
    handler: HandlerId,
    generation: i64,
}
impl Route {
    pub(super) fn new(node: &crate::tree::Node) -> Option<Self> {
        let config = node.list_input?;
        (!config.disabled).then_some(Self {
            owner: node.id,
            handler: node.handler?,
            generation: config.generation,
        })
    }
    // GPUI retains a matched mouse-down by element identity. Retiring this ID
    // prevents a mouse-up on a new interaction epoch from completing an old press.
    pub(super) fn row_element_id(self, row: i64) -> gpui::ElementId {
        gpui::SharedString::from(format!(
            "list-option-{row}-{}-{}-{}",
            self.handler.slot(),
            self.handler.generation(),
            self.generation
        ))
        .into()
    }
    fn current(self, node: &crate::tree::Node) -> bool {
        node.id == self.owner
            && node.handler == Some(self.handler)
            && node
                .list_input
                .is_some_and(|config| !config.disabled && config.generation == self.generation)
    }
}
fn toggle(modifiers: gpui::Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.platform
    } else {
        modifiers.control
    }
}
fn gesture(modifiers: gpui::Modifiers) -> Gesture {
    if modifiers.shift {
        Gesture::Range {
            extend: toggle(modifiers),
        }
    } else if toggle(modifiers) {
        Gesture::Toggle
    } else {
        Gesture::Replace
    }
}
fn confirmation(modifiers: gpui::Modifiers) -> Confirmation {
    if toggle(modifiers) {
        Confirmation::Secondary
    } else {
        Confirmation::Primary
    }
}
fn intent(key: &Keystroke, axis: Axis, select_on_navigation: bool, query: bool) -> Option<Request> {
    let m = key.modifiers;
    if m.alt
        || m.function
        || (cfg!(target_os = "macos") && m.control)
        || (!cfg!(target_os = "macos") && m.platform)
    {
        return None;
    }
    let direction = match key.key.as_str() {
        "up" if query || axis == Axis::Vertical => Some(Navigation::Previous),
        "down" if query || axis == Axis::Vertical => Some(Navigation::Next),
        "left" if !query && axis == Axis::Horizontal => Some(Navigation::Previous),
        "right" if !query && axis == Axis::Horizontal => Some(Navigation::Next),
        "home" if !query => Some(Navigation::First),
        "end" if !query => Some(Navigation::Last),
        _ => None,
    };
    if let Some(direction) = direction {
        let selection = if m.shift {
            Some(gesture(m))
        } else if toggle(m) {
            None
        } else {
            select_on_navigation.then_some(Gesture::Replace)
        };
        return Some(Request::Navigate(direction, selection));
    }
    match key.key.as_str() {
        "space" if !query => Some(Request::SelectActive(if m.shift {
            gesture(m)
        } else {
            Gesture::Toggle
        })),
        "enter" if !m.shift => Some(Request::ConfirmActive(confirmation(m))),
        "escape" if !m.modified() => Some(Request::Cancel),
        "f10" if m.shift && !toggle(m) => Some(Request::ContextActive),
        "menu" if !m.modified() => Some(Request::ContextActive),
        _ => None,
    }
}
// Preserve the row handle as an ancestry boundary for retained child controls,
// including section decorations. Bare-row mouse-down must not steal composite
// focus; matched primary option clicks focus the owner, context preserves it.
pub(super) fn preserve_owner_focus(mut row: Stateful<Div>) -> Stateful<Div> {
    for button in [
        gpui::MouseButton::Left,
        gpui::MouseButton::Right,
        gpui::MouseButton::Middle,
    ] {
        row = row.on_mouse_down(button, |_, window, _| window.prevent_default());
    }
    row
}

impl View {
    pub(super) fn list_input_request(&mut self, route: Route, request: Request) -> bool {
        if !self.focus.borrow().allows(route.owner) {
            return false;
        }
        let event = {
            let session = self.session.borrow();
            session.tree(self.id).and_then(|tree| {
                let owner = tree.get(route.owner)?;
                if !route.current(owner) {
                    return None;
                }
                if let Some(target) = request.target() {
                    let row = owner.list_rows.iter().find(|row| row.id == target)?;
                    if !self.focus.borrow().allows(row.node) {
                        return None;
                    }
                }
                session.list_input(
                    self.id,
                    route.owner,
                    route.handler,
                    tree.revision(),
                    route.generation,
                    request,
                )
            })
        };
        let Some(event) = event else {
            return false;
        };
        if !self.transport.input(event) && self.session.borrow_mut().overload(self.id) {
            self.transport.fault(self.id);
        }
        true
    }
    /// Only an explicitly linked query may lend focus to this list. Child
    /// editors keep their native keybindings, including composition cancellation.
    pub(super) fn list_accessibility_owner(
        &self,
        owner: NodeId,
        config: Config,
        window: &Window,
        cx: &gpui::App,
    ) -> Option<FocusHandle> {
        if config.disabled || !self.focus.borrow().allows(owner) {
            return None;
        }
        let root = self.lists.get(&owner)?.borrow().list_focus.clone()?;
        if root.is_focused(window) {
            return Some(root);
        }
        let query = config.query?;
        if !self.focus.borrow().eligible(query) {
            return None;
        }
        let editor = self.editors.get(&query)?;
        let focus = editor.focus_handle(cx);
        (focus.is_focused(window) && !editor.is_composing(cx)).then_some(focus)
    }
    /// Called by the window interceptor after explicit Override commands, before
    /// editor-bound Enter/Escape actions. Returning false leaves native dispatch intact.
    pub(super) fn list_input_key(
        &mut self,
        key: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !window.is_window_active() {
            return false;
        }
        let request = (|| {
            let focused = self.focus.borrow().focused_node(window, cx)?;
            let session = self.session.borrow();
            let tree = session.tree(self.id)?;
            let owner = if tree.get(focused)?.list_input.is_some() {
                focused
            } else {
                tree.list_query_owner(focused)?
            };
            let node = tree.get(owner)?;
            let route = Route::new(node)?;
            if !self.focus.borrow().allows(owner) {
                return None;
            }
            let config = node.list_input?;
            let root = self.lists.get(&owner)?.borrow().list_focus.clone()?;
            let query = if root.is_focused(window) {
                false
            } else {
                let query = config.query?;
                if focused != query || !self.focus.borrow().eligible(query) {
                    return None;
                }
                let editor = self.editors.get(&query)?;
                if !editor.focus_handle(cx).is_focused(window) || editor.is_composing(cx) {
                    return None;
                }
                true
            };
            intent(key, node.list_axis, config.selection_on_navigation, query)
                .map(|request| (route, request))
        })();
        let Some((route, request)) = request else {
            return false;
        };
        if self.list_input_request(route, request) {
            window.prevent_default();
            cx.stop_propagation();
            true
        } else {
            false
        }
    }
    pub(super) fn list_root_input(
        &mut self,
        element: Stateful<Div>,
        route: Route,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let focus = self.lists[&route.owner]
            .borrow()
            .list_focus
            .clone()
            .expect("enabled list focus");
        let recorded = focus.clone();
        let gate = self.focus.clone();
        let weak = cx.weak_entity();
        element
            .track_focus(&focus.clone().tab_stop(true))
            .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
                let _ = weak.update(cx, |view, cx| {
                    let live = view
                        .session
                        .borrow()
                        .tree(view.id)
                        .and_then(|tree| tree.get(route.owner))
                        .is_some_and(|node| route.current(node));
                    if live && view.focus.borrow().allows(route.owner) {
                        focus.focus(window, cx);
                    }
                });
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        if gate.borrow().visible(route.owner) {
                            gate.borrow_mut().record(
                                route.owner,
                                recorded.clone(),
                                true,
                                recorded.is_focused(window),
                                bounds,
                            );
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
    pub(super) fn list_row_input(
        &mut self,
        mut row: Stateful<Div>,
        route: Route,
        binding: Row,
        item: OptionItem,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        if item.disabled {
            return row;
        }
        let Row { id, node } = binding;
        let focus = self.lists[&route.owner]
            .borrow()
            .list_focus
            .clone()
            .expect("enabled list focus");
        let click_focus = focus.clone();
        row = row
            .on_click(
                cx.listener(move |view, event: &gpui::ClickEvent, window, cx| {
                    if matches!(event, gpui::ClickEvent::Keyboard(_))
                        || !view.list_pointer_enabled(node)
                    {
                        return;
                    }
                    // The first click already selected. A double click confirms without
                    // toggling a second time or replacing a range selection.
                    let request = match event.click_count() {
                        1 => Request::Select(id, gesture(event.modifiers())),
                        2 => Request::Confirm(id, confirmation(event.modifiers())),
                        _ => return,
                    };
                    if view.list_input_request(route, request) {
                        click_focus.focus(window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .on_aux_click(cx.listener(move |view, event: &gpui::ClickEvent, _, cx| {
                if event.is_secondary()
                    && view.list_pointer_enabled(node)
                    && view.list_input_request(route, Request::Context(id))
                {
                    cx.stop_propagation();
                }
            }));
        for (action, request) in [
            (AccessibleAction::Focus, Request::Focus(id)),
            (
                AccessibleAction::Click,
                Request::Select(id, Gesture::Replace),
            ),
        ] {
            let weak = cx.weak_entity();
            let focus = focus.clone();
            row = row.on_a11y_action(action, move |_, window, cx| {
                let _ = weak.update(cx, |view, cx| {
                    if view.list_input_request(route, request)
                        && matches!(request, Request::Focus(_))
                    {
                        focus.focus(window, cx);
                    }
                });
            });
        }
        let weak = cx.weak_entity();
        row = row.on_a11y_action(AccessibleAction::CustomAction, move |data, _, cx| {
            use crate::semantics::{
                LIST_CONFIRM, LIST_CONFIRM_SECONDARY, LIST_CONTEXT, LIST_DESELECT, LIST_SELECT,
            };
            let request = match data {
                Some(gpui::accesskit::ActionData::CustomAction(LIST_SELECT)) => {
                    Request::SetSelected(id, true)
                }
                Some(gpui::accesskit::ActionData::CustomAction(LIST_DESELECT)) => {
                    Request::SetSelected(id, false)
                }
                Some(gpui::accesskit::ActionData::CustomAction(LIST_CONFIRM)) => {
                    Request::Confirm(id, Confirmation::Primary)
                }
                Some(gpui::accesskit::ActionData::CustomAction(LIST_CONFIRM_SECONDARY)) => {
                    Request::Confirm(id, Confirmation::Secondary)
                }
                Some(gpui::accesskit::ActionData::CustomAction(LIST_CONTEXT)) => {
                    Request::Context(id)
                }
                _ => return,
            };
            let _ = weak.update(cx, |view, _| view.list_input_request(route, request));
        });
        row
    }
    fn list_pointer_enabled(&self, node: NodeId) -> bool {
        self.focus.borrow().allows(node)
            && self
                .session
                .borrow()
                .tree(self.id)
                .is_some_and(|tree| super::pointer_enabled(tree, node))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axis_query_caret_modifiers_and_confirmation_are_independent() {
        let key = |text: &str| Keystroke::parse(text).unwrap();
        for axis in [Axis::Vertical, Axis::Horizontal] {
            for text in [
                "left",
                "right",
                "home",
                "end",
                "space",
                "tab",
                "shift-tab",
                "a",
            ] {
                assert_eq!(intent(&key(text), axis, true, true), None, "query {text}");
            }
            assert_eq!(
                intent(&key("down"), axis, false, true),
                Some(Request::Navigate(Navigation::Next, None))
            );
            assert_eq!(
                intent(&key("enter"), axis, false, true),
                Some(Request::ConfirmActive(Confirmation::Primary))
            );
            assert_eq!(
                intent(&key("escape"), axis, false, true),
                Some(Request::Cancel)
            );
        }
        for (axis, previous, next) in [
            (Axis::Vertical, "up", "down"),
            (Axis::Horizontal, "left", "right"),
        ] {
            assert_eq!(
                intent(&key(next), axis, true, false),
                Some(Request::Navigate(Navigation::Next, Some(Gesture::Replace)))
            );
            assert_eq!(
                intent(&key(previous), axis, false, false),
                Some(Request::Navigate(Navigation::Previous, None))
            );
            assert_eq!(
                intent(&key(&format!("shift-{next}")), axis, false, false),
                Some(Request::Navigate(
                    Navigation::Next,
                    Some(Gesture::Range { extend: false })
                ))
            );
            let modifier = if cfg!(target_os = "macos") {
                "cmd"
            } else {
                "ctrl"
            };
            assert_eq!(
                intent(&key(&format!("{modifier}-{next}")), axis, true, false),
                Some(Request::Navigate(Navigation::Next, None))
            );
            assert_eq!(
                intent(&key(&format!("{modifier}-shift-{next}")), axis, true, false),
                Some(Request::Navigate(
                    Navigation::Next,
                    Some(Gesture::Range { extend: true })
                ))
            );
            assert_eq!(
                intent(&key(&format!("{modifier}-enter")), axis, true, false),
                Some(Request::ConfirmActive(Confirmation::Secondary))
            );
        }
        assert_eq!(
            intent(&key("space"), Axis::Vertical, false, false),
            Some(Request::SelectActive(Gesture::Toggle))
        );
        assert_eq!(
            intent(&key("shift-f10"), Axis::Vertical, false, false),
            Some(Request::ContextActive)
        );
        for text in [
            "tab",
            "shift-tab",
            "shift-enter",
            "alt-down",
            "alt-space",
            "alt-enter",
            "shift-escape",
        ] {
            assert_eq!(intent(&key(text), Axis::Vertical, true, false), None);
        }
    }
}
