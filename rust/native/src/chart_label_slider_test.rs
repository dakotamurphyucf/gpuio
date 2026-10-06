//! Native slider track input and source visibility changes between frames.
use super::*;
use gpuio_protocol::{numeric::Domain, slider as s};

fn events(transport: &Transport) -> Vec<s::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter_map(|event| match event {
            Event::SliderEvent(_, node, _, _, event) if node == id(5) => Some(event),
            Event::Press(_, node, ..) if node == id(5) => panic!("slider emitted Press"),
            _ => None,
        })
        .collect()
}

fn snapshot(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> s::Snapshot {
    handle
        .update(cx, |view, window, cx| {
            match view.slider_command(id(5), s::Command::ReadSnapshot, window, cx) {
                s::Response::Applied(value) => value,
                response => panic!("slider snapshot: {response:?}"),
            }
        })
        .unwrap()
}

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
                id(5),
                Kind::Slider,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::Create(id(6), Kind::Container, "".into(), None),
            Op::SetSlider(
                id(5),
                s::Config {
                    domain: Domain::new(0., 100., 1.).unwrap(),
                    label: "Radar threshold".into(),
                    lower_label: "Minimum".into(),
                    upper_label: "Maximum".into(),
                    axis: s::Axis::Horizontal,
                    scale: s::Scale::Linear,
                    disabled: false,
                    read_only: false,
                },
                s::Value::Single(0.),
            ),
            Op::SetStyle(
                id(5),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(100.)),
                    Field::Height(Length::Px(30.)),
                ])],
            ),
            Op::Splice(id(6), 0, 0, vec![id(3), id(5)]),
            Op::Splice(id(2), 0, 1, vec![id(6)]),
        ],
    );
    let revision = session
        .borrow()
        .chart(source)
        .unwrap()
        .snapshot()
        .unwrap()
        .revision();
    ready(cx, handle, revision, 0xff0000ff).await;
    draw(cx, handle);
    events(transport);
    let position = handle
        .update(cx, |view, _, _| {
            assert!(
                view.focus.borrow().allows(id(5)),
                "slider fixture must be visible"
            );
            view.probes.borrow()[&id(5)].bounds.center()
        })
        .unwrap();
    crate::host::native_test::move_mouse(cx, handle, position, false);
    crate::host::native_test::mouse(cx, handle, position, true);
    let started = snapshot(cx, handle);
    assert_eq!(
        started.dragging,
        Some(s::Thumb::Single),
        "label slider track must start dragging"
    );
    assert_eq!(started.value, s::Value::Single(50.));
    assert!(matches!(
        events(transport).first(),
        Some(s::Event::DragStarted(_))
    ));
    let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
    // A publication retaining the same axis must preserve the current gesture.
    stage_data(
        session,
        source,
        original.revision(),
        original.generation(),
        original.data(),
    );
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, original.revision() + 1))
    else {
        panic!("unchanged slider publication")
    };
    let completion = work.run();
    cx.update(|cx| {
        assert_eq!(
            session.borrow_mut().complete_chart(completion),
            Response::Ack
        );
        crate::host::chart_source_changed(Some(source), cx);
        handle
            .update(cx, |_, window, _| {
                assert!(window.captured_hitbox().is_some())
            })
            .unwrap();
    });
    assert_eq!(snapshot(cx, handle), started);
    assert!(events(transport).is_empty());
    let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
    stage_data(
        session,
        source,
        original.revision(),
        original.generation(),
        absent,
    );
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, original.revision() + 1))
    else {
        panic!("slider publication")
    };
    let completion = work.run();
    cx.update(|cx| {
        assert_eq!(
            session.borrow_mut().complete_chart(completion),
            Response::Ack
        );
        crate::host::chart_source_changed(Some(source), cx);
        handle
            .update(cx, |view, window, cx| {
                assert!(!view.focus.borrow().allows(id(5)));
                assert!(
                    window.captured_hitbox().is_none(),
                    "hidden slider releases capture before paint"
                );
                let s::Response::Applied(snapshot) =
                    view.slider_command(id(5), s::Command::ReadSnapshot, window, cx)
                else {
                    panic!("hidden snapshot")
                };
                assert!(snapshot.dragging.is_none());
            })
            .unwrap();
    });
    let observed = events(transport);
    assert!(
        matches!(
            observed.as_slice(),
            [s::Event::Cancelled(s::CancelReason::Hidden, _)]
        ),
        "{observed:?}"
    );
    stage_data(
        session,
        source,
        original.revision() + 1,
        original.generation(),
        original.data(),
    );
    cx.update(|cx| {
        dispatch(
            cx,
            transport,
            95,
            Request::Publish(source, original.revision() + 2),
        )
    });
    ready(cx, handle, original.revision() + 2, 0xff0000ff).await;
    draw(cx, handle);
    events(transport);
    crate::host::native_test::mouse(cx, handle, position, false);
    assert!(
        events(transport).is_empty(),
        "old mouse up cannot commit a retired drag"
    );
    crate::host::native_test::move_mouse(cx, handle, position, false);
    crate::host::native_test::mouse(cx, handle, position, true);
    crate::host::native_test::mouse(cx, handle, position, false);
    assert!(matches!(
        events(transport).last(),
        Some(s::Event::Committed(..))
    ));
    assert!(snapshot(cx, handle).dragging.is_none());
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(6), 0, 2, vec![]),
            Op::Splice(id(2), 0, 1, vec![id(3)]),
            Op::Remove(id(5)),
            Op::Remove(id(6)),
        ],
    );
    eprintln!(
        "GPUIO_RADAR_SLIDER_OK: native track input; same-axis publication preserves drag; hidden drag cancellation before paint; stale release rejection and recovery"
    );
}
