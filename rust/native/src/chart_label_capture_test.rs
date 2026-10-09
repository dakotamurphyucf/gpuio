//! Native input dispatch and source retirement before another frame.
use super::*;

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
    absent: &Data,
) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                id(4),
                Kind::PointerArea,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(4, 1).unwrap()),
            ),
            Op::SetPointer(
                id(4),
                PointerConfig {
                    label: "Drag radar label".into(),
                    button: PointerButton::Left,
                    disabled: false,
                    prevent_default: true,
                    stop_propagation: true,
                },
            ),
            Op::SetStyle(
                id(4),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::Splice(id(2), 0, 1, vec![id(4)]),
            Op::Splice(id(4), 0, 0, vec![id(3)]),
        ],
    );
    let original = session
        .borrow()
        .chart(source)
        .unwrap()
        .snapshot()
        .unwrap()
        .data()
        .clone();
    let pie = data(1.);
    for (next, reset) in [(absent, false), (&original, true), (&pie, true)] {
        let generation = session
            .borrow()
            .chart(source)
            .unwrap()
            .snapshot()
            .unwrap()
            .generation();
        let position = start_capture(cx, handle);
        // An ordinary update must not interrupt the same still-visible label.
        publish(
            cx,
            handle,
            source,
            session,
            &original,
            generation,
            Some(true),
        );
        publish(
            cx,
            handle,
            source,
            session,
            next,
            generation + i64::from(reset),
            Some(false),
        );
        assert_eq!(
            phases(transport),
            vec![
                PointerPhase::Started,
                PointerPhase::Cancelled(PointerCancel::Hidden)
            ]
        );
        let revision = publish(
            cx,
            handle,
            source,
            session,
            &original,
            generation + i64::from(reset),
            None,
        );
        ready(cx, handle, revision, 0xff0000ff).await;
        draw(cx, handle);
        // An old mouse-up cannot finish the retired gesture after the same axis
        // returns. A new native down/up must still work normally.
        crate::host::native_test::mouse(cx, handle, position, false);
        assert!(phases(transport).is_empty());
        let position = start_capture(cx, handle);
        crate::host::native_test::mouse(cx, handle, position, false);
        assert_eq!(
            phases(transport),
            vec![PointerPhase::Started, PointerPhase::Released]
        );
    }
    start_capture(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(content_config(source, false)))],
    );
    assert_retired(cx, handle, transport);
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(content_config(source, true)))],
    );

    let next = match immediate(session, Request::Create) {
        Response::Created(id) => id,
        _ => panic!("replacement source"),
    };
    stage_data(session, next, 0, 1, &original);
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(next, 1))
    else {
        panic!("replacement publication")
    };
    let completion = work.run();
    cx.update(|cx| {
        assert_eq!(
            session.borrow_mut().complete_chart(completion),
            Response::Ack
        );
        crate::host::chart_source_changed(Some(next), cx);
    });
    start_capture(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(content_config(next, true)))],
    );
    assert_retired(cx, handle, transport);
    ready(cx, handle, 1, 0xff0000ff).await;
    start_capture(cx, handle);
    cx.update(|cx| dispatch(cx, transport, 94, Request::Release(next)));
    assert_retired(cx, handle, transport);
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(content_config(source, true)))],
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(4), 0, 1, vec![]),
            Op::Splice(id(2), 0, 1, vec![id(3)]),
            Op::Remove(id(4)),
        ],
    );
    eprintln!(
        "GPUIO_RADAR_CAPTURE_RETIRE_OK: value-update preservation; removal/reset/family/hide/replacement/release retirement before paint; stale release rejection and recovery"
    );
}

fn phases(transport: &Transport) -> Vec<PointerPhase> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter_map(|event| match event {
            Event::PointerEvent(_, node, _, _, sample) if node == id(4) => Some(sample.phase),
            _ => None,
        })
        .collect()
}

fn start_capture(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> gpui::Point<gpui::Pixels> {
    draw(cx, handle);
    let position = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&id(4)].bounds.center()
        })
        .unwrap();
    crate::host::native_test::move_mouse(cx, handle, position, false);
    crate::host::native_test::mouse(cx, handle, position, true);
    handle
        .update(cx, |view, window, _| {
            assert!(
                window.captured_hitbox().is_some(),
                "label pointer region starts capture"
            );
            assert!(view.pointer_capture.borrow().is_active(id(4)));
        })
        .unwrap();
    position
}

fn publish(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    data: &Data,
    generation: i64,
    capture: Option<bool>,
) -> i64 {
    let snapshot = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let revision = snapshot.revision() + 1;
    stage_data(session, source, snapshot.revision(), generation, data);
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, revision))
    else {
        panic!("capture publication")
    };
    let completion = work.run();
    cx.update(|cx| {
        assert_eq!(
            session.borrow_mut().complete_chart(completion),
            Response::Ack
        );
        crate::host::chart_source_changed(Some(source), cx);
        if let Some(expected) = capture {
            handle
                .update(cx, |view, window, _| {
                    assert_eq!(
                        window.captured_hitbox().is_some(),
                        expected,
                        "capture must reconcile before paint"
                    );
                    assert_eq!(view.pointer_capture.borrow().is_active(id(4)), expected);
                    if !expected {
                        assert!(!view.focus.borrow().allows(id(4)));
                    }
                })
                .unwrap();
        }
    });
    revision
}

fn assert_retired(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    handle
        .update(cx, |view, window, _| {
            assert!(window.captured_hitbox().is_none());
            assert!(!view.pointer_capture.borrow().is_active(id(4)));
            assert!(!view.focus.borrow().allows(id(4)));
        })
        .unwrap();
    assert_eq!(
        phases(transport),
        vec![
            PointerPhase::Started,
            PointerPhase::Cancelled(PointerCancel::Hidden)
        ]
    );
}
