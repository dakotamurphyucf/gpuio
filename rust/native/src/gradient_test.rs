//! GPU evidence through the production declarative style bridge.
use super::*;

fn near(actual: [u8; 4], expected: [u8; 4]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 5),
        "gradient pixel {actual:?}, expected {expected:?}"
    );
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    let node = NodeId::from_parts(12, 1).unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Create(node, Kind::Container, String::new(), None),
            Op::SetRoot(Some(node)),
        ],
    );
    let mut legacy_mid: Option<[u8; 4]> = None;
    for space in [None, Some(0), Some(1), None] {
        let fill = match space {
            None => Fill::LinearGradient(
                90.,
                Color::Rgba(0xff0000ff),
                0.25,
                Color::Rgba(0x0000ffff),
                0.75,
            ),
            Some(space) => Fill::LinearGradientIn(
                space,
                90.,
                Color::Rgba(0xff0000ff),
                0.25,
                Color::Rgba(0x0000ffff),
                0.75,
            ),
        };
        apply(
            cx,
            window,
            vec![Op::SetStyle(
                node,
                vec![Style::Fields(vec![
                    Field::Width(Length::Percent(100.)),
                    Field::Height(Length::Percent(100.)),
                    Field::Background(fill),
                ])],
            )],
        );
        cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        window
            .update(cx, |_, window, _| {
                let image = window.render_to_image().unwrap();
                let y = image.height() / 2;
                near(image.get_pixel(image.width() / 10, y).0, [255, 0, 0, 255]);
                near(
                    image.get_pixel(image.width() * 9 / 10, y).0,
                    [0, 0, 255, 255],
                );
                let middle = image.get_pixel(image.width() / 2, y).0;
                if space == Some(1) {
                    // Independent red/blue Oklab midpoint ≈ (140,83,162), unlike
                    // sRGB's (128,0,128). Tolerance covers center-pixel sampling.
                    near(middle, [140, 83, 162, 255]);
                    assert!(middle[1] > legacy_mid.unwrap()[1] + 60);
                } else {
                    near(middle, [128, 0, 128, 255]);
                    if let Some(old) = legacy_mid {
                        assert_eq!(middle, old);
                    }
                    legacy_mid = Some(middle);
                }
            })
            .unwrap();
    }
    apply(cx, window, vec![Op::SetRoot(None), Op::Remove(node)]);
    eprintln!(
        "GPUIO_NATIVE_GRADIENT_OK: legacy and explicit sRGB identical GPU pixels, Oklab independent midpoint, clamped stops and same-node restyle back to sRGB"
    );
}
