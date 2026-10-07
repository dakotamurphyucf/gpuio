//! Real AppKit leases under a retained radar label, retired without paint.
use super::*;
use binprot::BinProtWrite;
use gpuio_protocol::{
    ResourceId,
    chart_data::{Contents, Data, RadarAxis},
    chart_resource::{Request, Response, Update},
    chart_view::Config,
};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
pub(super) fn immediate(session: &host::SharedSession, request: Request) -> Response {
    match session.borrow_mut().chart_request(request) {
        crate::session::ChartDispatch::Immediate(response) => response,
        _ => panic!("immediate request"),
    }
}
fn publish(session: &host::SharedSession, source: ResourceId, first_axis: i64, cx: &mut App) {
    let data = Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Radar(
            [first_axis, 9, 11]
                .into_iter()
                .map(|id| RadarAxis {
                    id,
                    label: "Axis".into(),
                    maximum: 100.,
                })
                .collect(),
            vec![],
        ),
    };
    publish_data(session, source, &data, cx);
}
pub(super) fn publish_data(
    session: &host::SharedSession,
    source: ResourceId,
    data: &Data,
    cx: &mut App,
) {
    let base = session
        .borrow()
        .chart(source)
        .ok()
        .and_then(|lease| lease.snapshot())
        .map_or(0, |s| s.revision());
    let mut bytes = vec![];
    data.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        immediate(
            session,
            Request::Begin(Update {
                id: source,
                base,
                revision: base + 1,
                generation: 1,
                bytes: bytes.len() as i64,
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
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, base + 1))
    else {
        panic!("chart publication")
    };
    assert_eq!(
        session.borrow_mut().complete_chart(work.run()),
        Response::Ack
    );
    host::chart_source_changed(Some(source), cx);
}
async fn ready(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, owner: NodeId) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        host::editor_test::frame(cx, handle).await;
        if handle
            .update(cx, |view, _, _| view.focus.borrow().allows(owner))
            .unwrap()
        {
            return;
        }
        assert!(Instant::now() < deadline, "radar label not eligible");
    }
}
pub(super) fn assert_retired(handle: WindowHandle<View>, owner: NodeId, cx: &mut App) {
    let retired = handle
        .update(cx, |view, window, cx| {
            let retired = !view.menus[&owner].borrow().tracking();
            // Failure cleanup only: a failing assertion must not strand AppKit's
            // nested tracking loop. The saved result still fails the regression.
            if !retired {
                view.close_menu(owner, false, window, cx);
            }
            retired
        })
        .unwrap();
    assert!(
        retired,
        "hidden chart content must retire its popup lease before paint"
    );
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, owner: NodeId) {
    let session = handle
        .update(cx, |view, _, _| view.session.clone())
        .unwrap();
    let source = match immediate(&session, Request::Create) {
        Response::Created(id) => id,
        _ => panic!("chart source"),
    };
    cx.update(|cx| publish(&session, source, 7, cx));
    handle
        .update(cx, |view, window, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(2), Kind::ChartView, "".into(), None),
                    Op::SetChart(
                        id(2),
                        Box::new(Config {
                            version: -2,
                            source: Some(source),
                            label: "Radar popup owner".into(),
                            options: Default::default(),
                            sampling: Default::default(),
                            style: Default::default(),
                            radar_labels: vec![7],
                            inspection_content: vec![],
                            legend: false,
                            disabled: false,
                        }),
                    ),
                    Op::SetStyle(
                        id(2),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(400.)),
                            Field::Height(Length::Px(160.)),
                        ])],
                    ),
                    Op::Create(id(3), Kind::Container, "".into(), None),
                    Op::SetRoot(None),
                    Op::Splice(id(3), 0, 0, vec![owner]),
                    Op::Splice(id(2), 0, 0, vec![id(3)]),
                    Op::SetRoot(Some(id(2))),
                ],
            );
        })
        .unwrap();
    ready(cx, handle, owner).await;
    let before = popup::tracking_calls();
    // Hide and return in one App update: the old prepared radar still has axis
    // 7, so returning it immediately reopens its eligibility gate. The earlier
    // popup lease must nonetheless stay retired when its queued runner executes.
    cx.update(|cx| {
        handle
            .update(cx, |view, window, cx| show(view, window, cx, owner, 2))
            .unwrap();
        publish(&session, source, 13, cx);
        assert_retired(handle, owner, cx);
        publish(&session, source, 7, cx);
        handle
            .update(cx, |view, _, _| {
                assert!(
                    view.focus.borrow().allows(owner),
                    "same axis must already be eligible again"
                );
                assert!(!view.menus[&owner].borrow().tracking());
            })
            .unwrap();
    });
    retired(cx).await;
    assert_eq!(popup::tracking_calls(), before);
    ready(cx, handle, owner).await;

    // The chart's production D-key route opens the original-data browser.
    // Keep this in one App update so no redraw can retire the old menu for us.
    let center = gpui::point(px(200.), px(80.));
    host::native_test::move_mouse(cx, handle, center, false);
    host::native_test::mouse(cx, handle, center, true);
    host::native_test::mouse(cx, handle, center, false);
    cx.update(|cx| {
        handle
            .update(cx, |view, window, cx| {
                assert!(view.charts[&id(2)].borrow().chart_focused(window));
                show(view, window, cx, owner, 2);
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                    keystroke: gpui::Keystroke::parse("d").unwrap(),
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
        })
        .unwrap();
        handle
            .update(cx, |view, _, _| assert!(!view.focus.borrow().allows(owner)))
            .unwrap();
        assert_retired(handle, owner, cx);
    });
    retired(cx).await;
    assert_eq!(popup::tracking_calls(), before);
    host::editor_test::key(cx, handle, "escape");
    ready(cx, handle, owner).await;

    browser_capture(cx, handle, owner).await;

    let live_session = session.clone();
    let closer = cx.spawn(async move |cx| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while popup::tracking_calls() == before {
            assert!(
                Instant::now() < deadline,
                "radar popup never entered AppKit"
            );
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;
        }
        cx.update(|cx| {
            publish(&live_session, source, 13, cx);
            assert_retired(handle, owner, cx);
        });
    });
    handle
        .update(cx, |view, window, cx| show(view, window, cx, owner, 2))
        .unwrap();
    retired(cx).await;
    closer.await;
    assert_eq!(popup::tracking_calls(), before + 1);
    handle
        .update(cx, |view, window, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Splice(id(3), 0, 1, vec![]),
                    Op::SetRoot(Some(owner)),
                    Op::Remove(id(3)),
                    Op::Remove(id(2)),
                ],
            );
        })
        .unwrap();
    assert_eq!(immediate(&session, Request::Release(source)), Response::Ack);
    host::editor_test::frame(cx, handle).await;
    eprintln!(
        "POPUP_RADAR_RETIRE_OK: queued hide/return rejection; browser cancels popup/capture; live AppKit lease retires before paint"
    );
}

