//! Owned session-bus lease. Native callbacks enqueue data; no GPUI or OCaml access.
use crate::transport::Transport;
use gpuio_portal::instance::{self, Launch};
use gpuio_protocol::{
    desktop::{Error, LaunchResponse},
    v1::Event,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) struct Lease {
    cancel: async_channel::Sender<()>,
    active: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Lease {
    pub(crate) fn active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.cancel.close();
        if let Some(worker) = self.worker.take() {
            // A callback temporarily upgrades a weak transport. Its last owner
            // may disappear there; cancellation still runs, but never self-join.
            if worker.thread().id() != std::thread::current().id() {
                let _ = worker.join();
            }
        }
    }
}

fn pending(transport: &Transport) {
    transport
        .mailbox
        .lock()
        .expect("mailbox poisoned")
        .control(Event::DesktopPending);
    transport.wake_ocaml();
}

pub(crate) fn prepare(
    transport: &Arc<Transport>,
    identifier: &str,
    links: Vec<String>,
) -> LaunchResponse {
    let weak = Arc::downgrade(transport);
    let admit = Arc::new(move |links: Vec<String>| {
        let transport = weak.upgrade().ok_or(Error::Closed)?;
        let reopen = links.is_empty();
        let notify = transport
            .desktop_inbox
            .lock()
            .expect("desktop inbox poisoned")
            .try_push_batch(links)?;
        if notify {
            pending(&transport);
        }
        if reopen {
            transport
                .mailbox
                .lock()
                .expect("mailbox poisoned")
                .control(Event::ReopenRequested);
            transport.wake_ocaml();
        }
        Ok(())
    });
    match futures_lite::future::block_on(instance::launch(identifier, links, admit)) {
        Err(error) => LaunchResponse::Failed(error),
        Ok(Launch::Forwarded) => LaunchResponse::Forwarded,
        Ok(Launch::Primary(server)) => {
            let (cancel, receiver) = async_channel::bounded(1);
            let active = Arc::new(AtomicBool::new(true));
            let live = active.clone();
            let weak = Arc::downgrade(transport);
            let worker = std::thread::Builder::new()
                .name("gpuio-desktop-instance".into())
                .spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        futures_lite::future::block_on(server.serve(receiver))
                    }))
                    .unwrap_or(Err(Error::NativeFailure));
                    live.store(false, Ordering::Release);
                    if let Err(error) = result
                        && let Some(transport) = weak.upgrade()
                    {
                        let notify = transport
                            .desktop_inbox
                            .lock()
                            .expect("desktop inbox poisoned")
                            .fail(error);
                        if notify {
                            pending(&transport);
                        }
                    }
                });
            match worker {
                Ok(worker) => {
                    *transport
                        .desktop_instance
                        .lock()
                        .expect("desktop lease poisoned") = Some(Lease {
                        cancel,
                        active,
                        worker: Some(worker),
                    });
                    LaunchResponse::Primary
                }
                Err(_) => LaunchResponse::Failed(Error::NativeFailure),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;
    #[test]
    fn invalid_preflight_never_owns_a_worker_or_changes_inbox() {
        let (read, _write) = std::os::unix::net::UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(read.as_raw_fd()).unwrap());
        assert_eq!(
            prepare(&transport, "invalid", vec![]),
            LaunchResponse::Failed(Error::InvalidRequest)
        );
        assert!(!transport.desktop_instance_active());
        assert!(!transport.desktop_inbox.lock().unwrap().pending());
        transport.close_desktop();
    }
}
