//! Exports are panic-contained by ocaml-interop. Registry handles never wrap or
//! expose a Rust pointer. Disposal cannot race an active native application.
use crate::transport::Transport;
use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, decode, v1::*};
use ocaml_interop::{OCaml, OCamlBytes, OCamlInt, OCamlRuntime, ToOCaml};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
struct Registry {
    next: i64,
    entries: BTreeMap<i64, Arc<Transport>>,
}
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
static APP_ACTIVE: AtomicBool = AtomicBool::new(false);
fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(Mutex::default)
}
fn lookup(id: i64) -> Arc<Transport> {
    let value = registry()
        .lock()
        .expect("registry poisoned")
        .entries
        .get(&id)
        .cloned();
    value.expect("disposed native runtime")
}
fn claim_run(id: i64) -> Result<Arc<Transport>, &'static str> {
    let registry = registry().lock().expect("registry poisoned");
    let transport = registry.entries.get(&id).ok_or("disposed native runtime")?;
    if transport.finished.load(Ordering::Acquire) {
        return Err("runtime already finished");
    }
    APP_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "only one native application may run")?;
    transport.running.store(true, Ordering::Release);
    Ok(transport.clone())
}
fn insert(transport: Arc<Transport>) -> Result<i64, &'static str> {
    let mut registry = registry().lock().expect("registry poisoned");
    if registry.entries.len() >= 8 {
        return Err("too many native runtime handles");
    }
    let id = registry
        .next
        .checked_add(1)
        .filter(|n| *n < (1_i64 << 62))
        .ok_or("runtime IDs exhausted")?;
    registry.next = id;
    registry.entries.insert(id, transport);
    Ok(id)
}
fn dispose(id: i64) -> Result<(), &'static str> {
    let mut registry = registry().lock().expect("registry poisoned");
    if registry
        .entries
        .get(&id)
        .is_some_and(|transport| transport.running.load(Ordering::Acquire))
    {
        return Err("runtime still running");
    }
    registry.entries.remove(&id);
    Ok(())
}
fn status(result: Result<(), ErrorCode>) -> i64 {
    result.err().map_or(0, |error| error as i64 + 1)
}

#[ocaml_interop::export]
pub fn gpuio_v1_create(cr: &mut OCamlRuntime, fd: OCaml<OCamlInt>) -> OCaml<OCamlInt> {
    let fd: i64 = fd.to_rust();
    let transport = Arc::new(
        Transport::new(i32::try_from(fd).expect("invalid wake fd")).expect("duplicate wake fd"),
    );
    insert(transport)
        .expect("create native runtime")
        .to_ocaml(cr)
}
#[ocaml_interop::export]
pub fn gpuio_v1_create_with_options(
    cr: &mut OCamlRuntime,
    fd: OCaml<OCamlInt>,
    exit_on_last_window: OCaml<bool>,
) -> OCaml<OCamlInt> {
    let fd: i64 = fd.to_rust();
    let transport = Arc::new(
        Transport::with_options(
            i32::try_from(fd).expect("invalid wake fd"),
            exit_on_last_window.to_rust(),
        )
        .expect("duplicate wake fd"),
    );
    insert(transport)
        .expect("create native runtime")
        .to_ocaml(cr)
}
#[ocaml_interop::export]
pub fn gpuio_v1_submit(
    cr: &mut OCamlRuntime,
    id: OCaml<OCamlInt>,
    bytes: OCaml<OCamlBytes>,
) -> OCaml<OCamlInt> {
    let transport = lookup(id.to_rust());
    let bytes = bytes.as_bytes();
    let result = decode(bytes)
        .map_err(|e| match e {
            DecodeError::Malformed => ErrorCode::Malformed,
            DecodeError::LimitExceeded => ErrorCode::LimitExceeded,
        })
        .and_then(|message| transport.submit(message, bytes.len()));
    status(result).to_ocaml(cr)
}
#[ocaml_interop::export]
pub fn gpuio_v1_drain(cr: &mut OCamlRuntime, id: OCaml<OCamlInt>) -> OCaml<OCamlBytes> {
    let transport = lookup(id.to_rust());
    let (events, more) = {
        let mut mailbox = transport.mailbox.lock().expect("mailbox poisoned");
        let events = mailbox.drain(256);
        (events, mailbox.has_output())
    };
    if more {
        transport.wake_ocaml();
    }
    let mut bytes = Vec::new();
    events.binprot_write(&mut bytes).expect("encode events");
    bytes.to_ocaml(cr)
}
#[ocaml_interop::export]
pub fn gpuio_v1_run(cr: &mut OCamlRuntime, id: OCaml<OCamlInt>) {
    #[cfg(target_os = "macos")]
    assert!(
        unsafe { libc::pthread_main_np() } != 0,
        "GPUI must run on the OS main thread"
    );
    let transport = claim_run(id.to_rust()).expect("start native runtime");
    let result = cr.releasing_runtime(|| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::host::run(transport.clone())
        }))
    });
    transport.finish();
    APP_ACTIVE.store(false, Ordering::Release);
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
#[ocaml_interop::export]
pub fn gpuio_v1_dispose(_cr: &mut OCamlRuntime, id: OCaml<OCamlInt>) {
    dispose(id.to_rust()).expect("dispose native runtime");
}

#[ocaml_interop::export]
pub fn gpuio_v1_abort(_cr: &mut OCamlRuntime, id: OCaml<OCamlInt>) {
    let transport = lookup(id.to_rust());
    transport.aborting.store(true, Ordering::Release);
    let _ = transport.tx.try_send(());
}
