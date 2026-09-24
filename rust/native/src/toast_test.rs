//! Real GPUI notification deadlines, native input and hidden lifetime checks.
use super::*;
fn config(timeout_ns: Option<i64>) -> ToastConfig {
    ToastConfig {
        label: "Saved notification".into(),
        close_label: "Dismiss saved".into(),
        timeout_ns,
        politeness: ToastPoliteness::Polite,
    }
}
fn stack_config() -> ToastStackConfig {
    ToastStackConfig {
        label: "Notifications".into(),
        corner: ToastCorner::BottomRight,
        width: 240.,
        max_visible: 1,
    }
}
fn create(slot: i64, timeout: Option<i64>) -> Vec<Op> {
    vec![
        Op::Create(
            node(slot),
            Kind::Toast,
            "".into(),
            Some(gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()),
        ),
        Op::SetToast(node(slot), config(timeout)),
        Op::Create(node(slot + 1), Kind::Text, "Draft saved".into(), None),
        Op::Splice(node(slot), 0, 0, vec![node(slot + 1)]),
    ]
}
fn dismissals(transport: &Transport) -> Vec<(NodeId, ToastDismissal)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::ToastDismissed(_, node, _, _, reason) = event {
                Some((node, reason))
            } else {
                None
            }
        })
        .collect()
}
fn status(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) -> (bool, bool) {
    handle
        .update(cx, |view, _, _| view.toasts[&node(slot)].status())
        .unwrap()
}
async fn pause(cx: &mut gpui::AsyncApp, ms: u64) {
    cx.background_executor()
        .timer(std::time::Duration::from_millis(ms))
        .await;
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    super::super::native_test::move_mouse(cx, handle, gpui::point(px(1.), px(1.)), false);
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(2)].focus, cx)
        })
        .unwrap();
    let mut ops = vec![
        Op::Create(node(61), Kind::ToastStack, "".into(), None),
        Op::SetToastStack(node(61), stack_config()),
    ];
    ops.extend(create(62, None));
    ops.extend([
        Op::Splice(node(61), 0, 0, vec![node(62)]),
        Op::Splice(node(0), 4, 0, vec![node(61)]),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(2)),
        "notifications never autofocus"
    );
    assert_eq!(
        status(cx, handle, 62),
        (false, false),
        "persistent notification has no timer"
    );
    handle
        .update(cx, |view, window, _| {
            let bounds = view.toasts[&node(62)].bounds.get();
            assert_eq!(bounds.size.width, px(240.));
            assert_eq!(bounds.right(), window.viewport_size().width - px(16.));
            assert_eq!(bounds.bottom(), window.viewport_size().height - px(16.));
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Dismiss saved", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Dismiss saved", false).unwrap().role,
            "AXButton"
        );
        accessible(cx, handle, "Dismiss saved", true).unwrap();
        frame(cx, handle).await;
        assert_eq!(
            dismissals(transport),
            vec![(node(62), ToastDismissal::CloseButton)]
        );
        assert_eq!(status(cx, handle, 62), (true, false));
    }
    #[cfg(not(target_os = "macos"))]
    {
        handle
            .update(cx, |view, window, cx| {
                window.focus(&view.toasts[&node(62)].close_focus, cx)
            })
            .unwrap();
        key(cx, handle, "escape");
        frame(cx, handle).await;
        assert_eq!(
            dismissals(transport),
            vec![(node(62), ToastDismissal::Escape)]
        );
    }
    apply(
        cx,
        handle,
        vec![Op::SetToast(node(62), config(Some(1_000_000)))],
    );
    pause(cx, 30).await;
    assert_eq!(
        status(cx, handle, 62),
        (true, false),
        "config cannot rearm a closed keyed session"
    );
    assert!(dismissals(transport).is_empty());
    let mut ops = create(64, Some(800_000_000));
    ops.push(Op::Splice(node(61), 0, 1, vec![node(64)]));
    ops.extend([
        Op::Splice(node(62), 0, 1, vec![]),
        Op::Remove(node(63)),
        Op::Remove(node(62)),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    assert_eq!(status(cx, handle, 64), (false, true));
    let target = handle
        .update(cx, |view, _, _| {
            view.toasts[&node(64)].bounds.get().center()
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, target, false);
    frame(cx, handle).await;
    assert_eq!(
        status(cx, handle, 64),
        (false, false),
        "hover pauses the deadline"
    );
    // The host OS may replace injected mouse coordinates while the owner uses
    // the desktop. Verify native hover pause immediately; use keyboard focus
    // for sustained pause and deterministic clock tests for elapsed-time math.
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.toasts[&node(64)].close_focus, cx)
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, gpui::point(px(1.), px(1.)), false);
    frame(cx, handle).await;
    assert_eq!(
        status(cx, handle, 64),
        (false, false),
        "keyboard focus keeps the stack paused"
    );
    pause(cx, 900).await;
    assert!(dismissals(transport).is_empty());
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(
        dismissals(transport),
        vec![(node(64), ToastDismissal::Escape)]
    );
    assert!(
        focused(cx, handle, node(2)),
        "closing focused toast restores previous focus"
    );
    let mut ops = create(66, Some(250_000_000));
    ops.push(Op::Splice(node(61), 0, 1, vec![node(66)]));
    ops.extend([
        Op::Splice(node(64), 0, 1, vec![]),
        Op::Remove(node(65)),
        Op::Remove(node(64)),
    ]);
    // Hide on mount: no time is consumed before first visibility.
    ops.push(Op::SetStyle(
        node(61),
        vec![Style::Fields(vec![Field::Visibility(1)])],
    ));
    apply(cx, handle, ops);
    frame(cx, handle).await;
    pause(cx, 350).await;
    assert_eq!(status(cx, handle, 66), (false, false));
    assert!(dismissals(transport).is_empty());
    apply(cx, handle, vec![Op::SetStyle(node(61), vec![])]);
    frame(cx, handle).await;
    assert_eq!(status(cx, handle, 66), (false, true));
    pause(cx, 400).await; // Native executor must wake without an application commit.
    assert_eq!(
        dismissals(transport),
        vec![(node(66), ToastDismissal::Timeout)]
    );
    assert_eq!(status(cx, handle, 66), (true, false));
    let mut ops = create(68, None);
    ops.extend(create(70, None));
    ops.push(Op::Splice(node(61), 0, 1, vec![node(68), node(70)]));
    ops.extend([
        Op::Splice(node(66), 0, 1, vec![]),
        Op::Remove(node(67)),
        Op::Remove(node(66)),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    assert_eq!(
        dismissals(transport),
        vec![(node(68), ToastDismissal::Overflow)]
    );
    assert_eq!(status(cx, handle, 68), (true, false));
    assert_eq!(status(cx, handle, 70), (false, false));
    // Modal blocking must pause even a notification mounted after the modal.
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(72),
                Kind::FocusScope,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(72, 1).unwrap()),
            ),
            Op::SetFocusScope(
                node(72),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node(72),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Notification modal".into(),
                    width: 220.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::Create(
                node(73),
                Kind::Button,
                "Modal action".into(),
                Some(gpuio_protocol::HandlerId::from_parts(73, 1).unwrap()),
            ),
            Op::SetControl(node(73), Control::Button(false)),
            Op::Splice(node(72), 0, 0, vec![node(73)]),
            Op::Splice(node(0), 5, 0, vec![node(72)]),
        ],
    );
    frame(cx, handle).await;
    let mut ops = create(74, Some(800_000_000));
    ops.extend([
        Op::Splice(node(61), 1, 1, vec![node(74)]),
        Op::Splice(node(70), 0, 1, vec![]),
        Op::Remove(node(71)),
        Op::Remove(node(70)),
        Op::Create(
            node(76),
            Kind::Button,
            "Notification action".into(),
            Some(gpuio_protocol::HandlerId::from_parts(76, 1).unwrap()),
        ),
        Op::SetControl(node(76), Control::Button(false)),
        Op::Splice(node(74), 1, 0, vec![node(76)]),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(73)));
    assert_eq!(
        status(cx, handle, 74),
        (false, false),
        "outside modal notification pauses"
    );
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Dismiss saved", true);
    }
    pause(cx, 900).await;
    assert!(
        dismissals(transport).is_empty(),
        "blocked accessibility and timeout cannot close a notification"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 5, 1, vec![]),
            Op::Splice(node(72), 0, 1, vec![]),
            Op::Remove(node(73)),
            Op::Remove(node(72)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(76)].focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(
        presses(transport),
        vec![node(76)],
        "ordinary content actions use the existing bridge"
    );
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.toasts[&node(74)].close_focus, cx)
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(74),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(
                !view.toasts[&node(74)].close_focus.is_focused(window),
                "hidden toast relinquishes keyboard focus before paint"
            );
        })
        .unwrap();
    assert_eq!(status(cx, handle, 74), (false, false));
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(74),
            vec![Style::Fields(vec![Field::PointerEvents(false)])],
        )],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.toasts[&node(74)].close_focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "space");
    frame(cx, handle).await;
    assert_eq!(
        dismissals(transport),
        vec![(node(74), ToastDismissal::CloseButton)],
        "pointer-disabled close remains keyboard operable"
    );
    // Native composition consumes the first Escape; the next closes the toast.
    let mut ops = create(77, None);
    ops.extend([
        Op::Create(
            node(79),
            Kind::Input,
            "".into(),
            Some(gpuio_protocol::HandlerId::from_parts(79, 1).unwrap()),
        ),
        Op::SetEditor(
            node(79),
            EditorConfig {
                label: "Notification editor".into(),
                placeholder: "".into(),
                read_only: false,
                disabled: false,
                auto_focus: false,
                submit_on_enter: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::Splice(node(77), 1, 0, vec![node(79)]),
        Op::Splice(node(61), 2, 0, vec![node(77)]),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(79)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "に", true);
        frame(cx, handle).await;
        handle
            .update(cx, |view, _, cx| {
                assert!(view.editors[&node(79)].is_composing(cx))
            })
            .unwrap();
        dismissals(transport);
        key(cx, handle, "escape");
        frame(cx, handle).await;
        assert!(
            dismissals(transport).is_empty(),
            "composition Escape does not dismiss the notification"
        );
        handle
            .update(cx, |view, _, cx| {
                assert!(!view.editors[&node(79)].is_composing(cx))
            })
            .unwrap();
    }
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(
        dismissals(transport),
        vec![(node(77), ToastDismissal::Escape)]
    );
    // Mount an active timer and remove its entire stack before the deadline.
    let mut ops = create(80, Some(200_000_000));
    ops.push(Op::Splice(node(61), 3, 0, vec![node(80)]));
    apply(cx, handle, ops);
    assert_eq!(status(cx, handle, 80), (false, true));
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 4, 1, vec![]),
            Op::Splice(node(61), 0, 4, vec![]),
            Op::Splice(node(68), 0, 1, vec![]),
            Op::Splice(node(74), 0, 2, vec![]),
            Op::Splice(node(77), 0, 2, vec![]),
            Op::Splice(node(80), 0, 1, vec![]),
            Op::Remove(node(69)),
            Op::Remove(node(75)),
            Op::Remove(node(76)),
            Op::Remove(node(78)),
            Op::Remove(node(79)),
            Op::Remove(node(81)),
            Op::Remove(node(68)),
            Op::Remove(node(74)),
            Op::Remove(node(77)),
            Op::Remove(node(80)),
            Op::Remove(node(61)),
        ],
    );
    frame(cx, handle).await;
    pause(cx, 300).await;
    assert!(
        dismissals(transport).is_empty(),
        "removed deadline cannot publish stale dismissal"
    );
    handle
        .update(cx, |view, _, _| {
            assert!(view.toasts.is_empty());
            assert!(view.toast_stacks.is_empty());
        })
        .unwrap();
    println!(
        "GPUIO_TOAST_NATIVE_OK: corner geometry, no autofocus, close/Escape, terminal config, hover/focus/hidden pause, native expiry, overflow, modal gating, arbitrary actions, pointer-independent keyboard activation, native editor Escape priority and unmount cancellation"
    );
}
