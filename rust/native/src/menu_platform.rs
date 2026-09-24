//! Application-wide menu ownership follows the active GPUIO window. Snapshot
//! comparison avoids replacing native menus on unrelated streaming renders.
use super::{View, command::Route};
use gpui::{App, Context, Global, Subscription, Window};
use gpuio_protocol::{NodeId, WindowId, v1::*};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, PartialEq, Eq, gpui::Action)]
#[action(namespace = gpuio, no_json, no_register)]
pub(super) struct Invoke {
    window: WindowId,
    menu: NodeId,
    command: String,
    route: Route,
}
#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    window: WindowId,
    menu: NodeId,
    config: Arc<MenuConfig>,
    // Revision is intentionally omitted: unrelated tree commits must not replace
    // menus. Command/scope generations still guard the installed action payload.
    commands: Vec<(String, NodeId, Arc<CommandConfig>, bool)>,
}
struct Installed {
    snapshot: Option<Snapshot>,
    _closed: Subscription,
}
impl Global for Installed {}
fn initialize(cx: &mut App) {
    if cx.try_global::<Installed>().is_some() {
        return;
    }
    let closed = cx.on_window_closed(|cx, _| {
        if cx.windows().is_empty() {
            cx.set_menus(Vec::<gpui::Menu>::new());
            cx.global_mut::<Installed>().snapshot = None;
        } else if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.refresh());
        }
    });
    cx.set_global(Installed {
        snapshot: None,
        _closed: closed,
    });
}
fn items(menu: &MenuDefinition, routes: &BTreeMap<String, (Invoke, bool)>) -> Vec<gpui::MenuItem> {
    menu.items
        .iter()
        .filter_map(|item| {
            Some(match item {
                MenuItem::Separator => gpui::MenuItem::separator(),
                MenuItem::Submenu(menu) => gpui::MenuItem::submenu(
                    gpui::Menu::new(menu.label.clone())
                        .items(items(menu, routes))
                        .disabled(menu.disabled),
                ),
                MenuItem::Command(id) => {
                    let (action, available) = routes.get(id)?;
                    gpui::MenuItem::action(action.route.config.label.clone(), action.clone())
                        .disabled(!available)
                        .checked(action.route.config.checked == Some(true))
                }
            })
        })
        .collect()
}
impl View {
    pub(super) fn install_menu_observers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        initialize(cx);
        if self.menu_activation.is_none() {
            self.menu_activation =
                Some(cx.observe_window_activation(window, |view, window, cx| {
                    // App menus are global: update ownership when activation is
                    // delivered, even if the surviving window has no new tree data.
                    view.sync_platform_menus(window, cx);
                    cx.notify();
                }));
        }
    }
    fn platform_route(
        &self,
        tree: &crate::tree::Tree,
        menu: NodeId,
        command: &str,
        window: &Window,
        cx: &App,
    ) -> Option<Route> {
        let origin = self
            .focus
            .borrow()
            .focused_node(window)
            .or_else(|| {
                self.editors
                    .iter()
                    .find(|(_, editor)| editor.focus_handle(cx).is_focused(window))
                    .map(|(node, _)| *node)
            })
            .unwrap_or(menu);
        let (scope, config) = tree
            .command(origin, command)
            .or_else(|| tree.command(menu, command))?;
        Some(Route::new(tree, scope, config, CommandSource::Menu(menu)))
    }
    pub(super) fn platform_menu_action(
        &mut self,
        action: &Invoke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if action.window != self.id
            || !window.is_window_active()
            || !self.focus.borrow().visible(action.menu)
        {
            return;
        }
        let current = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return;
            };
            self.platform_route(tree, action.menu, &action.command, window, cx)
        };
        let Some(current) = current else {
            return;
        };
        let old = action.route.request();
        let now = current.request();
        if old.scope != now.scope || old.generation != now.generation {
            return;
        }
        if self.invoke_command(&action.route, window, cx) {
            cx.stop_propagation();
        }
    }
    pub(super) fn sync_platform_menus(&self, window: &Window, cx: &mut Context<Self>) {
        if !cfg!(target_os = "macos") || !window.is_window_active() {
            return;
        }
        initialize(cx);
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return;
        };
        let config = self.menus.keys().find_map(|id| {
            tree.get(*id)
                .and_then(|node| node.menu.as_ref())
                .filter(|config| {
                    config.presentation == MenuPresentation::PlatformBar
                        && self.focus.borrow().visible(*id)
                })
                .map(|config| (*id, config.clone()))
        });
        let Some((id, config)) = config else {
            if cx.global::<Installed>().snapshot.is_some() {
                cx.set_menus(Vec::<gpui::Menu>::new());
                cx.global_mut::<Installed>().snapshot = None;
            }
            return;
        };
        let routes: BTreeMap<_, _> = config
            .command_ids()
            .into_iter()
            .filter_map(|command| {
                let route = self.platform_route(tree, id, command, window, cx)?;
                let available = self.command_available(&route.config, window, cx)
                    && !self.focus.borrow().blocks_pointer(route.request().scope);
                Some((
                    command.to_owned(),
                    (
                        Invoke {
                            window: self.id,
                            menu: id,
                            command: command.to_owned(),
                            route,
                        },
                        available,
                    ),
                ))
            })
            .collect();
        let snapshot = Snapshot {
            window: self.id,
            menu: id,
            config: config.clone(),
            commands: routes
                .iter()
                .map(|(id, (action, enabled))| {
                    (
                        id.clone(),
                        action.route.request().scope,
                        action.route.config.clone(),
                        *enabled,
                    )
                })
                .collect(),
        };
        if cx.global::<Installed>().snapshot.as_ref() == Some(&snapshot) {
            return;
        }
        cx.set_menus(config.menus.iter().map(|menu| {
            gpui::Menu::new(menu.label.clone())
                .items(items(menu, &routes))
                .disabled(menu.disabled)
        }));
        cx.global_mut::<Installed>().snapshot = Some(snapshot);
    }
}
