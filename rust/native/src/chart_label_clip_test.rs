//! Window clipping must retire a child independently of its visible composite.
use super::*;

async fn resize(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, width: f32, shown: bool) {
    handle
        .update(cx, |_, window, _| {
            window.resize(gpui::size(px(width), px(240.)))
        })
        .unwrap();
    for _ in 0..100 {
        draw(cx, handle);
        if handle
            .update(cx, |view, window, _| {
                window.viewport_size().width == px(width)
                    && view.focus.borrow().allows(id(5)) == shown
            })
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    handle
        .update(cx, |view, window, _| {
            assert_eq!(
                window.viewport_size().width,
                px(width),
                "native resize acknowledged"
            );
            assert_eq!(
                view.focus.borrow().allows(id(5)),
                shown,
                "child clipping gate"
            );
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
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(160.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::SetStyle(
                id(5),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(80.)),
                    Field::Top(Length::Px(0.)),
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
        ],
    );
    let target = accessibility::target(cx, handle).await;
    presses(transport);
    accessibility::press(&target);
    accessibility::delivered(cx, transport).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&id(5)].focus, cx);
            assert!(view.buttons[&id(5)].focus.is_focused(window));
        })
        .unwrap();
    resize(cx, handle, 80., false).await;
    for _ in 0..100 {
        if handle
            .update(cx, |view, window, _| {
                !view.buttons[&id(5)].focus.is_focused(window)
            })
            .unwrap()
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    handle
        .update(cx, |view, window, _| {
            assert!(
                view.focus.borrow().allows(id(3)),
                "composite remains partly visible"
            );
            assert!(
                !view.focus.borrow().allows(id(5)),
                "button is fully clipped"
            );
            assert!(
                !view.buttons[&id(5)].focus.is_focused(window),
                "clipped child releases native focus"
            );
        })
        .unwrap();
    resize(cx, handle, 240., true).await;
    unsafe {
        let _: objc2::runtime::Bool = objc2::msg_send![&*target, accessibilityPerformPress];
    }
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert_eq!(
        presses(transport),
        0,
        "old AX target must retire across window clipping without a tree update"
    );
    let fresh = accessibility::target(cx, handle).await;
    accessibility::press(&fresh);
    accessibility::delivered(cx, transport).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::SetStyle(
                id(5),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(0.)),
                    Field::Top(Length::Px(0.)),
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
        ],
    );
    draw(cx, handle);
    for oversized in [false, true] {
        let target = accessibility::target(cx, handle).await;
        handle
            .update(cx, |view, window, cx| {
                window.focus(&view.buttons[&id(5)].focus, cx)
            })
            .unwrap();
        let mut fields = vec![
            Field::Width(Length::Px(80.)),
            Field::Height(Length::Px(if oversized { 20. } else { 0. })),
        ];
        if oversized {
            // Each style value is admitted; their combined natural width exceeds
            // the label geometry bound and must never reach native prepaint.
            fields.extend([
                Field::PaddingLeft(Length::Px(1_000_000.)),
                Field::PaddingRight(Length::Px(1_000_000.)),
            ]);
        }
        apply(
            cx,
            handle,
            vec![Op::SetStyle(id(3), vec![Style::Fields(fields)])],
        );
        draw(cx, handle);
        handle
            .update(cx, |view, _, _| {
                assert!(
                    !view.focus.borrow().allows(id(5)),
                    "invalid label bounds must gate descendants, oversized={oversized}"
                )
            })
            .unwrap();
        unsafe {
            let _: objc2::runtime::Bool = objc2::msg_send![&*target, accessibilityPerformPress];
        }
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;
        assert_eq!(presses(transport), 0);
        handle
            .update(cx, |view, window, _| {
                assert!(
                    !view.buttons[&id(5)].focus.is_focused(window),
                    "invalid bounds release focus"
                )
            })
            .unwrap();
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            )],
        );
        let fresh = accessibility::target(cx, handle).await;
        accessibility::press(&fresh);
        accessibility::delivered(cx, transport).await;
    }
    // Visibility changes may request a repair frame, not an idle render loop.
    let mut count = handle.update(cx, |view, _, _| view.render_count).unwrap();
    let mut quiet = 0;
    for _ in 0..100 {
        cx.background_executor()
            .timer(Duration::from_millis(20))
            .await;
        let next = handle.update(cx, |view, _, _| view.render_count).unwrap();
        quiet = if count == next { quiet + 1 } else { 0 };
        count = next;
        if quiet == 5 {
            break;
        }
    }
    assert_eq!(
        quiet, 5,
        "clipping cleanup must settle without idle redraws"
    );
    eprintln!(
        "GPUIO_RADAR_CLIP_ACTION_OK: partial composite, clipped child focus/AX retirement on resize, zero/oversized bounds rejection and fresh recovery"
    );
}
