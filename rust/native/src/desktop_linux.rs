//! Linux desktop workers own descriptors and portal connections, never GPUI
//! windows. Also compiled in macOS unit tests to exercise cleanup locally.
use crate::desktop_operations::Ticket;
use gpuio_protocol::desktop::{Error, Response};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

const MAX_WORKERS: usize = 16;
struct Job {
    cancel: async_channel::Sender<()>,
    done: async_channel::Receiver<()>,
    failed: Arc<AtomicBool>,
}
#[derive(Default)]
struct Inner {
    closed: bool,
    sequence: u64,
    jobs: BTreeMap<u64, Job>,
}
#[derive(Clone, Default)]
pub(crate) struct Services(Arc<Mutex<Inner>>);

#[must_use = "await portal cancellation before finishing native shutdown"]
pub(crate) struct Cleanup(Vec<Completion>);
struct Completion {
    done: async_channel::Receiver<()>,
    failed: Arc<AtomicBool>,
}
impl Completion {
    fn report(&self) {
        if self.failed.load(Ordering::Acquire) {
            eprintln!("GPUIO_DESKTOP_CLEANUP_FAILED: portal dismissal was not confirmed");
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
    pub fn wait_before_quit(self) {
        for completion in self.0 {
            let _ = completion.done.recv_blocking();
            completion.report();
        }
    }
}

struct Worker {
    owner: Weak<Mutex<Inner>>,
    token: u64,
    done: async_channel::Sender<()>,
    ticket: Ticket,
    failed: Arc<AtomicBool>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        // Covers thread-spawn failure and a panicked worker as well as success.
        self.ticket.finish(Response::Failed(Error::NativeFailure));
        if let Some(owner) = self.owner.upgrade() {
            owner
                .lock()
                .expect("desktop workers poisoned")
                .jobs
                .remove(&self.token);
        }
        self.done.close();
    }
}

impl Services {
    pub fn start(
        &self,
        ticket: Ticket,
        work: impl FnOnce(async_channel::Receiver<()>) -> Response + Send + 'static,
    ) {
        let mut inner = self.0.lock().expect("desktop workers poisoned");
        let error = if inner.closed {
            Some(Error::Closed)
        } else if inner.jobs.len() >= MAX_WORKERS {
            Some(Error::Busy)
        } else {
            None
        };
        if let Some(error) = error {
            drop(inner);
            ticket.finish(Response::Failed(error));
            return;
        }
        let Some(token) = inner.sequence.checked_add(1) else {
            drop(inner);
            ticket.finish(Response::Failed(Error::Busy));
            return;
        };
        inner.sequence = token;
        let (cancel, receive_cancel) = async_channel::bounded(1);
        let (done, receive_done) = async_channel::bounded(1);
        let failed = Arc::new(AtomicBool::new(true));
        inner.jobs.insert(
            token,
            Job {
                cancel,
                done: receive_done,
                failed: failed.clone(),
            },
        );
        drop(inner);
        let worker = Worker {
            owner: Arc::downgrade(&self.0),
            token,
            done,
            ticket,
            failed,
        };
        let _ = std::thread::Builder::new()
            .name("gpuio-desktop".into())
            .spawn(move || {
                let result = work(receive_cancel);
                worker.failed.store(
                    matches!(result, Response::Failed(Error::NativeFailure)),
                    Ordering::Release,
                );
                worker.ticket.finish(result);
            });
    }

    pub fn close(&self) -> Cleanup {
        let mut inner = self.0.lock().expect("desktop workers poisoned");
        inner.closed = true;
        let done = inner
            .jobs
            .values()
            .map(|job| {
                job.cancel.close();
                Completion {
                    done: job.done.clone(),
                    failed: job.failed.clone(),
                }
            })
            .collect();
        Cleanup(done)
    }

