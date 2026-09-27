//! Tree input is opt-in and asynchronous; native code never owns the hierarchy.
use super::View;
use gpui::{AccessibleAction, Context, Div, FocusHandle, Stateful, Window, canvas, prelude::*};
use gpuio_protocol::{
    NodeId,
    accessibility::TreeItem,
    list::Row,
    tree_input::{Navigation, Request, Selection},
};

pub(super) fn within_input_collection(tree: &crate::tree::Tree, id: NodeId) -> bool {
    let mut current = tree.get(id).and_then(|node| node.parent);
    while let Some(id) = current {
        let Some(node) = tree.get(id) else {
            return false;
        };
        if node.tree_input || node.table.is_some() {
            return true;
        }
        current = node.parent;
    }
    false
}

fn toggle(modifiers: gpui::Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.platform
    } else {
        modifiers.control
    }
}
fn selection(modifiers: gpui::Modifiers) -> Selection {
    if modifiers.shift {
        Selection::Range {
            extend: toggle(modifiers),
        }
    } else if toggle(modifiers) {
        Selection::Toggle
    } else {
        Selection::Replace
    }
}

impl View {
    fn clear_tree_typeahead(&self, owner: NodeId) {
        if let Some(state) = self.lists.get(&owner) {
            state.borrow_mut().tree_typeahead.clear();
        }
    }

    pub(super) fn tree_request(&mut self, owner: NodeId, request: Request) {
        if !matches!(request, Request::Typeahead { .. }) {
            self.clear_tree_typeahead(owner);
        }
        if !self.focus.borrow().allows(owner) {
            return;
        }
        let event = {
            let session = self.session.borrow();
            session.tree(self.id).and_then(|tree| {
                let node = tree.get(owner)?;
                for target in request.targets() {
                    let row = node.list_rows.iter().find(|row| row.id == target)?;
                    if !self.focus.borrow().allows(row.node) {
                        return None;
                    }
                }
                session.tree_input(self.id, owner, node.handler?, tree.revision(), request)
            })
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.id)
        {
            self.transport.fault(self.id);
        }
    }

    fn tree_key(
        &mut self,
        owner: NodeId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !window.is_window_active()
            || !self.focus.borrow().allows(owner)
            || !self
                .lists
                .get(&owner)
                .is_some_and(|state| state.borrow().owns_tree_focus(window))
        {
            self.clear_tree_typeahead(owner);
            return;
        }
        let modifiers = event.keystroke.modifiers;
        // Embedded editors/widgets keep all keys because their own handle, not
        // the exact tree surface/row handle, owns keyboard focus.
        let direction = match event.keystroke.key.as_str() {
            "up" => Some(Navigation::Previous),
            "down" => Some(Navigation::Next),
            "home" => Some(Navigation::First),
            "end" => Some(Navigation::Last),
            "left" => Some(Navigation::Parent),
            "right" => Some(Navigation::Child),
            _ => None,
        };
        let request = if let Some(direction) = direction {
            if modifiers.alt
                || modifiers.function
                || (cfg!(target_os = "macos") && modifiers.control)
                || (!cfg!(target_os = "macos") && modifiers.platform)
            {
                self.clear_tree_typeahead(owner);
                return;
            }
            let gesture = if toggle(modifiers) && !modifiers.shift {
                None
            } else {
                Some(selection(modifiers))
            };
            Some(Request::Navigate(direction, gesture))
        } else {
            match event.keystroke.key.as_str() {
                "space"
                    if !modifiers.alt
                        && !modifiers.function
                        && !(cfg!(target_os = "macos") && modifiers.control)
                        && !(!cfg!(target_os = "macos") && modifiers.platform) =>
                {
                    Some(Request::SelectActive(if modifiers.shift {
                        selection(modifiers)
                    } else {
                        Selection::Toggle
                    }))
                }
                "enter" if !modifiers.modified() => Some(Request::ActivateActive),
                _ if !modifiers.control && !modifiers.platform && !modifiers.function => {
                    let text = event.keystroke.key_char.as_deref().or_else(|| {
                        (!modifiers.alt && event.keystroke.key.chars().count() == 1)
                            .then_some(event.keystroke.key.as_str())
                    });
                    text.and_then(|text| {
                        self.lists[&owner]
                            .borrow_mut()
                            .tree_typeahead
                            .input(text, std::time::Instant::now())
                    })
                }
                _ => None,
            }
        };
        if let Some(request) = request {
            self.tree_request(owner, request);
            cx.stop_propagation();
        } else {
            self.clear_tree_typeahead(owner);
        }
    }

