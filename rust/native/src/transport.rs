use crate::mailbox::Mailbox;
use gpuio_protocol::{WindowId, v1::*};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Transport {
    pub mailbox: Mutex<Mailbox>,
    pub desktop_inbox: Arc<Mutex<crate::desktop_state::DesktopState>>,
    #[cfg(any(target_os = "linux", test))]
    pub(crate) desktop_instance: Mutex<Option<crate::desktop_instance::Lease>>,
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
        let _ = crate::extensions::registry();
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
            desktop_inbox: Arc::default(),
            #[cfg(any(target_os = "linux", test))]
            desktop_instance: Mutex::new(None),
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

    pub(crate) fn respond_desktop(
        &self,
        correlation: i64,
        response: gpuio_protocol::desktop::Response,
    ) {
        let accepted = self
            .mailbox
            .lock()
            .expect("mailbox poisoned")
            .desktop_response(correlation, response);
        if accepted {
            self.wake_ocaml();
        }
    }
    pub fn input(&self, event: Event) -> bool {
        if matches!(event, Event::ColorInputEvent(..)) {
            return self.color_batch(vec![event]);
        }
        let success = self
            .mailbox
            .lock()
            .expect("mailbox poisoned")
            .input(event)
            .is_ok();
        self.wake_ocaml();
        success
    }
    pub fn otp_completion(&self, events: [Event; 2]) -> bool {
        let success = self
            .mailbox
            .lock()
            .expect("mailbox poisoned")
            .otp_completion(events)
            .is_ok();
        self.wake_ocaml();
        success
    }
    pub fn calendar_completion(&self, events: [Event; 2]) -> bool {
        let success = self
            .mailbox
            .lock()
            .expect("mailbox poisoned")
            .calendar_completion(events)
            .is_ok();
        self.wake_ocaml();
        success
    }
    pub fn fault(&self, id: WindowId) {
        self.mailbox.lock().expect("mailbox poisoned").fault(id);
        self.wake_ocaml();
    }
    pub fn color_batch(&self, events: Vec<Event>) -> bool {
        let success = self
            .mailbox
            .lock()
            .expect("mailbox poisoned")
            .color_batch(events)
            .is_ok();
        self.wake_ocaml();
        success
    }
    pub(crate) fn desktop_instance_active(&self) -> bool {
        #[cfg(any(target_os = "linux", test))]
        {
            self.desktop_instance
                .lock()
                .expect("desktop lease poisoned")
                .as_ref()
                .is_some_and(|lease| lease.active())
        }
        #[cfg(not(any(target_os = "linux", test)))]
        {
            false
        }
    }
    pub(crate) fn close_desktop(&self) {
        self.desktop_inbox
            .lock()
            .expect("desktop inbox poisoned")
            .close();
        #[cfg(any(target_os = "linux", test))]
        {
            let lease = self
                .desktop_instance
                .lock()
                .expect("desktop lease poisoned")
                .take();
            drop(lease);
        }
    }
    pub fn finish(&self) {
        self.close_desktop();
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