    pub fn file(
        &self,
        ticket: Ticket,
        path: gpuio_protocol::file_path::FilePath,
        operation: gpuio_portal::desktop::FileOperation,
        correlation: i64,
    ) {
        self.start(ticket, move |cancel| {
            if cancel.is_closed() {
                return Response::Failed(Error::Closed);
            }
            let file = match descriptor(&path) {
                Ok(file) => file,
                Err(error) => return Response::Failed(error),
            };
            let token = format!("gpuio_desktop_{}_{}", std::process::id(), correlation);
            match futures_lite::future::block_on(gpuio_portal::desktop::file(
                file, operation, &token, cancel,
            )) {
                Ok(()) => Response::Requested,
                Err(error) => Response::Failed(error),
            }
        });
    }
}

fn descriptor(path: &gpuio_protocol::file_path::FilePath) -> Result<std::fs::File, Error> {
    use std::{
        ffi::OsStr,
        os::unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        path::Path,
    };
    let io_error = |error: std::io::Error| match error.kind() {
        std::io::ErrorKind::NotFound => Error::Unavailable,
        std::io::ErrorKind::PermissionDenied => Error::Denied,
        _ => Error::NativeFailure,
    };
    #[cfg(target_os = "linux")]
    let flags = libc::O_PATH | libc::O_CLOEXEC;
    #[cfg(not(target_os = "linux"))]
    let flags = libc::O_NONBLOCK | libc::O_CLOEXEC;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(Path::new(OsStr::from_bytes(path.as_bytes())))
        .map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    if !(metadata.is_file() || metadata.is_dir()) {
        return Err(Error::InvalidRequest);
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop_operations::Operations;

    #[test]
    fn an_unacknowledged_cleanup_is_observable_after_the_worker_finishes() {
        let operations = Arc::new(Operations::default());
        let services = Services::default();
        let ticket = operations.admit(1, |_| ()).unwrap();
        services.start(ticket, |cancel| {
            let _ = cancel.recv_blocking();
            Response::Failed(Error::NativeFailure)
        });
        let cleanup = services.close();
        let failed = cleanup.0[0].failed.clone();
        cleanup.wait_before_quit();
        assert!(failed.load(Ordering::Acquire));
        assert!(services.0.lock().unwrap().jobs.is_empty());
    }

    #[test]
    fn shutdown_waits_for_workers_and_rejects_new_work() {
        let operations = Arc::new(Operations::default());
        let services = Services::default();
        let (sent, results) = async_channel::unbounded();
        let (started, entered) = async_channel::bounded(1);
        let ticket = operations
            .admit(1, move |result| sent.send_blocking(result).unwrap())
            .unwrap();
        services.start(ticket, move |cancel| {
            started.send_blocking(()).unwrap();
            let _ = cancel.recv_blocking();
            Response::Failed(Error::Closed)
        });
        entered.recv_blocking().unwrap();
        futures_lite::future::block_on(services.close().wait());
        assert_eq!(
            results.recv_blocking().unwrap(),
            Response::Failed(Error::Closed)
        );
        assert!(services.0.lock().unwrap().jobs.is_empty());
        let ticket = operations
            .admit(2, |result| {
                assert_eq!(result, Response::Failed(Error::Closed))
            })
            .unwrap();
        services.start(ticket, |_| panic!("closed service started work"));
        services.close().wait_before_quit();
    }

    #[test]
    fn non_files_are_rejected_and_missing_paths_report_unavailable() {
        use gpuio_protocol::file_path::FilePath;
        assert!(matches!(
            descriptor(&FilePath::new(b"/dev/null".to_vec()).unwrap()),
            Err(Error::InvalidRequest)
        ));
        let missing = format!("/gpuio-missing-{}", std::process::id());
        assert!(matches!(
            descriptor(&FilePath::new(missing.into_bytes()).unwrap()),
            Err(Error::Unavailable)
        ));
        assert!(descriptor(&FilePath::new(b"/".to_vec()).unwrap()).is_ok());
    }

    #[test]
    fn the_file_adapter_reports_descriptor_failure_without_connecting_to_a_bus() {
        let operations = Arc::new(Operations::default());
        let services = Services::default();
        let (sent, results) = async_channel::bounded(1);
        let ticket = operations
            .admit(1, move |result| sent.send_blocking(result).unwrap())
            .unwrap();
        let missing = gpuio_protocol::file_path::FilePath::new(
            format!("/gpuio-missing-{}", std::process::id()).into_bytes(),
        )
        .unwrap();
        services.file(
            ticket,
            missing,
            gpuio_portal::desktop::FileOperation::Open,
            1,
        );
        assert_eq!(
            results.recv_blocking().unwrap(),
            Response::Failed(Error::Unavailable)
        );
        services.close().wait_before_quit();
    }

    #[test]
    fn worker_admission_is_bounded_and_cleanup_releases_every_slot() {
        let operations = Arc::new(Operations::default());
        let overflow = Arc::new(Operations::default());
        let services = Services::default();
        for id in 1..=16 {
            let ticket = operations
                .admit(id, |response| {
                    assert_eq!(response, Response::Failed(Error::Closed))
                })
                .unwrap();
            services.start(ticket, |cancel| {
                let _ = cancel.recv_blocking();
                Response::Failed(Error::Closed)
            });
        }
        let ticket = overflow
            .admit(17, |response| {
                assert_eq!(response, Response::Failed(Error::Busy))
            })
            .unwrap();
        services.start(ticket, |_| panic!("overflow started a worker"));
        services.close().wait_before_quit();
        assert!(services.0.lock().unwrap().jobs.is_empty());
    }
}
