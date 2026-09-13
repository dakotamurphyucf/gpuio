//! Real-window control activation, focus traversal and native accessibility.
use super::editor_test::{frame, key};
use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn presses(transport: &Transport) -> Vec<NodeId> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::Press(_, node, _, _) = event {
                Some(node)
            } else {
                None
            }
        })
        .collect()
}

fn choices(transport: &Transport) -> Vec<String> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::Choice(_, _, _, _, selected) = event {
                Some(selected)
            } else {
                None
            }
        })
        .collect()
}

async fn radio(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut config = ChoiceConfig {
        label: "Mode".into(),
        selected: Some("fast".into()),
        disabled: false,
        items: [
            ("fast", "Fast", false),
            ("blocked", "Disabled option", true),
            ("deep", "Deep", false),
        ]
        .into_iter()
        .map(|(id, label, disabled)| ChoiceItem {
            id: id.into(),
            label: label.into(),
            disabled,
        })
        .collect(),
    };
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(5),
                Kind::RadioGroup,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::SetChoice(node(5), config.clone()),
            Op::SetStyle(
                node(5),
                vec![Style::Fields(vec![Field::Width(Length::Px(300.))])],
            ),
            Op::Splice(node(0), 4, 0, vec![node(5)]),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(5)), "radio group is one Tab stop");
    choices(transport);
    key(cx, handle, "down");
    key(cx, handle, "down");
    assert_eq!(
        choices(transport),
        ["deep", "fast"],
        "skip disabled, wrap, and preserve consecutive navigation before value commits"
    );
    key(cx, handle, "end");
    assert_eq!(choices(transport), ["deep"]);
    // Committed selected value is independent of the active navigation target.
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .get(node(5))
                    .unwrap()
                    .choice
                    .as_ref()
                    .unwrap()
                    .selected
                    .as_deref(),
                Some("fast")
            )
        })
        .unwrap();
    config.items.reverse();
    apply(cx, handle, vec![Op::SetChoice(node(5), config.clone())]);
    frame(cx, handle).await;
    key(cx, handle, "space");
    assert_eq!(
        choices(transport),
        ["deep"],
        "reorder retains active option ID"
    );
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Fast", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Fast", false),
            Some(Accessible {
                role: "AXRadioButton".into(),
                value: 1,
                enabled: true
            })
        );
        assert_eq!(
            accessible(cx, handle, "Deep", true),
            Some(Accessible {
                role: "AXRadioButton".into(),
                value: 0,
                enabled: true
            })
        );
        frame(cx, handle).await;
        assert_eq!(choices(transport), ["deep"]);
        assert_eq!(
            accessible(cx, handle, "Disabled option", false),
            Some(Accessible {
                role: "AXRadioButton".into(),
                value: 0,
                enabled: false
            })
        );
    }
    config.items.retain(|item| item.id != "deep");
    apply(cx, handle, vec![Op::SetChoice(node(5), config.clone())]);
    frame(cx, handle).await;
    key(cx, handle, "space");
    assert_eq!(
        choices(transport),
        ["fast"],
        "removed active option falls back to a valid value"
    );
    config.disabled = true;
    apply(cx, handle, vec![Op::SetChoice(node(5), config.clone())]);
    frame(cx, handle).await;
    assert!(
        !focused(cx, handle, node(5)),
        "disabled group releases focus"
    );
    key(cx, handle, "space");
    assert!(choices(transport).is_empty());
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(!focused(cx, handle, node(5)), "disabled group is skipped");
    config.disabled = false;
    apply(cx, handle, vec![Op::SetChoice(node(5), config)]);
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(5)].focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(4)),
        "radio options do not add duplicate Tab stops"
    );
    apply(
        cx,
        handle,
        vec![Op::Remove(node(5)), Op::Splice(node(0), 4, 1, vec![])],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| assert!(view.radios.is_empty()))
        .unwrap();
    println!(
        "GPUIO_RADIO_NATIVE_OK: stable choices, arrow navigation, disabled options/groups, reorder, single Tab stop and disposal"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_RADIO_MACOS_AX_OK: option roles, checked values, disabled state and activation"
    );
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
fn focused(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: NodeId) -> bool {
    handle
        .update(cx, |view, window, cx| {
            if let Some(editor) = view.editors.get(&id) {
                editor.focus_handle(cx).is_focused(window)
            } else {
                view.buttons
                    .get(&id)
                    .is_some_and(|button| button.focus.is_focused(window))
            }
        })
        .unwrap()
}

