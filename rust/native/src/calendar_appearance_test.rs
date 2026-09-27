//! Actual GPU readback of calendar themes, density, typography and clipping.
use super::*;
use gpuio_protocol::v1::Length;

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let original_scale = handle.update(cx, |_, w, _| w.scale_factor()).unwrap();
    let original = snapshot(cx, handle);
    let mut cases = 0;
    for (name, background, foreground) in [
        ("light", 0xf4f6faff, 0x182332ff),
        ("dark", 0x161b22ff, 0xd7e0edff),
    ] {
        for width in [296., 220., 144., 48.] {
            for font_size in [13., 20.] {
                for presentation in [
                    c::Presentation::Days,
                    c::Presentation::Months,
                    c::Presentation::Years,
                ] {
                    // A short height additionally exercises clipped calendars; ordinary
                    // applications should allocate the natural content height.
                    for height in [None, Some(80.)] {
                        let mut fields = vec![
                            Field::Width(Length::Px(width)),
                            Field::Background(Fill::Solid(Color::Rgba(background))),
                            Field::Foreground(Color::Rgba(foreground)),
                            Field::FontSize(font_size),
                        ];
                        if let Some(height) = height {
                            fields.push(Field::Height(Length::Px(height)));
                        }
                        apply(
                            cx,
                            handle,
                            vec![Op::SetStyle(node(), vec![Style::Fields(fields)])],
                        );
                        handle
                            .update(cx, |v, w, cx| {
                                assert!(matches!(
                                    v.calendars[&node()].command(
                                        &c::Command::SetPresentation(presentation),
                                        w,
                                        cx
                                    ),
                                    c::Response::Applied(_)
                                ));
                            })
                            .unwrap();
                        frame(cx, handle).await;
                        events(transport);
                        for scale in [1., 2.] {
                            let image = cx
                                .update_window(handle.into(), |_, w, cx| {
                                    w.set_scale_factor(scale);
                                    w.draw(cx).clear(cx);
                                    w.render_to_image().unwrap()
                                })
                                .unwrap();
                            let (bounds, painted_font) = handle
                                .update(cx, |v, _, cx| {
                                    let bounds = v.probes.borrow()[&node()].bounds;
                                    let (_, font) =
                                        v.calendars[&node()].state.read(cx).paint_probe.unwrap();
                                    (bounds, font)
                                })
                                .unwrap();
                            assert_eq!(f32::from(bounds.size.width), width as f32);
                            assert_eq!(
                                f32::from(painted_font),
                                font_size as f32,
                                "caller font override"
                            );
                            let bg = [
                                (background >> 24) as u8,
                                (background >> 16) as u8,
                                (background >> 8) as u8,
                            ];
                            let fg = [
                                (foreground >> 24) as u8,
                                (foreground >> 16) as u8,
                                (foreground >> 8) as u8,
                            ];
                            let mut ink = 0;
                            let mut foreground_ink = 0;
                            let mut outside_ink = 0;
                            for (x, y, pixel) in image.enumerate_pixels() {
                                let p = point(
                                    px((x as f32 + 0.5) / scale),
                                    px((y as f32 + 0.5) / scale),
                                );
                                if bounds.contains(&p) {
                                    if pixel.0[..3]
                                        .iter()
                                        .zip(fg)
                                        .all(|(a, b)| a.abs_diff(b) <= 10)
                                    {
                                        foreground_ink += 1;
                                    }
                                    if pixel.0[..3].iter().zip(bg).any(|(a, b)| a.abs_diff(b) > 10)
                                    {
                                        ink += 1;
                                    }
                                } else if pixel.0[..3].iter().any(|v| *v > 5) {
                                    outside_ink += 1;
                                }
                            }
                            assert!(
                                ink > 10,
                                "{name} {width} {font_size} {presentation:?} {height:?} @{scale}: no calendar ink"
                            );
                            if width >= 144. {
                                assert!(
                                    foreground_ink > 10,
                                    "{name} {width} {font_size} {presentation:?} {height:?} @{scale}: missing foreground glyphs"
                                );
                            }
                            assert_eq!(
                                outside_ink, 0,
                                "{name} {width} {font_size} {presentation:?} {height:?} @{scale}: paint escaped bounds"
                            );
                            assert_eq!(snapshot(cx, handle).selection, original.selection);
                            if width == 296. && height.is_none() && scale == 2. && font_size == 13.
                            {
                                capture(
                                    cx,
                                    handle,
                                    &format!("calendar-layout-{name}-{presentation:?}"),
                                );
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    handle
        .update(cx, |_, w, _| w.set_scale_factor(original_scale))
        .unwrap();
    eprintln!(
        "GPUIO_CALENDAR_APPEARANCE_OK: {cases} GPU theme/density/font/presentation/constrained-layout cases; retained selection"
    );
}
