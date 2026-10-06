//! Real hidden-window text pixels and captured-font replacement.
use super::*;
use crate::chart_geometry::{Shape, Source};
use gpuio_protocol::{
    chart_data::{Edge, Node},
    chart_node_labels::{Line, Node as LabelNode},
    chart_options::LabelPlacement,
};

fn chart(source: ResourceId, placement: LabelPlacement, labels: bool) -> Config {
    let mut value = config(source, 0x444444ff);
    value.options.sankey.label_placement = placement;
    value.options.sankey.labels = labels;
    value.style.node_labels = [
        (1, "IN X", 0xff0000ff),
        (2, "MID", 0x00ff00ff),
        (3, "OUT", 0x0000ffff),
    ]
    .into_iter()
    .map(|(node, text, color)| LabelNode {
        node,
        lines: vec![Line {
            text: text.into(),
            font_size: Some(11.),
            color: Some(color),
        }],
    })
    .collect();
    value
}
fn dimensions(width: f64, weight: i64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(200.)),
        Field::FontWeight(weight),
    ])]
}
async fn settled(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    placement: LabelPlacement,
    labels: bool,
    width: f64,
    weight: f32,
) {
    for _ in 0..300 {
        draw(cx, handle);
        if handle
            .update(cx, |view, _, _| {
                let state = view.charts[&id(1)].borrow();
                state.ready.as_ref().is_some_and(|ready| {
                    ready.snapshot.revision() == 5
                        && ready.config.options.sankey.label_placement == placement
                        && ready.config.options.sankey.labels == labels
                        && state.ready_frame.is_some_and(|frame| frame.width == width)
                        && if placement == LabelPlacement::Outside && labels {
                            ready
                                .label_style
                                .as_ref()
                                .is_some_and(|style| style.font.weight == gpui::FontWeight(weight))
                        } else {
                            ready.label_style.is_none()
                        }
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
    panic!(
        "measured chart did not settle: {placement:?} labels={labels} width={width} weight={weight}"
    );
}
fn inspect(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, outside: bool, labels: bool) {
    draw(cx, handle);
    handle
        .update(cx, |view, window, _| {
            let state = view.charts[&id(1)].borrow();
            let ready = state.ready.as_ref().unwrap();
            let frame = state.ready_frame.unwrap();
            let geometry = ready.plan.geometry();
            let image = window.render_to_image().unwrap();
            let scale = f64::from(window.scale_factor());
            assert_eq!(geometry.labels.len(), if labels { 3 } else { 0 });
            for label in &geometry.labels {
                let rect = frame.label(label).rect;
                let (node_index, channel) = match label.text.as_str() {
                    "IN X" => (2, 0),
                    "MID" => (0, 1),
                    "OUT" => (1, 2),
                    _ => panic!("unexpected label"),
                };
                let mark = geometry
                    .marks
                    .iter()
                    .find(|mark| mark.source == Source::Node(node_index))
                    .unwrap();
                let Shape::Node(node) = mark.shape else {
                    panic!("node shape")
                };
                if outside {
                    match label.text.as_str() {
                        "IN X" => assert!(rect.x + rect.width <= frame.plot.x + node.left + 0.001),
                        "OUT" => assert!(rect.x >= frame.plot.x + node.right - 0.001),
                        "MID" => assert!(rect.y + rect.height <= frame.plot.y + node.top + 0.001),
                        _ => unreachable!(),
                    }
                }
                assert_eq!(
                    ready
                        .plan
                        .hit_test(crate::chart_geometry::Point {
                            x: (node.left + node.right) / 2.,
                            y: (node.top + node.bottom) / 2.,
                        })
                        .unwrap()
                        .source,
                    Source::Node(node_index)
                );
                let mut colored = 0;
                let mut span = (u32::MAX, 0);
                for y in
                    (rect.y * scale).floor() as u32..((rect.y + rect.height) * scale).ceil() as u32
                {
                    for x in (rect.x * scale).floor() as u32
                        ..((rect.x + rect.width) * scale).ceil() as u32
                    {
                        let rgba = image
                            .get_pixel(x.min(image.width() - 1), y.min(image.height() - 1))
                            .0;
                        if rgba[channel] > 150
                            && (0..3)
                                .filter(|c| *c != channel)
                                .all(|c| rgba[channel].saturating_sub(rgba[c]) > 80)
                        {
                            colored += 1;
                            span.0 = span.0.min(x);
                            span.1 = span.1.max(x);
                        }
                    }
                }
                if label.text == "IN X" && outside && frame.width == 200. {
                    assert!(
                        f64::from(span.1.saturating_sub(span.0)) / scale > 13.,
                        "multiword caption lost its final word: {span:?} at scale {scale}, rect {rect:?}"
                    );
                }
                assert!(
                    colored > 4,
                    "label {} has no colored text in its actual plot placement: {rect:?}",
                    label.text
                );
            }
        })
        .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    let data = Data {
        version: 1,
        contents: Contents::Sankey(
            [(2, "middle"), (3, "last"), (1, "first")]
                .into_iter()
                .map(|(id, label)| Node {
                    id,
                    label: label.into(),
                })
                .collect(),
            vec![
                Edge {
                    id: 1,
                    source: 1,
                    target: 2,
                    value: 10.,
                },
                Edge {
                    id: 2,
                    source: 2,
                    target: 3,
                    value: 10.,
                },
            ],
        ),
    };
    stage_data(session, source, 4, 3, &data);
    cx.update(|cx| dispatch(cx, transport, 70, Request::Publish(source, 5)));
    for (placement, labels, width, weight) in [
        (LabelPlacement::Outside, true, 200., 400),
        (LabelPlacement::Inside, true, 200., 400),
        (LabelPlacement::Outside, true, 200., 700),
        (LabelPlacement::Outside, true, 120., 400),
        (LabelPlacement::Outside, false, 120., 400),
        (LabelPlacement::Outside, true, 200., 400),
    ] {
        apply(
            cx,
            handle,
            vec![
                Op::SetChart(id(1), chart(source, placement, labels)),
                Op::SetStyle(id(1), dimensions(width, weight)),
            ],
        );
        // Start and immediately supersede one font request before polling readiness.
        if weight == 700 {
            draw(cx, handle);
            for transient in [300, 800, 700] {
                apply(
                    cx,
                    handle,
                    vec![Op::SetStyle(id(1), dimensions(width, transient))],
                );
                draw(cx, handle);
            }
        }
        settled(cx, handle, placement, labels, width, weight as f32).await;
        inspect(cx, handle, placement == LabelPlacement::Outside, labels);
        transport.mailbox.lock().unwrap().drain(128);
    }
    eprintln!(
        "GPUIO_CHART_LABEL_VIEW_OK: three-column native text pixels, topology-independent IDs, inside/outside, narrow extent, font replacement, global hiding and node hits"
    );
}
