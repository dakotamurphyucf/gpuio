//! Actual GPU paint of the host table's box and inherited text styling.
use super::*;

fn styles(background: Fill) -> Vec<Style> {
    vec![
        Style::Width(Length::Px(520.)),
        Style::Height(Length::Px(300.)),
        Style::Padding(12.),
        Style::Fields(vec![
            Field::Background(background),
            Field::Foreground(Color::Rgba(0x009900ff)),
            Field::BorderTopWidth(4.),
            Field::BorderRightWidth(4.),
            Field::BorderBottomWidth(4.),
            Field::BorderLeftWidth(4.),
            Field::BorderColor(Color::Rgba(0x0000ffff)),
            Field::TopLeftRadius(16.),
            Field::TopRightRadius(16.),
            Field::BottomLeftRadius(16.),
            Field::BottomRightRadius(16.),
            Field::FontSize(18.),
        ]),
    ]
}
fn image(cx: &mut gpui::AsyncApp, handle: gpui::WindowHandle<View>) -> image::RgbaImage {
    cx.update_window(handle.into(), |_, window, cx| {
        window.set_scale_factor(1.);
        window.draw(cx).clear(cx);
        window.render_to_image().unwrap()
    })
    .unwrap()
}
fn pixel(image: &image::RgbaImage, x: u32, y: u32, expected: [u8; 4], label: &str) {
    let actual = image.get_pixel(x, y).0;
    assert!(
        actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
        "{label} at {x},{y}: {actual:?} != {expected:?}"
    );
}

fn label_color(image: &image::RgbaImage, expected: [u8; 3], label: &str) {
    let count = (16..48)
        .flat_map(|y| (40..120).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            image.get_pixel(x, y).0[..3]
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual.abs_diff(expected) < 8)
        })
        .count();
    assert!(
        count > 10,
        "{label}: only {count} pixels match {expected:?}"
    );
}