    pub(super) fn tree_root_input(
        &mut self,
        element: Stateful<Div>,
        owner: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        if self.lists[&owner]
            .borrow()
            .tree_typeahead_activation
            .is_none()
        {
            let subscription = cx.observe_window_activation(window, move |view, window, _| {
                if !window.is_window_active() {
                    view.clear_tree_typeahead(owner);
                }
            });
            self.lists[&owner].borrow_mut().tree_typeahead_activation = Some(subscription);
        }
        let focus = self.lists[&owner]
            .borrow()
            .tree_focus
            .clone()
            .expect("enabled tree focus");
        let gate = self.focus.clone();
        let recorded = focus.clone();
        element
            .track_focus(&focus.tab_stop(true))
            .on_key_down(
                cx.listener(move |view, event, window, cx| view.tree_key(owner, event, window, cx)),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        if gate.borrow().visible(owner) {
                            gate.borrow_mut().record(
                                owner,
                                recorded.clone(),
                                true,
                                recorded.is_focused(window),
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

    pub(super) fn tree_row_input(
        &mut self,
        mut row: Stateful<Div>,
        owner: NodeId,
        binding: Row,
        focus: FocusHandle,
        item: TreeItem,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Row { id, node } = binding;
        if item.disabled {
            return row;
        }
        row = self.tree_row_drag(row, owner, binding, item.expanded.is_some(), cx);
        let click_focus = focus.clone();
        row = row.on_click(
            cx.listener(move |view, event: &gpui::ClickEvent, window, cx| {
                if matches!(event, gpui::ClickEvent::Keyboard(_))
                    || !view.focus.borrow().allows(node)
                    || !view
                        .session
                        .borrow()
                        .tree(view.id)
                        .is_some_and(|tree| super::pointer_enabled(tree, node))
                {
                    return;
                }
                click_focus.focus(window, cx);
                view.tree_request(owner, Request::Select(id, selection(event.modifiers())));
                cx.stop_propagation();
            }),
        );
        for (action, request) in [
            (AccessibleAction::Focus, Request::Focus(id)),
            (
                AccessibleAction::Click,
                Request::Select(id, Selection::Replace),
            ),
        ] {
            let weak = cx.weak_entity();
            let focus = focus.clone();
            row = row.on_a11y_action(action, move |_, window, cx| {
                let _ = weak.update(cx, |view, cx| {
                    if !view.focus.borrow().allows(node) {
                        return;
                    }
                    if matches!(request, Request::Focus(_)) {
                        focus.focus(window, cx);
                    }
                    view.tree_request(owner, request.clone());
                });
            });
        }
        let weak = cx.weak_entity();
        row = row.on_a11y_action(AccessibleAction::CustomAction, move |data, _, cx| {
            let selected = match data {
                Some(gpui::accesskit::ActionData::CustomAction(crate::semantics::TREE_SELECT)) => {
                    true
                }
                Some(gpui::accesskit::ActionData::CustomAction(
                    crate::semantics::TREE_DESELECT,
                )) => false,
                _ => return,
            };
            let _ = weak.update(cx, |view, _| {
                view.tree_request(owner, Request::SetSelected(id, selected));
            });
        });
        if item.expanded.is_some() {
            for (action, expanded) in [
                (AccessibleAction::Expand, true),
                (AccessibleAction::Collapse, false),
            ] {
                let weak = cx.weak_entity();
                row = row.on_a11y_action(action, move |_, _, cx| {
                    let _ = weak.update(cx, |view, _| {
                        view.tree_request(owner, Request::SetExpanded(id, expanded))
                    });
                });
            }
        }
        let gate = self.focus.clone();
        row.child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    if gate.borrow().visible(node) {
                        gate.borrow_mut().record(
                            node,
                            focus.clone(),
                            false,
                            focus.is_focused(window),
                        );
                        if focus.is_focused(window) && window.is_window_active() {
                            window.paint_quad(gpui::outline(
                                bounds.inset(gpui::px(1.)),
                                window.text_style().color,
                                gpui::BorderStyle::default(),
                            ));
                        }
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}
