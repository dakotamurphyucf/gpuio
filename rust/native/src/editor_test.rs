//! Real GPUI window/keyboard/clipboard checks and macOS NSTextInputClient calls.
//! The IME scenario enters through the native NSView, not an editor test double.
use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
async fn frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let (sender, receiver) = async_channel::bounded(1);
    cx.update_window(handle.into(), |_, window, _| {
        window.refresh();
        // The second callback guarantees the preceding frame has painted and
        // installed its input handler. A fixed sleep can race an inactive or
        // busy window's frame scheduling.
        window.on_next_frame(move |window, _| {
            window.on_next_frame(move |_, _| {
                let _ = sender.try_send(());
            });
        });
    })
    .unwrap();
    receiver.recv().await.unwrap();
}
fn key(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, key: &str) {
    cx.update_window(handle.into(), |_, window, cx| {
        let keystroke = gpui::Keystroke::parse(key).unwrap();
        window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }),
            cx,
        );
    })
    .unwrap();
}
fn snapshot(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, node: NodeId) -> EditorSnapshot {
    handle
        .update(cx, |view, window, cx| {
            view.editors.get(&node).unwrap().snapshot(window, cx)
        })
        .unwrap()
}
fn command(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    node: NodeId,
    command: EditorCommand,
) -> EditorResult {
    handle
        .update(cx, |view, window, cx| {
            view.editors
                .get_mut(&node)
                .unwrap()
                .command(&command, window, cx)
        })
        .unwrap()
}
#[cfg(target_os = "macos")]
fn native_view(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> usize {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    handle
        .update(cx, |_, window, _| {
            let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else {
                panic!("expected AppKit window");
            };
            handle.ns_view.as_ptr() as usize
        })
        .unwrap()
}

#[cfg(target_os = "macos")]
fn accessible_input(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    replace: bool,
) -> Option<bool> {
    use objc2::{msg_send, runtime::AnyObject};
    use objc2_foundation::NSString;
    // Walk only this test window's native accessibility subtree. Invoking
    // accessibilityChildren also activates AccessKit's initial-tree callback.
    unsafe fn visit(object: *mut AnyObject, depth: usize, replace: bool) -> Option<bool> {
        if object.is_null() || depth > 16 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            let description: *mut NSString = msg_send![object, accessibilityHelp];
            let named = [title, description]
                .into_iter()
                .any(|value| !value.is_null() && (*value).to_string() == "Input 1");
            if named {
                let role: *mut NSString = msg_send![object, accessibilityRole];
                assert_eq!((*role).to_string(), "AXTextField");
                if replace {
                    let value = NSString::from_str("accessible é");
                    let _: () = msg_send![object, setAccessibilityValue: &*value];
                    let _: () = msg_send![object, setAccessibilityFocused: true];
                }
                let focused: objc2::runtime::Bool = msg_send![object, isAccessibilityFocused];
                return Some(focused.as_bool());
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 512);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex: index];
                if let Some(focused) = visit(child, depth + 1, replace) {
                    return Some(focused);
                }
            }
        }
        None
    }
    let view = native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, 0, replace)
    }
}