async fn states(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    let mut styled = styles(Fill::Solid(Color::Rgba(0xffffffff)));
    styled.extend([
        Style::State(1, vec![Field::Foreground(Color::Rgba(0xff0000ff))]),
        Style::State(2, vec![Field::Foreground(Color::Rgba(0x0000ffff))]),
        Style::State(3, vec![Field::Foreground(Color::Rgba(0xffff00ff))]),
        Style::State(
            6,
            vec![
                Field::Foreground(Color::Rgba(0xff00ffff)),
                Field::Opacity(1.),
            ],
        ),
    ]);
    apply(cx, window, vec![Op::SetStyle(node(0), styled)]);
    window.update(cx, |_, window, cx| window.blur(cx)).unwrap();
    frame(cx, window).await;
    label_color(&image(cx, window), [0, 153, 0], "base text");
    window
        .update(cx, |view, window, cx| {
            view.tables[&node(0)]
                .borrow()
                .native
                .focus_handle(cx)
                .focus(window, cx)
        })
        .unwrap();
    frame(cx, window).await;
    label_color(&image(cx, window), [255, 0, 0], "focused text");
    let padding = gpui::point(px(8.), px(150.));
    super::super::super::native_test::move_mouse(cx, window, padding, false);
    frame(cx, window).await;
    label_color(&image(cx, window), [0, 0, 255], "hover overrides focus");
    super::super::super::native_test::mouse(cx, window, padding, true);
    frame(cx, window).await;
    label_color(&image(cx, window), [255, 255, 0], "pressed overrides hover");
    super::super::super::native_test::mouse(cx, window, padding, false);
    let mut disabled = config();
    disabled.disabled = true;
    apply(cx, window, vec![Op::SetTable(node(0), disabled)]);
    frame(cx, window).await;
    label_color(
        &image(cx, window),
        [255, 0, 255],
        "disabled suppresses pointer styles",
    );
    apply(cx, window, vec![Op::SetTable(node(0), config())]);
    super::super::super::native_test::move_mouse(
        cx,
        window,
        gpui::point(px(530.), px(330.)),
        false,
    );
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    apply(
        cx,
        window,
        vec![
            Op::Create(node(65), Kind::Container, String::new(), None),
            Op::SetStyle(
                node(65),
                vec![
                    Style::Width(Length::Px(540.)),
                    Style::Height(Length::Px(340.)),
                    Style::Background(Color::Rgba(0xffffffff)),
                ],
            ),
            Op::Splice(node(65), 0, 0, vec![node(0)]),
            Op::SetRoot(Some(node(65))),
            Op::SetStyle(node(0), styles(Fill::Solid(Color::Rgba(0xff000080)))),
        ],
    );
    super::super::super::native_test::move_mouse(
        cx,
        window,
        gpui::point(px(530.), px(330.)),
        false,
    );
    frame(cx, window).await;
    let first = image(cx, window);
    if let Ok(path) = std::env::var("GPUIO_TABLE_STYLE_SCREENSHOT") {
        first.save(path).unwrap();
    }
    pixel(
        &first,
        480,
        28,
        [255, 127, 127, 255],
        "header sees one translucent surface",
    );
    pixel(
        &first,
        480,
        200,
        [255, 127, 127, 255],
        "body sees one translucent surface",
    );
    pixel(
        &first,
        8,
        150,
        [255, 127, 127, 255],
        "padding inside one border",
    );
    pixel(&first, 2, 150, [0, 0, 255, 255], "outer border");
    pixel(
        &first,
        1,
        1,
        [255, 255, 255, 255],
        "rounded corner clips all layers",
    );
    for (label, range) in [("header", 16..48), ("body", 48..80)] {
        let green = range
            .flat_map(|y| (24..130).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let [r, g, b, _] = first.get_pixel(x, y).0;
                g > 100 && r < 30 && b < 30
            })
            .count();
        assert!(
            green > 10,
            "{label} inherits foreground: {green} green pixels"
        );
    }
    for selected in [
        Selection::Cell {
            row: RowKey(1),
            column: "name".into(),
        },
        Selection::Row(RowKey(1)),
    ] {
        window
            .update(cx, |view, _, cx| {
                view.tables[&node(0)]
                    .borrow()
                    .native
                    .update(cx, |state, cx| {
                        assert!(state.replace_selection(selected.clone(), cx));
                    });
            })
            .unwrap();
        frame(cx, window).await;
        let selected_image = image(cx, window);
        let green = (52..78)
            .flat_map(|y| (42..130).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let [r, g, b, _] = selected_image.get_pixel(x, y).0;
                g > 100 && r < 30 && b < 30
            })
            .count();
        assert!(
            green > 10,
            "selection must not cover its text: {selected:?}: {green}"
        );
    }
    window
        .update(cx, |view, _, cx| {
            view.tables[&node(0)]
                .borrow()
                .native
                .update(cx, |state, cx| {
                    state.replace_selection(Selection::Empty, cx);
                });
        })
        .unwrap();
    // The same mounted table must expose a gradient across header and body;
    // no opaque adapter fill can cover it, including the pinned column header.
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            styles(Fill::LinearGradient(
                90.,
                Color::Rgba(0xff0000ff),
                0.,
                Color::Rgba(0x0000ffff),
                1.,
            )),
        )],
    );
    frame(cx, window).await;
    let gradient = image(cx, window);
    let head = gradient.get_pixel(480, 28).0;
    let body = gradient.get_pixel(480, 200).0;
    assert!(
        head[0] > 0 && head[2] > 0 && head[1] <= 3,
        "gradient header: {head:?}"
    );
    assert!(
        body[0] > 0 && body[2] > 0 && body[1] <= 3,
        "gradient body: {body:?}"
    );
    apply(
        cx,
        window,
        vec![
            Op::SetStyle(node(3), vec![Style::Background(Color::Rgba(0xff0000ff))]),
            Op::SetStyle(node(5), vec![Style::Background(Color::Rgba(0x0000ffff))]),
        ],
    );
    frame(cx, window).await;
    let pinned = image(cx, window);
    pixel(
        &pinned,
        100,
        77,
        [255, 0, 0, 255],
        "scrolling column cannot overpaint pinned cell",
    );
    pixel(
        &pinned,
        300,
        77,
        [0, 0, 255, 255],
        "scrolling cell stays visible",
    );
    states(cx, window).await;
    apply(
        cx,
        window,
        vec![
            Op::SetStyle(node(3), vec![]),
            Op::SetStyle(node(5), vec![]),
            Op::Splice(node(65), 0, 1, vec![]),
            Op::SetRoot(Some(node(0))),
            Op::Remove(node(65)),
            Op::SetStyle(
                node(0),
                vec![
                    Style::Width(Length::Px(520.)),
                    Style::Height(Length::Px(300.)),
                ],
            ),
        ],
    );
    frame(cx, window).await;
    let reset = image(cx, window);
    pixel(
        &reset,
        480,
        12,
        [23, 25, 31, 255],
        "default header restored",
    );
    pixel(&reset, 480, 200, [23, 25, 31, 255], "default body restored");
    eprintln!(
        "GPUIO_TABLE_STYLE_OK: one alpha surface, gradient header/body, single border/padding, corner clipping and inherited text"
    );
}
