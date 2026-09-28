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