#[cfg(target_os = "macos")]
fn native_text(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, value: &str, marked: bool) {
    use objc2::{Encode, Encoding};
    use objc2::{msg_send, runtime::AnyObject};
    use objc2_foundation::{NSNotFound, NSString};
    // This GPUI revision registers Cocoa's NSRange encoding (without the
    // underscore used by objc2-foundation). Match that registered ABI exactly.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NativeRange {
        location: usize,
        length: usize,
    }
    unsafe impl Encode for NativeRange {
        const ENCODING: Encoding = Encoding::Struct("NSRange", &[usize::ENCODING, usize::ENCODING]);
    }
    let address = native_view(cx, handle);
    // Call outside Window::update: the OS callback must acquire the GPUI window
    // itself, as it would for actual NSTextInputClient input.
    let text = NSString::from_str(value);
    let replacement = NativeRange {
        location: NSNotFound as usize,
        length: 0,
    };
    unsafe {
        let view = &*(address as *const AnyObject);
        if marked {
            let selected = NativeRange {
                location: value.encode_utf16().count(),
                length: 0,
            };
            let _: () = msg_send![view, setMarkedText: &*text, selectedRange: selected, replacementRange: replacement];
        } else {
            let _: () = msg_send![view, insertText: &*text, replacementRange: replacement];
        }
    }
}
#[cfg(not(target_os = "macos"))]
fn native_text(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, value: &str, marked: bool) {
    assert!(
        !marked,
        "Linux OS IME scenario is separately validated under OCH-17"
    );
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.into())));
    key(cx, handle, "ctrl-v");
}
async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: Arc<Transport>,
    session: SharedSession,
    window_id: WindowId,
) {
    for _ in 0..100 {
        if handle
            .update(cx, |view, _, _| view.probes.borrow().contains_key(&node(2)))
            .unwrap()
        {
            break;
        }
        frame(cx, handle).await;
    }
    frame(cx, handle).await;
    native_text(cx, handle, "Aé👨‍👩‍👧‍👦", false);
    assert_eq!(snapshot(cx, handle, node(1)).text, "Aé👨‍👩‍👧‍👦");
    key(cx, handle, "backspace");
    assert_eq!(snapshot(cx, handle, node(1)).text, "Aé");
    assert_eq!(
        command(
            cx,
            handle,
            node(1),
            EditorCommand::Select(EditorSelection { anchor: 2, head: 2 })
        ),
        EditorResult::Failed(EditorError::InvalidSelection)
    );
    #[cfg(target_os = "macos")]
    let primary = "cmd";
    #[cfg(not(target_os = "macos"))]
    let primary = "ctrl";
    key(cx, handle, &format!("{primary}-a"));
    key(cx, handle, &format!("{primary}-c"));
    assert_eq!(
        cx.update(|cx| cx.read_from_clipboard().and_then(|item| item.text())),
        Some("Aé".into())
    );
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        snapshot(cx, handle, node(2)).focused,
        "Tab reaches composer"
    );
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(
        snapshot(cx, handle, node(1)).focused,
        "Shift-Tab returns to input"
    );
    command(cx, handle, node(2), EditorCommand::Focus);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        native_text(cx, handle, "に", true);
        let marked = snapshot(cx, handle, node(2));
        assert_eq!(marked.text, "に");
        assert!(marked.composition.is_some());
        key(cx, handle, "enter");
        let events = transport.mailbox.lock().unwrap().drain(256);
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::EditorEvent(_, _, _, _, EditorEventKind::Submitted, _)
        )));
        assert_eq!(
            command(
                cx,
                handle,
                node(2),
                EditorCommand::Replace(
                    "erase".into(),
                    EditorSelectionPolicy::Start,
                    EditorUndoPolicy::Reset,
                    None
                )
            ),
            EditorResult::Failed(EditorError::Composing)
        );
        native_text(cx, handle, "日本", false);
        assert!(snapshot(cx, handle, node(2)).composition.is_none());
        assert_eq!(snapshot(cx, handle, node(2)).text, "日本");
        command(cx, handle, node(2), EditorCommand::Undo);
        assert_eq!(snapshot(cx, handle, node(2)).text, "");
        command(cx, handle, node(2), EditorCommand::Redo);
        assert_eq!(snapshot(cx, handle, node(2)).text, "日本");
    }
    let mut heights = Vec::new();
    for rows in [1, 4, 8] {
        command(
            cx,
            handle,
            node(2),
            EditorCommand::Replace(
                std::iter::repeat_n("line", rows)
                    .collect::<Vec<_>>()
                    .join("\n"),
                EditorSelectionPolicy::End,
                EditorUndoPolicy::Reset,
                None,
            ),
        );
        frame(cx, handle).await;
        heights.push(
            handle
                .update(cx, |view, _, _| {
                    view.probes
                        .borrow()
                        .get(&node(2))
                        .unwrap()
                        .bounds
                        .size
                        .height
                })
                .unwrap(),
        );
    }
    assert!(
        heights[1] > heights[0],
        "composer grows with lines: {heights:?}"
    );
    assert_eq!(heights[1], heights[2], "composer stops growing at max_rows");
    command(
        cx,
        handle,
        node(2),
        EditorCommand::Replace(
            "prompt".into(),
            EditorSelectionPolicy::End,
            EditorUndoPolicy::Reset,
            None,
        ),
    );
    transport.mailbox.lock().unwrap().drain(256);
    key(cx, handle, "shift-enter");
    assert_eq!(snapshot(cx, handle, node(2)).text, "prompt\n");
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::EditorEvent(_, _, _, _, EditorEventKind::Submitted, _)
    )));
    let before = snapshot(cx, handle, node(2));
    key(cx, handle, "enter");
    native_text(cx, handle, "x", false);
    key(cx, handle, "backspace");
    let events = transport.mailbox.lock().unwrap().drain(256);
    let submits = events
        .iter()
        .filter_map(|event| match event {
            Event::EditorEvent(_, id, _, _, EditorEventKind::Submitted, snapshot)
                if *id == node(2) =>
            {
                Some(snapshot)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(submits.len(), 1);
    assert_eq!(submits[0].text, before.text);
    assert_eq!(submits[0].revision, before.revision);
    assert_eq!(snapshot(cx, handle, node(2)).text, before.text);
    assert_eq!(
        command(
            cx,
            handle,
            node(2),
            EditorCommand::Replace(
                "".into(),
                EditorSelectionPolicy::Start,
                EditorUndoPolicy::Record,
                Some(before.revision)
            )
        ),
        EditorResult::Failed(EditorError::StaleRevision)
    );
    let live = snapshot(cx, handle, node(2));
    let applied = session
        .borrow_mut()
        .apply(&Transaction {
            window: window_id,
            base: 1,
            revision: 2,
            operations: vec![Op::SetStyle(
                node(2),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(280.)),
                    Field::FontSize(18.),
                ])],
            )],
        })
        .unwrap();
    handle
        .update(cx, |view, window, cx| {
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle, node(2)), live);
    #[cfg(target_os = "macos")]
    {
        accessible_input(cx, handle, false);
        frame(cx, handle).await;
        assert!(
            accessible_input(cx, handle, true).is_some(),
            "native accessible input exists"
        );
        frame(cx, handle).await;
        assert_eq!(
            accessible_input(cx, handle, false),
            Some(true),
            "native accessibility reports focus"
        );
        assert_eq!(snapshot(cx, handle, node(1)).text, "accessible é");
        assert!(snapshot(cx, handle, node(1)).focused);
    }
    for (revision, read_only, disabled) in [(3, true, false), (4, false, true), (5, false, false)] {
        let mut config = session
            .borrow()
            .tree(window_id)
            .unwrap()
            .get(node(1))
            .unwrap()
            .editor
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        config.read_only = read_only;
        config.disabled = disabled;
        let applied = session
            .borrow_mut()
            .apply(&Transaction {
                window: window_id,
                base: revision - 1,
                revision,
                operations: vec![Op::SetEditor(node(1), config)],
            })
            .unwrap();
        handle
            .update(cx, |view, window, cx| {
                view.update_editors(&applied.dirty, window, cx);
                cx.notify();
            })
            .unwrap();
        frame(cx, handle).await;
        command(cx, handle, node(1), EditorCommand::Focus);
        frame(cx, handle).await;
        if disabled {
            assert!(!snapshot(cx, handle, node(1)).focused);
            command(cx, handle, node(2), EditorCommand::Focus);
            frame(cx, handle).await;
            key(cx, handle, "shift-tab");
            frame(cx, handle).await;
            assert!(
                snapshot(cx, handle, node(2)).focused,
                "disabled input is skipped"
            );
        } else {
            assert!(snapshot(cx, handle, node(1)).focused);
            let before = snapshot(cx, handle, node(1));
            native_text(cx, handle, "X", false);
            let after = snapshot(cx, handle, node(1));
            if read_only {
                assert_eq!(after.text, before.text);
                assert_eq!(after.revision, before.revision);
            } else {
                assert!(
                    after.revision > before.revision,
                    "re-enabled edit: {before:?} -> {after:?}"
                );
            }
        }
    }
    session.borrow_mut().close(window_id).unwrap();
    assert_eq!(
        command(cx, handle, node(2), EditorCommand::Focus),
        EditorResult::Failed(EditorError::StaleEditor)
    );
    handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
}
pub fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let window_id = WindowId::from_parts(0, 1).unwrap();
    gpui_platform::application().run(move |cx| {
        gpui_base::init(cx);
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let clipboard = cx.read_from_clipboard();
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "GPUIO native editor test", 460., 360.)
            .unwrap();
        let mut operations = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
        for (slot, kind) in [(1, Kind::Input), (2, Kind::Textarea)] {
            operations.extend([
                Op::Create(
                    node(slot),
                    kind,
                    "".into(),
                    Some(gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()),
                ),
                Op::SetEditor(
                    node(slot),
                    EditorConfig {
                        label: format!("Input {slot}"),
                        placeholder: "Type here".into(),
                        read_only: false,
                        disabled: false,
                        submit_on_enter: true,
                        auto_focus: slot == 1,
                        min_rows: 1,
                        max_rows: if slot == 1 { 1 } else { 4 },
                    },
                ),
                Op::SetStyle(
                    node(slot),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(400.)),
                        Field::FontSize(16.),
                    ])],
                ),
            ]);
        }
        operations.extend([
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ]);
        let applied = session
            .borrow_mut()
            .apply(&Transaction {
                window: window_id,
                base: 0,
                revision: 1,
                operations,
            })
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    inactive_frame_interval: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(460.), px(360.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
            )
            .unwrap();
        handle
            .update(cx, |view, window, cx| {
                view.update_editors(&applied.dirty, window, cx);
                window.activate_window();
                cx.notify();
            })
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result =
                super::native_test::protect(exercise(cx, handle, transport, session, window_id))
                    .await;
            *task_failure.borrow_mut() = result.err();
            cx.update(|cx| {
                if let Some(clipboard) = clipboard {
                    cx.write_to_clipboard(clipboard);
                }
                stop_application(cx);
            });
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
    println!(
        "GPUIO_EDITOR_NATIVE_OK: actual window/input, graphemes, clipboard, Tab/Shift-Tab, key policy, revision guard, auto-grow, resize and close"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_EDITOR_MACOS_TEXT_CLIENT_OK: native NSView marked/committed UTF-16 input, composition submit guard, undo transaction and accessibility focus/value actions"
    );
}
