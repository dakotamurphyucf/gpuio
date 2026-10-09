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
fn preview_changes_preserve_the_prepared_snapshot_and_queue_presentation_states() {
    preview(Mode::Markdown);
}
#[test]
fn html_preview_preserves_the_prepared_snapshot_and_queues_presentation_states() {
    preview(Mode::Html);
}
fn preview(mode: Mode) {
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
    let text = if matches!(mode, Mode::Html) {
        "<p>Hello <b>世界</b>.</p><p>Second paragraph.</p><p>Third paragraph.</p><p>Fourth.</p>"
    } else {
        "Hello **世界**.\n\nSecond paragraph.\n\nThird paragraph.\n\nFourth.\n"
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
    let node = NodeId::from_parts(0, 1).unwrap();
    apply(
        &view,
        cx,
        vec![
            Op::Create(
                node,
                Kind::DocumentView,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(1, 1).unwrap()),
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
            Op::SetDocumentPreview(
                node,
                gpuio_protocol::document_preview::Config {
                    epoch: 1,
                    max_lines: Some(2),
                    observe: true,
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
    use gpuio_protocol::document_preview::{Config as Preview, State as PreviewState};
    let events = || {
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .into_iter()
            .filter_map(|event| match event {
                Event::DocumentPreviewObserved(_, _, _, _, _, event) => Some(event),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    draw(cx);
    let initial = events();
    assert!(
        initial
            .iter()
            .any(|event| event.config_epoch == 1 && event.state == PreviewState::Rich(true)),
        "{initial:?}"
    );
    let installed = presentation.read_with(cx, |p, _| p.installed.clone().unwrap());
    apply(
        &view,
        cx,
        vec![Op::SetDocumentPreview(
            node,
            Preview {
                epoch: 2,
                max_lines: None,
                observe: true,
            },
        )],
    );
    let expanded = events();
    assert_eq!(expanded.len(), 1, "{expanded:?}");
    assert_eq!(expanded[0].state, PreviewState::Rich(false));
    presentation.read_with(cx, |p, _| {
        assert_eq!(
            p.markdown.as_ref().unwrap().entity_id(),
            markdown.entity_id()
        );
        assert!(Arc::ptr_eq(p.installed.as_ref().unwrap(), &installed));
    });
    draw(cx);
    draw(cx);
    assert!(events().is_empty());
    presentation.update(cx, |p, cx| {
        p.collapsed = true;
        cx.notify();
    });
    draw(cx);
    assert_eq!(events().last().unwrap().state, PreviewState::Collapsed);
    presentation.update(cx, |p, cx| {
        p.collapsed = false;
        cx.notify();
    });
    draw(cx);
    assert_eq!(events().last().unwrap().state, PreviewState::Rich(false));
    apply(
        &view,
        cx,
        vec![Op::SetDocumentPreview(
            node,
            Preview {
                epoch: 3,
                max_lines: Some(2),
                observe: false,
            },
        )],
    );
    assert!(events().is_empty());
    assert!(markdown.read_with(cx, |m, _| m.is_clamped()));
    apply(
        &view,
        cx,
        vec![Op::SetDocumentPreview(
            node,
            Preview {
                epoch: 4,
                max_lines: Some(2),
                observe: true,
            },
        )],
    );
    assert_eq!(events().last().unwrap().state, PreviewState::Rich(true));
    cx.update(|window, cx| {
        presentation.update(cx, |p, cx| {
            p.show_source(vec![], window, cx);
            cx.notify();
        })
    });
    draw(cx);
    assert_eq!(events().last().unwrap().state, PreviewState::SourceView);
    apply(
        &view,
        cx,
        vec![Op::SetDocumentPreview(
            node,
            Preview {
                epoch: 5,
                max_lines: None,
                observe: true,
            },
        )],
    );
    assert_eq!(events().last().unwrap().state, PreviewState::SourceView);
    let observation = gpuio_protocol::document_preview::Event {
        config_epoch: 5,
        source_revision: 1,
        source_generation: 1,
        state: PreviewState::SourceView,
    };
    let handler = gpuio_protocol::HandlerId::from_parts(1, 1).unwrap();
    assert!(
        session
            .borrow()
            .document_preview_observed(window, node, handler, source, observation.clone())
            .is_some()
    );
    for stale in [
        gpuio_protocol::document_preview::Event {
            config_epoch: 4,
            ..observation.clone()
        },
        gpuio_protocol::document_preview::Event {
            source_generation: 2,
            ..observation.clone()
        },
        gpuio_protocol::document_preview::Event {
            source_revision: 2,
            ..observation.clone()
        },
    ] {
        assert!(
            session
                .borrow()
                .document_preview_observed(window, node, handler, source, stale)
                .is_none()
        );
    }
    assert!(
        session
            .borrow()
            .document_preview_observed(
                window,
                node,
                gpuio_protocol::HandlerId::from_parts(1, 2).unwrap(),
                source,
                observation
            )
            .is_none()
    );
    // A source replacement and window resize can change overflow without a
    // new preview epoch. The observation must describe the new installed text.
    let replacement = "A wrapping preview with enough words to occupy several narrow lines while fitting a wide document body. ".repeat(3);
    for request in [
        Request::Begin(Update {
            id: source,
            base: 1,
            revision: 2,
            generation: 2,
            from_byte: 0,
            suffix_bytes: replacement.len() as i64,
            status: Status::Complete,
        }),
        Request::Chunk(
            source,
            2,
            0,
            gpuio_protocol::asset::Chunk::new(replacement.into_bytes()).unwrap(),
        ),
        Request::Publish(source, 2),
    ] {
        assert_eq!(
            session.borrow_mut().document_request(request),
            Response::Ack
        );
    }
    presentation.update(cx, |p, cx| {
        p.source_mode = false;
        cx.notify();
    });
    cx.simulate_resize(gpui::size(px(220.), px(600.)));
    apply(
        &view,
        cx,
        vec![Op::SetDocumentPreview(
            node,
            Preview {
                epoch: 6,
                max_lines: Some(3),
                observe: true,
            },
        )],
    );
    for _ in 0..1000 {
        draw(cx);
        if presentation.read_with(cx, |p, _| {
            p.installed.as_ref().is_some_and(|s| s.generation == 2)
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    draw(cx);
    let narrow = events();
    assert!(
        narrow.iter().any(|e| e.config_epoch == 6
            && e.source_generation == 2
            && e.source_revision == 2
            && e.state == PreviewState::Rich(true)),
        "{narrow:?}"
    );
    cx.simulate_resize(gpui::size(px(2000.), px(600.)));
    draw(cx);
    let wide = events();
    assert!(
        wide.iter().any(|e| e.config_epoch == 6
            && e.source_generation == 2
            && e.state == PreviewState::Rich(false)),
        "{wide:?}"
    );
    // Source fallback must expose the same page interval and reason that a
    // sighted reader sees. Use the production tree and platform AX action path.
    let large = "A native source line · λ 世界\n".repeat(5000);
    for request in [
        Request::Begin(Update {
            id: source,
            base: 2,
            revision: 3,
            generation: 3,
            from_byte: 0,
            suffix_bytes: large.len() as i64,
            status: Status::Complete,
        }),
        Request::Chunk(
            source,
            3,
            0,
            gpuio_protocol::asset::Chunk::new(large.clone().into_bytes()).unwrap(),
        ),
        Request::Publish(source, 3),
    ] {
        assert_eq!(
            session.borrow_mut().document_request(request),
            Response::Ack
        );
    }
    let mut config = presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(
        &view,
        cx,
        vec![
            Op::SetDocumentPreview(
                node,
                Preview {
                    epoch: 7,
                    max_lines: None,
                    observe: false,
                },
            ),
            Op::SetDocument(node, config),
        ],
    );
    cx.simulate_a11y_active(true);
    for _ in 0..1000 {
        draw(cx);
        if presentation.read_with(cx, |p, _| {
            p.installed.as_ref().is_some_and(|s| s.revision == 3)
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let assert_labels = |cx: &mut VisualTestContext| {
        draw(cx);
        let (label, error) = presentation.read_with(cx, |p, _| {
            assert_eq!(p.installed.as_ref().unwrap().revision, 3);
            assert!(p.source_mode);
            (
                format!(
                    "Source bytes {}–{} of {}",
                    p.page_start,
                    p.page_end,
                    large.len()
                ),
                p.error.clone().expect("explicit fallback reason"),
            )
        });
        let tree = cx.a11y_tree().unwrap();
        for expected in [label, error] {
            assert!(
                tree.nodes.iter().any(|(_, n)| n.role() == gpui::Role::Label
                    && (n.label() == Some(expected.as_str())
                        || n.value() == Some(expected.as_str()))),
                "visible document metadata missing from accessibility: {expected}"
            );
        }
    };
    assert_labels(cx);
    let next = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, n)| n.role() == gpui::Role::Button && n.label() == Some("Next source page"))
        .unwrap()
        .0;
    let first_end = presentation.read_with(cx, |p, _| p.page_end);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: next,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert_eq!(presentation.read_with(cx, |p, _| p.page_start), first_end);
    assert_labels(cx);
    let weak = presentation.downgrade();
    drop(presentation);
    drop(markdown);
    apply(&view, cx, vec![Op::SetRoot(None), Op::Remove(node)]);
    assert!(weak.upgrade().is_none());
    assert!(events().is_empty());
}
