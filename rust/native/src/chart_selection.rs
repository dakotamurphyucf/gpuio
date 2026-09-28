//! Resolve private geometry provenance against its exact immutable dataset.
//! Never pass a newer publication here, even when it reuses the same stable IDs.
use crate::chart_geometry::Source;
use gpuio_protocol::{
    chart_data::{Contents, Data, Layer},
    chart_sampling::{Bar, Candlestick, Policy},
    chart_selection::{Aggregation, Selection, Span},
};

fn span<T>(values: &[T], start: usize, end: usize, id: impl Fn(&T) -> i64) -> Option<Span> {
    let values = values.get(start..end)?;
    let span = Span {
        start_index: i64::try_from(start).ok()?,
        length: i64::try_from(values.len()).ok()?,
        first: id(values.first()?),
        last: id(values.last()?),
    };
    span.is_valid().then_some(span)
}
/// Data and policy have already passed resource validation. Bounds and family
/// checks here also reject malformed/stale internal provenance without panicking.
pub fn resolve(data: &Data, policy: &Policy, source: Source) -> Option<Selection> {
    let target = match (&data.contents, source) {
        (Contents::Cartesian(layers), Source::Cartesian { series, start, end }) => {
            let layer = layers.get(series)?;
            let series = layer.series();
            let span = span(&series.points, start, end, |p| p.id)?;
            let aggregation = match layer {
                Layer::Line(_) | Layer::Area(_) => Aggregation::Exact,
                Layer::Bar(_) => match policy.bars {
                    Bar::Exact => Aggregation::Exact,
                    Bar::Sum(_) => Aggregation::Sum,
                    Bar::Mean(_) => Aggregation::Mean,
                },
            };
            Selection::Cartesian {
                series: series.id,
                span,
                aggregation,
            }
        }
        (Contents::Pie(values), Source::Slice(index)) => Selection::Slice(values.get(index)?.id),
        (Contents::Radar(axes, values), Source::Radar { series, axis }) => Selection::Radar {
            series: values.get(series)?.id,
            axis: axes.get(axis)?.id,
        },
        (Contents::Candlestick(values), Source::Candle(source)) => Selection::Candlestick {
            span: span(values, source.start(), source.end(), |c| c.id)?,
            aggregated: matches!(policy.candles, Candlestick::Ohlc(_)),
        },
        (Contents::Sankey(nodes, _), Source::Node(index)) => Selection::Node(nodes.get(index)?.id),
        (Contents::Sankey(_, edges), Source::Edge(index)) => Selection::Edge(edges.get(index)?.id),
        _ => return None,
    };
    target.is_valid().then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_geometry;
    use gpuio_protocol::{chart_data::*, chart_options::Options};
    use std::sync::atomic::AtomicBool;
    fn plan(data: &Data, policy: Policy) -> chart_geometry::Plan {
        chart_geometry::prepare(
            data,
            policy,
            &Options::default(),
            640.,
            320.,
            &AtomicBool::new(false),
        )
        .unwrap()
    }
    #[test]
    fn actual_geometry_for_every_family_resolves_to_stable_source_ids() {
        for fixture in include_str!("../../../test/fixtures/chart-v1-data.hex").lines() {
            let (_, hex) = fixture.split_once(' ').unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            let data = gpuio_protocol::decode_chart_data(&bytes).unwrap();
            let policy = Policy::default();
            let plan = plan(&data, policy);
            assert!(!plan.marks.is_empty());
            for mark in plan.marks {
                let selection = resolve(&data, &policy, mark.source).unwrap();
                assert!(selection.is_valid());
                match (&data.contents, selection) {
                    (
                        Contents::Cartesian(layers),
                        Selection::Cartesian {
                            series,
                            span,
                            aggregation,
                        },
                    ) => {
                        assert_eq!(aggregation, Aggregation::Exact);
                        let series = layers
                            .iter()
                            .map(Layer::series)
                            .find(|s| s.id == series)
                            .unwrap();
                        assert_eq!(span.length, 1);
                        assert_eq!(series.points[span.start_index as usize].id, span.first);
                    }
                    (Contents::Pie(_), Selection::Slice(7)) => (),
                    (
                        Contents::Radar(_, _),
                        Selection::Radar {
                            series: 9,
                            axis: 1..=3,
                        },
                    ) => (),
                    (
                        Contents::Candlestick(_),
                        Selection::Candlestick {
                            span,
                            aggregated: false,
                        },
                    ) => assert_eq!(span.first, 1),
                    (Contents::Sankey(_, _), Selection::Node(1..=2) | Selection::Edge(5)) => (),
                    other => panic!("unexpected fixture selection {other:?}"),
                }
            }
        }
    }
    #[test]
    fn aggregates_preserve_original_membership_without_assuming_id_order() {
        let points = [42, 99, 7]
            .into_iter()
            .enumerate()
            .map(|(i, id)| Point {
                id,
                x: i as f64,
                y: Some((i + 1) as f64),
                label: String::new(),
            })
            .collect::<Vec<_>>();
        let data = Data {
            version: 1,
            contents: Contents::Cartesian(vec![Layer::Bar(Series {
                id: 9,
                name: "Sums".into(),
                points: points.clone(),
            })]),
        };
        for (bars, aggregation) in [
            (Bar::Sum(1), Aggregation::Sum),
            (Bar::Mean(1), Aggregation::Mean),
        ] {
            let policy = Policy {
                bars,
                ..Default::default()
            };
            let plan = plan(&data, policy);
            assert_eq!(plan.marks.len(), 1);
            assert_eq!(
                resolve(&data, &policy, plan.marks[0].source),
                Some(Selection::Cartesian {
                    series: 9,
                    span: Span {
                        start_index: 0,
                        length: 3,
                        first: 42,
                        last: 7
                    },
                    aggregation,
                })
            );
        }
        let candles = Data {
            version: 1,
            contents: Contents::Candlestick(
                points
                    .iter()
                    .map(|p| Candle {
                        id: p.id,
                        x: p.x,
                        label: String::new(),
                        open_: 0.,
                        high: 2.,
                        low: -1.,
                        close: 1.,
                    })
                    .collect(),
            ),
        };
        let policy = Policy {
            candles: Candlestick::Ohlc(1),
            ..Default::default()
        };
        let plan = plan(&candles, policy);
        assert_eq!(plan.marks.len(), 1);
        assert_eq!(
            resolve(&candles, &policy, plan.marks[0].source),
            Some(Selection::Candlestick {
                span: Span {
                    start_index: 0,
                    length: 3,
                    first: 42,
                    last: 7
                },
                aggregated: true,
            })
        );
        let invalid = [
            Source::Cartesian {
                series: 0,
                start: 0,
                end: usize::MAX,
            },
            Source::Cartesian {
                series: 0,
                start: 2,
                end: 1,
            },
            Source::Cartesian {
                series: usize::MAX,
                start: 0,
                end: 1,
            },
            Source::Cartesian {
                series: 0,
                start: 1,
                end: 1,
            },
            Source::Slice(0),
        ];
        for source in invalid {
            assert_eq!(resolve(&data, &policy, source), None);
        }
    }
    #[test]
    fn line_envelopes_refer_to_original_representatives_not_an_aggregate_span() {
        let points = (0..1000)
            .map(|i| Point {
                id: 1000 - i,
                x: i as f64,
                y: Some((i % 13) as f64),
                label: String::new(),
            })
            .collect();
        let data = Data {
            version: 1,
            contents: Contents::Cartesian(vec![Layer::Line(Series {
                id: 8,
                name: "Line".into(),
                points,
            })]),
        };
        let policy = Policy {
            line: gpuio_protocol::chart_sampling::Line::Envelope(1),
            ..Default::default()
        };
        let plan = plan(&data, policy);
        assert!(plan.marks.len() <= 4);
        for mark in plan.marks {
            let Some(Selection::Cartesian {
                span, aggregation, ..
            }) = resolve(&data, &policy, mark.source)
            else {
                panic!("line target");
            };
            assert_eq!(aggregation, Aggregation::Exact);
            assert_eq!(span.length, 1);
            assert_eq!(span.first, 1000 - span.start_index);
        }
    }
}
