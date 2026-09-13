//! Application-owned correlated native pickers. Drop owners outside map borrows:
//! native cancellation may synchronously invoke their completion callbacks.
use crate::transport::Transport;
use gpuio_protocol::{WindowId, v1::*};
use std::sync::Arc;

#[cfg(target_os = "macos")]
#[path = "file_panel_macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::Panel;
#[cfg(all(target_os = "macos", feature = "native-tests"))]
pub(crate) use macos::test;

#[cfg(target_os = "macos")]
type Pending = std::collections::BTreeMap<WindowId, (i64, Panel)>;

#[derive(Default)]
pub(crate) struct Dialogs {
    #[cfg(target_os = "macos")]
    pending: std::rc::Rc<std::cell::RefCell<Pending>>,
}

impl Dialogs {
    #[cfg(target_os = "macos")]
    pub(crate) fn show(
        &self,
        correlation: i64,
        id: WindowId,
        config: FileDialogConfig,
        window: &gpui::Window,
        transport: Arc<Transport>,
    ) {
        if self.pending.borrow().contains_key(&id) {
            transport.respond(Event::FileDialogResult(
                correlation,
                id,
                FileDialogResult::Failed(FileDialogError::Busy),
            ));
            return;
        }
        let pending = std::rc::Rc::downgrade(&self.pending);
        let complete_transport = transport.clone();
        let result = Panel::show(config, window, move |result| {
            if let Some(pending) = pending.upgrade() {
                let removed = {
                    let mut pending = pending.borrow_mut();
                    if pending
                        .get(&id)
                        .is_some_and(|(request, _)| *request == correlation)
                    {
                        pending.remove(&id)
                    } else {
                        None
                    }
                };
                drop(removed);
            }
            complete_transport.respond(Event::FileDialogResult(correlation, id, result));
        });
        match result {
            Ok(panel) => {
                // A native failure/completion can occur during presentation.
                if panel.is_pending() {
                    self.pending.borrow_mut().insert(id, (correlation, panel));
                }
            }
            Err(error) => transport.respond(Event::FileDialogResult(
                correlation,
                id,
                FileDialogResult::Failed(error),
            )),
        }
    }

    // The Linux portal backend is the next implementation step. Keep its result
    // explicit while the correlated protocol/runtime are developed locally.
    #[cfg(not(target_os = "macos"))]
    pub(crate) fn show(
        &self,
        correlation: i64,
        id: WindowId,
        _config: FileDialogConfig,
        _window: &gpui::Window,
        transport: Arc<Transport>,
    ) {
        transport.respond(Event::FileDialogResult(
            correlation,
            id,
            FileDialogResult::Failed(FileDialogError::Unsupported),
        ));
    }

    pub(crate) fn close(&self, _id: WindowId) {
        #[cfg(target_os = "macos")]
        {
            let removed = self.pending.borrow_mut().remove(&_id);
            drop(removed);
        }
    }

    pub(crate) fn clear(&self) {
        #[cfg(target_os = "macos")]
        {
            let removed = std::mem::take(&mut *self.pending.borrow_mut());
            drop(removed);
        }
    }
}

impl Drop for Dialogs {
    fn drop(&mut self) {
        self.clear();
    }
}