async fn browser_capture(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, owner: NodeId) {
    // A retained PointerArea shares the same browser visibility contract.
    // Its prevent_default policy preserves chart focus so D can retire a held
    // child gesture through the actual chart keyboard route.
    handle
        .update(cx, |view, window, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(
                        id(4),
                        Kind::PointerArea,
                        "".into(),
                        Some(HandlerId::from_parts(4, 1).unwrap()),
                    ),
                    Op::SetPointer(
                        id(4),
                        PointerConfig {
                            label: "Radar drag label".into(),
                            button: PointerButton::Left,
                            disabled: false,
                            prevent_default: true,
                            stop_propagation: true,
                        },
                    ),
                    Op::SetStyle(
                        id(4),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(140.)),
                            Field::Height(Length::Px(22.)),
                        ])],
                    ),
                    Op::Splice(owner, 0, 1, vec![id(4)]),
                    Op::Splice(id(4), 0, 0, vec![NodeId::from_parts(1, 2).unwrap()]),
                ],
            );
        })
        .unwrap();
    ready(cx, handle, owner).await;
    let drag = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&id(4)].bounds.center()
        })
        .unwrap();
    host::native_test::move_mouse(cx, handle, drag, false);
    host::native_test::mouse(cx, handle, drag, true);
    handle
        .update(cx, |view, window, _| {
            assert!(view.charts[&id(2)].borrow().chart_focused(window));
            assert!(view.pointer_capture.borrow().is_active(id(4)));
        })
        .unwrap();
    host::editor_test::key(cx, handle, "d");
    handle
        .update(cx, |view, window, _| {
            assert!(!view.focus.borrow().allows(owner));
            assert!(window.captured_hitbox().is_none());
            assert!(!view.pointer_capture.borrow().is_active(id(4)));
            let phases: Vec<_> = view
                .transport
                .mailbox
                .lock()
                .unwrap()
                .drain(1024)
                .into_iter()
                .filter_map(|event| match event {
                    Event::PointerEvent(_, node, _, _, sample) if node == id(4) => {
                        Some(sample.phase)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                phases,
                vec![
                    PointerPhase::Started,
                    PointerPhase::Cancelled(PointerCancel::Hidden)
                ]
            );
        })
        .unwrap();
    host::editor_test::key(cx, handle, "escape");
    ready(cx, handle, owner).await;
    host::native_test::mouse(cx, handle, drag, false);
    handle
        .update(cx, |view, window, cx| {
            assert!(!view.transport.mailbox.lock().unwrap().drain(1024).into_iter().any(
                |event| matches!(event, Event::PointerEvent(_, node, _, _, _) if node == id(4))
            ), "old mouse-up must not finish a retired gesture after browser close");
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Splice(id(4), 0, 1, vec![]),
                    Op::Splice(owner, 0, 1, vec![NodeId::from_parts(1, 2).unwrap()]),
                    Op::Remove(id(4)),
                ],
            );
        })
        .unwrap();
    ready(cx, handle, owner).await;
}
