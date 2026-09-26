//! Real-window disclosure input, nested focus restoration and hidden ownership.
use super::*;

fn focus(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: i64) {
    handle
        .update(cx, |view, window, cx| {
            let focus = view
                .editors
                .get(&node(id))
                .map(|e| e.focus_handle(cx))
                .unwrap_or_else(|| view.buttons[&node(id)].focus.clone());
            window.focus(&focus, cx);
        })
        .unwrap();
}
fn hide(id: i64) -> Op {
    Op::SetStyle(node(id), vec![Style::Fields(vec![Field::Display(3)])])
}

#[cfg(target_os = "macos")]
fn expanded(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, label: &str) -> Option<bool> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(object: *mut AnyObject, label: &str, depth: usize) -> Option<bool> {
        if object.is_null() || depth > 32 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !title.is_null()
                && !role.is_null()
                && (*title).to_string() == label
                && (*role).to_string() == "AXButton"
            {
                let value: Bool = msg_send![object, isAccessibilityExpanded];
                return Some(value.as_bool());
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex:index];
                if let Some(value) = visit(child, label, depth + 1) {
                    return Some(value);
                }
            }
            None
        }
    }
    let view = super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, label, 0)
    }
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let config = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(4))
                .unwrap()
                .editor
                .clone()
                .unwrap()
        })
        .unwrap();
    let mut operations = vec![Op::SetRoot(None)];
    operations.extend((0..=4).map(|i| Op::Remove(node(i))));
    for (id, kind, label) in [
        (5, Kind::Accordion, ""),
        (6, Kind::Disclosure, ""),
        (7, Kind::Button, "First section"),
        (8, Kind::Panel, "First content"),
        (9, Kind::Input, "Preserved draft"),
        (10, Kind::Disclosure, ""),
        (11, Kind::Button, "Disabled section"),
        (12, Kind::Panel, "Disabled content"),
        (13, Kind::Disclosure, ""),
        (14, Kind::Button, "Last section"),
        (15, Kind::Panel, "Last content"),
        (16, Kind::Text, "Last body"),
        (17, Kind::Accordion, ""),
        (18, Kind::Disclosure, ""),
        (19, Kind::Button, "Nested section"),
        (20, Kind::Panel, "Nested content"),
        (21, Kind::Input, "Nested draft"),
    ] {
        let handler = matches!(kind, Kind::Button | Kind::Input)
            .then(|| gpuio_protocol::HandlerId::from_parts(id, 1).unwrap());
        operations.push(Op::Create(node(id), kind, label.into(), handler));
        if kind == Kind::Input {
            operations.push(Op::SetEditor(node(id), (*config).clone()));
        }
        if kind == Kind::Button {
            operations.push(Op::SetControl(node(id), Control::Button(id == 11)));
        }
    }
    for (parent, children) in [
        (5, vec![6, 10, 13]),
        (6, vec![7, 8]),
        (8, vec![9, 17]),
        (10, vec![11, 12]),
        (13, vec![14, 15]),
        (15, vec![16]),
        (17, vec![18]),
        (18, vec![19, 20]),
        (20, vec![21]),
    ] {
        operations.push(Op::Splice(
            node(parent),
            0,
            0,
            children.into_iter().map(node).collect(),
        ));
    }
    operations.extend([hide(12), hide(15), Op::SetRoot(Some(node(5)))]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    focus(cx, handle, 7);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        let _ = expanded(cx, handle, "First section");
        frame(cx, handle).await;
        assert_eq!(expanded(cx, handle, "First section"), Some(true));
        assert_eq!(expanded(cx, handle, "Last section"), Some(false));
    }

    for (key_name, target) in [
        ("down", 14),
        ("down", 7),
        ("end", 14),
        ("up", 7),
        ("home", 7),
    ] {
        key(cx, handle, key_name);
        assert!(
            focused(cx, handle, node(target)),
            "{key_name} should focus {target}"
        );
    }
    key(cx, handle, "shift-down");
    assert!(focused(cx, handle, node(7)));
    let _ = presses(transport);
    key(cx, handle, "enter");
    assert_eq!(presses(transport), vec![node(7)]);
    focus(cx, handle, 19);
    key(cx, handle, "down");
    assert!(
        focused(cx, handle, node(19)),
        "nested group does not move outer headers"
    );
    focus(cx, handle, 21);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "に", true);
        frame(cx, handle).await;
        handle
            .update(cx, |v, w, cx| {
                assert!(v.editors[&node(21)].snapshot(w, cx).composition.is_some());
            })
            .unwrap();
    }
    let (before, native_focus) = handle
        .update(cx, |v, w, cx| {
            (
                v.editors[&node(21)].snapshot(w, cx),
                v.editors[&node(21)].focus_handle(cx),
            )
        })
        .unwrap();
    apply(cx, handle, vec![hide(8)]);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(7)),
        "outer collapse restores outer trigger, not hidden nested trigger"
    );
    handle
        .update(cx, |v, w, cx| {
            let result =
                v.editors
                    .get_mut(&node(21))
                    .unwrap()
                    .command(&EditorCommand::Focus, w, cx);
            assert!(matches!(
                result,
                EditorResult::Failed(EditorError::FocusBlocked)
            ));
            assert_eq!(before.text, v.editors[&node(21)].snapshot(w, cx).text);
            assert!(native_focus == v.editors[&node(21)].focus_handle(cx));
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        assert_eq!(expanded(cx, handle, "First section"), Some(false));
        super::super::editor_test::native_text(cx, handle, "must not reach hidden editor", false);
        frame(cx, handle).await;
        handle
            .update(cx, |v, w, cx| {
                assert_eq!(before.text, v.editors[&node(21)].snapshot(w, cx).text);
            })
            .unwrap();
    }
    key(cx, handle, "tab");
    assert!(!focused(cx, handle, node(9)) && !focused(cx, handle, node(21)));
    apply(cx, handle, vec![Op::SetStyle(node(8), vec![])]);
    frame(cx, handle).await;
    focus(cx, handle, 21);
    frame(cx, handle).await;
    let removed_alive = handle
        .update(cx, |v, _, _| v.editors[&node(21)].liveness_probe())
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            hide(20),
            Op::Splice(node(20), 0, 1, vec![]),
            Op::Remove(node(21)),
        ],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(19)),
        "unmount uses previous ancestry to restore trigger"
    );
    handle
        .update(cx, |v, _, _| {
            assert!(!v.editors.contains_key(&node(21)));
            assert!(!removed_alive());
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        let _ = accessible_with_role(cx, handle, "First section", Some("AXButton"), false);
        frame(cx, handle).await;
        assert!(
            accessible_with_role(cx, handle, "First section", Some("AXButton"), false).is_some()
        );
        let _ = presses(transport);
        assert!(
            accessible_with_role(cx, handle, "First section", Some("AXButton"), true).is_some()
        );
        frame(cx, handle).await;
        assert_eq!(presses(transport), vec![node(7)]);
    }
    // Hidden scopes must leave the modal stack even when native content is retained.
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(22), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(22),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(
                node(23),
                Kind::Button,
                "Inside scope".into(),
                Some(gpuio_protocol::HandlerId::from_parts(23, 1).unwrap()),
            ),
            Op::Splice(node(22), 0, 0, vec![node(23)]),
            Op::Splice(node(8), 2, 0, vec![node(22)]),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(23)));
    apply(cx, handle, vec![hide(8)]);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(7)));
    handle
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().handle(node(22)).is_none());
        })
        .unwrap();
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((5..=23).filter(|i| *i != 21).map(|i| Op::Remove(node(i))))
            .collect(),
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(v.editors.is_empty());
            assert!(v.buttons.is_empty());
            assert!(!v.focus.borrow_mut().take_pending());
        })
        .unwrap();
    pagination_and_breadcrumbs(cx, handle, transport).await;
    custom_disclosure_header(cx, handle, transport, (*config).clone()).await;
    inert_content(cx, handle, transport, (*config).clone()).await;
    inert_drag_cleanup(cx, handle, (*config).clone()).await;
    println!(
        "GPUIO_DISCLOSURE_NATIVE_OK: header traversal, single activation, nested collapse/unmount focus, retained editors, hidden scopes and disposal"
    );
}

