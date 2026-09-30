//! CoreGraphics events addressed only to this process, through AppKit's queue.
//! Marked text uses the real NSTextInputClient callbacks, not an installed IME.
use super::*;
use std::{ffi::c_void, time::Duration};

type Raw = *const c_void;
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(source: Raw, key: u16, down: bool) -> Raw;
    fn CGEventSetFlags(event: Raw, flags: u64);
    fn CGEventPostToPid(pid: libc::pid_t, event: Raw);
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: Raw);
}

const COMMAND: u64 = 1 << 20;
const SHIFT: u64 = 1 << 17;

async fn key(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, code: u16, flags: u64) {
    // Keep both transitions targeted to our process even if the user changes
    // foreground applications. Never send keyboard events to the global tap.
    for down in [true, false] {
        unsafe {
            let event = CGEventCreateKeyboardEvent(std::ptr::null(), code, down);
            assert!(!event.is_null(), "cannot create native keyboard event");
            CGEventSetFlags(event, flags);
            CGEventPostToPid(std::process::id().try_into().unwrap(), event);
            CFRelease(event);
        }
    }
    cx.background_executor()
        .timer(Duration::from_millis(75))
        .await;
    frame(cx, window).await;
}

pub(super) async fn table(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    window
        .update(cx, |_, window, cx| {
            window.activate_window();
            cx.activate(true);
        })
        .unwrap();
    select(
        cx,
        window,
        Selection::Cell {
            row: RowKey(1),
            column: "name".into(),
        },
    );
    frame(cx, window).await;
    requests(cx, window);
    // activate_window queues an AppKit operation; manual layout frames above
    // do not yield to that queue. Do not post the first OS key until this
    // process's native window has actually become active.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !window.update(cx, |_, w, _| w.is_window_active()).unwrap() {
        assert!(
            std::time::Instant::now() < deadline,
            "table window did not activate before OS keyboard validation"
        );
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    key(cx, window, 125, 0).await; // Down
    assert_selection(cx, window, wire::Selection::Cell(2, "name".into()));
    key(cx, window, 124, 0).await; // Right
    assert_selection(cx, window, wire::Selection::Cell(2, "value".into()));
    requests(cx, window);
    key(cx, window, 36, 0).await; // Return
    key(cx, window, 109, SHIFT).await; // Shift-F10
    key(cx, window, 8, COMMAND).await; // Command-C
    assert_eq!(clipboard(cx), "日本語 row 2");
    assert_eq!(
        requests(cx, window),
        vec![
            wire::Request::Activate(2, Some("value".into())),
            wire::Request::Context(wire::Selection::Cell(2, "value".into())),
            wire::Request::Copy(wire::Selection::Cell(2, "value".into())),
        ]
    );
    // Restore the fixture for the separate GPUI-dispatch scenarios.
    select(cx, window, Selection::Empty);
    frame(cx, window).await;
    eprintln!("GPUIO_TABLE_APPKIT_KEYS_OK: targeted OS arrows, Return, Shift-F10 and Command-C");
}

fn snapshot(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) -> EditorSnapshot {
    window
        .update(cx, |view, window, cx| {
            view.editors[&node(63)].snapshot(window, cx)
        })
        .unwrap()
}

pub(super) async fn editor(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    use super::super::super::super::editor_test::native_text;
    assert!(snapshot(cx, window).focused);
    requests(cx, window);
    key(cx, window, 0, COMMAND).await; // Select the existing draft.
    native_text(cx, window, "に", true);
    assert_eq!(snapshot(cx, window).text, "に");
    assert!(snapshot(cx, window).composition.is_some());
    native_text(cx, window, "日本", true);
    assert_eq!(snapshot(cx, window).text, "日本");
    native_text(cx, window, "日本語👨‍👩‍👧‍👦", false);
    assert!(snapshot(cx, window).composition.is_none());
    assert_eq!(snapshot(cx, window).text, "日本語👨‍👩‍👧‍👦");
    key(cx, window, 51, 0).await; // Backspace deletes the joined grapheme.
    assert_eq!(snapshot(cx, window).text, "日本語");
    key(cx, window, 0, COMMAND).await;
    key(cx, window, 8, COMMAND).await;
    assert_eq!(clipboard(cx), "日本語");
    key(cx, window, 124, 0).await;
    key(cx, window, 36, 0).await;
    key(cx, window, 109, SHIFT).await;
    assert!(snapshot(cx, window).focused);
    assert!(
        requests(cx, window).is_empty(),
        "AppKit editor input leaked table actions"
    );
    eprintln!(
        "GPUIO_TABLE_APPKIT_EDITOR_OK: marked-text replacement and commit, joined-grapheme deletion, child clipboard/action priority"
    );
}
