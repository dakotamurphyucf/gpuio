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
fn markdown_links_deliver_release_metadata_and_keyboard_without_opening_urls() {
    exercise_links(Mode::Markdown);
}
#[test]
fn html_links_deliver_release_metadata_and_keyboard_without_opening_urls() {
    exercise_links(Mode::Html);
}
fn exercise_links(mode: Mode) {
    let text = match mode {
        Mode::Markdown => "[example](https://example.test)",
        Mode::Html => "<a href=\"https://example.test\">example</a>",
        _ => unreachable!(),
    };
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
                    label: "Link input metadata".into(),
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

    let position = markdown.read_with(cx, |m, _| m.bounds().origin + gpui::point(px(10.), px(10.)));
    let release = gpui::Modifiers {
        shift: true,
        control: true,
        alt: true,
        platform: true,
        function: true,
    };
    for (button, expected) in [
        (gpui::MouseButton::Left, PointerButton::Left),
        (gpui::MouseButton::Middle, PointerButton::Middle),
        (gpui::MouseButton::Right, PointerButton::Right),
        (
            gpui::MouseButton::Navigate(gpui::NavigationDirection::Back),
            PointerButton::Back,
        ),
        (
            gpui::MouseButton::Navigate(gpui::NavigationDirection::Forward),
            PointerButton::Forward,
        ),
    ] {
        transport.mailbox.lock().unwrap().drain(128);
        cx.simulate_mouse_move(position, None, Default::default());
        cx.simulate_mouse_down(position, button, Default::default());
        cx.simulate_mouse_up(position, button, release);
        draw(cx);
        let events = transport.mailbox.lock().unwrap().drain(128);
        let navigations: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::DocumentNavigation(
                    _,
                    event_node,
                    _,
                    _,
                    event_source,
                    generation,
                    navigation,
                ) => {
                    assert_eq!(*event_node, node);
                    assert_eq!(*event_source, source);
                    assert_eq!(*generation, 1);
                    Some(navigation)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            navigations,
            vec![&Navigation::LinkActivated(
                "https://example.test".into(),
                Activation {
                    source: ActivationSource::Mouse(expected),
                    modifiers: super::super::pointer::modifiers(release)
                }
            )]
        );
    }
    cx.update(|window, cx| markdown.read(cx).focus_handle().clone().focus(window, cx));
    transport.mailbox.lock().unwrap().drain(128);
    cx.simulate_keystrokes("tab enter");
    draw(cx);
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|event| matches!(event,
        Event::DocumentNavigation(_, _, _, _, _, _, Navigation::LinkActivated(url, Activation { source: ActivationSource::Keyboard, modifiers }))
        if url == "https://example.test" && *modifiers == PointerModifiers::default()
    )), "keyboard navigation missing: {events:?}");
    assert_eq!(cx.opened_url(), None);

    // A superseded source cannot navigate using the installed old picture.
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Begin(Update {
                id: source,
                base: 1,
                revision: 2,
                generation: 2,
                from_byte: 0,
                suffix_bytes: 0,
                status: Status::Complete,
            })),
        Response::Ack
    );
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Publish(source, 2)),
        Response::Ack
    );
    transport.mailbox.lock().unwrap().drain(128);
    presentation.update(cx, |p, cx| {
        p.navigate(
            Navigation::LinkActivated(
                "https://example.test".into(),
                Activation {
                    source: ActivationSource::Keyboard,
                    modifiers: Default::default(),
                },
            ),
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
            .any(|event| matches!(event, Event::DocumentNavigation(..)))
    );
    apply(&view, cx, vec![Op::SetRoot(None), Op::Remove(node)]);
}

#[test]
fn touch_and_synthetic_keyboard_metadata_are_honest() {
    for long_press in [false, true] {
        let activation = link_activation(&gpui::ClickEvent::Touch(gpui::TouchClickEvent {
            long_press,
            ..Default::default()
        }));
        assert_eq!(activation.source, ActivationSource::Touch { long_press });
        assert_eq!(activation.modifiers, PointerModifiers::default());
    }
    assert_eq!(
        link_activation(&gpui::ClickEvent::default()).source,
        ActivationSource::Keyboard
    );
}
