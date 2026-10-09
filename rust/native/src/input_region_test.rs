//! Actual GPUI dispatch through the mounted region and retained child editor.
use super::*;
use gpuio_protocol::{HandlerId, input};
fn config() -> input::Config {
    use input::Kind::*;
    input::Config {
        label: "Input observations".into(),
        disabled: false,
        focus: input::Focus::Tab,
        subscriptions: [
            Click,
            AuxiliaryClick,
            MouseDown,
            MouseUp,
            MouseMove,
            MouseEnter,
            MouseLeave,
            MouseDownOutside,
            KeyDown,
            KeyUp,
            Focus,
            Blur,
            Scroll,
        ]
        .into_iter()
        .map(|kind| input::Subscription {
            kind,
            phase: if matches!(kind, KeyDown | KeyUp) {
                input::Phase::Capture
            } else {
                input::Phase::Bubble
            },
            policy: input::Policy::Observe,
        })
        .collect(),
    }
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(5, generation).unwrap()
}
fn styles(hidden: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(20.)),
        Field::Top(Length::Px(20.)),
        Field::Width(Length::Px(360.)),
        Field::Height(Length::Px(230.)),
        Field::Display(if hidden { 3 } else { 1 }),
        Field::Background(Fill::Solid(Color::Rgba(0x203448ff))),
    ])]
}
fn routed(transport: &Transport) -> Vec<(NodeId, input::Event)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::InputObserved(_, node, _, _, event) => Some((node, event)),
            _ => None,
        })
        .collect()
}
fn observations(transport: &Transport) -> Vec<input::Event> {
    routed(transport)
        .into_iter()
        .map(|(_, event)| event)
        .collect()
}
fn input(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, event: gpui::PlatformInput) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(event, cx);
    })
    .unwrap();
}
fn mouse(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    position: gpui::Point<gpui::Pixels>,
    button: gpui::MouseButton,
    down: bool,
) {
    let modifiers = gpui::Modifiers {
        shift: true,
        ..Default::default()
    };
    input(
        cx,
        handle,
        if down {
            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                position,
                button,
                modifiers,
                click_count: 2,
                first_mouse: false,
            })
        } else {
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position,
                button,
                modifiers,
                click_count: 2,
            })
        },
    );
}
fn move_mouse(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, p: gpui::Point<gpui::Pixels>) {
    super::super::native_test::move_mouse(cx, handle, p, false);
}
fn changed(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    generation: i64,
    config: input::Config,
) {
    apply(
        cx,
        handle,
        vec![
            Op::Bind(node(5), Some(handler(generation))),
            Op::SetInputRegion(node(5), config),
        ],
    );
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(5), Kind::InputRegion, "".into(), Some(handler(1))),
            Op::SetInputRegion(node(5), config()),
            Op::SetStyle(node(5), styles(false)),
            Op::SetStyle(
                node(4),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(320.)),
                    Field::Height(Length::Px(40.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::Create(node(6), Kind::Text, "Input surface".into(), None),
            Op::Splice(node(0), 0, 4, vec![]),
            Op::Splice(node(5), 0, 0, vec![node(4), node(6)]),
            Op::Splice(node(0), 0, 0, vec![node(5)]),
            Op::Remove(node(1)),
            Op::Remove(node(2)),
            Op::Remove(node(3)),
        ],
    );
    frame(cx, handle).await;
    let (retained_editor, region) = handle
        .update(cx, |view, _, _| {
            (
                view.editors[&node(4)].liveness_probe(),
                Rc::downgrade(&view.input_regions[&node(5)]),
            )
        })
        .unwrap();
    observations(transport);
    let inside = gpui::point(px(100.), px(180.));
    let outside = gpui::point(px(390.), px(260.));
    let editor_bounds = handle
        .update(cx, |view, _, cx| view.editors[&node(4)].input_bounds(cx))
        .unwrap();
    assert!(
        !editor_bounds.contains(&inside),
        "fixture blank point intersects editor: {editor_bounds:?}"
    );
    move_mouse(cx, handle, outside);
    observations(transport);
    move_mouse(cx, handle, inside);
    assert!(
        observations(transport)
            .iter()
            .any(|event| matches!(event, input::Event::MouseEnter))
    );
    for button in [
        gpui::MouseButton::Left,
        gpui::MouseButton::Right,
        gpui::MouseButton::Middle,
        gpui::MouseButton::Navigate(gpui::NavigationDirection::Back),
        gpui::MouseButton::Navigate(gpui::NavigationDirection::Forward),
    ] {
        mouse(cx, handle, inside, button, true);
        mouse(cx, handle, inside, button, false);
        let events = observations(transport);
        let mouse_events: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                input::Event::MouseDown(m)
                | input::Event::MouseUp(m)
                | input::Event::Click(m)
                | input::Event::AuxiliaryClick(m) => Some(m),
                _ => None,
            })
            .collect();
        assert_eq!(
            mouse_events.len(),
            3,
            "matched raw mouse edges/click: {events:?}"
        );
        assert!(mouse_events.iter().all(|m| m.click_count == 2
            && m.location.modifiers.shift
            && m.location.local.x == 80.));
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, input::Event::Click(_)))
                .count(),
            usize::from(button == gpui::MouseButton::Left)
        );
    }
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    move_mouse(cx, handle, outside);
    mouse(cx, handle, outside, gpui::MouseButton::Left, false);
    mouse(cx, handle, outside, gpui::MouseButton::Left, true);
    mouse(cx, handle, outside, gpui::MouseButton::Left, false);
    move_mouse(cx, handle, inside);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    let events = observations(transport);
    assert!(events.iter().any(|e| matches!(e, input::Event::MouseLeave)));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, input::Event::MouseDownOutside(_)))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, input::Event::Click(_) | input::Event::AuxiliaryClick(_)))
    );
    // Some platforms send MouseExited without a final outside MouseMove. One
    // leave, no stale re-entry on repaint, and no click after returning with up.
    move_mouse(cx, handle, inside);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    observations(transport);
    input(
        cx,
        handle,
        gpui::PlatformInput::MouseExited(gpui::MouseExitEvent {
            position: outside,
            pressed_button: Some(gpui::MouseButton::Left),
            modifiers: Default::default(),
        }),
    );
    let events = observations(transport);
    assert_eq!(events, vec![input::Event::MouseLeave], "window exit edge");
    draw(cx, handle);
    frame(cx, handle).await;
    let events = observations(transport);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, input::Event::MouseEnter | input::Event::MouseLeave)),
        "repaint must not infer hover transitions: {events:?}"
    );
    assert!(
        !handle
            .update(cx, |view, _, _| view.input_pointer_inside.get())
            .unwrap()
    );
    move_mouse(cx, handle, inside);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    let events = observations(transport);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, input::Event::MouseEnter))
            .count(),
        1
    );
    assert!(
        !events.iter().any(|e| matches!(e, input::Event::Click(_))),
        "exit cancels held click"
    );
    eprintln!("GPUIO_INPUT_REGION_WINDOW_EXIT_OK: leave, repaint, re-entry and cancelled click");
    for phase in [
        gpui::TouchPhase::Started,
        gpui::TouchPhase::Moved,
        gpui::TouchPhase::Ended,
        gpui::TouchPhase::Cancelled,
    ] {
        for delta in [
            gpui::ScrollDelta::Pixels(gpui::point(px(0.5), px(-4.))),
            gpui::ScrollDelta::Lines(gpui::point(-1., 0.)),
        ] {
            input(
                cx,
                handle,
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position: inside,
                    delta,
                    touch_phase: phase,
                    modifiers: Default::default(),
                }),
            );
        }
    }
    let events = observations(transport);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, input::Event::Scroll(_)))
            .count(),
        8,
        "wheel boundaries/deltas: {events:?}"
    );
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |view, window, _| view.input_regions[&node(5)]
                .borrow()
                .focus
                .is_focused(window))
            .unwrap(),
        "blank region click focuses the region"
    );
    key(cx, handle, "shift-a");
    let events = observations(transport);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, input::Event::KeyDown(k, false) if k.modifiers.shift)),
        "direct keys: {events:?}"
    );
    assert!(events.iter().any(|e| matches!(e, input::Event::KeyUp(_))));
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |view, window, cx| view.editors[&node(4)]
                .focus_handle(cx)
                .is_focused(window))
            .unwrap(),
        "Tab enters child editor after region"
    );
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |view, window, _| view.input_regions[&node(5)]
                .borrow()
                .focus
                .is_focused(window))
            .unwrap(),
        "Shift-Tab returns to region"
    );
    // Focused native descendants receive normal editing while capture observes keys.
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx);
        })
        .unwrap();
    frame(cx, handle).await;
    observations(transport);
    super::super::editor_test::native_text(cx, handle, "hello", false);
    key(cx, handle, "left");
    let events = observations(transport);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, input::Event::KeyDown(k, _) if k.key == "left")),
        "native binding consumes Left before raw listeners: {events:?}"
    );
    key(cx, handle, "f13");
    let events = observations(transport);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, input::Event::KeyDown(k, _) if k.key == "f13")),
        "unbound child key reaches capture listener: {events:?}"
    );
    let snapshot = handle
        .update(cx, |view, window, cx| {
            view.editors[&node(4)].snapshot(window, cx)
        })
        .unwrap();
    assert_eq!(snapshot.text, "hello");
    assert_eq!(snapshot.selection.head, 4);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    let mut next = config();
    next.label = "Renamed".into();
    changed(cx, handle, 2, next.clone());
    draw(cx, handle);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(
        !observations(transport)
            .iter()
            .any(|e| matches!(e, input::Event::Click(_)))
    );
    assert!(retained_editor());
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    apply(cx, handle, vec![Op::SetStyle(node(5), styles(true))]);
    draw(cx, handle);
    apply(cx, handle, vec![Op::SetStyle(node(5), styles(false))]);
    draw(cx, handle);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(
        !observations(transport)
            .iter()
            .any(|e| matches!(e, input::Event::Click(_)))
    );
    next.disabled = true;
    changed(cx, handle, 3, next.clone());
    draw(cx, handle);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(observations(transport).is_empty());
    next.disabled = false;
    changed(cx, handle, 4, next);
    draw(cx, handle);
    nested(cx, handle, transport).await;
    eligibility(cx, handle, transport).await;
    occlusion_and_capture(cx, handle, transport).await;
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(5), 0, 2, vec![]),
            Op::Splice(node(0), 0, 1, vec![node(4)]),
            Op::Remove(node(6)),
            Op::Remove(node(5)),
        ],
    );
    frame(cx, handle).await;
    draw(cx, handle);
    assert!(
        region.upgrade().is_none(),
        "region callbacks/subscriptions survived disposal"
    );
    assert!(
        retained_editor(),
        "child editor identity survives reparenting"
    );
    handle
        .update(cx, |view, _, _| {
            view.session.borrow_mut().close(view.id).unwrap();
            assert_eq!(view.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    println!(
        "GPUIO_INPUT_REGION_NATIVE_OK: pointer/auxiliary/hover/outside, units/phases, focus/key capture, native editor retention, reconfigure/hidden/disabled fencing and disposal"
    );
}

async fn nested(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut child = config();
    child.label = "Nested input".into();
    child.focus = input::Focus::Click;
    let child_handler = |generation| HandlerId::from_parts(7, generation).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(7),
                Kind::InputRegion,
                "".into(),
                Some(child_handler(1)),
            ),
            Op::SetInputRegion(node(7), child.clone()),
            Op::SetStyle(
                node(7),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(200.)),
                    Field::Top(Length::Px(100.)),
                    Field::Width(Length::Px(100.)),
                    Field::Height(Length::Px(80.)),
                ])],
            ),
            Op::Splice(node(5), 2, 0, vec![node(7)]),
        ],
    );
    frame(cx, handle).await;
    let inside = gpui::point(px(250.), px(160.));
    for (generation, phase, policy, expected) in [
        (
            5,
            input::Phase::Capture,
            input::Policy::Observe,
            vec![node(5), node(7)],
        ),
        (
            6,
            input::Phase::Bubble,
            input::Policy::Observe,
            vec![node(7), node(5)],
        ),
        (
            7,
            input::Phase::Capture,
            input::Policy::PreventAndStop,
            vec![node(5)],
        ),
    ] {
        let mut parent = config();
        let subscription = parent
            .subscriptions
            .iter_mut()
            .find(|s| s.kind == input::Kind::MouseDown)
            .unwrap();
        subscription.phase = phase;
        subscription.policy = policy;
        changed(cx, handle, generation, parent);
        draw(cx, handle);
        handle
            .update(cx, |view, window, cx| {
                window.focus(&view.input_regions[&node(5)].borrow().focus, cx)
            })
            .unwrap();
        draw(cx, handle);
        move_mouse(cx, handle, inside);
        observations(transport);
        mouse(cx, handle, inside, gpui::MouseButton::Left, true);
        mouse(cx, handle, inside, gpui::MouseButton::Left, false);
        let events = routed(transport);
        let owners: Vec<_> = events
            .iter()
            .filter_map(|(owner, event)| {
                matches!(event, input::Event::MouseDown(_)).then_some(*owner)
            })
            .collect();
        assert_eq!(owners, expected, "nested {phase:?}/{policy:?}: {events:?}");
        if policy == input::Policy::PreventAndStop {
            assert!(
                handle
                    .update(cx, |view, window, _| view.input_regions[&node(5)]
                        .borrow()
                        .focus
                        .is_focused(window))
                    .unwrap(),
                "capture prevention keeps focus from the child"
            );
        }
    }
    changed(cx, handle, 8, config());
    child
        .subscriptions
        .iter_mut()
        .find(|s| s.kind == input::Kind::MouseDown)
        .unwrap()
        .policy = input::Policy::StopPropagation;
    apply(
        cx,
        handle,
        vec![
            Op::Bind(node(7), Some(child_handler(2))),
            Op::SetInputRegion(node(7), child),
        ],
    );
    draw(cx, handle);
    move_mouse(cx, handle, inside);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    let owners: Vec<_> = routed(transport)
        .into_iter()
        .filter_map(|(owner, event)| matches!(event, input::Event::MouseDown(_)).then_some(owner))
        .collect();
    assert_eq!(
        owners,
        vec![node(7)],
        "child bubble policy stops parent observation"
    );
    apply(
        cx,
        handle,
        vec![Op::Splice(node(5), 2, 1, vec![]), Op::Remove(node(7))],
    );
    frame(cx, handle).await;
    println!(
        "GPUIO_INPUT_REGION_NESTED_OK: parent capture, child-first bubble, native capture prevention and child stop propagation"
    );
}

