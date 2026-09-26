//! Production managed-list tree semantics through an actual GPUI window.
use super::*;
use gpuio_protocol::accessibility::{Config as Metadata, Live, Role, TreeItem};
use gpuio_protocol::list::{Config, IdRun, Order, Row, ScrollPolicy};

fn metadata(role: Role, label: &str) -> Metadata {
    Metadata {
        role: Some(role),
        label: Some(label.into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    }
}
fn item(level: i64, expanded: Option<bool>, selected: bool, disabled: bool) -> TreeItem {
    TreeItem {
        level,
        index: 0,
        count: None,
        expanded,
        selected,
        disabled,
        busy: false,
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, PartialEq, Eq)]
struct AxRow {
    label: String,
    level: Option<isize>,
    expanded: Option<bool>,
    disclosed: Option<bool>,
    selected: bool,
    enabled: bool,
}

#[cfg(target_os = "macos")]
fn rows(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Vec<AxRow> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
        sel,
    };
    use objc2_foundation::NSString;
    unsafe fn visit(object: *mut AnyObject, found: &mut Vec<AxRow>, depth: usize) {
        if object.is_null() || depth > 32 {
            return;
        }
        unsafe {
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !role.is_null() && (*role).to_string() == "AXRow" {
                let label: *mut NSString = msg_send![object, accessibilityTitle];
                let level: Bool =
                    msg_send![object, respondsToSelector:sel!(accessibilityDisclosureLevel)];
                let expanded: Bool =
                    msg_send![object, isAccessibilitySelectorAllowed:sel!(isAccessibilityExpanded)];
                let disclosed: Bool =
                    msg_send![object, respondsToSelector:sel!(isAccessibilityDisclosed)];
                let disclosed = if disclosed.as_bool() {
                    let allowed: Bool = msg_send![object, isAccessibilitySelectorAllowed:sel!(isAccessibilityDisclosed)];
                    allowed.as_bool()
                } else {
                    false
                };
                let selected: Bool = msg_send![object, isAccessibilitySelected];
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                found.push(AxRow {
                    label: if label.is_null() {
                        String::new()
                    } else {
                        (*label).to_string()
                    },
                    level: level
                        .as_bool()
                        .then(|| msg_send![object, accessibilityDisclosureLevel]),
                    expanded: expanded.as_bool().then(|| {
                        let value: Bool = msg_send![object, isAccessibilityExpanded];
                        value.as_bool()
                    }),
                    disclosed: disclosed.then(|| {
                        let value: Bool = msg_send![object, isAccessibilityDisclosed];
                        value.as_bool()
                    }),
                    selected: selected.as_bool(),
                    enabled: enabled.as_bool(),
                });
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex:index];
                visit(child, found, depth + 1);
            }
        }
    }
    let view = super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    let mut found = vec![];
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, &mut found, 0);
    }
    found
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let mut operations = vec![Op::SetRoot(None)];
    operations.extend((0..=4).map(|id| Op::Remove(node(id))));
    operations.extend([
        Op::Create(
            node(5),
            Kind::VirtualList,
            String::new(),
            Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
        ),
        Op::SetListConfig(
            node(5),
            Config {
                estimated_height: 32.,
                overscan: 0.,
                max_active: 8,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: true,
            },
        ),
        Op::SetListOrder(
            node(5),
            Order {
                revision: 1,
                runs: vec![IdRun { first: 1, count: 3 }],
            },
        ),
        Op::SetStyle(
            node(5),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(380.)),
                Field::Height(Length::Px(180.)),
            ])],
        ),
        Op::SetAccessibility(node(5), Some(metadata(Role::Tree(true), "Projects"))),
        Op::Create(node(6), Kind::Container, String::new(), None),
        Op::SetAccessibility(
            node(6),
            Some(metadata(
                Role::TreeItem(item(1, Some(true), true, false)),
                "Folder",
            )),
        ),
        Op::Create(node(7), Kind::Text, "Folder".into(), None),
        Op::Splice(node(6), 0, 0, vec![node(7)]),
        Op::Create(node(8), Kind::Container, String::new(), None),
        Op::SetAccessibility(
            node(8),
            Some(metadata(
                Role::TreeItem(item(2, None, false, true)),
                "Child",
            )),
        ),
        Op::Create(node(9), Kind::Text, "Child".into(), None),
        Op::Splice(node(8), 0, 0, vec![node(9)]),
        Op::Create(node(10), Kind::Text, "Load more".into(), None),
        Op::SetAccessibility(node(10), Some(metadata(Role::Status, "Load more"))),
        Op::SetListRows(
            node(5),
            vec![
                Row {
                    id: 1,
                    node: node(6),
                },
                Row {
                    id: 2,
                    node: node(8),
                },
                Row {
                    id: 3,
                    node: node(10),
                },
            ],
        ),
        Op::Splice(node(5), 0, 0, vec![node(6), node(8), node(10)]),
        Op::SetRoot(Some(node(5))),
    ]);
    for slot in [6, 8, 10] {
        operations.push(Op::SetStyle(
            node(slot),
            vec![Style::Fields(vec![
                Field::Height(Length::Px(32.)),
                Field::Shrink(0.),
            ])],
        ));
    }
    apply(cx, handle, operations);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        // Querying enables the lazy native accessibility tree, then allow paint.
        let _ = accessible_with_role(cx, handle, "Projects", Some("AXOutline"), false);
        frame(cx, handle).await;
        assert!(accessible_with_role(cx, handle, "Projects", Some("AXOutline"), false).is_some());
        assert_eq!(
            rows(cx, handle),
            vec![
                AxRow {
                    label: "Folder".into(),
                    level: Some(0),
                    expanded: Some(true),
                    disclosed: Some(true),
                    selected: true,
                    enabled: true
                },
                AxRow {
                    label: "Child".into(),
                    level: Some(1),
                    expanded: None,
                    disclosed: None,
                    selected: false,
                    enabled: false
                },
            ],
            "one native outline row per item; boundary is not an item"
        );
    }
    // Metadata moved to the native row envelope must still obey the logical
    // row's inert visibility gate; hiding only its inner content would leak AXRow.
    for inert in [true, false] {
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node(6),
                vec![Style::Fields(vec![
                    Field::Height(Length::Px(32.)),
                    Field::Shrink(0.),
                    Field::Inert(inert),
                ])],
            )],
        );
        frame(cx, handle).await;
        #[cfg(target_os = "macos")]
        assert_eq!(
            rows(cx, handle).iter().any(|row| row.label == "Folder"),
            !inert,
            "moved row metadata follows logical inert state"
        );
    }
    apply(
        cx,
        handle,
        vec![
            Op::SetAccessibility(
                node(6),
                Some(metadata(
                    Role::TreeItem(item(1, Some(false), false, false)),
                    "Folder",
                )),
            ),
            Op::SetListOrder(
                node(5),
                Order {
                    revision: 2,
                    runs: vec![IdRun { first: 1, count: 1 }],
                },
            ),
            Op::SetListRows(
                node(5),
                vec![Row {
                    id: 1,
                    node: node(6),
                }],
            ),
            Op::Splice(node(5), 1, 2, vec![]),
            Op::Remove(node(8)),
            Op::Remove(node(9)),
            Op::Remove(node(10)),
        ],
    );
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    assert_eq!(
        rows(cx, handle),
        vec![AxRow {
            label: "Folder".into(),
            level: Some(0),
            expanded: Some(false),
            disclosed: Some(false),
            selected: false,
            enabled: true
        }]
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(5)),
            Op::Remove(node(6)),
            Op::Remove(node(7)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| assert!(view.lists.is_empty()))
        .unwrap();
    eprintln!(
        "GPUIO_TREE_SEMANTICS_OK: mounted outline/item hierarchy, selection/expanded/disabled, no duplicate/boundary items, update/removal and teardown; keyboard remains separate"
    );
}
