//! Foreground window coverage for retained tooltip content and native triggers.
use super::*;
fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn events(transport: &Transport) -> Vec<bool> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::TooltipOpenChanged(_, _, _, _, open) => Some(open),
            _ => None,
        })
        .collect()
}
fn visible(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
    handle
        .update(cx, |view, _, _| !view.focus.borrow().hidden(node(30)))
        .unwrap()
}
fn focus(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(slot)].focus, cx)
        })
        .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut config = TooltipConfig {
        label: "Tooltip details".into(),
        width: 220.,
        open_state: TooltipOpenState::Managed(false),
        disabled: false,
        hoverable: true,
        show_delay_ns: 0,
        hide_delay_ns: 0,
        skip_delay_ns: 300_000_000,
    };
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(27), Kind::Container, "".into(), None),
            Op::SetStyle(
                node(27),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(100.)),
                    Field::Top(Length::Px(180.)),
                    Field::Width(Length::Px(140.)),
                ])],
            ),
            Op::Create(node(28), Kind::Tooltip, "".into(), Some(handler(28))),
            Op::SetTooltip(node(28), config.clone()),
            Op::Create(
                node(29),
                Kind::Button,
                "Tooltip anchor".into(),
                Some(handler(29)),
            ),
            Op::SetControl(node(29), Control::Button(false)),
            Op::SetStyle(
                node(29),
                vec![
                    Style::Width(Length::Px(140.)),
                    Style::Height(Length::Px(28.)),
                ],
            ),
            Op::Create(
                node(30),
                Kind::Input,
                "Retained é".into(),
                Some(handler(30)),
            ),
            Op::SetEditor(
                node(30),
                EditorConfig {
                    label: "Tooltip editor".into(),
                    placeholder: "".into(),
                    disabled: false,
                    read_only: false,
                    submit_on_enter: false,
                    auto_focus: true,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::Splice(node(28), 0, 0, vec![node(29), node(30)]),
            Op::Splice(node(27), 0, 0, vec![node(28)]),
            Op::Splice(node(0), 4, 0, vec![node(27)]),
        ],
    );
    frame(cx, handle).await;
    assert!(!visible(cx, handle));
    assert!(
        focused(cx, handle, node(4)),
        "hidden editor must not autofocus"
    );
    #[cfg(target_os = "macos")]
    {
        assert!(
            accessible(cx, handle, "Tooltip details", false).is_none(),
            "hidden content is absent from accessibility"
        );
    }
    let editor = handle
        .update(cx, |view, window, cx| {
            assert_eq!(
                view.editors
                    .get_mut(&node(30))
                    .unwrap()
                    .command(&EditorCommand::Focus, window, cx),
                EditorResult::Failed(EditorError::FocusBlocked)
            );
            view.editors[&node(30)].focus_handle(cx)
        })
        .unwrap();
    events(transport);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(29)));
    assert!(visible(cx, handle), "anchor focus opens immediately");
    assert_eq!(events(transport), [true]);
    #[cfg(target_os = "macos")]
    {
        // AccessKit maps Tooltip to AXGroup plus its tooltip subrole.
        assert_eq!(
            accessible(cx, handle, "Tooltip details", false)
                .unwrap()
                .role,
            "AXGroup"
        );
    }

    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(!visible(cx, handle));
    assert!(focused(cx, handle, node(29)), "Escape retains anchor focus");
    assert_eq!(events(transport), [false]);
    frame(cx, handle).await;
    assert!(
        !visible(cx, handle),
        "dismissal suppresses reopening from unchanged focus"
    );
    focus(cx, handle, 2);
    frame(cx, handle).await;
    focus(cx, handle, 29);
    frame(cx, handle).await;
    assert!(visible(cx, handle));
    handle
        .update(cx, |view, window, cx| {
            assert_eq!(view.editors[&node(30)].focus_handle(cx), editor);
            window.focus(&editor, cx);
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(
        visible(cx, handle),
        "interactive content focus keeps tooltip open"
    );
    key(cx, handle, "escape");
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(!visible(cx, handle));
    assert!(
        !focused(cx, handle, node(30)),
        "closed content cannot retain keyboard focus"
    );
    config.open_state = TooltipOpenState::Controlled(false);
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    focus(cx, handle, 2);
    frame(cx, handle).await;
    events(transport);
    focus(cx, handle, 29);
    frame(cx, handle).await;
    assert!(
        !visible(cx, handle),
        "controlled opening awaits the accepted value"
    );
    assert_eq!(events(transport), [true]);
    config.open_state = TooltipOpenState::Controlled(true);
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    frame(cx, handle).await;
    assert!(visible(cx, handle));
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(
        visible(cx, handle),
        "controlled dismissal remains an intent"
    );
    assert_eq!(events(transport), [false]);
    config.open_state = TooltipOpenState::Controlled(false);
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    frame(cx, handle).await;
    assert!(!visible(cx, handle));
    handle
        .update(cx, |view, window, cx| {
            assert_eq!(view.editors[&node(30)].focus_handle(cx), editor);
            assert_eq!(
                view.editors[&node(30)].snapshot(window, cx).text,
                "Retained é"
            );
        })
        .unwrap();
    // A trap retained inside hidden tooltip content must not capture the window.
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(31), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(31),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Splice(node(28), 1, 1, vec![node(31)]),
            Op::Splice(node(31), 0, 0, vec![node(30)]),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            assert!(view.focus.borrow().allows(node(2)));
            assert!(view.focus.borrow().handle(node(31)).is_none());
        })
        .unwrap();
    config.open_state = TooltipOpenState::Controlled(true);
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    frame(cx, handle).await;
    assert!(visible(cx, handle));
    assert!(
        focused(cx, handle, node(30)),
        "explicit content scope enters on opening"
    );
    config.open_state = TooltipOpenState::Controlled(false);
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            assert!(view.focus.borrow().allows(node(2)));
            assert!(view.focus.borrow().handle(node(31)).is_none());
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(31), 0, 1, vec![]),
            Op::Splice(node(28), 1, 1, vec![node(30)]),
            Op::Remove(node(31)),
        ],
    );
    frame(cx, handle).await;
    // Deferred tooltip content beyond a popover's bounds still belongs to it.
    config.open_state = TooltipOpenState::Controlled(true);
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(32), Kind::FocusScope, "".into(), Some(handler(32))),
            Op::SetFocusScope(
                node(32),
                FocusScopeConfig {
                    trap: false,
                    auto_focus: false,
                    restore_focus: false,
                },
            ),
            Op::SetOverlay(
                node(32),
                Some(OverlayConfig {
                    kind: OverlayKind::Popover,
                    label: "Tooltip parent".into(),
                    width: 180.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: true,
                }),
            ),
            Op::Splice(node(27), 0, 1, vec![node(32)]),
            Op::Splice(node(32), 0, 0, vec![node(28)]),
            Op::SetTooltip(node(28), config.clone()),
        ],
    );
    frame(cx, handle).await;
    let point = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(30)].bounds.center()
        })
        .unwrap();
    handle
        .update(cx, |view, _, _| {
            assert!(!view.probes.borrow()[&node(32)].bounds.contains(&point));
            assert!(view.focus.borrow().surface_contains(node(32), point));
        })
        .unwrap();
    transport.mailbox.lock().unwrap().drain(128);
    super::super::native_test::move_mouse(cx, handle, point, false);
    super::super::native_test::mouse(cx, handle, point, true);
    super::super::native_test::mouse(cx, handle, point, false);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(30)));
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::OverlayDismissed(..)))
    );
    config.open_state = TooltipOpenState::Controlled(false);
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(32), 0, 1, vec![]),
            Op::Splice(node(27), 0, 1, vec![node(28)]),
            Op::Remove(node(32)),
            Op::SetTooltip(node(28), config.clone()),
        ],
    );
    frame(cx, handle).await;
    // Timers are tested through actual pointer entry/exit, including the gap to
    // interactive content and cancellation before an opening deadline.
    focus(cx, handle, 2);
    frame(cx, handle).await;
    config.open_state = TooltipOpenState::Managed(false);
    config.show_delay_ns = 200_000_000;
    config.hide_delay_ns = 80_000_000;
    config.skip_delay_ns = 0;
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    let anchor = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(29)].bounds.center()
        })
        .unwrap();
    let outside = gpui::point(px(390.), px(270.));
    super::super::native_test::move_mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    events(transport);
    super::super::native_test::move_mouse(cx, handle, anchor, false);
    frame(cx, handle).await;
    assert!(!visible(cx, handle), "hover opening honors its delay");
    super::super::native_test::move_mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    cx.background_executor()
        .timer(std::time::Duration::from_millis(240))
        .await;
    frame(cx, handle).await;
    assert!(!visible(cx, handle), "leaving cancels pending opening");
    assert!(events(transport).is_empty());
    super::super::native_test::move_mouse(cx, handle, anchor, false);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(240))
        .await;
    frame(cx, handle).await;
    assert!(
        visible(cx, handle),
        "hover timer opens natively: {}",
        handle
            .update(cx, |view, _, _| view.tooltips[&node(28)].diagnostics())
            .unwrap()
    );
    assert_eq!(events(transport), [true]);
    let content = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(30)].bounds.center()
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, content, false);
    frame(cx, handle).await;
    let entered = handle
        .update(cx, |view, window, _| {
            format!(
                "target={content:?}, mouse={:?}, {}",
                window.mouse_position(),
                view.tooltips[&node(28)].diagnostics()
            )
        })
        .unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(120))
        .await;
    frame(cx, handle).await;
    assert!(
        visible(cx, handle),
        "hoverable content cancels delayed closure; entered: {entered}; final: {}",
        handle
            .update(cx, |view, window, _| format!(
                "mouse={:?}, {}",
                window.mouse_position(),
                view.tooltips[&node(28)].diagnostics()
            ))
            .unwrap()
    );
    super::super::native_test::move_mouse(cx, handle, outside, false);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(120))
        .await;
    frame(cx, handle).await;
    assert!(!visible(cx, handle));
    assert_eq!(events(transport), [false]);
    config.show_delay_ns = 10_000_000_000;
    config.skip_delay_ns = 1_000_000_000;
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    super::super::native_test::move_mouse(cx, handle, anchor, false);
    frame(cx, handle).await;
    assert!(
        visible(cx, handle),
        "recent closure skips the normal hover delay"
    );
    assert_eq!(events(transport), [true]);
    config.open_state = TooltipOpenState::Controlled(false);
    apply(cx, handle, vec![Op::SetTooltip(node(28), config.clone())]);
    super::super::native_test::move_mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    config.open_state = TooltipOpenState::Managed(false);
    config.show_delay_ns = 200_000_000;
    config.skip_delay_ns = 0;
    apply(cx, handle, vec![Op::SetTooltip(node(28), config)]);
    events(transport);
    super::super::native_test::move_mouse(cx, handle, anchor, false);
    frame(cx, handle).await;
    assert!(!visible(cx, handle));
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(30)),
            Op::Remove(node(29)),
            Op::Remove(node(28)),
            Op::Remove(node(27)),
            Op::Splice(node(0), 4, 1, vec![]),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            assert!(view.tooltips.is_empty());
            assert!(!view.editors.contains_key(&node(30)));
            assert!(view.focus.borrow().handle(node(28)).is_none());
        })
        .unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(240))
        .await;
    frame(cx, handle).await;
    assert!(
        events(transport).is_empty(),
        "unmount cancels delayed callbacks"
    );
    println!(
        "GPUIO_TOOLTIP_POINTER_OK: delayed hover, cancellation, interactive content, shared grace interval and unmount timer disposal"
    );
    println!(
        "GPUIO_TOOLTIP_KEYBOARD_OK: focus/Escape, managed/controlled opening, hidden editor focus denial, native identity retention and disposal"
    );
}
