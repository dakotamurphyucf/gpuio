//! Native show-more behavior, queued observations and retired-action rejection.
use super::*;
use gpuio_protocol::{
    HandlerId,
    document_diff::{Config as DiffConfig, LineLimit, Observation},
};
#[path = "document_diff_headers_test.rs"]
mod headers;
#[path = "document_diff_line_test.rs"]
mod lines;

const SOURCE: &str = "--- a/a.ml\n+++ b/a.ml\n@@ -1,3 +1,3 @@\n a\n b\n c\n";
fn apply(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(id()).unwrap().revision();
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
    draw(cx, window);
}
fn config(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, line_limit: LineLimit) {
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
                line_limit,
                ..DiffConfig::default()
            }),
        )],
    );
}
fn activate(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    p: &Entity<Presentation>,
    action: &DiffAction,
) {
    cx.update_window(window.into(), |_, window, cx| {
        p.update(cx, |p, cx| p.show_more_diff(action, window, cx))
    })
    .unwrap();
    draw(cx, window);
}
fn observations(transport: &Transport) -> Vec<gpuio_protocol::document_diff::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::DocumentDiffEvent(_, _, _, _, _, event) => Some(event),
            _ => None,
        })
        .collect()
}
async fn await_observations(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    transport: &Transport,
) -> Vec<gpuio_protocol::document_diff::Event> {
    for _ in 0..100 {
        let events = observations(transport);
        if !events.is_empty() {
            return events;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
        draw(cx, window);
    }
    panic!("native diff observation not delivered");
}
fn visible(p: &Entity<Presentation>, cx: &mut gpui::AsyncApp) -> usize {
    p.read_with(cx, |p, _| p.projection.as_ref().unwrap().shown_body_lines())
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
    configure(cx, window, source, gpuio_protocol::document::Mode::Diff, "");
    let handler = HandlerId::from_parts(77, 1).unwrap();
    apply(cx, window, vec![Op::Bind(node(), Some(handler))]);
    config(
        cx,
        window,
        LineLimit::Managed {
            initial: Some(1),
            step: 1,
        },
    );
    let p = settle(cx, window).await;
    let transport = window.update(cx, |v, _, _| v.transport.clone()).unwrap();
    observations(&transport);
    let stale = p.read_with(cx, |p, _| p.diff_action().unwrap());
    window
        .update(cx, |_, window, cx| {
            window.focus(&p.read(cx).buttons["document-diff-more"].clone(), cx)
        })
        .unwrap();
    super::super::super::editor_test::key(cx, window, "enter");
    draw(cx, window);
    assert_eq!(visible(&p, cx), 2);
    let events = observations(&transport);
    assert_eq!(events.len(), 1);
    assert_eq!(
        (events[0].source_revision, events[0].source_generation),
        (1, 1)
    );
    assert_eq!(
        events[0].observation,
        Observation::ShowMore {
            visible: 1,
            hidden: 2,
            applied_limit: Some(2)
        }
    );
    activate(cx, window, &p, &stale);
    assert_eq!(visible(&p, cx), 2);
    assert!(
        observations(&transport).is_empty(),
        "old rendered page cannot apply twice"
    );
    super::super::super::editor_test::key(cx, window, "space");
    draw(cx, window);
    assert_eq!(visible(&p, cx), 3);
    assert_eq!(observations(&transport).len(), 1);
    window
        .update(cx, |_, window, cx| {
            assert!(
                p.read(cx)
                    .editor
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window),
                "removed footer returns focus to editor"
            )
        })
        .unwrap();
    config(cx, window, LineLimit::Controlled(Some(1)));
    let current = p.read_with(cx, |p, _| p.diff_action().unwrap());
    #[cfg(target_os = "macos")]
    {
        // Enable accessibility, then draw the requested native AX tree.
        let _ =
            crate::host::control_test::accessible_button(cx, window, "Show more diff lines", false);
        draw(cx, window);
        assert!(crate::host::control_test::accessible_button(
            cx,
            window,
            "Show more diff lines",
            true
        ));
    }
    #[cfg(not(target_os = "macos"))]
    activate(cx, window, &p, &current);
    draw(cx, window);
    assert_eq!(
        visible(&p, cx),
        1,
        "controlled action cannot mutate the limit"
    );
    assert_eq!(
        await_observations(cx, window, &transport).await[0].observation,
        Observation::ShowMore {
            visible: 1,
            hidden: 2,
            applied_limit: None
        }
    );
    config(cx, window, LineLimit::Controlled(None));
    window
        .update(cx, |_, window, cx| {
            assert!(
                p.read(cx)
                    .editor
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window),
                "controlled acceptance repairs removed footer focus"
            )
        })
        .unwrap();
    config(cx, window, LineLimit::Controlled(Some(2)));
    activate(cx, window, &p, &current);
    assert!(
        observations(&transport).is_empty(),
        "old configuration is retired"
    );
    let current = p.read_with(cx, |p, _| p.diff_action().unwrap());
    apply(
        cx,
        window,
        vec![Op::Bind(
            node(),
            Some(HandlerId::from_parts(77, 2).unwrap()),
        )],
    );
    activate(cx, window, &p, &current);
    assert!(
        observations(&transport).is_empty(),
        "old callback binding is retired"
    );
    // Native-managed controls work without an application callback.
    apply(cx, window, vec![Op::Bind(node(), None)]);
    config(
        cx,
        window,
        LineLimit::Managed {
            initial: Some(1),
            step: 1,
        },
    );
    let point = p.read_with(cx, |p, _| p.diff_more_bounds.borrow().unwrap().center());
    crate::host::native_test::mouse(cx, window, point, true);
    crate::host::native_test::mouse(cx, window, point, false);
    draw(cx, window);
    assert_eq!(visible(&p, cx), 2);
    assert!(observations(&transport).is_empty());
    // Both whole-document collapse and source reset fence old displayed actions.
    let current = p.read_with(cx, |p, _| p.diff_action().unwrap());
    p.update(cx, |p, _| p.collapsed = true);
    activate(cx, window, &p, &current);
    assert_eq!(visible(&p, cx), 2);
    p.update(cx, |p, _| p.collapsed = false);
    publish(&mut session.borrow_mut(), source, 1, 2, 0, SOURCE);
    activate(cx, window, &p, &current);
    assert!(observations(&transport).is_empty());
    settle(cx, window).await;
    assert_eq!(
        visible(&p, cx),
        1,
        "new generation restores managed initial limit"
    );
    let current = p.read_with(cx, |p, _| p.diff_action().unwrap());
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    activate(cx, window, &p, &current);
    assert_eq!(
        visible(&p, cx),
        1,
        "released registration cannot mutate leased display"
    );
    let epoch = p.read_with(cx, |p, _| p.diff_epoch + 1);
    apply(
        cx,
        window,
        vec![
            Op::SetDocumentDiff(node(), epoch, None),
            Op::Bind(node(), None),
        ],
    );
    lines::exercise(cx, window, session).await;
    headers::exercise(cx, window, session).await;
    eprintln!(
        "GPUIO_NATIVE_DIFF_MORE_OK: pointer, Enter/Space, macOS AX press, managed local state and controlled queued intent, focus handoff, retired page/config/handler/reset/release rejection"
    );
}
