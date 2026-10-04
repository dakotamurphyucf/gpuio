#[path = "document_frontmatter_test.rs"]
mod frontmatter;
// Retained production document on TestPlatform, not physical clipboard evidence.
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

pub(super) fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
pub(super) fn apply(view: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
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
fn markdown_options_install_atomically_and_clear_incompatible_selection() {
    let mode = Mode::Markdown;
    let text = "---\nname: Native 世界\n---\n\n<Card>Child **text** {1 + 2}</Card>\n";
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
    let (view, cx) =
        app.add_window_view(|_, _| View::new(window, session.clone(), transport.clone()));
    cx.simulate_resize(gpui::size(px(800.), px(1800.)));
    let node = NodeId::from_parts(0, 1).unwrap();
    apply(
        &view,
        cx,
        vec![
            Op::Create(
                node,
                Kind::DocumentView,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetDocument(
                node,
                Config {
                    source: Some(source),
                    mode,
                    dark: false,
                    layout: Layout::Flow,
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

    markdown.update(cx, |m, cx| m.select_all(cx));
    draw(cx);
    let before = markdown.read_with(cx, |m, _| m.displayed_text().unwrap());
    let selected = markdown.read_with(cx, |m, _| m.selected_text());
    assert!(selected.contains("<Card>"));
    let installed = presentation.read_with(cx, |p, _| p.installed.clone().unwrap());
    let old_interpretation = presentation.read_with(cx, |p, _| p.interpretation.clone());
    let options = gpuio_protocol::document::MarkdownOptions {
        frontmatter: gpuio_protocol::document::Frontmatter::CodeBlock,
        mdx: true,
    };
    // Pause installation while allowing the worker and renderer to proceed.
    presentation.update(cx, |p, _| p.defer_prepared_install = true);
    apply(
        &view,
        cx,
        vec![Op::SetDocumentMarkdownOptions(node, options)],
    );
    for _ in 0..3 {
        draw(cx);
    }
    assert!(Arc::ptr_eq(
        &before,
        &markdown.read_with(cx, |m, _| m.displayed_text().unwrap())
    ));
    assert_eq!(markdown.read_with(cx, |m, _| m.selected_text()), selected);
    presentation.read_with(cx, |p, _| {
        assert!(!p.ready);
        assert_eq!(p.markdown_options, options);
        assert_eq!(p.installed_markdown_options, Default::default());
    });
    presentation.update(cx, |p, _| p.defer_prepared_install = false);
    for _ in 0..1000 {
        draw(cx);
        if presentation.read_with(cx, |p, _| p.ready) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    presentation.read_with(cx, |p, _| {
        assert!(p.ready);
        assert_eq!(p.installed_markdown_options, options);
        assert!(Arc::ptr_eq(&installed, p.installed.as_ref().unwrap()));
        assert_eq!(
            p.markdown.as_ref().unwrap().entity_id(),
            markdown.entity_id()
        );
    });
    assert!(markdown.read_with(cx, |m, _| m.selected_text()).is_empty());
    // Replay the exact installed-link callback path with old and current pictures.
    transport.mailbox.lock().unwrap().drain(128);
    presentation.update(cx, |p, cx| {
        p.navigate_link(
            &old_interpretation,
            Some((1, 1)),
            &"gpuio:next".into(),
            &gpui::ClickEvent::default(),
            cx,
        )
    });
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|e| matches!(e, Event::DocumentNavigation(..)))
    );
    presentation.update(cx, |p, cx| {
        p.navigate_link(
            &p.interpretation.clone(),
            Some((1, 1)),
            &"gpuio:next".into(),
            &gpui::ClickEvent::default(),
            cx,
        )
    });
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|e| matches!(e, Event::DocumentNavigation(..)))
    );
    let after = markdown.read_with(cx, |m, _| m.displayed_text().unwrap());
    assert!(!Arc::ptr_eq(&before, &after));
    let fragments = after
        .fragments()
        .iter()
        .map(|f| f.text().as_ref())
        .collect::<Vec<_>>()
        .join("|");
    assert!(
        fragments.contains("Child text 1 + 2"),
        "MDX displays expression, never computes it: {fragments}"
    );
    assert!(!fragments.contains("<Card>"));
    assert!(fragments.contains("name: Native 世界"));
    // The main-thread renderer must not kick off a second parse with old flags.
    draw(cx);
    draw(cx);
    assert!(Arc::ptr_eq(
        &after,
        &markdown.read_with(cx, |m, _| m.displayed_text().unwrap())
    ));
    markdown.update(cx, |m, cx| m.select_all(cx));
    apply(
        &view,
        cx,
        vec![Op::SetDocumentMarkdownOptions(node, Default::default())],
    );
    for _ in 0..1000 {
        draw(cx);
        if presentation.read_with(cx, |p, _| p.ready) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(presentation.read_with(cx, |p, _| p.ready));
    assert!(markdown.read_with(cx, |m, _| m.selected_text()).is_empty());
    transport.mailbox.lock().unwrap().drain(128);
    presentation.update(cx, |p, cx| {
        p.navigate_link(
            &old_interpretation,
            Some((1, 1)),
            &"gpuio:next".into(),
            &gpui::ClickEvent::default(),
            cx,
        )
    });
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|e| matches!(e, Event::DocumentNavigation(..))),
        "A/B/A must not revive old link callbacks"
    );
    let restored = markdown.read_with(cx, |m, _| m.displayed_text().unwrap());
    assert!(
        restored
            .fragments()
            .iter()
            .any(|f| f.text().contains("<Card>"))
    );
    apply(
        &view,
        cx,
        vec![Op::SetDocumentMarkdownOptions(
            node,
            gpuio_protocol::document::MarkdownOptions {
                frontmatter: gpuio_protocol::document::Frontmatter::DescriptionList,
                mdx: false,
            },
        )],
    );
    for _ in 0..1000 {
        draw(cx);
        if presentation.read_with(cx, |p, _| p.ready) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    presentation.read_with(cx, |p, _| {
        assert!(p.ready);
        assert!(Arc::ptr_eq(&installed, p.installed.as_ref().unwrap()));
        assert_eq!(
            p.markdown.as_ref().unwrap().entity_id(),
            markdown.entity_id()
        );
    });
    let displayed = markdown.read_with(cx, |m, _| m.displayed_text().unwrap());
    assert!(
        displayed
            .fragments()
            .iter()
            .any(|f| f.text().as_ref() == "name:")
    );
    assert!(
        displayed
            .fragments()
            .iter()
            .any(|f| f.text().as_ref() == "Native 世界")
    );
    // Redrawing must retain the installed description configuration too.
    draw(cx);
    assert!(Arc::ptr_eq(
        &displayed,
        &markdown.read_with(cx, |m, _| m.displayed_text().unwrap())
    ));
    let weak = presentation.downgrade();
    drop(presentation);
    drop(markdown);
    apply(&view, cx, vec![Op::SetRoot(None), Op::Remove(node)]);
    assert!(weak.upgrade().is_none());
}
