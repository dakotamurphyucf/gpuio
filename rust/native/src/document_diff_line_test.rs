//! Rich line observations through actual mounted pointer/key/accessibility paths.
use super::*;
use gpuio_protocol::document_diff::{Collapse, FileKey, Line};
const PATCH: &str = "--- a/hidden\n+++ b/hidden\n@@ -1 +1 @@\n-old\n+new\n--- a/世界.ml\n+++ b/世界.ml\n@@ -7,2 +9,2 @@\n-old λ\r\n+new 🦀\r\n\\ No newline annotation\n keep β";
fn select(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    p: &Entity<Presentation>,
    text: &str,
    end: bool,
) {
    window
        .update(cx, |_, window, cx| {
            let editor = p.read(cx).editor.clone();
            editor.update(cx, |editor, cx| {
                let start = editor.value().find(text).unwrap() + if end { text.len() } else { 0 };
                editor.bridge_select(start, start, cx);
                window.focus(&editor.focus_handle(cx), cx);
            });
        })
        .unwrap();
    draw(cx, window);
}
fn line(transport: &Transport, text: &str, before: Option<i64>, after: Option<i64>) -> Line {
    let events = observations(transport);
    assert_eq!(events.len(), 1, "one rich line observation");
    assert_eq!(
        (events[0].source_revision, events[0].source_generation),
        (1, 1)
    );
    let Observation::Line(line) = &events[0].observation else {
        panic!("line")
    };
    assert_eq!(line.file.index, 1);
    assert_eq!(line.file.key, FileKey::Path("世界.ml".into()));
    assert_eq!((line.before, line.after), (before, after));
    assert_eq!(line.text, text);
    assert_eq!(
        &PATCH[line.start_byte as usize..line.end_byte as usize],
        text
    );
    line.clone()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    let Response::Created(source) = session.borrow_mut().document_request(Request::Create) else {
        panic!("source")
    };
    publish(&mut session.borrow_mut(), source, 0, 1, 0, PATCH);
    configure(cx, window, source, gpuio_protocol::document::Mode::Diff, "");
    let epoch = window
        .update(cx, |v, _, _| {
            v.session
                .borrow()
                .tree(id())
                .unwrap()
                .get(node())
                .unwrap()
                .document_diff_epoch
                + 1
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Bind(node(), Some(HandlerId::from_parts(79, 1).unwrap())),
            Op::SetDocumentDiff(
                node(),
                epoch,
                Some(DiffConfig {
                    collapse: Collapse::Controlled(vec![FileKey::Path("hidden".into())]),
                    ..DiffConfig::default()
                }),
            ),
        ],
    );
    let p = settle(cx, window).await;
    let transport = window.update(cx, |v, _, _| v.transport.clone()).unwrap();
    observations(&transport);
    select(cx, window, &p, "old λ", false);
    crate::host::editor_test::key(cx, window, "enter");
    line(&transport, "old λ", Some(7), None);
    let point = p.read_with(cx, |p, cx| {
        let editor = p.editor.read(cx);
        let start = editor.value().find("new 🦀").unwrap();
        editor
            .range_to_bounds(&(start..start + "new 🦀".len()))
            .unwrap()
            .center()
    });
    crate::host::native_test::mouse(cx, window, point, true);
    crate::host::native_test::mouse(cx, window, point, false);
    draw(cx, window);
    line(&transport, "new 🦀", None, Some(9));
    // Native gutter controls and text selection must not become line activation.
    let header = p.read_with(cx, |p, cx| {
        let editor = p.editor.read(cx);
        let start = editor.value().find("@@").unwrap();
        editor.range_to_bounds(&(start..start + 2)).unwrap()
    });
    let gutter = gpui::point(header.left() - px(19.), header.center().y);
    p.read_with(cx, |p, _| p.editor.clone())
        .update(cx, |editor, cx| {
            editor.bridge_select(0, 0, cx);
        });
    crate::host::native_test::move_mouse(cx, window, gutter, false);
    draw(cx, window);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(10))
        .await;
    for step in 0..2 {
        crate::host::native_test::mouse(cx, window, gutter, true);
        crate::host::native_test::mouse(cx, window, gutter, false);
        draw(cx, window);
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
        draw(cx, window);
        assert!(
            observations(&transport).is_empty(),
            "gutter fold is not a line click"
        );
        let body_visible = p.read_with(cx, |p, cx| {
            let editor = p.editor.read(cx);
            let start = editor.value().find("new 🦀").unwrap();
            editor
                .range_to_bounds(&(start..start + "new 🦀".len()))
                .is_some()
        });
        assert_eq!(
            body_visible,
            step == 1,
            "final nonterminated hunk actually folds and expands"
        );
    }
    let points = p.read_with(cx, |p, cx| {
        let editor = p.editor.read(cx);
        ["old λ", "keep β"].map(|text| {
            let start = editor.value().find(text).unwrap();
            editor
                .range_to_bounds(&(start..start + text.len()))
                .unwrap()
                .center()
        })
    });
    crate::host::native_test::mouse(cx, window, points[0], true);
    crate::host::native_test::move_mouse(cx, window, points[1], true);
    crate::host::native_test::mouse(cx, window, points[1], false);
    draw(cx, window);
    assert!(p.read_with(cx, |p, cx| !p.editor.read(cx).selected_range().is_empty()));
    assert!(
        observations(&transport).is_empty(),
        "selection drag is not line activation"
    );
    // EOF belongs to the final nonterminated body row, not a synthetic blank row.
    select(cx, window, &p, "keep β", true);
    crate::host::editor_test::key(cx, window, "enter");
    line(&transport, "keep β", Some(8), Some(10));
    select(cx, window, &p, "No newline annotation", false);
    #[cfg(target_os = "macos")]
    {
        let _ = crate::host::control_test::accessible_button(cx, window, "Go to line", false);
        draw(cx, window);
        assert!(crate::host::control_test::accessible_button(
            cx,
            window,
            "Go to line",
            true
        ));
        // AX dispatch is asynchronous; retain the received event for the shared checker.
        let events = await_observations(cx, window, &transport).await;
        assert_eq!(events.len(), 1);
        let Observation::Line(payload) = &events[0].observation else {
            panic!("line")
        };
        assert_eq!((payload.before, payload.after), (None, None));
        assert_eq!(payload.text, "No newline annotation");
        assert_eq!(
            &PATCH[payload.start_byte as usize..payload.end_byte as usize],
            payload.text
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        crate::host::editor_test::key(cx, window, "enter");
        line(&transport, "No newline annotation", None, None);
    }
    select(cx, window, &p, "@@", false);
    crate::host::editor_test::key(cx, window, "enter");
    assert!(
        observations(&transport).is_empty(),
        "metadata header is not a line observation"
    );
    select(cx, window, &p, "keep β", false);
    publish(&mut session.borrow_mut(), source, 1, 1, PATCH.len(), "\n");
    p.update(cx, |p, cx| p.refresh(p.config.clone(), cx));
    assert!(!p.read_with(cx, |p, _| p.ready));
    crate::host::editor_test::key(cx, window, "enter");
    line(&transport, "keep β", Some(8), Some(10));
    settle(cx, window).await;
    assert_eq!(
        p.read_with(cx, |p, _| p.installed.as_ref().unwrap().revision),
        2
    );
    let old = p.read_with(cx, |p, _| p.diff_action().unwrap());
    config(cx, window, LineLimit::Controlled(Some(0)));
    cx.update_window(window.into(), |_, _, cx| {
        p.update(cx, |p, cx| assert!(!p.observe_diff_line(&old, None, cx)))
    })
    .unwrap();
    assert!(
        observations(&transport).is_empty(),
        "retired projection cannot activate old line"
    );
    apply(
        cx,
        window,
        vec![
            Op::SetDocumentDiff(node(), epoch + 2, None),
            Op::Bind(node(), None),
        ],
    );
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    eprintln!(
        "GPUIO_NATIVE_DIFF_LINES_OK: native Enter, measured pointer, AX toolbar; canonical Unicode/CRLF text and old/new coordinates, projected file index, annotation/EOF semantics and retired-line rejection"
    );
}
