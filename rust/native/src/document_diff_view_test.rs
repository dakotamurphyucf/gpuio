//! Mounted projection checks through the retained tree and native read-only editor.
use super::*;
use gpuio_protocol::document_diff::{Collapse, Config as DiffConfig, FileKey, LineLimit};

const SOURCE: &str = "--- a/a.ml\n+++ b/a.ml\n@@ -1 +1 @@\n-old λ\n+new λ\n--- a/b.ml\n+++ b/b.ml\n@@ -1 +1 @@\n keep β\n";

fn settings(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, config: Option<DiffConfig>) {
    window
        .update(cx, |view, window, cx| {
            let (base, epoch) = {
                let session = view.session.borrow();
                let tree = session.tree(id()).unwrap();
                (
                    tree.revision(),
                    tree.get(node()).unwrap().document_diff_epoch + 1,
                )
            };
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: id(),
                    base,
                    revision: base + 1,
                    operations: vec![Op::SetDocumentDiff(node(), epoch, config)],
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
    draw(cx, window);
}
fn collapsed() -> DiffConfig {
    DiffConfig {
        collapse: Collapse::Controlled(vec![FileKey::Path("a.ml".into())]),
        word_diff: false,
        ..DiffConfig::default()
    }
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    let Response::Created(source) = session.borrow_mut().document_request(Request::Create) else {
        panic!("source")
    };
    publish(&mut session.borrow_mut(), source, 0, 1, 0, SOURCE);
    configure(
        cx,
        window,
        source,
        gpuio_protocol::document::Mode::Diff,
        "old λ",
    );
    settings(cx, window, Some(collapsed()));
    let p = settle(cx, window).await;
    let editor = p.read_with(cx, |p, cx| {
        let projection = p.projection.as_ref().unwrap();
        assert_eq!(projection.collapsed_body_lines(), 2);
        assert!(!p.editor.read(cx).value().contains("old λ"));
        assert!(p.editor.read(cx).value().contains("keep β"));
        assert!(p.projection_charge.is_some());
        assert_eq!(
            p.highlight_source().unwrap().unwrap().len(),
            p.editor.read(cx).value().len()
        );
        assert_eq!(p.installed.as_ref().unwrap().text.to_string(), SOURCE);
        p.editor.clone()
    });
    // Preserve backwards selection in a surviving file while preceding rows return.
    window
        .update(cx, |_, _, cx| {
            p.update(cx, |p, cx| {
                let text = p.editor.read(cx).value();
                let start = text.find("keep β").unwrap();
                p.editor.update(cx, |editor, cx| {
                    editor.bridge_select(start + "keep β".len(), start, cx)
                });
            })
        })
        .unwrap();
    settings(cx, window, Some(DiffConfig::default()));
    p.read_with(cx, |p, cx| {
        assert_eq!(
            p.editor, editor,
            "configuration does not remount the editor"
        );
        let text = p.editor.read(cx).value();
        let (a, b) = p.editor.read(cx).bridge_selection();
        assert!(a > b);
        assert_eq!(&text[b..a], "keep β");
        assert_eq!(
            p.navigation_at_caret(cx),
            Some(Navigation::Line(
                Some("b.ml".into()),
                gpuio_protocol::document::Side::After,
                1
            ))
        );
    });
    // A selection that newly crosses hidden rows must clear, never copy hidden text.
    window
        .update(cx, |_, _, cx| {
            p.update(cx, |p, cx| {
                let text = p.editor.read(cx).value();
                let end = text.find("keep β").unwrap() + "keep β".len();
                p.editor
                    .update(cx, |editor, cx| editor.bridge_select(0, end, cx));
            })
        })
        .unwrap();
    settings(cx, window, Some(collapsed()));
    p.read_with(cx, |p, cx| {
        assert_eq!(p.editor.read(cx).bridge_selection(), (0, 0))
    });
    // Canonical hidden search opens source; returning restores controlled values.
    window
        .update(cx, |_, window, cx| {
            p.update(cx, |p, cx| {
                p.next_match(true, window, cx);
                assert!(p.raw_diff);
                assert!(p.projected_page.is_none());
                assert!(p.editor.read(cx).value().starts_with("old λ"));
                assert_eq!(p.editor.read(cx).bridge_selection(), (0, "old λ".len()));
                p.raw_diff = false;
                p.page_start = 0;
                p.previous_pages.clear();
                p.show_page_for(p.installed.clone().unwrap(), None, true, window, cx);
                assert!(!p.editor.read(cx).value().contains("old λ"));
            })
        })
        .unwrap();
    settings(
        cx,
        window,
        Some(DiffConfig {
            line_limit: LineLimit::Controlled(Some(1)),
            ..DiffConfig::default()
        }),
    );
    p.read_with(cx, |p, cx| {
        assert_eq!(p.projection.as_ref().unwrap().shown_body_lines(), 1);
        assert_eq!(p.projection.as_ref().unwrap().hidden_body_lines(), 2);
        assert!(p.editor.read(cx).value().contains("old λ"));
        assert!(!p.editor.read(cx).value().contains("new λ"));
        assert!(!p.editor.read(cx).value().contains("b.ml"));
    });
    // Same-generation publication leaves the exact old picture interactive until installed.
    publish(
        &mut session.borrow_mut(),
        source,
        1,
        1,
        SOURCE.len(),
        "partial metadata λ\n",
    );
    window
        .update(cx, |_, window, cx| {
            p.update(cx, |p, cx| {
                p.refresh(p.config.clone(), p.markdown_options, cx);
                assert!(!p.ready);
                assert_eq!(p.installed.as_ref().unwrap().revision, 1);
                p.show_page(window, cx);
                assert_eq!(
                    p.installed.as_ref().unwrap().revision,
                    1,
                    "paging must not install unprepared new source"
                );
            })
        })
        .unwrap();
    settle(cx, window).await;
    p.read_with(cx, |p, _| {
        assert_eq!(p.installed.as_ref().unwrap().revision, 2)
    });
    // The bounded native page applies to projected text too.
    let large = format!(
        "--- a/large\n+++ b/large\n@@ -1,3000 +1,3000 @@\n{}",
        " unchanged λ\n".repeat(3000)
    );
    publish(&mut session.borrow_mut(), source, 2, 2, 0, &large);
    settings(cx, window, Some(DiffConfig::default()));
    settle(cx, window).await;
    window
        .update(cx, |_, window, cx| {
            p.update(cx, |p, cx| {
                assert!(p.page_end < p.display_bytes());
                assert!(p.editor.read(cx).value().len() <= 65536);
                assert!(p.source_lines <= 1024);
                let end = p.page_end;
                p.page_start = end;
                p.show_page(window, cx);
                assert_eq!(p.installed_page_start, end);
                assert!(p.page_end > end);
                assert_eq!(
                    p.navigation_at_caret(cx),
                    Some(Navigation::Line(
                        Some("large".into()),
                        gpuio_protocol::document::Side::After,
                        1021
                    ))
                );
            })
        })
        .unwrap();
    pending_preparation_keeps_installed_geometry(cx, window, &p);
    settings(cx, window, None);
    p.read_with(cx, |p, _| {
        assert!(p.projection.is_none() && p.projected_page.is_none());
        assert!(p.diff_controls.is_none() && p.projection_charge.is_none());
    });
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    eprintln!(
        "GPUIO_NATIVE_DIFF_PROJECTION_OK: controlled collapse/preview, retained editor, exact selection/navigation, canonical search and raw return, pending source fences, bounded projected pages and charge disposal; control activation acceptance remains pending"
    );
}
