//! Retained production document on TestPlatform, not physical clipboard evidence.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::{
    WindowId,
    document::{Mode, Request, Response, Status, Update},
};
use std::{
    os::{fd::AsRawFd, unix::net::UnixStream},
    time::Duration,
};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(view: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        view.update(cx, |v, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, w, cx);
            cx.notify();
        })
    });
    draw(cx);
}

#[test]
fn document_copy_format_retains_native_selection_and_source_revision() {
    copy_format(Mode::Markdown, "Hello **世界** and `code`.\n");
}
#[test]
fn html_copy_selection_is_plain_and_preserves_original_source() {
    copy_format(
        Mode::Html,
        "<p>Hello <b>世界</b> and <code>code</code>.</p>\n",
    );
}
fn copy_format(mode: Mode, text: &str) {
    let html = matches!(mode, Mode::Html);
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window, "Document", 520., 320.)
        .unwrap();
    let Response::Created(source) = session.borrow_mut().document_request(Request::Create) else {
        panic!("source")
    };
    for request in [
        Request::Begin(Update {
            id: source,
            base: 0,
            revision: 1,
            generation: 1,
            from_byte: 0,
            suffix_bytes: text.len() as i64,
            status: Status::Complete,
        }),
        Request::Chunk(
            source,
            1,
            0,
            gpuio_protocol::asset::Chunk::new(text.as_bytes().to_vec()).unwrap(),
        ),
        Request::Publish(source, 1),
    ] {
        assert_eq!(
            session.borrow_mut().document_request(request),
            Response::Ack
        );
    }
    let (view, cx) = app.add_window_view(|_, _| View::new(window, session.clone(), transport));
    let node = NodeId::from_parts(0, 1).unwrap();
    apply(
        &view,
        cx,
        vec![
            Op::Create(node, Kind::DocumentView, "".into(), None),
            Op::SetDocument(
                node,
                Config {
                    source: Some(source),
                    mode,
                    dark: false,
                    layout: Layout::Viewport(200.),
                    label: "Copy formats".into(),
                    path: None,
                    line_numbers: false,
                    initially_collapsed: false,
                    search: "".into(),
                    images: vec![],
                },
            ),
            Op::SetRoot(Some(node)),
        ],
    );
    let presentation = view.read_with(cx, |v, _| v.documents[&node].presentation.clone().unwrap());
    for _ in 0..1000 {
        draw(cx);
        if presentation.read_with(cx, |p, _| p.markdown.is_some() && p.installed.is_some()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let markdown =
        presentation.read_with(cx, |p, _| p.markdown.clone().expect("prepared Markdown"));
    assert_eq!(
        presentation.read_with(cx, |p, _| p.installed.as_ref().unwrap().text.to_string()),
        text
    );
    markdown.update(cx, |m, cx| m.select_all(cx));
    let plain = markdown.read_with(cx, |m, _| m.selected_text());
    assert!(plain.contains("世界") && !plain.contains("**"), "{plain:?}");
    for source_copy in [true, false, true] {
        apply(
            &view,
            cx,
            vec![Op::SetDocumentSelectionFormat(node, source_copy)],
        );
        presentation.read_with(cx, |p, _| {
            assert_eq!(
                p.markdown.as_ref().unwrap().entity_id(),
                markdown.entity_id()
            );
            assert_eq!(p.installed.as_ref().unwrap().revision, 1);
        });
        assert_eq!(
            markdown.read_with(cx, |m, _| m.selected_text()),
            if source_copy && !html { text } else { &plain }
        );
        let copied = cx.update(gpui_base::TextSelection::selected_text);
        assert_eq!(
            copied,
            if source_copy && !html {
                text
            } else {
                plain.trim()
            },
            "the window copy provider must preserve source whitespace"
        );
    }
    let weak = presentation.downgrade();
    drop(presentation);
    drop(markdown);
    apply(&view, cx, vec![Op::SetRoot(None), Op::Remove(node)]);
    assert!(weak.upgrade().is_none());
    assert!(view.read_with(cx, |v, _| v.documents.is_empty()));
}
