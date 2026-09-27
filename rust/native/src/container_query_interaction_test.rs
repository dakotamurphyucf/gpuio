//! Real native editor input, accessibility exposure and held capture at selection.
use super::*;

fn editor_config(label: &str) -> EditorConfig {
    EditorConfig {
        label: label.into(),
        placeholder: "".into(),
        read_only: false,
        disabled: false,
        submit_on_enter: true,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    }
}
fn snapshot(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, id: NodeId) -> EditorSnapshot {
    window
        .update(cx, |v, w, cx| v.editors[&id].snapshot(w, cx))
        .unwrap()
}
fn focus(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, id: NodeId) -> EditorResult {
    window
        .update(cx, |v, w, cx| {
            v.editors
                .get_mut(&id)
                .unwrap()
                .command(&EditorCommand::Focus, w, cx)
        })
        .unwrap()
}
#[cfg(target_os = "macos")]
fn accessible_labels(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> Vec<String> {
    use objc2::{msg_send, runtime::AnyObject};
    use objc2_foundation::NSString;
    unsafe fn visit(object: *mut AnyObject, depth: usize, labels: &mut Vec<String>) {
        if object.is_null() || depth > 20 {
            return;
        }
        unsafe {
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !role.is_null() && (*role).to_string() == "AXTextField" {
                let title: *mut NSString = msg_send![object, accessibilityTitle];
                if !title.is_null() {
                    labels.push((*title).to_string());
                }
                let help: *mut NSString = msg_send![object, accessibilityHelp];
                if !help.is_null() {
                    labels.push((*help).to_string());
                }
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex: index];
                visit(child, depth + 1, labels);
            }
        }
    }
    let mut labels = Vec::new();
    unsafe {
        let view = super::super::editor_test::native_view(cx, window) as *mut AnyObject;
        let native_window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![native_window, contentView];
        visit(content, 0, &mut labels);
    }
    labels
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = cx.update(|cx| {
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Query native input", 360., 220.)
            .unwrap();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(360.), px(220.)),
                    cx,
                ))),
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone())),
        )
        .unwrap()
    });
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::ContainerQuery, "".into(), Some(handler(0))),
            Op::Create(node(1), Kind::Container, "".into(), None),
            Op::Create(node(2), Kind::Input, "wide draft".into(), Some(handler(2))),
            Op::Create(node(3), Kind::Input, "".into(), Some(handler(3))),
            Op::Create(node(4), Kind::PointerArea, "".into(), Some(handler(4))),
            Op::SetContainerQuery(node(0), config(1, 400.)),
            Op::SetEditor(node(2), editor_config("Wide query editor")),
            Op::SetEditor(node(3), editor_config("Compact query editor")),
            Op::SetStyle(
                node(2),
                vec![
                    Style::Width(Length::Px(250.)),
                    Style::Height(Length::Px(40.)),
                ],
            ),
            Op::SetStyle(
                node(3),
                vec![
                    Style::Width(Length::Px(250.)),
                    Style::Height(Length::Px(40.)),
                ],
            ),
            Op::SetPointer(
                node(4),
                PointerConfig {
                    label: "Query drag".into(),
                    button: PointerButton::Left,
                    disabled: false,
                    prevent_default: true,
                    stop_propagation: true,
                },
            ),
            Op::SetStyle(
                node(4),
                vec![
                    Style::Width(Length::Px(180.)),
                    Style::Height(Length::Px(60.)),
                ],
            ),
            Op::Splice(node(1), 0, 0, vec![node(3), node(4)]),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, window).await;
    #[cfg(target_os = "macos")]
    {
        accessible_labels(cx, window); // Activate AccessKit before the next paint.
        frame(cx, window).await;
        let labels = accessible_labels(cx, window);
        assert!(
            labels.iter().any(|s| s == "Compact query editor"),
            "{labels:?}"
        );
        assert!(
            !labels.iter().any(|s| s == "Wide query editor"),
            "hidden editor absent from AppKit AX tree"
        );
    }
    assert!(matches!(
        focus(cx, window, node(3)),
        EditorResult::Applied(_)
    ));
    frame(cx, window).await;
    super::super::editor_test::native_text(cx, window, "draft 👩🏽‍💻", false);
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, window, "仮", true);
        assert!(snapshot(cx, window, node(3)).composition.is_some());
    }
    let before = snapshot(cx, window, node(3));
    assert!(before.focused);
    window
        .update(cx, |_, w, _| w.resize(size(px(480.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    let hidden = snapshot(cx, window, node(3));
    assert!(!hidden.focused);
    assert_eq!(hidden.text, before.text);
    assert_eq!(hidden.selection, before.selection);
    assert_eq!(
        focus(cx, window, node(3)),
        EditorResult::Failed(EditorError::FocusBlocked)
    );
    assert_eq!(snapshot(cx, window, node(2)).text, "wide draft");
    assert!(snapshot(cx, window, node(2)).composition.is_none());
    // No editor receives native text until the application explicitly focuses one.
    super::super::editor_test::native_text(cx, window, "late input", false);
    assert_eq!(snapshot(cx, window, node(3)).text, hidden.text);
    assert_eq!(snapshot(cx, window, node(2)).text, "wide draft");
    #[cfg(target_os = "macos")]
    {
        let labels = accessible_labels(cx, window);
        assert!(!labels.iter().any(|s| s == "Compact query editor"));
        assert!(labels.iter().any(|s| s == "Wide query editor"));
    }
    window
        .update(cx, |_, w, _| w.resize(size(px(360.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    assert!(matches!(
        focus(cx, window, node(3)),
        EditorResult::Applied(_)
    ));
    frame(cx, window).await;
    super::super::editor_test::native_text(cx, window, "確", false);
    assert!(snapshot(cx, window, node(3)).text.ends_with("確"));
    assert!(snapshot(cx, window, node(3)).composition.is_none());
    assert_eq!(snapshot(cx, window, node(2)).text, "wide draft");

    // Native held capture is cancelled when its presentation disappears.
    let position = window
        .update(cx, |v, _, _| v.probes.borrow()[&node(4)].bounds.center())
        .unwrap();
    transport.mailbox.lock().unwrap().drain(256);
    super::super::native_test::move_mouse(cx, window, position, false);
    super::super::native_test::mouse(cx, window, position, true);
    assert!(
        window
            .update(cx, |_, w, _| w.captured_hitbox().is_some())
            .unwrap()
    );
    window
        .update(cx, |_, w, _| w.resize(size(px(480.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    assert!(
        window
            .update(cx, |_, w, _| w.captured_hitbox().is_none())
            .unwrap()
    );
    let events = transport.mailbox.lock().unwrap().drain(256);
    let phases: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::PointerEvent(_, id, _, _, sample) if *id == node(4) => Some(sample.phase),
            _ => None,
        })
        .collect();
    assert_eq!(phases.first(), Some(&PointerPhase::Started));
    assert_eq!(
        phases.last(),
        Some(&PointerPhase::Cancelled(PointerCancel::Hidden))
    );
    super::super::native_test::mouse(cx, window, position, false);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|e| matches!(e, Event::PointerEvent(..))),
        "late release does not finish a cancelled gesture"
    );
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(4)),
            Op::Remove(node(3)),
            Op::Remove(node(2)),
            Op::Remove(node(1)),
            Op::Remove(node(0)),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |v, w, _| {
            assert!(v.editors.is_empty());
            assert!(v.container_queries.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_CONTAINER_QUERY_INPUT_OK: retained native drafts, focus denial, composition isolation, held capture cancellation, late-release suppression and disposal"
    );
    #[cfg(target_os = "macos")]
    eprintln!(
        "GPUIO_CONTAINER_QUERY_AX_IME_OK: AppKit accessibility exposes only the selected editor; NSTextInputClient marked/committed input stays with its retained owner"
    );
}
