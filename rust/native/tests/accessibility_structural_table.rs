//! Actual AppKit table properties without an OS window or external AX observer.
#[cfg(target_os = "macos")]
mod macos {
    use accesskit_macos::Adapter;
    use gpui::accesskit::{
        ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Role, Tree, TreeId,
        TreeUpdate,
    };
    use objc2::{
        msg_send,
        rc::{Retained, autoreleasepool},
        runtime::AnyObject,
        sel,
    };
    use objc2_app_kit::NSView;
    use objc2_foundation::{MainThreadMarker, NSArray, NSRange, NSString};

    struct Actions;
    impl ActionHandler for Actions {
        fn do_action(&mut self, _: ActionRequest) {}
    }
    struct Initial(Option<TreeUpdate>);
    impl ActivationHandler for Initial {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            self.0.take()
        }
    }
    fn update(short: bool) -> TreeUpdate {
        let mut window = Node::new(Role::Window);
        window.set_children(vec![NodeId(1)]);
        let mut table = Node::new(Role::Table);
        table.set_label("Summary");
        table.set_row_count(if short { 1 } else { 2 });
        table.set_column_count(if short { 1 } else { 2 });
        table.set_children(vec![NodeId(2)]);
        let mut group = Node::new(Role::RowGroup);
        group.set_children(if short {
            vec![NodeId(3)]
        } else {
            vec![NodeId(3), NodeId(6)]
        });
        let mut head = Node::new(Role::Row);
        head.set_row_index(0);
        head.set_children(vec![NodeId(4)]);
        let mut header = Node::new(Role::ColumnHeader);
        header.set_label("Heading");
        header.set_row_index(0);
        header.set_column_index(0);
        header.set_column_span(if short { 1 } else { 2 });
        let mut nodes = vec![
            (NodeId(0), window),
            (NodeId(1), table),
            (NodeId(2), group),
            (NodeId(3), head),
            (NodeId(4), header),
        ];
        if !short {
            let mut row = Node::new(Role::Row);
            row.set_row_index(1);
            row.set_children(vec![NodeId(7), NodeId(8)]);
            let mut row_head = Node::new(Role::RowHeader);
            row_head.set_label("Project");
            row_head.set_row_index(1);
            row_head.set_column_index(0);
            row_head.set_column_span(1);
            let mut cell = Node::new(Role::Cell);
            cell.set_label("Ready");
            cell.set_row_index(1);
            cell.set_column_index(1);
            cell.set_column_span(1);
            nodes.extend([(NodeId(6), row), (NodeId(7), row_head), (NodeId(8), cell)]);
        }
        TreeUpdate {
            nodes,
            tree: Some(Tree::new(NodeId(0))),
            tree_id: TreeId::ROOT,
            focus: NodeId(0),
        }
    }
    fn find(nodes: &NSArray<AnyObject>, title: &str) -> Option<Retained<AnyObject>> {
        for node in nodes {
            let label: Option<Retained<NSString>> =
                unsafe { msg_send![&*node, accessibilityTitle] };
            if label
                .as_ref()
                .is_some_and(|label| label.to_string() == title)
            {
                return Some(node);
            }
            let children: Option<Retained<NSArray<AnyObject>>> =
                unsafe { msg_send![&*node, accessibilityChildren] };
            if let Some(children) = children
                && let Some(found) = find(&children, title)
            {
                return Some(found);
            }
        }
        None
    }
    pub fn run() {
        autoreleasepool(|_| {
            let view = NSView::new(MainThreadMarker::new().expect("main thread"));
            let mut adapter =
                unsafe { Adapter::new((&*view as *const NSView).cast_mut().cast(), true, Actions) };
            let mut initial = Initial(Some(update(false)));
            let children = adapter
                .view_children(&mut initial)
                .cast::<NSArray<AnyObject>>();
            let children = unsafe { Retained::retain(children) }.unwrap();
            let table = find(&children, "Summary").unwrap();
            let header = find(&children, "Heading").unwrap();
            let cell = find(&children, "Ready").unwrap();
            let row_header = find(&children, "Project").unwrap();
            // External AX dispatch checks selector availability; directly
            // invoking the getter alone misses an omitted role in its gate.
            for selector in [
                sel!(accessibilityRowIndexRange),
                sel!(accessibilityColumnIndexRange),
            ] {
                let allowed: bool =
                    unsafe { msg_send![&*row_header, isAccessibilitySelectorAllowed: selector] };
                assert!(allowed, "row-header range selector must be advertised");
            }
            let range: NSRange = unsafe { msg_send![&*row_header, accessibilityRowIndexRange] };
            assert_eq!((range.location, range.length), (1, 1));
            let range: NSRange = unsafe { msg_send![&*row_header, accessibilityColumnIndexRange] };
            assert_eq!((range.location, range.length), (0, 1));
            let rows: isize = unsafe { msg_send![&*table, accessibilityRowCount] };
            let columns: isize = unsafe { msg_send![&*table, accessibilityColumnCount] };
            assert_eq!((rows, columns), (2, 2));
            let headers: Retained<NSArray<AnyObject>> =
                unsafe { msg_send![&*table, accessibilityColumnHeaderUIElements] };
            let row_headers: Retained<NSArray<AnyObject>> =
                unsafe { msg_send![&*table, accessibilityRowHeaderUIElements] };
            assert_eq!((headers.len(), row_headers.len()), (1, 1));
            let range: NSRange = unsafe { msg_send![&*header, accessibilityColumnIndexRange] };
            assert_eq!((range.location, range.length), (0, 2));
            let range: NSRange = unsafe { msg_send![&*cell, accessibilityRowIndexRange] };
            assert_eq!((range.location, range.length), (1, 1));
            drop(adapter.update_if_active(|| update(true)).unwrap());
            let allowed: bool = unsafe {
                msg_send![&*row_header, isAccessibilitySelectorAllowed: sel!(accessibilityRowIndexRange)]
            };
            assert!(!allowed, "removed row-header selectors must retire");
            let range: NSRange = unsafe { msg_send![&*header, accessibilityColumnIndexRange] };
            assert_eq!((range.location, range.length), (0, 1));
            let range: NSRange = unsafe { msg_send![&*cell, accessibilityColumnIndexRange] };
            assert_eq!((range.location, range.length), (0, 0));
            drop(adapter);
            let range: NSRange = unsafe { msg_send![&*header, accessibilityColumnIndexRange] };
            assert_eq!((range.location, range.length), (0, 0));
        });
        println!(
            "GPUIO_STRUCTURAL_TABLE_APPKIT_OK: table counts, header collections, zero-based merged-cell ranges, update/removal/drop; no OS window or VoiceOver claim"
        );
    }
}
#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("macOS structural table adapter check is not applicable on this platform");
}
