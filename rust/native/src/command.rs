//! Window-owned command routing. Native shortcut matching, focus gates and edit
//! actions complete on the GPUI thread; application actions are queued to OCaml.
use super::View;
use crate::session::CommandInvocation;
use gpui::{App, Context, Focusable, Keystroke, Window};
use gpuio_protocol::{HandlerId, NodeId, v1::*};
use std::{collections::BTreeSet, sync::Arc};

#[cfg(test)]
#[path = "command_lifetime_test.rs"]
mod lifetime_test;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Route {
    scope: NodeId,
    handler: HandlerId,
    revision: i64,
    pub(super) config: Arc<CommandConfig>,
    source: CommandSource,
}
impl Route {
    pub(super) fn new(
        tree: &crate::tree::Tree,
        scope: NodeId,
        config: &Arc<CommandConfig>,
        source: CommandSource,
    ) -> Self {
        Self {
            scope,
            handler: tree
                .get(scope)
                .expect("scope")
                .handler
                .expect("registry handler"),
            revision: tree.revision(),
            config: config.clone(),
            source,
        }
    }
    pub(super) fn request(&self) -> CommandInvocation<'_> {
        CommandInvocation {
            scope: self.scope,
            handler: self.handler,
            revision: self.revision,
            command: &self.config.id,
            generation: self.config.generation,
            source: self.source,
        }
    }
}
fn keybinding(shortcut: &Shortcut) -> gpui::KeybindingKeystroke {
    let mut modifiers = gpui::Modifiers::default();
    for modifier in &shortcut.modifiers {
        match modifier {
            ShortcutModifier::Primary => {
                #[cfg(target_os = "macos")]
                {
                    modifiers.platform = true;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    modifiers.control = true;
                }
            }
            ShortcutModifier::Control => modifiers.control = true,
            ShortcutModifier::Alt => modifiers.alt = true,
            ShortcutModifier::Shift => modifiers.shift = true,
            ShortcutModifier::Super => modifiers.platform = true,
        }
    }
    gpui::KeybindingKeystroke::from_keystroke(Keystroke {
        key: shortcut.key.clone(),
        key_char: None,
        modifiers,
    })
}
impl View {
    pub(super) fn install_command_interceptor(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.command_subscription.is_some() {
            return;
        }
        let target = window.window_handle();
        let owner = cx.weak_entity();
        self.command_subscription = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle() != target {
                return;
            }
            let _ = owner.update(cx, |view, cx| {
                if event.keystroke.key == "escape"
                    && !event.keystroke.modifiers.modified()
                    && super::drag_drop::cancel(
                        view.id,
                        gpuio_protocol::drag_drop::CancelReason::Escape,
                        window,
                        cx,
                    )
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "escape"
                    && !event.keystroke.modifiers.modified()
                    && view
                        .pointer_capture
                        .borrow_mut()
                        .cancel(PointerCancel::Escape, window)
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    return;
                }
                view.command_shortcut(&event.keystroke, ShortcutPriority::Override, window, cx)
            });
        }));
    }
    pub(super) fn command_editor(&self, window: &Window, cx: &App) -> Option<NodeId> {
        self.editors
            .iter()
            .find(|(_, editor)| editor.focus_handle(cx).is_focused(window))
            .map(|(node, _)| *node)
            .or_else(|| self.focus.borrow().last_editor())
            .filter(|node| self.editors.contains_key(node) && self.focus.borrow().allows(*node))
    }
    pub(super) fn command_available(
        &self,
        config: &CommandConfig,
        window: &Window,
        cx: &App,
    ) -> bool {
        if !config.enabled {
            return false;
        }
        let CommandTarget::Native(action) = config.target else {
            return true;
        };
        let Some(editor) = self.command_editor(window, cx) else {
            return false;
        };
        self.editors[&editor].command_available(action, cx)
    }

    pub(super) fn invoke_command(
        &mut self,
        route: &Route,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let source = match route.source {
            CommandSource::Button(node)
            | CommandSource::Palette(node)
            | CommandSource::Menu(node) => node,
            CommandSource::Shortcut => route.scope,
        };
        let allowed = match route.source {
            CommandSource::Menu(menu) => {
                if !self.focus.borrow().visible(menu) {
                    return false;
                }
                let platform = self
                    .session
                    .borrow()
                    .tree(self.id)
                    .and_then(|tree| tree.get(menu))
                    .and_then(|node| node.menu.as_ref())
                    .is_some_and(|config| config.presentation == MenuPresentation::PlatformBar);
                if platform {
                    !self.focus.borrow().blocks_pointer(route.scope)
                } else {
                    self.focus.borrow().allows(source)
                }
            }
            CommandSource::Palette(_) => !self.focus.borrow().blocks_pointer(route.scope),
            CommandSource::Button(_) => self.focus.borrow().allows(source),
            CommandSource::Shortcut => !self.focus.borrow().blocks_pointer(source),
        };
        if !allowed || !self.command_available(&route.config, window, cx) {
            return false;
        }
        let target = self
            .session
            .borrow()
            .command_target(self.id, route.request());
        match target {
            Some(CommandTarget::Callback) => {
                let event = self
                    .session
                    .borrow()
                    .invoke_command(self.id, route.request());
                let Some(event) = event else {
                    return false;
                };
                if !self.transport.input(event) && self.session.borrow_mut().overload(self.id) {
                    self.transport.fault(self.id);
                }
                true
            }
            Some(CommandTarget::Native(action)) => {
                let Some(editor) = self.command_editor(window, cx) else {
                    return false;
                };
                // Toolbar/button activation may have moved focus. Native edit
                // actions deliberately return it to the retained editing target.
                window.focus(&self.editors[&editor].focus_handle(cx), cx);
                let action: Box<dyn gpui::Action> = match action {
                    NativeCommand::Copy => Box::new(gpui_base::input::Copy),
                    NativeCommand::Cut => Box::new(gpui_base::input::Cut),
                    NativeCommand::Paste => Box::new(gpui_base::input::Paste),
                    NativeCommand::SelectAll => Box::new(gpui_base::input::SelectAll),
                    NativeCommand::Undo => Box::new(gpui_base::input::Undo),
                    NativeCommand::Redo => Box::new(gpui_base::input::Redo),
                };
                window.dispatch_action(action, cx);
                true
            }
            None => false,
        }
    }
    pub(super) fn command_shortcut(
        &mut self,
        key: &Keystroke,
        priority: ShortcutPriority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focused = self.focus.borrow().focused_node(window);
        if priority == ShortcutPriority::NativeFirst {
            // Tab is handled by the native traversal adapter. Buttons synthesize
            // Enter/Space clicks on key-up, after raw key-down bubbling.
            let tab = key.key == "tab"
                && !key.modifiers.control
                && !key.modifiers.platform
                && !key.modifiers.alt;
            let activate = !key.modifiers.modified()
                && matches!(key.key.as_str(), "enter" | "space")
                && focused.is_some_and(|id| {
                    self.session
                        .borrow()
                        .tree(self.id)
                        .and_then(|tree| tree.get(id))
                        .is_some_and(|node| {
                            matches!(
                                node.kind,
                                Kind::Button | Kind::CommandButton | Kind::Checkbox | Kind::Switch
                            )
                        })
                });
            if tab || activate {
                return;
            }
        }
        let editor = self
            .editors
            .values()
            .find(|editor| editor.focus_handle(cx).is_focused(window));
        let palette = self.palettes.values().find(|state| {
            !state.closed && state.query.read(cx).focus_handle(cx).is_focused(window)
        });
        let composing = editor.is_some_and(|editor| editor.is_composing(cx))
            || palette.is_some_and(|state| state.query.read(cx).bridge_composition().is_some());
        let editing = editor.is_some() || palette.is_some();
        let route = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                return;
            };
            let mut cursor = focused.or(tree.root());
            let mut seen = BTreeSet::new();
            let mut matched = None;
            'scopes: while let Some(id) = cursor {
                let Some(node) = tree.get(id) else {
                    break;
                };
                if let Some(commands) = &node.commands {
                    for command in commands.iter() {
                        if !seen.insert(command.id.as_str()) {
                            continue;
                        }
                        if command.shortcuts.iter().any(|shortcut| {
                            let allow_text = match shortcut.text_input {
                                ShortcutTextInput::Always => true,
                                ShortcutTextInput::Never => !editing,
                                ShortcutTextInput::ModifiedOnly => {
                                    !editing
                                        || key.modifiers.control
                                        || key.modifiers.platform
                                        || key.modifiers.alt
                                }
                            };
                            shortcut.priority == priority
                                && (!composing || shortcut.during_composition)
                                && allow_text
                                && key.should_match(&keybinding(shortcut))
                        }) {
                            matched = Some(Route::new(tree, id, command, CommandSource::Shortcut));
                            break 'scopes;
                        }
                    }
                }
                cursor = node.parent;
            }
            matched
        };
        if let Some(route) = route
            && self.invoke_command(&route, window, cx)
        {
            cx.stop_propagation();
        }
    }
}
