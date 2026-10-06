//! Commands use the mounted observer generation, never a bare node slot.
use super::*;
use gpuio_protocol::menu_command::{Command, Error, Response};

impl View {
    pub(in crate::host) fn menu_command(
        &mut self,
        id: NodeId,
        observer: gpuio_protocol::HandlerId,
        command: &Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Response {
        let config = {
            let session = self.session.borrow();
            let Some(node) = session.tree(self.id).and_then(|tree| tree.get(id)) else {
                return Response::Failed(Error::StaleMenu);
            };
            let Some(config) = node.menu.as_ref().filter(|config| {
                matches!(
                    config.presentation,
                    MenuPresentation::Context | MenuPresentation::PlatformContext
                )
            }) else {
                return Response::Failed(Error::StaleMenu);
            };
            if node.handler != Some(observer) {
                return Response::Failed(Error::StaleMenu);
            }
            config.clone()
        };
        if matches!(command, Command::Close) {
            self.close_menu(id, true, window, cx);
            return Response::Applied;
        }
        let Command::Show(position) = command else {
            unreachable!()
        };
        if !position.is_valid() {
            return Response::Failed(Error::InvalidPosition);
        }
        let Some(state) = self.menus.get(&id).cloned() else {
            return Response::Failed(Error::Unavailable);
        };
        if !window.is_window_active()
            || !self.focus.borrow().interactive(id)
            || !self.focus.borrow().allows(id)
            || state.borrow().config != config
            || config.menus.first().is_none_or(|menu| menu.disabled)
        {
            return Response::Failed(Error::Unavailable);
        }
        if state.borrow().tracking() || !state.borrow().path.is_empty() {
            return Response::Failed(Error::Busy);
        }
        let position = Some(gpui::point(px(position.x as f32), px(position.y as f32)));
        #[cfg(target_os = "macos")]
        if config.presentation == MenuPresentation::PlatformContext {
            return match self.open_platform_popup(id, state, position, window, cx) {
                Ok(()) => Response::Applied,
                Err(error) => Response::Failed(error),
            };
        }
        self.open_menu(id, 0, position, window, cx);
        Response::Applied
    }
}
