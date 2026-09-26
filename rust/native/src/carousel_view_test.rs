//! Carousel presentation checks; gestures and native timing receive separate coverage.
use super::*;
use gpuio_protocol::carousel::{Axis, Config, Direction};
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
            assert!(v.editors.is_empty() && v.buttons.is_empty() && v.navigation.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    println!(
        "GPUIO_CAROUSEL_PRESENTATION_OK: vertical GPU transition, retained editor, hidden focus rejection, outside/control focus preservation, page focus handoff and disposal; gestures/timers pending"
    );
}
