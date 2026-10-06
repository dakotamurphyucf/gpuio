//! Real hidden-window measurement/paint and transition-time slot gating.
//! This does not qualify foreground keyboard input or VoiceOver.
use super::*;
use gpuio_protocol::chart_data::RadarAxis;

#[path = "chart_label_capture_test.rs"]
mod capture;

fn content_config(source: ResourceId, labels: bool) -> Config {
    let mut value = config(source, 0xff0000ff);
    value.radar_labels = vec![7];
    value.options.radar.labels = labels;
    value
}

fn green_pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> usize {
    draw(cx, handle);
    handle
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .filter(|p| p.0 == [0, 255, 0, 255])
                .count()
        })
        .unwrap()
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    let snapshot = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let data = Data {
        version: 1,
        contents: Contents::Radar(
            [7, 9, 11]
                .into_iter()
                .map(|id| RadarAxis {
                    id,
                    label: "Same".into(),
                    maximum: 100.,
                })
                .collect(),
            vec![],
        ),
    };
    stage_data(
        session,
        source,
        snapshot.revision(),
        snapshot.generation(),
        &data,
    );
    cx.update(|cx| {
        dispatch(
            cx,
            transport,
            91,
            Request::Publish(source, snapshot.revision() + 1),
        )
    });
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(content_config(source, true))),
            Op::SetStyle(
                id(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(200.)),
                    Field::Height(Length::Px(200.)),
                ])],
            ),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::Create(id(3), Kind::Text, "Custom".into(), None),
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(40.)),
                    Field::Height(Length::Px(20.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
                ])],
            ),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
        ],
    );
    ready(cx, handle, snapshot.revision() + 1, 0xff0000ff).await;
    assert!(
        green_pixels(cx, handle) > 100,
        "ordinary custom View must paint real pixels"
    );
    handle
        .update(cx, |view, _, _| {
            assert!(view.focus.borrow().visible(id(3)));
            assert_eq!(view.charts[&id(1)].borrow().label_positions().len(), 1);
        })
        .unwrap();
    // Hiding must fence actions before another frame, including covered windows.
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(content_config(source, false)))],
    );
    handle
        .update(cx, |view, _, _| assert!(!view.focus.borrow().allows(id(3))))
        .unwrap();
    assert_eq!(green_pixels(cx, handle), 0);
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(content_config(source, true)))],
    );
    for _ in 0..300 {
        if green_pixels(cx, handle) > 100 {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(green_pixels(cx, handle) > 100);
    // A source reset retires current label gates before worker preparation/paint.
    stage_data(
        session,
        source,
        snapshot.revision() + 1,
        snapshot.generation() + 1,
        &data,
    );
    cx.update(|cx| {
        dispatch(
            cx,
            transport,
            92,
            Request::Publish(source, snapshot.revision() + 2),
        )
    });
    // Observe publication; platform-driven frames may run while this task awaits.
    for _ in 0..300 {
        if session
            .borrow()
            .chart(source)
            .unwrap()
            .snapshot()
            .unwrap()
            .generation()
            == snapshot.generation() + 1
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert_eq!(
        session
            .borrow()
            .chart(source)
            .unwrap()
            .snapshot()
            .unwrap()
            .generation(),
        snapshot.generation() + 1
    );
    handle
        .update(cx, |view, _, _| {
            let state = view.charts[&id(1)].borrow();
            if view.focus.borrow().allows(id(3)) {
                assert_eq!(
                    state.ready.as_ref().map(|r| r.snapshot.generation()),
                    Some(snapshot.generation() + 1)
                );
            }
        })
        .unwrap();
    ready(cx, handle, snapshot.revision() + 2, 0xff0000ff).await;
    assert!(green_pixels(cx, handle) > 100);
    // Control the notification boundary: the asynchronous case above may have
    // already painted the new generation by the time its publication is observed.
    // Complete a second reset and inspect its gate in the same App update, before
    // any replacement frame or worker plan can restore eligibility.
    stage_data(
        session,
        source,
        snapshot.revision() + 2,
        snapshot.generation() + 2,
        &data,
    );
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, snapshot.revision() + 3))
    else {
        panic!("reset work");
    };
    let completion = work.run();
    cx.update(|cx| {
        assert_eq!(
            session.borrow_mut().complete_chart(completion),
            Response::Ack
        );
        crate::host::chart_source_changed(Some(source), cx);
        handle
            .update(cx, |view, _, _| {
                assert!(!view.focus.borrow().allows(id(3)));
                assert!(view.charts[&id(1)].borrow().ready.is_none());
            })
            .unwrap();
    });
    ready(cx, handle, snapshot.revision() + 3, 0xff0000ff).await;
    assert!(green_pixels(cx, handle) > 100);
    // Intrinsic text sizing changes the retained View without publishing data.
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(3),
            vec![Style::Fields(vec![
                Field::FontSize(12.),
                Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
            ])],
        )],
    );
    let short = green_pixels(cx, handle);
    assert!(short > 20);
    apply(
        cx,
        handle,
        vec![Op::SetText(id(3), "Custom label with more text".into())],
    );
    assert!(
        green_pixels(cx, handle) > short,
        "native text must be remeasured"
    );
    // Removing a live axis hides its retained slot before replacement geometry
    // is ready; returning the ID makes the same retained node eligible again.
    let mut absent = data.clone();
    let Contents::Radar(axes, _) = &mut absent.contents else {
        unreachable!()
    };
    axes[0].id = 13;
    let mut reordered = data.clone();
    let Contents::Radar(axes, _) = &mut reordered.contents else {
        unreachable!()
    };
    axes.rotate_left(1);
    axes.iter_mut().for_each(|a| a.label = "Renamed".into());
    for (offset, next, shown) in [(4, &absent, false), (5, &reordered, true), (6, &data, true)] {
        stage_data(
            session,
            source,
            snapshot.revision() + offset - 1,
            snapshot.generation() + 2,
            next,
        );
        let crate::session::ChartDispatch::Publish(work) = session
            .borrow_mut()
            .chart_request(Request::Publish(source, snapshot.revision() + offset))
        else {
            panic!("axis update work")
        };
        let completion = work.run();
        cx.update(|cx| {
            assert_eq!(
                session.borrow_mut().complete_chart(completion),
                Response::Ack
            );
            crate::host::chart_source_changed(Some(source), cx);
            if !shown {
                handle
                    .update(cx, |view, _, _| {
                        assert!(!view.focus.borrow().allows(id(3)));
                    })
                    .unwrap();
            }
        });
        ready(cx, handle, snapshot.revision() + offset, 0xff0000ff).await;
        assert_eq!(green_pixels(cx, handle) > 20, shown);
        handle
            .update(cx, |view, _, _| {
                assert_eq!(view.focus.borrow().allows(id(3)), shown);
            })
            .unwrap();
    }
    capture::exercise(cx, handle, source, session, transport, &absent).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(config(source, 0xff0000ff))),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::Remove(id(3)),
            Op::Remove(id(2)),
        ],
    );
    assert_eq!(green_pixels(cx, handle), 0);
    eprintln!(
        "GPUIO_RADAR_CONTENT_PAINT_OK: intrinsic text resize/pixels, empty series, hide without paint, generation reset, live-axis removal/return/reorder, unmount"
    );
}
