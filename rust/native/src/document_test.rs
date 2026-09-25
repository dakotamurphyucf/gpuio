//! Real macOS/Linux GPUI document layout, source updates and resource teardown.
use super::*;
use crate::{session::Session, transport::Transport};
use gpuio_protocol::{
    WindowId,
    document::{Request, Response, Status, Update},
};
use std::{
    cell::RefCell,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};
static LAYOUT_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
fn draw(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    let start = std::time::Instant::now();
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    LAYOUT_US.fetch_add(
        start.elapsed().as_micros() as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
}
fn peak_rss_bytes() -> u64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) }, 0);
    #[cfg(target_os = "macos")]
    {
        usage.ru_maxrss as u64
    }
    #[cfg(not(target_os = "macos"))]
    {
        usage.ru_maxrss as u64 * 1024
    }
}
fn id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn publish(
    session: &mut Session,
    source: ResourceId,
    base: i64,
    generation: i64,
    from: usize,
    text: &str,
) {
    let update = Update {
        id: source,
        base,
        revision: base + 1,
        generation,
        from_byte: from as i64,
        suffix_bytes: text.len() as i64,
        status: Status::Streaming,
    };
    assert_eq!(
        session.document_request(Request::Begin(update)),
        Response::Ack
    );
    assert_eq!(
        session.document_request(Request::Chunk(
            source,
            base + 1,
            0,
            gpuio_protocol::asset::Chunk::new(text.as_bytes().to_vec()).unwrap()
        )),
        Response::Ack
    );
    assert_eq!(
        session.document_request(Request::Publish(source, base + 1)),
        Response::Ack
    );
}
fn configure(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    source: ResourceId,
    mode: gpuio_protocol::document::Mode,
    search: &str,
) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(id()).unwrap().revision();
            let mut operations = if base == 0 {
                vec![
                    Op::Create(node(), Kind::DocumentView, "".into(), None),
                    Op::SetRoot(Some(node())),
                ]
            } else {
                vec![]
            };
            operations.push(Op::SetDocument(
                node(),
                Config {
                    source: Some(source),
                    mode,
                    dark: false,
                    layout: Layout::Viewport(300.),
                    label: "Native document".into(),
                    path: None,
                    line_numbers: true,
                    initially_collapsed: false,
                    search: search.to_string(),
                    images: vec![],
                },
            ));
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: id(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn settle(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) -> Entity<Presentation> {
    for _ in 0..1000 {
        draw(cx, window);
        let value = window
            .update(cx, |view, _, cx| {
                view.documents[&node()]
                    .presentation
                    .as_ref()
                    .filter(|p| p.read(cx).ready)
                    .cloned()
            })
            .unwrap();
        if let Some(value) = value {
            for _ in 0..3 {
                draw(cx, window);
            }
            return value;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    panic!("document preparation did not settle in ten seconds");
}
async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: Rc<RefCell<Session>>,
    source: ResourceId,
) {
    use gpuio_protocol::document::Mode;
    let started = std::time::Instant::now();
    let text = "let greeting = \"λ 👨‍👩‍👧‍👦\"\n";
    publish(&mut session.borrow_mut(), source, 0, 1, 0, text);
    configure(cx, window, source, Mode::Code("ml".into()), "");
    let presentation = settle(cx, window).await;
    window
        .update(cx, |_, window, cx| {
            presentation.update(cx, |p, cx| {
                assert!(p.error.is_none(), "{:?}", p.error);
                assert_eq!(p.editor.read(cx).value().as_ref(), text);
                p.editor.update(cx, |editor, cx| {
                    assert!(editor.bridge_select(4, 12, cx));
                    window.focus(&editor.focus_handle(cx), cx);
                });
            });
        })
        .unwrap();
    // Route real GPUI key events through the production composite and Base
    // button handlers; no physical keyboard or active OS window is required.
    window
        .update(cx, |_, window, cx| {
            let focus = presentation.read(cx).buttons["document-collapse"].clone();
            window.focus(&focus, cx);
        })
        .unwrap();
    super::super::editor_test::key(cx, window, "enter");
    assert!(presentation.read_with(cx, |p, _| p.collapsed));
    let _ = settle(cx, window).await;
    super::super::editor_test::key(cx, window, "enter");
    assert!(!presentation.read_with(cx, |p, _| p.collapsed));
    let _ = settle(cx, window).await;
    super::super::editor_test::key(cx, window, "tab");
    window
        .update(cx, |_, window, cx| {
            assert!(presentation.read(cx).buttons["document-copy"].is_focused(window))
        })
        .unwrap();
    super::super::editor_test::key(cx, window, "tab");
    window
        .update(cx, |_, window, cx| {
            assert!(
                presentation
                    .read(cx)
                    .editor
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window)
            )
        })
        .unwrap();
    publish(
        &mut session.borrow_mut(),
        source,
        1,
        1,
        text.len(),
        "(* streaming *)\n",
    );
    window
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    let presentation = settle(cx, window).await;
    presentation.read_with(cx, |p, cx| {
        assert_eq!(p.editor.read(cx).bridge_selection(), (4, 12));
        assert_eq!(
            p.editor.read(cx).value().as_ref(),
            format!("{text}(* streaming *)\n")
        );
    });
    publish(
        &mut session.borrow_mut(),
        source,
        2,
        2,
        0,
        "# Markdown\n\n[link](https://example.test)\n\n|a|b|\n|-|-|\n|λ|x|\n\n```ml\nlet n = 1\n```\n\n![alt](asset://demo)\n\n![reference][picture]\n\n[picture]: asset://demo\n\n<img src=\"file:///must-not-open\">\n",
    );
    configure(cx, window, source, Mode::Markdown, "");
    let image = {
        let mut session = session.borrow_mut();
        let store = session.assets().unwrap();
        let mut bytes = b"P6\n4 4\n255\n".to_vec();
        for _ in 0..16 {
            bytes.extend([20, 100, 240]);
        }
        let image = store
            .begin(gpuio_protocol::asset::Format::Pnm, bytes.len())
            .unwrap();
        store.append(image, 0, &bytes).unwrap();
        store.finish(image).unwrap();
        image
    };
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(id()).unwrap().revision();
            let mut config = (**view
                .session
                .borrow()
                .tree(id())
                .unwrap()
                .get(node())
                .unwrap()
                .document
                .as_ref()
                .unwrap())
            .clone();
            config.images = vec![("asset://demo".into(), ImageSource::Reference(image))];
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
        })
        .unwrap();
    let markdown = settle(cx, window).await;
    markdown.read_with(cx, |p, cx| {
        assert!(p.error.is_none(), "{:?}", p.error);
        assert!(p.markdown.as_ref().unwrap().read(cx).bounds().size.height > px(0.));
    });
    for _ in 0..200 {
        let _ = settle(cx, window).await;
        if markdown.read_with(cx, |p, _| p.images.0.len() == 1) {
            break;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    markdown.read_with(cx, |p, _| {
        assert_eq!(p.images.0.len(), 1, "explicit registered image decoded")
    });
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(image)
        .unwrap();
    let selected = markdown.update(cx, |p, cx| {
        let state = p.markdown.as_ref().unwrap();
        state.update(cx, |state, cx| state.select_all(cx));
        state.read(cx).selected_text()
    });
    let bytes = session
        .borrow()
        .document(source)
        .unwrap()
        .snapshot()
        .text
        .len();
    publish(
        &mut session.borrow_mut(),
        source,
        3,
        2,
        bytes,
        " new streamed text",
    );
    window
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    let markdown = settle(cx, window).await;
    markdown.read_with(cx, |p, cx| {
        assert_eq!(
            p.markdown.as_ref().unwrap().read(cx).selected_text(),
            selected,
            "select-all is a snapshot; streamed bytes must not enter an existing selection"
        )
    });
    publish(
        &mut session.borrow_mut(),
        source,
        4,
        3,
        0,
        "--- a/a.ml\n+++ b/a.ml\n@@ -1 +1 @@\n-let a = 1\n+let a = 2\n",
    );
    configure(cx, window, source, Mode::Diff, "");
    let diff = settle(cx, window).await;
    diff.read_with(cx, |p, _| {
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!(p.diff.as_ref().unwrap().lines.len(), 5);
    });
    let huge = format!("{}last λ target", "row\n".repeat(25000));
    publish(&mut session.borrow_mut(), source, 5, 4, 0, &huge);
    configure(cx, window, source, Mode::Markdown, "last λ target");
    let huge_view = settle(cx, window).await;
    window
        .update(cx, |_, window, cx| {
            huge_view.update(cx, |p, cx| {
                assert!(p.error.is_some());
                assert!(p.editor.read(cx).value().len() < 65537);
                assert_eq!(p.search.total, 1);
                p.next_match(true, window, cx);
                assert_eq!(p.editor.read(cx).value().as_ref(), "last λ target");
                assert_eq!(
                    p.editor.read(cx).bridge_selection(),
                    (0, "last λ target".len())
                );
            })
        })
        .unwrap();
    drop(huge_view);
    // Registration release leaves a mounted lease readable, while unmount
    // drops presentations and their worker/cache reservations.
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    drop(presentation);
    drop(markdown);
    drop(diff);
    window
        .update(cx, |view, window, cx| {
            let revision = view.session.borrow().tree(id()).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: id(),
                    base: revision,
                    revision: revision + 1,
                    operations: vec![Op::SetRoot(None), Op::Remove(node())],
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            assert!(view.documents.is_empty());
        })
        .unwrap();
    cx.update(|cx| {
        eprintln!(
            "GPUIO_DOCUMENT_METRICS {:?}; test_elapsed_ms={}",
            crate::document_host::measurements(cx),
            started.elapsed().as_millis()
        )
    });
    eprintln!(
        "GPUIO_DOCUMENT_LAYOUT layout_paint_us={} process_peak_rss_bytes={}",
        LAYOUT_US.load(std::sync::atomic::Ordering::Relaxed),
        peak_rss_bytes()
    );
    eprintln!(
        "GPUIO_NATIVE_DOCUMENT_OK: code, Markdown/table/fence/safe image, diff, streaming Unicode selection and lease teardown; native layout/paint, no physical presentation claim"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, id(), "Document test", 800., 600.)
            .unwrap();
        let Response::Created(source) = session.borrow_mut().document_request(Request::Create)
        else {
            panic!("create");
        };
        let window = cx
            .open_window(
                gpui::WindowOptions {
                    focus: false,
                    window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::centered(
                        None,
                        gpui::size(px(800.), px(600.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id(), session.clone(), transport)),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result =
                super::super::native_test::protect(exercise(cx, window, session, source)).await;
            let _ = window.update(cx, |_, window, _| window.remove_window());
            crate::document_host::shutdown(cx).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(super::super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
