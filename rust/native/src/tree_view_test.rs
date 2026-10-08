//! Production managed-list tree semantics through an actual GPUI window.
use super::*;
use gpuio_protocol::accessibility::{Config as Metadata, Live, Role, TreeItem};
use gpuio_protocol::list::{Config, IdRun, Order, Row, ScrollPolicy};
use gpuio_protocol::tree_input::{Navigation, Request, Selection};

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
    selection_settable: bool,
    expansion_settable: bool,
}

#[cfg(target_os = "macos")]
fn rows(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Vec<AxRow> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
        sel,
    };
    use objc2_foundation::{NSString, ns_string};
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
                // Exercise the legacy discovery route used by screen readers.
                // The expected rows below cover enabled and disabled actions.
                let selection_settable: Bool =
                    msg_send![object, accessibilityIsAttributeSettable:ns_string!("AXSelected")];
                let expansion_settable: Bool =
                    msg_send![object, accessibilityIsAttributeSettable:ns_string!("AXExpanded")];
                let disclosure_settable: Bool =
                    msg_send![object, accessibilityIsAttributeSettable:ns_string!("AXDisclosed")];
                let role_settable: Bool =
                    msg_send![object, accessibilityIsAttributeSettable:ns_string!("AXRole")];
                assert!(!role_settable.as_bool());
                assert_eq!(expansion_settable.as_bool(), disclosure_settable.as_bool());
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
                    selection_settable: selection_settable.as_bool(),
                    expansion_settable: expansion_settable.as_bool(),
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