fn assert_color(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: NodeId, expected: u32) {
    handle
        .update(cx, |view, _, _| {
            assert_eq!(view.probes.borrow()[&id].color, rgba(expected).into());
        })
        .unwrap();
}

#[cfg(target_os = "macos")]
#[derive(Debug, PartialEq)]
struct Accessible {
    role: String,
    value: isize,
    enabled: bool,
}

#[cfg(target_os = "macos")]
fn accessible(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    label: &str,
    press: bool,
) -> Option<Accessible> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(
        object: *mut AnyObject,
        label: &str,
        press: bool,
        depth: usize,
    ) -> Option<Accessible> {
        if object.is_null() || depth > 16 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if !title.is_null() && (*title).to_string() == label {
                let role: *mut NSString = msg_send![object, accessibilityRole];
                let value: *mut AnyObject = msg_send![object, accessibilityValue];
                let value: isize = if value.is_null() {
                    -1
                } else {
                    msg_send![value, integerValue]
                };
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                if press {
                    let _: () = msg_send![object, setAccessibilityFocused: true];
                    let accepted: Bool = msg_send![object, accessibilityPerformPress];
                    assert!(accepted.as_bool());
                }
                return Some(Accessible {
                    role: (*role).to_string(),
                    value,
                    enabled: enabled.as_bool(),
                });
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex: index];
                if let Some(found) = visit(child, label, press, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
    }
    let view = super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, label, press, 0)
    }
}

