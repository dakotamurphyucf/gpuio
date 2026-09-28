//! Bounded tooltip descriptions of the exact prepared source. Aggregate values
//! are retained during worker reduction, never recomputed from a large span here.
use crate::chart_geometry::{self as geometry, Plan, Point, Shape, Source, Summary};
use gpuio_protocol::{
    chart_data::{Contents, Data},
    chart_options::Options,
    chart_sampling::{Bar, Policy},
};

pub(crate) struct Details {
    pub title: String,
    pub text: String,
    pub anchor: Point,
}
fn anchor(shape: Shape) -> Point {
    match shape {
        Shape::Dot { center, .. } => center,
        Shape::Bar(r) | Shape::Node(r) => Point {
            x: (r.left + r.right) / 2.,
            y: (r.top + r.bottom) / 2.,
        },
        Shape::Candle {
            center,
            open,
            close,
            ..
        } => Point {
            x: center,
            y: (open + close) / 2.,
        },
        Shape::Wedge {
            center,
            inner,
            outer,
            start,
            end,
        } => {
            let angle = (start + end) / 2.;
            let radius = (inner + outer) / 2.;
            Point {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            }
        }
        Shape::Ribbon {
            start_top: a,
            start_bottom: b,
            end_top: c,
            end_bottom: d,
        } => Point {
            x: (a.x + c.x) / 2.,
            y: (a.y + b.y + c.y + d.y) / 4.,
        },
    }
}
pub(crate) fn describe(
    data: &Data,
    policy: &Policy,
    options: &Options,
    plan: &Plan,
    index: usize,
) -> Option<Details> {
    let mark = plan.marks.get(index)?;
    let x = |n| geometry::format_number(n, options.axes.x_format);
    let y = |n| geometry::format_number(n, options.axes.y_format);
    let (title, text) = match (&data.contents, mark.source) {
        (Contents::Cartesian(layers), Source::Cartesian { series, start, end }) => {
            let series = layers.get(series)?.series();
            let first = series.points.get(start)?;
            let count = end.checked_sub(start)?;
            let last = series.points.get(end.checked_sub(1)?)?;
            if count == 0 {
                return None;
            }
            let value = match plan.summary(index) {
                Some(Summary::Bar(value)) => value,
                None if count == 1 => first.y?,
                _ => return None,
            };
            let aggregate =
                matches!(mark.shape, Shape::Bar(_)) && !matches!(policy.bars, Bar::Exact);
            let text = if aggregate {
                let operation = match policy.bars {
                    Bar::Sum(_) => "Sum",
                    Bar::Mean(_) => "Mean",
                    Bar::Exact => return None,
                };
                format!(
                    "{operation} of {count} samples\nx: {} – {}\ny: {}",
                    x(first.x),
                    x(last.x),
                    y(value)
                )
            } else {
                let label = if first.label.is_empty() {
                    String::new()
                } else {
                    format!("{}\n", first.label)
                };
                format!("{label}x: {}\ny: {}", x(first.x), y(value))
            };
            (series.name.clone(), text)
        }
        (Contents::Pie(values), Source::Slice(index)) => {
            let slice = values.get(index)?;
            let total: f64 = values.iter().map(|s| s.value).sum();
            (
                slice.label.clone(),
                format!(
                    "{} · {:.1}%",
                    y(slice.value),
                    if total > 0. {
                        slice.value / total * 100.
                    } else {
                        0.
                    }
                ),
            )
        }
        (
            Contents::Radar(axes, series),
            Source::Radar {
                series: index,
                axis,
            },
        ) => {
            let series = series.get(index)?;
            let axis = axes.get(axis)?;
            let value = series.values.iter().find(|(id, _)| *id == axis.id)?.1;
            (
                series.name.clone(),
                format!("{}\n{} / {}", axis.label, y(value), y(axis.maximum)),
            )
        }
        (Contents::Candlestick(values), Source::Candle(span)) => {
            let first = values.get(span.start())?;
            let last = values.get(span.end().checked_sub(1)?)?;
            let (open, high, low, close) = match plan.summary(index) {
                Some(Summary::Candle {
                    open,
                    high,
                    low,
                    close,
                }) => (open, high, low, close),
                None if span.len() == 1 => (first.open_, first.high, first.low, first.close),
                _ => return None,
            };
            let title = if span.len() > 1 {
                format!("OHLC · {} samples", span.len())
            } else if first.label.is_empty() {
                "Price".into()
            } else {
                first.label.clone()
            };
            (
                title,
                format!(
                    "x: {} – {}\nOpen {} · High {}\nLow {} · Close {}",
                    x(first.x),
                    x(last.x),
                    y(open),
                    y(high),
                    y(low),
                    y(close)
                ),
            )
        }
        (Contents::Sankey(nodes, edges), Source::Node(index)) => {
            let node = nodes.get(index)?;
            // Source schema bounds this traversal to 2,048 edges, independent of
            // Cartesian/candle datasets. Recomputed only when details are built.
            let incoming: f64 = edges
                .iter()
                .filter(|e| e.target == node.id)
                .map(|e| e.value)
                .sum();
            let outgoing: f64 = edges
                .iter()
                .filter(|e| e.source == node.id)
                .map(|e| e.value)
                .sum();
            (
                node.label.clone(),
                format!("Incoming {}\nOutgoing {}", y(incoming), y(outgoing)),
            )
        }
        (Contents::Sankey(nodes, edges), Source::Edge(index)) => {
            let edge = edges.get(index)?;
            let source = nodes.iter().find(|n| n.id == edge.source)?;
            let target = nodes.iter().find(|n| n.id == edge.target)?;
            (
                format!("{} → {}", source.label, target.label),
                format!("Flow {}", y(edge.value)),
            )
        }
        _ => return None,
    };
    Some(Details {
        title,
        text,
        anchor: anchor(mark.shape),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::{
        chart_data::{Point, *},
        chart_sampling::Candlestick,
    };
    use std::sync::atomic::AtomicBool;
    fn prepare(data: &Data, policy: Policy) -> Plan {
        geometry::prepare(
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
    fn aggregate_tooltips_use_reduced_values_and_original_interval() {
        let data = Data {
            version: 1,
            contents: Contents::Cartesian(vec![Layer::Bar(Series {
                id: 9,
                name: "Throughput".into(),
                points: [2., 6., 10.]
                    .into_iter()
                    .enumerate()
                    .map(|(i, y)| Point {
                        id: i as i64 + 1,
                        x: i as f64,
                        y: Some(y),
                        label: String::new(),
                    })
                    .collect(),
            })]),
        };
        for (bars, value, operation) in [(Bar::Sum(1), 18., "Sum"), (Bar::Mean(1), 6., "Mean")] {
            let policy = Policy {
                bars,
                ..Default::default()
            };
            let plan = prepare(&data, policy);
            assert_eq!(plan.summary(0), Some(Summary::Bar(value)));
            let details = describe(&data, &policy, &Options::default(), &plan, 0).unwrap();
            assert_eq!(details.title, "Throughput");
            assert_eq!(
                details.text,
                format!("{operation} of 3 samples\nx: 0 – 2\ny: {value}")
            );
            assert!(describe(&data, &policy, &Options::default(), &plan, 1).is_none());
            assert!(
                plan.retained_bytes()
                    >= plan.summaries.capacity() * std::mem::size_of::<(usize, Summary)>()
            );
        }
        let candles = Data {
            version: 1,
            contents: Contents::Candlestick(vec![
                Candle {
                    id: 42,
                    x: 0.,
                    label: String::new(),
                    open_: 2.,
                    high: 9.,
                    low: 1.,
                    close: 4.,
                },
                Candle {
                    id: 7,
                    x: 1.,
                    label: String::new(),
                    open_: 3.,
                    high: 8.,
                    low: -2.,
                    close: 6.,
                },
            ]),
        };
        let policy = Policy {
            candles: Candlestick::Ohlc(1),
            ..Default::default()
        };
        let plan = prepare(&candles, policy);
        let details = describe(&candles, &policy, &Options::default(), &plan, 0).unwrap();
        assert_eq!(details.title, "OHLC · 2 samples");
        assert_eq!(details.text, "x: 0 – 1\nOpen 2 · High 9\nLow -2 · Close 6");
    }
    #[test]
    fn every_family_describes_actual_marks_and_rejects_unrelated_sources() {
        for fixture in include_str!("../../../test/fixtures/chart-v1-data.hex").lines() {
            let (_, hex) = fixture.split_once(' ').unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            let data = gpuio_protocol::decode_chart_data(&bytes).unwrap();
            let policy = Policy::default();
            let plan = prepare(&data, policy);
            for index in 0..plan.marks.len() {
                let details = describe(&data, &policy, &Options::default(), &plan, index).unwrap();
                assert!(!details.title.is_empty());
                assert!(!details.text.is_empty());
                assert!(details.anchor.x.is_finite() && details.anchor.y.is_finite());
                let empty = Data {
                    version: 1,
                    contents: Contents::Pie(vec![]),
                };
                assert!(describe(&empty, &policy, &Options::default(), &plan, index).is_none());
            }
        }
    }
}
