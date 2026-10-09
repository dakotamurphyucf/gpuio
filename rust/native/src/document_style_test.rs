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
fn document_style_reflows_without_reparsing_or_losing_selection() {
    let mode = Mode::Markdown;
    let text = "# Heading\n\nFirst `inline` paragraph.\n\nSecond paragraph.\n\n```txt\ncode block\n```\n\n| A | B |\n|---|---|\n| cell | body |\n";
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
    cx.simulate_resize(gpui::size(px(800.), px(1800.)));
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
    let selected = markdown.read_with(cx, |m, _| m.selected_text());
    assert!(selected.contains("Heading"));
    let installed = presentation.read_with(cx, |p, _| p.installed.clone().unwrap());
    let prepared = markdown.read_with(cx, |m, _| m.displayed_text().unwrap());
    let initial = markdown.read_with(cx, |m, _| m.bounds().size.height);
    let part = |color| {
        vec![Style::Fields(vec![Field::Background(Fill::Solid(
            Color::Rgba(color),
        ))])]
    };
    let config = gpuio_protocol::document_style::Config {
        paragraph_gap_rem: Some(4.),
        heading_sizes: Some(gpuio_protocol::document_style::HeadingSizes {
            h1: 80.,
            h2: 60.,
            h3: 40.,
            h4: 30.,
            h5: 24.,
            h6: 20.,
        }),
        colors: vec![(
            gpuio_protocol::document_style::Part::Link,
            Color::Rgba(0x8844ccff),
        )],
        code_block: part(0x116633ff),
        table: part(0x442277ff),
        table_head: part(0x993311ff),
        table_cell: part(0x227799ff),
        inline_code: gpuio_protocol::document_style::InlineCode {
            background: Some(Color::Rgba(0xaa2266ff)),
            font_weight: Some(700),
            italic: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };
    apply(
        &view,
        cx,
        vec![Op::SetDocumentTextStyle(node, Some(config.clone()))],
    );
    draw(cx);
    cx.update(|window, _| {
        let quads = window.painted_quads();
        for rgba in [0x116633ff, 0x442277ff, 0x993311ff, 0x227799ff, 0xaa2266ff] {
            let background: gpui::Background = gpui::rgba(rgba).into();
            assert!(
                quads.iter().any(|quad| quad.background == background),
                "internal part paint missing: {rgba:08x}"
            );
        }
    });
    let enlarged = markdown.read_with(cx, |m, _| m.bounds().size.height);
    assert!(
        enlarged > initial,
        "reader geometry must reflect internal metrics: {initial:?} -> {enlarged:?}"
    );
    assert_eq!(markdown.read_with(cx, |m, _| m.selected_text()), selected);
    presentation.read_with(cx, |p, _| {
        assert!(Arc::ptr_eq(&installed, p.installed.as_ref().unwrap()));
        assert_eq!(
            p.markdown.as_ref().unwrap().entity_id(),
            markdown.entity_id()
        );
        assert!(p.ready, "styling must not submit a parser job");
    });
    apply(&view, cx, vec![Op::SetDocumentTextStyle(node, None)]);
    draw(cx);
    assert_eq!(
        markdown.read_with(cx, |m, _| m.bounds().size.height),
        initial
    );
    assert_eq!(markdown.read_with(cx, |m, _| m.selected_text()), selected);
    // A virtualized viewport can retain old row heights even when visible
    // blocks use the new style. Check the actual scroll extent, not just bounds.
    let mut viewport = presentation.read_with(cx, |p, _| (*p.config).clone());
    viewport.layout = Layout::Viewport(150.);
    apply(&view, cx, vec![Op::SetDocument(node, viewport)]);
    draw(cx);
    let list = markdown.read_with(cx, |m, _| m.list_state().clone());
    let before = list.max_offset_for_scrollbar().y;
    assert!(before > px(0.));
    markdown.update(cx, |m, cx| m.select_all(cx));
    draw(cx);
    let selected = markdown.read_with(cx, |m, _| m.selected_text());
    apply(
        &view,
        cx,
        vec![Op::SetDocumentTextStyle(node, Some(config))],
    );
    draw(cx);
    let after = list.max_offset_for_scrollbar().y;
    assert!(
        after > before,
        "virtualized reader scroll extent stayed stale: {before:?} -> {after:?}"
    );
    assert_eq!(markdown.read_with(cx, |m, _| m.selected_text()), selected);
    apply(&view, cx, vec![Op::SetDocumentTextStyle(node, None)]);
    draw(cx);
    assert_eq!(list.max_offset_for_scrollbar().y, before);
    assert_eq!(markdown.read_with(cx, |m, _| m.selected_text()), selected);
    assert!(
        Arc::ptr_eq(
            &prepared,
            &markdown.read_with(cx, |m, _| m.displayed_text().unwrap())
        ),
        "style/layout updates must retain the exact prepared text allocation"
    );
    let weak = presentation.downgrade();
    drop(presentation);
    drop(markdown);
    apply(&view, cx, vec![Op::SetRoot(None), Op::Remove(node)]);
    assert!(weak.upgrade().is_none());
    assert!(view.read_with(cx, |v, _| v.documents.is_empty()));
}
