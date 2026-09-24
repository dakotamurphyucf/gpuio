//! Real native capture, including redraws in the middle of a held gesture.
use super::*;
fn config() -> PointerConfig {
    PointerConfig {
        label: "Resize handle".into(),
        button: PointerButton::Left,
        disabled: false,
        prevent_default: true,
        stop_propagation: true,
    }
}
fn style(left: f64) -> Vec<Style> {
    vec![
        Style::State(3, vec![Field::Foreground(Color::Rgba(0xee6633ff))]),
        Style::Fields(vec![
            Field::Position(1),
            Field::Left(Length::Px(left)),
            Field::Top(Length::Px(30.)),
            Field::Foreground(Color::Rgba(0xffffffff)),
            Field::Width(Length::Px(180.)),
            Field::Height(Length::Px(100.)),
            Field::Background(Fill::Solid(Color::Rgba(0x4488ccff))),
        ]),
    ]
}
fn events(transport: &Transport) -> Vec<(NodeId, PointerSample)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::PointerEvent(_, node, _, _, sample) => Some((node, sample)),
            Event::Press(..) => panic!("unexpected native click during captured gesture"),
            _ => None,
        })
        .collect()
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    // Synchronous real GPUI draw avoids assuming the owner keeps their physical
    // mouse stationary between injected drag events and a redraw.
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
fn captured(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Option<gpui::HitboxId> {
    cx.update_window(handle.into(), |_, window, _| window.captured_hitbox())
        .unwrap()
}
fn down(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, point: gpui::Point<gpui::Pixels>) {
    super::super::native_test::move_mouse(cx, handle, point, false);
    super::super::native_test::mouse(cx, handle, point, true);
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(2)].focus, cx)
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(82),
                Kind::PointerArea,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(82, 1).unwrap()),
            ),
            Op::SetPointer(node(82), config()),
            Op::SetStyle(node(82), style(100.)),
            Op::Create(node(83), Kind::Text, "Drag".into(), None),
            Op::Splice(node(82), 0, 0, vec![node(83)]),
            Op::Splice(node(0), 4, 0, vec![node(82)]),
        ],
    );
    frame(cx, handle).await;
    let inside = gpui::point(px(140.), px(60.));
    let outside = gpui::point(px(390.), px(260.));
    down(cx, handle, inside);
    let first_hitbox = captured(cx, handle).expect("native region takes pointer capture");
    assert!(
        focused(cx, handle, node(2)),
        "prevent_default preserves existing keyboard focus"
    );
    super::super::native_test::move_mouse(cx, handle, outside, true);
    draw(cx, handle);
    assert_ne!(
        captured(cx, handle),
        Some(first_hitbox),
        "capture rebinds to this frame's hitbox"
    );
    assert!(captured(cx, handle).is_some());
    assert_color(cx, handle, node(82), 0xee6633ff);
    apply(cx, handle, vec![Op::SetStyle(node(82), style(120.))]);
    draw(cx, handle);
    super::super::native_test::move_mouse(cx, handle, outside, true);
    super::super::native_test::mouse(cx, handle, outside, false);
    assert!(captured(cx, handle).is_none());
    draw(cx, handle);
    assert_color(cx, handle, node(82), 0xffffffff);
    let samples = events(transport);
    assert_eq!(samples.first().unwrap().1.phase, PointerPhase::Started);
    assert_eq!(samples.last().unwrap().1.phase, PointerPhase::Released);
    assert!(
        samples
            .iter()
            .all(|(id, sample)| *id == node(82) && sample.gesture == samples[0].1.gesture)
    );
    assert_eq!(samples.last().unwrap().1.window_x, 390.);
    assert_eq!(
        samples.last().unwrap().1.local_x,
        270.,
        "local position uses current region geometry"
    );
    assert!(presses(transport).is_empty());
    down(cx, handle, inside);
    key(cx, handle, "escape");
    assert!(captured(cx, handle).is_none());
    let samples = events(transport);
    assert_eq!(
        samples.last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::Escape)
    );
    assert!(samples[0].1.gesture > 1);
    down(cx, handle, inside);
    apply(
        cx,
        handle,
        vec![Op::SetPointer(
            node(82),
            PointerConfig {
                disabled: true,
                ..config()
            },
        )],
    );
    assert!(captured(cx, handle).is_none());
    assert_eq!(
        events(transport).last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::Disabled)
    );
    draw(cx, handle);
    down(cx, handle, inside);
    assert!(captured(cx, handle).is_none());
    assert!(events(transport).is_empty());
    apply(cx, handle, vec![Op::SetPointer(node(82), config())]);
    draw(cx, handle);
    down(cx, handle, inside);
    let mut hidden = style(120.);
    hidden.push(Style::Fields(vec![Field::Visibility(1)]));
    apply(cx, handle, vec![Op::SetStyle(node(82), hidden)]);
    assert!(captured(cx, handle).is_none());
    assert_eq!(
        events(transport).last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::Hidden)
    );
    apply(cx, handle, vec![Op::SetStyle(node(82), style(120.))]);
    draw(cx, handle);
    down(cx, handle, inside);
    cx.update_window(handle.into(), |_, window, _| window.release_pointer())
        .unwrap();
    draw(cx, handle);
    assert_eq!(
        events(transport).last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::CaptureLost)
    );
    // Configuration changes end the old gesture, then the new button is usable.
    down(cx, handle, inside);
    apply(
        cx,
        handle,
        vec![Op::SetPointer(
            node(82),
            PointerConfig {
                button: PointerButton::Right,
                ..config()
            },
        )],
    );
    assert_eq!(
        events(transport).last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::Reconfigured)
    );
    draw(cx, handle);
    cx.update_window(handle.into(), |_, window, cx| {
        let modifiers = gpui::Modifiers {
            shift: true,
            control: true,
            alt: true,
            platform: true,
            function: true,
        };
        window.dispatch_event(
            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                button: gpui::MouseButton::Right,
                position: inside,
                modifiers,
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        assert!(window.default_prevented());
        window.dispatch_event(
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                button: gpui::MouseButton::Right,
                position: outside,
                modifiers,
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    let samples = events(transport);
    assert_eq!(samples.len(), 2);
    assert!(
        samples
            .iter()
            .all(|(_, sample)| sample.button == PointerButton::Right
                && sample.modifiers.shift
                && sample.modifiers.control
                && sample.modifiers.alt
                && sample.modifiers.command
                && sample.modifiers.function)
    );
    apply(cx, handle, vec![Op::SetPointer(node(82), config())]);
    draw(cx, handle);
    down(cx, handle, inside);
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                button: gpui::MouseButton::Right,
                position: inside,
                ..Default::default()
            }),
            cx,
        );
    })
    .unwrap();
    assert_eq!(
        events(transport).last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::CaptureLost)
    );
    assert!(captured(cx, handle).is_none());
    down(cx, handle, inside);
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(84), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(84),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Splice(node(0), 5, 0, vec![node(84)]),
        ],
    );
    assert_eq!(
        events(transport).last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::Blocked)
    );
    draw(cx, handle);
    down(cx, handle, inside);
    assert!(captured(cx, handle).is_none());
    assert!(events(transport).is_empty());
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 5, 1, vec![]), Op::Remove(node(84))],
    );
    draw(cx, handle);
    // Do not steal or release a capture assigned by another native component.
    cx.update_window(handle.into(), |_, window, _| {
        window.capture_pointer(first_hitbox)
    })
    .unwrap();
    down(cx, handle, inside);
    draw(cx, handle);
    assert_eq!(captured(cx, handle), Some(first_hitbox));
    assert!(events(transport).is_empty());
    cx.update_window(handle.into(), |_, window, _| window.release_pointer())
        .unwrap();
    // Activating a second native window cancels the first window's drag.
    down(cx, handle, inside);
    let other = cx
        .update(|cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(200.), px(120.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| gpui::Empty),
            )
        })
        .unwrap();
    other
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(100))
        .await;
    assert!(captured(cx, handle).is_none());
    assert_eq!(
        events(transport).last().unwrap().1.phase,
        PointerPhase::Cancelled(PointerCancel::WindowInactive)
    );
    other
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    frame(cx, handle).await;
    // Nested capture regions must select the inner owner once.
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(85),
                Kind::PointerArea,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(85, 1).unwrap()),
            ),
            Op::SetPointer(node(85), config()),
            Op::SetStyle(
                node(85),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(50.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x996633ff))),
                ])],
            ),
            Op::Splice(node(82), 0, 1, vec![node(85)]),
            Op::Remove(node(83)),
        ],
    );
    draw(cx, handle);
    down(cx, handle, inside);
    super::super::native_test::move_mouse(cx, handle, outside, true);
    super::super::native_test::mouse(cx, handle, outside, false);
    let samples = events(transport);
    assert!(samples.len() >= 2);
    assert!(
        samples.iter().all(|(id, _)| *id == node(85)),
        "innermost region owns captured gesture"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(86),
                Kind::Button,
                "Child action".into(),
                Some(gpuio_protocol::HandlerId::from_parts(86, 1).unwrap()),
            ),
            Op::SetControl(node(86), Control::Button(false)),
            Op::SetStyle(
                node(86),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(50.)),
                ])],
            ),
            Op::Splice(node(85), 0, 0, vec![node(86)]),
        ],
    );
    draw(cx, handle);
    down(cx, handle, inside);
    super::super::native_test::mouse(cx, handle, inside, false);
    assert!(
        captured(cx, handle).is_none(),
        "native child control handles input before ancestor capture"
    );
    let emitted = transport.mailbox.lock().unwrap().drain(128);
    assert_eq!(
        emitted
            .iter()
            .filter(|event| matches!(event,Event::Press(_,id,..) if *id==node(86)))
            .count(),
        1
    );
    assert!(
        !emitted
            .iter()
            .any(|event| matches!(event, Event::PointerEvent(..)))
    );
    apply(
        cx,
        handle,
        vec![Op::Splice(node(85), 0, 1, vec![]), Op::Remove(node(86))],
    );
    draw(cx, handle);
    down(cx, handle, inside);
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 4, 1, vec![]),
            Op::Splice(node(82), 0, 1, vec![]),
            Op::Remove(node(85)),
            Op::Remove(node(82)),
        ],
    );
    assert!(
        captured(cx, handle).is_none(),
        "unmount releases native capture synchronously"
    );
    let samples = events(transport);
    assert_eq!(
        samples.len(),
        1,
        "removed handler cannot receive terminal callbacks"
    );
    assert_eq!(samples[0].1.phase, PointerPhase::Started);
    frame(cx, handle).await;
    println!(
        "GPUIO_POINTER_NATIVE_OK: native capture outside bounds, redraw/reposition retention, release/Escape/disable/hide/loss/unmount, reconfiguration, modifiers, modal gating, foreign capture, window deactivation, captured styling, child control precedence and nested ownership"
    );
}