async fn eligibility(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let inside = gpui::point(px(100.), px(180.));
    // Pointer inheritance is independent of keyboard focus and native editing.
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![Field::PointerEvents(false)])],
        )],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.input_regions[&node(5)].borrow().focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    observations(transport);
    move_mouse(cx, handle, inside);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(
        observations(transport).is_empty(),
        "inherited pointer disable"
    );
    key(cx, handle, "f13");
    assert!(
        observations(transport)
            .iter()
            .any(|e| matches!(e, input::Event::KeyDown(k, _) if k.key == "f13")),
        "pointer disable must preserve keyboard scope"
    );
    let mut override_style = styles(false);
    override_style.push(Style::Fields(vec![Field::PointerEvents(true)]));
    apply(cx, handle, vec![Op::SetStyle(node(5), override_style)]);
    draw(cx, handle);
    move_mouse(cx, handle, inside);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(
        observations(transport)
            .iter()
            .any(|e| matches!(e, input::Event::Click(_))),
        "nearest pointer override"
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(node(5), styles(false)),
            Op::SetStyle(
                node(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(180.)),
                    Field::Height(Length::Px(140.)),
                    Field::OverflowX(1),
                    Field::OverflowY(1),
                ])],
            ),
        ],
    );
    draw(cx, handle);
    move_mouse(cx, handle, inside);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    let events = observations(transport);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, input::Event::MouseDownOutside(_))),
        "clipped boundary: {events:?}"
    );
    assert!(
        !events.iter().any(|e| matches!(
            e,
            input::Event::MouseDown(_) | input::Event::MouseUp(_) | input::Event::Click(_)
        )),
        "clipped input must not reach surface: {events:?}"
    );
    apply(cx, handle, vec![Op::SetStyle(node(0), vec![])]);
    draw(cx, handle);
    move_mouse(cx, handle, inside);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    // A newly mounted real modal traps input outside its subtree and cancels the
    // held press, including after the modal closes and focus is restored.
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(8), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(8),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Splice(node(0), 1, 0, vec![node(8)]),
        ],
    );
    frame(cx, handle).await;
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    key(cx, handle, "f13");
    assert!(
        observations(transport).is_empty(),
        "modal fences background region"
    );
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 1, 1, vec![]), Op::Remove(node(8))],
    );
    frame(cx, handle).await;
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(
        !observations(transport)
            .iter()
            .any(|e| matches!(e, input::Event::Click(_))),
        "modal retirement must not revive held click"
    );
    // Real native activation change: the inactive window must reject input and
    // must not complete its old press after regaining focus.
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
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
    assert!(
        !handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
    );
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    key(cx, handle, "f13");
    assert!(
        observations(transport).is_empty(),
        "inactive window rejects input"
    );
    other
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    frame(cx, handle).await;
    move_mouse(cx, handle, inside);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(
        !observations(transport)
            .iter()
            .any(|e| matches!(e, input::Event::Click(_))),
        "reactivation must not revive held click"
    );
    // Repeat and direct blur have their own native observations, not inferred
    // from text insertion or focus-within.
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.input_regions[&node(5)].borrow().focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    observations(transport);
    input(
        cx,
        handle,
        gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("f13").unwrap(),
            is_held: true,
            prefer_character_input: false,
        }),
    );
    let events = observations(transport);
    let keys: Vec<_> = events
        .iter()
        .filter(|event| matches!(event, input::Event::KeyDown(..)))
        .collect();
    assert!(
        matches!(keys.as_slice(), [input::Event::KeyDown(_, true)]),
        "one native repeat observation: {events:?}"
    );
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    let events = observations(transport);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, input::Event::Blur))
            .count(),
        1,
        "direct blur: {events:?}"
    );
    assert!(
        !events.iter().any(|e| matches!(e, input::Event::Focus)),
        "child focus is not region focus"
    );
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "かな", true);
        let marked = handle
            .update(cx, |view, window, cx| {
                view.editors[&node(4)].snapshot(window, cx)
            })
            .unwrap();
        assert!(marked.composition.is_some());
        // Changing the observation binding must not replace the editor or IME.
        let mut config = config();
        config.label = "IME retained".into();
        changed(cx, handle, 9, config);
        draw(cx, handle);
        let retained = handle
            .update(cx, |view, window, cx| {
                view.editors[&node(4)].snapshot(window, cx)
            })
            .unwrap();
        assert_eq!(marked.composition, retained.composition);
        super::super::editor_test::native_text(cx, handle, "仮名", false);
        let committed = handle
            .update(cx, |view, window, cx| {
                view.editors[&node(4)].snapshot(window, cx)
            })
            .unwrap();
        assert!(committed.composition.is_none());
        assert!(committed.text.contains("仮名"));
        assert!(
            !observations(transport)
                .iter()
                .any(|e| matches!(e, input::Event::KeyDown(..))),
            "IME text must not fabricate raw keys"
        );
        println!(
            "GPUIO_INPUT_REGION_IME_OK: native marked text survives binding update and commits"
        );
    }
    println!(
        "GPUIO_INPUT_REGION_ELIGIBILITY_OK: pointer inheritance/override, clipping, modal, activation, repeat and direct focus edges"
    );
}

