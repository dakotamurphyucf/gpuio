//! Actual GPUI wheel dispatch through the mounted carousel and nested scrollers.
use super::*;
use gpui::{Pixels, Point, ScrollDelta, TouchPhase};
fn wheel(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    position: Point<Pixels>,
    delta: ScrollDelta,
    touch_phase: TouchPhase,
) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position,
                delta,
                touch_phase,
                modifiers: Default::default(),
            }),
            cx,
        );
    })
    .unwrap();
}
fn delta(x: f32, y: f32) -> ScrollDelta {
    ScrollDelta::Pixels(gpui::point(px(x), px(y)))
}
fn timer(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
    handle
        .update(cx, |v, _, _| v.carousels[&node(462)].borrow().wheel_timer())
        .unwrap()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    apply(
        cx,
        handle,
        vec![Op::SetCarousel(
            node(462),
            config(100, 1, Axis::Horizontal, Direction::Direct),
        )],
    );
    frame(cx, handle).await;
    let point = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(463)].bounds.center())
        .unwrap();
    super::super::super::native_test::move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    requests(transport);
    wheel(cx, handle, point, delta(0., -80.), TouchPhase::Started);
    wheel(cx, handle, point, delta(0., 0.), TouchPhase::Ended);
    assert!(
        requests(transport).is_empty(),
        "cross-axis input does not navigate"
    );
    wheel(cx, handle, point, delta(-20., 0.), TouchPhase::Started);
    assert!(timer(cx, handle));
    wheel(cx, handle, point, delta(-30., 0.), TouchPhase::Moved);
    assert!(
        requests(transport).is_empty(),
        "no application event for individual deltas"
    );
    wheel(cx, handle, point, delta(0., 0.), TouchPhase::Ended);
    assert_eq!(requests(transport), vec![Request::Next]);
    assert!(!timer(cx, handle));
    // Accept the wrapped selection while trackpad momentum is still arriving.
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(462), config(101, 0, Axis::Horizontal, Direction::Next)),
            Op::SetNavigationStack(node(463), presentation(0, 0)),
        ],
    );
    frame(cx, handle).await;
    for _ in 0..100 {
        wheel(cx, handle, point, delta(-40., 0.), TouchPhase::Moved);
    }
    assert!(
        requests(transport).is_empty(),
        "accepted selection does not reset momentum fence"
    );
    assert!(!timer(cx, handle));
    wheel(cx, handle, point, delta(80., 0.), TouchPhase::Started);
    wheel(cx, handle, point, delta(0., 0.), TouchPhase::Cancelled);
    assert!(!timer(cx, handle));
    assert!(requests(transport).is_empty());
    // Missing Ended receives one quiet-deadline fallback, with no idle polling.
    wheel(cx, handle, point, delta(40., 0.), TouchPhase::Started);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(70))
        .await;
    wheel(cx, handle, point, delta(40., 0.), TouchPhase::Moved);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "old deadline must follow the latest sample"
    );
    assert!(timer(cx, handle));
    cx.background_executor()
        .timer(std::time::Duration::from_millis(100))
        .await;
    assert_eq!(requests(transport), vec![Request::Previous]);
    assert!(!timer(cx, handle));
    // Ordinary wheel events are bounded into bursts too.
    wheel(
        cx,
        handle,
        point,
        ScrollDelta::Lines(gpui::point(-1., 0.)),
        TouchPhase::Started,
    );
    wheel(
        cx,
        handle,
        point,
        ScrollDelta::Lines(gpui::point(-1., 0.)),
        TouchPhase::Moved,
    );
    assert_eq!(requests(transport), vec![Request::Next]);
    assert!(!timer(cx, handle));

    apply(
        cx,
        handle,
        vec![Op::SetCarousel(
            node(462),
            config(102, 0, Axis::Vertical, Direction::Direct),
        )],
    );
    frame(cx, handle).await;
    wheel(cx, handle, point, delta(0., -60.), TouchPhase::Started);
    wheel(cx, handle, point, delta(0., 0.), TouchPhase::Ended);
    assert_eq!(requests(transport), vec![Request::Next]);
    wheel(cx, handle, point, delta(0., -60.), TouchPhase::Started);
    let mut disabled = config(103, 0, Axis::Vertical, Direction::Direct);
    disabled.disabled = true;
    apply(cx, handle, vec![Op::SetCarousel(node(462), disabled)]);
    frame(cx, handle).await;
    assert!(!timer(cx, handle));
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    assert!(
        requests(transport).is_empty(),
        "disabled owner cancels unfinished burst"
    );

    // A native scrolling page gets first refusal, even on the carousel axis.
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(
                node(462),
                config(104, 0, Axis::Horizontal, Direction::Direct),
            ),
            Op::Create(node(471), Kind::Container, "Child scroller".into(), None),
            Op::Create(node(472), Kind::Text, "Wide child".into(), None),
            Op::SetStyle(
                node(471),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(40.)),
                    Field::OverflowX(3),
                ])],
            ),
            Op::SetStyle(
                node(472),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(900.)),
                    Field::Height(Length::Px(30.)),
                ])],
            ),
            Op::Splice(node(471), 0, 0, vec![node(472)]),
            Op::Splice(node(464), 0, 0, vec![node(471)]),
        ],
    );
    frame(cx, handle).await;
    let nested = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(471)].bounds.center())
        .unwrap();
    super::super::super::native_test::move_mouse(cx, handle, nested, false);
    frame(cx, handle).await;
    wheel(cx, handle, nested, delta(-70., 0.), TouchPhase::Started);
    wheel(cx, handle, nested, delta(0., 0.), TouchPhase::Ended);
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, _, _| v.scrolls[&node(471)].handle.offset().x
                < px(0.))
            .unwrap(),
        "native child scrolled"
    );
    assert!(!timer(cx, handle));
    assert!(
        requests(transport).is_empty(),
        "nested scroll does not also navigate carousel"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(464), 0, 1, vec![]),
            Op::Remove(node(471)),
            Op::Remove(node(472)),
        ],
    );
    frame(cx, handle).await;
    // Reduced motion disables automation, not deliberate user navigation.
    cx.update(|cx| cx.set_reduce_motion(true));
    frame(cx, handle).await;
    super::super::super::native_test::move_mouse(cx, handle, point, false);
    wheel(cx, handle, point, delta(-60., 0.), TouchPhase::Started);
    wheel(cx, handle, point, delta(0., 0.), TouchPhase::Ended);
    assert_eq!(requests(transport), vec![Request::Next]);
    cx.update(|cx| cx.set_reduce_motion(false));
    frame(cx, handle).await;
    for field in [Field::Display(3), Field::PointerEvents(false)] {
        wheel(cx, handle, point, delta(-80., 0.), TouchPhase::Started);
        assert!(timer(cx, handle));
        apply(
            cx,
            handle,
            vec![Op::SetStyle(node(460), vec![Style::Fields(vec![field])])],
        );
        frame(cx, handle).await;
        assert!(!timer(cx, handle));
        cx.background_executor()
            .timer(std::time::Duration::from_millis(180))
            .await;
        assert!(
            requests(transport).is_empty(),
            "hidden or pointer-disabled owner cancels pending wheel"
        );
        apply(cx, handle, vec![Op::SetStyle(node(460), vec![])]);
        frame(cx, handle).await;
        super::super::super::native_test::move_mouse(cx, handle, point, false);
    }
    // Leave a deadline pending so the parent's teardown asserts actual disposal.
    super::super::super::native_test::move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    wheel(cx, handle, point, delta(-80., 0.), TouchPhase::Started);
    assert!(timer(cx, handle));
    println!(
        "GPUIO_CAROUSEL_WHEEL_OK: native horizontal/vertical, Ended/cancel/fallback, line wheel, momentum after accepted selection, disabled/hidden/pointer-policy cancellation, child-scroll precedence, reduced motion and extended deadline"
    );
}
