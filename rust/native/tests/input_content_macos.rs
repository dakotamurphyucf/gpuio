//! Real headless AppKit property/lifetime boundary; no window/autofill claim.
#[cfg(target_os = "macos")]
fn main() {
    use gpuio_native::input_content_macos::{Binding, Unavailable};
    use objc2::{
        ClassType, msg_send,
        rc::{Retained, Weak, autoreleasepool},
        runtime::{AnyObject, ClassBuilder, Sel},
        sel,
    };
    use objc2_app_kit::NSView;
    use objc2_foundation::{MainThreadMarker, NSString};
    use std::ptr;
    let _main = MainThreadMarker::new().expect("AppKit main thread");
    autoreleasepool(|_| {
        let class = ClassBuilder::new(c"GPUIOContentHintFixture", NSView::class())
            .unwrap()
            .register();
        let make = || -> Retained<NSView> { unsafe { msg_send![class, new] } };
        let read = |view: &NSView| -> Option<String> {
            let value: Option<Retained<NSString>> = unsafe { msg_send![view, contentType] };
            value.map(|value| value.to_string())
        };
        let view = make();
        let other = make();
        let mut a = Binding::attach(view.clone()).expect("NSTextContent adapter available");
        a.set(Some("email")).unwrap();
        assert!(a.is_current());
        assert_eq!(read(&view).as_deref(), Some("email"));
        assert!(read(&other).is_none());
        let mut b = Binding::attach(view.clone()).unwrap();
        b.set(Some("username")).unwrap();
        assert!(!a.is_current());
        assert!(b.is_current());
        drop(a);
        assert_eq!(
            read(&view).as_deref(),
            Some("username"),
            "old owner cannot clear new owner"
        );
        b.set(Some("password")).unwrap();
        for bad in ["".to_owned(), "x".repeat(65), "a\0b".into()] {
            assert_eq!(b.set(Some(&bad)), Err(Unavailable::InvalidValue));
            assert_eq!(read(&view).as_deref(), Some("password"));
        }
        // A foreign setter invalidates the current lease. Later stale cleanup
        // must preserve that caller's value, even when it equals the old string.
        let foreign = NSString::from_str("password");
        unsafe {
            let _: () = msg_send![&*view,setContentType:&*foreign];
        }
        assert!(!b.is_current());
        drop(b);
        assert_eq!(read(&view).as_deref(), Some("password"));
        let mut c = Binding::attach(view.clone()).unwrap();
        c.set(Some("url")).unwrap();
        c.set(None).unwrap();
        assert!(read(&view).is_none());
        c.set(Some("tel")).unwrap();
        drop(c);
        assert!(read(&view).is_none());
        // Holding a lease keeps the raw native pointer safe during teardown; it
        // releases the view after clearing its own association.
        let weak = Weak::new(&*other);
        let mut lease = Binding::attach(other).unwrap();
        lease.set(Some("name")).unwrap();
        assert!(weak.load().is_some());
        drop(lease);
        assert!(weak.load().is_none());
        // Never replace inherited/platform/third-party content property methods.
        unsafe extern "C-unwind" fn foreign_getter(_: &AnyObject, _: Sel) -> *mut AnyObject {
            ptr::null_mut()
        }
        let mut conflict = ClassBuilder::new(c"GPUIOContentHintConflict", NSView::class()).unwrap();
        unsafe {
            conflict.add_method(
                sel!(contentType),
                foreign_getter as unsafe extern "C-unwind" fn(_, _) -> _,
            );
        }
        let conflict = conflict.register();
        let view: Retained<NSView> = unsafe { msg_send![conflict, new] };
        assert!(matches!(
            Binding::attach(view.clone()),
            Err(Unavailable::ForeignMethods)
        ));
        assert!(read(&view).is_none());
    });
    println!(
        "GPUIO_CONTENT_HINT_PROPERTY_OK: native getters/setters, per-view ownership, stale cleanup, foreign replacement, bounded input, release and method collision; no OS focus/autofill claim"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("macOS content-hint property test is not applicable");
}
