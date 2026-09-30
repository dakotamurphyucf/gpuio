//! Opaque table surfaces follow the document palette, not Base's global theme.
use super::*;

fn luminance(color: [u8; 4]) -> f64 {
    let channel = |value: u8| {
        let value = f64::from(value) / 255.;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn check_pixels(image: &image::RgbaImage, dark: bool) {
    let (body, header, foreground, border) = if dark {
        (
            [17, 19, 24, 255],
            [27, 30, 38, 255],
            [230, 231, 237, 255],
            [43, 47, 57, 255],
        )
    } else {
        (
            [255, 255, 255, 255],
            [243, 243, 241, 255],
            [38, 40, 50, 255],
            [224, 225, 223, 255],
        )
    };
    let mut x0 = image.width();
    let mut y0 = image.height();
    let (mut x1, mut y1) = (0, 0);
    let mut border_pixels = 0;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel.0 == header {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        border_pixels += usize::from(pixel.0 == border);
    }
    assert!(
        x1 > x0 + 20 && y1 > y0 + 10,
        "painted header missing: dark={dark}"
    );
    assert!(
        border_pixels > 20,
        "painted table border missing: dark={dark}"
    );
    // Inspect the first body row using the painted header's geometry, excluding
    // boundaries. This avoids counting the outer window's light background.
    let mut body_pixels = 0;
    let mut text_pixels = 0;
    let mut white_pixels = 0;
    for y in y1 + 2..(y1 + (y1 - y0) - 2).min(image.height()) {
        for x in x0 + 3..x1 - 3 {
            let pixel = image.get_pixel(x, y).0;
            body_pixels += usize::from(pixel == body);
            text_pixels += usize::from(pixel == foreground);
            white_pixels += usize::from(pixel == [255, 255, 255, 255]);
        }
    }
    eprintln!(
        "GPUIO_DOCUMENT_TABLE_PAINT dark={dark} body={body_pixels} text={text_pixels} white={white_pixels} border={border_pixels}"
    );
    assert!(
        body_pixels > 1000,
        "document table surface missing: dark={dark}"
    );
    assert!(
        text_pixels > 20,
        "document table body text missing: dark={dark}"
    );
    for background in [body, header] {
        let a = luminance(foreground);
        let b = luminance(background);
        assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 7.);
    }
}

fn opposite_global_theme(cx: &mut App, dark_document: bool) {
    let theme = gpui_base::Theme::global_mut(cx);
    theme.appearance = if dark_document {
        gpui_base::ThemeAppearance::Light
    } else {
        gpui_base::ThemeAppearance::Dark
    };
    theme.tokens.colors = if dark_document {
        gpui_base::ColorTokens::light()
    } else {
        gpui_base::ColorTokens::dark()
    };
}

struct TableRoot {
    state: Entity<TextViewState>,
    dark: bool,
    scroll: bool,
}

impl Render for TableRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut style = markdown_style(self.dark, None);
        if self.scroll {
            let mut table = style.table().clone();
            table.overflow.x = Some(gpui::Overflow::Scroll);
            let mut cell = gpui::StyleRefinement::default();
            cell.text.white_space = Some(gpui::WhiteSpace::Nowrap);
            style = style.with_table(table).with_table_cell(cell);
        }
        div()
            .size_full()
            .bg(gpui::rgb(0xff00ff))
            .child(TextView::new(&self.state).style(style))
    }
}

fn check_layout_paths(cx: &mut gpui::AsyncApp) {
    let prepared = gpui_base::text::PreparedMarkdown::parse(
        "| Very_long_header_one | Very_long_header_two |\n| --- | --- |\n| Readable body text | More readable text |\n| Last body row | Final cell |\n",
        gpui_base::text::MarkdownExtensions::default(),
    ).unwrap();
    let handle = cx
        .open_window(
            gpui::WindowOptions {
                focus: false,
                window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::new(
                    gpui::point(px(100.), px(100.)),
                    gpui::size(px(320.), px(200.)),
                ))),
                ..Default::default()
            },
            |_, cx| {
                let state = cx.new(TextViewState::externally_prepared);
                state.update(cx, |state, cx| state.set_prepared(prepared, None, cx));
                cx.new(|_| TableRoot {
                    state,
                    dark: false,
                    scroll: false,
                })
            },
        )
        .unwrap();
    for scroll in [false, true] {
        for dark in [true, false, true, false] {
            handle
                .update(cx, |root, _, cx| {
                    opposite_global_theme(cx, dark);
                    root.dark = dark;
                    root.scroll = scroll;
                    cx.notify();
                })
                .unwrap();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx))
                .unwrap();
            let image = handle
                .update(cx, |_, window, _| window.render_to_image().unwrap())
                .unwrap();
            eprintln!("GPUIO_DOCUMENT_TABLE_LAYOUT scroll={scroll} dark={dark}");
            check_pixels(&image, dark);
        }
    }
    handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    let original_theme = cx.update(|cx| gpui_base::Theme::global(cx));
    let Response::Created(source) = session.borrow_mut().document_request(Request::Create) else {
        panic!("table appearance source");
    };
    publish(
        &mut session.borrow_mut(),
        source,
        0,
        1,
        0,
        "| Header one | Header two |\n| --- | --- |\n| Readable body text | More readable text |\n| Last body row | Final cell |\n",
    );
    configure(
        cx,
        window,
        source,
        gpuio_protocol::document::Mode::Markdown,
        "",
    );
    let presentation = settle(cx, window).await;
    for dark in [true, false, true, false] {
        window
            .update(cx, |view, window, cx| {
                opposite_global_theme(cx, dark);
                let (base, mut config) = {
                    let session = view.session.borrow();
                    let tree = session.tree(id()).unwrap();
                    (
                        tree.revision(),
                        (**tree.get(node()).unwrap().document.as_ref().unwrap()).clone(),
                    )
                };
                config.dark = dark;
                let applied = view
                    .session
                    .borrow_mut()
                    .apply(&Transaction {
                        window: id(),
                        base,
                        revision: base + 1,
                        operations: vec![Op::SetDocument(node(), config)],
                    })
                    .unwrap();
                view.update_editors(&applied.dirty, window, cx);
                cx.notify();
            })
            .unwrap();
        let current = settle(cx, window).await;
        assert_eq!(
            current, presentation,
            "appearance retains native presentation"
        );
        let image = window
            .update(cx, |_, window, _| window.render_to_image().unwrap())
            .unwrap();
        check_pixels(&image, dark);
    }
    check_layout_paths(cx);
    cx.update(|cx| cx.set_global(original_theme));
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    eprintln!(
        "GPUIO_DOCUMENT_TABLE_APPEARANCE_OK: production table GPU body/header/text/border, independent palette, light/dark switches, contrast, retained presentation and wrap/scroll render paths"
    );
}
