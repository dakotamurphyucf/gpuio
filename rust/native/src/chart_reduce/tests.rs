use super::*;
use data::{Data, Layer, Point};

fn points(values: &[Option<f64>]) -> Vec<Point> {
    values
        .iter()
        .enumerate()
        .map(|(i, &y)| Point {
            id: 1000 - i as i64,
            x: i as f64,
            y,
            label: String::new(),
        })
        .collect()
}
fn dataset(points: Vec<Point>) -> Data {
    Data {
        version: 1,
        contents: data::Contents::Cartesian(vec![Layer::Line(data::Series {
            id: 1,
            name: "Signal".into(),
            points,
        })]),
    }
}
fn reduce(data: &Data, policy: policy::Policy, width: f64) -> Reduction {
    prepare(data, policy, width, &AtomicBool::new(false)).unwrap()
}
fn line_points(result: &Reduction) -> &[LinePoint] {
    let Contents::Cartesian(series) = &result.contents else {
        panic!()
    };
    let Series::Line(points) = &series[0] else {
        panic!()
    };
    points
}

#[test]
fn extrema_keep_source_order_ids_and_gaps_inside_one_bucket() {
    let data = dataset(points(&[
        Some(0.),
        Some(9.),
        Some(-7.),
        Some(2.),
        None,
        Some(5.),
        Some(1.),
        None,
        Some(0.),
    ]));
    let result = reduce(&data, policy::Policy::default(), 1.);
    assert_eq!(
        line_points(&result),
        &[
            LinePoint {
                source: 0,
                starts_run: true
            },
            LinePoint {
                source: 1,
                starts_run: false
            },
            LinePoint {
                source: 2,
                starts_run: false
            },
            LinePoint {
                source: 3,
                starts_run: false
            },
            LinePoint {
                source: 5,
                starts_run: true
            },
            LinePoint {
                source: 6,
                starts_run: false
            },
            LinePoint {
                source: 8,
                starts_run: true
            },
        ]
    );
    let data::Contents::Cartesian(source) = &data.contents else {
        panic!()
    };
    assert_eq!(source[0].series().points[2].id, 998);
    assert_eq!(result.source_values, 9);
    assert_eq!(result.rendered_values, 7);
}

// Independent grouping oracle: collect each connected run, partition by an
// integer x-bin formula, then select extrema/endpoints from complete groups.
fn oracle(values: &[Option<f64>], buckets: usize) -> Vec<LinePoint> {
    let mut result = vec![];
    let mut start = 0;
    while start < values.len() {
        if values[start].is_none() {
            start += 1;
            continue;
        }
        let end = (start..values.len())
            .find(|&i| values[i].is_none())
            .unwrap_or(values.len());
        let mut groups = std::collections::BTreeMap::<usize, Vec<usize>>::new();
        for index in start..end {
            groups
                .entry((index * buckets / (values.len() - 1)).min(buckets - 1))
                .or_default()
                .push(index);
        }
        let mut first = true;
        for indices in groups.values() {
            let low = *indices
                .iter()
                .min_by(|&&a, &&b| {
                    values[a]
                        .unwrap()
                        .partial_cmp(&values[b].unwrap())
                        .unwrap()
                        .then(a.cmp(&b))
                })
                .unwrap();
            let high = *indices
                .iter()
                .min_by(|&&a, &&b| {
                    values[b]
                        .unwrap()
                        .partial_cmp(&values[a].unwrap())
                        .unwrap()
                        .then(a.cmp(&b))
                })
                .unwrap();
            let mut selected = vec![indices[0], *indices.last().unwrap(), low, high];
            selected.sort_unstable();
            selected.dedup();
            for source in selected {
                result.push(LinePoint {
                    source,
                    starts_run: first,
                });
                first = false;
            }
        }
        start = end;
    }
    result
}

#[test]
fn all_small_gap_extrema_patterns_agree_with_independent_grouping() {
    for code in 0..4usize.pow(7) {
        let values: Vec<_> = (0..7)
            .map(|i| match code / 4usize.pow(i) % 4 {
                0 => None,
                n => Some(n as f64 - 2.),
            })
            .collect();
        let data = dataset(points(&values));
        for bins in [1, 2, 3, 7] {
            let result = reduce(&data, policy::Policy::default(), bins as f64);
            assert_eq!(
                line_points(&result),
                oracle(&values, bins),
                "code={code}, bins={bins}"
            );
        }
    }
}

