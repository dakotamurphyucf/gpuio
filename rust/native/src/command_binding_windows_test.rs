//! Window identity is part of every subscription, including reused node IDs.
use super::*;

fn samples(transport: &Transport) -> BTreeMap<(WindowId, NodeId), binding::Observation> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::CommandBindingObserved(window, node, _, _, sample) => {
                Some(((window, node), sample))
            }
            _ => None,
        })
        .collect()
}
fn enabled(sample: &binding::Observation) -> bool {
    let binding::State::Ready(entries) = &sample.state else {
        panic!("not ready: {sample:?}");
    };
    let binding::Entry::Registry { enabled, .. } = &entries[0] else {
        panic!("not registry: {sample:?}");
    };
    *enabled
}
fn open(cx: &mut gpui::AsyncApp, first: WindowHandle<View>, id: WindowId) -> WindowHandle<View> {
    let (session, transport) = first
        .update(cx, |view, _, _| {
            (view.session.clone(), view.transport.clone())
        })
        .unwrap();
    session
        .borrow_mut()
        .open(2, id, "Independent binding query", 260., 100.)
        .unwrap();
    cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(260.), px(100.)),
                    cx,
                ))),
                focus: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(id, session, transport)),
        )
        .unwrap()
    })
}
fn mount(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, active: bool) {
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0, 1))),
            Op::SetCommands(node(0), vec![command(active)]),
            Op::Create(node(1), Kind::Container, "".into(), Some(handler(1, 1))),
            Op::SetCommandBinding(node(1), Some(query(binding::Context::Here))),
            Op::Create(node(2), Kind::Text, "Independent query".into(), None),
            Op::Splice(node(1), 0, 0, vec![node(2)]),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
}
fn close(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    window
        .update(cx, |view, window, _| {
            view.session.borrow_mut().close(view.id).unwrap();
            window.remove_window();
        })
        .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    first: WindowHandle<View>,
    transport: &Transport,
) {
    let second_id = WindowId::from_parts(1, 1).unwrap();
    let second = open(cx, first, second_id);
    mount(cx, second, false);
    frame(cx, second).await;
    let initial = samples(transport);
    assert!(!enabled(&initial[&(second_id, node(1))]));
    assert_eq!(initial[&(second_id, node(1))].epoch, 1);
    apply(
        cx,
        first,
        vec![Op::SetCommands(node(0), vec![command(false)])],
    );
    frame(cx, first).await;
    apply(
        cx,
        second,
        vec![Op::SetCommands(node(0), vec![command(true)])],
    );
    frame(cx, second).await;
    let simultaneous = samples(transport);
    assert!(!enabled(&simultaneous[&(id(), node(1))]));
    assert!(enabled(&simultaneous[&(second_id, node(1))]));
    assert_eq!(
        simultaneous[&(second_id, node(1))].epoch,
        2,
        "identical node/handler IDs in another window cannot coalesce this owner"
    );
    apply(
        cx,
        second,
        vec![Op::SetCommands(node(0), vec![command(false)])],
    );
    frame(cx, second).await;
    apply(
        cx,
        first,
        vec![Op::SetCommands(node(0), vec![command(true)])],
    );
    frame(cx, first).await;
    // Neither queue is drained before destroying the second native window.
    close(cx, second);
    frame(cx, first).await;
    let survivor = samples(transport);
    assert!(
        survivor.keys().all(|(window, _)| *window == id()),
        "destroyed window left queued observations"
    );
    assert!(
        enabled(&survivor[&(id(), node(1))]),
        "closing another window pruned a live owner's queue"
    );
    let reopened_id = WindowId::from_parts(1, 2).unwrap();
    let reopened = open(cx, first, reopened_id);
    mount(cx, reopened, true);
    frame(cx, reopened).await;
    let remounted = samples(transport);
    assert!(enabled(&remounted[&(reopened_id, node(1))]));
    assert_eq!(remounted[&(reopened_id, node(1))].epoch, 1);
    assert!(!remounted.keys().any(|(window, _)| *window == second_id));
    close(cx, reopened);
    frame(cx, first).await;
    assert!(!samples(transport).keys().any(|(window, _)| *window != id()));
    eprintln!(
        "GPUIO_BINDING_WINDOWS_OK: same node IDs, independent values/epochs, queued close and window-generation reuse"
    );
}
