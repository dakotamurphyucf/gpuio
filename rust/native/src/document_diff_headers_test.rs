//! File headers share the source editor's layout, lifetime and queued events.
use super::*;
use gpuio_protocol::document_diff::{Collapse, FileKey};

#[path = "document_diff_retention_test.rs"]
mod retention;

const PATCH: &str = "--- a/世界.ml\n+++ b/世界.ml\n@@ -1 +1 @@\n-old λ\n+new 🦀\n--- a/second.ml\n+++ b/second.ml\n@@ -1 +1 @@\n keep β\n";

fn settings(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, collapse: Collapse) {
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
        vec![Op::SetDocumentDiff(
            node(),
            epoch,
            Some(DiffConfig {
                collapse,
                ..DiffConfig::default()
            }),
        )],
    );
}
fn click_header(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    p: &Entity<Presentation>,
    index: usize,
) {
    let point = p.read_with(cx, |p, _| p.file_visible.borrow()[&index].center());
    crate::host::native_test::mouse(cx, window, point, true);
    crate::host::native_test::mouse(cx, window, point, false);
    draw(cx, window);
}
fn assert_toggle(
    events: &[gpuio_protocol::document_diff::Event],
    index: usize,
    collapsed: bool,
    applied: bool,
) {
    assert_eq!(
        events.len(),
        1,
        "header produces one toggle, no line action"
    );
    let Observation::ToggleFile {
        file,
        collapsed: actual_collapsed,
        applied: actual_applied,
    } = &events[0].observation
    else {
        panic!("file toggle expected: {events:?}")
    };
    assert_eq!(file.index, index as i64);
    assert_eq!(
        file.key,
        FileKey::Path(if index == 0 { "世界.ml" } else { "second.ml" }.into())
    );
    assert_eq!((*actual_collapsed, *actual_applied), (collapsed, applied));
}
fn focus_header(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    p: &Entity<Presentation>,
    index: usize,
) {
    window
        .update(cx, |_, window, cx| {
            window.focus(&p.read(cx).file_buttons[&index].1.clone(), cx)
        })
        .unwrap();
    draw(cx, window);
}
fn admission(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    window
        .update(cx, |_, window, cx| {
            let input = cx.new(|cx| EditorState::new(window, cx).soft_wrap(false));
            input.update(cx, |input, cx| {
                let entries = |row, width| {
                    Some(Rc::new(BTreeMap::from([(
                        row,
                        gpui_base::input::RowAdornment {
                            gutter: None,
                            suffix: None,
                            suffix_width: px(width),
                        },
                    )])))
                };
                assert!(
                    input.set_row_adornments(entries(0, 10.), None, cx).is_err(),
                    "editable inputs reject row widgets"
                );
                input.set_readonly(true, cx);
                input.set_soft_wrap(true, window, cx);
                assert!(
                    input.set_row_adornments(entries(0, 10.), None, cx).is_err(),
                    "wrapped inputs reject fixed row widgets"
                );
                input.set_soft_wrap(false, window, cx);
                for width in [f32::NAN, f32::INFINITY, -1., 1025.] {
                    assert!(
                        input
                            .set_row_adornments(entries(0, width), None, cx)
                            .is_err()
                    );
                }
                assert!(
                    input.set_row_adornments(entries(10, 1.), None, cx).is_err(),
                    "row must belong to current source"
                );
                input.bridge_replace_all("row\n".repeat(1025).into(), (0, 0), false, window, cx);
                let rows = (0..1025)
                    .map(|row| {
                        (
                            row,
                            gpui_base::input::RowAdornment {
                                gutter: None,
                                suffix: None,
                                suffix_width: px(1.),
                            },
                        )
                    })
                    .collect();
                assert!(
                    input
                        .set_row_adornments(Some(Rc::new(rows)), None, cx)
                        .is_err(),
                    "row map is bounded"
                );
                assert!(
                    input
                        .set_row_adornments(entries(0, 1024.), None, cx)
                        .is_ok()
                );
                input.set_readonly(false, cx);
                assert!(
                    input.set_row_adornments(None, None, cx).is_ok(),
                    "clearing is always allowed"
                );
            });
        })
        .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    admission(cx, window);
    let Response::Created(source) = session.borrow_mut().document_request(Request::Create) else {
        panic!("source")
    };
    publish(&mut session.borrow_mut(), source, 0, 1, 0, PATCH);
    configure(cx, window, source, gpuio_protocol::document::Mode::Diff, "");
    apply(
        cx,
        window,
        vec![Op::Bind(
            node(),
            Some(HandlerId::from_parts(81, 1).unwrap()),
        )],
    );
    settings(cx, window, Collapse::Managed(vec![]));
    let p = settle(cx, window).await;
    let transport = window.update(cx, |v, _, _| v.transport.clone()).unwrap();
    observations(&transport);
    let editor = p.read_with(cx, |p, cx| {
        assert_eq!(p.file_visible.borrow().len(), 2);
        assert_eq!(
            p.editor.read(cx).value(),
            PATCH,
            "adornments never replace source bytes"
        );
        p.editor.clone()
    });
    // Actual Tab traversal enters only currently visible header controls.
    window
        .update(cx, |_, window, cx| {
            window.focus(&p.read(cx).buttons["document-location"].clone(), cx)
        })
        .unwrap();
    for index in 0..2 {
        crate::host::editor_test::key(cx, window, "tab");
        draw(cx, window);
        window
            .update(cx, |_, window, cx| {
                assert!(
                    p.read(cx).file_buttons[&index].1.is_focused(window),
                    "Tab reaches header {index}"
                )
            })
            .unwrap();
    }
    crate::host::editor_test::key(cx, window, "tab");
    window
        .update(cx, |_, window, cx| {
            assert!(editor.read(cx).focus_handle(cx).is_focused(window))
        })
        .unwrap();
    let retired = p.read_with(cx, |p, _| p.diff_action().unwrap());
    click_header(cx, window, &p, 0);
    assert_toggle(&observations(&transport), 0, true, true);
    assert!(!editor.read_with(cx, |e, _| e.value()).contains("old λ"));
    assert!(editor.read_with(cx, |e, _| e.value()).contains("keep β"));
    // A callback retained from the prior page cannot undo the collapse.
    cx.update_window(window.into(), |_, window, cx| {
        p.update(cx, |p, cx| p.toggle_diff_file(&retired, 0, window, cx))
    })
    .unwrap();
    assert!(observations(&transport).is_empty());
    focus_header(cx, window, &p, 0);
    crate::host::editor_test::key(cx, window, "space");
    draw(cx, window);
    assert_toggle(&observations(&transport), 0, false, true);
    assert_eq!(editor.read_with(cx, |e, _| e.value()), PATCH);
    crate::host::editor_test::key(cx, window, "enter");
    draw(cx, window);
    assert_toggle(&observations(&transport), 0, true, true);
    settings(cx, window, Collapse::Controlled(vec![]));
    #[cfg(target_os = "macos")]
    {
        let _ = crate::host::control_test::accessible_button(
            cx,
            window,
            "Collapse file second.ml",
            false,
        );
        draw(cx, window);
        assert!(crate::host::control_test::accessible_button(
            cx,
            window,
            "Collapse file second.ml",
            true
        ));
        assert_toggle(
            &await_observations(cx, window, &transport).await,
            1,
            true,
            false,
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        click_header(cx, window, &p, 1);
        assert_toggle(&observations(&transport), 1, true, false);
    }
    assert_eq!(
        editor.read_with(cx, |e, _| e.value()),
        PATCH,
        "controlled intent cannot change source projection"
    );
    // Callback replacement must refresh retained header actions even with the same epoch.
    apply(
        cx,
        window,
        vec![Op::Bind(
            node(),
            Some(HandlerId::from_parts(81, 2).unwrap()),
        )],
    );
    click_header(cx, window, &p, 1);
    assert_toggle(&observations(&transport), 1, true, false);
    focus_header(cx, window, &p, 1);
    settings(
        cx,
        window,
        Collapse::Controlled(vec![FileKey::Path("second.ml".into())]),
    );
    window
        .update(cx, |_, window, cx| {
            assert!(
                p.read(cx).file_buttons[&1].1.is_focused(window),
                "same file retains focus across controlled acceptance"
            )
        })
        .unwrap();
    assert!(!editor.read_with(cx, |e, _| e.value()).contains("keep β"));
    // Managed state works without an OCaml callback.
    apply(cx, window, vec![Op::Bind(node(), None)]);
    settings(cx, window, Collapse::Managed(vec![]));
    click_header(cx, window, &p, 1);
    assert!(!editor.read_with(cx, |e, _| e.value()).contains("keep β"));
    assert!(observations(&transport).is_empty());

    // Scroll positions come from this frame's actual shaped source rows.
    let long = format!(
        "--- a/first.ml\n+++ b/first.ml\n@@ -1,80 +1,80 @@\n{}--- a/last.ml\n+++ b/last.ml\n@@ -1 +1 @@\n end\n",
        " context\n".repeat(80)
    );
    publish(&mut session.borrow_mut(), source, 1, 2, 0, &long);
    settle(cx, window).await;
    assert!(p.read_with(cx, |p, _| p.file_visible.borrow().contains_key(&0)));
    assert!(!p.read_with(cx, |p, _| p.file_visible.borrow().contains_key(&1)));
    // Both directions must clamp before the first painted frame, not repair
    // empty row geometry on a later frame. Finish at the last header for the
    // stale offscreen-action and focus-retirement checks below.
    for (offset, visible, hidden, needle) in [
        (gpui::point(px(-70.), px(-10000.)), 1, 0, "--- a/last.ml"),
        (gpui::point(px(70.), px(10000.)), 0, 1, "--- a/first.ml"),
        (gpui::point(px(0.), px(-10000.)), 1, 0, "--- a/last.ml"),
    ] {
        editor.update(cx, |e, cx| e.set_scroll_offset(offset, cx));
        draw(cx, window);
        let geometry = |p: &Presentation, cx: &gpui::App| {
            assert!(
                !p.file_visible.borrow().contains_key(&hidden),
                "offscreen headers leave the native interaction map"
            );
            let e = p.editor.read(cx);
            let start = e.value().find(needle).unwrap();
            let text = e.range_to_bounds(&(start..start + 3));
            let button = *p.file_visible.borrow().get(&visible).unwrap_or_else(|| {
                panic!(
                    "diff header {visible} missing: requested={offset:?}, offset={:?}, input={:?}, text={text:?}, visible={:?}",
                    e.scroll_offset(),
                    e.input_bounds(),
                    p.file_visible.borrow()
                )
            });
            let text = text.expect("visible header has shaped source bounds");
            assert!(
                (f32::from(button.center().y - text.center().y)).abs() < 1.,
                "header and source row share vertical geometry"
            );
            assert!(button.intersects(&e.input_bounds()));
            assert_eq!(
                e.scroll_offset().x,
                px(0.),
                "short source cannot scroll horizontally"
            );
            (e.scroll_offset(), text, button)
        };
        let first_frame = p.read_with(cx, geometry);
        draw(cx, window);
        assert_eq!(
            p.read_with(cx, geometry),
            first_frame,
            "no follow-up frame jump"
        );
    }
    // A stale offscreen activation is rejected without changing native state.
    let current = p.read_with(cx, |p, _| p.diff_action().unwrap());
    cx.update_window(window.into(), |_, window, cx| {
        p.update(cx, |p, cx| p.toggle_diff_file(&current, 0, window, cx))
    })
    .unwrap();
    assert_eq!(editor.read_with(cx, |e, _| e.value()), long);
    // Clearing the feature repairs focused header ownership and disposes adornments.
    focus_header(cx, window, &p, 1);
    let epoch = p.read_with(cx, |p, _| p.diff_epoch + 1);
    apply(cx, window, vec![Op::SetDocumentDiff(node(), epoch, None)]);
    window
        .update(cx, |_, window, cx| {
            let p = p.read(cx);
            assert!(p.file_buttons.is_empty());
            assert!(p.file_visible.borrow().is_empty());
            assert!(p.editor.read(cx).focus_handle(cx).is_focused(window));
            assert_eq!(
                p.editor, editor,
                "same retained editor across header lifecycle"
            );
        })
        .unwrap();
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    retention::exercise(cx, window, session).await;
    eprintln!(
        "GPUIO_NATIVE_DIFF_HEADERS_OK: measured pointer, Tab/Space/Enter, macOS AX press; managed and controlled per-file collapse, callback refresh, current-frame scrolling, stale page/offscreen rejection and focus cleanup"
    );
}
