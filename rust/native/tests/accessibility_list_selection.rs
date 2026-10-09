//! Real AppKit selected setters on a retained NSView adapter, with no
//! NSApplication, OS window, activation, external AX observer or VoiceOver claim.
#[cfg(target_os = "macos")]
mod macos {
    use accesskit_macos::Adapter;
    use gpui::accesskit::{
        Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, CustomAction, Node,
        NodeId, Role, Tree, TreeId, TreeUpdate,
    };
    use objc2::{
        msg_send,
        rc::{Retained, autoreleasepool},
        runtime::AnyObject,
        sel,
    };
    use objc2_app_kit::NSView;
    use objc2_foundation::{MainThreadMarker, NSArray, NSString};
    use std::sync::{Arc, Mutex};
    const SELECT: i32 = 0x4753_0001;
    const DESELECT: i32 = 0x4753_0002;
    struct Actions(Arc<Mutex<Vec<ActionRequest>>>);
    impl ActionHandler for Actions {
        fn do_action(&mut self, request: ActionRequest) {
            self.0.lock().unwrap().push(request);
        }
    }
    struct Initial(Option<TreeUpdate>);
    impl ActivationHandler for Initial {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            self.0.take()
        }
    }
    fn update(role: Role, ids: &[i32], disabled: bool, present: bool) -> TreeUpdate {
        let mut root = Node::new(Role::Window);
        let mut list = Node::new(Role::ListBox);
        list.set_label("Choices");
        list.set_multiselectable();
        root.set_children(vec![NodeId(1)]);
        let mut nodes = vec![];
        if present {
            list.set_children(vec![NodeId(2)]);
            let mut option = Node::new(role);
            option.set_label("Option 世界");
            option.set_selected(true);
            if disabled {
                option.set_disabled();
            } else {
                option.add_action(Action::CustomAction);
                if ids.len() == 2 {
                    option.add_action(Action::Click);
                }
                option.set_custom_actions(
                    ids.iter()
                        .map(|id| CustomAction {
                            id: *id,
                            description: "Select state".into(),
                        })
                        .collect::<Vec<_>>(),
                );
            }
            nodes.push((NodeId(2), option));
        }
        nodes.push((NodeId(1), list));
        nodes.push((NodeId(0), root));
        TreeUpdate {
            nodes,
            tree: Some(Tree::new(NodeId(0))),
            tree_id: TreeId::ROOT,
            focus: NodeId(0),
        }
    }
    fn find(nodes: &NSArray<AnyObject>) -> Option<Retained<AnyObject>> {
        for node in nodes {
            let label: Option<Retained<NSString>> =
                unsafe { msg_send![&*node, accessibilityTitle] };
            if label
                .as_ref()
                .is_some_and(|label| label.to_string() == "Option 世界")
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
    fn set(node: &AnyObject, selected: bool) {
        unsafe {
            let _: () = msg_send![node,setAccessibilitySelected:selected];
        }
    }
    fn allowed(node: &AnyObject) -> bool {
        unsafe { msg_send![node,isAccessibilitySelectorAllowed:sel!(setAccessibilitySelected:)] }
    }
    pub fn run() {
        autoreleasepool(|_| {
            let mtm = MainThreadMarker::new().expect("main-thread harness");
            let view = NSView::new(mtm);
            for (role, ids, opted_in) in [
                (Role::ListBoxOption, vec![SELECT, DESELECT], true),
                (Role::ListBoxOption, vec![SELECT], false),
                (Role::ListBoxOption, vec![], false),
                (Role::Button, vec![SELECT, DESELECT], false),
            ] {
                let actions = Arc::new(Mutex::new(vec![]));
                let mut adapter = unsafe {
                    Adapter::new(
                        (&*view as *const NSView).cast_mut().cast(),
                        true,
                        Actions(actions.clone()),
                    )
                };
                let mut initial = Initial(Some(update(role, &ids, false, true)));
                let children = adapter
                    .view_children(&mut initial)
                    .cast::<NSArray<AnyObject>>();
                let children = unsafe { Retained::retain(children) }.unwrap();
                let option = find(&children).unwrap();
                assert_eq!(
                    allowed(&option),
                    opted_in,
                    "only the complete role/action contract opts in"
                );
                for selected in [false, true, true, false] {
                    set(&option, selected);
                }
                let requests = std::mem::take(&mut *actions.lock().unwrap());
                if opted_in {
                    assert_eq!(
                        requests.len(),
                        4,
                        "every desired state survives until application reduction"
                    );
                    for (request, id) in requests.iter().zip([DESELECT, SELECT, SELECT, DESELECT]) {
                        assert_eq!(request.action, Action::CustomAction);
                        assert_eq!(request.target_node, NodeId(2));
                        assert_eq!(request.data, Some(ActionData::CustomAction(id)));
                    }
                    let selected: bool = unsafe { msg_send![&*option, isAccessibilitySelected] };
                    assert!(selected, "setter must not mutate application snapshot");
                } else {
                    assert!(requests.is_empty());
                }
                drop(
                    adapter
                        .update_if_active(|| update(role, &ids, true, true))
                        .unwrap(),
                );
                assert!(!allowed(&option));
                set(&option, false);
                assert!(actions.lock().unwrap().is_empty());
                drop(
                    adapter
                        .update_if_active(|| update(role, &ids, false, true))
                        .unwrap(),
                );
                assert_eq!(allowed(&option), opted_in);
                drop(
                    adapter
                        .update_if_active(|| update(role, &ids, false, false))
                        .unwrap(),
                );
                assert!(!allowed(&option));
                set(&option, false);
                assert!(actions.lock().unwrap().is_empty());
                drop(adapter);
                set(&option, true);
                assert!(actions.lock().unwrap().is_empty());
            }
        });
        println!(
            "GPUIO_LIST_SELECTION_APPKIT_OK: ordered desired setters, no synchronous selection mutation, role/action opt-in, disabled/recovery/removal/drop; no OS window or VoiceOver claim"
        );
    }
}
#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("macOS selected-setter adapter check is not applicable on this platform");
}
