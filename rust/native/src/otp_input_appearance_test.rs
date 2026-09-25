//! GPU readback of the mounted segmented editor, including masked paint.
use super::*;
use gpuio_protocol::v1::{Color, Field, Fill, Length, Style};

fn replace(cx: &mut AsyncApp, handle: WindowHandle<View>, text: &str) {
    assert!(matches!(
        command(
            cx,
            handle,
            o::Command::Replace {
                value: text.into(),
                selection: o::SelectionPolicy::End,
                undo: o::UndoPolicy::Reset,
                if_revision: None,
            }
        ),
        o::Response::Applied(_)
    ));
}

fn style(width: f64, height: f64, background: i64, foreground: i64) -> Vec<Style> {
    vec![
        Style::Fields(vec![
            Field::Width(Length::Px(width)),
            Field::Height(Length::Px(height)),
            Field::Background(Fill::Solid(Color::Rgba(background))),
            Field::Foreground(Color::Rgba(foreground)),
            Field::FontSize(16.),
            Field::BorderTopWidth(2.),
            Field::BorderRightWidth(2.),
            Field::BorderBottomWidth(2.),
            Field::BorderLeftWidth(2.),
            Field::BorderColor(Color::Rgba(foreground)),
        ]),
        Style::State(1, vec![Field::BorderColor(Color::Rgba(0x228866ff))]),
    ]
}

