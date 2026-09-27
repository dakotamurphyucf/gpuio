use super::super::{
    editor_test::key,
    native_test::{mouse, move_mouse},
};
use super::*;
use gpuio_protocol::{
    rating::{Config as RatingConfig, Request},
    v1::Field as StyleField,
};

fn config(value: i64) -> RatingConfig {
    RatingConfig {
        label: "Response rating".into(),
        value,
        maximum: 5,
        star_size: 24.,
        disabled: false,
        read_only: false,
    }
}
fn requests(transport: &Transport) -> Vec<Request> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::RatingRequested(_, id, _, _, request) if id == node(6) => Some(request),
            Event::Press(_, id, ..) if id == node(6) => {
                panic!("rating must not emit generic button presses")
            }
            _ => None,
        })
        .collect()
}
fn position(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    star: i64,
) -> gpui::Point<gpui::Pixels> {
    handle
        .update(cx, |v, _, _| {
            let session = v.session.borrow();
            let size = session
                .tree(v.id)
                .unwrap()
                .get(node(6))
                .unwrap()
                .rating
                .as_ref()
                .unwrap()
                .star_size as f32;
            v.probes.borrow()[&node(6)].bounds.origin
                + gpui::point(px(1. + size * (star as f32 - 0.5)), px(1. + size / 2.))
        })
        .unwrap()
}
#[cfg(feature = "native-image-tests")]
fn pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, star: i64, filled: bool) {
    let position = position(cx, handle, star);
    handle
        .update(cx, |_, window, _| {
            let scale = window.scale_factor();
            let image = window.render_to_image().unwrap();
            let x = (f32::from(position.x) * scale) as u32;
            let y = (f32::from(position.y) * scale) as u32;
            assert!(
                x > 0 && y > 0 && x + 1 < image.width() && y + 1 < image.height(),
                "star {star} sample ({x}, {y}) at density {scale} must fit capture {}x{}",
                image.width(),
                image.height()
            );
            // Fractional density can put the logical center on a multisample
            // boundary. Require actual opaque fill in its nearest pixel region.
            let colors: Vec<_> = (y - 1..=y + 1)
                .flat_map(|y| (x - 1..=x + 1).map(move |x| (x, y)))
                .map(|(x, y)| image.get_pixel(x, y).0)
                .collect();
            assert_eq!(
                colors.contains(&[255, 136, 0, 255]),
                filled,
                "star {star} center pixels at density {scale}: {colors:?}"
            );
        })
        .unwrap();
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum Action {
    Read,
    Increase,
    Decrease,
    Set(f64),
}
#[cfg(target_os = "macos")]
fn accessible(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    action: Action,
) -> Option<(f64, f64, f64, bool)> {
    use objc2::{
        class, msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(
        object: *mut AnyObject,
        action: Action,
        depth: usize,
    ) -> Option<(f64, f64, f64, bool)> {
        if object.is_null() || depth > 20 {
            return None;
        }
        unsafe {
            let role: *mut NSString = msg_send![object, accessibilityRole];
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if role.as_ref().is_some_and(|s| s.to_string() == "AXSlider")
                && title
                    .as_ref()
                    .is_some_and(|s| s.to_string() == "Response rating")
            {
                let value: *mut AnyObject = msg_send![object, accessibilityValue];
                let min: *mut AnyObject = msg_send![object, accessibilityMinValue];
                let max: *mut AnyObject = msg_send![object, accessibilityMaxValue];
                assert!(!value.is_null() && !min.is_null() && !max.is_null());
                let value: f64 = msg_send![value, doubleValue];
                let min: f64 = msg_send![min, doubleValue];
                let max: f64 = msg_send![max, doubleValue];
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                match action {
                    Action::Read => (),
                    Action::Increase => {
                        let _: Bool = msg_send![object, accessibilityPerformIncrement];
                    }
                    Action::Decrease => {
                        let _: Bool = msg_send![object, accessibilityPerformDecrement];
                    }
                    Action::Set(value) => {
                        let number: *mut AnyObject =
                            msg_send![class!(NSNumber),numberWithDouble:value];
                        let _: () = msg_send![object,setAccessibilityValue:number];
                    }
                }
                return Some((value, min, max, enabled.as_bool()));
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for i in 0..count {
                let child: *mut AnyObject = msg_send![children,objectAtIndex:i];
                if let Some(found) = visit(child, action, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
    }
    let address = super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![address, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, action, 0)
    }
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
            Op::Create(node(5), Kind::Container, "".into(), None),
            Op::Create(node(6), Kind::Rating, "".into(), Some(handler(6))),
            Op::SetRating(node(6), config(2)),
            Op::SetStyle(
                node(6),
                vec![Style::Fields(vec![StyleField::Foreground(Color::Rgba(
                    0xff8800ff,
                ))])],
            ),
            Op::Create(
                node(7),
                Kind::Button,
                "After rating".into(),
                Some(handler(7)),
            ),
            Op::Splice(node(5), 0, 0, vec![node(6), node(7)]),
            Op::SetRoot(Some(node(5))),
        ],
    );
    frame(cx, handle).await;
    requests(transport);
    let fourth = position(cx, handle, 4);
    move_mouse(cx, handle, fourth, false);
    frame(cx, handle).await;
    assert_eq!(
        handle
            .update(cx, |v, _, _| v.ratings[&node(6)].borrow().hovered)
            .unwrap(),
        Some(4)
    );
    assert!(requests(transport).is_empty(), "hover is native-only");
    #[cfg(feature = "native-image-tests")]
    {
        pixels(cx, handle, 4, true);
        pixels(cx, handle, 5, false);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, Action::Read);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, Action::Read),
            Some((2., 0., 5., true)),
            "hover does not change AX value"
        );
    }
    mouse(cx, handle, fourth, true);
    mouse(cx, handle, fourth, false);
    frame(cx, handle).await;
    assert_eq!(requests(transport), vec![Request::Toggle(4)]);
    // No OCaml transaction between these key presses. Requests must stay relative.
    for _ in 0..4 {
        key(cx, handle, "right");
    }
    assert_eq!(requests(transport), vec![Request::Increase; 4]);
    handle
        .update(cx, |v, _, _| {
            assert_eq!(
                v.session
                    .borrow()
                    .tree(v.id)
                    .unwrap()
                    .get(node(6))
                    .unwrap()
                    .rating
                    .as_ref()
                    .unwrap()
                    .value,
                2
            )
        })
        .unwrap();
    apply(cx, handle, vec![Op::SetRating(node(6), config(5))]);
    frame(cx, handle).await;
    move_mouse(cx, handle, gpui::point(px(-10.), px(-10.)), false);
    frame(cx, handle).await;
    #[cfg(feature = "native-image-tests")]
    pixels(cx, handle, 5, true);
    for key_ in ["left", "down", "up", "home", "end", "backspace", "delete"] {
        key(cx, handle, key_);
    }
    assert_eq!(
        requests(transport),
        vec![
            Request::Decrease,
            Request::Decrease,
            Request::Increase,
            Request::Set(0),
            Request::Set(5),
            Request::Set(0),
            Request::Set(0)
        ]
    );
    key(cx, handle, "secondary-right");
    assert!(requests(transport).is_empty());
    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            accessible(cx, handle, Action::Read),
            Some((5., 0., 5., true))
        );
        for action in [Action::Increase, Action::Decrease, Action::Set(3.)] {
            accessible(cx, handle, action);
            frame(cx, handle).await;
        }
        assert_eq!(
            requests(transport),
            vec![Request::Increase, Request::Decrease, Request::Set(3)]
        );
        accessible(cx, handle, Action::Set(2.5));
        frame(cx, handle).await;
        assert!(requests(transport).is_empty());
    }
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, w, _| v.buttons[&node(7)].focus.is_focused(w))
            .unwrap()
    );
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, w, _| v.buttons[&node(6)].focus.is_focused(w))
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetRating(
                node(6),
                RatingConfig {
                    read_only: true,
                    ..config(3)
                },
            ),
            Op::Bind(node(6), None),
        ],
    );
    frame(cx, handle).await;
    key(cx, handle, "right");
    move_mouse(cx, handle, fourth, false);
    mouse(cx, handle, fourth, true);
    mouse(cx, handle, fourth, false);
    frame(cx, handle).await;
    assert!(requests(transport).is_empty());
    assert!(
        handle
            .update(cx, |v, _, _| v.ratings[&node(6)].borrow().hovered.is_none())
            .unwrap()
    );
    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            accessible(cx, handle, Action::Read),
            Some((3., 0., 5., true))
        );
        accessible(cx, handle, Action::Increase);
        frame(cx, handle).await;
        assert!(requests(transport).is_empty());
    }
    apply(
        cx,
        handle,
        vec![Op::SetRating(
            node(6),
            RatingConfig {
                disabled: true,
                ..config(3)
            },
        )],
    );
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, w, _| v.buttons[&node(6)].focus.is_focused(w))
            .unwrap()
    );
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, Action::Read),
        Some((3., 0., 5., false))
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetRating(node(6), config(3)),
            Op::Bind(node(6), Some(handler(6))),
        ],
    );
    frame(cx, handle).await;
    move_mouse(cx, handle, gpui::point(px(-10.), px(-10.)), false);
    move_mouse(cx, handle, fourth, false);
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(5),
            vec![Style::Fields(vec![StyleField::Visibility(1)])],
        )],
    );
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, _, _| v
                .ratings
                .get(&node(6))
                .is_none_or(|state| state.borrow().hovered.is_none()))
            .unwrap()
    );
    #[cfg(target_os = "macos")]
    assert!(accessible(cx, handle, Action::Read).is_none());
    apply(cx, handle, vec![Op::SetStyle(node(5), vec![])]);
    frame(cx, handle).await;
    // Pointer policy suppresses hover/click while preserving semantic keyboard input.
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(6),
            vec![Style::Fields(vec![
                StyleField::PointerEvents(false),
                StyleField::Foreground(Color::Rgba(0xff8800ff)),
            ])],
        )],
    );
    frame(cx, handle).await;
    move_mouse(cx, handle, fourth, false);
    mouse(cx, handle, fourth, true);
    mouse(cx, handle, fourth, false);
    frame(cx, handle).await;
    assert!(requests(transport).is_empty());
    assert!(
        handle
            .update(cx, |v, _, _| v.ratings[&node(6)].borrow().hovered.is_none())
            .unwrap()
    );
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(6)].focus, cx))
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "up");
    assert_eq!(requests(transport), vec![Request::Increase]);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(6),
            vec![Style::Fields(vec![StyleField::Foreground(Color::Rgba(
                0xff8800ff,
            ))])],
        )],
    );
    // A real modal excludes the rating from pointer and accessibility activation.
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(8), Kind::FocusScope, "".into(), Some(handler(8))),
            Op::SetFocusScope(
                node(8),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node(8),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Rating modal".into(),
                    width: 180.,
                    dismiss_on_escape: false,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::Create(
                node(9),
                Kind::Button,
                "Modal action".into(),
                Some(handler(9)),
            ),
            Op::Splice(node(8), 0, 0, vec![node(9)]),
            Op::Splice(node(5), 2, 0, vec![node(8)]),
        ],
    );
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.focus.borrow().allows(node(6)))
            .unwrap()
    );
    move_mouse(cx, handle, fourth, false);
    mouse(cx, handle, fourth, true);
    mouse(cx, handle, fourth, false);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        accessible(cx, handle, Action::Increase);
        frame(cx, handle).await;
    }
    assert!(requests(transport).is_empty());
    assert!(
        handle
            .update(cx, |v, _, _| v
                .ratings
                .get(&node(6))
                .is_none_or(|s| s.borrow().hovered.is_none()))
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(5), 2, 1, vec![]),
            Op::Remove(node(9)),
            Op::Remove(node(8)),
        ],
    );
    frame(cx, handle).await;
    // Maximum geometry remains bounded and respects synthetic native density.
    move_mouse(cx, handle, gpui::point(px(-10.), px(-10.)), false);
    apply(
        cx,
        handle,
        vec![Op::SetRating(
            node(6),
            RatingConfig {
                maximum: 32,
                value: 32,
                star_size: 8.,
                ..config(3)
            },
        )],
    );
    frame(cx, handle).await;
    #[cfg(feature = "native-image-tests")]
    {
        let original_scale = handle.update(cx, |_, w, _| w.scale_factor()).unwrap();
        let original_size = handle.update(cx, |_, w, _| w.viewport_size()).unwrap();
        // Test scale overrides do not resize the platform drawable. Reserve
        // enough capture space even on a 1x host for all 32 eight-pixel stars
        // at 3x, including the border and the sampled pixel neighborhood.
        handle
            .update(cx, |_, w, _| w.resize(size(px(800.), original_size.height)))
            .unwrap();
        frame(cx, handle).await;
        let revision = handle
            .update(cx, |v, _, _| {
                v.session.borrow().tree(v.id).unwrap().revision()
            })
            .unwrap();
        for scale in [1., 1.5, 2., 3., original_scale] {
            handle
                .update(cx, |_, w, _| w.set_scale_factor(scale))
                .unwrap();
            frame(cx, handle).await;
            assert_eq!(
                handle.update(cx, |_, w, _| w.scale_factor()).unwrap(),
                scale
            );
            pixels(cx, handle, 32, true);
            assert_eq!(
                handle
                    .update(cx, |v, _, _| v
                        .session
                        .borrow()
                        .tree(v.id)
                        .unwrap()
                        .revision())
                    .unwrap(),
                revision
            );
        }
        handle
            .update(cx, |_, w, _| w.resize(original_size))
            .unwrap();
        frame(cx, handle).await;
        eprintln!(
            "GPUIO_RATING_GPU_OK: star fill/outline, maximum geometry and synthetic density 1x/1.5x/2x/3x; restored window size/scale"
        );
    }
    move_mouse(cx, handle, gpui::point(px(-10.), px(-10.)), false);
    frame(cx, handle).await;
    let renders = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(150))
        .await;
    assert_eq!(
        handle.update(cx, |v, _, _| v.render_count).unwrap(),
        renders,
        "rating has no idle timer"
    );
    let weak = handle
        .update(cx, |v, _, _| Rc::downgrade(&v.ratings[&node(6)]))
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(6)),
            Op::Remove(node(7)),
            Op::Remove(node(5)),
        ],
    );
    frame(cx, handle).await;
    assert!(weak.upgrade().is_none());
    assert_eq!(
        handle
            .update(cx, |v, _, _| v.session.borrow().retained_bytes())
            .unwrap(),
        0
    );
    eprintln!(
        "GPUIO_RATING_NATIVE_OK: hover/GPU, ordered keys, AX actions, readonly/disabled, pointer/modal gates, idle and disposal"
    );
}