fn requests(transport: &Transport) -> Vec<Request> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::TreeInput(_, owner, _, _, request) if owner == node(5) => Some(request),
            _ => None,
        })
        .collect()
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let transport = handle
        .update(cx, |view, _, _| view.transport.clone())
        .unwrap();

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
        Op::SetTreeInput(node(5), true),
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
    exercise_moves(cx, handle, &transport).await;
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
                    enabled: true,
                    selection_settable: true,
                    expansion_settable: true,
                },
                AxRow {
                    label: "Child".into(),
                    level: Some(1),
                    expanded: None,
                    disclosed: None,
                    selected: false,
                    enabled: false,
                    selection_settable: false,
                    expansion_settable: false,
                },
            ],
            "one native outline row per item; boundary is not an item"
        );
    }
    handle
        .update(cx, |view, window, cx| {
            let focus = view.lists[&node(5)].borrow().tree_focus.clone().unwrap();
            window.focus(&focus, cx);
        })
        .unwrap();
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(
                view.lists[&node(5)]
                    .borrow()
                    .tree_focus
                    .as_ref()
                    .unwrap()
                    .is_focused(window),
                "tree root focus survives paint"
            );
            assert!(view.focus.borrow().allows(node(5)));
            assert!(
                view.session
                    .borrow()
                    .tree_input(
                        view.id,
                        node(5),
                        gpuio_protocol::HandlerId::from_parts(5, 1).unwrap(),
                        view.session.borrow().tree(view.id).unwrap().revision(),
                        Request::ActivateActive
                    )
                    .is_some()
            );
        })
        .unwrap();
    requests(&transport);
    for input in ["down", "down", "shift-end", "left", "space", "enter"] {
        key(cx, handle, input);
    }
    assert_eq!(
        requests(&transport),
        vec![
            Request::Navigate(Navigation::Next, Some(Selection::Replace)),
            Request::Navigate(Navigation::Next, Some(Selection::Replace)),
            Request::Navigate(Navigation::Last, Some(Selection::Range { extend: false })),
            Request::Navigate(Navigation::Parent, Some(Selection::Replace)),
            Request::SelectActive(Selection::Toggle),
            Request::ActivateActive,
        ]
    );
    // Printable Option-modified text retains its UTF-8 bytes; OCaml searches it.
    for (index, text) in ["é", "e\u{301}", "👨‍👩‍👧‍👦", "ab"].into_iter().enumerate()
    {
        cx.update_window(handle.into(), |_, window, cx| {
            let mut keystroke = gpui::Keystroke::parse("e").unwrap();
            keystroke.key_char = Some(text.into());
            keystroke.modifiers.alt = true;
            window.dispatch_event(
                gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
        })
        .unwrap();
        assert_eq!(
            requests(&transport),
            vec![Request::Typeahead {
                text: text.into(),
                reset: index == 0,
                cycle: index != 3,
            }]
        );
    }
    key(cx, handle, "down");
    requests(&transport);
    key(cx, handle, "z");
    assert_eq!(
        requests(&transport),
        vec![Request::Typeahead {
            text: "z".into(),
            reset: true,
            cycle: true
        }]
    );
    key(cx, handle, "ctrl-z");
    assert!(
        requests(&transport).is_empty(),
        "modified shortcut is not text intent"
    );
    #[cfg(target_os = "macos")]
    {
        assert!(accessible_with_role(cx, handle, "Folder", Some("AXRow"), true).is_some());
        frame(cx, handle).await;
        let input = requests(&transport);
        assert!(
            input.contains(&Request::Focus(1)),
            "AppKit focus uses tree input: {input:?}"
        );
        assert!(
            input.contains(&Request::Select(1, Selection::Replace)),
            "AX press selects without activation: {input:?}"
        );
        assert!(!input.contains(&Request::Activate(1)));
        for request in [
            AccessibilityRequest::SetSelected(true),
            AccessibilityRequest::SetSelected(false),
            AccessibilityRequest::SetSelected(false),
            AccessibilityRequest::SetSelected(true),
            AccessibilityRequest::SetExpanded(false),
            AccessibilityRequest::SetExpanded(true),
            AccessibilityRequest::SetDisclosed(false),
            AccessibilityRequest::SetDisclosed(true),
        ] {
            assert!(accessible_request(cx, handle, "Folder", Some("AXRow"), request).is_some());
        }
        frame(cx, handle).await;
        assert_eq!(
            requests(&transport),
            vec![
                Request::SetSelected(1, true),
                Request::SetSelected(1, false),
                Request::SetSelected(1, false),
                Request::SetSelected(1, true),
                Request::SetExpanded(1, false),
                Request::SetExpanded(1, true),
                Request::SetExpanded(1, false),
                Request::SetExpanded(1, true),
            ],
            "native AX setters preserve desired state and order before application rerender"
        );
    }
    let row_point = handle
        .update(cx, |view, _, _| {
            view.lists[&node(5)]
                .borrow()
                .native
                .handle()
                .bounds_for_item(0)
                .unwrap()
                .center()
        })
        .unwrap();
    requests(&transport);
    super::super::native_test::move_mouse(cx, handle, row_point, false);
    frame(cx, handle).await;
    super::super::native_test::mouse(cx, handle, row_point, true);
    super::super::native_test::mouse(cx, handle, row_point, false);
    frame(cx, handle).await;
    assert_eq!(
        requests(&transport),
        vec![Request::Select(1, Selection::Replace)]
    );
    key(cx, handle, "enter");
    key(cx, handle, "space");
    assert_eq!(
        requests(&transport),
        vec![
            Request::ActivateActive,
            Request::SelectActive(Selection::Toggle)
        ],
        "native on_click keyboard synthesis must not duplicate tree key requests"
    );
    for enabled in [false, true] {
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node(5),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(380.)),
                    Field::Height(Length::Px(180.)),
                    Field::PointerEvents(enabled),
                ])],
            )],
        );
        frame(cx, handle).await;
        requests(&transport);
        super::super::native_test::move_mouse(cx, handle, row_point, false);
        frame(cx, handle).await;
        super::super::native_test::mouse(cx, handle, row_point, true);
        super::super::native_test::mouse(cx, handle, row_point, false);
        assert_eq!(
            requests(&transport).len(),
            usize::from(enabled),
            "tree row honors inherited pointer policy"
        );
    }
    // A real native text input inside a tree row retains all editing keys,
    // including keys which it deliberately propagates to its ancestors.
    apply(
        cx,
        handle,
        vec![
            Op::SetTreeMoves(node(5), true),
            Op::Create(
                node(11),
                Kind::Input,
                "draft".into(),
                Some(gpuio_protocol::HandlerId::from_parts(11, 1).unwrap()),
            ),
            Op::SetEditor(
                node(11),
                EditorConfig {
                    label: "Tree draft".into(),
                    placeholder: String::new(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: true,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::Splice(node(6), 1, 0, vec![node(11)]),
            Op::SetStyle(
                node(6),
                vec![Style::Fields(vec![
                    Field::Height(Length::Px(80.)),
                    Field::Shrink(0.),
                ])],
            ),
        ],
    );
    frame(cx, handle).await;
    let point = handle
        .update(cx, |view, _, cx| {
            view.editors[&node(11)].input_bounds(cx).center()
        })
        .unwrap();
    requests(&transport);
    super::super::native_test::move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    super::super::native_test::mouse(cx, handle, point, true);
    super::super::native_test::mouse(cx, handle, point, false);
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            assert!(
                view.editors[&node(11)].focus_handle(cx).is_focused(window),
                "pointer focuses child editor"
            );
        })
        .unwrap();
    assert!(
        requests(&transport).is_empty(),
        "child editor click must not select its tree row"
    );
    super::super::native_test::mouse(cx, handle, point, true);
    super::super::native_test::move_mouse(cx, handle, point + gpui::point(px(20.), px(0.)), true);
    frame(cx, handle).await;
    cx.update_window(handle.into(), |_, _, cx| {
        assert!(
            !cx.has_active_drag(),
            "child editor selection cannot start a tree drag"
        )
    })
    .unwrap();
    super::super::native_test::mouse(cx, handle, point + gpui::point(px(20.), px(0.)), false);
    frame(cx, handle).await;
    requests(&transport);
    for input in [
        "left", "right", "up", "down", "home", "end", "space", "enter", "a",
    ] {
        key(cx, handle, input);
    }
    assert!(
        requests(&transport).is_empty(),
        "child editor keys must not select/navigate tree"
    );
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "仮入力", true);
        frame(cx, handle).await;
        handle
            .update(cx, |view, window, cx| {
                assert!(
                    view.editors[&node(11)]
                        .snapshot(window, cx)
                        .composition
                        .is_some()
                )
            })
            .unwrap();
        requests(&transport);
        key(cx, handle, "escape");
        frame(cx, handle).await;
        assert!(requests(&transport).is_empty());
        handle
            .update(cx, |view, window, cx| {
                assert!(
                    view.editors[&node(11)]
                        .snapshot(window, cx)
                        .composition
                        .is_none()
                )
            })
            .unwrap();
    }
    apply(
        cx,
        handle,
        vec![Op::Splice(node(6), 1, 1, vec![]), Op::Remove(node(11))],
    );
    frame(cx, handle).await;
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
        vec![Op::SetAccessibility(
            node(8),
            Some(metadata(
                Role::TreeItem(item(2, None, false, false)),
                "Child",
            )),
        )],
    );
    frame(cx, handle).await;
    let source_point = handle
        .update(cx, |view, _, _| {
            view.lists[&node(5)]
                .borrow()
                .native
                .handle()
                .bounds_for_item(1)
                .unwrap()
                .center()
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, source_point, false);
    frame(cx, handle).await;
    super::super::native_test::mouse(cx, handle, source_point, true);
    super::super::native_test::move_mouse(
        cx,
        handle,
        source_point + gpui::point(px(12.), px(0.)),
        true,
    );
    frame(cx, handle).await;
    cx.update_window(handle.into(), |_, _, cx| assert!(cx.has_active_drag()))
        .unwrap();
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
    cx.update_window(handle.into(), |_, _, cx| {
        assert!(
            !cx.has_active_drag(),
            "collapse/deletion cancels the removed source drag"
        )
    })
    .unwrap();
    handle
        .update(cx, |view, _, _| assert!(view.tree_drag.upgrade().is_none()))
        .unwrap();
    super::super::native_test::mouse(cx, handle, source_point, false);
    assert!(
        requests(&transport)
            .iter()
            .all(|request| !matches!(request, Request::Move { .. }))
    );
    #[cfg(target_os = "macos")]
    assert_eq!(
        rows(cx, handle),
        vec![AxRow {
            label: "Folder".into(),
            level: Some(0),
            expanded: Some(false),
            disclosed: Some(false),
            selected: false,
            enabled: true,
            selection_settable: true,
            expansion_settable: true,
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
        "GPUIO_TREE_SEMANTICS_OK: mounted outline/item hierarchy, selection/expanded/disabled, no duplicate/boundary items, update/removal and teardown; ordered keyboard, AppKit focus/select and child editor/IME isolation"
    );
}

#[cfg(feature = "native-image-tests")]
fn assert_tree_pixel(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    point: gpui::Point<gpui::Pixels>,
    expected: [u8; 4],
    label: &str,
) {
    let (image, active, mouse, dragging) = cx
        .update_window(handle.into(), |_, window, cx| {
            window.set_scale_factor(1.);
            window.draw(cx).clear(cx);
            (
                window.render_to_image().unwrap(),
                window.is_window_active(),
                window.mouse_position(),
                cx.has_active_drag(),
            )
        })
        .unwrap();
    let x = f32::from(point.x).floor() as u32;
    let y = f32::from(point.y).floor() as u32;
    assert_eq!(
        image.get_pixel(x, y).0,
        expected,
        "{label} at {x},{y}: active={active}, mouse={mouse:?}, dragging={dragging}"
    );
}

async fn exercise_moves(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    use gpuio_protocol::tree_input::Placement;
    fn bounds(
        cx: &mut gpui::AsyncApp,
        handle: WindowHandle<View>,
        index: usize,
    ) -> gpui::Bounds<gpui::Pixels> {
        handle
            .update(cx, |view, _, _| {
                view.lists[&node(5)]
                    .borrow()
                    .native
                    .handle()
                    .bounds_for_item(index)
                    .unwrap()
            })
            .unwrap()
    }
    fn active(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
        cx.update_window(handle.into(), |_, _, cx| cx.has_active_drag())
            .unwrap()
    }
    async fn begin(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
        let point = bounds(cx, handle, 1).center();
        super::super::native_test::move_mouse(cx, handle, point, false);
        frame(cx, handle).await;
        super::super::native_test::mouse(cx, handle, point, true);
        super::super::native_test::move_mouse(
            cx,
            handle,
            point + gpui::point(px(12.), px(0.)),
            true,
        );
        frame(cx, handle).await;
        assert!(active(cx, handle), "native GPUI owns the tree drag");
    }
    async fn release(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, fraction: f32) {
        let rect = bounds(cx, handle, 0);
        let point = gpui::point(rect.center().x, rect.origin.y + rect.size.height * fraction);
        super::super::native_test::move_mouse(cx, handle, point, true);
        frame(cx, handle).await;
        #[cfg(feature = "native-image-tests")]
        if active(cx, handle) {
            let sample = if fraction < 0.25 {
                gpui::point(rect.right() - px(8.), rect.origin.y + px(2.))
            } else if fraction < 0.75 {
                gpui::point(rect.right() - px(3.), rect.center().y)
            } else {
                gpui::point(rect.right() - px(8.), rect.bottom() - px(4.))
            };
            // A real platform move can arrive while awaiting a display frame.
            // Keep synthetic hover dispatch and GPU capture in one UI turn.
            super::super::native_test::move_mouse(cx, handle, point, true);
            assert_tree_pixel(
                cx,
                handle,
                sample,
                [102, 255, 136, 255],
                "drop cue above opaque content",
            );
            let outside = gpui::point(rect.right() - px(8.), rect.bottom() + px(50.));
            super::super::native_test::move_mouse(cx, handle, outside, true);
            assert_tree_pixel(
                cx,
                handle,
                sample,
                [40, 53, 75, 255],
                "leaving a target clears its cue during the same gesture",
            );
            super::super::native_test::move_mouse(cx, handle, point, true);
            assert_tree_pixel(
                cx,
                handle,
                sample,
                [102, 255, 136, 255],
                "return restores drop cue",
            );
        }
        super::super::native_test::move_mouse(cx, handle, point, true);
        super::super::native_test::mouse(cx, handle, point, false);
        frame(cx, handle).await;
    }
    requests(transport);
    apply(
        cx,
        handle,
        vec![
            Op::SetTreeMoves(node(5), true),
            Op::SetAccessibility(
                node(8),
                Some(metadata(
                    Role::TreeItem(item(2, None, false, false)),
                    "Child",
                )),
            ),
        ],
    );
    frame(cx, handle).await;
    #[cfg(feature = "native-image-tests")]
    {
        apply(
            cx,
            handle,
            vec![
                Op::SetStyle(
                    node(5),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(380.)),
                        Field::Height(Length::Px(180.)),
                        Field::Foreground(Color::Rgba(0x66ff88ff)),
                    ])],
                ),
                Op::SetStyle(
                    node(7),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Percent(100.)),
                        Field::Height(Length::Px(32.)),
                        Field::Background(Fill::Solid(Color::Rgba(0x28354bff))),
                    ])],
                ),
            ],
        );
        frame(cx, handle).await;
        let rect = bounds(cx, handle, 0);
        let point = gpui::point(rect.right() - px(2.), rect.center().y);
        handle
            .update(cx, |view, window, cx| {
                view.lists[&node(5)]
                    .borrow()
                    .tree_focus
                    .as_ref()
                    .unwrap()
                    .focus(window, cx);
            })
            .unwrap();
        frame(cx, handle).await;
        assert_tree_pixel(
            cx,
            handle,
            point,
            [40, 53, 75, 255],
            "selected row without native focus has no outline",
        );
        super::super::native_test::move_mouse(cx, handle, rect.center(), false);
        frame(cx, handle).await;
        super::super::native_test::mouse(cx, handle, rect.center(), true);
        super::super::native_test::mouse(cx, handle, rect.center(), false);
        frame(cx, handle).await;
        assert_tree_pixel(
            cx,
            handle,
            point,
            [102, 255, 136, 255],
            "actual focus above opaque content",
        );
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node(5),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(380.)),
                    Field::Height(Length::Px(180.)),
                    Field::Foreground(Color::Rgba(0xff88ccff)),
                ])],
            )],
        );
        frame(cx, handle).await;
        assert_tree_pixel(
            cx,
            handle,
            point,
            [255, 136, 204, 255],
            "focus inherits current foreground",
        );
        handle
            .update(cx, |view, window, cx| {
                view.lists[&node(5)]
                    .borrow()
                    .tree_focus
                    .as_ref()
                    .unwrap()
                    .focus(window, cx);
            })
            .unwrap();
        frame(cx, handle).await;
        assert_tree_pixel(
            cx,
            handle,
            point,
            [40, 53, 75, 255],
            "blur removes focus outline",
        );
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node(5),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(380.)),
                    Field::Height(Length::Px(180.)),
                    Field::Foreground(Color::Rgba(0x66ff88ff)),
                ])],
            )],
        );
        frame(cx, handle).await;
        requests(transport);
    }
    for (fraction, placement) in [
        (0.1, Placement::Before),
        (0.5, Placement::Inside),
        (0.9, Placement::After),
    ] {
        begin(cx, handle).await;
        release(cx, handle, fraction).await;
        assert_eq!(
            requests(transport),
            vec![Request::Move {
                source: 2,
                destination: 1,
                placement
            }]
        );
        assert!(!active(cx, handle));
        handle
            .update(cx, |view, _, _| {
                assert!(
                    view.tree_drag.upgrade().is_none(),
                    "drag lease retires after drop"
                )
            })
            .unwrap();
    }
    begin(cx, handle).await;
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(!active(cx, handle));
    release(cx, handle, 0.5).await;
    assert!(
        requests(transport)
            .iter()
            .all(|request| !matches!(request, Request::Move { .. }))
    );
    for change in [
        Op::SetTreeMoves(node(5), false),
        Op::Bind(
            node(5),
            Some(gpuio_protocol::HandlerId::from_parts(5, 2).unwrap()),
        ),
        Op::SetAccessibility(
            node(8),
            Some(metadata(
                Role::TreeItem(item(2, None, false, true)),
                "Child",
            )),
        ),
        Op::SetStyle(
            node(8),
            vec![Style::Fields(vec![
                Field::Height(Length::Px(32.)),
                Field::Inert(true),
            ])],
        ),
    ] {
        begin(cx, handle).await;
        apply(cx, handle, vec![change]);
        frame(cx, handle).await;
        assert!(
            !active(cx, handle),
            "policy/disabled/hidden source cancels without a timer"
        );
        release(cx, handle, 0.5).await;
        assert!(
            requests(transport)
                .iter()
                .all(|request| !matches!(request, Request::Move { .. }))
        );
        apply(
            cx,
            handle,
            vec![
                Op::SetTreeMoves(node(5), true),
                Op::Bind(
                    node(5),
                    Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
                ),
                Op::SetAccessibility(
                    node(8),
                    Some(metadata(
                        Role::TreeItem(item(2, None, false, false)),
                        "Child",
                    )),
                ),
                Op::SetStyle(
                    node(8),
                    vec![Style::Fields(vec![
                        Field::Height(Length::Px(32.)),
                        Field::Shrink(0.),
                    ])],
                ),
            ],
        );
        frame(cx, handle).await;
    }
    apply(
        cx,
        handle,
        vec![
            Op::SetTreeMoves(node(5), false),
            Op::SetAccessibility(
                node(8),
                Some(metadata(
                    Role::TreeItem(item(2, None, false, true)),
                    "Child",
                )),
            ),
        ],
    );
    frame(cx, handle).await;
    #[cfg(feature = "native-image-tests")]
    {
        let rect = bounds(cx, handle, 0);
        assert_tree_pixel(
            cx,
            handle,
            gpui::point(rect.right() - px(3.), rect.center().y),
            [40, 53, 75, 255],
            "finished drag leaves no cue",
        );
        apply(
            cx,
            handle,
            vec![
                Op::SetStyle(
                    node(5),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(380.)),
                        Field::Height(Length::Px(180.)),
                    ])],
                ),
                Op::SetStyle(node(7), vec![]),
            ],
        );
        frame(cx, handle).await;
        eprintln!(
            "GPUIO_TREE_INDICATORS_OK: actual-focus/blur, foreground update, selected opaque row content and three native drop placements via GPU pixels"
        );
    }
    requests(transport);
    eprintln!(
        "GPUIO_TREE_MOVES_OK: native start/drop, before/inside/after, no hierarchy mutation, escape/policy/handler/disabled/inert cancellation and lease disposal"
    );
}
