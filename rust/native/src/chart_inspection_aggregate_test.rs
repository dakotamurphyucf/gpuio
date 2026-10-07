//! Actual aggregate marks never borrow singular or another publication's content.
use super::*;
use gpuio_protocol::{
    chart_data::{Candle, Layer, Point, Series},
    chart_sampling::{Bar, Candlestick},
    chart_selection::{Aggregation, Span},
};

fn data(count: usize, candles: bool) -> Data {
    let ids = [7, 2, 9, 3];
    Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: if candles {
            Contents::Candlestick(
                (0..count)
                    .map(|i| Candle {
                        id: ids[i],
                        x: i as f64,
                        label: "candle".into(),
                        open_: 1.,
                        high: 4.,
                        low: 1.,
                        close: 3.,
                    })
                    .collect(),
            )
        } else {
            Contents::Cartesian(vec![Layer::Bar(Series {
                id: 11,
                name: "series".into(),
                points: (0..count)
                    .map(|i| Point {
                        id: ids[i],
                        x: i as f64,
                        y: Some(i as f64 + 1.),
                        label: "bar".into(),
                    })
                    .collect(),
            })])
        },
    }
}
fn eligible(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, node: NodeId) -> bool {
    handle
        .update(cx, |view, _, _| view.focus.borrow().allows(node))
        .unwrap()
}
fn presses(transport: &Transport, node: NodeId) -> usize {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter(|e| matches!(e, Event::Press(_, id, ..) if *id == node))
        .count()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let foreign = match immediate(session, Request::Create) {
        Response::Created(id) => id,
        _ => panic!("foreign resource"),
    };
    let mut generation = 4;
    for aggregation in [Aggregation::Sum, Aggregation::Mean, Aggregation::Exact] {
        for count in [1, 4] {
            let candles = aggregation == Aggregation::Exact;
            let data = data(count, candles);
            let revision = cx.update(|cx| publish(session, source, &data, cx));
            let wrapper = NodeId::from_parts(2, generation).unwrap();
            let node = NodeId::from_parts(3, generation).unwrap();
            let mut chart = config(source, 0xff0000ff);
            chart.style.inspection.card.width = 100.;
            if candles {
                chart.sampling.candles = Candlestick::Ohlc(1);
            } else {
                chart.sampling.bars = if aggregation == Aggregation::Sum {
                    Bar::Sum(1)
                } else {
                    Bar::Mean(1)
                };
            }
            chart.inspection_content = vec![Entry {
                target: Some(if candles {
                    Target::Candlestick(7)
                } else {
                    Target::Cartesian(11, 7)
                }),
                container: Container::Card,
            }];
            apply(
                cx,
                handle,
                vec![
                    Op::SetChart(id(1), Box::new(chart.clone())),
                    Op::Create(wrapper, Kind::Container, "".into(), None),
                    Op::Create(
                        node,
                        Kind::Button,
                        "Aggregate action".into(),
                        Some(gpuio_protocol::HandlerId::from_parts(903, generation).unwrap()),
                    ),
                    Op::SetStyle(
                        node,
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(80.)),
                            Field::Height(Length::Px(28.)),
                            Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
                        ])],
                    ),
                    Op::Splice(wrapper, 0, 0, vec![node]),
                    Op::Splice(id(1), 0, 0, vec![wrapper]),
                ],
            );
            ready(cx, handle, revision, 0xff0000ff).await;
            handle
                .update(cx, |view, window, cx| {
                    window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
                })
                .unwrap();
            key(cx, handle, "home");
            key(cx, handle, "enter");
            draw(cx, handle);
            let span = Span {
                start_index: 0,
                length: count as i64,
                first: 7,
                last: if count == 1 { 7 } else { 3 },
            };
            let selection = if candles {
                Selection::Candlestick {
                    span,
                    aggregated: true,
                }
            } else {
                Selection::Cartesian {
                    series: 11,
                    span,
                    aggregation,
                }
            };
            handle
                .update(cx, |view, _, _| {
                    assert_eq!(view.charts[&id(1)].borrow().input.selected, Some(selection))
                })
                .unwrap();
            assert!(
                !eligible(cx, handle, node),
                "even a length-one aggregate cannot borrow first-datum content"
            );
            let target = Target::Aggregate {
                source,
                data_revision: revision,
                data_generation: original.generation(),
                selection,
            };
            chart.inspection_content[0].target = Some(target);
            apply(
                cx,
                handle,
                vec![Op::SetChart(id(1), Box::new(chart.clone()))],
            );
            draw(cx, handle);
            assert!(eligible(cx, handle, node));
            handle
                .update(cx, |_, window, _| {
                    assert!(
                        window
                            .render_to_image()
                            .unwrap()
                            .pixels()
                            .filter(|p| p.0 == [0, 255, 0, 255])
                            .count()
                            > 100
                    )
                })
                .unwrap();
            // Equal publication numbers and selection cannot transfer authority.
            chart.inspection_content[0].target = Some(Target::Aggregate {
                source: foreign,
                data_revision: revision,
                data_generation: original.generation(),
                selection,
            });
            apply(
                cx,
                handle,
                vec![Op::SetChart(id(1), Box::new(chart.clone()))],
            );
            assert!(!eligible(cx, handle, node));
            chart.inspection_content[0].target = Some(target);
            apply(
                cx,
                handle,
                vec![Op::SetChart(id(1), Box::new(chart.clone()))],
            );
            draw(cx, handle);
            let point = handle
                .update(cx, |view, _, _| view.probes.borrow()[&node].bounds.center())
                .unwrap();
            move_mouse(cx, handle, point, false);
            presses(transport, node);
            mouse(cx, handle, point, true);
            let mut changed = data.clone();
            let index = usize::from(count > 1);
            match &mut changed.contents {
                Contents::Cartesian(layers) => {
                    let Layer::Bar(series) = &mut layers[0] else {
                        unreachable!()
                    };
                    if count > 1 {
                        series.points[index].id = 20;
                    }
                    series.points[index].y = Some(99.);
                }
                Contents::Candlestick(values) => {
                    if count > 1 {
                        values[index].id = 20;
                    }
                    values[index].high = 99.;
                }
                _ => unreachable!(),
            }
            let next = cx.update(|cx| publish(session, source, &changed, cx));
            assert!(
                !eligible(cx, handle, node),
                "new revision retires aggregate before paint despite equal endpoints/count"
            );
            ready(cx, handle, next, 0xff0000ff).await;
            handle
                .update(cx, |view, _, _| {
                    assert!(
                        view.charts[&id(1)].borrow().input.selected.is_none(),
                        "aggregate selection itself does not cross publications"
                    )
                })
                .unwrap();
            assert!(
                !eligible(cx, handle, node),
                "old publication stays hidden after replacement geometry"
            );
            chart.inspection_content[0].target = Some(Target::Aggregate {
                source,
                data_revision: next,
                data_generation: original.generation(),
                selection,
            });
            apply(
                cx,
                handle,
                vec![Op::SetChart(id(1), Box::new(chart.clone()))],
            );
            draw(cx, handle);
            assert!(
                !eligible(cx, handle, node),
                "new metadata does not manufacture a new native preview"
            );
            handle
                .update(cx, |view, window, cx| {
                    window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
                })
                .unwrap();
            key(cx, handle, "home");
            key(cx, handle, "enter");
            draw(cx, handle);
            assert!(eligible(cx, handle, node));
            mouse(cx, handle, point, false);
            assert_eq!(
                presses(transport, node),
                0,
                "rebinding content cannot finish the retired gesture"
            );
            move_mouse(cx, handle, point, false);
            mouse(cx, handle, point, true);
            mouse(cx, handle, point, false);
            assert_eq!(
                presses(transport, node),
                1,
                "fresh gesture recovers on explicit rebinding"
            );
            apply(
                cx,
                handle,
                vec![
                    Op::SetChart(id(1), Box::new(config(source, 0xff0000ff))),
                    Op::Splice(id(1), 0, 1, vec![]),
                    Op::Remove(node),
                    Op::Remove(wrapper),
                ],
            );
            generation += 1;
        }
    }
    let revision = cx.update(|cx| publish(session, source, original.data(), cx));
    ready(cx, handle, revision, 0xff0000ff).await;
    assert_eq!(immediate(session, Request::Release(foreign)), Response::Ack);
    presses(transport, NodeId::from_parts(3, generation - 1).unwrap());
    eprintln!(
        "GPUIO_INSPECTION_AGGREGATE_OK: Sum/Mean/OHLC one/four-value GPU content, singular/foreign-source rejection, publication retirement despite equal spans, stale gesture rejection and explicit rebinding recovery"
    );
}
