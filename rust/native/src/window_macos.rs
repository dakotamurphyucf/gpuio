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

pub(super) fn install_text_input_reset(window: &mut Window) {
    window.on_text_input_reset(|window| {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return;
        };
        // SAFETY: GPUI invokes this on its main thread with a live NSView and
        // no installed input handler. The previous client's marked range has
        // already been cleared. AppKit owns the returned input context.
        unsafe {
            let view = handle.ns_view.cast::<AnyObject>().as_ref();
            let context: *mut AnyObject = msg_send![view, inputContext];
            if !context.is_null() {
                let _: () = msg_send![context, discardMarkedText];
            }
        }
    });
}

fn native_window(
    window: &Window,
) -> Result<Retained<objc2_app_kit::NSWindow>, gpuio_protocol::window::Error> {
    use gpuio_protocol::window::Error;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    objc2::MainThreadMarker::new().ok_or(Error::NativeFailure)?;
    let handle = HasWindowHandle::window_handle(window).map_err(|_| Error::NativeFailure)?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err(Error::Unsupported);
    };
    // SAFETY: GPUI owns this live NSView, accessed on its main thread. Retain
    // the native window before using it independently of the borrowed handle.
    let view = unsafe { handle.ns_view.cast::<objc2_app_kit::NSView>().as_ref() };
    view.window().ok_or(Error::Closed)
}

pub(super) fn document(window: &Window) -> Option<gpuio_protocol::window::Document> {
    use std::os::unix::ffi::OsStrExt;
    let native = native_window(window).ok()?;
    let path = match native.representedURL() {
        Some(url) => {
            let path = url.to_file_path()?;
            Some(
                gpuio_protocol::file_path::FilePath::new(path.as_os_str().as_bytes().to_vec())
                    .ok()?,
            )
        }
        None => None,
    };
    Some(gpuio_protocol::window::Document {
        path,
        edited: native.isDocumentEdited(),
    })
}

pub(super) fn set_document(
    window: &Window,
    document: &gpuio_protocol::window::Document,
) -> Result<(), gpuio_protocol::window::Error> {
    use gpuio_protocol::window::Error;
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};
    let native = native_window(window)?;
    // Convert before mutating either field. NSURL uses filesystem bytes rather
    // than a lossy UTF-8 conversion; no file is read, created or saved.
    let url = document
        .path
        .as_ref()
        .map(|path| {
            objc2_foundation::NSURL::from_file_path(Path::new(OsStr::from_bytes(path.as_bytes())))
                .ok_or(Error::InvalidRequest)
        })
        .transpose()?;
    native.setRepresentedURL(url.as_deref());
    native.setDocumentEdited(document.edited);
    Ok(())
}

pub(super) fn set_edited(
    window: &Window,
    edited: bool,
) -> Result<(), gpuio_protocol::window::Error> {
    native_window(window)?.setDocumentEdited(edited);
    Ok(())
}
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