async fn occlusion_and_capture(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let inside = gpui::point(px(100.), px(180.));
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(9),
                Kind::Button,
                "Occluding button".into(),
                Some(HandlerId::from_parts(9, 1).unwrap()),
            ),
            Op::SetControl(node(9), Control::Button(false)),
            Op::SetStyle(
                node(9),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(60.)),
                    Field::Top(Length::Px(160.)),
                    Field::Width(Length::Px(100.)),
                    Field::Height(Length::Px(40.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x405060ff))),
                    Field::PointerOcclusion(1),
                ])],
            ),
            Op::Splice(node(0), 1, 0, vec![node(9)]),
        ],
    );
    frame(cx, handle).await;
    move_mouse(cx, handle, inside);
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Press(_, id, ..) if *id == node(9))),
        "actual occluding button activated: {events:?}"
    );
    assert!(
        !events.iter().any(|e| matches!(
            e,
            Event::InputObserved(
                _,
                _,
                _,
                _,
                input::Event::MouseDown(_) | input::Event::MouseUp(_) | input::Event::Click(_)
            )
        )),
        "covered region must not observe pointer edges: {events:?}"
    );
    for mode in [0, 1, 2] {
        let mut style = handle
            .update(cx, |view, _, _| {
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .get(node(9))
                    .unwrap()
                    .style
                    .to_vec()
            })
            .unwrap();
        style.push(Style::Fields(vec![Field::PointerOcclusion(mode)]));
        apply(cx, handle, vec![Op::SetStyle(node(9), style)]);
        draw(cx, handle);
        move_mouse(cx, handle, inside);
        observations(transport);
        mouse(cx, handle, inside, gpui::MouseButton::Left, true);
        mouse(cx, handle, inside, gpui::MouseButton::Left, false);
        let events = observations(transport);
        assert_eq!(
            events
                .iter()
                .any(|e| matches!(e, input::Event::MouseDown(_))),
            mode == 0,
            "pointer occlusion mode {mode}: {events:?}"
        );
        input(
            cx,
            handle,
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: inside,
                delta: gpui::ScrollDelta::Lines(gpui::point(0., -1.)),
                touch_phase: gpui::TouchPhase::Moved,
                modifiers: Default::default(),
            }),
        );
        let events = observations(transport);
        assert_eq!(
            events.iter().any(|e| matches!(e, input::Event::Scroll(_))),
            mode != 2,
            "wheel occlusion mode {mode}: {events:?}"
        );
    }
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 1, 1, vec![]),
            Op::Remove(node(9)),
            Op::Create(
                node(10),
                Kind::PointerArea,
                "".into(),
                Some(HandlerId::from_parts(10, 1).unwrap()),
            ),
            Op::SetPointer(
                node(10),
                PointerConfig {
                    label: "Foreign capture".into(),
                    button: PointerButton::Left,
                    disabled: false,
                    prevent_default: true,
                    stop_propagation: true,
                },
            ),
            Op::SetStyle(
                node(10),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(300.)),
                    Field::Top(Length::Px(250.)),
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(25.)),
                ])],
            ),
            Op::Splice(node(0), 1, 0, vec![node(10)]),
        ],
    );
    frame(cx, handle).await;
    let foreign = gpui::point(px(340.), px(260.));
    move_mouse(cx, handle, foreign);
    mouse(cx, handle, foreign, gpui::MouseButton::Left, true);
    let hitbox = handle
        .update(cx, |_, window, _| {
            window
                .captured_hitbox()
                .expect("real foreign gesture captured")
        })
        .unwrap();
    mouse(cx, handle, foreign, gpui::MouseButton::Left, false);
    move_mouse(cx, handle, inside);
    mouse(cx, handle, inside, gpui::MouseButton::Left, true);
    handle
        .update(cx, |_, window, _| window.capture_pointer(hitbox))
        .unwrap();
    draw(cx, handle);
    assert_eq!(
        handle
            .update(cx, |_, window, _| window.captured_hitbox())
            .unwrap(),
        Some(hitbox),
        "observer must not steal foreign capture"
    );
    observations(transport);
    mouse(cx, handle, inside, gpui::MouseButton::Left, false);
    assert!(
        !observations(transport)
            .iter()
            .any(|e| matches!(e, input::Event::Click(_))),
        "foreign capture cancels pending click"
    );
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 1, 1, vec![]), Op::Remove(node(10))],
    );
    frame(cx, handle).await;
    println!(
        "GPUIO_INPUT_REGION_OCCLUSION_OK: sibling native button priority and foreign capture ownership/cancellation"
    );
}
