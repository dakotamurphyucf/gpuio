//! Snapshot routing between retained menus and the AppKit tracking adapter.
use super::{State, popup};
use crate::host::{View, command::Route};
use gpui::{App, Context, Pixels, Point, Window};
use gpuio_protocol::{NodeId, v1::*};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};

impl View {
    fn popup_items(
        &self,
        tree: &crate::tree::Tree,
        id: NodeId,
        menu: (&MenuDefinition, &[usize]),
        ui: (&mut Window, &mut App),
        routes: &mut Vec<Route>,
        disabled: bool,
    ) -> Vec<popup::Item> {
        let (window, cx) = ui;
        let (definition, path) = menu;
        let disabled = disabled || definition.disabled;
        let node = tree.get(id);
        let indices = node
            .and_then(|node| node.menu.as_ref())
            .and_then(|menu| menu.row_content_indices(path));
        self.menu_rows(tree, id, definition, window, cx)
            .into_iter()
            .zip(&definition.items)
            .enumerate()
            .map(|(index, (row, item))| {
                let icon = indices
                    .as_ref()
                    .and_then(|indices| indices.get(index))
                    .and_then(|index| node?.children.get(1 + index))
                    .and_then(|slot| tree.get(*slot)?.children.first())
                    .and_then(|icon| self.platform_menu_pixels(*icon, window, cx));
                match item {
                    MenuItem::Separator => popup::Item::Separator,
                    MenuItem::Submenu(child) => popup::Item::Submenu {
                        label: row.label,
                        enabled: !disabled && row.enabled,
                        items: {
                            let mut path = path.to_vec();
                            path.push(index);
                            self.popup_items(
                                tree,
                                id,
                                (child, &path),
                                (window, cx),
                                routes,
                                disabled,
                            )
                        },
                        icon,
                    },
                    MenuItem::Command(_) | MenuItem::Label(_) => {
                        let action = row.route.map(|route| {
                            let index = routes.len();
                            routes.push(route);
                            index
                        });
                        popup::Item::Row {
                            label: row.label,
                            enabled: !disabled && row.enabled,
                            checked: row.checked == Some(true),
                            action,
                            icon,
                        }
                    }
                }
            })
            .collect()
    }

    pub(super) fn open_platform_popup(
        &mut self,
        id: NodeId,
        state: Rc<RefCell<State>>,
        position: Option<Point<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), gpuio_protocol::menu_command::Error> {
        use gpuio_protocol::menu_command::Error;
        if popup::busy() {
            return Err(Error::Busy);
        }
        let config = state.borrow().config.clone();
        if state.borrow().tracking()
            || config.menus.first().is_none_or(|menu| menu.disabled)
            || !window.is_window_active()
        {
            return Err(Error::Unavailable);
        }
        let position = position.unwrap_or_else(|| state.borrow().triggers[0].get().bottom_left());
        let mut routes = Vec::new();
        let (items, observer) = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return Err(Error::StaleMenu);
            };
            (
                self.popup_items(
                    tree,
                    id,
                    (&config.menus[0], &[0]),
                    (window, cx),
                    &mut routes,
                    false,
                ),
                tree.get(id).and_then(|node| node.handler),
            )
        };
        let Some((owner, runner)) =
            popup::prepare(&items, window, cx.foreground_executor().clone())
        else {
            return Err(Error::NativeFailure);
        };
        let others: Vec<_> = self
            .menus
            .keys()
            .copied()
            .filter(|other| *other != id)
            .collect();
        for other in others {
            self.close_menu(other, false, window, cx);
        }
        state.borrow_mut().popup = Some(owner);
        self.publish_menu_observation(id, window, cx);
        let handle = window.window_handle();
        let entity = cx.weak_entity();
        let editor = (
            self.command_target_node(window, cx),
            window.focused(cx).map(|focus| focus.downgrade()),
        );
        // A run-loop block releases the main dispatch queue before tracking;
        // AsyncApp borrows are taken only before and after the nested OS loop.
        let expected = Rc::downgrade(&state);
        let mut app = cx.to_async();
        let scheduled = popup::defer(move || {
            let cx = &mut app;
            let state = expected;
            let valid = handle
                .update(cx, |_, window, cx| {
                    entity
                        .update(cx, |view, _| {
                            view.popup_current(id, &state, &config, observer, window)
                        })
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            let selected = if valid { runner.run(position) } else { None };
            let _ = handle.update(cx, |_, window, cx| {
                window.refresh();
                let _ = entity.update(cx, |view, cx| {
                    let current = view.popup_current(id, &state, &config, observer, window);
                    // Only the captured native state is retired, never a new
                    // owner at a recycled node slot or replacement menu.
                    if let Some(state) = state.upgrade() {
                        state.borrow_mut().popup.take();
                    }
                    view.publish_menu_observation(id, window, cx);
                    if !current {
                        return;
                    }
                    if let Some(route) = selected.and_then(|index| routes.get(index)) {
                        if matches!(route.config.target, CommandTarget::Native(_))
                            && (
                                view.command_target_node(window, cx),
                                window.focused(cx).map(|focus| focus.downgrade()),
                            ) != editor
                        {
                            return;
                        }
                        view.invoke_command(route, window, cx);
                    }
                });
            });
        });
        if !scheduled {
            state.borrow_mut().popup.take();
            self.publish_menu_observation(id, window, cx);
            return Err(Error::NativeFailure);
        }
        Ok(())
    }

    fn popup_current(
        &self,
        id: NodeId,
        expected: &Weak<RefCell<State>>,
        config: &Arc<MenuConfig>,
        observer: Option<gpuio_protocol::HandlerId>,
        window: &Window,
    ) -> bool {
        window.is_window_active()
            && self.focus.borrow().interactive(id)
            && self.focus.borrow().allows(id)
            && self.menus.get(&id).is_some_and(|state| {
                expected.ptr_eq(&Rc::downgrade(state)) && state.borrow().tracking()
            })
            && self
                .session
                .borrow()
                .tree(self.id)
                .and_then(|tree| tree.get(id))
                .filter(|node| node.handler == observer)
                .and_then(|node| node.menu.as_ref())
                .is_some_and(|current| Arc::ptr_eq(current, config))
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "menu_popup_test.rs"]
mod tests;
