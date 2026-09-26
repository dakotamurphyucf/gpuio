//! Carousel presentation checks; gestures and native timing receive separate coverage.
use super::*;
use gpuio_protocol::carousel::{Axis, Config, Direction, Request};
fn config(revision: i64, selected: i64, axis: Axis, direction: Direction) -> Config {
    Config {
        revision,
        selected: Some(selected),
        ids: vec!["a".into(), "b".into()],
        looping: true,
        disabled: false,
        axis,
        direction,
        auto_advance_ms: None,
    }
}
fn presentation(selected: i64, duration_ms: i64) -> gpuio_protocol::navigation_stack::Config {
    gpuio_protocol::navigation_stack::Config {
        selected: Some(selected),
        retain: true,
        motion: gpuio_protocol::navigation_stack::Motion::Slide,
        duration_ms,
    }
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    editor: EditorConfig,
    transport: &Transport,
) {
    let mut operations = vec![];
    for (id, kind, label) in [
        (460, Kind::Container, ""),
        (461, Kind::Button, "Outside gallery"),
        (462, Kind::Carousel, "Gallery"),
        (463, Kind::NavigationStack, "Gallery viewport"),
        (464, Kind::Panel, "A"),
        (465, Kind::Button, "A action"),
        (466, Kind::Panel, "B"),
        (467, Kind::Button, "B action"),
        (468, Kind::Button, "Gallery controls"),
        (469, Kind::Input, "A draft"),
        (470, Kind::Input, "B draft"),
    ] {
        let handler = matches!(kind, Kind::Carousel | Kind::Button | Kind::Input)
            .then(|| gpuio_protocol::HandlerId::from_parts(id, 1).unwrap());
        operations.push(Op::Create(node(id), kind, label.into(), handler));
        if kind == Kind::Button {
            operations.push(Op::SetControl(node(id), Control::Button(false)));
        }
        if kind == Kind::Input {
            operations.push(Op::SetEditor(
                node(id),
                EditorConfig {
                    label: label.into(),
                    ..editor.clone()
                },
            ));
        }
    }
    operations.extend([
        Op::SetCarousel(node(462), config(0, 0, Axis::Vertical, Direction::Direct)),
        Op::SetNavigationStack(node(463), presentation(0, 2000)),
        Op::SetStyle(
            node(462),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(240.)),
            ])],
        ),
        Op::SetStyle(
            node(463),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(200.)),
            ])],
        ),
        Op::SetStyle(
            node(464),
            vec![Style::Fields(vec![
                Field::Width(Length::Percent(100.)),
                Field::Height(Length::Percent(100.)),
                Field::Background(Fill::Solid(Color::Rgba(0xe13599ff))),
            ])],
        ),
        Op::SetStyle(
            node(466),
            vec![Style::Fields(vec![
                Field::Width(Length::Percent(100.)),
                Field::Height(Length::Percent(100.)),
                Field::Background(Fill::Solid(Color::Rgba(0x2563ebff))),
            ])],
        ),
        Op::Splice(node(464), 0, 0, vec![node(469), node(465)]),
        Op::Splice(node(466), 0, 0, vec![node(470), node(467)]),
        Op::Splice(node(463), 0, 0, vec![node(464), node(466)]),
        Op::Splice(node(462), 0, 0, vec![node(463), node(468)]),
        Op::Splice(node(460), 0, 0, vec![node(461), node(462)]),
        Op::SetRoot(Some(node(460))),
    ]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(461)].focus, cx))
        .unwrap();
    frame(cx, handle).await;
    let first_handle = handle
        .update(cx, |v, _, cx| v.editors[&node(469)].focus_handle(cx))
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(462), config(1, 1, Axis::Vertical, Direction::Next)),
            Op::SetNavigationStack(node(463), presentation(1, 2000)),
        ],
    );
    frame(cx, handle).await;
    cx.background_executor()
        .timer(std::time::Duration::from_millis(400))
        .await;
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(461)),
        "programmatic selection must preserve outside focus"
    );
    handle
        .update(cx, |v, w, cx| {
            assert_eq!(first_handle, v.editors[&node(469)].focus_handle(cx));
            assert!(!v.focus.borrow().visible(node(469)));
            assert!(matches!(
                v.editors
                    .get_mut(&node(469))
                    .unwrap()
                    .command(&EditorCommand::Focus, w, cx),
                EditorResult::Failed(EditorError::FocusBlocked)
            ));
            #[cfg(feature = "native-image-tests")]
            {
                let image = w.render_to_image().unwrap();
                let bounds = v.probes.borrow()[&node(463)].bounds;
                let scale = w.scale_factor();
                for (y, expected) in [
                    (bounds.top() + px(4.), [0xe1, 0x35, 0x99, 0xff]),
                    (bounds.bottom() - px(4.), [0x25, 0x63, 0xeb, 0xff]),
                ] {
                    let pixel = image.get_pixel(
                        (f32::from(bounds.right() - px(4.)) * scale) as u32,
                        (f32::from(y) * scale) as u32,
                    );
                    assert_eq!(
                        pixel.0, expected,
                        "vertical incoming and inert outgoing paint"
                    );
                }
            }
        })
        .unwrap();
    // Axis is presentation-only and can change at the same model revision. An
    // owner-only update settles the old axis, instead of rotating a live slide.
    apply(
        cx,
        handle,
        vec![Op::SetCarousel(
            node(462),
            config(1, 1, Axis::Horizontal, Direction::Next),
        )],
    );
    frame(cx, handle).await;
    #[cfg(feature = "native-image-tests")]
    handle
        .update(cx, |v, w, _| {
            let image = w.render_to_image().unwrap();
            let bounds = v.probes.borrow()[&node(463)].bounds;
            let scale = w.scale_factor();
            for y in [bounds.top() + px(4.), bounds.bottom() - px(4.)] {
                assert_eq!(
                    image
                        .get_pixel(
                            (f32::from(bounds.right() - px(4.)) * scale) as u32,
                            (f32::from(y) * scale) as u32
                        )
                        .0,
                    [0x25, 0x63, 0xeb, 0xff],
                    "axis change settles the old transition"
                );
            }
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(463), presentation(1, 0))],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.editors[&node(470)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(462), config(2, 0, Axis::Horizontal, Direction::Next)),
            Op::SetNavigationStack(node(463), presentation(0, 0)),
        ],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(469)),
        "departing page focus transfers to destination"
    );
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(468)].focus, cx))
        .unwrap();
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(462), config(3, 1, Axis::Horizontal, Direction::Next)),
            Op::SetNavigationStack(node(463), presentation(1, 0)),
        ],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(468)),
        "controls retain focus across selection"
    );
    // Native keyboard routing is local to the owner and its ordinary controls.
    requests(transport);
    for name in ["left", "home", "right", "end"] {
        key(cx, handle, name);
    }
    frame(cx, handle).await;
    assert_eq!(
        requests(transport),
        vec![
            Request::Previous,
            Request::First,
            Request::Next,
            Request::Last
        ]
    );
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.carousels[&node(462)].borrow().focus_handle(), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "down");
    key(cx, handle, "alt-right");
    key(cx, handle, "left");
    frame(cx, handle).await;
    assert_eq!(requests(transport), vec![Request::Previous]);
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.editors[&node(470)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    for name in ["left", "right", "home", "end"] {
        key(cx, handle, name);
    }
    frame(cx, handle).await;
    assert!(
        requests(transport).is_empty(),
        "editor navigation does not escape to carousel"
    );

    let automatic = |revision, selected| {
        let mut value = config(revision, selected, Axis::Horizontal, Direction::Direct);
        value.auto_advance_ms = Some(1000);
        Op::SetCarousel(node(462), value)
    };
    let outside = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(461)].bounds.center())
        .unwrap();
    let inside = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(463)].bounds.center())
        .unwrap();
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(461)].focus, cx))
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, outside, false);
    apply(
        cx,
        handle,
        vec![
            automatic(4, 0),
            Op::SetNavigationStack(node(463), presentation(0, 0)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            let state = v.carousels[&node(462)].borrow();
            assert!(state.has_timer(), "{}", state.diagnostics(w, cx));
        })
        .unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    frame(cx, handle).await;
    assert_eq!(
        requests(transport),
        vec![Request::AutoNext {
            revision: 4,
            from: "a".into(),
            target: "b".into()
        }]
    );
    handle
        .update(cx, |v, _, _| {
            let state = v.carousels[&node(462)].borrow();
            assert!(state.pending() && !state.has_timer());
        })
        .unwrap();
    let idle_renders = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "unacknowledged automatic request does not repeat"
    );
    assert_eq!(
        handle.update(cx, |v, _, _| v.render_count).unwrap(),
        idle_renders,
        "pending auto-advance must not poll or request idle frames"
    );

    // The next deadline starts only after the accepted transition settles.
    apply(
        cx,
        handle,
        vec![
            automatic(5, 1),
            Op::SetNavigationStack(node(463), presentation(1, 2000)),
        ],
    );
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(463), presentation(1, 0))],
    );
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    super::super::native_test::move_mouse(cx, handle, inside, false);
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(requests(transport).is_empty(), "hover pauses deadline");
    super::super::native_test::move_mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    cx.background_executor()
        .timer(std::time::Duration::from_millis(200))
        .await;
    assert!(
        requests(transport).is_empty(),
        "resume waits a fresh full interval"
    );
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.editors[&node(470)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "editor focus pauses deadline"
    );
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(461)].focus, cx))
        .unwrap();
    frame(cx, handle).await;
    cx.update(|cx| cx.set_reduce_motion(true));
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "reduced motion pauses deadline"
    );
    cx.update(|cx| cx.set_reduce_motion(false));
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(460),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "hidden ancestor cancels deadline"
    );
    apply(cx, handle, vec![Op::SetStyle(node(460), vec![])]);
    frame(cx, handle).await;
    // Restored focus may choose the carousel; move it outside to rearm.
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(461)].focus, cx))
        .unwrap();
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );

    // A fully clipped viewport is not eligible merely because its owner is mounted.
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(462),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(240.)),
                Field::Position(1),
                Field::Top(Length::Px(5000.)),
            ])],
        )],
    );
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "clipped viewport cancels deadline"
    );
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(462),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(240.)),
            ])],
        )],
    );
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    // Actual native window activation must cancel immediately, without repainting
    // or waiting for the old deadline to wake and discover the inactive window.
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
        !handle
            .update(cx, |v, _, _| v.carousels[&node(462)].borrow().has_timer())
            .unwrap()
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "inactive window cancels deadline"
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
    handle
        .update(cx, |v, w, cx| {
            eprintln!(
                "CAROUSEL_REACTIVATED {}",
                v.carousels[&node(462)].borrow().diagnostics(w, cx)
            );
        })
        .unwrap();
    // Activation can publish the real OS pointer position. Establish the same
    // outside-hover condition as the initial automatic-advance test.
    super::super::native_test::move_mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            let state = v.carousels[&node(462)].borrow();
            assert!(state.has_timer(), "{}", state.diagnostics(w, cx));
        })
        .unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(200))
        .await;
    assert!(
        requests(transport).is_empty(),
        "activation restarts a full interval"
    );

    wheel_input::exercise(cx, handle, transport).await;
    let retired_carousel = handle
        .update(cx, |view, _, _| view.carousels[&node(462)].clone())
        .unwrap();

    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((460..=470).map(|i| Op::Remove(node(i))))
            .collect(),
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(
                v.editors.is_empty()
                    && v.buttons.is_empty()
                    && v.navigation.is_empty()
                    && v.carousels.is_empty()
            );
            assert!(v.carousel_activation.is_none());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "unmount cancels the last deadline"
    );
    assert!(!retired_carousel.borrow().has_timer());
    assert!(!retired_carousel.borrow().wheel_timer());
    assert_eq!(
        Rc::strong_count(&retired_carousel),
        1,
        "retired input handlers release owner"
    );
    println!(
        "GPUIO_CAROUSEL_PRESENTATION_OK: vertical GPU transition, retained editor, hidden focus rejection, outside/control focus preservation, page focus handoff and disposal; keyboard, editor key isolation, one pending automatic proposal, settled paint, hover/focus/reduced/hidden/clipped/inactive pause, idle frame count and timer disposal; wheel dispatch and teardown; pointer drag pending"
    );
}

pub(super) async fn standalone(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let config = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(4))
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .as_ref()
                .clone()
        })
        .unwrap();
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((0..=4).map(|i| Op::Remove(node(i))))
            .collect(),
    );
    frame(cx, handle).await;
    exercise(cx, handle, config, transport).await;
}

fn requests(transport: &Transport) -> Vec<Request> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::CarouselRequested(_, owner, _, _, request) if owner == node(462) => {
                Some(request)
            }
            _ => None,
        })
        .collect::<Vec<_>>()
}

#[path = "carousel_wheel_test.rs"]
mod wheel_input;
