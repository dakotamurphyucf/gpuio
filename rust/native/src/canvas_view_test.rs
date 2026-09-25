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
            interaction: Some(gpuio_protocol::canvas_scene::Interaction {
                label: "Task".into(),
                hit_region: HitRegion::Rectangle(Rect {
                    x: 10.,
                    y: 10.,
                    width: 60.,
                    height: 60.,
                }),
                draggable: true,
                activatable: true,
            }),
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
fn apply_to_view(
    view: &mut View,
    window: &mut Window,
    cx: &mut gpui::Context<View>,
    operations: Vec<Op>,
) {
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
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            apply_to_view(view, window, cx, operations)
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
        .update(cx, |view, window, cx| {
            view.canvas_changed(source, window, cx)
        })
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
        .update(cx, |view, window, cx| {
            view.canvas_changed(source, window, cx)
        })
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
        .update(cx, |view, window, cx| {
            view.canvas_changed(source, window, cx)
        })
        .unwrap();
    ready(cx, handle, 4).await;
    transport.mailbox.lock().unwrap().drain(128);
    publish(&session, source, 4, 2, 0x0000ffff);
    handle
        .update(cx, |view, window, cx| {
            view.canvas_changed(source, window, cx);
            let mut pending = config(Some(source));
            pending.command = Some(Command {
                sequence: 2,
                action: Action::ResetViewport,
            });
            apply_to_view(view, window, cx, vec![Op::SetCanvas(id(1), pending)]);
            assert!(
                !transport
                    .mailbox
                    .lock()
                    .unwrap()
                    .drain(128)
                    .iter()
                    .any(|event| matches!(event, Event::CanvasEvent(..))),
                "pending command must not acknowledge an old publication"
            );
        })
        .unwrap();
    ready(cx, handle, 5).await;
    let reset_events = transport.mailbox.lock().unwrap().drain(128);
    assert!(reset_events.iter().any(|event| matches!(
        event,
        Event::CanvasEvent(_, _, _, _, _, 5, 2, Observation::CommandCompleted(2))
    )));
    assert!(!reset_events.iter().any(|event| matches!(
        event,
        Event::CanvasEvent(_, _, _, _, _, _, _, Observation::CommandCompleted(1))
    )));
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
    run_mode(false);
}
pub(crate) fn run_input() {
    run_mode(true);
}
fn run_mode(input: bool) {
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
                    focus: input,
                    show: input,
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
        if input {
            cx.activate(true);
        }
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(async {
                if input {
                    exercise_input(cx, handle, source, session.clone(), transport.clone()).await;
                } else {
                    exercise(cx, handle, source, session.clone(), transport.clone()).await;
                }
            })
            .await;
            let _ = handle.update(cx, |view, window, _| {
                for state in view.canvases.values() {
                    state.borrow_mut().close(window);
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

fn observations(transport: &Transport) -> Vec<Observation> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::CanvasEvent(_, _, _, _, _, _, _, event) => Some(event),
            _ => None,
        })
        .collect()
}
fn key(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, value: &str) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: gpui::Keystroke::parse(value).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
}
fn wheel(window: &mut Window, cx: &mut App, x: f32) {
    window.dispatch_event(
        gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
            position: position(50., 30.),
            delta: gpui::ScrollDelta::Pixels(position(x, 0.)),
            touch_phase: gpui::TouchPhase::Moved,
            modifiers: Default::default(),
        }),
        cx,
    );
}
fn position(x: f32, y: f32) -> gpui::Point<gpui::Pixels> {
    gpui::point(px(x), px(y))
}
async fn frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    draw(cx, handle);
    cx.background_executor()
        .timer(Duration::from_millis(25))
        .await;
    draw(cx, handle);
}
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum AccessRequest {
    Inspect,
    Select,
    Activate,
}
#[cfg(target_os = "macos")]
#[derive(Debug)]
struct AccessibleObject {
    role: String,
    selected: bool,
    focused: bool,
    enabled: bool,
    bounds: objc2_foundation::NSRect,
}
#[cfg(target_os = "macos")]
fn accessible_object(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    request: AccessRequest,
) -> Option<AccessibleObject> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(
        object: *mut AnyObject,
        request: AccessRequest,
        depth: usize,
    ) -> Option<AccessibleObject> {
        if object.is_null() || depth > 16 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if !title.is_null()
                && (*title).to_string()
                    == if request == AccessRequest::Activate {
                        "Activate Task"
                    } else {
                        "Task"
                    }
            {
                let role: *mut NSString = msg_send![object, accessibilityRole];
                let selected: Bool = msg_send![object, isAccessibilitySelected];
                let focused: Bool = msg_send![object, isAccessibilityFocused];
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                let bounds = msg_send![object, accessibilityFrame];
                if request != AccessRequest::Inspect {
                    let _: () = msg_send![object, setAccessibilityFocused: true];
                    let accepted: Bool = msg_send![object, accessibilityPerformPress];
                    assert!(!enabled.as_bool() || accepted.as_bool());
                }
                return Some(AccessibleObject {
                    role: (*role).to_string(),
                    selected: selected.as_bool(),
                    focused: focused.as_bool(),
                    enabled: enabled.as_bool(),
                    bounds,
                });
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if !children.is_null() {
                let count: usize = msg_send![children, count];
                assert!(count <= 2048);
                for index in 0..count {
                    let child = msg_send![children, objectAtIndex: index];
                    if let Some(found) = visit(child, request, depth + 1) {
                        return Some(found);
                    }
                }
            }
            None
        }
    }
    let view = super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content = msg_send![window, contentView];
        visit(content, request, 0)
    }
}
#[cfg(target_os = "macos")]
async fn exercise_accessibility(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    transport: &Transport,
) {
    let _ = accessible_object(cx, handle, AccessRequest::Inspect); // activate the native adapter
    frame(cx, handle).await;
    let object =
        accessible_object(cx, handle, AccessRequest::Inspect).expect("accessible canvas object");
    assert_eq!(object.role, "AXStaticText");
    assert!(
        object.enabled && object.selected && object.focused,
        "{object:?}"
    );
    assert_eq!(object.bounds.size.width, 60.);
    assert_eq!(object.bounds.size.height, 60.);
    let mut clear = config(Some(source));
    clear.command = Some(Command {
        sequence: 2,
        action: Action::Select(None),
    });
    apply(cx, handle, vec![Op::SetCanvas(id(1), clear)]);
    observations(transport);
    frame(cx, handle).await;
    assert!(
        !accessible_object(cx, handle, AccessRequest::Inspect)
            .unwrap()
            .selected
    );
    accessible_object(cx, handle, AccessRequest::Select).unwrap();
    frame(cx, handle).await;
    assert_eq!(
        observations(transport),
        vec![Observation::SelectionChanged(Some(1))]
    );
    accessible_object(cx, handle, AccessRequest::Activate).unwrap();
    frame(cx, handle).await;
    assert_eq!(observations(transport), vec![Observation::Activated(1)]);
    let before = accessible_object(cx, handle, AccessRequest::Inspect).unwrap();
    assert!(before.selected && before.focused);
    key(cx, handle, "alt-shift-right");
    frame(cx, handle).await;
    let after = accessible_object(cx, handle, AccessRequest::Inspect).unwrap();
    assert_eq!(after.bounds.origin.x - before.bounds.origin.x, 10.);
    assert!(matches!(
        observations(transport).as_slice(),
        [Observation::Moved(1, _)]
    ));
    let mut offscreen = config(Some(source));
    offscreen.command = Some(Command {
        sequence: 3,
        action: Action::SetViewport(Viewport {
            origin: Point { x: 5000., y: 5000. },
            zoom: 1.,
        }),
    });
    apply(cx, handle, vec![Op::SetCanvas(id(1), offscreen)]);
    observations(transport);
    frame(cx, handle).await;
    accessible_object(cx, handle, AccessRequest::Select)
        .expect("offscreen object stays discoverable");
    frame(cx, handle).await;
    assert!(matches!(
        observations(transport).as_slice(),
        [Observation::ViewportChanged(_)]
    ));
    let visible = accessible_object(cx, handle, AccessRequest::Inspect).unwrap();
    assert!(visible.bounds.origin.x > 0. && visible.bounds.origin.y > 0.);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    assert!(accessible_object(cx, handle, AccessRequest::Inspect).is_none());
    apply(cx, handle, vec![Op::SetStyle(id(0), vec![])]);
    ready(cx, handle, 2).await;
    frame(cx, handle).await;
    assert!(accessible_object(cx, handle, AccessRequest::Inspect).is_some());
    eprintln!(
        "GPUIO_CANVAS_MACOS_AX_OK: native object label/role, selected active-descendant focus, focus/press delivery, keyboard movement and transformed AX bounds"
    );
}
async fn exercise_input(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: SharedSession,
    transport: Arc<Transport>,
) {
    use crate::host::native_test::{mouse, move_mouse};
    apply(cx, handle, mount(source));
    ready(cx, handle, 1).await;
    for _ in 0..100 {
        if handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(
        handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
    );
    move_mouse(cx, handle, position(20., 30.), false);
    mouse(cx, handle, position(20., 30.), true);
    handle
        .update(cx, |view, window, _| {
            assert!(view.canvases[&id(1)].borrow().canvas_focused(window));
            assert!(window.captured_hitbox().is_some());
        })
        .unwrap();
    assert_eq!(
        observations(&transport),
        vec![Observation::SelectionChanged(Some(1))]
    );
    move_mouse(cx, handle, position(45., 30.), true);
    frame(cx, handle).await;
    assert!(
        observations(&transport).is_empty(),
        "native preview emits no movement traffic"
    );
    mouse(cx, handle, position(45., 30.), false);
    frame(cx, handle).await;
    let events = observations(&transport);
    assert!(
        matches!(events.as_slice(),[Observation::Moved(1,transform)] if transform.tx==25. && transform.ty==0.)
    );
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            assert_eq!(
                image
                    .get_pixel((50. * scale) as u32, (30. * scale) as u32)
                    .0,
                [255, 0, 0, 255]
            );
            assert_eq!(
                image
                    .get_pixel((20. * scale) as u32, (30. * scale) as u32)
                    .0,
                [16, 16, 16, 255]
            );
            let mut outline = 0;
            for x in (34. * scale) as u32..(37. * scale) as u32 {
                for y in (12. * scale) as u32..(66. * scale) as u32 {
                    let [r, g, b, _] = image.get_pixel(x, y).0;
                    if r > 220 && g > 220 && b > 220 {
                        outline += 1;
                    }
                }
            }
            assert!(outline > 30, "selection outline follows moved hit region");
            assert!(window.captured_hitbox().is_none());
        })
        .unwrap();
    key(cx, handle, "enter");
    assert_eq!(observations(&transport), vec![Observation::Activated(1)]);
    key(cx, handle, "shift-right");
    assert!(matches!(observations(&transport).as_slice(),[Observation::Moved(1,t)] if t.tx==26.));
    // Capture survives a repaint and movement outside the element; Escape rolls
    // back the preview without a completed-movement observation.
    move_mouse(cx, handle, position(50., 30.), false);
    mouse(cx, handle, position(50., 30.), true);
    move_mouse(cx, handle, position(225., 30.), true);
    frame(cx, handle).await;
    key(cx, handle, "escape");
    mouse(cx, handle, position(225., 30.), false);
    assert!(observations(&transport).is_empty());
    handle
        .update(cx, |view, window, _| {
            let state = view.canvases[&id(1)].borrow();
            let native = state.native.as_ref().unwrap();
            assert_eq!(native.transform(native.item(1).unwrap()).tx, 26.);
            assert!(!native.has_gesture());
            assert!(window.captured_hitbox().is_none());
        })
        .unwrap();
    // Disable/re-enable without a paint cannot resurrect captured callbacks.
    move_mouse(cx, handle, position(50., 30.), false);
    mouse(cx, handle, position(50., 30.), true);
    move_mouse(cx, handle, position(80., 30.), true);
    apply(
        cx,
        handle,
        vec![Op::SetCanvas(
            id(1),
            Config {
                disabled: true,
                ..config(Some(source))
            },
        )],
    );
    apply(cx, handle, vec![Op::SetCanvas(id(1), config(Some(source)))]);
    mouse(cx, handle, position(80., 30.), false);
    assert!(observations(&transport).is_empty());
    frame(cx, handle).await;
    // Focus loss cancels capture immediately.
    move_mouse(cx, handle, position(50., 30.), false);
    mouse(cx, handle, position(50., 30.), true);
    move_mouse(cx, handle, position(75., 30.), true);
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |_, window, _| window.captured_hitbox().is_none())
            .unwrap()
    );
    mouse(cx, handle, position(75., 30.), false);
    assert!(observations(&transport).is_empty());
    // Publication cancels before new geometry becomes ready.
    move_mouse(cx, handle, position(50., 30.), false);
    mouse(cx, handle, position(50., 30.), true);
    move_mouse(cx, handle, position(70., 30.), true);
    publish(&session, source, 1, 1, 0x00ff00ff);
    handle
        .update(cx, |view, window, cx| {
            view.canvas_changed(source, window, cx)
        })
        .unwrap();
    mouse(cx, handle, position(70., 30.), false);
    assert!(observations(&transport).is_empty());
    ready(cx, handle, 2).await;
    // A trapping focus scope added and removed without painting cannot leave
    // the earlier pointer gesture alive behind it.
    move_mouse(cx, handle, position(50., 30.), false);
    mouse(cx, handle, position(50., 30.), true);
    move_mouse(cx, handle, position(80., 30.), true);
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(2), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                id(2),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ],
    );
    assert!(
        handle
            .update(cx, |_, window, _| window.captured_hitbox().is_none())
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![Op::Splice(id(0), 1, 1, vec![]), Op::Remove(id(2))],
    );
    mouse(cx, handle, position(80., 30.), false);
    assert!(observations(&transport).is_empty());
    frame(cx, handle).await;
    // Native window deactivation cancels a held gesture. A second small window
    // supplies a real activation transition and is removed immediately afterward.
    move_mouse(cx, handle, position(50., 30.), false);
    mouse(cx, handle, position(50., 30.), true);
    move_mouse(cx, handle, position(80., 30.), true);
    let other_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, other_id, "Canvas activation test", 120., 80.)
        .unwrap();
    let other = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                focus: true,
                show: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(120.), px(80.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(other_id, session.clone(), transport.clone())),
        )
        .unwrap()
    });
    for _ in 0..100 {
        if !handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    handle
        .update(cx, |view, window, _| {
            assert!(!window.is_window_active());
            assert!(window.captured_hitbox().is_none());
            assert!(
                !view.canvases[&id(1)]
                    .borrow()
                    .native
                    .as_ref()
                    .unwrap()
                    .has_gesture()
            );
        })
        .unwrap();
    other
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    session.borrow_mut().close(other_id).unwrap();
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    for _ in 0..100 {
        if handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    mouse(cx, handle, position(80., 30.), false);
    assert!(observations(&transport).is_empty());
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "tab");
    assert!(
        handle
            .update(cx, |view, window, _| view.canvases[&id(1)]
                .borrow()
                .canvas_focused(window))
            .unwrap()
    );
    move_mouse(cx, handle, position(120., 100.), false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                position: position(120., 100.),
                button: gpui::MouseButton::Middle,
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                position: position(140., 115.),
                pressed_button: Some(gpui::MouseButton::Middle),
                modifiers: Default::default(),
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position: position(140., 115.),
                button: gpui::MouseButton::Middle,
                modifiers: Default::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    assert!(
        matches!(observations(&transport).as_slice(),[Observation::ViewportChanged(v)] if v.origin==Point{x:-20.,y:-15.})
    );
    key(cx, handle, "+");
    assert!(
        matches!(observations(&transport).as_slice(),[Observation::ViewportChanged(v)] if v.zoom>1.)
    );
    key(cx, handle, "-");
    assert!(
        matches!(observations(&transport).as_slice(),[Observation::ViewportChanged(v)] if (v.zoom-1.).abs()<1e-9)
    );
    move_mouse(cx, handle, position(120., 100.), false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: position(120., 100.),
                delta: gpui::ScrollDelta::Pixels(position(0., 20.)),
                touch_phase: gpui::TouchPhase::Moved,
                modifiers: gpui::Modifiers {
                    control: true,
                    ..Default::default()
                },
            }),
            cx,
        );
    })
    .unwrap();
    frame(cx, handle).await;
    assert!(
        matches!(observations(&transport).as_slice(),[Observation::ViewportChanged(v)] if v.zoom>1.)
    );
    let mut reset = config(Some(source));
    reset.command = Some(Command {
        sequence: 1,
        action: Action::ResetViewport,
    });
    apply(cx, handle, vec![Op::SetCanvas(id(1), reset)]);
    observations(&transport);
    draw(cx, handle);
    // Coalesce samples until an actual frame, irrespective of whether GPUI
    // chooses to paint before AsyncApp.update_window returns to this task.
    move_mouse(cx, handle, position(50., 30.), false);
    cx.update_window(handle.into(), |_, window, cx| {
        for _ in 0..3 {
            wheel(window, cx, 1.);
        }
        assert!(observations(&transport).is_empty());
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert!(matches!(observations(&transport).as_slice(),
        [Observation::ViewportChanged(v)] if v.origin.x == -3.));
    // A configuration accepted before painting cancels the queued observation.
    // It preserves the viewport itself; only stale delivery is discarded.
    cx.update_window(handle.into(), |root, window, cx| {
        wheel(window, cx, 1.);
        assert!(observations(&transport).is_empty());
        root.downcast::<View>()
            .ok()
            .unwrap()
            .update(cx, |view, cx| {
                apply_to_view(
                    view,
                    window,
                    cx,
                    vec![Op::SetCanvas(
                        id(1),
                        Config {
                            label: "Updated canvas".into(),
                            ..config(Some(source))
                        },
                    )],
                );
            });
        window.draw(cx).clear(cx);
        assert!(observations(&transport).is_empty());
    })
    .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: position(50., 30.),
                delta: gpui::ScrollDelta::Pixels(position(4., 0.)),
                touch_phase: gpui::TouchPhase::Moved,
                modifiers: Default::default(),
            }),
            cx,
        );
    })
    .unwrap();
    frame(cx, handle).await;
    assert!(
        matches!(observations(&transport).as_slice(),[Observation::ViewportChanged(v)] if v.origin.x == -8.)
    );
    #[cfg(target_os = "macos")]
    exercise_accessibility(cx, handle, source, &transport).await;
    // Disable rejects key/pointer changes, while explicit commands still work.
    let mut disabled = config(Some(source));
    disabled.disabled = true;
    disabled.command = Some(Command {
        sequence: 4,
        action: Action::ResetViewport,
    });
    apply(cx, handle, vec![Op::SetCanvas(id(1), disabled)]);
    let events = observations(&transport);
    assert!(events.contains(&Observation::CommandCompleted(4)));
    key(cx, handle, "shift-right");
    move_mouse(cx, handle, position(50., 30.), false);
    mouse(cx, handle, position(50., 30.), true);
    mouse(cx, handle, position(50., 30.), false);
    assert!(observations(&transport).is_empty());
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    assert!(
        !accessible_object(cx, handle, AccessRequest::Inspect)
            .unwrap()
            .enabled
    );
    #[cfg(target_os = "macos")]
    {
        accessible_object(cx, handle, AccessRequest::Activate).unwrap();
        frame(cx, handle).await;
        assert!(observations(&transport).is_empty());
    }
    key(cx, handle, "tab");
    assert!(
        !handle
            .update(cx, |view, window, _| view.canvases[&id(1)]
                .borrow()
                .canvas_focused(window))
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(id(1)),
            Op::SetRoot(None),
            Op::Remove(id(0)),
        ],
    );
    assert_eq!(
        session
            .borrow_mut()
            .canvas_request(Request::Release(source)),
        Response::Ack
    );
    #[cfg(target_os = "macos")]
    {
        frame(cx, handle).await;
        assert!(accessible_object(cx, handle, AccessRequest::Inspect).is_none());
    }
    eprintln!(
        "GPUIO_NATIVE_CANVAS_INPUT_OK: native dispatch focus, drag capture/preview/commit, selection pixels, keyboard activation/movement, cancellation, publication fencing and wheel coalescing"
    );
}
