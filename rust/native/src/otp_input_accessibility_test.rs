//! Read/set the actual AppKit accessibility object, including protected values.
use super::*;
use objc2::{
    msg_send,
    runtime::{AnyObject, Bool},
};
use objc2_foundation::NSString;
#[derive(Debug)]
pub(super) struct Accessible {
    pub value: Option<String>,
    pub subrole: Option<String>,
    pub enabled: bool,
}
pub(super) fn field(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    label: &str,
    set: Option<&str>,
) -> Option<Accessible> {
    unsafe fn text(value: *mut NSString) -> Option<String> {
        unsafe { value.as_ref().map(|value| value.to_string()) }
    }
    unsafe fn visit(
        object: *mut AnyObject,
        label: &str,
        set: Option<&str>,
        depth: usize,
    ) -> Option<Accessible> {
        if object.is_null() || depth > 24 {
            return None;
        }
        unsafe {
            let role: *mut NSString = msg_send![object, accessibilityRole];
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if text(role).as_deref() == Some("AXTextField") && text(title).as_deref() == Some(label)
            {
                let value: *mut NSString = msg_send![object, accessibilityValue];
                let subrole: *mut NSString = msg_send![object, accessibilitySubrole];
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                let result = Accessible {
                    value: text(value),
                    subrole: text(subrole),
                    enabled: enabled.as_bool(),
                };
                if let Some(set) = set {
                    let value = NSString::from_str(set);
                    let _: () = msg_send![object,setAccessibilityValue:&*value];
                }
                return Some(result);
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 256);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children,objectAtIndex:index];
                if let Some(result) = visit(child, label, set, depth + 1) {
                    return Some(result);
                }
            }
        }
        None
    }
    let view = super::super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, label, set, 0)
    }
}
