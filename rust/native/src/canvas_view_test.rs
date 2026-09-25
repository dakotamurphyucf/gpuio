//! Actual retained-tree mount/paint lifecycle; hidden GPU window, no OS input claim.
use super::*;
use binprot::BinProtWrite;
use gpuio_protocol::{
    canvas::*,
    canvas_resource::{Request, Response, Update},
    canvas_scene::*,
    canvas_view::{Action, Command, Viewport},
};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Duration;
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config(source: Option<ResourceId>) -> Config {
    Config {
        source,
        label: "Mounted diagram".into(),
        initial_viewport: Viewport::default(),
        minimum_zoom: 0.05,
        maximum_zoom: 64.,
        selectable: true,
        draggable: true,
        pan_zoom: true,
        disabled: false,
        selection_color: 0xffffffff,
        command: None,
    }
}
fn scene(color: i64) -> Scene {
    Scene {
        version: 1,
        description: "Canvas test".into(),
        resources: vec![],
        items: vec![Item {
            id: 1,
            transform: Transform::IDENTITY,
            clips: vec![],
            interaction: None,
            drawing: Drawing::Shape(
                Shape::Rectangle(Rect {
                    x: 10.,
                    y: 10.,
                    width: 60.,
                    height: 60.,
                }),
                Paint {
                    fill: Some(color),
                    stroke: None,
                },
            ),
        }],
    }
}
fn publish(session: &SharedSession, source: ResourceId, base: i64, generation: i64, color: i64) {
    publish_scene(session, source, base, generation, scene(color));
}
fn publish_scene(
    session: &SharedSession,
    source: ResourceId,
    base: i64,
    generation: i64,
    scene: Scene,
) {
    let mut bytes = Vec::new();
    scene.binprot_write(&mut bytes).unwrap();
    let request = Request::Begin(Update {
        id: source,
        base,
        revision: base + 1,
        generation,
        bytes: bytes.len() as i64,
    });
    assert_eq!(session.borrow_mut().canvas_request(request), Response::Ack);
    for (index, bytes) in bytes
        .chunks(gpuio_protocol::canvas_resource::MAX_CHUNK_BYTES)
        .enumerate()
    {
        assert_eq!(
            session.borrow_mut().canvas_request(Request::Chunk(
                source,
                base + 1,
                (index * gpuio_protocol::canvas_resource::MAX_CHUNK_BYTES) as i64,
                gpuio_protocol::asset::Chunk::new(bytes.to_vec()).unwrap()
            )),
            Response::Ack
        );
    }
    assert_eq!(
        session
            .borrow_mut()
            .canvas_request(Request::Publish(source, base + 1)),
        Response::Ack
    );
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
}
async fn ready(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, revision: i64) {
    for _ in 0..300 {
        draw(cx, handle);
        if handle
            .update(cx, |view, _, _| {
                view.canvases.get(&id(1)).is_some_and(|state| {
                    state
                        .borrow()
                        .ready
                        .as_ref()
                        .is_some_and(|ready| ready.snapshot.revision == revision)
                })
            })
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("mounted canvas did not prepare revision {revision}");
}
fn pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: [u8; 4]) {
    draw(cx, handle);
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            assert_eq!(
                image
                    .get_pixel((30. * scale) as u32, (30. * scale) as u32)
                    .0,
                expected
            );
        })
        .unwrap();
}
fn mount(source: ResourceId) -> Vec<Op> {
    vec![
        Op::Create(id(0), Kind::Container, "".into(), None),
        Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(240.)),
                Field::Height(Length::Px(240.)),
                Field::Background(Fill::Solid(Color::Rgba(0x101010ff))),
            ])],
        ),
        Op::Create(
            id(1),
            Kind::CanvasView,
            "".into(),
            Some(HandlerId::from_parts(1, 1).unwrap()),
        ),
        Op::SetCanvas(id(1), config(Some(source))),
        Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(200.)),
                Field::Height(Length::Px(200.)),
            ])],
        ),
        Op::Splice(id(0), 0, 0, vec![id(1)]),
        Op::SetRoot(Some(id(0))),
    ]
}
async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: SharedSession,
    transport: Arc<Transport>,
) {
    apply(cx, handle, mount(source));
    ready(cx, handle, 1).await;
    pixels(cx, handle, [255, 0, 0, 255]);
    let start = std::time::Instant::now();
    for generation in 1..=32 {
        let sibling = NodeId::from_parts(2, generation).unwrap();
        apply(
            cx,
            handle,
            vec![
                Op::Create(sibling, Kind::CanvasView, "".into(), None),
                Op::SetCanvas(sibling, config(Some(source))),
                Op::SetStyle(
                    sibling,
                    vec![Style::Fields(vec![
                        Field::Position(1),
                        Field::Left(Length::Px(100.)),
                        Field::Top(Length::Px(0.)),
                        Field::Width(Length::Px(100.)),
                        Field::Height(Length::Px(100.)),
                    ])],
                ),
                Op::Splice(id(0), 1, 0, vec![sibling]),
            ],
        );
        let mut settled = false;
        for _ in 0..300 {
            draw(cx, handle);
            if handle
                .update(cx, |view, _, _| {
                    view.canvases[&sibling].borrow().ready.is_some()
                })
                .unwrap()
            {
                settled = true;
                break;
            }
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;
        }
        assert!(settled);
        for _ in 0..2 {
            draw(cx, handle);
            handle
                .update(cx, |view, _, _| {
                    assert_eq!(
                        view.canvas_budget.borrow().used_vertices(),
                        12,
                        "two canvas meshes share one fresh frame budget"
                    )
                })
                .unwrap();
        }
        apply(
            cx,
            handle,
            vec![Op::Splice(id(0), 1, 1, vec![]), Op::Remove(sibling)],
        );
    }
    eprintln!(
        "GPUIO_CANVAS_VIEW_LIFECYCLE: 32 mount/dispose cycles {:?}",
        start.elapsed()
    );
    publish(&session, source, 1, 1, 0x00ff00ff);
    handle
        .update(cx, |view, _, cx| view.canvas_changed(source, cx))
        .unwrap();
    ready(cx, handle, 2).await;
    pixels(cx, handle, [0, 255, 0, 255]);
    let mut configured = config(Some(source));
    configured.command = Some(Command {
        sequence: 1,
        action: Action::SetViewport(Viewport {
            origin: Point { x: 10., y: 10. },
            zoom: 2.,
        }),
    });
    apply(cx, handle, vec![Op::SetCanvas(id(1), configured)]);
    draw(cx, handle);
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::CanvasEvent(_, _, _, _, _, 2, 1, Observation::CommandCompleted(1))
    )));
    // Hide/show without a paint in between must still drop pending preparation
    // and presentation, while the native viewport/command watermark survives.
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    handle
        .update(cx, |view, _, _| {
            let state = view.canvases[&id(1)].borrow();
            assert!(state.job.is_none() && state.ready.is_none() && state.content.is_none());
            assert_eq!(state.native.as_ref().unwrap().viewport().zoom, 2.);
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(240.)),
                Field::Height(Length::Px(240.)),
            ])],
        )],
    );
    ready(cx, handle, 2).await;
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(
                event,
                Event::CanvasEvent(_, _, _, _, _, _, _, Observation::CommandCompleted(1))
            ))
    );
    // A scene can pass geometry admission but exceed mesh preparation limits.
    // Keep the previous frame, report the rejected replacement's identity once,
    // and recover on a later publication without remounting.
    let mut large = scene(0xffffffff);
    large.items = (1..=4097)
        .map(|id| Item {
            id,
            transform: Transform::IDENTITY,
            clips: vec![],
            interaction: None,
            drawing: Drawing::Shape(
                Shape::Rectangle(Rect {
                    x: 0.,
                    y: 0.,
                    width: id as f64,
                    height: 1.,
                }),
                Paint {
                    fill: Some(0xffffffff),
                    stroke: None,
                },
            ),
        })
        .collect();
    publish_scene(&session, source, 2, 1, large);
    handle
        .update(cx, |view, _, cx| view.canvas_changed(source, cx))
        .unwrap();
    let mut failed = false;
    for _ in 0..300 {
        draw(cx, handle);
        if handle
            .update(cx, |view, _, _| {
                view.canvases[&id(1)].borrow().failure.is_some()
            })
            .unwrap()
        {
            failed = true;
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(failed);
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(
                event,
                Event::CanvasEvent(_, _, _, _, _, 3, 1, Observation::Failed(Error::RenderLimit))
            ))
    );
    pixels(cx, handle, [0, 255, 0, 255]);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::CanvasEvent(..)))
    );
    // Simulate an independent content error on the still-displayed old frame.
    // Repeated painting must not alternate and replay both error identities.
    for _ in 0..3 {
        handle
            .update(cx, |view, _, cx| {
                view.canvases[&id(1)]
                    .borrow_mut()
                    .report(Error::UnavailableImage, cx)
            })
            .unwrap();
        draw(cx, handle);
    }
    let observations: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::CanvasEvent(_, _, _, _, _, revision, generation, observation) => {
                Some((revision, generation, observation))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        observations,
        vec![(2, 1, Observation::Failed(Error::UnavailableImage))]
    );
    publish(&session, source, 3, 1, 0x00ff00ff);
    handle
        .update(cx, |view, _, cx| view.canvas_changed(source, cx))
        .unwrap();
    ready(cx, handle, 4).await;
    publish(&session, source, 4, 2, 0x0000ffff);
    handle
        .update(cx, |view, _, cx| view.canvas_changed(source, cx))
        .unwrap();
    ready(cx, handle, 5).await;
    handle
        .update(cx, |view, window, _| {
            let state = view.canvases[&id(1)].borrow();
            assert_eq!(
                state.native.as_ref().unwrap().viewport(),
                Viewport::default()
            );
            assert_eq!(
                state.ready.as_ref().unwrap().quality,
                Quality::new(1., f64::from(window.scale_factor())).unwrap()
            );
        })
        .unwrap();
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(
                event,
                Event::CanvasEvent(_, _, _, _, _, _, _, Observation::CommandCompleted(1))
            ))
    );
    // Existing mounts hold leases after release; a newly mounted view cannot
    // reacquire that same generational resource identity.
    assert_eq!(
        session
            .borrow_mut()
            .canvas_request(Request::Release(source)),
        Response::Ack
    );
    pixels(cx, handle, [0, 0, 255, 255]);
    let old = handle
        .update(cx, |view, _, _| view.canvases[&id(1)].clone())
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
    );
    assert!(old.borrow().closed && old.borrow().lease.is_none() && old.borrow().ready.is_none());
    drop(old);
    let replacement = NodeId::from_parts(1, 2).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                replacement,
                Kind::CanvasView,
                "".into(),
                Some(HandlerId::from_parts(1, 2).unwrap()),
            ),
            Op::SetCanvas(replacement, config(Some(source))),
            Op::SetStyle(
                replacement,
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(200.)),
                    Field::Height(Length::Px(200.)),
                ])],
            ),
            Op::Splice(id(0), 0, 0, vec![replacement]),
        ],
    );
    draw(cx, handle);
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::CanvasEvent(
            _,
            _,
            _,
            _,
            _,
            0,
            0,
            Observation::Failed(Error::UnavailableScene)
        )
    )));
    draw(cx, handle);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::CanvasEvent(..)))
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(replacement),
            Op::SetRoot(None),
            Op::Remove(id(0)),
        ],
    );
    handle
        .update(cx, |view, _, _| {
            assert!(view.canvases.is_empty());
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            );
        })
        .unwrap();
    eprintln!(
        "GPUIO_NATIVE_CANVAS_VIEW_OK: retained-tree GPU mount/update, commands, hide/show eviction, preparation failure/recovery, generation reset, lease release/remount rejection, unmount; no OS input/AX acceptance"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Canvas view test", 240., 240.)
            .unwrap();
        let source = match session.borrow_mut().canvas_request(Request::Create) {
            Response::Created(id) => id,
            _ => panic!("create scene"),
        };
        publish(&session, source, 0, 1, 0xff0000ff);
        let handle = cx
            .open_window(
                WindowOptions {
                    focus: false,
                    show: false,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(240.), px(240.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(exercise(
                cx,
                handle,
                source,
                session.clone(),
                transport,
            ))
            .await;
            let _ = handle.update(cx, |view, window, _| {
                for state in view.canvases.values() {
                    state.borrow_mut().close();
                }
                view.canvases.clear();
                window.remove_window();
            });
            canvas_host::shutdown(cx).await;
            crate::image_host::shutdown(cx).await;
            if result.is_ok() {
                assert_eq!(session.borrow().retained_canvas_bytes(), 0);
                cx.update(|cx| {
                    let metrics = canvas_host::measurements(cx).unwrap();
                    assert!(metrics.2 <= 2);
                    assert_eq!(metrics.3, 0);
                    assert_eq!(crate::canvas_content::retained_text_bytes(cx), 0);
                    eprintln!("GPUIO_CANVAS_VIEW_FINAL_METRICS: {metrics:?}");
                });
            }
            *task_failure.borrow_mut() = result.err();
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
