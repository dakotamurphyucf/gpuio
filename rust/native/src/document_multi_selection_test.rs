//! Independent prepared documents share window selection without sharing owners.
use super::*;

fn bounds(cx: &mut AsyncApp, p: &Entity<Presentation>) -> gpui::Bounds<gpui::Pixels> {
    p.read_with(cx, |p, cx| p.markdown.as_ref().unwrap().read(cx).bounds())
}
fn focus(cx: &mut AsyncApp, handle: WindowHandle<View>, p: &Entity<Presentation>) {
    handle
        .update(cx, |_, window, cx| {
            let focus = p
                .read(cx)
                .markdown
                .as_ref()
                .unwrap()
                .read(cx)
                .focus_handle()
                .clone();
            window.focus(&focus, cx);
        })
        .unwrap();
}
fn endpoints(
    cx: &mut AsyncApp,
    first: &Entity<Presentation>,
    last: &Entity<Presentation>,
) -> (gpui::Point<gpui::Pixels>, gpui::Point<gpui::Pixels>) {
    let a = bounds(cx, first);
    let b = bounds(cx, last);
    assert!(
        a.bottom() <= b.top(),
        "independent document bodies are vertically ordered"
    );
    (
        gpui::point(a.left() + px(1.), a.top() + px(10.)),
        gpui::point(b.right() - px(1.), b.top() + px(10.)),
    )
}
pub(super) async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    first: &Entity<Presentation>,
) {
    let base = session
        .borrow()
        .document(source)
        .unwrap()
        .snapshot()
        .revision;
    publish(
        &mut session.borrow_mut(),
        source,
        base,
        "first **α** body\n",
    );
    handle
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    let Response::Created(second_source) = session.borrow_mut().document_request(Request::Create)
    else {
        panic!("second document source")
    };
    publish(
        &mut session.borrow_mut(),
        second_source,
        0,
        "second **β** body\n",
    );
    let mut config = document(source, Mode::Markdown);
    config.layout = Layout::Viewport(80.);
    let mut second_config = config.clone();
    second_config.source = Some(second_source);
    apply(
        cx,
        handle,
        vec![
            Op::SetDocument(node(1), config),
            Op::Create(node(6), Kind::DocumentView, "".into(), None),
            Op::SetDocument(node(6), second_config),
            Op::Splice(node(0), 1, 0, vec![node(6)]),
        ],
    );
    let second = handle
        .update(cx, |view, _, _| {
            view.documents[&node(6)].presentation.clone().unwrap()
        })
        .unwrap();
    installed_revision(cx, handle, transport, first, base + 1).await;
    installed_revision(cx, handle, transport, &second, 1).await;
    frame(cx, handle).await;
    let first_widget = first.read_with(cx, |p, _| p.markdown.as_ref().unwrap().entity_id());
    let second_widget = second.read_with(cx, |p, _| p.markdown.as_ref().unwrap().entity_id());
    let (start, end) = endpoints(cx, first, &second);
    for (a, b) in [(start, end), (end, start)] {
        drag(cx, handle, a, b).await;
        for p in [first, &second] {
            focus(cx, handle, p);
            assert_eq!(
                copy(cx, handle),
                "first α body\nsecond β body",
                "document Copy is focus independent"
            );
        }
    }
    key(cx, handle, "secondary-a");
    frame(cx, handle).await;
    assert_eq!(
        copy(cx, handle),
        "second β body",
        "Select All is local to the focused document"
    );
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 0, 2, vec![node(6), node(1)])],
    );
    frame(cx, handle).await;
    assert_eq!(
        first.read_with(cx, |p, _| p.markdown.as_ref().unwrap().entity_id()),
        first_widget
    );
    assert_eq!(
        second.read_with(cx, |p, _| p.markdown.as_ref().unwrap().entity_id()),
        second_widget
    );
    let (start, end) = endpoints(cx, &second, first);
    drag(cx, handle, start, end).await;
    assert_eq!(
        copy(cx, handle),
        "second β body\nfirst α body",
        "Copy follows reordered documents"
    );
    publish(
        &mut session.borrow_mut(),
        second_source,
        1,
        "replacement **λ** body\n",
    );
    handle
        .update(cx, |view, _, cx| view.document_changed(second_source, cx))
        .unwrap();
    installed_revision(cx, handle, transport, &second, 2).await;
    frame(cx, handle).await;
    assert_eq!(
        copy(cx, handle),
        "unchanged",
        "replacing one document retires shared geometry"
    );
    let (start, end) = endpoints(cx, &second, first);
    drag(cx, handle, start, end).await;
    assert_eq!(copy(cx, handle), "replacement λ body\nfirst α body");
    let retired = second.downgrade();
    let retired_widget = second.read_with(cx, |p, _| p.markdown.as_ref().unwrap().downgrade());
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 0, 2, vec![node(1)]),
            Op::Remove(node(6)),
        ],
    );
    drop(second);
    frame(cx, handle).await;
    assert!(retired.upgrade().is_none(), "removed presentation released");
    assert!(
        retired_widget.upgrade().is_none(),
        "removed Markdown owner released"
    );
    focus(cx, handle, first);
    assert_eq!(
        copy(cx, handle),
        "unchanged",
        "removing selected document clears surviving range"
    );
    let recycled = NodeId::from_parts(6, 2).unwrap();
    let mut config = document(second_source, Mode::Markdown);
    config.layout = Layout::Viewport(80.);
    apply(
        cx,
        handle,
        vec![
            Op::Create(recycled, Kind::DocumentView, "".into(), None),
            Op::SetDocument(recycled, config),
            Op::Splice(node(0), 1, 0, vec![recycled]),
        ],
    );
    let replacement = handle
        .update(cx, |v, _, _| {
            v.documents[&recycled].presentation.clone().unwrap()
        })
        .unwrap();
    installed_revision(cx, handle, transport, &replacement, 2).await;
    frame(cx, handle).await;
    focus(cx, handle, &replacement);
    assert_eq!(
        copy(cx, handle),
        "unchanged",
        "recycled document node cannot revive old selection"
    );
    let (start, end) = endpoints(cx, first, &replacement);
    drag(cx, handle, start, end).await;
    assert_eq!(
        copy(cx, handle),
        "first α body\nreplacement λ body",
        "fresh gesture uses recycled document only"
    );
    let retired_replacement = replacement.downgrade();
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 0, 2, vec![node(1)]),
            Op::Remove(recycled),
        ],
    );
    drop(replacement);
    frame(cx, handle).await;
    assert!(retired_replacement.upgrade().is_none());
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(second_source)),
        Response::Ack
    );
    eprintln!(
        "GPUIO_MULTI_DOCUMENT_SELECTION_OK: independent sources, forward/reverse focus-independent Copy, local Select All, reorder identity/order, source retirement, unmount release and fresh node generation"
    );
}
