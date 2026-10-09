//! Actual GPU readback through native_presentation; not a headless unit test.
use super::*;
use gpuio_protocol::rating::Appearance;

fn painted_color(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    star: i64,
    expected: [u8; 3],
) {
    let center = position(cx, handle, star);
    handle
        .update(cx, |_, window, _| {
            let scale = window.scale_factor();
            let image = window
                .render_to_image()
                .expect("rating appearance GPU readback");
            let half = 16. * scale;
            let x = f32::from(center.x) * scale;
            let y = f32::from(center.y) * scale;
            assert!(x > half && y > half);
            assert!(x + half < image.width() as f32 && y + half < image.height() as f32);
            let mut matching = 0;
            for y in (y - half) as u32..(y + half) as u32 {
                for x in (x - half) as u32..(x + half) as u32 {
                    let pixel = image.get_pixel(x, y).0;
                    if pixel[..3]
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(b) <= 3)
                    {
                        matching += 1;
                    }
                }
            }
            assert!(
                matching >= 4,
                "star {star} must actually paint {expected:?}, found {matching} pixels"
            );
        })
        .unwrap();
}
fn foreground(opacity: f64) -> Vec<Style> {
    vec![
        Style::Fields(vec![
            StyleField::Foreground(Color::Rgba(0xff8800ff)),
            StyleField::Opacity(opacity),
        ]),
        Style::State(2, vec![StyleField::Foreground(Color::Rgba(0x2266eeff))]),
    ]
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    move_mouse(cx, handle, gpui::point(px(-10.), px(-10.)), false);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                node(5),
                vec![Style::Fields(vec![
                    StyleField::Width(Length::Percent(100.)),
                    StyleField::Height(Length::Percent(100.)),
                    StyleField::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                ])],
            ),
            Op::SetRating(
                node(6),
                RatingConfig {
                    star_size: 32.,
                    ..config(2)
                },
            ),
            Op::SetStyle(node(6), foreground(1.)),
        ],
    );
    frame(cx, handle).await;
    let owner = handle
        .update(cx, |view, _, _| view.ratings[&node(6)].clone())
        .unwrap();
    requests(transport);
    let colors = Appearance {
        active: Some(0x0adc1eff),
        inactive: Some(0xf01e4680),
    };
    apply(
        cx,
        handle,
        vec![Op::SetRatingAppearance(node(6), Some(colors))],
    );
    frame(cx, handle).await;
    painted_color(cx, handle, 1, [10, 220, 30]);
    painted_color(cx, handle, 5, [120, 15, 35]);
    let fourth = position(cx, handle, 4);
    move_mouse(cx, handle, fourth, false);
    frame(cx, handle).await;
    painted_color(cx, handle, 4, [10, 220, 30]);
    painted_color(cx, handle, 5, [120, 15, 35]);
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, Action::Read),
        Some((2., 0., 5., true))
    );
    assert!(requests(transport).is_empty());
    apply(cx, handle, vec![Op::SetStyle(node(6), foreground(0.5))]);
    frame(cx, handle).await;
    painted_color(cx, handle, 4, [5, 110, 15]);
    painted_color(cx, handle, 5, [60, 8, 18]);

    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(node(6), foreground(1.)),
            Op::SetRatingAppearance(
                node(6),
                Some(Appearance {
                    active: None,
                    ..colors
                }),
            ),
        ],
    );
    frame(cx, handle).await;
    painted_color(cx, handle, 4, [34, 102, 238]);
    painted_color(cx, handle, 5, [120, 15, 35]);
    move_mouse(cx, handle, gpui::point(px(-10.), px(-10.)), false);
    frame(cx, handle).await;
    painted_color(cx, handle, 1, [255, 136, 0]);
    painted_color(cx, handle, 5, [120, 15, 35]);
    apply(cx, handle, vec![Op::SetRatingAppearance(node(6), None)]);
    frame(cx, handle).await;
    painted_color(cx, handle, 1, [255, 136, 0]);
    painted_color(cx, handle, 5, [179, 95, 0]);
    handle
        .update(cx, |view, _, _| {
            assert!(Rc::ptr_eq(&owner, &view.ratings[&node(6)]));
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .get(node(6))
                    .unwrap()
                    .rating
                    .as_ref()
                    .unwrap()
                    .value,
                2
            );
        })
        .unwrap();
    assert!(requests(transport).is_empty());
    eprintln!(
        "GPUIO_RATING_APPEARANCE_OK: independent colors/alpha, hover, inherited opacity/foreground, reset, stable owner/value"
    );
}