async fn inert_drag_cleanup(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    mut config: EditorConfig,
) {
    config.label = "Drag selection editor".into();
    config.min_rows = 3;
    config.max_rows = 3;
    let text = (0..200)
        .map(|i| format!("Row {i}: retained text during drag selection\n"))
        .collect::<String>();
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(51), Kind::Container, "".into(), None),
            Op::Create(node(52), Kind::Container, "".into(), None),
            Op::Create(
                node(53),
                Kind::Textarea,
                text,
                Some(gpuio_protocol::HandlerId::from_parts(53, 1).unwrap()),
            ),
            Op::SetEditor(node(53), config),
            Op::SetStyle(
                node(53),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(80.)),
                ])],
            ),
            Op::Splice(node(52), 0, 0, vec![node(53)]),
            Op::Splice(node(51), 0, 0, vec![node(52)]),
            Op::SetRoot(Some(node(51))),
        ],
    );
    frame(cx, handle).await;
    focus(cx, handle, 53);
    frame(cx, handle).await;
    let bounds = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(53)].bounds)
        .unwrap();
    let start = bounds.origin + gpui::point(px(20.), px(20.));
    let end = gpui::point(start.x, bounds.bottom() + px(40.));
    super::super::native_test::move_mouse(cx, handle, start, false);
    super::super::native_test::mouse(cx, handle, start, true);
    super::super::native_test::move_mouse(cx, handle, end, true);
    frame(cx, handle).await;
    let active = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    assert!(
        handle.update(cx, |v, _, _| v.render_count).unwrap() > active + 2,
        "real drag auto-scroll timer was active"
    );
    let selection = handle
        .update(cx, |v, w, cx| {
            v.editors[&node(53)].snapshot(w, cx).selection
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(52),
            vec![Style::Fields(vec![Field::Inert(true)])],
        )],
    );
    frame(cx, handle).await;
    // Drain already queued rendering before checking idle without a mouse-up.
    cx.background_executor()
        .timer(std::time::Duration::from_millis(80))
        .await;
    let stopped = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    assert_eq!(
        handle.update(cx, |v, _, _| v.render_count).unwrap(),
        stopped,
        "inert retained editor must stop drag auto-scroll without waiting for mouse-up"
    );
    handle
        .update(cx, |v, w, cx| {
            assert_eq!(
                v.editors[&node(53)].snapshot(w, cx).selection,
                selection,
                "blur preserves the actual selected range"
            )
        })
        .unwrap();
    super::super::native_test::mouse(cx, handle, end, false);
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((51..=53).map(|id| Op::Remove(node(id))))
            .collect(),
    );
    frame(cx, handle).await;
    println!(
        "GPUIO_INERT_DRAG_IDLE_OK: active textarea auto-scroll stops on inert blur before mouse-up"
    );
}

