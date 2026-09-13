//! Application-owned correlated native pickers. Drop owners outside map borrows:
//! native cancellation may synchronously invoke their completion callbacks.
use crate::transport::Transport;
use gpuio_protocol::{WindowId, v1::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(target_os = "macos")]
#[path = "file_panel_macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::Panel;
#[cfg(all(target_os = "macos", feature = "native-tests"))]
pub(crate) use macos::test;

// Compile the Linux ownership adapter in macOS unit tests too. No Linux
// display library is needed for the scalar-parent/request-worker boundary.
#[cfg(any(target_os = "linux", test))]
#[path = "file_dialog_portal.rs"]
pub mod portal;

#[derive(Clone)]
struct Completion {
    window: WindowId,
    correlation: i64,
    done: async_channel::Receiver<()>,
    failed: Arc<AtomicBool>,
}

/// Await before disposing native windows/application resources. Closing the
/// completion channel broadcasts to all cleanup waiters; no receiver steals a
/// single completion value from another shutdown observer.
#[must_use = "await native dialog cleanup before disposing the window/application"]
pub struct Cleanup(Vec<Completion>);

impl Completion {
    fn report(&self) {
        if self.failed.load(Ordering::Acquire) {
            eprintln!(
                "GPUIO_FILE_DIALOG_CLEANUP_FAILED: window={:?} request={}",
                self.window, self.correlation
            );
        }
    }
}

impl Cleanup {
    pub async fn wait(self) {
        for completion in self.0 {
            let _ = completion.done.recv().await;
            completion.report();
        }
    }

    // Only for unconditional GPUI quit. Its shutdown clears windows BEFORE
    // polling quit futures, with a 200 ms limit. Workers here perform only
    // background I/O, so drain them during observer construction, before that
    // disposal. Ordinary close/OCaml shutdown uses the nonblocking async path.
    pub(crate) fn wait_before_quit(self) {
        for completion in self.0 {
            let _ = completion.done.recv_blocking();
            completion.report();
        }
    }
}

#[cfg(target_os = "macos")]
type Pending = std::collections::BTreeMap<WindowId, (i64, Panel)>;

#[derive(Clone, Default)]
pub(crate) struct Dialogs {
    #[cfg(target_os = "macos")]
    pending: std::rc::Rc<std::cell::RefCell<Pending>>,
    #[cfg(target_os = "linux")]
    portal: portal::Dialogs,
}

impl Dialogs {
    pub(crate) fn finish_before_quit(&self) {
        #[cfg(target_os = "macos")]
        self.clear().wait_before_quit();
        #[cfg(target_os = "linux")]
        self.portal.finish_before_quit();
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn show(
        &self,
        correlation: i64,
        id: WindowId,
        config: FileDialogConfig,
        window: &gpui::Window,
        _cx: &gpui::App,
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

    #[cfg(target_os = "linux")]
    pub(crate) fn show(
        &self,
        correlation: i64,
        id: WindowId,
        config: FileDialogConfig,
        window: &gpui::Window,
        cx: &gpui::App,
        transport: Arc<Transport>,
    ) {
        self.portal
            .show(correlation, id, config, window, cx, transport);
    }

    pub(crate) fn close(&self, id: WindowId) -> Cleanup {
        #[cfg(target_os = "macos")]
        {
            let removed = self.pending.borrow_mut().remove(&id);
            drop(removed);
            Cleanup(Vec::new())
        }
        #[cfg(target_os = "linux")]
        {
            self.portal.close(id)
        }
    }

    pub(crate) fn clear(&self) -> Cleanup {
        #[cfg(target_os = "macos")]
        {
            let removed = std::mem::take(&mut *self.pending.borrow_mut());
            drop(removed);
            Cleanup(Vec::new())
        }
        #[cfg(target_os = "linux")]
        {
            self.portal.clear()
        }
    }
}
