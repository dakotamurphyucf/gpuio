//! Resolve private geometry provenance against its exact immutable dataset.
//! Never pass a newer publication here, even when it reuses the same stable IDs.
use crate::{
    chart_cartesian::{Kind, Layers},
    chart_geometry::Source,
};
use gpuio_protocol::{
    chart_data::{Contents, Data},
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
        (
            Contents::Cartesian(_) | Contents::Categorical(..),
            Source::Cartesian { series, start, end },
        ) => {
            let layer = Layers::of(data)?.get(series)?;
            if end > layer.points.len() || start >= end {
                return None;
            }
            let span = Span {
                start_index: start as i64,
                length: (end - start) as i64,
                first: layer.points.get(start)?.id,
                last: layer.points.get(end - 1)?.id,
            };
            if !span.is_valid() {
                return None;
            }
            let aggregation = match layer.kind {
                Kind::Line | Kind::Area => Aggregation::Exact,
                Kind::Bar => match policy.bars {
                    Bar::Exact => Aggregation::Exact,
                    Bar::Sum(_) => Aggregation::Sum,
                    Bar::Mean(_) => Aggregation::Mean,
                },
            };
            Selection::Cartesian {
                series: layer.id,
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

/// Singular identities survive publication without reusing old source indices.
/// Aggregates deliberately have no stable key: endpoint IDs don't prove that
/// their interior membership remains the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Cartesian(i64, i64),
    Slice(i64),
    Radar(i64, i64),
    Candle(i64),
    Node(i64),
    Edge(i64),
}
fn key(selection: Selection) -> Option<Key> {
    match selection {
        Selection::Cartesian {
            series,
            span,
            aggregation: Aggregation::Exact,
        } => Some(Key::Cartesian(series, span.first)),
        Selection::Cartesian { .. }
        | Selection::Candlestick {
            aggregated: true, ..
        } => None,
        Selection::Slice(id) => Some(Key::Slice(id)),
        Selection::Radar { series, axis } => Some(Key::Radar(series, axis)),
        Selection::Candlestick {
            span,
            aggregated: false,
        } => Some(Key::Candle(span.first)),
        Selection::Node(id) => Some(Key::Node(id)),
        Selection::Edge(id) => Some(Key::Edge(id)),
    }
}
pub struct StableIndex(Vec<(Key, usize)>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexError {
    Cancelled,
    InvalidSource,
    LimitExceeded,
}
impl StableIndex {
    pub fn prepare(
        data: &Data,
        policy: &Policy,
        plan: &crate::chart_geometry::Plan,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Self, IndexError> {
        use std::sync::atomic::Ordering;
        if plan.marks.len() > 100_000 {
            return Err(IndexError::LimitExceeded);
        }
        let mut entries = Vec::new();
        for (index, mark) in plan.marks.iter().enumerate() {
            if index % 256 == 0 && cancel.load(Ordering::Relaxed) {
                return Err(IndexError::Cancelled);
            }
            let selection = resolve(data, policy, mark.source).ok_or(IndexError::InvalidSource)?;
            if let Some(key) = key(selection) {
                entries.push((key, index));
            }
        }
        entries.sort_unstable_by_key(|(key, _)| *key);
        if cancel.load(Ordering::Relaxed) {
            return Err(IndexError::Cancelled);
        }
        if entries.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(IndexError::InvalidSource);
        }
        let index = Self(entries);
        if index.retained_bytes() > 8 * 1024 * 1024 {
            return Err(IndexError::LimitExceeded);
        }
        Ok(index)
    }
    pub fn find(&self, selection: Selection) -> Option<usize> {
        let key = key(selection)?;
        self.0
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok()
            .map(|i| self.0[i].1)
    }
    pub fn retained_bytes(&self) -> usize {
        self.0.capacity() * std::mem::size_of::<(Key, usize)>()
    }
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
        for fixture in include_str!("../../../test/fixtures/chart-v2-data.hex").lines() {
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
            version: 2,
            bar_backgrounds: vec![],
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
            version: 2,
            bar_backgrounds: vec![],
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
            version: 2,
            bar_backgrounds: vec![],
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
    #[test]
    fn stable_lookup_follows_identity_after_source_positions_change() {
        let original = Data {
            version: 2,
            bar_backgrounds: vec![],
            contents: Contents::Cartesian(vec![Layer::Line(Series {
                id: 9,
                name: "Series".into(),
                points: [42, 7]
                    .into_iter()
                    .enumerate()
                    .map(|(i, id)| Point {
                        id,
                        x: i as f64,
                        y: Some(1.),
                        label: String::new(),
                    })
                    .collect(),
            })]),
        };
        let policy = Policy::default();
        let old_plan = plan(&original, policy);
        let old = resolve(&original, &policy, old_plan.marks[1].source).unwrap();
        let mut updated = original.clone();
        let Contents::Cartesian(layers) = &mut updated.contents else {
            unreachable!()
        };
        let Layer::Line(series) = &mut layers[0] else {
            unreachable!()
        };
        series.points.remove(0);
        let new_plan = plan(&updated, policy);
        let index =
            StableIndex::prepare(&updated, &policy, &new_plan, &AtomicBool::new(false)).unwrap();
        assert_eq!(index.find(old), Some(0));
        assert_eq!(
            index.find(resolve(&original, &policy, old_plan.marks[0].source).unwrap()),
            None
        );
        assert!(index.retained_bytes() >= std::mem::size_of::<(Key, usize)>());
        assert!(matches!(
            StableIndex::prepare(&updated, &policy, &new_plan, &AtomicBool::new(true)),
            Err(IndexError::Cancelled)
        ));
    }
    #[test]
    fn aggregated_membership_is_not_inferred_from_endpoint_identity() {
        let data = Data {
            version: 2,
            bar_backgrounds: vec![],
            contents: Contents::Cartesian(vec![Layer::Bar(Series {
                id: 9,
                name: "One sample still aggregated".into(),
                points: vec![Point {
                    id: 7,
                    x: 0.,
                    y: Some(2.),
                    label: String::new(),
                }],
            })]),
        };
        let policy = Policy {
            bars: Bar::Sum(1),
            ..Default::default()
        };
        let plan = plan(&data, policy);
        let selection = resolve(&data, &policy, plan.marks[0].source).unwrap();
        let index = StableIndex::prepare(&data, &policy, &plan, &AtomicBool::new(false)).unwrap();
        assert_eq!(index.find(selection), None);
        assert_eq!(index.retained_bytes(), 0);
    }
}
