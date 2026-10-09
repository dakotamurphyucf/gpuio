//! macOS pointer activation measured through the native accessibility frames.
use super::*;
use objc2::{
    msg_send,
    runtime::{AnyObject, Bool},
};
use objc2_foundation::{NSPoint, NSRect, NSString};

unsafe fn button_frames(
    object: *mut AnyObject,
    label: &str,
    depth: usize,
    frames: &mut Vec<NSRect>,
) {
    if object.is_null() || depth > 16 {
        return;
    }
    unsafe {
        let title: *mut NSString = msg_send![object, accessibilityTitle];
        let role: *mut NSString = msg_send![object, accessibilityRole];
        if !title.is_null()
            && !role.is_null()
            && (*title).to_string() == label
            && (*role).to_string() == "AXButton"
        {
            frames.push(msg_send![object, accessibilityFrame]);
        }
        let children: *mut AnyObject = msg_send![object, accessibilityChildren];
        if children.is_null() {
            return;
        }
        let count: usize = msg_send![children, count];
        assert!(count < 2048);
        for index in 0..count {
            button_frames(
                msg_send![children, objectAtIndex: index],
                label,
                depth + 1,
                frames,
            );
        }
    }
}
async fn click_button(cx: &mut AsyncApp, handle: WindowHandle<View>, label: &str) -> String {
    let mut frames = Vec::new();
    for _ in 0..30 {
        frames.clear();
        // Reacquire after each await; do not retain a raw native view across frames.
        let view = crate::host::editor_test::native_view(cx, handle) as *mut AnyObject;
        unsafe {
            let window: *mut AnyObject = msg_send![view, window];
            let content: *mut AnyObject = msg_send![window, contentView];
            button_frames(content, label, 0, &mut frames);
        }
        if !frames.is_empty() {
            break;
        }
        frame(cx, handle).await;
    }
    assert_eq!(frames.len(), 1, "one native {label} button");
    let rect = frames[0];
    assert!(rect.size.width > 0. && rect.size.height > 0.);
    let view = crate::host::editor_test::native_view(cx, handle) as *mut AnyObject;
    let position = unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let screen = NSPoint::new(
            rect.origin.x + rect.size.width / 2.,
            rect.origin.y + rect.size.height / 2.,
        );
        let point: NSPoint = msg_send![window, convertPointFromScreen: screen];
        let point: NSPoint =
            msg_send![view, convertPoint: point, fromView: std::ptr::null::<AnyObject>()];
        let bounds: NSRect = msg_send![view, bounds];
        let flipped: Bool = msg_send![view, isFlipped];
        let y = if flipped.as_bool() {
            point.y - bounds.origin.y
        } else {
            bounds.origin.y + bounds.size.height - point.y
        };
        gpui::point(px((point.x - bounds.origin.x) as f32), px(y as f32))
    };
    let viewport = handle.update(cx, |_, w, _| w.viewport_size()).unwrap();
    assert!(
        position.x >= px(0.)
            && position.x < viewport.width
            && position.y >= px(0.)
            && position.y < viewport.height,
        "{label} must be visible: {position:?}/{viewport:?}"
    );
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("unchanged".into())));
    move_mouse(cx, handle, position, false);
    mouse(cx, handle, position, true);
    mouse(cx, handle, position, false);
    frame(cx, handle).await;
    cx.update(|cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap()
    })
}
pub(super) async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    let mut base = session
        .borrow()
        .document(source)
        .unwrap()
        .snapshot()
        .revision;
    for (mode, source_text, action) in [
        (
            Mode::Markdown,
            "```ocaml\nlet x = \"β\"\n```\n",
            Some(("Copy code", "let x = \"β\"")),
        ),
        (
            Mode::Markdown,
            "| A | B |\n| --- | --- |\n| α | β |\n",
            Some(("Copy table", "| A | B |\n| :-- | :-- |\n| α | β |")),
        ),
        (Mode::Code("ocaml".into()), "let x = \"β\"\n", None),
        (
            Mode::Diff,
            "--- a/test\n+++ b/test\n@@ -1 +1 @@\n-α\n+β\n",
            None,
        ),
    ] {
        publish(&mut session.borrow_mut(), source, base, source_text);
        base += 1;
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        let mut config = document(source, mode);
        config.layout = Layout::Viewport(230.);
        apply(
            cx,
            handle,
            vec![
                Op::SetDocument(node(1), config),
                Op::SetStyle(node(1), vec![Style::Fields(vec![Field::UserSelect(false)])]),
            ],
        );
        installed_revision(cx, handle, transport, p, base).await;
        frame(cx, handle).await;
        assert_eq!(
            click_button(cx, handle, "Copy source").await,
            source_text,
            "pointer Copy source preserves exact Unicode/newline bytes with selection disabled"
        );
        if let Some((label, expected)) = action {
            assert_eq!(
                click_button(cx, handle, label).await,
                expected,
                "pointer block Copy works with selection disabled"
            );
        }
    }
    apply(cx, handle, vec![Op::SetStyle(node(1), vec![])]);
    eprintln!(
        "GPUIO_DOCUMENT_POINTER_COPY_OK: measured native AX frames, pointer Copy source in Markdown/code/diff, fence code and normalized table Copy while User_select=false"
    );
}
