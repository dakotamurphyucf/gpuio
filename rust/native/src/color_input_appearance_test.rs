//! Actual GPU readback of color input themes, density, typography and clipping.
use super::*;
use gpuio_protocol::v1::Length;

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let original_scale = handle.update(cx, |_, w, _| w.scale_factor()).unwrap();
    apply(cx, handle, vec![set(config())]);
    frame(cx, handle).await;
    events(transport);
    let original = snapshot(cx, handle);
    let mut cases = 0;
    for (name, background, foreground) in [
        ("light", 0xf4f6faff, 0x182332ff),
        ("dark", 0x161b22ff, 0xd7e0edff),
    ] {
        for width in [340., 220., 144., 48.] {
            for font_size in [13., 20.] {
                // A short height additionally exercises clipped color inputs; ordinary
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
                                let font = v.color_inputs[&node()].state.read(cx).painted_font;
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
                            let p =
                                point(px((x as f32 + 0.5) / scale), px((y as f32 + 0.5) / scale));
                            if bounds.contains(&p) {
                                if pixel.0[..3]
                                    .iter()
                                    .zip(fg)
                                    .all(|(a, b)| a.abs_diff(b) <= 10)
                                {
                                    foreground_ink += 1;
                                }
                                if pixel.0[..3].iter().zip(bg).any(|(a, b)| a.abs_diff(b) > 10) {
                                    ink += 1;
                                }
                            } else if pixel.0[..3].iter().any(|v| *v > 5) {
                                outside_ink += 1;
                            }
                        }
                        assert!(
                            ink > 10,
                            "{name} {width} {font_size} {height:?} @{scale}: no color input ink"
                        );
                        if width >= 144. {
                            assert!(
                                foreground_ink > 10,
                                "{name} {width} {font_size} {height:?} @{scale}: missing foreground glyphs"
                            );
                        }
                        assert_eq!(
                            outside_ink, 0,
                            "{name} {width} {font_size} {height:?} @{scale}: paint escaped bounds"
                        );
                        assert_eq!(snapshot(cx, handle), original);
                        if width == 340. && height.is_none() && scale == 2. && font_size == 13. {
                            capture(cx, handle, &format!("color-layout-{name}"));
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    handle
        .update(cx, |_, w, _| w.set_scale_factor(original_scale))
        .unwrap();
    transparency(cx, handle, transport).await;
    eprintln!(
        "GPUIO_COLOR_APPEARANCE_OK: {cases} GPU theme/density/font/constrained-layout cases; retained native state"
    );
}

async fn transparency(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(340.)),
                Field::Background(Fill::Solid(Color::Rgba(0x182332ff))),
                Field::Foreground(Color::Rgba(0xe3e9f3ff)),
            ])],
        )],
    );
    for (name, value) in [
        ("transparent", Value::Color(Rgba::new(255, 0, 0, 0))),
        ("half", Value::Color(Rgba::new(255, 0, 0, 128))),
        ("opaque", Value::Color(Rgba::new(255, 0, 0, 255))),
        ("empty", Value::Empty),
    ] {
        handle
            .update(cx, |v, w, cx| {
                assert!(matches!(
                    v.color_inputs[&node()].command(
                        &c::Command::Set {
                            value,
                            if_revision: None
                        },
                        w,
                        cx
                    ),
                    c::Response::Applied(_)
                ));
            })
            .unwrap();
        frame(cx, handle).await;
        events(transport);
        let image = cx
            .update_window(handle.into(), |_, w, cx| {
                w.set_scale_factor(1.);
                w.draw(cx).clear(cx);
                w.render_to_image().unwrap()
            })
            .unwrap();
        let mut colors = std::collections::BTreeSet::new();
        for y in 18..32 {
            for x in 18..32 {
                colors.insert(image.get_pixel(x, y).0);
            }
        }
        match name {
            "transparent" => {
                assert!(
                    colors.contains(&[238, 238, 238, 255])
                        && colors.contains(&[153, 153, 153, 255]),
                    "{colors:?}"
                );
            }
            "half" => assert!(
                colors.len() >= 2 && colors.iter().all(|p| p[0] > p[1]),
                "{colors:?}"
            ),
            "opaque" => assert_eq!(colors, std::collections::BTreeSet::from([[255, 0, 0, 255]])),
            "empty" => assert_eq!(
                colors,
                std::collections::BTreeSet::from([[24, 35, 50, 255]])
            ),
            _ => unreachable!(),
        }
        capture(cx, handle, &format!("color-swatch-{name}"));
    }
    handle
        .update(cx, |v, w, cx| {
            assert!(matches!(
                v.color_inputs[&node()].command(&c::Command::Reset { if_revision: None }, w, cx),
                c::Response::Applied(_)
            ));
        })
        .unwrap();
    frame(cx, handle).await;
    events(transport);
}