async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: Arc<Transport>) {
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(1)));
    assert_color(cx, handle, node(1), 0x555555ff);
    presses(&transport);
    key(cx, handle, "space");
    key(cx, handle, "space");
    assert_eq!(
        presses(&transport),
        [node(1), node(1)],
        "two activations before any value commit"
    );
    key(cx, handle, "enter");
    assert_eq!(presses(&transport), [node(1)]);
    let position = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(2)].bounds.center()
        })
        .unwrap();
    super::native_test::move_mouse(cx, handle, position, false);
    super::native_test::mouse(cx, handle, position, true);
    super::native_test::mouse(cx, handle, position, false);
    assert!(
        focused(cx, handle, node(1)),
        "pointer-disabled control does not steal focus"
    );
    assert!(presses(&transport).is_empty());
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(2)));
    key(cx, handle, "space");
    assert_eq!(
        presses(&transport),
        [node(2)],
        "pointer policy does not suppress keyboard"
    );
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(4)),
        "disabled button skipped; editor owns one Tab stop"
    );
    assert_color(cx, handle, node(4), 0x555555ff);
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(2)));

    apply(
        cx,
        handle,
        vec![Op::SetControl(node(2), Control::Switch(true, true))],
    );
    frame(cx, handle).await;
    assert!(!focused(cx, handle, node(2)), "disable blurs focus");
    assert_color(cx, handle, node(2), 0x444444ff);
    presses(&transport);
    key(cx, handle, "space");
    assert!(presses(&transport).is_empty());
    apply(
        cx,
        handle,
        vec![
            Op::SetControl(node(2), Control::Switch(true, false)),
            Op::SetControl(node(1), Control::Checkbox(CheckState::Indeterminate, false)),
        ],
    );
    frame(cx, handle).await;

    #[cfg(target_os = "macos")]
    {
        // First query activates AccessKit, then a frame publishes the tree.
        let _ = accessible(cx, handle, "Check", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Check", false),
            Some(Accessible {
                role: "AXCheckBox".into(),
                value: 2,
                enabled: true
            })
        );
        assert_eq!(
            accessible(cx, handle, "Switch", true),
            Some(Accessible {
                role: "AXCheckBox".into(),
                value: 1,
                enabled: true
            })
        );
        frame(cx, handle).await;
        assert!(focused(cx, handle, node(2)));
        assert_eq!(
            presses(&transport),
            [node(2)],
            "accessibility works when pointer events are disabled"
        );
        apply(
            cx,
            handle,
            vec![Op::SetControl(
                node(1),
                Control::Checkbox(CheckState::Unchecked, true),
            )],
        );
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Check", false),
            Some(Accessible {
                role: "AXCheckBox".into(),
                value: 0,
                enabled: false
            })
        );
    }
    radio(cx, handle, &transport).await;
    // Remove a focused native node and enter the surviving Tab order again.
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(2)].focus, cx)
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::SetControl(node(1), Control::Checkbox(CheckState::Indeterminate, false)),
            Op::Remove(node(2)),
            Op::Splice(node(0), 1, 1, vec![]),
        ],
    );
    frame(cx, handle).await;
    assert_color(cx, handle, node(1), 0x333333ff);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(1)),
        "removed focus recovers at the window root"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(1)),
            Op::Remove(node(3)),
            Op::Remove(node(4)),
            Op::Remove(node(0)),
            Op::SetRoot(None),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(view.buttons.is_empty());
            assert!(view.editors.is_empty());
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            );
            window.remove_window();
        })
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
    gpui_platform::application().run(move |cx| {
        gpui_base::init(cx);
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let id = WindowId::from_parts(0, 1).unwrap();
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, id, "GPUIO controls test", 400., 280.)
            .unwrap();
        let mut operations = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
        for (slot, label, control) in [
            (1, "Check", Control::Checkbox(CheckState::Unchecked, false)),
            (2, "Switch", Control::Switch(false, false)),
            (3, "Disabled", Control::Button(true)),
        ] {
            operations.extend([
                Op::Create(
                    node(slot),
                    control.kind(),
                    label.into(),
                    Some(gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()),
                ),
                Op::SetControl(node(slot), control),
                Op::SetStyle(
                    node(slot),
                    vec![
                        Style::Fields(vec![
                            Field::Width(Length::Px(300.)),
                            Field::Height(Length::Px(45.)),
                            Field::PointerEvents(slot != 2),
                            Field::Foreground(Color::Rgba(0x111111ff)),
                        ]),
                        Style::State(1, vec![Field::Foreground(Color::Rgba(0x555555ff))]),
                        Style::State(4, vec![Field::Foreground(Color::Rgba(0x222222ff))]),
                        Style::State(5, vec![Field::Foreground(Color::Rgba(0x333333ff))]),
                        Style::State(6, vec![Field::Foreground(Color::Rgba(0x444444ff))]),
                    ],
                ),
            ]);
        }
        operations.extend([
            Op::Create(
                node(4),
                Kind::Input,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(4, 1).unwrap()),
            ),
            Op::SetEditor(
                node(4),
                EditorConfig {
                    label: "Editor".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: true,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                node(4),
                vec![Style::State(
                    1,
                    vec![Field::Foreground(Color::Rgba(0x555555ff))],
                )],
            ),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2), node(3), node(4)]),
            Op::SetRoot(Some(node(0))),
        ]);
        let applied = session
            .borrow_mut()
            .apply(&Transaction {
                window: id,
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
                        size(px(400.), px(280.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id, session, transport.clone())),
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
            let result = super::native_test::protect(exercise(cx, handle, transport)).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
    println!(
        "GPUIO_CONTROLS_NATIVE_OK: activation, keyboard, Tab/Shift-Tab, pointer policy, native state styling, disable/re-enable and disposal"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_CONTROLS_MACOS_AX_OK: roles, mixed/checked/unchecked values, disabled state and focus/press actions"
    );
}
