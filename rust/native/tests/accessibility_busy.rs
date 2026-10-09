//! Actual AppKit attribute boundary without NSApplication, windows or activation.
//! This is not external AX notification delivery or VoiceOver acceptance.
#[cfg(target_os = "macos")]
mod macos {
    use accesskit_macos::Adapter;
    use gpui::accesskit::{
        Action, ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Role, Tree, TreeId,
        TreeUpdate,
    };
    use objc2::{
        msg_send,
        rc::{Retained, autoreleasepool},
        runtime::AnyObject,
        sel,
    };
    use objc2_app_kit::NSView;
    use objc2_foundation::{MainThreadMarker, NSArray, NSString};
    use std::ffi::c_void;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFGetTypeID(value: *const c_void) -> usize;
        fn CFBooleanGetTypeID() -> usize;
    }
    struct NoActions;
    impl ActionHandler for NoActions {
        fn do_action(&mut self, _: ActionRequest) {
            panic!("a getter must not invoke an action");
        }
    }
    struct Initial(Option<TreeUpdate>);
    impl ActivationHandler for Initial {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            self.0.take()
        }
    }
    fn control(role: Role, busy: bool, disabled: bool) -> Node {
        let mut node = Node::new(role);
        node.set_label("Upload 世界");
        node.set_braille_label("Braille label");
        node.set_braille_role_description("Braille role");
        node.set_value("Preserved value");
        node.add_action(Action::Focus);
        node.add_action(Action::Click);
        if role == Role::TextInput {
            node.add_action(Action::SetValue);
        }
        if busy {
            node.set_busy();
        }
        if disabled {
            node.set_disabled();
        }
        node
    }
    fn update(role: Role, busy: bool, disabled: bool, present: bool) -> TreeUpdate {
        let mut root = Node::new(Role::Window);
        let mut nodes = vec![];
        if present {
            root.set_children(vec![NodeId(1)]);
            nodes.push((NodeId(1), control(role, busy, disabled)));
        }
        nodes.push((NodeId(0), root));
        TreeUpdate {
            nodes,
            tree: Some(Tree::new(NodeId(0))),
            tree_id: TreeId::ROOT,
            focus: NodeId(if present { 1 } else { 0 }),
        }
    }
    fn find(array: &NSArray<AnyObject>) -> Option<Retained<AnyObject>> {
        for node in array {
            let title: Option<Retained<NSString>> =
                unsafe { msg_send![&*node, accessibilityTitle] };
            if title
                .as_ref()
                .is_some_and(|title| title.to_string() == "Upload 世界")
            {
                return Some(node);
            }
            let children: Option<Retained<NSArray<AnyObject>>> =
                unsafe { msg_send![&*node, accessibilityChildren] };
            if let Some(children) = children
                && let Some(found) = find(&children)
            {
                return Some(found);
            }
        }
        None
    }
    fn attribute(node: &AnyObject, name: &str) -> Option<Retained<AnyObject>> {
        unsafe { msg_send![node, accessibilityAttributeValue: &*NSString::from_str(name)] }
    }
    fn check(node: &AnyObject, role: Role, busy: bool, disabled: bool) {
        let names: Retained<NSArray<NSString>> =
            unsafe { msg_send![node, accessibilityAttributeNames] };
        assert_eq!(
            names
                .iter()
                .filter(|name| name.to_string() == "AXElementBusy")
                .count(),
            1
        );
        let allowed: bool = unsafe {
            msg_send![node, isAccessibilitySelectorAllowed: sel!(accessibilityAttributeValue:)]
        };
        assert!(allowed);
        let value = attribute(node, "AXElementBusy").expect("busy getter must return a Boolean");
        assert_eq!(
            unsafe { CFGetTypeID((&*value as *const AnyObject).cast()) },
            unsafe { CFBooleanGetTypeID() }
        );
        let actual: bool = unsafe { msg_send![&*value, boolValue] };
        assert_eq!(actual, busy);
        let settable: bool = unsafe {
            msg_send![node, accessibilityIsAttributeSettable: &*NSString::from_str("AXElementBusy")]
        };
        assert!(!settable, "loading is application owned");
        for (name, expected) in [
            ("AXBrailleLabel", "Braille label"),
            ("AXBrailleRoleDescription", "Braille role"),
        ] {
            assert!(
                names
                    .iter()
                    .any(|name_in_list| name_in_list.to_string() == name)
            );
            let value = attribute(node, name).unwrap();
            let expected = NSString::from_str(expected);
            let equal: bool = unsafe { msg_send![&*value, isEqualToString: &*expected] };
            assert!(equal);
        }
        let enabled: bool = unsafe { msg_send![node, isAccessibilityEnabled] };
        assert_eq!(enabled, !disabled);
        let focused: bool = unsafe { msg_send![node, isAccessibilityFocused] };
        assert!(
            focused,
            "the adapter must not confuse loading with focus state"
        );
        let title: Retained<NSString> = unsafe { msg_send![node, accessibilityTitle] };
        assert_eq!(title.to_string(), "Upload 世界");
        let actual_role: Retained<NSString> = unsafe { msg_send![node, accessibilityRole] };
        assert_eq!(
            actual_role.to_string(),
            match role {
                Role::Link => "AXLink",
                Role::TextInput => "AXTextField",
                _ => "AXButton",
            }
        );
        let value: Retained<NSString> = unsafe { msg_send![node, accessibilityValue] };
        assert_eq!(value.to_string(), "Preserved value");
        // AppKit's superclass does not project modern Role/Value into this
        // legacy getter. Its unrelated-attribute behavior remains untouched;
        // modern role/value selectors above are the authoritative getters.
        assert!(attribute(node, "GPUIOUnknownAttribute").is_none());
        let value_settable: bool = unsafe {
            msg_send![node, accessibilityIsAttributeSettable: &*NSString::from_str("AXValue")]
        };
        if !disabled {
            assert_eq!(value_settable, role == Role::TextInput);
        }
        // Upstream text-range writability tracks read-only, independently of
        // disabled action gating; this patch does not change that policy.
    }
    pub fn run() {
        autoreleasepool(|_| {
            let mtm =
                MainThreadMarker::new().expect("test harness must run on process main thread");
            let view = NSView::new(mtm);
            for role in [Role::Button, Role::Link, Role::TextInput] {
                // The view remains retained for the adapter's entire lifetime.
                let mut adapter = unsafe {
                    Adapter::new((&*view as *const NSView).cast_mut().cast(), true, NoActions)
                };
                let mut initial = Initial(Some(update(role, false, false, true)));
                let children = adapter
                    .view_children(&mut initial)
                    .cast::<NSArray<AnyObject>>();
                let children = unsafe { Retained::retain(children) }.unwrap();
                let node = find(&children).expect("single named semantic owner");
                check(&node, role, false, false);
                for (busy, disabled) in [(true, false), (true, true), (false, true), (false, false)]
                {
                    // Deliberately inspect getters only; no external notification receiver.
                    drop(
                        adapter
                            .update_if_active(|| update(role, busy, disabled, true))
                            .unwrap(),
                    );
                    check(&node, role, busy, disabled);
                    let roots = adapter
                        .view_children(&mut initial)
                        .cast::<NSArray<AnyObject>>();
                    let roots = unsafe { Retained::retain(roots) }.unwrap();
                    let current = find(&roots).unwrap();
                    assert!(
                        std::ptr::eq(&*current, &*node),
                        "busy update replaced platform identity"
                    );
                }
                drop(
                    adapter
                        .update_if_active(|| update(role, false, false, false))
                        .unwrap(),
                );
                assert!(
                    attribute(&node, "AXElementBusy").is_none(),
                    "retired node must not expose stale busy state"
                );
                drop(adapter);
                assert!(attribute(&node, "AXElementBusy").is_none());
            }
        });
        println!(
            "GPUIO_ACCESSIBILITY_BUSY_GETTERS_OK: AppKit Boolean, enumeration, read-only, ready/busy/disabled/recovery, role/name/value/focus/Braille identity and retirement; no external AX notification or VoiceOver claim"
        );
    }
}
#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("macOS adapter getter test is not applicable on this platform");
}
