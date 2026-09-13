use crate::mailbox::Mailbox;
use gpuio_protocol::{WindowId, v1::*};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Transport {
    pub mailbox: Mutex<Mailbox>,
    pub tx: async_channel::Sender<()>,
    pub rx: async_channel::Receiver<()>,
    wake: OwnedFd,
    pub running: AtomicBool,
    pub aborting: AtomicBool,
    pub finished: AtomicBool,
    pub exit_on_last_window: bool,
}
impl Transport {
    pub fn new(fd: i32) -> std::io::Result<Self> {
        Self::with_options(fd, true)
    }
    pub fn with_options(fd: i32, exit_on_last_window: bool) -> std::io::Result<Self> {
        let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let wake = unsafe { OwnedFd::from_raw_fd(duplicate) };
        let flags = unsafe { libc::fcntl(duplicate, libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(duplicate, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let (tx, rx) = async_channel::bounded(1);
        Ok(Self {
            mailbox: Mutex::new(Mailbox::default()),
            tx,
            rx,
            wake,
            running: AtomicBool::new(false),
            aborting: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            exit_on_last_window,
        })
    }
    pub fn submit(&self, message: Message, bytes: usize) -> Result<(), ErrorCode> {
        self.mailbox
            .lock()
            .expect("mailbox poisoned")
            .submit(message, bytes)?;
        let _ = self.tx.try_send(());
        Ok(())
    }
    pub fn wake_ocaml(&self) {
        let byte = [1u8];
        loop {
            let result = unsafe { libc::write(self.wake.as_raw_fd(), byte.as_ptr().cast(), 1) };
            if result >= 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                break;
            }
        }
    }
    pub fn respond(&self, event: Event) {
        self.mailbox
            .lock()
            .expect("mailbox poisoned")
            .respond(event);
        self.wake_ocaml();
    }
    pub fn input(&self, event: Event) -> bool {
        let success = self
            .mailbox
            .lock()
            .expect("mailbox poisoned")
            .input(event)
            .is_ok();
        self.wake_ocaml();
        success
    }
    pub fn fault(&self, id: WindowId) {
        self.mailbox.lock().expect("mailbox poisoned").fault(id);
        self.wake_ocaml();
    }
    pub fn finish(&self) {
        self.mailbox
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .close();
        self.finished.store(true, Ordering::Release);
        self.running.store(false, Ordering::Release);
        self.tx.close();
        self.wake_ocaml();
    }
}
