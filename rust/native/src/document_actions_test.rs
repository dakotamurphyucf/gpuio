#[path = "document_flow_selection_test.rs"]
mod flow_selection;

// Production mounted action rows on TestPlatform, not physical OS input evidence.
use super::markdown_options_test::{apply, draw};
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::{
    WindowId,
    document::{Mode, Request, Response, Status, Update},
    document_actions::{Action, Block, Config as Actions, Event as ActionEvent},
};
use std::{
    os::{fd::AsRawFd, unix::net::UnixStream},
    time::Duration,
};
const MARKDOWN: &str =
    "Visible\n\nfiller\n\n```ml\nlet answer = 42\n```\n\n| Name |\n| --- |\n| value |\n";
const HTML: &str = "<p>Visible</p><p>filler</p><pre><code>let answer = 42</code></pre><table><tr><th>Name</th></tr><tr><td>value</td></tr></table>";
struct Fixture {
    view: Entity<View>,
    presentation: Entity<Presentation>,
    transport: Arc<Transport>,
    node: NodeId,
    source: ResourceId,
    _reader: UnixStream,
}
fn actions(epoch: i64, enabled: bool) -> Actions {
    Actions {
        epoch,
        observe: true,
        copy_code: true,
        copy_table: true,
        code: vec![
            Action {
                id: "inspect".into(),
                label: "Inspect code".into(),
                enabled,
            },
            Action {
                id: "disabled".into(),
                label: "Unavailable action".into(),
                enabled: false,
            },
        ],
        table: vec![Action {
            id: "export".into(),
            label: "Export table".into(),
            enabled: true,
        }],
    }
}
fn ready(p: &Entity<Presentation>, cx: &mut VisualTestContext) {
    for _ in 0..1000 {
        draw(cx);
        if p.read_with(cx, |p, _| p.ready) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("document preparation did not settle");
}
fn mount(app: &mut TestAppContext, mode: Mode) -> (Fixture, &mut VisualTestContext) {
    let text = if mode == Mode::Html { HTML } else { MARKDOWN };
    mount_source(app, mode, text)
}
fn mount_source<'a>(
    app: &'a mut TestAppContext,
    mode: Mode,
    text: &str,
) -> (Fixture, &'a mut VisualTestContext) {
    app.update(gpui_base::init);
    let (reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window, "Actions", 800., 1200.)
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
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetDocument(
                node,
                Config {
                    source: Some(source),
                    mode,
                    dark: false,
                    layout: Layout::Flow,
                    label: "Actions".into(),
                    path: None,
                    line_numbers: false,
                    initially_collapsed: false,
                    search: String::new(),
                    images: vec![],
                },
            ),
            Op::SetDocumentActions(node, actions(1, true)),
            Op::SetRoot(Some(node)),
        ],
    );
    let presentation = view.read_with(cx, |v, _| v.documents[&node].presentation.clone().unwrap());
    ready(&presentation, cx);
    cx.simulate_a11y_active(true);
    draw(cx);
    (
        Fixture {
            view,
            presentation,
            transport,
            node,
            source,
            _reader: reader,
        },
        cx,
    )
}
fn events(f: &Fixture) -> Vec<ActionEvent> {
    f.transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::DocumentAction(_, _, _, _, _, event) => Some(event),
            _ => None,
        })
        .collect()
}
fn ax(cx: &mut VisualTestContext, label: &str, action: gpui::accesskit::Action) {
    let tree = cx.a11y_tree().unwrap();
    let target = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label))
        .unwrap_or_else(|| panic!("missing {label}"))
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action,
        target_node: target,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
}
fn assert_focus(cx: &VisualTestContext, label: &str) {
    let tree = cx.a11y_tree().unwrap();
    assert_eq!(
        tree.nodes
            .iter()
            .find(|(id, _)| *id == tree.focus)
            .and_then(|(_, n)| n.label()),
        Some(label),
        "actual accessibility focus {:?}",
        tree.focus
    );
}
#[test]
fn document_actions_route_real_native_pointer_keyboard_and_accessibility() {
    for mode in [Mode::Markdown, Mode::Html] {
        let html = mode == Mode::Html;
        let mut app = TestAppContext::single();
        let (f, cx) = mount(&mut app, mode);
        events(&f);
        let tree = cx.a11y_tree().unwrap();
        let rect = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Inspect code"))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        let scale = cx.update(|w, _| f64::from(w.scale_factor()));
        cx.simulate_click(
            gpui::point(
                px(((rect.x0 + rect.x1) / (2. * scale)) as f32),
                px(((rect.y0 + rect.y1) / (2. * scale)) as f32),
            ),
            gpui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        draw(cx);
        let clicked = events(&f);
        assert_eq!(clicked.len(), 1);
        let code = &clicked[0];
        assert_eq!(code.action, "inspect");
        assert_eq!((code.source_generation, code.source_revision), (1, 1));
        assert!(matches!(code.block,Block::Code(_,ref text) if text=="let answer = 42"));
        assert!(matches!(
            code.activation.source,
            gpuio_protocol::document::ActivationSource::Mouse(_)
        ));
        assert!(code.activation.modifiers.shift);
        assert_eq!(code.source_range.is_none(), html);
        if let Some(span) = &code.source_range {
            assert!(
                MARKDOWN[span.start_byte as usize..span.end_byte as usize].starts_with("```ml")
            );
        }
        cx.update(|window, _| window.activate_window());
        draw(cx);
        ax(cx, "Inspect code", gpui::accesskit::Action::Focus);
        assert_focus(cx, "Inspect code");
        cx.update(|window, _| window.activate_window());
        let keystroke = gpui::Keystroke::parse("enter").unwrap();
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
        draw(cx);
        let keyboard = events(&f);
        assert_eq!(keyboard.len(), 1);
        assert_eq!(
            keyboard[0].activation.source,
            gpuio_protocol::document::ActivationSource::Keyboard
        );
        // Traverse the actual nested native tab stops, skipping disabled
        // controls, and return to the text owner's logical link navigation.
        ax(cx, "Document content", gpui::accesskit::Action::Focus);
        for label in ["Copy code", "Inspect code", "Copy table", "Export table"] {
            cx.simulate_keystrokes("tab");
            draw(cx);
            assert_focus(cx, label);
        }
        for label in [
            "Copy table",
            "Inspect code",
            "Copy code",
            "Document content",
        ] {
            cx.simulate_keystrokes("shift-tab");
            draw(cx);
            assert_focus(cx, label);
        }
        ax(cx, "Export table", gpui::accesskit::Action::Click);
        let table = events(&f);
        assert_eq!(table.len(), 1);
        assert_eq!(table[0].action, "export");
        assert_eq!(table[0].source_range.is_none(), html);
        assert!(
            matches!(&table[0].block,Block::Table(headers,rows,markdown) if headers==&["Name"] && rows==&[vec!["value"]] && markdown.contains("value"))
        );
        ax(cx, "Unavailable action", gpui::accesskit::Action::Click);
        assert!(events(&f).is_empty());
        let weak = f.presentation.downgrade();
        let Fixture {
            view,
            presentation,
            node,
            ..
        } = f;
        drop(presentation);
        apply(&view, cx, vec![Op::SetRoot(None), Op::Remove(node)]);
        assert!(weak.upgrade().is_none());
    }
}
#[test]
fn document_actions_retain_text_and_fence_configuration_interpretation_and_source() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    let owner = f.presentation.update(cx, |p, cx| p.action_owner(cx));
    ax(cx, "Inspect code", gpui::accesskit::Action::Click);
    let original = events(&f).pop().unwrap();
    let markdown = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    markdown.update(cx, |m, cx| m.select_all(cx));
    draw(cx);
    let selected = markdown.read_with(cx, |m, _| m.selected_text());
    let displayed = markdown.read_with(cx, |m, _| m.displayed_text().unwrap());
    let source = f
        .presentation
        .read_with(cx, |p, _| p.installed.clone().unwrap());
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentActions(f.node, actions(2, false))],
    );
    assert_eq!(markdown.read_with(cx, |m, _| m.selected_text()), selected);
    assert!(Arc::ptr_eq(
        &displayed,
        &markdown.read_with(cx, |m, _| m.displayed_text().unwrap())
    ));
    assert!(f.presentation.read_with(cx, |p, _| p.ready
        && Arc::ptr_eq(&source, p.installed.as_ref().unwrap())));
    ax(cx, "Inspect code", gpui::accesskit::Action::Click);
    assert!(events(&f).is_empty());
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentActions(f.node, actions(3, true))],
    );
    cx.update(|_, cx| {
        owner.invoke(
            Some("inspect"),
            &original.block,
            original.source_range.clone(),
            &gpui::ClickEvent::default(),
            cx,
        )
    });
    assert!(events(&f).is_empty());
    let owner = f.presentation.update(cx, |p, cx| p.action_owner(cx));
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentMarkdownOptions(
            f.node,
            gpuio_protocol::document::MarkdownOptions {
                frontmatter: gpuio_protocol::document::Frontmatter::CodeBlock,
                mdx: false,
            },
        )],
    );
    ready(&f.presentation, cx);
    cx.update(|_, cx| {
        owner.invoke(
            Some("inspect"),
            &original.block,
            original.source_range.clone(),
            &gpui::ClickEvent::default(),
            cx,
        )
    });
    assert!(events(&f).is_empty());
    let owner = f.presentation.update(cx, |p, cx| p.action_owner(cx));
    f.view.update(cx, |view, _| {
        for request in [
            Request::Begin(Update {
                id: f.source,
                base: 1,
                revision: 2,
                generation: 2,
                from_byte: 0,
                suffix_bytes: MARKDOWN.len() as i64,
                status: Status::Complete,
            }),
            Request::Chunk(
                f.source,
                2,
                0,
                gpuio_protocol::asset::Chunk::new(MARKDOWN.as_bytes().to_vec()).unwrap(),
            ),
            Request::Publish(f.source, 2),
        ] {
            assert_eq!(
                view.session.borrow_mut().document_request(request),
                Response::Ack
            );
        }
    });
    cx.update(|_, cx| {
        owner.invoke(
            Some("inspect"),
            &original.block,
            original.source_range.clone(),
            &gpui::ClickEvent::default(),
            cx,
        )
    });
    assert!(
        events(&f).is_empty(),
        "generation reset retires old picture before new parse"
    );
}
#[test]
fn document_actions_preview_clip_disables_accessibility_and_restores_on_expand() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentPreview(
            f.node,
            gpuio_protocol::document_preview::Config {
                epoch: 1,
                max_lines: Some(2),
                observe: false,
            },
        )],
    );
    events(&f);
    ax(cx, "Inspect code", gpui::accesskit::Action::Click);
    ax(cx, "Export table", gpui::accesskit::Action::Click);
    assert!(events(&f).is_empty());
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentPreview(
            f.node,
            gpuio_protocol::document_preview::Config {
                epoch: 2,
                max_lines: None,
                observe: false,
            },
        )],
    );
    ax(cx, "Inspect code", gpui::accesskit::Action::Click);
    assert_eq!(events(&f).len(), 1);
}

