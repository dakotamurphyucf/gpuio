//! Native semantic-hint storage. Integration with focused editors is separate.
//!
//! Associated metadata is bounded by live NSViews; there is no address-keyed map.
use gpui::Window;
use objc2::{
    ffi, msg_send,
    rc::Retained,
    runtime::{AnyObject, AnyProtocol, Imp, Sel},
    sel,
};
use objc2_app_kit::NSView;
use objc2_foundation::{MainThreadMarker, NSObject, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{ffi::c_void, mem, ptr};

static VALUE_KEY: u8 = 0;
static OWNER_KEY: u8 = 0;
fn value_key() -> *const c_void {
    ptr::from_ref(&VALUE_KEY).cast()
}
fn owner_key() -> *const c_void {
    ptr::from_ref(&OWNER_KEY).cast()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    NotAppKit,
    Protocol,
    ForeignMethods,
    InvalidValue,
}

unsafe extern "C-unwind" fn content_type(this: &AnyObject, _: Sel) -> *mut AnyObject {
    // The association retains the NSString for at least the duration of this
    // main-thread getter, matching the +0 ownership of an Objective-C property.
    unsafe { ffi::objc_getAssociatedObject(this, value_key()).cast_mut() }
}
unsafe extern "C-unwind" fn set_content_type(this: &AnyObject, _: Sel, value: *mut AnyObject) {
    // NSTextContent's property is an NSString. Copy rather than retain a mutable
    // caller string. A foreign setter also relinquishes our old lease identity.
    let value: Option<Retained<NSString>> = if value.is_null() {
        None
    } else {
        let length: usize = unsafe { msg_send![value, length] };
        if length > 64 {
            None
        } else {
            Some(unsafe { msg_send![value, copy] })
        }
    };
    unsafe {
        ffi::objc_setAssociatedObject(
            ptr::from_ref(this).cast_mut(),
            owner_key(),
            ptr::null_mut(),
            ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
        );
        ffi::objc_setAssociatedObject(
            ptr::from_ref(this).cast_mut(),
            value_key(),
            value
                .as_ref()
                .map_or(ptr::null_mut(), |v| Retained::as_ptr(v).cast_mut().cast()),
            ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
        );
    }
}
fn getter_imp() -> Imp {
    unsafe {
        mem::transmute(
            content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel) -> *mut AnyObject,
        )
    }
}
fn setter_imp() -> Imp {
    unsafe {
        mem::transmute(
            set_content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject),
        )
    }
}
fn owns_methods(view: &NSView) -> bool {
    let class = view.class();
    class
        .instance_method(sel!(contentType))
        .is_some_and(|method| ptr::fn_addr_eq(method.implementation(), getter_imp()))
        && class
            .instance_method(sel!(setContentType:))
            .is_some_and(|method| ptr::fn_addr_eq(method.implementation(), setter_imp()))
}
fn install(view: &NSView) -> Result<(), Unavailable> {
    if owns_methods(view) {
        return Ok(());
    }
    let class = view.class();
    if class.instance_method(sel!(contentType)).is_some()
        || class.instance_method(sel!(setContentType:)).is_some()
    {
        return Err(Unavailable::ForeignMethods);
    }
    let protocol = AnyProtocol::get(c"NSTextContent").ok_or(Unavailable::Protocol)?;
    // Main-thread NSView access serializes installation. Add, never replace,
    // methods; incompatible inherited implementations are rejected above.
    unsafe {
        let class = ptr::from_ref(class).cast_mut();
        ffi::class_addProtocol(class, ptr::from_ref(protocol).cast_mut());
        ffi::class_addMethod(class, sel!(contentType), getter_imp(), c"@@:".as_ptr());
        ffi::class_addMethod(class, sel!(setContentType:), setter_imp(), c"v@:@".as_ptr());
    }
    if owns_methods(view) {
        Ok(())
    } else {
        Err(Unavailable::ForeignMethods)
    }
}

/// Main-thread lease on one native view. The host will hold one per window.
/// A superseded lease cannot clear the current owner's value when dropped.
pub struct Binding {
    view: Retained<NSView>,
    owner: Retained<NSObject>,
}
impl Binding {
    pub fn for_window(window: &Window) -> Result<Self, Unavailable> {
        let RawWindowHandle::AppKit(handle) = HasWindowHandle::window_handle(window)
            .map_err(|_| Unavailable::NotAppKit)?
            .as_raw()
        else {
            return Err(Unavailable::NotAppKit);
        };
        // AppKit's raw handle contract supplies a live NSView. Retaining it makes
        // cleanup safe even if the GPUI window is released before the host view.
        let view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
            .ok_or(Unavailable::NotAppKit)?;
        Self::attach(view)
    }
    pub fn attach(view: Retained<NSView>) -> Result<Self, Unavailable> {
        // NSView is main-thread-only; also make the entry requirement explicit.
        let _main = MainThreadMarker::new().expect("native input hint on main thread");
        install(&view)?;
        Ok(Self {
            view,
            owner: NSObject::new(),
        })
    }
    pub fn set(&mut self, value: Option<&str>) -> Result<(), Unavailable> {
        if !owns_methods(&self.view) {
            return Err(Unavailable::ForeignMethods);
        }
        let Some(value) = value else {
            self.clear();
            return Ok(());
        };
        if value.is_empty() || value.len() > 64 || value.contains('\0') {
            return Err(Unavailable::InvalidValue);
        }
        let value = NSString::from_str(value);
        unsafe {
            let _: () = msg_send![&*self.view,setContentType:&*value];
            ffi::objc_setAssociatedObject(
                (&*self.view as *const NSView as *const AnyObject).cast_mut(),
                owner_key(),
                Retained::as_ptr(&self.owner).cast_mut().cast(),
                ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        }
        Ok(())
    }
    /// Whether this lease still owns the property after possible foreign writes.
    pub fn is_current(&self) -> bool {
        owns_methods(&self.view)
            && unsafe {
                ffi::objc_getAssociatedObject(
                    &*self.view as *const NSView as *const AnyObject,
                    owner_key(),
                ) == Retained::as_ptr(&self.owner).cast()
            }
    }
    pub fn clear(&mut self) {
        let view = &*self.view as *const NSView as *const AnyObject;
        unsafe {
            if ffi::objc_getAssociatedObject(view, owner_key())
                == Retained::as_ptr(&self.owner).cast_mut().cast()
            {
                ffi::objc_setAssociatedObject(
                    view.cast_mut(),
                    value_key(),
                    ptr::null_mut(),
                    ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
                );
                ffi::objc_setAssociatedObject(
                    view.cast_mut(),
                    owner_key(),
                    ptr::null_mut(),
                    ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
                );
            }
        }
    }
}
impl Drop for Binding {
    fn drop(&mut self) {
        self.clear();
    }
}
