//! GPU readback and layout checks on the actual mounted numeric editor.
use super::*;
use gpuio_protocol::v1::Fill;

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let original_scale = handle.update(cx, |_, w, _| w.scale_factor()).unwrap();
    let original_size = handle.update(cx, |_, w, _| w.viewport_size()).unwrap();
    // GPUI's synthetic scale does not resize the platform drawable. Keep the
    // entire 300px field available through 3x on a physical 1x display too.
    handle
        .update(cx, |_, w, _| w.resize(size(px(940.), px(220.))))
        .unwrap();
    frame(cx, handle).await;
    for (name, background, foreground) in [
        ("light", 0xf4f6faff, 0x2244aaff),
        ("dark", 0x161b22ff, 0x88bbffff),
    ] {
        for controls in [
            n::StepControls::Sides,
            n::StepControls::Stacked,
            n::StepControls::Hidden,
        ] {
            apply(
                cx,
                handle,
                vec![
                    Op::SetNumberInput(
                        node(1),
                        n::Config {
                            step_controls: controls,
                            ..config()
                        },
                        n::Value::Empty,
                    ),
                    Op::SetStyle(
                        node(0),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(320.)),
                            Field::Height(Length::Px(120.)),
                            Field::Background(Fill::Solid(Color::Rgba(background))),
                        ])],
                    ),
                    Op::SetStyle(
                        node(1),
                        vec![
                            Style::Fields(vec![
                                Field::Width(Length::Px(300.)),
                                Field::Height(Length::Px(48.)),
                                Field::Foreground(Color::Rgba(foreground)),
                                Field::FontSize(16.),
                                Field::BorderTopWidth(2.),
                                Field::BorderRightWidth(2.),
                                Field::BorderBottomWidth(2.),
                                Field::BorderLeftWidth(2.),
                                Field::BorderColor(Color::Rgba(foreground)),
                            ]),
                            Style::State(1, vec![Field::BorderColor(Color::Rgba(0x228866ff))]),
                        ],
                    ),
                ],
            );
            replace(cx, handle, "3.25");
            for scale in [1., 2., 3.] {
                for focused in [false, true] {
                    handle
                        .update(cx, |v, w, cx| {
                            let focus = if focused {
                                v.numbers[&node(1)].focus_handle(cx)
                            } else {
                                v.root_focus.clone().unwrap()
                            };
                            w.focus(&focus, cx);
                        })
                        .unwrap();
                    frame(cx, handle).await;
                    cx.update_window(handle.into(), |_, w, cx| {
                        w.set_scale_factor(scale);
                        w.draw(cx).clear(cx);
                    })
                    .unwrap();
                    handle
                        .update(cx, |v, w, cx| {
                            let editor_bounds = v.numbers[&node(1)].state.read(cx).input_bounds();
                            // The absolute probe fills the content box; include the known 2px border.
                            let bounds = v.probes.borrow()[&node(1)].bounds.dilate(px(2.));
                            assert_eq!(bounds.size, size(px(300.), px(48.)));
                            let owner = v.numbers[&node(1)].owner.borrow();
                            if controls != n::StepControls::Hidden {
                                for button in owner.repeat.buttons {
                                    assert!(
                                        button.left() >= bounds.left()
                                            && button.right() <= bounds.right()
                                            && button.top() >= bounds.top()
                                            && button.bottom() <= bounds.bottom(),
                                        "{controls:?} button outside field: {button:?}, {bounds:?}"
                                    );
                                }
                            }
                            let image = w.render_to_image().unwrap();
                            assert_eq!(w.scale_factor(), scale);
                            assert!(
                                bounds.left() >= px(0.) && bounds.top() >= px(0.)
                                    && f32::from(bounds.right()) * scale <= image.width() as f32
                                    && f32::from(bounds.bottom()) * scale <= image.height() as f32,
                                "{name} {controls:?} scale={scale}: field {bounds:?} exceeds capture {}x{}",
                                image.width(), image.height()
                            );
                            let rgba = |color: i64| {
                                [
                                    (color >> 24) as u8,
                                    (color >> 16) as u8,
                                    (color >> 8) as u8,
                                    color as u8,
                                ]
                            };
                            let sample = |point: Point<Pixels>| {
                                image
                                    .get_pixel(
                                        (f32::from(point.x) * scale) as u32,
                                        (f32::from(point.y) * scale) as u32,
                                    )
                                    .0
                            };
                            let equal = |a: [u8; 4], b: [u8; 4]| {
                                a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 2)
                            };
                            let border = if focused { 0x228866ff } else { foreground };
                            assert!(
                                equal(
                                    sample(point(bounds.center().x, bounds.top() + px(1.))),
                                    rgba(border)
                                ),
                                "{name} {controls:?} focus={focused} scale={scale}: focus border"
                            );
                            assert!(
                                equal(
                                    sample(point(bounds.left() + px(80.), bounds.top() + px(5.))),
                                    rgba(background)
                                ),
                                "{name} {controls:?}: inherited background"
                            );
                            // Exclude both button columns and the border; verify native
                            // text inherits the supplied foreground rather than a fixed theme color.
                            let mut ink = 0;
                            for y in ((f32::from(editor_bounds.top()) + 1.) * scale) as u32
                                ..((f32::from(editor_bounds.bottom()) - 1.) * scale) as u32
                            {
                                for x in ((f32::from(editor_bounds.left()) + 1.) * scale) as u32
                                    ..((f32::from(editor_bounds.right()) - 1.) * scale) as u32
                                {
                                    if equal(image.get_pixel(x, y).0, rgba(foreground)) {
                                        ink += 1;
                                    }
                                }
                            }
                            assert!(
                                ink > 2,
                                "{name} {controls:?} scale={scale}: missing inherited text ink"
                            );
                            if scale == 2.
                                && !focused
                                && let Ok(directory) = std::env::var("GPUIO_NUMBER_SCREENSHOTS")
                            {
                                image
                                    .save(
                                        std::path::Path::new(&directory)
                                            .join(format!("number-{name}-{controls:?}.png")),
                                    )
                                    .unwrap();
                            }
                        })
                        .unwrap();
                    assert_eq!(snapshot(cx, handle).draft, "3.25");
                    events(transport);
                }
            }
        }
    }
    for controls in [
        n::StepControls::Sides,
        n::StepControls::Stacked,
        n::StepControls::Hidden,
    ] {
        for (width, height) in [(80., 40.), (180., 40.), (180., 64.)] {
            apply(
                cx,
                handle,
                vec![
                    Op::SetNumberInput(
                        node(1),
                        n::Config {
                            step_controls: controls,
                            ..config()
                        },
                        n::Value::Empty,
                    ),
                    Op::SetStyle(
                        node(1),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(width)),
                            Field::Height(Length::Px(height)),
                            Field::BorderTopWidth(2.),
                            Field::BorderRightWidth(2.),
                            Field::BorderBottomWidth(2.),
                            Field::BorderLeftWidth(2.),
                        ])],
                    ),
                ],
            );
            frame(cx, handle).await;
            handle.update(cx, |v, _, cx| {
                let bounds = v.probes.borrow()[&node(1)].bounds;
                assert_eq!(bounds.size, size(px(width as f32 - 4.), px(height as f32 - 4.)));
                let inside = |child: Bounds<Pixels>| {
                    child.left() >= bounds.left() && child.right() <= bounds.right()
                        && child.top() >= bounds.top() && child.bottom() <= bounds.bottom()
                        && child.size.width > px(0.) && child.size.height > px(0.)
                };
                let instance = &v.numbers[&node(1)];
                let input = instance.state.read(cx).input_bounds();
                assert!(inside(input), "{controls:?} {width}x{height}: editor {input:?} outside {bounds:?}");
                if controls != n::StepControls::Hidden {
                    for button in instance.owner.borrow().repeat.buttons {
                        assert!(inside(button), "{controls:?} {width}x{height}: button {button:?} outside {bounds:?}");
                        assert!(!button.intersects(&input), "step button overlaps editable text");
                    }
                }
            }).unwrap();
            assert_eq!(snapshot(cx, handle).draft, "3.25");
            events(transport);
        }
    }
    handle
        .update(cx, |_, w, _| {
            w.set_scale_factor(original_scale);
            w.resize(original_size);
        })
        .unwrap();
    frame(cx, handle).await;
    eprintln!(
        "GPUIO_NUMBER_APPEARANCE_OK: light/dark, all layouts, native text and focus-border pixels, synthetic density, constrained geometry and retained drafts"
    );
}