#[test]
fn bins_use_numeric_shared_domain_not_each_series_array_index() {
    let mut a = points(&[Some(1.); 5]);
    for (point, x) in a.iter_mut().zip([0., 1., 2., 99., 100.]) {
        point.x = x;
    }
    let b = vec![Point {
        id: 1,
        x: 200.,
        y: Some(3.),
        label: String::new(),
    }];
    let data = Data {
        version: 1,
        contents: data::Contents::Cartesian(vec![
            Layer::Bar(data::Series {
                id: 1,
                name: "A".into(),
                points: a,
            }),
            Layer::Bar(data::Series {
                id: 2,
                name: "B".into(),
                points: b,
            }),
        ]),
    };
    let result = reduce(
        &data,
        policy::Policy {
            bars: policy::Bar::Sum(2),
            ..Default::default()
        },
        800.,
    );
    let Contents::Cartesian(series) = result.contents else {
        panic!()
    };
    let Series::Bar(a) = &series[0] else { panic!() };
    assert_eq!(
        a,
        &[
            Bar {
                source: SourceSpan { start: 0, end: 4 },
                x: 49.5,
                value: 4.
            },
            Bar {
                source: SourceSpan { start: 4, end: 5 },
                x: 100.,
                value: 1.
            },
        ]
    );
    let Series::Bar(b) = &series[1] else { panic!() };
    assert_eq!(b[0].value, 3.);
}

#[test]
fn bars_use_explicit_compensated_sum_or_mean_and_complete_provenance() {
    let mut data = dataset(points(&[Some(1e100), Some(1.), Some(-1e100)]));
    let data::Contents::Cartesian(layers) = &mut data.contents else {
        panic!()
    };
    let series = layers[0].series().clone();
    layers[0] = Layer::Bar(series);
    for (policy, expected, count) in [
        (policy::Bar::Sum(1), 1., 1),
        (policy::Bar::Mean(1), 1. / 3., 1),
        (policy::Bar::Exact, 1e100, 3),
    ] {
        let result = reduce(
            &data,
            policy::Policy {
                bars: policy,
                ..Default::default()
            },
            500.,
        );
        let Contents::Cartesian(series) = result.contents else {
            panic!()
        };
        let Series::Bar(bars) = &series[0] else {
            panic!()
        };
        assert_eq!(bars.len(), count);
        assert_eq!(bars[0].value, expected);
        let spans: Vec<_> = bars
            .iter()
            .flat_map(|b| b.source.start()..b.source.end())
            .collect();
        assert_eq!(spans, vec![0, 1, 2]);
    }
}

#[test]
fn candles_aggregate_ohlc_and_never_inherit_line_reduction() {
    let data = Data {
        version: 1,
        contents: data::Contents::Candlestick(vec![
            data::Candle {
                id: 9,
                x: 0.,
                label: "A".into(),
                open_: -4.,
                high: -1.,
                low: -7.,
                close: -2.,
            },
            data::Candle {
                id: 2,
                x: 2.,
                label: "B".into(),
                open_: -2.,
                high: 5.,
                low: -3.,
                close: 4.,
            },
            data::Candle {
                id: 7,
                x: 3.,
                label: "C".into(),
                open_: 4.,
                high: 4.,
                low: 1.,
                close: 1.,
            },
        ]),
    };
    let exact = reduce(&data, policy::Policy::default(), 1.);
    assert_eq!(exact.rendered_values, 3);
    let reduced = reduce(
        &data,
        policy::Policy {
            candles: policy::Candlestick::Ohlc(1),
            ..Default::default()
        },
        1.,
    );
    assert_eq!(
        reduced.contents,
        Contents::Candlestick(vec![Candle {
            source: SourceSpan { start: 0, end: 3 },
            x: 1.5,
            open: -4.,
            high: 5.,
            low: -7.,
            close: 1.,
        }])
    );
}

