//! Header geometry, source selection and page replacement on the actual editor.
use super::*;

fn scroll(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    editor: &Entity<EditorState>,
    x: f32,
) {
    editor.update(cx, |e, cx| {
        e.set_scroll_offset(gpui::point(px(x), px(0.)), cx)
    });
    // The public setter applies after layout; the following frame paints that offset.
    draw(cx, window);
    draw(cx, window);
}
fn copy_selection(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    editor: &Entity<EditorState>,
    expected: &str,
) {
    let saved = cx.update(|cx| cx.read_from_clipboard());
    window
        .update(cx, |_, window, cx| {
            window.focus(&editor.read(cx).focus_handle(cx), cx)
        })
        .unwrap();
    crate::host::editor_test::key(cx, window, "secondary-c");
    let copied = cx.update(|cx| cx.read_from_clipboard().and_then(|item| item.text()));
    cx.update(|cx| {
        cx.write_to_clipboard(
            saved.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
        )
    });
    assert_eq!(
        copied.as_deref(),
        Some(expected),
        "native copy contains source bytes only"
    );
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    let Response::Created(source) = session.borrow_mut().document_request(Request::Create) else {
        panic!("source")
    };
    let path = format!("{}世界.ml", "wide_path_".repeat(20));
    let first_header = format!("--- a/{path}");
    let patch = format!(
        "{first_header}\n+++ b/{path}\n@@ -1 +1 @@\n-old λ\n+new 🦀\n--- a/x.ml\n+++ b/x.ml\n@@ -1 +1 @@\n keep β\n"
    );
    publish(&mut session.borrow_mut(), source, 0, 1, 0, &patch);
    configure(cx, window, source, gpuio_protocol::document::Mode::Diff, "");
    settings(cx, window, Collapse::Managed(vec![]));
    let p = settle(cx, window).await;
    let editor = p.read_with(cx, |p, _| p.editor.clone());
    scroll(cx, window, &editor, 0.);
    let fixed_gutter = p.read_with(cx, |p, cx| {
        assert_eq!(p.editor.read(cx).value(), patch);
        assert!(
            !p.file_suffixes.borrow().contains_key(&0),
            "long header suffix starts offscreen"
        );
        p.file_visible.borrow()[&0]
    });
    scroll(cx, window, &editor, -130.);
    p.read_with(cx, |p, cx| {
        let e = editor.read(cx);
        assert_eq!(
            e.scroll_offset().x,
            px(-130.),
            "fixture must actually scroll horizontally"
        );
        assert_eq!(
            p.file_visible.borrow()[&0],
            fixed_gutter,
            "gutter is fixed horizontally"
        );
        let short = p.file_suffixes.borrow()[&1];
        assert!(
            short.bounds.left() < short.clip.left(),
            "short caption straddles text/gutter boundary"
        );
        assert!(
            short.clip.left() >= fixed_gutter.right(),
            "caption paint cannot cover the gutter"
        );
        assert!(short.clip.right() <= e.input_bounds().right());
    });
    scroll(cx, window, &editor, -100000.);
    let far_offset = p.read_with(cx, |p, cx| {
        let e = editor.read(cx);
        assert!(e.scroll_offset().x < px(-500.));
        assert_eq!(p.file_visible.borrow()[&0], fixed_gutter);
        let suffix = p.file_suffixes.borrow()[&0];
        let header = e.range_to_bounds(&(0..first_header.len())).unwrap();
        assert!(
            (f32::from(suffix.bounds.left() - header.right())).abs() < 1.,
            "suffix starts at the shaped header end: suffix={:?}, header={:?}, offset={:?}",
            suffix.bounds,
            header,
            e.scroll_offset()
        );
        assert!((f32::from(suffix.bounds.center().y - header.center().y)).abs() < 1.);
        assert!(suffix.bounds.left() >= suffix.clip.left());
        assert!(
            suffix.bounds.right() <= suffix.clip.right(),
            "full caption slot is horizontally reachable"
        );
        assert!(
            !p.file_suffixes.borrow().contains_key(&1),
            "fully left-clipped caption does not enter current-frame map"
        );
        e.scroll_offset().x
    });
    scroll(cx, window, &editor, f32::from(far_offset + px(40.)));
    p.read_with(cx, |p, cx| {
        let e = editor.read(cx);
        let suffix = p.file_suffixes.borrow()[&0];
        let header = e.range_to_bounds(&(0..first_header.len())).unwrap();
        assert!(
            (f32::from(suffix.bounds.left() - header.right())).abs() < 1.,
            "caption follows horizontal movement exactly once"
        );
    });
    scroll(cx, window, &editor, 0.);
    // A backwards selection includes the original header and Unicode/line endings.
    let selected = patch.find("--- a/x.ml").unwrap();
    editor.update(cx, |e, cx| {
        assert!(e.bridge_select(patch.len(), selected, cx))
    });
    draw(cx, window);
    copy_selection(cx, window, &editor, &patch[selected..]);
    let stale = p.read_with(cx, |p, _| p.diff_action().unwrap());
    let retained_header = p.read_with(cx, |p, _| p.file_buttons[&1].1.clone());
    let append = "--- a/new.json\n+++ b/new.json\n@@ -1 +1 @@\n-1\n+2\n";
    publish(&mut session.borrow_mut(), source, 1, 1, patch.len(), append);
    window
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    settle(cx, window).await;
    p.read_with(cx, |p, cx| {
        assert_eq!(p.editor, editor);
        assert_eq!(
            p.editor.read(cx).value().as_ref(),
            format!("{patch}{append}")
        );
        assert_eq!(
            p.editor.read(cx).bridge_selection(),
            (patch.len(), selected)
        );
        assert_eq!(
            p.file_buttons[&1].1, retained_header,
            "same file keeps its focus identity across streaming"
        );
        assert_eq!(p.file_buttons.len(), 3);
    });
    copy_selection(cx, window, &editor, &patch[selected..]);
    // A callback from the preceding installed revision must not collapse a new picture.
    cx.update_window(window.into(), |_, window, cx| {
        p.update(cx, |p, cx| p.toggle_diff_file(&stale, 0, window, cx));
    })
    .unwrap();
    assert_eq!(
        editor.read_with(cx, |e, _| e.value()).as_ref(),
        format!("{patch}{append}")
    );
    // Removing preceding rows preserves the exact surviving selection, including direction.
    click_header(cx, window, &p, 0);
    let text = editor.read_with(cx, |e, _| e.value());
    let (anchor, head) = editor.read_with(cx, |e, _| e.bridge_selection());
    assert!(anchor > head);
    assert_eq!(&text[head..anchor], &patch[selected..]);
    copy_selection(cx, window, &editor, &patch[selected..]);
    // Reopening restores the same header/source geometry; select-all excludes decorations.
    click_header(cx, window, &p, 0);
    editor.update(cx, |e, cx| {
        assert!(e.bridge_select(0, patch.len() + append.len(), cx))
    });
    copy_selection(cx, window, &editor, &format!("{patch}{append}"));

    // Header handles and current-frame samples belong to the bounded installed page.
    let large: String = (0..350)
        .map(|i| format!("--- a/file-{i}.ml\n+++ b/file-{i}.ml\n@@ -1 +1 @@\n keep λ\n"))
        .collect();
    publish(&mut session.borrow_mut(), source, 2, 2, 0, &large);
    window
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    settle(cx, window).await;
    let (first_count, first_focus) = p.read_with(cx, |p, _| {
        assert!(p.page_end < p.display_bytes());
        (p.file_buttons.len(), p.file_buttons[&0].1.clone())
    });
    focus_header(cx, window, &p, 0);
    window
        .update(cx, |_, window, cx| {
            p.update(cx, |p, cx| {
                p.page_start = p.page_end;
                p.show_page(window, cx);
                assert!(!p.file_buttons.contains_key(&0));
                assert!(!first_focus.is_focused(window));
                assert!(
                    p.editor.read(cx).focus_handle(cx).is_focused(window),
                    "paging repairs outgoing header focus"
                );
                assert!(p.file_buttons.len() < first_count);
                assert!(
                    p.file_visible.borrow().is_empty(),
                    "retired geometry removed before the new frame"
                );
                assert!(p.file_suffixes.borrow().is_empty());
            })
        })
        .unwrap();
    draw(cx, window);
    p.read_with(cx, |p, cx| {
        assert_eq!(p.editor, editor);
        assert!(p.file_visible.borrow().keys().all(|i| *i > 0));
        assert!(p.file_suffixes.borrow().keys().all(|i| *i > 0));
        assert!(p.editor.read(cx).value().len() <= 65536);
        assert!(p.source_lines <= 1024);
    });
    let epoch = p.read_with(cx, |p, _| p.diff_epoch + 1);
    apply(cx, window, vec![Op::SetDocumentDiff(node(), epoch, None)]);
    p.read_with(cx, |p, _| {
        assert!(p.file_buttons.is_empty());
        assert!(p.file_visible.borrow().is_empty());
        assert!(p.file_suffixes.borrow().is_empty());
    });
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    eprintln!(
        "GPUIO_NATIVE_DIFF_RETENTION_OK: actual horizontal scroll, shaped suffix alignment/clip/extent, fixed gutter, streamed backwards selection and native clipboard, managed collapse remapping, bounded page header/focus retirement"
    );
}