fn capture(cx: &mut AsyncApp, handle: WindowHandle<View>, scale: f32) -> image::RgbaImage {
    cx.update_window(handle.into(), |_, w, cx| {
        w.set_scale_factor(scale);
        w.draw(cx).clear(cx);
        w.render_to_image().unwrap()
    })
    .unwrap()
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let original = snapshot(cx, handle);
    let (original_config, original_scale) = handle
        .update(cx, |v, w, cx| {
            (
                v.otps[&node()].state.read(cx).model.config().clone(),
                w.scale_factor(),
            )
        })
        .unwrap();
    for (name, background, foreground) in [
        ("light", 0xf4f6faff, 0x2244aaff),
        ("dark", 0x161b22ff, 0x88bbffff),
    ]
    .into_iter()
    .cycle()
    .take(10)
    {
        for masked in [false, true] {
            apply(
                cx,
                handle,
                vec![
                    Op::SetOtpInput(
                        node(),
                        o::Config {
                            masked,
                            read_only: true,
                            ..config()
                        },
                        "".into(),
                    ),
                    Op::SetStyle(node(), style(300., 48., background, foreground)),
                ],
            );
            replace(cx, handle, "123456");
            for scale in [1., 2.] {
                for focused in [false, true] {
                    handle
                        .update(cx, |v, w, cx| {
                            if focused {
                                w.focus(&v.otps[&node()].focus_handle(cx), cx);
                            } else {
                                w.blur(cx);
                            }
                        })
                        .unwrap();
                    frame(cx, handle).await;
                    let image = capture(cx, handle, scale);
                    assert_eq!(
                        snapshot(cx, handle).value,
                        "123456",
                        "visual fixture value changed"
                    );
                    let bounds = handle
                        .update(cx, |v, _, cx| {
                            v.otps[&node()]
                                .state
                                .read(cx)
                                .layout
                                .as_ref()
                                .unwrap()
                                .bounds
                        })
                        .unwrap();
                    assert_eq!(bounds.size, size(px(296.), px(44.)));
                    let outer = bounds.dilate(px(2.));
                    let rgba = |color: i64| {
                        [
                            (color >> 24) as u8,
                            (color >> 16) as u8,
                            (color >> 8) as u8,
                            color as u8,
                        ]
                    };
                    let equal =
                        |a: [u8; 4], b: [u8; 4]| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 2);
                    let pixel = |p: Point<Pixels>| {
                        image
                            .get_pixel(
                                (f32::from(p.x) * scale) as u32,
                                (f32::from(p.y) * scale) as u32,
                            )
                            .0
                    };
                    assert!(
                        equal(
                            pixel(point(outer.center().x, outer.top() + px(1.))),
                            rgba(if focused { 0x228866ff } else { foreground })
                        ),
                        "{name} masked={masked} scale={scale} focus={focused}: focus border"
                    );
                    assert!(
                        equal(
                            pixel(point(bounds.right() - px(4.), bounds.top() + px(3.))),
                            rgba(background)
                        ),
                        "{name}: background"
                    );
                    for cell in 0..6 {
                        let left = f32::from(bounds.left()) + cell as f32 * 37.;
                        let mut ink = 0;
                        for y in ((f32::from(bounds.top()) + 9.) * scale) as u32
                            ..((f32::from(bounds.bottom()) - 9.) * scale) as u32
                        {
                            for x in ((left + 8.) * scale) as u32..((left + 24.) * scale) as u32 {
                                if equal(image.get_pixel(x, y).0, rgba(foreground)) {
                                    ink += 1;
                                }
                            }
                        }
                        assert!(
                            ink > 2,
                            "{name} masked={masked} scale={scale} focused={focused} cell={cell}: missing native glyph"
                        );
                    }
                    if !focused
                        && scale == 2.
                        && let Ok(directory) = std::env::var("GPUIO_OTP_SCREENSHOTS")
                    {
                        image
                            .save(std::path::Path::new(&directory).join(format!(
                                "otp-{name}-{}.png",
                                if masked { "masked" } else { "plain" }
                            )))
                            .unwrap();
                    }
                    if masked {
                        replace(cx, handle, "987654");
                        frame(cx, handle).await;
                        let changed = capture(cx, handle, scale);
                        assert!(image == changed, "masked pixels depend on the secret code");
                        replace(cx, handle, "123456");
                    }
                    events(transport);
                }
            }
        }
    }
    // Inspect selection and the continuously shaped IME presentation separately
    // from the accepted segmented cells. Mask changes must preserve composition.
    apply(
        cx,
        handle,
        vec![
            Op::SetOtpInput(node(), config(), "".into()),
            Op::SetStyle(node(), style(300., 48., 0x161b22ff, 0x88bbffff)),
        ],
    );
    replace(cx, handle, "123456");
    command(cx, handle, o::Command::Focus);
    command(
        cx,
        handle,
        o::Command::Select(o::Selection { anchor: 1, head: 4 }),
    );
    frame(cx, handle).await;
    let selected = capture(cx, handle, 2.);
    if let Ok(directory) = std::env::var("GPUIO_OTP_SCREENSHOTS") {
        selected
            .save(std::path::Path::new(&directory).join("otp-dark-selection.png"))
            .unwrap();
    }
    #[cfg(target_os = "macos")]
    {
        key(cx, handle, SELECT_ALL);
        native_text(cx, handle, "１２", true);
        frame(cx, handle).await;
        let before = snapshot(cx, handle);
        let plain = capture(cx, handle, 2.);
        apply(
            cx,
            handle,
            vec![Op::SetOtpInput(
                node(),
                o::Config {
                    masked: true,
                    ..config()
                },
                "".into(),
            )],
        );
        frame(cx, handle).await;
        let masked = capture(cx, handle, 2.);
        let after = snapshot(cx, handle);
        assert_eq!(
            (
                &after.value,
                &after.draft,
                after.selection,
                after.composition
            ),
            (
                &before.value,
                &before.draft,
                before.selection,
                before.composition
            )
        );
        assert!(plain != masked, "mask failed to conceal active preedit");
        native_text(cx, handle, "９８", true);
        frame(cx, handle).await;
        assert_eq!(
            capture(cx, handle, 2.),
            masked,
            "masked preedit pixels depend on raw characters"
        );
        if let Ok(directory) = std::env::var("GPUIO_OTP_SCREENSHOTS") {
            plain
                .save(std::path::Path::new(&directory).join("otp-dark-composition.png"))
                .unwrap();
            masked
                .save(std::path::Path::new(&directory).join("otp-dark-composition-masked.png"))
                .unwrap();
        }
        command(cx, handle, o::Command::CancelComposition);
        events(transport);
    }
    // Scroll-to-caret and platform geometry agree even when cells exceed the box.
    for (width, height) in [(40., 20.), (80., 40.), (180., 64.)] {
        apply(
            cx,
            handle,
            vec![
                Op::SetOtpInput(node(), config(), "".into()),
                Op::SetStyle(node(), style(width, height, 0x161b22ff, 0x88bbffff)),
            ],
        );
        replace(cx, handle, "123456");
        command(cx, handle, o::Command::Focus);
        frame(cx, handle).await;
        capture(cx, handle, 1.);
        handle
            .update(cx, |v, w, cx| {
                v.otps[&node()].state.update(cx, |s, cx| {
                    let bounds = s.layout.as_ref().unwrap().bounds;
                    assert_eq!(
                        bounds.size,
                        size(px(width as f32 - 4.), px(height as f32 - 4.))
                    );
                    let caret = s.bounds_for_range(6..6, bounds, w, cx).unwrap();
                    assert!(bounds.contains(&caret.origin));
                    assert_eq!(s.character_index_for_point(caret.origin, w, cx), Some(6));
                });
            })
            .unwrap();
        assert_eq!(snapshot(cx, handle).value, "123456");
        events(transport);
    }
    // Restore the native harness's state before its remaining lifecycle checks.
    apply(
        cx,
        handle,
        vec![
            Op::SetOtpInput(node(), original_config, "".into()),
            Op::SetStyle(node(), vec![]),
        ],
    );
    replace(cx, handle, &original.value);
    command(cx, handle, o::Command::Select(original.selection));
    handle
        .update(cx, |v, w, cx| {
            w.set_scale_factor(original_scale);
            if original.focused {
                w.focus(&v.otps[&node()].focus_handle(cx), cx);
            } else {
                w.blur(cx);
            }
        })
        .unwrap();
    frame(cx, handle).await;
    events(transport);
    eprintln!(
        "GPUIO_OTP_APPEARANCE_OK: light/dark, inherited glyph/background/focus pixels, synthetic density, masked invariance and constrained caret geometry"
    );
}
