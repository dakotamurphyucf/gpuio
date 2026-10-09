//! Shared AppKit semantic traversal for native chart child tests.
use super::*;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
};
use objc2_foundation::NSString;

unsafe fn find(
    object: *mut AnyObject,
    depth: usize,
    name: &str,
    expected_role: &str,
) -> Option<Retained<AnyObject>> {
    if object.is_null() || depth > 20 {
        return None;
    }
    unsafe {
        let title: *mut NSString = msg_send![object, accessibilityTitle];
        let role: *mut NSString = msg_send![object, accessibilityRole];
        if title.as_ref().is_some_and(|s| s.to_string() == name)
            && role
                .as_ref()
                .is_some_and(|s| s.to_string() == expected_role)
        {
            return Retained::retain(object);
        }
        let children: *mut AnyObject = msg_send![object, accessibilityChildren];
        if children.is_null() {
            return None;
        }
        let count: usize = msg_send![children, count];
        assert!(count < 128);
        for index in 0..count {
            let child: *mut AnyObject = msg_send![children, objectAtIndex:index];
            if let Some(found) = find(child, depth + 1, name, expected_role) {
                return Some(found);
            }
        }
    }
    None
}
pub(super) async fn target_named(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    name: &str,
) -> Retained<AnyObject> {
    target_role(cx, handle, name, "AXButton").await
}

pub(super) async fn target_role(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    name: &str,
    role: &str,
) -> Retained<AnyObject> {
    for _ in 0..100 {
        draw(cx, handle);
        let address = crate::host::editor_test::native_view(cx, handle) as *mut AnyObject;
        let found = unsafe {
            let window: *mut AnyObject = msg_send![address, window];
            let content: *mut AnyObject = msg_send![window, contentView];
            find(content, 0, name, role)
        };
        if let Some(found) = found {
            return found;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("native chart {role} missing: {name}");
}
pub(super) fn press(object: &Retained<AnyObject>) {
    unsafe {
        let accepted: Bool = msg_send![&**object, accessibilityPerformPress];
        assert!(accepted.as_bool());
    }
}
