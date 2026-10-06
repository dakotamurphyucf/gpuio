//! Production retained-tree chart mounting and idle retirement; hidden GPU window.
use super::*;
use binprot::BinProtWrite;
use gpuio_protocol::{
    chart_data::{Contents, Data, Slice},
    chart_resource::{Request, Response, Update},
};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config(source: ResourceId, color: i64) -> Config {
    let style = gpuio_protocol::chart_style::Style {
        palette: vec![color],
        ..Default::default()
    };
    Config {
        version: -1,
        radar_labels: vec![],
        source: Some(source),
        label: "Allocation".into(),
        legend: false,
        disabled: false,
        options: gpuio_protocol::chart_options::Options {
            pie: gpuio_protocol::chart_options::Pie {
                labels: false,
                ..gpuio_protocol::chart_options::Options::default().pie
            },
            ..Default::default()
        },
        sampling: Default::default(),
        style,
    }
}
fn data(value: f64) -> Data {
    Data {
        version: 1,
        contents: Contents::Pie(vec![Slice {
            id: 1,
            label: "Budget".into(),
            value,
        }]),
    }
}
fn immediate(session: &SharedSession, request: Request) -> Response {
    match session.borrow_mut().chart_request(request) {
        crate::session::ChartDispatch::Immediate(response) => response,
        _ => panic!("immediate chart request"),
    }
}
fn stage(session: &SharedSession, source: ResourceId, base: i64, generation: i64, value: f64) {
    stage_data(session, source, base, generation, &data(value));
}
fn stage_data(
    session: &SharedSession,
    source: ResourceId,
    base: i64,
    generation: i64,
    data: &Data,
) {
    let mut bytes = vec![];
    data.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        immediate(
            session,
            Request::Begin(Update {
                id: source,
                base,
                revision: base + 1,
                generation,
                bytes: bytes.len() as i64
            })
        ),
        Response::Ack
    );
    assert_eq!(
        immediate(
            session,
            Request::Chunk(
                source,
                base + 1,
                0,
                gpuio_protocol::asset::Chunk::new(bytes).unwrap()
            )
        ),
        Response::Ack
    );
}
fn publish(session: &SharedSession, source: ResourceId, base: i64, generation: i64) {
    stage(session, source, base, generation, 1.);
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, base + 1))
    else {
        panic!("publish work");
    };
    assert_eq!(
        session.borrow_mut().complete_chart(work.run()),
        Response::Ack
    );
}
fn dispatch(cx: &mut App, transport: &Transport, correlation: i64, request: Request) {
    transport
        .submit(Message::Chart(correlation, request.clone()), 64)
        .unwrap();
    transport.mailbox.lock().unwrap().pop();
    crate::chart_host::dispatch(correlation, request, cx);
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
async fn ready(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, revision: i64, color: i64) {
    for _ in 0..300 {
        draw(cx, handle);
        if handle
            .update(cx, |view, _, _| {
                let state = view.charts[&id(1)].borrow();
                state.ready.as_ref().is_some_and(|r| {
                    r.snapshot.revision() == revision
                        && r.config.style.palette[0] == color
                        && r.config == state.config
                        && state.requested_frame == state.ready_frame
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
    panic!("chart revision {revision} not prepared");
}
fn pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: [u8; 4]) {
    draw(cx, handle);
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            assert_eq!(
                image
                    .get_pixel((80. * scale) as u32, (80. * scale) as u32)
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
            Kind::ChartView,
            "".into(),
            Some(HandlerId::from_parts(1, 1).unwrap()),
        ),
        Op::SetChart(id(1), Box::new(config(source, 0xff0000ff))),
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
    ready(cx, handle, 1, 0xff0000ff).await;
    pixels(cx, handle, [255, 0, 0, 255]);
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(
                event,
                Event::ChartEvent(_, _, _, _, _, 1, 1, Observation::Ready(_))
            ))
    );
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(config(source, 0x0000ffff)))],
    );
    ready(cx, handle, 1, 0x0000ffff).await;
    pixels(cx, handle, [0, 0, 255, 255]);
    // Real publisher callback invalidates the mounted state and retained rows.
    stage(&session, source, 1, 1, 2.);
    cx.update(|cx| dispatch(cx, &transport, 21, Request::Publish(source, 2)));
    ready(cx, handle, 2, 0x0000ffff).await;
    // Reset clears the old picture synchronously before the next preparation.
    publish(&session, source, 2, 2);
    cx.update(|cx| crate::host::chart_source_changed(Some(source), cx));
    handle
        .update(cx, |view, _, _| {
            assert!(view.charts[&id(1)].borrow().ready.is_none())
        })
        .unwrap();
    ready(cx, handle, 3, 0x0000ffff).await;
    // Native publisher Release must drop idle readers without waiting for paint.
    cx.update(|cx| dispatch(cx, &transport, 22, Request::Release(source)));
    handle
        .update(cx, |view, _, _| {
            let state = view.charts[&id(1)].borrow();
            assert!(state.ready.is_none() && state.job.is_none() && state.lease.is_none());
        })
        .unwrap();
    pixels(cx, handle, [16, 16, 16, 255]);
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(
                event,
                Event::ChartEvent(
                    _,
                    _,
                    _,
                    _,
                    _,
                    0,
                    0,
                    Observation::Failed(Error::UnavailableData)
                )
            ))
    );
    // Generation-reused resource and source replacement cannot revive retired data.
    let next = match immediate(&session, Request::Create) {
        Response::Created(id) => id,
        _ => panic!("create replacement"),
    };
    assert_ne!(source, next);
    publish(&session, next, 0, 1);
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(config(next, 0x00ff00ff)))],
    );
    ready(cx, handle, 1, 0x00ff00ff).await;
    pixels(cx, handle, [0, 255, 0, 255]);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert!(view.charts[&id(1)].borrow().ready.is_none())
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(200.)),
                Field::Height(Length::Px(200.)),
            ])],
        )],
    );
    ready(cx, handle, 1, 0x00ff00ff).await;
    // Dense native legend has bounded visible height, real scroll extent and
    // stable scroll position across ordinary updates; reset clears that state.
    let dense = Data {
        version: 1,
        contents: Contents::Pie(
            (1..=256)
                .map(|id| Slice {
                    id,
                    label: format!("Slice {id}: {}", "long name ".repeat(20)),
                    value: 1.,
                })
                .collect(),
        ),
    };
    let mut dense_config = config(next, 0x00ff00ff);
    dense_config.legend = true;
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(dense_config))],
    );
    for (base, generation) in [(1, 1), (2, 1), (3, 2)] {
        stage_data(&session, next, base, generation, &dense);
        cx.update(|cx| dispatch(cx, &transport, 30 + base, Request::Publish(next, base + 1)));
        ready(cx, handle, base + 1, 0x00ff00ff).await;
        draw(cx, handle);
        handle
            .update(cx, |view, _, _| {
                let state = view.charts[&id(1)].borrow();
                let frame = state.ready_frame.unwrap();
                assert!(frame.legend.height <= 72.);
                assert!(state.legend_scroll.max_offset().y > px(1000.));
                if base == 2 {
                    assert_eq!(state.legend_scroll.offset().y, px(-100.));
                } else {
                    assert_eq!(state.legend_scroll.offset().y, px(0.));
                }
                state
                    .legend_scroll
                    .set_offset(gpui::point(px(0.), px(-100.)));
            })
            .unwrap();
    }
    labels::exercise(cx, handle, next, &session, &transport).await;
    pie_labels::exercise(cx, handle, next, &session, &transport).await;
    axis_labels::exercise(cx, handle, next, &session, &transport).await;
    label_content::exercise(cx, handle, next, &session, &transport).await;
    apply(
        cx,
        handle,
        vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
    );
    handle
        .update(cx, |view, _, _| assert!(view.charts.is_empty()))
        .unwrap();
    cx.update(|cx| dispatch(cx, &transport, 23, Request::Release(next)));
    eprintln!(
        "GPUIO_NATIVE_CHART_VIEW_OK: production tree GPU paint, style, async publish, reset, idle release, source replacement, hidden/unmount cleanup, dense legend scroll/update/reset"
    );
}
#[path = "chart_axis_view_test.rs"]
mod axis_labels;
#[path = "chart_input_test.rs"]
mod interaction;
#[path = "chart_label_content_test.rs"]
mod label_content;
#[path = "chart_label_view_test.rs"]
mod labels;
#[path = "chart_pie_label_view_test.rs"]
mod pie_labels;
#[path = "chart_stream_test.rs"]
mod streaming;
pub(crate) fn run() {
    run_mode(false);
}
pub(crate) fn run_input() {
    run_mode(true);
}
fn run_mode(interactive: bool) {
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
        crate::chart_host::init(&session, transport.clone(), cx);
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Chart mounting", 240., 240.)
            .unwrap();
        let source = match immediate(&session, Request::Create) {
            Response::Created(id) => id,
            _ => panic!("create chart"),
        };
        if interactive {
            interaction::publish_input(&session, source, 0, 1, false);
        } else {
            publish(&session, source, 0, 1);
        }
        let handle = cx
            .open_window(
                WindowOptions {
                    focus: interactive,
                    show: interactive,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        gpui::size(px(240.), px(240.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|cx| {
                        let mut view = View::new(window_id, session.clone(), transport.clone());
                        if interactive {
                            crate::host::window_host::watch(&mut view, window, cx);
                        }
                        view
                    })
                },
            )
            .unwrap();
        if interactive {
            cx.activate(true);
        }
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(async {
                if interactive {
                    interaction::exercise(cx, handle, source, session.clone(), transport.clone())
                        .await;
                } else {
                    exercise(cx, handle, source, session.clone(), transport.clone()).await;
                    streaming::exercise(cx, handle, session.clone(), transport.clone()).await;
                }
            })
            .await;
            let _ = handle.update(cx, |view, window, _| {
                for state in view.charts.values() {
                    state.borrow_mut().close(window);
                }
                view.charts.clear();
                window.remove_window();
            });
            crate::chart_host::shutdown(cx).await;
            renderer::shutdown(cx).await;
            crate::image_host::shutdown(cx).await;
            if result.is_ok() {
                assert_eq!(session.borrow().chart_bytes(), 0);
                cx.update(|cx| {
                    let metrics = renderer::measurements(cx).unwrap();
                    assert!(metrics.2 <= 2);
                    assert_eq!(metrics.3, 0);
                    assert_eq!(metrics.4, 0);
                    eprintln!("GPUIO_CHART_VIEW_FINAL_METRICS: {metrics:?}");
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
