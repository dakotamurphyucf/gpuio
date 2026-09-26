//! Mounted pointer dispatch and GPU preview; selection is application-owned.
use super::super::super::native_test::{mouse, move_mouse};
use super::*;
use gpui::{Pixels, Point};
fn bounds(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: i64) -> Bounds<Pixels> {
    handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(id)].bounds)
        .unwrap()
}
fn captured(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
    handle
        .update(cx, |_, w, _| w.captured_hitbox().is_some())
        .unwrap()
}
fn previewing(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
    handle
        .update(cx, |v, _, _| v.navigation[&node(463)].borrow().previewing())
        .unwrap()
}
fn origin(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> f32 {
    handle
        .update(cx, |v, _, _| {
            v.navigation[&node(463)].borrow().drag_origin()
        })
        .unwrap()
}
async fn start(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    point: Point<Pixels>,
    delta: Point<Pixels>,
) -> Point<Pixels> {
    move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    mouse(cx, handle, point, true);
    assert!(
        !captured(cx, handle),
        "pointer stays unclaimed until axis lock"
    );
    let end = point + delta;
    move_mouse(cx, handle, end, true);
    frame(cx, handle).await;
    assert!(captured(cx, handle), "locked drag captures the pointer");
    assert!(previewing(cx, handle));
    end
}
#[cfg(feature = "native-image-tests")]
fn colors(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    vertical: bool,
    expected: [[u8; 4]; 2],
) {
    handle
        .update(cx, |v, w, _| {
            let image = w.render_to_image().unwrap();
            let bounds = v.probes.borrow()[&node(463)].bounds;
            let scale = w.scale_factor();
            let samples = if vertical {
                [
                    gpui::point(bounds.right() - px(4.), bounds.top() + px(4.)),
                    gpui::point(bounds.right() - px(4.), bounds.bottom() - px(4.)),
                ]
            } else {
                [
                    gpui::point(bounds.left() + px(4.), bounds.bottom() - px(4.)),
                    gpui::point(bounds.right() - px(4.), bounds.bottom() - px(4.)),
                ]
            };
            for (point, expected) in samples.into_iter().zip(expected) {
                assert_eq!(
                    image
                        .get_pixel(
                            (f32::from(point.x) * scale) as u32,
                            (f32::from(point.y) * scale) as u32
                        )
                        .0,
                    expected,
                    "painted drag layers"
                );
            }
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
            Op::SetCarousel(node(462), config(6, 0, Axis::Horizontal, Direction::Direct)),
            Op::SetNavigationStack(node(463), presentation(0, 200)),
        ],
    );
    // Settle the pre-existing automatic transition before the first gesture.
    cx.background_executor()
        .timer(std::time::Duration::from_millis(240))
        .await;
    frame(cx, handle).await;
    let point = bounds(cx, handle, 463).center();
    requests(transport);
    move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    mouse(cx, handle, point, true);
    move_mouse(cx, handle, point + gpui::point(px(2.), px(40.)), true);
    mouse(cx, handle, point + gpui::point(px(2.), px(40.)), false);
    frame(cx, handle).await;
    assert!(!captured(cx, handle));
    assert!(!previewing(cx, handle));
    assert!(
        requests(transport).is_empty(),
        "cross-axis gesture is rejected"
    );

    let end = start(cx, handle, point, gpui::point(px(-100.), px(0.))).await;
    assert!((origin(cx, handle) + 100. / 320.).abs() < 0.001);
    assert!(
        requests(transport).is_empty(),
        "motion never calls application reducer"
    );
    handle
        .update(cx, |v, _, _| {
            let session = v.session.borrow();
            assert_eq!(
                session
                    .tree(v.id)
                    .unwrap()
                    .get(node(462))
                    .unwrap()
                    .carousel
                    .as_ref()
                    .unwrap()
                    .selected,
                Some(0)
            );
            assert!(
                !v.focus.borrow().visible(node(470)),
                "neighbor editor stays inert"
            );
        })
        .unwrap();
    #[cfg(feature = "native-image-tests")]
    colors(
        cx,
        handle,
        false,
        [[0xe1, 0x35, 0x99, 0xff], [0x25, 0x63, 0xeb, 0xff]],
    );
    mouse(cx, handle, end, false);
    assert!(!captured(cx, handle));
    assert_eq!(requests(transport), vec![Request::Next]);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(250))
        .await;
    frame(cx, handle).await;
    assert_eq!(
        origin(cx, handle),
        0.,
        "ignored request snaps back to accepted selection"
    );
    #[cfg(feature = "native-image-tests")]
    colors(cx, handle, false, [[0xe1, 0x35, 0x99, 0xff]; 2]);

    // Accept while the snap is beginning, then grab the still-running transition.
    let end = start(cx, handle, point, gpui::point(px(-100.), px(0.))).await;
    mouse(cx, handle, end, false);
    assert_eq!(requests(transport), vec![Request::Next]);
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(462), config(7, 1, Axis::Horizontal, Direction::Next)),
            Op::SetNavigationStack(node(463), presentation(1, 2000)),
        ],
    );
    assert!(
        (origin(cx, handle) - (1. - 100. / 320.)).abs() < 0.001,
        "acceptance uses painted neighbor position"
    );
    frame(cx, handle).await;
    move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    mouse(cx, handle, point, true);
    let painted = origin(cx, handle);
    move_mouse(cx, handle, point + gpui::point(px(-32.), px(0.)), true);
    frame(cx, handle).await;
    assert!(captured(cx, handle));
    assert!(
        (origin(cx, handle) - (painted - 0.1)).abs() < 0.001,
        "interruption starts at last accepted paint"
    );
    key(cx, handle, "escape");
    assert!(!captured(cx, handle));
    mouse(cx, handle, point, false);
    assert!(
        requests(transport).is_empty(),
        "Escape cancels without selection"
    );
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(463), presentation(1, 0))],
    );
    frame(cx, handle).await;

    // Dragging the native editor must neither claim its pointer nor navigate.
    let editor = bounds(cx, handle, 470).center();
    move_mouse(cx, handle, editor, false);
    frame(cx, handle).await;
    mouse(cx, handle, editor, true);
    move_mouse(cx, handle, editor + gpui::point(px(-90.), px(0.)), true);
    mouse(cx, handle, editor, false);
    frame(cx, handle).await;
    assert!(!previewing(cx, handle));
    assert!(
        requests(transport).is_empty(),
        "editor drag remains native to editor"
    );
    let button = bounds(cx, handle, 467).center();
    move_mouse(cx, handle, button, false);
    frame(cx, handle).await;
    mouse(cx, handle, button, true);
    mouse(cx, handle, button, false);
    frame(cx, handle).await;
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Press(_, id, _, _) if *id == node(467)))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::CarouselRequested(..)))
    );

    // Deliberate pointer input remains available with reduced motion.
    cx.update(|cx| cx.set_reduce_motion(true));
    frame(cx, handle).await;
    let end = start(cx, handle, point, gpui::point(px(-90.), px(0.))).await;
    mouse(cx, handle, end, false);
    assert_eq!(
        requests(transport),
        vec![Request::Next],
        "looping at last page"
    );
    frame(cx, handle).await;
    assert_eq!(origin(cx, handle), 0.);
    cx.update(|cx| cx.set_reduce_motion(false));
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(462), config(8, 0, Axis::Vertical, Direction::Previous)),
            Op::SetNavigationStack(node(463), presentation(0, 200)),
        ],
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(240))
        .await;
    frame(cx, handle).await;
    let end = start(cx, handle, point, gpui::point(px(0.), px(-60.))).await;
    #[cfg(feature = "native-image-tests")]
    colors(
        cx,
        handle,
        true,
        [[0xe1, 0x35, 0x99, 0xff], [0x25, 0x63, 0xeb, 0xff]],
    );
    // Capture keeps delivering after the pointer leaves the viewport.
    let outside = end + gpui::point(px(0.), px(-300.));
    move_mouse(cx, handle, outside, true);
    frame(cx, handle).await;
    assert!(captured(cx, handle));
    mouse(cx, handle, outside, false);
    assert_eq!(requests(transport), vec![Request::Next]);
    assert!(!captured(cx, handle));
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(463), presentation(0, 0))],
    );
    frame(cx, handle).await;

    // Policy changes and geometry changes cancel an already captured gesture.
    for field in [Field::Display(3), Field::PointerEvents(false)] {
        start(cx, handle, point, gpui::point(px(0.), px(-60.))).await;
        apply(
            cx,
            handle,
            vec![Op::SetStyle(node(460), vec![Style::Fields(vec![field])])],
        );
        frame(cx, handle).await;
        assert!(!captured(cx, handle));
        mouse(cx, handle, point, false);
        assert!(requests(transport).is_empty());
        apply(cx, handle, vec![Op::SetStyle(node(460), vec![])]);
        frame(cx, handle).await;
    }
    start(cx, handle, point, gpui::point(px(0.), px(-60.))).await;
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(463),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(300.)),
                Field::Height(Length::Px(180.)),
            ])],
        )],
    );
    frame(cx, handle).await;
    assert!(!captured(cx, handle), "resize cancels owned capture");
    mouse(cx, handle, point, false);
    assert!(requests(transport).is_empty());
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(463),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(200.)),
            ])],
        )],
    );
    frame(cx, handle).await;

    start(cx, handle, point, gpui::point(px(0.), px(-60.))).await;
    handle
        .update(cx, |_, window, _| window.release_pointer())
        .unwrap();
    move_mouse(cx, handle, point + gpui::point(px(0.), px(-65.)), true);
    frame(cx, handle).await;
    assert!(!previewing(cx, handle), "lost capture cancels presentation");
    mouse(cx, handle, point, false);
    assert!(requests(transport).is_empty());
    start(cx, handle, point, gpui::point(px(0.), px(-60.))).await;
    let foreign = handle
        .update(cx, |v, window, _| {
            let hitbox = v.carousels[&node(462)].borrow().shield_hitbox().unwrap();
            window.capture_pointer(hitbox);
            hitbox
        })
        .unwrap();
    key(cx, handle, "escape");
    assert_eq!(
        handle.update(cx, |_, w, _| w.captured_hitbox()).unwrap(),
        Some(foreign),
        "carousel cancellation must not release a foreign capture"
    );
    handle.update(cx, |_, w, _| w.release_pointer()).unwrap();
    mouse(cx, handle, point, false);
    frame(cx, handle).await;
    assert!(requests(transport).is_empty());
    start(cx, handle, point, gpui::point(px(0.), px(-60.))).await;
    let other = cx
        .update(|cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(160.), px(100.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| gpui::Empty),
            )
        })
        .unwrap();
    other.update(cx, |_, w, _| w.activate_window()).unwrap();
    for _ in 0..100 {
        if !handle.update(cx, |_, w, _| w.is_window_active()).unwrap() {
            break;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    assert!(!handle.update(cx, |_, w, _| w.is_window_active()).unwrap());
    assert!(
        !captured(cx, handle),
        "native window deactivation cancels capture"
    );
    other.update(cx, |_, w, _| w.remove_window()).unwrap();
    handle.update(cx, |_, w, _| w.activate_window()).unwrap();
    for _ in 0..100 {
        if handle.update(cx, |_, w, _| w.is_window_active()).unwrap() {
            break;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    frame(cx, handle).await;
    mouse(cx, handle, point, false);
    assert!(requests(transport).is_empty());
    start(cx, handle, point, gpui::point(px(0.), px(-60.))).await;
    let mut disabled = config(9, 0, Axis::Vertical, Direction::Direct);
    disabled.disabled = true;
    apply(cx, handle, vec![Op::SetCarousel(node(462), disabled)]);
    frame(cx, handle).await;
    assert!(!captured(cx, handle), "model invalidation cancels capture");
    mouse(cx, handle, point, false);
    assert!(requests(transport).is_empty());
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(
                node(462),
                config(10, 1, Axis::Horizontal, Direction::Direct),
            ),
            Op::SetNavigationStack(node(463), presentation(1, 0)),
        ],
    );
    frame(cx, handle).await;
    println!(
        "GPUIO_CAROUSEL_DRAG_OK: axis lock, native capture/rebind, two-page GPU preview, inert neighbor, one request, ignored snap, accepted painted retarget, in-flight grab, Escape, editor/button priority, reduced motion, loop, vertical/outside, hidden/pointer-policy/resize/capture-loss/model/inactivity cancellation, foreign capture ownership"
    );
}

/// A second small fixture proves disposal while captured without weakening the
/// tree invariant that every live node belongs to the mounted root.
pub(super) async fn teardown(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut operations = vec![];
    for (slot, kind, label) in [
        (473, Kind::Carousel, "Retirement gallery"),
        (474, Kind::NavigationStack, "Pages"),
        (475, Kind::Panel, "A"),
        (476, Kind::Panel, "B"),
    ] {
        operations.push(Op::Create(
            node(slot),
            kind,
            label.into(),
            (slot == 473).then(|| gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()),
        ));
        operations.push(Op::SetStyle(
            node(slot),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(200.)),
            ])],
        ));
    }
    operations.extend([
        Op::SetCarousel(node(473), config(0, 0, Axis::Horizontal, Direction::Direct)),
        Op::SetNavigationStack(node(474), presentation(0, 200)),
        Op::Splice(node(474), 0, 0, vec![node(475), node(476)]),
        Op::Splice(node(473), 0, 0, vec![node(474)]),
        Op::SetRoot(Some(node(473))),
    ]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    let point = bounds(cx, handle, 474).center();
    move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    mouse(cx, handle, point, true);
    move_mouse(cx, handle, point + gpui::point(px(-100.), px(0.)), true);
    frame(cx, handle).await;
    assert!(captured(cx, handle));
    let retired = handle
        .update(cx, |v, _, _| v.carousels[&node(473)].clone())
        .unwrap();
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((473..=476).map(|slot| Op::Remove(node(slot))))
            .collect(),
    );
    assert!(
        !captured(cx, handle),
        "unmount releases capture synchronously"
    );
    assert!(!retired.borrow().dragging());
    frame(cx, handle).await;
    mouse(cx, handle, point, false);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::CarouselRequested(..))),
        "release after unmount cannot emit a request"
    );
    handle
        .update(cx, |v, _, _| {
            assert!(v.carousels.is_empty() && v.navigation.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    assert_eq!(Rc::strong_count(&retired), 1);
    println!("GPUIO_CAROUSEL_CAPTURE_DISPOSAL_OK");
}
