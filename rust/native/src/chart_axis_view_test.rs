//! Production axis captions: real font/color pixels across style replacements.
use super::*;
use gpuio_protocol::chart_axis::{Axis, LabelAlign, LabelSide, Tick, TickPosition};
use gpuio_protocol::chart_options::Orientation;

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
        contents: Contents::Categorical(
            vec![
                gpuio_protocol::chart_data::Category {
                    id: 7,
                    label: "original seven".into(),
                },
                gpuio_protocol::chart_data::Category {
                    id: 9,
                    label: "original nine".into(),
                },
            ],
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
    let revision = snapshot.revision() + 1;
    cx.update(|cx| dispatch(cx, transport, 85, Request::Publish(source, revision)));
    for orientation in [
        Orientation::Vertical,
        Orientation::VerticalReversed,
        Orientation::Horizontal,
        Orientation::HorizontalReversed,
    ] {
        for side in [LabelSide::Before, LabelSide::After] {
            for labels in [true, false] {
                let mut config = config(source, 0x444444ff);
                config.options.cartesian.orientation = orientation;
                config.options.axes.y = false;
                config.options.axes.grid = false;
                *config.style.x_axis = Axis {
                    line: false,
                    labels,
                    position: Some(0.5),
                    label_side: side,
                    label_width: Some(60.),
                    ticks: Some(vec![
                        Tick {
                            position: TickPosition::Fraction(0.25),
                            text: "Axis".into(),
                            color: Some(0xff0000ff),
                            font_size: Some(12.),
                            align: LabelAlign::Auto,
                        },
                        Tick {
                            position: TickPosition::Fraction(0.75),
                            text: "Axis".into(),
                            color: Some(0x00ff00ff),
                            font_size: Some(24.),
                            align: LabelAlign::Auto,
                        },
                    ]),
                    ..Default::default()
                };
                apply(
                    cx,
                    handle,
                    vec![
                        Op::SetChart(id(1), Box::new(config.clone())),
                        Op::SetStyle(
                            id(1),
                            vec![Style::Fields(vec![
                                Field::Width(Length::Px(200.)),
                                Field::Height(Length::Px(200.)),
                                Field::FontWeight(400),
                            ])],
                        ),
                    ],
                );
                let mut complete = false;
                for _ in 0..300 {
                    draw(cx, handle);
                    complete = handle
                        .update(cx, |view, _, _| {
                            let state = view.charts[&id(1)].borrow();
                            state.ready.as_ref().is_some_and(|r| {
                                r.snapshot.revision() == revision && r.config.as_ref() == &config
                            }) && state.ready_frame.is_some_and(|f| f.width == 200.)
                        })
                        .unwrap();
                    if complete {
                        break;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(10))
                        .await;
                }
                assert!(complete, "axis style request failed to settle");
                draw(cx, handle);
                handle
                    .update(cx, |view, window, _| {
                        let state = view.charts[&id(1)].borrow();
                        let ready = state.ready.as_ref().unwrap();
                        assert_eq!(ready.snapshot.data(), &data);
                        let frame = state.ready_frame.unwrap();
                        let labels = &ready.plan.geometry().labels;
                        assert_eq!(labels.len(), if config.style.x_axis.labels { 2 } else { 0 });
                        let image = window.render_to_image().unwrap();
                        let scale = f64::from(window.scale_factor());
                        let mut ink = [0_usize; 2];
                        for (index, label) in labels.iter().enumerate() {
                            let rect = frame.label(label).rect;
                            assert!(rect.height >= if index == 0 { 18. } else { 28. });
                            for y in (rect.y * scale).floor() as u32
                                ..((rect.y + rect.height) * scale).ceil() as u32
                            {
                                for x in (rect.x * scale).floor() as u32
                                    ..((rect.x + rect.width) * scale).ceil() as u32
                                {
                                    let c = image.get_pixel(x, y).0;
                                    if c[index] > 150 && c[index].saturating_sub(c[1 - index]) > 80
                                    {
                                        ink[index] += 1;
                                    }
                                }
                            }
                            assert!(
                                ink[index] > 4,
                                "missing custom caption {orientation:?} {side:?}: {rect:?}"
                            );
                        }
                        if !labels.is_empty() {
                            assert!(
                                ink[1] > ink[0] * 3 / 2,
                                "larger font must produce more ink: {ink:?}"
                            );
                        }
                    })
                    .unwrap();
                transport.mailbox.lock().unwrap().drain(128);
            }
        }
    }
    eprintln!(
        "GPUIO_AXIS_LABEL_VIEW_OK: two caption colors and fonts, before/after and four orientations, hide/return, unchanged source; hidden-window GPU pixels"
    );
}
