//! Production pie caption text and leader pixels in the hidden chart window.
use super::*;
use crate::chart_geometry::LabelKind;
use gpuio_protocol::{
    chart_options::{LabelPlacement, SliceRadii},
    chart_pie_labels::Entry,
};

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
        contents: Contents::Pie(vec![
            Slice {
                id: 7,
                label: "raw seven".into(),
                value: 1.,
            },
            Slice {
                id: 3,
                label: "raw three".into(),
                value: 1.,
            },
        ]),
    };
    stage_data(
        session,
        source,
        snapshot.revision(),
        snapshot.generation(),
        &data,
    );
    let revision = snapshot.revision() + 1;
    cx.update(|cx| dispatch(cx, transport, 80, Request::Publish(source, revision)));
    for (placement, labels, hidden, width, weight) in [
        (LabelPlacement::Outside, true, false, 200., 400),
        (LabelPlacement::Outside, true, false, 200., 700),
        (LabelPlacement::Outside, true, true, 120., 400),
        (LabelPlacement::Inside, true, false, 200., 400),
        (LabelPlacement::Outside, false, false, 200., 400),
    ] {
        let mut config = config(source, 0x444444ff);
        config.options.pie.labels = labels;
        config.options.pie.label_placement = placement;
        config.options.pie.slice_radii = vec![SliceRadii {
            slice: 7,
            inner: 10.,
            outer: 30.,
        }];
        config.style.label_color = 0x00ff00ff;
        config.style.pie_label_line_color = Some(0x0000ffff);
        config.style.pie_labels = vec![
            Entry {
                slice: 7,
                text: Some(if hidden { "" } else { "RIGHT" }.into()),
                line_color: Some(0xff0000ff),
            },
            Entry {
                slice: 3,
                text: Some("LEFT".into()),
                line_color: None,
            },
        ];
        apply(
            cx,
            handle,
            vec![
                Op::SetChart(id(1), Box::new(config.clone())),
                Op::SetStyle(
                    id(1),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(width)),
                        Field::Height(Length::Px(200.)),
                        Field::FontWeight(weight),
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
                        r.snapshot.revision() == revision
                            && r.config.as_ref() == &config
                            && state.ready_frame.is_some_and(|f| f.width == width)
                            && (placement != LabelPlacement::Outside
                                || !labels
                                || r.label_style.as_ref().is_some_and(|s| {
                                    s.font.weight == gpui::FontWeight(weight as f32)
                                }))
                    })
                })
                .unwrap();
            if complete {
                break;
            }
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;
        }
        assert!(complete, "pie label request failed to settle");
        draw(cx, handle);
        handle
            .update(cx, |view, window, _| {
                let state = view.charts[&id(1)].borrow();
                let ready = state.ready.as_ref().unwrap();
                let frame = state.ready_frame.unwrap();
                let image = window.render_to_image().unwrap();
                let scale = f64::from(window.scale_factor());
                let geometry = ready.plan.geometry();
                assert_eq!(
                    geometry.labels.len(),
                    if !labels {
                        0
                    } else if hidden {
                        1
                    } else {
                        2
                    }
                );
                let count_color = |left: f64, top: f64, right: f64, bottom: f64, channel: usize| {
                    let mut count = 0;
                    for y in (top * scale).max(0.).floor() as u32
                        ..(bottom * scale).ceil().min(f64::from(image.height())) as u32
                    {
                        for x in (left * scale).max(0.).floor() as u32
                            ..(right * scale).ceil().min(f64::from(image.width())) as u32
                        {
                            let c = image.get_pixel(x, y).0;
                            if c[channel] > 150
                                && (0..3)
                                    .filter(|i| *i != channel)
                                    .all(|i| c[channel].saturating_sub(c[i]) > 80)
                            {
                                count += 1;
                            }
                        }
                    }
                    count
                };
                for label in &geometry.labels {
                    let rect = frame.label(label).rect;
                    assert!(
                        count_color(rect.x, rect.y, rect.x + rect.width, rect.y + rect.height, 1)
                            > 4,
                        "missing green pie caption {:?} at {:?}",
                        label.text,
                        rect
                    );
                    if let LabelKind::Pie {
                        slice_index,
                        placement: Some(p),
                    } = label.kind
                    {
                        // This two-half fixture has horizontal leaders, leaving a clean
                        // sample between gray wedge edge and green text.
                        let x = frame.plot.x + (p.edge.x + p.bend.x) / 2.;
                        let y = frame.plot.y + (p.edge.y + p.bend.y) / 2.;
                        assert!(
                            count_color(
                                x - 2.,
                                y - 2.,
                                x + 2.,
                                y + 2.,
                                if slice_index == 0 { 0 } else { 2 }
                            ) > 0,
                            "missing independently colored pie leader for {slice_index}"
                        );
                    }
                }
                assert_eq!(ready.snapshot.data(), &data);
            })
            .unwrap();
        transport.mailbox.lock().unwrap().drain(128);
    }
    eprintln!(
        "GPUIO_PIE_LABEL_VIEW_OK: measured text pixels, ID captions, per-slice/default leader colors, variable radius, hidden captions, inside/outside, narrow/font changes and unchanged data"
    );
}
