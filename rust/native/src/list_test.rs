//! Real GPUI layout through the production host, including sparse placeholders.
use super::*;
use gpuio_protocol::list::{
    Config, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget, Viewport,
};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn window_id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
fn apply(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let tx = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations,
            };
            let applied = view
                .session
                .borrow_mut()
                .apply(&tx)
                .unwrap_or_else(|error| panic!("{error:?}: {tx:?}"));
            view.update_editors(&applied.dirty, window, cx);
            view.list_actions(&applied.lists);
            cx.notify();
        })
        .unwrap();
}
fn viewport(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> Viewport {
    window
        .update(cx, |view, _, _| {
            view.lists[&node(0)]
                .borrow()
                .observed
                .clone()
                .expect("list laid out")
        })
        .unwrap()
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    super::editor_test::frame(cx, window).await;
    super::editor_test::frame(cx, window).await;
}
fn initial() -> Vec<Op> {
    let mut ops = vec![
        Op::Create(
            node(0),
            Kind::VirtualList,
            "".into(),
            Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetStyle(node(0), dimensions(420., 300.)),
        Op::SetListConfig(
            node(0),
            Config {
                estimated_height: 100.,
                overscan: 200.,
                max_active: 32,
                scroll_policy: ScrollPolicy::FollowTailWhenAtEnd,
                scrollbar: true,
                managed: true,
            },
        ),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 100_000,
                }],
            },
        ),
        Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(1, 0.),
            },
        ),
    ];
    for id in 1..=10 {
        ops.extend([
            Op::Create(
                node(id),
                if id == 1 { Kind::Button } else { Kind::Text },
                format!("Row {id}"),
                if id == 1 {
                    Some(gpuio_protocol::HandlerId::from_parts(1, 1).unwrap())
                } else {
                    None
                },
            ),
            Op::SetStyle(node(id), dimensions(400., 100.)),
        ]);
    }
    ops.extend([
        Op::SetListRows(
            node(0),
            (1..=10).map(|id| Row { id, node: node(id) }).collect(),
        ),
        Op::Splice(node(0), 0, 0, (1..=10).map(node).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    ops
}
async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    eprintln!("LIST_TEST waiting initial frame");
    frame(cx, window).await;
    eprintln!("LIST_TEST initial layout ready");
    let first = viewport(cx, window);
    assert_eq!(first.visible_first, 0, "{first:?}");
    assert!((2..=4).contains(&first.visible_last), "{first:?}");
    assert!(
        !first.requested.is_empty() && first.requested.len() <= 10,
        "{first:?}"
    );
    assert!(
        first.at_start && !first.at_end && !first.following_tail,
        "{first:?}"
    );
    // The UI gains focus after the last observed range, immediately before an
    // eviction arrives. Native admission must retain it without a layout roundtrip.
    window
        .update(cx, |view, window, cx| {
            view.buttons[&node(1)].focus.focus(window, cx);
            let pins = view.list_pins(window, cx);
            assert!(
                pins.iter()
                    .any(|pin| pin.node == node(0) && pin.rows.contains(&1))
            );
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let tx = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations: vec![
                    Op::Remove(node(1)),
                    Op::Splice(node(0), 0, 1, vec![]),
                    Op::SetListRows(
                        node(0),
                        (2..=10).map(|id| Row { id, node: node(id) }).collect(),
                    ),
                ],
            };
            assert!(matches!(
                view.session.borrow_mut().apply_guarded(&tx, &pins),
                Err(crate::tree::ApplyFailure::Retained(_))
            ));
            assert_eq!(
                view.session.borrow().tree(view.id).unwrap().revision(),
                base
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 2,
                target: ScrollTarget::Offset(50_001, 37.5),
            },
        )],
    );
    frame(cx, window).await;
    let jumped = viewport(cx, window);
    assert_eq!(jumped.anchor, Some((50_001, 37.5)), "{jumped:?}");
    assert_eq!(jumped.visible_first, 50_000);
    assert_eq!(jumped.pinned, vec![1]);
    window
        .update(cx, |view, window, cx| {
            assert!(view.buttons[&node(1)].focus.is_focused(window));
            window.focus(view.root_focus.as_ref().unwrap(), cx);
        })
        .unwrap();
    assert!(
        jumped.requested.contains(&50_001) && jumped.requested.len() <= 10,
        "{jumped:?}"
    );
    apply(
        cx,
        window,
        vec![Op::SetListOrder(
            node(0),
            Order {
                revision: 2,
                runs: vec![
                    IdRun {
                        first: 100_001,
                        count: 1,
                    },
                    IdRun {
                        first: 1,
                        count: 100_000,
                    },
                ],
            },
        )],
    );
    frame(cx, window).await;
    let prepended = viewport(cx, window);
    assert_eq!(prepended.anchor, Some((50_001, 37.5)), "{prepended:?}");
    assert_eq!(prepended.visible_first, 50_001);
    apply(
        cx,
        window,
        vec![Op::SetListOrder(
            node(0),
            Order {
                revision: 3,
                runs: vec![
                    IdRun {
                        first: 50_001,
                        count: 50_000,
                    },
                    IdRun {
                        first: 1,
                        count: 50_000,
                    },
                    IdRun {
                        first: 100_001,
                        count: 1,
                    },
                ],
            },
        )],
    );
    frame(cx, window).await;
    let reordered = viewport(cx, window);
    assert_eq!(reordered.anchor, Some((50_001, 37.5)), "{reordered:?}");
    assert_eq!(reordered.visible_first, 0);
    // Materialize a previously absent row with a different measured height.
    apply(
        cx,
        window,
        vec![
            Op::Create(node(11), Kind::Text, "streaming response".into(), None),
            Op::SetStyle(node(11), dimensions(400., 150.)),
            Op::SetListRows(
                node(0),
                (1..=10)
                    .map(|id| Row { id, node: node(id) })
                    .chain([Row {
                        id: 50_001,
                        node: node(11),
                    }])
                    .collect(),
            ),
            Op::Splice(node(0), 10, 0, vec![node(11)]),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| {
            let state = view.lists[&node(0)].borrow();
            assert_eq!(
                state
                    .native
                    .handle()
                    .bounds_for_item(0)
                    .unwrap()
                    .size
                    .height,
                px(150.)
            );
            assert_eq!(
                state.observed.as_ref().unwrap().anchor,
                Some((50_001, 37.5))
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::SetStyle(node(11), dimensions(400., 220.))],
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| {
            let state = view.lists[&node(0)].borrow();
            assert_eq!(
                state
                    .native
                    .handle()
                    .bounds_for_item(0)
                    .unwrap()
                    .size
                    .height,
                px(220.)
            );
            assert_eq!(
                state.observed.as_ref().unwrap().anchor,
                Some((50_001, 37.5))
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::SetStyle(node(0), dimensions(360., 180.))],
    );
    frame(cx, window).await;
    assert_eq!(viewport(cx, window).anchor, Some((50_001, 37.5)));
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 3,
                target: ScrollTarget::End,
            },
        )],
    );
    frame(cx, window).await;
    let tail = viewport(cx, window);
    assert!(tail.following_tail && tail.at_end, "{tail:?}");
    window
        .update(cx, |view, _, _| {
            assert_eq!(view.session.borrow().tree(view.id).unwrap().len(), 12);
            assert!(
                view.lists[&node(0)]
                    .borrow()
                    .observed
                    .as_ref()
                    .unwrap()
                    .requested
                    .len()
                    <= 32
            );
        })
        .unwrap();
    let owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.lists[&node(0)]))
        .unwrap();
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((1..=11).map(|id| Op::Remove(node(id))));
    remove.push(Op::Remove(node(0)));
    apply(cx, window, remove);
    frame(cx, window).await;
    assert!(owner.upgrade().is_none(), "unmounted list owner retained");
    eprintln!(
        "GPUIO_NATIVE_LIST_OK: 100k logical rows, sparse layout, programmatic viewport, exact prepend/reorder/height/resize anchor, tail jump and disposal"
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
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Managed lists", 480., 420.)
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(480.), px(420.)),
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
                eprintln!("LIST_TEST applying initial source");
                apply(cx, window, initial());
                exercise(cx, window).await;
                window
                    .update(cx, |_, window, _| window.remove_window())
                    .unwrap();
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
