use crate::protocol::{self, Batch, Event};
use binprot::BinProtWrite;
use ocaml_interop::{OCaml, OCamlBytes, OCamlInt, OCamlRuntime, ToOCaml};
use std::{
    collections::VecDeque,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{Mutex, OnceLock},
};

pub enum Command {
    Apply(Batch),
    Probe(i64),
    Quit,
}
struct Bridge {
    tx: async_channel::Sender<Command>,
    rx: async_channel::Receiver<Command>,
    events: Mutex<VecDeque<Event>>,
    wake: Mutex<Option<OwnedFd>>,
}
static BRIDGE: OnceLock<Bridge> = OnceLock::new();
fn bridge() -> &'static Bridge {
    BRIDGE.get_or_init(|| {
        let (tx, rx) = async_channel::bounded(64);
        Bridge {
            tx,
            rx,
            events: Mutex::new(VecDeque::new()),
            wake: Mutex::new(None),
        }
    })
}
pub fn emit(event: Event) {
    bridge().events.lock().unwrap().push_back(event);
    if let Some(fd) = bridge().wake.lock().unwrap().as_ref() {
        // Nonblocking pipe: EAGAIN means a wakeup is already pending. Payloads stay in the queue.
        let byte = [1u8];
        unsafe {
            libc::write(fd.as_raw_fd(), byte.as_ptr().cast(), 1);
        }
    }
}
pub fn request_quit() {
    let _ = send(Command::Quit);
}
pub fn receiver() -> async_channel::Receiver<Command> {
    bridge().rx.clone()
}
pub fn closed() {
    bridge().tx.close();
    emit(Event::Closed);
}
pub fn cleanup() {
    bridge().wake.lock().unwrap().take();
    bridge().events.lock().unwrap().clear();
}
fn send(command: Command) -> Result<(), String> {
    bridge().tx.try_send(command).map_err(|e| e.to_string())
}

#[ocaml_interop::export]
pub fn gpuio_init(_cr: &mut OCamlRuntime, fd: OCaml<OCamlInt>) {
    let fd: i64 = fd.to_rust();
    let duplicate = unsafe { libc::dup(fd as i32) };
    assert!(duplicate >= 0, "dup notification pipe failed");
    let owned = unsafe { OwnedFd::from_raw_fd(duplicate) };
    let flags = unsafe { libc::fcntl(duplicate, libc::F_GETFL) };
    assert!(flags >= 0, "read notification pipe flags failed");
    assert!(unsafe { libc::fcntl(duplicate, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0);
    assert!(unsafe { libc::fcntl(duplicate, libc::F_SETFD, libc::FD_CLOEXEC) } >= 0);
    *bridge().wake.lock().unwrap() = Some(owned);
}
#[ocaml_interop::export]
pub fn gpuio_submit(cr: &mut OCamlRuntime, bytes: OCaml<OCamlBytes>) -> OCaml<OCamlBytes> {
    let bytes: Vec<u8> = bytes.to_rust();
    let error = protocol::decode(&bytes)
        .and_then(|b| send(Command::Apply(b)))
        .err()
        .unwrap_or_default();
    error.into_bytes().to_ocaml(cr)
}
#[ocaml_interop::export]
pub fn gpuio_drain(cr: &mut OCamlRuntime, _unit: OCaml<()>) -> OCaml<OCamlBytes> {
    let events: Vec<_> = bridge().events.lock().unwrap().drain(..).collect();
    let mut bytes = Vec::new();
    events.binprot_write(&mut bytes).unwrap();
    bytes.to_ocaml(cr)
}
#[ocaml_interop::export]
pub fn gpuio_probe(_cr: &mut OCamlRuntime, code: OCaml<OCamlInt>) {
    let code: i64 = code.to_rust();
    if let Err(e) = send(Command::Probe(code)) {
        emit(Event::Error(e));
    }
}
#[ocaml_interop::export]
pub fn gpuio_quit(_cr: &mut OCamlRuntime, _unit: OCaml<()>) {
    let _ = send(Command::Quit);
}
#[ocaml_interop::export]
pub fn gpuio_run(cr: &mut OCamlRuntime, _unit: OCaml<()>) {
    cr.releasing_runtime(|| {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(crate::run)).is_err() {
            closed();
            panic!("GPUI host failed; see native panic log");
        }
    });
}
#[ocaml_interop::export]
pub fn gpuio_cleanup(_cr: &mut OCamlRuntime, _unit: OCaml<()>) {
    cleanup();
}

fn intentional_test_panic() {
    panic!("intentional GPUIO boundary test");
}

// Deliberate failure for the executable self-test; the export wrapper must catch it.
#[ocaml_interop::export]
pub fn gpuio_test_panic(_cr: &mut OCamlRuntime, _unit: OCaml<()>) {
    intentional_test_panic();
}

#[ocaml_interop::export]
pub fn gpuio_run_two_windows(cr: &mut OCamlRuntime, _unit: OCaml<()>) {
    cr.releasing_runtime(|| {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(crate::two_windows::run)).is_err()
        {
            cleanup();
            panic!("two-window foundation smoke failed");
        }
    });
    cleanup();
}