async fn inert_content(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    mut config: EditorConfig,
) {
    config.label = "Outgoing editor".into();
    let panel_style = |inert| {
        vec![Style::Fields(vec![
            Field::Width(Length::Px(240.)),
            Field::Height(Length::Px(180.)),
            Field::Background(Fill::Solid(Color::Rgba(0xe13599ff))),
            Field::Inert(inert),
        ])]
    };
    let mut ops = vec![];
    for (id, kind, label) in [
        (45, Kind::Container, ""),
        (46, Kind::Button, "Outside inert panel"),
        (47, Kind::Container, ""),
        (48, Kind::Text, "Painted outgoing content"),
        (49, Kind::Button, "Outgoing button"),
        (50, Kind::Input, "Retained Unicode 👨‍👩‍👧‍👦"),
    ] {
        let handler = matches!(kind, Kind::Button | Kind::Input)
            .then(|| gpuio_protocol::HandlerId::from_parts(id, 1).unwrap());
        ops.push(Op::Create(node(id), kind, label.into(), handler));
        if kind == Kind::Button {
            ops.push(Op::SetControl(node(id), Control::Button(false)));
        }
    }
    ops.extend([
        Op::SetEditor(node(50), config),
        // A child declaration cannot escape its ancestor's gate.
        Op::SetStyle(node(50), vec![Style::Fields(vec![Field::Inert(false)])]),
        Op::SetStyle(node(47), panel_style(false)),
        Op::Splice(node(47), 0, 0, vec![node(48), node(49), node(50)]),
        Op::Splice(node(45), 0, 0, vec![node(46), node(47)]),
        Op::SetRoot(Some(node(45))),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    focus(cx, handle, 50);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "日本", true);
        frame(cx, handle).await;
        handle
            .update(cx, |v, w, cx| {
                assert!(v.editors[&node(50)].snapshot(w, cx).composition.is_some())
            })
            .unwrap();
    }
    let (bounds, editor_focus, before) = handle
        .update(cx, |view, window, cx| {
            (
                view.probes.borrow()[&node(47)].bounds,
                view.editors[&node(50)].focus_handle(cx),
                view.editors[&node(50)].snapshot(window, cx),
            )
        })
        .unwrap();
    apply(cx, handle, vec![Op::SetStyle(node(47), panel_style(true))]);
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            assert_eq!(
                view.probes.borrow()[&node(47)].bounds,
                bounds,
                "inert preserves layout"
            );
            assert!(!view.focus.borrow().visible(node(50)));
            assert!(!editor_focus.is_focused(window));
            assert!(editor_focus == view.editors[&node(50)].focus_handle(cx));
            assert!(matches!(
                view.editors
                    .get_mut(&node(50))
                    .unwrap()
                    .command(&EditorCommand::Focus, window, cx),
                EditorResult::Failed(EditorError::FocusBlocked)
            ));
            #[cfg(feature = "native-image-tests")]
            {
                let image = window.render_to_image().unwrap();
                let position = bounds.bottom_right() - gpui::point(px(4.), px(4.));
                let scale = window.scale_factor();
                let pixel = image.get_pixel(
                    (f32::from(position.x) * scale) as u32,
                    (f32::from(position.y) * scale) as u32,
                );
                assert_eq!(
                    pixel.0,
                    [0xe1, 0x35, 0x99, 0xff],
                    "inert subtree still paints actual GPU pixels"
                );
            }
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        let _ = accessible_with_role(cx, handle, "Outside inert panel", Some("AXButton"), false);
        frame(cx, handle).await;
        assert!(
            accessible_with_role(cx, handle, "Outgoing button", Some("AXButton"), false).is_none()
        );
        assert!(
            accessible_with_role(cx, handle, "Outgoing editor", Some("AXTextField"), false)
                .is_none()
        );
        super::super::editor_test::native_text(cx, handle, "blocked native input", false);
    }
    let _ = presses(transport);
    for id in [49, 50] {
        let position = handle
            .update(cx, |v, _, _| v.probes.borrow()[&node(id)].bounds.center())
            .unwrap();
        super::super::native_test::move_mouse(cx, handle, position, false);
        super::super::native_test::mouse(cx, handle, position, true);
        super::super::native_test::mouse(cx, handle, position, false);
    }
    key(cx, handle, "enter");
    assert!(presses(transport).iter().all(|id| *id != node(49)));
    assert!(!focused(cx, handle, node(50)));
    handle
        .update(cx, |v, w, cx| {
            assert_eq!(v.editors[&node(50)].snapshot(w, cx).text, before.text)
        })
        .unwrap();
    let position = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(46)].bounds.center())
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, position, false);
    super::super::native_test::mouse(cx, handle, position, true);
    super::super::native_test::mouse(cx, handle, position, false);
    assert_eq!(
        presses(transport),
        vec![node(46)],
        "shield does not block unrelated controls"
    );
    apply(cx, handle, vec![Op::SetStyle(node(47), panel_style(false))]);
    frame(cx, handle).await;
    focus(cx, handle, 49);
    key(cx, handle, "enter");
    assert_eq!(presses(transport), vec![node(49)]);
    focus(cx, handle, 50);
    assert!(focused(cx, handle, node(50)));
    handle
        .update(cx, |v, w, cx| {
            assert!(v.editors[&node(50)].focus_handle(cx) == editor_focus);
            assert_eq!(v.editors[&node(50)].snapshot(w, cx).text, before.text);
        })
        .unwrap();
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((45..=50).map(|id| Op::Remove(node(id))))
            .collect(),
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(v.editors.is_empty() && v.buttons.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    println!(
        "GPUIO_INERT_NATIVE_OK: retained geometry/editor, focus and native-input denial, AX hiding, pointer shield, unrelated input, reactivation and teardown"
    );
}