#[test]
fn mixed_layers_keep_family_policies_and_many_gaps_never_collapse() {
    let values = vec![Some(1.), None, Some(-1.), None, Some(2.)];
    let line = data::Series {
        id: 1,
        name: "Line".into(),
        points: points(&values),
    };
    let mut area = line.clone();
    area.id = 2;
    area.name = "Area".into();
    let bars = data::Series {
        id: 3,
        name: "Bar".into(),
        points: points(&[Some(1.); 5]),
    };
    let data = Data {
        version: 1,
        contents: data::Contents::Cartesian(vec![
            Layer::Line(line),
            Layer::Area(area),
            Layer::Bar(bars),
        ]),
    };
    let result = reduce(
        &data,
        policy::Policy {
            bars: policy::Bar::Mean(1),
            ..Default::default()
        },
        1.,
    );
    let Contents::Cartesian(series) = result.contents else {
        panic!()
    };
    let (Series::Line(line), Series::Area(area), Series::Bar(bars)) =
        (&series[0], &series[1], &series[2])
    else {
        panic!()
    };
    assert_eq!(line, area);
    assert!(line.iter().all(|p| p.starts_run));
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].source.len(), 5);
    assert!(!bars[0].source.is_empty());

    let many_gaps = dataset(
        (0..100_000)
            .map(|i| Point {
                id: i + 1,
                x: i as f64,
                y: (i % 2 == 0).then_some(1.),
                label: String::new(),
            })
            .collect(),
    );
    let result = reduce(&many_gaps, policy::Policy::default(), 1.);
    assert_eq!(result.rendered_values, 50_000);
    assert!(line_points(&result).iter().all(|p| p.starts_run));
    assert!(result.retained_bytes() < 2 * 1024 * 1024);
}

#[test]
fn empty_degenerate_extreme_and_subnormal_inputs_remain_defined() {
    for contents in [
        data::Contents::Cartesian(vec![]),
        data::Contents::Candlestick(vec![]),
        data::Contents::Pie(vec![]),
        data::Contents::Radar(vec![], vec![]),
        data::Contents::Sankey(vec![], vec![]),
    ] {
        let result = reduce(
            &Data {
                version: 1,
                contents,
            },
            policy::Policy::default(),
            0.5,
        );
        assert_eq!(result.rendered_values, 0);
    }
    let tiny = f64::from_bits(1);
    let candle = data::Candle {
        id: 1,
        x: tiny,
        label: String::new(),
        open_: tiny,
        high: tiny,
        low: tiny,
        close: tiny,
    };
    let result = reduce(
        &Data {
            version: 1,
            contents: data::Contents::Candlestick(vec![candle]),
        },
        policy::Policy::default(),
        1.,
    );
    let Contents::Candlestick(candles) = result.contents else {
        panic!()
    };
    assert_eq!(candles[0].x, tiny);
    let mut data = dataset(points(&[Some(-1e100), Some(1e100)]));
    let data::Contents::Cartesian(layers) = &mut data.contents else {
        panic!()
    };
    let Layer::Line(series) = &mut layers[0] else {
        panic!()
    };
    series.points[0].x = -1e100;
    series.points[1].x = 1e100;
    assert_eq!(
        reduce(&data, policy::Policy::default(), 32768.).rendered_values,
        2
    );
    let gaps = reduce(&dataset(points(&[None; 8])), policy::Policy::default(), 1.);
    assert_eq!(gaps.rendered_values, 0);
}

#[test]
fn full_dataset_is_bounded_and_exact_is_an_explicit_choice() {
    let data = dataset(
        (0..100_000)
            .map(|i| Point {
                id: i + 1,
                x: i as f64,
                y: Some((i % 23) as f64 - 11.),
                label: String::new(),
            })
            .collect(),
    );
    let reduced = reduce(&data, policy::Policy::default(), 800.);
    assert!(reduced.rendered_values <= 3200);
    assert!(reduced.retained_bytes() < 128 * 1024);
    let exact = reduce(
        &data,
        policy::Policy {
            line: policy::Line::Exact,
            ..Default::default()
        },
        800.,
    );
    assert_eq!(exact.rendered_values, 100_000);
    assert!(exact.retained_bytes() < 2 * 1024 * 1024);
    let Contents::Cartesian(series) = &reduced.contents else {
        panic!()
    };
    assert_eq!(series[0].len(), reduced.rendered_values);
    assert!(!series[0].is_empty());
}

#[test]
fn invalid_inputs_and_cancellation_never_return_partial_results() {
    let data = dataset(points(&[Some(0.)]));
    assert_eq!(
        prepare(
            &data,
            policy::Policy::default(),
            100.,
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    );
    for width in [0., -1., f64::NAN, f64::INFINITY, 32769.] {
        assert_eq!(
            prepare(
                &data,
                policy::Policy::default(),
                width,
                &AtomicBool::new(false)
            ),
            Err(Error::InvalidWidth)
        );
    }
    assert_eq!(
        prepare(
            &data,
            policy::Policy {
                line: policy::Line::Envelope(0),
                ..Default::default()
            },
            100.,
            &AtomicBool::new(false)
        ),
        Err(Error::InvalidPolicy)
    );
    let mut bad = data;
    bad.version = 2;
    assert_eq!(
        prepare(
            &bad,
            policy::Policy::default(),
            100.,
            &AtomicBool::new(false)
        ),
        Err(Error::InvalidData)
    );
}