#[test]
fn document_actions_reflow_and_copy_toggles_preserve_prepared_selection() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    cx.simulate_resize(gpui::size(px(400.), px(1800.)));
    draw(cx);
    let text = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    text.update(cx, |m, cx| m.select_all(cx));
    draw(cx);
    let selected = text.read_with(cx, |m, _| m.selected_text());
    let prepared = text.read_with(cx, |m, _| m.displayed_text().unwrap());
    let before = text.read_with(cx, |m, _| m.bounds().size.height);
    let tree = cx.a11y_tree().unwrap();
    let code_bounds = tree
        .nodes
        .iter()
        .find(|(_, n)| n.value() == Some("let answer = 42"))
        .expect("code text semantics")
        .1
        .bounds()
        .unwrap();
    for label in ["Copy code", "Inspect code", "Unavailable action"] {
        let bounds = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(label))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        assert!(
            bounds.y0 >= code_bounds.y1,
            "{label} overlaps code: {bounds:?} {code_bounds:?}"
        );
    }
    let mut expanded = actions(2, true);
    expanded.code = (0..16)
        .map(|i| Action {
            id: format!("a{i}"),
            label: format!("Action number {i}"),
            enabled: true,
        })
        .collect();
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentActions(f.node, expanded.clone())],
    );
    let after = text.read_with(cx, |m, _| m.bounds().size.height);
    assert!(
        after > before,
        "wrapped action rows must grow: {before:?} -> {after:?}"
    );
    assert_eq!(text.read_with(cx, |m, _| m.selected_text()), selected);
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentActions(f.node, actions(3, true))],
    );
    assert_eq!(text.read_with(cx, |m, _| m.bounds().size.height), before);
    assert!(Arc::ptr_eq(
        &prepared,
        &text.read_with(cx, |m, _| m.displayed_text().unwrap())
    ));
    ax(cx, "Copy code", gpui::accesskit::Action::Click);
    assert_eq!(
        cx.update(|_, cx| cx.read_from_clipboard().unwrap().text().unwrap()),
        "let answer = 42"
    );
    assert!(
        events(&f).is_empty(),
        "native copy must not enqueue a custom action"
    );
    let mut hidden = actions(4, true);
    hidden.copy_code = false;
    hidden.copy_table = false;
    apply(&f.view, cx, vec![Op::SetDocumentActions(f.node, hidden)]);
    let tree = cx.a11y_tree().unwrap();
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| matches!(n.label(), Some("Copy code" | "Copy table")))
    );
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentActions(f.node, actions(5, true))],
    );
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(240.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    let list = text.read_with(cx, |m, _| m.list_state().clone());
    let before = list.max_offset_for_scrollbar().y;
    expanded.epoch = 6;
    apply(&f.view, cx, vec![Op::SetDocumentActions(f.node, expanded)]);
    let after = list.max_offset_for_scrollbar().y;
    assert!(
        after > before,
        "virtual row heights must reflect actions: {before:?} -> {after:?}"
    );
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentActions(f.node, actions(7, true))],
    );
    assert_eq!(list.max_offset_for_scrollbar().y, before);
    assert!(Arc::ptr_eq(
        &prepared,
        &text.read_with(cx, |m, _| m.displayed_text().unwrap())
    ));
}

#[path = "document_profile_test.rs"]
mod profiles;

#[path = "document_focus_test.rs"]
mod focus_test;
