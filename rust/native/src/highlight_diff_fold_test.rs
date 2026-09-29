//! Exercise native gutter folding against original UTF-8 source byte offsets.
use super::*;
use std::ops::Range;

fn bounds(
    cx: &mut AsyncApp,
    editor: &Entity<EditorState>,
    bytes: Range<usize>,
) -> Bounds<gpui::Pixels> {
    editor.read_with(cx, |editor, _| {
        editor
            .range_to_bounds(&bytes)
            .expect("visible source range")
    })
}
fn wash_in(cx: &mut AsyncApp, handle: WindowHandle<View>, bounds: Bounds<gpui::Pixels>) -> usize {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            let x0 = (f32::from(bounds.left()) * scale).max(0.) as u32;
            let x1 = (f32::from(bounds.right()) * scale).max(0.) as u32;
            let y0 = (f32::from(bounds.top()) * scale).max(0.) as u32;
            let y1 = (f32::from(bounds.bottom()) * scale).max(0.) as u32;
            (y0..y1.min(image.height()))
                .flat_map(|y| (x0..x1.min(image.width())).map(move |x| (x, y)))
                .filter(|(x, y)| image.get_pixel(*x, *y).0 == [255, 0, 0, 255])
                .count()
        })
        .unwrap()
}
async fn settle(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    for _ in 0..4 {
        draw(cx, handle);
        pause(cx).await;
    }
}
async fn gutter(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    editor: &Entity<EditorState>,
    header: Range<usize>,
) {
    let bounds = bounds(cx, editor, header);
    // Pinned Base places the 14px glyph within an 18px fold cell, before its
    // 10px right gutter margin. Derive y and source x from the actual layout.
    let point = gpui::point(bounds.left() - px(19.), bounds.center().y);
    crate::host::native_test::move_mouse(cx, handle, point, false);
    settle(cx, handle).await;
    crate::host::native_test::mouse(cx, handle, point, true);
    crate::host::native_test::mouse(cx, handle, point, false);
    settle(cx, handle).await;
}

pub(super) async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    let text = "diff --git a/世界.txt b/世界.txt\n--- a/世界.txt\n+++ b/世界.txt\n@@ -1,3 +1,3 @@\n-aaa old\n+aaa new\n keep α\n tail\ndiff --git a/other.txt b/other.txt\n--- a/other.txt\n+++ b/other.txt\n@@ -20,3 +20,3 @@\n context 🌍\n-plain\n+aaa later\n end\n";
    let first = text.find("aaa old").unwrap();
    let later = text.find("aaa later").unwrap();
    let header = text.find("@@").unwrap();
    let second_header = text[header + 2..].find("\n@@").unwrap() + header + 3;
    let base = p.read_with(cx, |p, _| p.installed.as_ref().unwrap().revision);
    publish(&mut session.borrow_mut(), source, base, text);
    handle
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    handle
        .update(cx, |_, window, _| window.resize(size(px(440.), px(520.))))
        .unwrap();
    let mut config_document = document(source, Mode::Diff);
    config_document.layout = Layout::Viewport(430.);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                node(0),
                vec![
                    Style::Width(Length::Px(440.)),
                    Style::Height(Length::Px(520.)),
                    Style::Background(Color::Rgba(0xffffffff)),
                ],
            ),
            Op::Bind(node(0), Some(handler(800))),
            Op::SetHighlightScope(node(0), config(0.)),
            Op::SetStyle(node(1), vec![]),
            Op::SetDocument(node(1), config_document),
        ],
    );
    installed_revision(cx, handle, transport, p, base + 1).await;
    ready(cx, handle, transport, 3).await;
    let editor = p.read_with(cx, |p, _| p.editor.clone());
    editor.update(cx, |editor, cx| {
        editor.bridge_select(0, 0, cx);
        editor.set_scroll_offset(gpui::point(px(0.), px(0.)), cx);
    });
    settle(cx, handle).await;
    let original = bounds(cx, &editor, later..later + 3);
    assert!(
        wash_in(cx, handle, original) > 20,
        "later source match initially paints at its byte range"
    );
    let total = red_pixels(cx, handle);
    let owner = p.read_with(cx, |p, _| p.highlight_paint.as_ref().unwrap().1.clone());
    let snapshot = p.read_with(cx, |p, _| p.installed.clone().unwrap());
    let _ = transport.mailbox.lock().unwrap().drain(128);
    gutter(cx, handle, &editor, header..header + 2).await;
    let next_file = text.find("diff --git a/other.txt").unwrap();
    assert!(
        editor.read_with(cx, |editor, _| editor
            .range_to_bounds(&(next_file..next_file + 4))
            .is_some()),
        "a folded hunk never consumes the next file header"
    );
    let folded = bounds(cx, &editor, later..later + 3);
    assert!(
        folded.top() < original.top(),
        "later source text moves up when the first hunk folds"
    );
    assert!(
        wash_in(cx, handle, folded) > 20,
        "later highlight follows source bytes across a hidden hunk"
    );
    assert!(
        red_pixels(cx, handle) < total,
        "hidden rows lose their painted washes"
    );
    assert!(
        editor.read_with(cx, |editor, _| editor
            .range_to_bounds(&(first..first + 3))
            .is_none()),
        "folded source bytes have no visible range bounds"
    );
    assert!(
        p.read_with(cx, |p, _| Rc::ptr_eq(
            &owner,
            &p.highlight_paint.as_ref().unwrap().1
        )),
        "folding reuses source matching and background ownership"
    );
    assert!(p.read_with(cx, |p, _| Arc::ptr_eq(
        &snapshot,
        p.installed.as_ref().unwrap()
    )));
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::HighlightObserved(..))),
        "hunk folding changes paint, not installed-page counts"
    );
    gutter(cx, handle, &editor, second_header..second_header + 2).await;
    assert_eq!(
        red_pixels(cx, handle),
        0,
        "both folded hunks leave no body wash"
    );
    assert!(
        editor.read_with(cx, |editor, _| editor
            .range_to_bounds(&(later..later + 3))
            .is_none()),
        "second folded hunk also has no visible range bounds"
    );
    gutter(cx, handle, &editor, header..header + 2).await;
    assert!(
        red_pixels(cx, handle) > 20,
        "expanding first hunk restores its original source matches"
    );
    gutter(cx, handle, &editor, second_header..second_header + 2).await;
    assert_eq!(bounds(cx, &editor, later..later + 3), original);
    assert_eq!(red_pixels(cx, handle), total);
    // Existing read-only editor selection still paints above the moved wash.
    handle
        .update(cx, |_, window, cx| {
            window.focus(&editor.read(cx).focus_handle(cx), cx)
        })
        .unwrap();
    editor.update(cx, |editor, cx| editor.bridge_select(later, later + 3, cx));
    draw(cx, handle);
    assert!(
        wash_in(cx, handle, original) < 20,
        "native selection takes precedence over the range wash"
    );
    editor.update(cx, |editor, cx| editor.bridge_select(0, 0, cx));
    draw(cx, handle);
    assert!(wash_in(cx, handle, original) > 20);
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.value().to_string()),
        text
    );
    eprintln!(
        "GPUIO_NATIVE_HIGHLIGHT_DIFF_FOLD_OK: real gutter collapse/expand, UTF-8 source offsets, hidden-row wash removal, later-range geometry, unchanged page matching/owners and selection precedence"
    );
}
