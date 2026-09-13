//! Linux portal worker ownership, also compiled/tested on macOS. GPUI native
//! handles are inspected only on the UI thread. Workers own their export queue;
//! the host's cleanup barrier retains borrowed native parents until they finish.
use super::{Cleanup, Completion};
use crate::transport::Transport;
use gpuio_protocol::{WindowId, v1::*};
#[cfg(feature = "wayland")]
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
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
    #[cfg(feature = "wayland")]
    display: Mutex<Option<(usize, gpuio_wayland::Display)>>,
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
        let parent = match Parent::new(window, &self.owner) {
            Ok(parent) => parent,
            Err(error) => {
                worker.finish(FileDialogResult::Failed(error));
                return;
            }
        };
        let token = format!("gpuio_{}_{}_{}", id.slot(), id.generation(), correlation);
        let executor = cx.background_executor().clone();
        let request = Request {
            parent: Some(parent),
            worker: Some(worker),
        };
        cx.background_executor()
            .spawn(request.run(config, token, cancel, executor))
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

    pub fn finish_before_quit(&self) {
        self.clear().wait_before_quit();
        // No worker/export remains. Release the one shared guest registry
        // before GPUI can dispose its foreign display, even if host tasks or
        // manager clones survive until after App::shutdown returns.
        #[cfg(feature = "wayland")]
        self.owner
            .display
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
    }
}

// An explicit owner also covers a task dropped BEFORE its first poll. Do not
// rely on the capture-field order of an anonymous async block to drop a foreign
// parent before its worker publishes completion.
struct Request {
    parent: Option<Parent>,
    worker: Option<Worker>,
}
impl Request {
    async fn run(
        mut self,
        config: FileDialogConfig,
        token: String,
        cancel: async_channel::Receiver<()>,
        executor: gpui::BackgroundExecutor,
    ) {
        let result = protect(async {
            match self
                .parent
                .as_mut()
                .unwrap()
                .resolve(&cancel, &executor)
                .await
            {
                Ok(identifier) => gpuio_portal::choose(config, &identifier, &token, cancel).await,
                Err(error) => FileDialogResult::Failed(error),
            }
        })
        .await;
        drop(self.parent.take());
        self.worker.take().unwrap().finish(result);
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        drop(self.parent.take());
        // Worker drop reports abandonment and closes its completion channel
        // only after the parent is gone, whether run was ever polled or not.
    }
}

enum Parent {
    X11(String),
    #[cfg(feature = "wayland")]
    Wayland(gpuio_wayland::Export),
}
impl Parent {
    fn new(window: &gpui::Window, _owner: &Owner) -> Result<Self, FileDialogError> {
        let handle =
            HasWindowHandle::window_handle(window).map_err(|_| FileDialogError::NativeFailure)?;
        #[cfg(feature = "wayland")]
        if let RawWindowHandle::Wayland(surface) = handle.as_raw() {
            let display = HasDisplayHandle::display_handle(window)
                .map_err(|_| FileDialogError::NativeFailure)?;
            let RawDisplayHandle::Wayland(display) = display.as_raw() else {
                return Err(FileDialogError::NativeFailure);
            };
            // SAFETY: GPUI supplied both handles from this exact live window.
            // Its host routes every close/abort/shutdown through Dialogs cleanup
            // and drains unconditional quit BEFORE App::shutdown clears windows.
            // Worker completion is published only after this export drops. No
            // detached exporter thread or raw surface outlives that barrier.
            let mut cached = _owner.display.lock().unwrap();
            let identity = display.display.as_ptr() as usize;
            if cached
                .as_ref()
                .is_some_and(|(existing, _)| *existing != identity)
            {
                return Err(FileDialogError::NativeFailure);
            }
            let (_, exporter) = cached.get_or_insert_with(|| {
                (identity, unsafe {
                    gpuio_wayland::Display::new(display.display)
                })
            });
            return unsafe { exporter.export(surface.surface) }
                .map(Self::Wayland)
                .map_err(export_error);
        }
        parent(handle.as_raw()).map(Self::X11)
    }

    async fn resolve(
        &mut self,
        _cancel: &async_channel::Receiver<()>,
        _executor: &gpui::BackgroundExecutor,
    ) -> Result<String, FileDialogError> {
        match self {
            Self::X11(identifier) => Ok(identifier.clone()),
            #[cfg(feature = "wayland")]
            Self::Wayland(export) => {
                futures_lite::future::or(
                    async {
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_secs(5);
                        loop {
                            if let Some(identifier) = export.poll().map_err(export_error)? {
                                return Ok(identifier);
                            }
                            if std::time::Instant::now() >= deadline {
                                return Err(FileDialogError::NativeFailure);
                            }
                            _executor.timer(std::time::Duration::from_millis(10)).await;
                        }
                    },
                    async {
                        let _ = _cancel.recv().await;
                        Err(FileDialogError::Closed)
                    },
                )
                .await
            }
        }
    }
}

#[cfg(feature = "wayland")]
fn export_error(error: gpuio_wayland::Error) -> FileDialogError {
    match error {
        gpuio_wayland::Error::Unsupported => FileDialogError::Unsupported,
        gpuio_wayland::Error::NativeFailure => FileDialogError::NativeFailure,
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
