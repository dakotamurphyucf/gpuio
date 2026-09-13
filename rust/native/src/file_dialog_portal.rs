//! Linux portal worker ownership, also compiled/tested on macOS. GPUI native
//! handles are inspected only on the UI thread; workers receive owned data.
use super::{Cleanup, Completion};
use crate::transport::Transport;
use gpuio_protocol::{WindowId, v1::*};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Running,
    Delivering,
    Cancelled,
}
struct Job {
    correlation: i64,
    phase: Phase,
    cancel: async_channel::Sender<()>,
    completion: Completion,
    transport: Arc<Transport>,
}
type Jobs = Arc<Mutex<BTreeMap<WindowId, Job>>>;

#[derive(Clone, Default)]
pub struct Dialogs {
    owner: Arc<Owner>,
}

#[derive(Default)]
struct Owner {
    jobs: Jobs,
}

impl Dialogs {
    pub fn show(
        &self,
        correlation: i64,
        id: WindowId,
        config: FileDialogConfig,
        window: &gpui::Window,
        cx: &gpui::App,
        transport: Arc<Transport>,
    ) {
        let parent = HasWindowHandle::window_handle(window)
            .map_err(|_| FileDialogError::NativeFailure)
            .and_then(|handle| parent(handle.as_raw()));
        let parent = match parent {
            Ok(parent) => parent,
            Err(error) => {
                transport.respond(Event::FileDialogResult(
                    correlation,
                    id,
                    FileDialogResult::Failed(error),
                ));
                return;
            }
        };
        let (cancel, worker) = match self.begin(correlation, id, transport.clone()) {
            Ok(request) => request,
            Err(error) => {
                transport.respond(Event::FileDialogResult(
                    correlation,
                    id,
                    FileDialogResult::Failed(error),
                ));
                return;
            }
        };
        let token = format!("gpuio_{}_{}_{}", id.slot(), id.generation(), correlation);
        cx.background_executor()
            .spawn(async move {
                let result = protect(gpuio_portal::choose(config, &parent, &token, cancel)).await;
                worker.finish(result);
            })
            .detach();
    }

    fn begin(
        &self,
        correlation: i64,
        id: WindowId,
        transport: Arc<Transport>,
    ) -> Result<(async_channel::Receiver<()>, Worker), FileDialogError> {
        let mut jobs = self.owner.jobs.lock().unwrap();
        if jobs.contains_key(&id) {
            return Err(FileDialogError::Busy);
        }
        let (cancel, receive_cancel) = async_channel::bounded(1);
        let (done, receive_done) = async_channel::bounded(1);
        // A dropped/unpolled/panicked worker is a failure until it explicitly
        // records its terminal result. Closing done wakes every cloned waiter.
        let failed = Arc::new(AtomicBool::new(true));
        let completion = Completion {
            window: id,
            correlation,
            done: receive_done,
            failed: failed.clone(),
        };
        jobs.insert(
            id,
            Job {
                correlation,
                phase: Phase::Running,
                cancel,
                completion,
                transport,
            },
        );
        Ok((
            receive_cancel,
            Worker {
                jobs: self.owner.jobs.clone(),
                id,
                correlation,
                _done: done,
                failed,
                finished: false,
            },
        ))
    }

    pub fn close(&self, id: WindowId) -> Cleanup {
        self.owner.cancel(Some(id))
    }
    pub fn clear(&self) -> Cleanup {
        self.owner.cancel(None)
    }
}

impl Owner {
    fn cancel(&self, id: Option<WindowId>) -> Cleanup {
        let (completions, responses) = {
            let mut jobs = self.jobs.lock().unwrap();
            let mut completions = Vec::new();
            let mut responses = Vec::new();
            for (window, job) in jobs.iter_mut() {
                if id.is_some_and(|id| id != *window) {
                    continue;
                }
                completions.push(job.completion.clone());
                if job.phase == Phase::Running {
                    job.phase = Phase::Cancelled;
                    let _ = job.cancel.try_send(());
                    responses.push((job.transport.clone(), job.correlation, *window));
                }
            }
            (completions, responses)
        };
        // No state lock crosses transport delivery or any await. A Delivering
        // job stays tracked until its existing response/wake has finished.
        for (transport, correlation, window) in responses {
            transport.respond(Event::FileDialogResult(
                correlation,
                window,
                FileDialogResult::Failed(FileDialogError::Closed),
            ));
        }
        Cleanup(completions)
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        drop(self.cancel(None));
    }
}

struct Worker {
    jobs: Jobs,
    id: WindowId,
    correlation: i64,
    _done: async_channel::Sender<()>,
    failed: Arc<AtomicBool>,
    finished: bool,
}
impl Worker {
    fn finish(mut self, result: FileDialogResult) {
        self.failed.store(
            matches!(
                result,
                FileDialogResult::Failed(FileDialogError::NativeFailure)
            ),
            Ordering::Release,
        );
        self.deliver(result);
        self.finished = true;
    }
    fn deliver(&self, result: FileDialogResult) {
        let response = {
            let mut jobs = self.jobs.lock().unwrap();
            match jobs.get_mut(&self.id) {
                Some(job) if job.correlation == self.correlation && job.phase == Phase::Running => {
                    job.phase = Phase::Delivering;
                    Some(job.transport.clone())
                }
                Some(_) | None => None,
            }
        };
        if let Some(transport) = response {
            transport.respond(Event::FileDialogResult(self.correlation, self.id, result));
        }
        let mut jobs = self.jobs.lock().unwrap();
        if jobs
            .get(&self.id)
            .is_some_and(|job| job.correlation == self.correlation)
        {
            jobs.remove(&self.id);
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if !self.finished {
            self.failed.store(true, Ordering::Release);
            self.deliver(FileDialogResult::Failed(FileDialogError::NativeFailure));
        }
        // done's sender closes after delivery/retirement, so shutdown cannot
        // dispose its wake pipe while a worker is still producing a response.
    }
}

async fn protect(future: impl Future<Output = FileDialogResult>) -> FileDialogResult {
    let mut future = std::pin::pin!(future);
    std::future::poll_fn(|cx| {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| future.as_mut().poll(cx))) {
            Ok(poll) => poll,
            Err(_) => {
                std::task::Poll::Ready(FileDialogResult::Failed(FileDialogError::NativeFailure))
            }
        }
    })
    .await
}

fn parent(handle: RawWindowHandle) -> Result<String, FileDialogError> {
    let xid = match handle {
        RawWindowHandle::Xlib(handle) => handle.window,
        RawWindowHandle::Xcb(handle) => u64::from(handle.window.get()),
        // Wayland needs an owned exported surface. Never send an empty parent or
        // borrow a raw pointer across the asynchronous portal worker as fallback.
        _ => return Err(FileDialogError::Unsupported),
    };
    if xid == 0 || xid > u64::from(u32::MAX) {
        return Err(FileDialogError::NativeFailure);
    }
    Ok(format!("x11:{xid:x}"))
}

#[cfg(test)]
#[path = "file_dialog_portal/tests.rs"]
mod tests;
