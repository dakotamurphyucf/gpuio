//! Actual assigned-size selection, input gating and retained native identity.
use super::*;
#[path = "container_query_nested_test.rs"]
mod nested;
use gpuio_protocol::container_query::{Config, Predicate, Range, Rule, Snapshot};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};

fn node(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn window_id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler(n: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(n, 1).unwrap()
}
fn config(generation: i64, threshold: f64) -> Config {
    Config {
        generation,
        branches: vec!["compact".into(), "wide".into()],
        default: 0,
        rules: vec![Rule {
            condition: Predicate {
                width: Range {
                    minimum: threshold,
                    maximum: None,
                },
                height: Range::ALL,
            },
            branch: 1,
        }],
    }
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |v, w, cx| {
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
        .unwrap();
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    super::editor_test::frame(cx, window).await;
    super::editor_test::frame(cx, window).await;
}
fn selections(transport: &Transport) -> Vec<Snapshot> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::ContainerSelected(_, _, _, _, snapshot) => Some(snapshot),
            _ => None,
        })
        .collect()
}
fn click(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, id: NodeId) {
    let position = window
        .update(cx, |v, _, _| v.probes.borrow()[&id].bounds.center())
        .unwrap();
    super::native_test::move_mouse(cx, window, position, false);
    super::native_test::mouse(cx, window, position, true);
    super::native_test::mouse(cx, window, position, false);
}
async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::ContainerQuery, "".into(), Some(handler(0))),
            Op::Create(
                node(1),
                Kind::Button,
                "Compact action".into(),
                Some(handler(1)),
            ),
            Op::Create(
                node(2),
                Kind::Button,
                "Wide action".into(),
                Some(handler(2)),
            ),
            Op::SetControl(node(1), Control::Button(false)),
            Op::SetControl(node(2), Control::Button(false)),
            Op::SetStyle(
                node(1),
                vec![
                    Style::Width(Length::Px(140.)),
                    Style::Height(Length::Px(40.)),
                ],
            ),
            Op::SetStyle(
                node(2),
                vec![
                    Style::Width(Length::Px(180.)),
                    Style::Height(Length::Px(40.)),
                ],
            ),
            Op::SetContainerQuery(node(0), config(1, 400.)),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, window).await;
    let first = selections(transport);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].branch, 0);
    assert!(first[0].width < 400.);
    let (small, revision) = window
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().allows(node(1)));
            assert!(!v.focus.borrow().allows(node(2)));
            (
                Rc::downgrade(&v.buttons[&node(1)]),
                v.session.borrow().tree(v.id).unwrap().revision(),
            )
        })
        .unwrap();
    click(cx, window, node(1));
    frame(cx, window).await;
    window
        .update(cx, |v, w, _| {
            assert!(v.buttons[&node(1)].focus.is_focused(w))
        })
        .unwrap();
    selections(transport);
    window
        .update(cx, |_, w, _| w.resize(size(px(480.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    let wide = selections(transport);
    assert_eq!(wide.len(), 1);
    assert_eq!(wide[0].branch, 1);
    assert!(wide[0].width >= 400.);
    assert!(wide[0].sequence > first[0].sequence);
    window
        .update(cx, |v, w, _| {
            assert_eq!(
                v.session.borrow().tree(v.id).unwrap().revision(),
                revision,
                "native resize chooses without an OCaml update"
            );
            assert!(!v.buttons[&node(1)].focus.is_focused(w));
            assert!(!v.focus.borrow().allows(node(1)));
            assert!(v.focus.borrow().allows(node(2)));
            assert!(Rc::ptr_eq(&small.upgrade().unwrap(), &v.buttons[&node(1)]));
        })
        .unwrap();
    click(cx, window, node(2));
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Press(_, n, _, _) if *n == node(2)))
            .count(),
        1
    );
    window
        .update(cx, |_, w, _| w.resize(size(px(520.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    assert!(
        selections(transport).is_empty(),
        "same branch resize is silent"
    );
    window
        .update(cx, |_, w, _| w.resize(size(px(360.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    assert_eq!(selections(transport)[0].branch, 0);
    assert!(small.upgrade().is_some());
    // Exact native boundary without allowing either branch to size its container.
    for (width, branch) in [(399., 0), (400., 1), (401., 1)] {
        apply(
            cx,
            window,
            vec![Op::SetStyle(
                node(0),
                vec![
                    Style::Width(Length::Px(width)),
                    Style::Height(Length::Px(120.)),
                ],
            )],
        );
        frame(cx, window).await;
        window
            .update(cx, |v, _, _| {
                assert!(
                    (f32::from(v.probes.borrow()[&node(0)].bounds.size.width) - width as f32).abs()
                        < 0.1
                );
                assert!(v.focus.borrow().allows(node(branch + 1)));
            })
            .unwrap();
        selections(transport);
    }
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )],
    );
    frame(cx, window).await;
    window
        .update(cx, |v, _, _| {
            assert!(!v.focus.borrow().allows(node(1)));
            assert!(!v.focus.borrow().allows(node(2)));
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    let before = window.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    assert_eq!(before, window.update(cx, |v, _, _| v.render_count).unwrap());
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(2)),
            Op::Remove(node(1)),
            Op::Remove(node(0)),
        ],
    );
    frame(cx, window).await;
    assert!(small.upgrade().is_none());
    window
        .update(cx, |v, w, _| {
            assert!(v.container_queries.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_CONTAINER_QUERY_OK: native resize/boundaries, silent same-branch layout, retained controls, focus/input gating and idle/disposal"
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
        let motion_watch = crate::motion_preference::init(cx);
        crate::motion_preference::set(gpuio_protocol::animation::Preference::Full, cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Container query acceptance", 360., 220.)
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(360.), px(220.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(async {
                exercise(cx, window, &transport).await;
                nested::exercise(cx, &transport).await;
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            drop(motion_watch);
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
