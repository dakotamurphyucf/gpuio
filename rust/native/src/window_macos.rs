//! AppKit's decision hook is distinct from GPUI's post-termination cleanup.
//! Add a no-ivar subclass to this application's delegate instance only. All
//! existing GPUI delegate behavior and ivar layout is inherited unchanged.
use super::*;
use objc2::{
    ffi, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, ClassBuilder, Sel},
    sel,
};
use std::sync::{OnceLock, Weak};
thread_local! {static TRANSPORT:RefCell<Option<Weak<Transport>>>=const{RefCell::new(None)};}
extern "C-unwind" fn should_terminate(_: &AnyObject, _: Sel, _: *mut AnyObject) -> usize {
    // Do not allow Rust panics or an AppKit termination to bypass FFI cleanup.
    // The callback only queues an intent, never borrows GPUI or calls OCaml.
    let _ = std::panic::catch_unwind(|| {
        TRANSPORT.with(|slot| {
            if let Some(transport) = slot.borrow().as_ref().and_then(Weak::upgrade) {
                window_host::control(&transport, Event::QuitRequested);
            }
        });
    });
    0 // NSTerminateCancel; approval later uses embedded stop_application.
}
struct Guard {
    delegate: Retained<AnyObject>,
    original: &'static AnyClass,
}
impl Drop for Guard {
    fn drop(&mut self) {
        TRANSPORT.with(|slot| slot.borrow_mut().take());
        // The strong reference keeps the exact delegate instance alive. The
        // subclass adds no ivars, and only this adapter changes its class.
        unsafe {
            ffi::object_setClass(Retained::as_ptr(&self.delegate).cast_mut(), self.original);
        }
    }
}
pub(super) fn install(cx: &mut App, transport: Arc<Transport>) {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    let delegate = unsafe {
        let app: *mut AnyObject = msg_send![objc2::class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        Retained::retain(delegate).expect("GPUI installed its application delegate")
    };
    let original = delegate.class();
    let class = *CLASS.get_or_init(|| {
        let mut builder = ClassBuilder::new(c"GPUIOApplicationDecisionDelegate", original)
            .expect("unique application delegate class");
        unsafe {
            builder.add_method(
                sel!(applicationShouldTerminate:),
                should_terminate as extern "C-unwind" fn(_, _, _) -> usize,
            );
        }
        builder.register()
    });
    assert_eq!(
        class.superclass(),
        Some(original),
        "delegate class changed between applications"
    );
    TRANSPORT.with(|slot| {
        assert!(slot.borrow().is_none(), "one native application per thread");
        *slot.borrow_mut() = Some(Arc::downgrade(&transport));
    });
    unsafe {
        ffi::object_setClass(Retained::as_ptr(&delegate).cast_mut(), class);
    }
    let mut guard = Some(Guard { delegate, original });
    cx.on_app_quit(move |_| {
        guard.take();
        std::future::ready(())
    })
    .detach();
}