#[cfg(target_os = "macos")]
fn native_help(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    label: &str,
) -> Option<Option<String>> {
    use objc2::{msg_send, runtime::AnyObject};
    use objc2_foundation::NSString;
    unsafe fn visit(object: *mut AnyObject, label: &str, depth: usize) -> Option<Option<String>> {
        if object.is_null() || depth > 32 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !title.is_null()
                && !role.is_null()
                && (*title).to_string() == label
                && ["AXButton", "AXLink"].contains(&(*role).to_string().as_str())
            {
                let help: *mut NSString = msg_send![object, accessibilityHelp];
                return Some(help.as_ref().map(|s| s.to_string()));
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex:index];
                if let Some(found) = visit(child, label, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
    }
    let view = super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, label, 0)
    }
}

async fn pagination_and_breadcrumbs(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    use gpuio_protocol::accessibility::{Config, Current, Live, Role};
    let metadata = |role, current, description: Option<&str>| Config {
        role,
        current,
        description: description.map(str::to_owned),
        label: None,
        live: Live::Off,
        field: None,
    };
    let mut operations = vec![
        Op::Create(node(24), Kind::Container, "".into(), None),
        Op::SetAccessibility(
            node(24),
            Some(Config {
                label: Some("Page navigation".into()),
                ..metadata(Some(Role::Navigation), None, None)
            }),
        ),
    ];
    for (id, label, disabled) in [
        (25, "Previous page", true),
        (26, "Page 1", false),
        (27, "Page 2", false),
        (28, "Next page", false),
        (29, "Home route", false),
    ] {
        operations.push(Op::Create(
            node(id),
            Kind::Button,
            label.into(),
            Some(gpuio_protocol::HandlerId::from_parts(id, 1).unwrap()),
        ));
        operations.push(Op::SetControl(node(id), Control::Button(disabled)));
    }
    operations.extend([
        Op::SetAccessibility(
            node(26),
            Some(metadata(None, Some(Current::Page), Some("Current page"))),
        ),
        Op::SetAccessibility(node(29), Some(metadata(Some(Role::Link), None, None))),
        Op::Splice(node(24), 0, 0, (25..=29).map(node).collect()),
        Op::SetRoot(Some(node(24))),
    ]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    focus(cx, handle, 26);
    frame(cx, handle).await;
    let retained_focus = handle
        .update(cx, |v, _, _| v.buttons[&node(26)].focus.clone())
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        let _ = native_help(cx, handle, "Page 1");
        frame(cx, handle).await;
        assert_eq!(
            native_help(cx, handle, "Page 1"),
            Some(Some("Current page".into()))
        );
        assert!(
            accessible_with_role(cx, handle, "Page navigation", Some("AXGroup"), false).is_some()
        );
    }
    let _ = presses(transport);
    key(cx, handle, "enter");
    key(cx, handle, "enter");
    assert_eq!(
        presses(transport),
        vec![node(26), node(26)],
        "no optimistic native selection or duplicate activation"
    );
    key(cx, handle, "tab");
    assert!(focused(cx, handle, node(27)));
    key(cx, handle, "shift-tab");
    assert!(focused(cx, handle, node(26)));
    // Application acknowledgement changes semantics, not focus or button identity.
    apply(
        cx,
        handle,
        vec![
            Op::SetAccessibility(node(26), Some(metadata(None, None, None))),
            Op::SetAccessibility(
                node(27),
                Some(metadata(None, Some(Current::Page), Some("Page actuelle"))),
            ),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(26)));
    handle
        .update(cx, |v, _, _| {
            assert!(v.buttons[&node(26)].focus == retained_focus)
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        assert_eq!(native_help(cx, handle, "Page 1"), Some(None));
        assert_eq!(
            native_help(cx, handle, "Page 2"),
            Some(Some("Page actuelle".into()))
        );
        let _ = presses(transport);
        assert!(accessible_with_role(cx, handle, "Home route", Some("AXLink"), true).is_some());
        frame(cx, handle).await;
        assert_eq!(presses(transport), vec![node(29)]);
    }
    // The next model disables navigation at its boundary, without remounting.
    focus(cx, handle, 28);
    apply(
        cx,
        handle,
        vec![Op::SetControl(node(28), Control::Button(true))],
    );
    frame(cx, handle).await;
    let _ = presses(transport);
    assert!(!focused(cx, handle, node(28)));
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((24..=29).map(|i| Op::Remove(node(i))))
            .collect(),
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(v.buttons.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    println!(
        "GPUIO_NAVIGATION_SEMANTICS_OK: current-page native help updates, retained focus, queued activation, Tab, link AX press, disabled boundary and teardown"
    );
}

async fn custom_disclosure_header(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    config: EditorConfig,
) {
    use gpuio_protocol::accessibility::{Config, Live, Role};
    let mut ops = Vec::new();
    for (id, kind, text) in [
        (30, Kind::Accordion, ""),
        (31, Kind::Disclosure, ""),
        (32, Kind::Container, ""),
        (33, Kind::Button, "Archive route"),
        (34, Kind::Button, "Expand archive"),
        (35, Kind::Panel, "Archive children"),
        (36, Kind::Input, "Draft in nested destination"),
        (37, Kind::Disclosure, ""),
        (38, Kind::Button, "Next disclosure"),
        (39, Kind::Panel, "Next panel"),
        (40, Kind::Tooltip, ""),
        (41, Kind::Button, "Nested tooltip anchor"),
        (42, Kind::Text, "Nested tooltip content"),
        (43, Kind::FocusScope, ""),
        (44, Kind::Text, "Nested popover content"),
    ] {
        let handler = matches!(kind, Kind::Button | Kind::Input | Kind::FocusScope)
            .then(|| gpuio_protocol::HandlerId::from_parts(id, 1).unwrap());
        ops.push(Op::Create(node(id), kind, text.into(), handler));
    }
    ops.extend([
        Op::SetEditor(node(36), config),
        Op::SetAccessibility(
            node(33),
            Some(Config {
                role: Some(Role::Link),
                label: None,
                description: None,
                live: Live::Off,
                field: None,
                current: None,
            }),
        ),
        Op::Splice(node(32), 0, 0, vec![node(33), node(34)]),
        Op::Splice(node(35), 0, 0, vec![node(36), node(40), node(43)]),
        Op::SetTooltip(
            node(40),
            TooltipConfig {
                label: "Retained nested tooltip".into(),
                width: 180.,
                open_state: TooltipOpenState::Managed(false),
                disabled: true,
                hoverable: false,
                show_delay_ns: 0,
                hide_delay_ns: 0,
                skip_delay_ns: 0,
            },
        ),
        Op::Splice(node(40), 0, 0, vec![node(41), node(42)]),
        Op::SetFocusScope(
            node(43),
            FocusScopeConfig {
                trap: false,
                auto_focus: false,
                restore_focus: false,
            },
        ),
        Op::SetOverlay(
            node(43),
            Some(OverlayConfig {
                kind: OverlayKind::Popover,
                label: "Retained popover".into(),
                width: 180.,
                dismiss_on_escape: true,
                dismiss_on_outside_pointer: true,
            }),
        ),
        Op::Splice(node(43), 0, 0, vec![node(44)]),
        Op::Splice(node(31), 0, 0, vec![node(32), node(35)]),
        Op::Splice(node(37), 0, 0, vec![node(38), node(39)]),
        Op::Splice(node(30), 0, 0, vec![node(31), node(37)]),
        Op::SetRoot(Some(node(30))),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    focus(cx, handle, 33);
    key(cx, handle, "down");
    assert!(
        focused(cx, handle, node(33)),
        "navigation link must not acquire toggle arrow behavior"
    );
    let _ = presses(transport);
    key(cx, handle, "enter");
    assert_eq!(presses(transport), vec![node(33)]);
    focus(cx, handle, 34);
    key(cx, handle, "down");
    assert!(
        focused(cx, handle, node(38)),
        "custom toggle participates in accordion traversal"
    );
    key(cx, handle, "home");
    assert!(focused(cx, handle, node(34)));
    key(cx, handle, "enter");
    assert_eq!(presses(transport), vec![node(34)]);
    #[cfg(target_os = "macos")]
    {
        let _ = expanded(cx, handle, "Expand archive");
        frame(cx, handle).await;
        assert_eq!(expanded(cx, handle, "Expand archive"), Some(true));
        assert!(accessible_with_role(cx, handle, "Archive route", Some("AXLink"), false).is_some());
    }
    focus(cx, handle, 36);
    frame(cx, handle).await;
    let editor = handle
        .update(cx, |v, _, _| v.editors[&node(36)].liveness_probe())
        .unwrap();
    apply(cx, handle, vec![hide(35)]);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(34)),
        "collapse restores the dedicated toggle, not the navigation link"
    );
    handle
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().handle(node(40)).is_none());
            assert!(v.focus.borrow().handle(node(43)).is_none());
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    assert_eq!(expanded(cx, handle, "Expand archive"), Some(false));
    apply(cx, handle, vec![Op::SetStyle(node(35), vec![])]);
    frame(cx, handle).await;
    focus(cx, handle, 36);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(35),
            vec![Style::Fields(vec![Field::Inert(true)])],
        )],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(34)));
    handle
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().handle(node(40)).is_none());
            assert!(v.focus.borrow().handle(node(43)).is_none());
        })
        .unwrap();
    apply(cx, handle, vec![Op::SetStyle(node(35), vec![])]);
    frame(cx, handle).await;
    focus(cx, handle, 36);
    apply(
        cx,
        handle,
        vec![Op::Splice(node(35), 0, 1, vec![]), Op::Remove(node(36))],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(34)));
    assert!(!editor());
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((30..=44).filter(|i| *i != 36).map(|i| Op::Remove(node(i))))
            .collect(),
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(v.buttons.is_empty() && v.editors.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    println!(
        "GPUIO_CUSTOM_DISCLOSURE_OK: independent navigation/toggle, mixed header traversal, AppKit expanded state, collapse/unmount focus and disposal"
    );
}
