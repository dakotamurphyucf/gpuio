use super::*;
fn options() -> options::Options {
    options::Options::default()
}
fn dataset(contents: data::Contents) -> data::Data {
    data::Data {
        version: 1,
        contents,
    }
}
fn plan(contents: data::Contents) -> Plan {
    prepare(
        &dataset(contents),
        Policy::default(),
        &options(),
        800.,
        400.,
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn series(id: i64, ys: &[Option<f64>]) -> data::Series {
    data::Series {
        id,
        name: format!("series {id}"),
        points: ys
            .iter()
            .enumerate()
            .map(|(i, y)| data::Point {
                id: (i + 1) as i64,
                x: i as f64,
                y: *y,
                label: String::new(),
            })
            .collect(),
    }
}
fn finite_point(p: Point) {
    assert!(p.x.is_finite() && p.y.is_finite(), "{p:?}");
}
fn finite(plan: &Plan) {
    for path in &plan.paths {
        for cmd in &path.commands {
            match cmd {
                Command::Move(p) | Command::Line(p) => finite_point(*p),
                Command::Cubic(a, b, c) => {
                    finite_point(*a);
                    finite_point(*b);
                    finite_point(*c);
                }
                Command::Close => {}
            }
        }
    }
    for mark in &plan.marks {
        match mark.shape {
            Shape::Dot { center, .. } => finite_point(center),
            Shape::Bar(r) | Shape::Node(r) => {
                finite_point(Point::new(r.left, r.top));
                finite_point(Point::new(r.right, r.bottom));
                assert!(r.left <= r.right && r.top <= r.bottom);
            }
            Shape::Candle {
                center,
                left,
                right,
                open,
                high,
                low,
                close,
            } => {
                assert!(
                    [center, left, right, open, high, low, close]
                        .iter()
                        .all(|v| v.is_finite())
                );
                assert!(
                    left <= center
                        && center <= right
                        && high <= open
                        && open <= low
                        && high <= close
                        && close <= low
                );
            }
            Shape::Wedge {
                center,
                inner,
                outer,
                start,
                end,
            } => {
                finite_point(center);
                assert!([inner, outer, start, end].iter().all(|v| v.is_finite()));
                assert!(inner <= outer && start < end);
            }
            Shape::Ribbon {
                start_top,
                start_bottom,
                end_top,
                end_bottom,
            } => {
                finite_point(start_top);
                finite_point(start_bottom);
                finite_point(end_top);
                finite_point(end_bottom);
            }
        }
    }
    for (a, b) in &plan.grid {
        finite_point(*a);
        finite_point(*b);
    }
    for label in &plan.labels {
        finite_point(label.position);
        assert!(label.text.len() < 1024);
    }
}
#[test]
fn gaps_singletons_and_all_curve_endpoints_are_preserved() {
    for style in [
        options::Curve::Linear,
        options::Curve::Natural,
        options::Curve::StepAfter,
    ] {
        for horizontal in [false, true] {
            let mut options = options();
            options.cartesian.curve = style;
            if horizontal {
                options.cartesian.orientation = options::Orientation::Horizontal;
            }
            let data = dataset(data::Contents::Cartesian(vec![data::Layer::Area(series(
                1,
                &[Some(3.), None, Some(-2.), Some(1.), None, Some(0.)],
            ))]));
            let p = prepare(
                &data,
                Policy::default(),
                &options,
                800.,
                400.,
                &AtomicBool::new(false),
            )
            .unwrap();
            finite(&p);
            assert_eq!(p.paths.len(), 2);
            assert_eq!(p.marks.len(), 4);
            for index in [0, 3] {
                assert!(matches!(
                    p.marks[index].shape,
                    Shape::Dot { visible: true, .. }
                ));
            }
            assert!(matches!(
                p.marks[1].shape,
                Shape::Dot { visible: false, .. }
            ));
            assert!(p.paths[0].fill);
            assert!(!p.paths[1].fill);
            assert!(matches!(p.paths[0].commands.last(), Some(Command::Close)));
            let Shape::Dot { center: last, .. } = p.marks[2].shape else {
                panic!()
            };
            let endpoint = match p.paths[1].commands.last().unwrap() {
                Command::Line(p) | Command::Cubic(_, _, p) => *p,
                _ => panic!(),
            };
            assert_eq!(endpoint, last, "every curve reaches its final data point");
            assert_eq!(
                p.marks[2].source,
                Source::Cartesian {
                    series: 0,
                    start: 3,
                    end: 4
                }
            );
        }
    }
}
#[test]
fn grouped_bars_use_zero_baseline_and_numeric_spacing() {
    let mut first = series(1, &[Some(-4.), Some(2.), Some(3.)]);
    first.points[2].x = 100.;
    let mut second = first.clone();
    second.id = 2;
    let p = plan(data::Contents::Cartesian(vec![
        data::Layer::Bar(first),
        data::Layer::Bar(second),
    ]));
    finite(&p);
    assert_eq!(p.y_domain, Some(Domain { min: -4., max: 3. }));
    let rect = |i: usize| match p.marks[i].shape {
        Shape::Bar(r) => r,
        _ => panic!(),
    };
    assert!(rect(0).right <= rect(3).left + 1e-9);
    assert!((rect(2).left - rect(1).left) > 90. * (rect(1).left - rect(0).left));
    assert!(rect(0).bottom > rect(0).top);
    assert!(
        (rect(0).top - rect(1).bottom).abs() < 1e-9,
        "both signed bars meet at zero"
    );
    assert!(rect(0).left >= 0. && rect(5).right <= 800.);
}
#[test]
fn aggregate_ranges_remain_attached_to_bar_and_candle_marks() {
    let data = dataset(data::Contents::Cartesian(vec![data::Layer::Bar(series(
        1,
        &[Some(3.), Some(4.), Some(-2.)],
    ))]));
    let policy = Policy {
        bars: gpuio_protocol::chart_sampling::Bar::Sum(1),
        ..Policy::default()
    };
    let p = prepare(
        &data,
        policy,
        &options(),
        800.,
        400.,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(p.marks.len(), 1);
    assert_eq!(
        p.marks[0].source,
        Source::Cartesian {
            series: 0,
            start: 0,
            end: 3
        }
    );
    assert_eq!(p.y_domain.unwrap().max, 5.);
    let data = dataset(data::Contents::Candlestick(vec![
        data::Candle {
            id: 6,
            x: 0.,
            label: "a".into(),
            open_: -3.,
            high: 0.,
            low: -4.,
            close: -1.,
        },
        data::Candle {
            id: 2,
            x: 1.,
            label: "b".into(),
            open_: -1.,
            high: 2.,
            low: -5.,
            close: -4.,
        },
    ]));
    let policy = Policy {
        candles: gpuio_protocol::chart_sampling::Candlestick::Ohlc(1),
        ..Policy::default()
    };
    let p = prepare(
        &data,
        policy,
        &options(),
        800.,
        400.,
        &AtomicBool::new(false),
    )
    .unwrap();
    finite(&p);
    assert_eq!(p.marks.len(), 1);
    let Source::Candle(span) = p.marks[0].source else {
        panic!()
    };
    assert_eq!((span.start(), span.end()), (0, 2));
    assert_eq!(p.y_domain, Some(Domain { min: -5., max: 2. }));
}
#[test]
fn empty_constant_subnormal_and_extreme_datasets_have_finite_geometry() {
    for contents in [
        data::Contents::Cartesian(vec![]),
        data::Contents::Pie(vec![]),
        data::Contents::Radar(vec![], vec![]),
        data::Contents::Candlestick(vec![]),
        data::Contents::Sankey(vec![], vec![]),
    ] {
        let p = plan(contents);
        finite(&p);
        assert!(p.marks.is_empty() && p.paths.is_empty());
    }
    for value in [0., -7., f64::from_bits(1), 1e100, -1e100] {
        let p = plan(data::Contents::Cartesian(vec![data::Layer::Line(series(
            1,
            &[Some(value)],
        ))]));
        finite(&p);
        assert_eq!(
            p.marks[0].shape,
            Shape::Dot {
                center: Point::new(400., 200.),
                visible: true
            }
        );
        let p = plan(data::Contents::Pie(vec![data::Slice {
            id: 1,
            label: "slice".into(),
            value: value.abs(),
        }]));
        finite(&p);
        assert_eq!(p.marks.len(), usize::from(value != 0.));
    }
    let p = plan(data::Contents::Cartesian(vec![data::Layer::Line(series(
        1,
        &[Some(-1e100), Some(1e100)],
    ))]));
    finite(&p);
    assert_eq!(
        p.y_domain,
        Some(Domain {
            min: -1e100,
            max: 1e100
        })
    );
}
#[test]
fn pie_donut_and_radar_preserve_proportions_and_axis_identity() {
    let mut opts = options();
    opts.pie.inner_radius = 0.6;
    opts.pie.pad_angle = 0.1;
    let data = dataset(data::Contents::Pie(vec![
        data::Slice {
            id: 8,
            label: "one".into(),
            value: 1e100,
        },
        data::Slice {
            id: 2,
            label: "three".into(),
            value: 3e100,
        },
        data::Slice {
            id: 1,
            label: "zero".into(),
            value: 0.,
        },
    ]));
    // Each individual value must respect the source maximum.
    let mut data = data;
    let data::Contents::Pie(slices) = &mut data.contents else {
        panic!()
    };
    slices[0].value = 1e99;
    slices[1].value = 3e99;
    let p = prepare(
        &data,
        Policy::default(),
        &opts,
        800.,
        400.,
        &AtomicBool::new(false),
    )
    .unwrap();
    finite(&p);
    assert_eq!(p.marks.len(), 2);
    let Shape::Wedge {
        inner,
        outer,
        start,
        end,
        ..
    } = p.marks[0].shape
    else {
        panic!()
    };
    assert_eq!((inner, outer), (120., 200.));
    assert!((end - start - (PI / 2. - 0.1)).abs() < 1e-9);
    let axes = vec![
        data::RadarAxis {
            id: 9,
            label: "a".into(),
            maximum: 10.,
        },
        data::RadarAxis {
            id: 3,
            label: "b".into(),
            maximum: 20.,
        },
        data::RadarAxis {
            id: 7,
            label: "c".into(),
            maximum: 1e100,
        },
    ];
    let p = plan(data::Contents::Radar(
        axes,
        vec![data::RadarSeries {
            id: 1,
            name: "series".into(),
            values: vec![(7, 0.), (9, 10.), (3, 10.)],
        }],
    ));
    finite(&p);
    assert_eq!(p.marks[0].source, Source::Radar { series: 0, axis: 0 });
    let Shape::Dot { center, .. } = p.marks[0].shape else {
        panic!()
    };
    assert!((center.y - 0.).abs() < 1e-9);
    let Shape::Dot { center, .. } = p.marks[2].shape else {
        panic!()
    };
    assert_eq!(center, Point::new(400., 200.));
}
#[test]
fn sankey_handles_large_tiny_zero_imbalanced_and_isolated_flows() {
    for magnitude in [0., f64::from_bits(1), 1e-100, 1., 1e100] {
        for alignment in [
            options::Alignment::Left,
            options::Alignment::Right,
            options::Alignment::Center,
            options::Alignment::Justify,
        ] {
            for scale in [options::FlowScale::Linear, options::FlowScale::Sqrt] {
                let nodes = (1..=4)
                    .map(|id| data::Node {
                        id,
                        label: format!("n{id}"),
                    })
                    .collect();
                let edges = vec![
                    data::Edge {
                        id: 1,
                        source: 1,
                        target: 2,
                        value: magnitude,
                    },
                    data::Edge {
                        id: 2,
                        source: 2,
                        target: 3,
                        value: magnitude / 2.,
                    },
                ];
                let mut opts = options();
                opts.sankey.alignment = alignment;
                opts.sankey.scale = scale;
                let p = prepare(
                    &dataset(data::Contents::Sankey(nodes, edges)),
                    Policy::default(),
                    &opts,
                    800.,
                    400.,
                    &AtomicBool::new(false),
                )
                .unwrap();
                finite(&p);
                assert_eq!(
                    p.marks
                        .iter()
                        .filter(|m| matches!(m.source, Source::Node(_)))
                        .count(),
                    4
                );
                for mark in &p.marks {
                    if let Shape::Node(r) = mark.shape {
                        assert!(
                            r.left >= -0.001
                                && r.right <= 800.001
                                && r.top >= -0.001
                                && r.bottom <= 400.001,
                            "{r:?}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn large_sources_are_reduced_once_and_pathological_gaps_stay_separate() {
    for gaps in [false, true] {
        let points = (0..100_000)
            .map(|i| data::Point {
                id: i + 1,
                x: i as f64,
                y: if gaps && i % 2 == 1 {
                    None
                } else {
                    Some((i % 17) as f64)
                },
                label: String::new(),
            })
            .collect();
        let p = plan(data::Contents::Cartesian(vec![data::Layer::Area(
            data::Series {
                id: 1,
                name: "stream".into(),
                points,
            },
        )]));
        finite(&p);
        assert_eq!(p.source_values, 100_000);
        assert!(p.retained_bytes() < 16 * 1024 * 1024);
        if gaps {
            assert_eq!(p.marks.len(), 50_000);
            assert!(p.paths.is_empty());
            assert!(
                p.marks
                    .iter()
                    .all(|m| matches!(m.shape, Shape::Dot { visible: true, .. }))
            );
        } else {
            assert!(p.marks.len() <= 3200);
            assert_eq!(p.paths.len(), 2);
        }
    }
}
#[test]
fn native_formatters_and_invalid_inputs_are_bounded() {
    use options::NumberFormat::*;
    assert_eq!(format_number(-0., Fixed(2)), "0.00");
    assert_eq!(format_number(0.125, Percent(1)), "12.5%");
    assert_eq!(format_number(-1250., Compact), "-1.25K");
    assert_eq!(format_number(100., Compact), "100");
    assert_eq!(format_number(1e100, Compact), "1.00e100");
    assert_eq!(format_number(f64::NAN, Fixed(0)), "—");
    assert_eq!(format_number(1., Fixed(i64::MAX)), "—");
    let data = dataset(data::Contents::Pie(vec![]));
    for (w, h) in [(0., 1.), (1., f64::INFINITY), (f64::NAN, 1.), (32769., 1.)] {
        assert!(matches!(
            prepare(
                &data,
                Policy::default(),
                &options(),
                w,
                h,
                &AtomicBool::new(false)
            ),
            Err(Error::InvalidInput)
        ));
    }
    assert!(matches!(
        prepare(
            &data,
            Policy::default(),
            &options(),
            800.,
            400.,
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    ));
}

#[test]
fn exact_large_area_and_candles_fit_the_explicit_plan_budget() {
    let cancel = AtomicBool::new(false);
    let policy = Policy {
        line: gpuio_protocol::chart_sampling::Line::Exact,
        ..Policy::default()
    };
    let points = (0..100_000)
        .map(|i| data::Point {
            id: i + 1,
            x: i as f64,
            y: Some((i % 31) as f64 - 15.),
            label: String::new(),
        })
        .collect();
    let data = dataset(data::Contents::Cartesian(vec![data::Layer::Area(
        data::Series {
            id: 1,
            name: "exact".into(),
            points,
        },
    )]));
    let mut options = options();
    options.cartesian.curve = options::Curve::Natural;
    let p = prepare(&data, policy, &options, 800., 400., &cancel).unwrap();
    finite(&p);
    assert_eq!(p.marks.len(), 100_000);
    assert_eq!(p.paths.len(), 2);
    assert!(p.retained_bytes() < MAX_PLAN_BYTES);
    drop(p);
    drop(data);
    let values = (0..100_000)
        .map(|i| data::Candle {
            id: i + 1,
            x: i as f64,
            label: String::new(),
            open_: -1.,
            high: 2.,
            low: -2.,
            close: 1.,
        })
        .collect();
    let p = plan(data::Contents::Candlestick(values));
    finite(&p);
    assert_eq!(p.marks.len(), 100_000);
    assert!(p.retained_bytes() < MAX_PLAN_BYTES);
}

#[test]
fn empty_series_domains_and_crowded_sankey_columns_remain_usable() {
    let p = plan(data::Contents::Cartesian(vec![
        data::Layer::Area(series(1, &[])),
        data::Layer::Bar(series(2, &[])),
    ]));
    assert_eq!(p.x_domain, Some(Domain { min: 0., max: 1. }));
    assert_eq!(p.y_domain, Some(Domain { min: 0., max: 1. }));
    let nodes = (1..=256)
        .map(|id| data::Node {
            id,
            label: format!("n{id}"),
        })
        .collect::<Vec<_>>();
    let edges = (2..=256)
        .map(|id| data::Edge {
            id,
            source: 1,
            target: id,
            value: 1.,
        })
        .collect::<Vec<_>>();
    let data = dataset(data::Contents::Sankey(nodes, edges));
    for (w, h) in [(800., 400.), (1., 1.), (0.125, 0.125), (32768., 32768.)] {
        let p = prepare(
            &data,
            Policy::default(),
            &options(),
            w,
            h,
            &AtomicBool::new(false),
        )
        .unwrap();
        finite(&p);
        let Shape::Node(first) = p
            .marks
            .iter()
            .find(|m| m.source == Source::Node(0))
            .unwrap()
            .shape
        else {
            panic!()
        };
        if w == 800. {
            assert_eq!(
                first.right - first.left,
                16.,
                "node width fits two columns regardless of total node count"
            );
        }
        for mark in &p.marks {
            if let Shape::Node(r) = mark.shape {
                assert!(
                    r.bottom > r.top,
                    "positive flows retain area in a crowded column"
                );
            }
        }
    }
}
