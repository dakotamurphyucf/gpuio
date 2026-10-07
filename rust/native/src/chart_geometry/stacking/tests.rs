use super::*;
use crate::{chart_details, chart_reduce, chart_selection, chart_table};
use gpuio_protocol::{chart_data::*, chart_options::*, chart_sampling::*, chart_selection::*};

fn series(id: i64, values: &[Option<f64>]) -> data::Series {
    data::Series {
        id,
        name: format!("Series {id}"),
        points: values
            .iter()
            .enumerate()
            .map(|(i, y)| data::Point {
                id: 100 - i as i64,
                x: i as f64,
                y: *y,
                label: String::new(),
            })
            .collect(),
    }
}
fn numeric(layers: Vec<Layer>) -> Data {
    Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Cartesian(layers),
    }
}
fn categorical(data: &Data) -> Data {
    let Contents::Cartesian(layers) = &data.contents else {
        panic!()
    };
    let count = layers[0].series().points.len();
    Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Categorical(
            (0..count)
                .map(|i| Category {
                    id: 1000 - i as i64,
                    label: format!("Category {i}"),
                })
                .collect(),
            layers
                .iter()
                .map(|l| {
                    let s = l.series();
                    let s = CategoricalSeries {
                        id: s.id,
                        name: s.name.clone(),
                        points: s
                            .points
                            .iter()
                            .enumerate()
                            .map(|(i, p)| CategoricalPoint {
                                id: p.id,
                                category: 1000 - i as i64,
                                value: p.y,
                                label: p.label.clone(),
                            })
                            .collect(),
                    };
                    match l {
                        Layer::Line(_) => CategoricalLayer::Line(s),
                        Layer::Area(_) => CategoricalLayer::Area(s),
                        Layer::Bar(_) => CategoricalLayer::Bar(s),
                    }
                })
                .collect(),
        ),
    }
}
fn options() -> Options {
    let mut options = Options::default();
    options.cartesian.stacking = Stacking::Stacked;
    options
}
fn plan(data: &Data, options: &Options, policy: Policy) -> Plan {
    prepare(data, policy, options, 600., 300., &AtomicBool::new(false)).unwrap()
}
fn bounds(plan: &Plan, series: usize, source: usize) -> (f64, f64, f64) {
    let (index, _) = plan
        .marks
        .iter()
        .enumerate()
        .find(|(_, m)| {
            m.layer == series
                && matches!(m.source, Source::Cartesian { start, .. } if start == source)
        })
        .unwrap();
    let Some(Summary::Stacked {
        baseline: _,
        value,
        lower,
        upper,
    }) = plan.summary(index)
    else {
        panic!()
    };
    (value, lower, upper)
}

#[test]
fn signed_missing_stacks_preserve_original_values_and_independent_mixed_families() {
    let original = numeric(vec![
        Layer::Bar(series(7, &[Some(2.), Some(0.), Some(-3.)])),
        Layer::Area(series(9, &[Some(10.), Some(20.), Some(30.)])),
        Layer::Line(series(3, &[Some(1.), Some(2.), Some(3.)])),
        Layer::Bar(series(2, &[Some(5.), Some(-4.), Some(2.)])),
        Layer::Area(series(4, &[Some(1.), None, Some(-2.)])),
    ]);
    for data in [&original, &categorical(&original)] {
        let options = options();
        let plan = plan(data, &options, Policy::default());
        assert_eq!(bounds(&plan, 0, 2), (-3., 0., -3.));
        assert_eq!(bounds(&plan, 3, 0), (5., 2., 7.));
        assert_eq!(bounds(&plan, 3, 1), (-4., 0., -4.));
        assert_eq!(bounds(&plan, 3, 2), (2., -3., -1.));
        assert_eq!(bounds(&plan, 4, 0), (1., 10., 11.));
        assert_eq!(bounds(&plan, 4, 2), (-2., 30., 28.));
        assert_eq!(plan.y_domain, Some(Domain { min: -4., max: 30. }));
        assert_eq!((plan.source_values, plan.rendered_values), (15, 14));
        assert_eq!(chart_table::count(data), 15);
        for (i, mark) in plan.marks.iter().enumerate() {
            let Source::Cartesian { series, start, .. } = mark.source else {
                panic!()
            };
            let Selection::Cartesian {
                series: id,
                span,
                aggregation,
            } = chart_selection::resolve(data, &Policy::default(), mark.source).unwrap()
            else {
                panic!()
            };
            assert_eq!(id, [7, 9, 3, 2, 4][series]);
            assert_eq!(span.first, 100 - start as i64);
            assert_eq!(aggregation, Aggregation::Exact);
            let details =
                chart_details::describe(data, &Policy::default(), &options, &plan, i).unwrap();
            assert_eq!(details.text.contains("Stack baseline:"), series != 2);
        }
    }
}

#[test]
fn aggregation_precedes_stacking_and_missing_buckets_do_not_shift_identity() {
    let data = categorical(&numeric(vec![
        Layer::Bar(series(1, &[None, None, Some(2.), Some(4.)])),
        Layer::Bar(series(2, &[Some(10.), Some(20.), None, Some(6.)])),
    ]));
    for (bars, expected) in [(Bar::Sum(2), (6., 6., 12.)), (Bar::Mean(2), (6., 3., 9.))] {
        let policy = Policy {
            bars,
            ..Policy::default()
        };
        let plan = plan(&data, &options(), policy);
        assert_eq!(plan.marks.len(), 3);
        assert_eq!(bounds(&plan, 1, 2), expected);
        let Selection::Cartesian { span, .. } =
            chart_selection::resolve(&data, &policy, plan.marks[2].source).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (span.start_index, span.length, span.first, span.last),
            (2, 2, 98, 97)
        );
        let text = chart_details::describe(&data, &policy, &options(), &plan, 2)
            .unwrap()
            .text;
        assert!(text.contains("across 2 categories (missing values excluded)"));
        assert!(text.contains("Stack endpoint:"));
    }
}

#[test]
fn stacked_numeric_alignment_is_required_only_within_each_kind() {
    let mut second = series(2, &[Some(3.), Some(4.)]);
    second.points[1].x = 2.;
    for wrap in [Layer::Bar, Layer::Area] {
        let data = numeric(vec![
            wrap(series(1, &[Some(1.), Some(2.)])),
            wrap(second.clone()),
        ]);
        assert_eq!(
            chart_reduce::prepare_with_options(
                &data,
                Policy::default(),
                600.,
                &options(),
                &AtomicBool::new(false)
            ),
            Err(chart_reduce::Error::MisalignedStack)
        );
        assert!(
            prepare(
                &data,
                Policy::default(),
                &Options::default(),
                600.,
                300.,
                &AtomicBool::new(false)
            )
            .is_ok()
        );
    }
    let data = numeric(vec![
        Layer::Bar(series(1, &[Some(1.)])),
        Layer::Area(second.clone()),
        Layer::Line(series(3, &[])),
    ]);
    assert!(
        prepare(
            &data,
            Policy::default(),
            &options(),
            600.,
            300.,
            &AtomicBool::new(false)
        )
        .is_ok()
    );
    let short = numeric(vec![Layer::Area(second), Layer::Area(series(3, &[]))]);
    assert_eq!(
        chart_reduce::prepare_with_options(
            &short,
            Policy::default(),
            600.,
            &options(),
            &AtomicBool::new(false)
        ),
        Err(chart_reduce::Error::MisalignedStack)
    );
}

#[test]
fn shared_envelope_preserves_both_boundaries_extrema_and_all_gap_edges() {
    let data = numeric(vec![
        Layer::Area(series(
            1,
            &[
                Some(1.),
                Some(90.),
                Some(1.),
                None,
                None,
                Some(1.),
                Some(1.),
                Some(1.),
            ],
        )),
        Layer::Area(series(
            2,
            &[
                Some(1.),
                Some(-90.),
                Some(1.),
                Some(1.),
                Some(1.),
                Some(1.),
                Some(80.),
                Some(1.),
            ],
        )),
    ]);
    let policy = Policy {
        line: Line::Envelope(1),
        ..Policy::default()
    };
    let reduction = chart_reduce::prepare_with_options(
        &data,
        policy,
        600.,
        &options(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let chart_reduce::Contents::Cartesian(reduced) = reduction.contents else {
        panic!()
    };
    let (reduce::Series::StackedArea(a), reduce::Series::StackedArea(b)) =
        (&reduced[0], &reduced[1])
    else {
        panic!()
    };
    assert_eq!(
        a.iter().map(|p| p.source).collect::<Vec<_>>(),
        b.iter().map(|p| p.source).collect::<Vec<_>>()
    );
    for index in [0, 1, 2, 3, 4, 5, 6, 7] {
        assert!(a.iter().any(|p| p.source == index));
    }
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.bounds.upper, b.bounds.lower);
    }
    let plan = plan(&data, &options(), policy);
    assert_eq!(plan.y_domain, Some(Domain { min: 0., max: 90. }));
    assert_eq!(
        plan.paths.iter().filter(|p| p.layer == 0 && p.fill).count(),
        2
    );
    assert_eq!(
        plan.paths.iter().filter(|p| p.layer == 1 && p.fill).count(),
        1
    );
    assert!(
        !plan
            .marks
            .iter()
            .any(|m| m.layer == 0 && matches!(m.source, Source::Cartesian { start: 3 | 4, .. }))
    );
}

#[test]
fn adjacent_area_boundaries_are_identical_under_every_curve_and_direction() {
    let data = numeric(vec![
        Layer::Area(series(1, &[Some(2.), Some(5.), Some(-1.), Some(3.)])),
        Layer::Area(series(2, &[Some(1.), Some(2.), Some(4.), Some(1.)])),
    ]);
    for data in [&data, &categorical(&data)] {
        for direction in [
            Orientation::Vertical,
            Orientation::Horizontal,
            Orientation::VerticalReversed,
            Orientation::HorizontalReversed,
        ] {
            for curve in [Curve::Linear, Curve::Natural, Curve::StepAfter] {
                let mut options = options();
                options.cartesian.orientation = direction;
                options.cartesian.curve = curve;
                let plan = plan(data, &options, Policy::default());
                let front = &plan
                    .paths
                    .iter()
                    .find(|p| p.layer == 0 && !p.fill)
                    .unwrap()
                    .commands;
                let fill = &plan
                    .paths
                    .iter()
                    .find(|p| p.layer == 1 && p.fill)
                    .unwrap()
                    .commands;
                let expected = reverse(front);
                let start = front.len() + 1;
                assert_eq!(&fill[start..fill.len() - 1], &expected[1..]);
                let Command::Move(p) = expected[0] else {
                    panic!()
                };
                assert_eq!(fill[start - 1], Command::Line(p));
                assert_eq!(fill.last(), Some(&Command::Close));
            }
        }
    }
}

#[test]
fn clipping_missing_runs_keeps_shared_natural_tangents() {
    let data = numeric(vec![
        Layer::Area(series(1, &[Some(2.), Some(5.), None, Some(4.), Some(3.)])),
        Layer::Area(series(
            2,
            &[Some(1.), Some(2.), Some(4.), Some(1.), Some(2.)],
        )),
    ]);
    let mut options = options();
    options.cartesian.curve = Curve::Natural;
    let plan = plan(&data, &options, Policy::default());
    let first = &plan
        .paths
        .iter()
        .find(|p| p.layer == 0 && !p.fill)
        .unwrap()
        .commands;
    let upper_fill = &plan
        .paths
        .iter()
        .find(|p| p.layer == 1 && p.fill)
        .unwrap()
        .commands;
    let back = reverse(first);
    // The final reversed segment of the complete lower boundary must be exactly
    // the reversed first segment of the lower series, despite that series' gap.
    assert_eq!(upper_fill[upper_fill.len() - 2], back[1]);
}

#[test]
fn stacking_is_bounded_cancellable_and_empty_safe() {
    let mut a = series(1, &[]);
    let mut b = series(2, &[]);
    for i in 0..50_000 {
        for (s, phase) in [(&mut a, 0.), (&mut b, 1.)] {
            s.points.push(data::Point {
                id: i + 1,
                x: i as f64,
                y: Some((i % 97) as f64 + phase),
                label: String::new(),
            });
        }
    }
    let data = numeric(vec![Layer::Area(a), Layer::Area(b)]);
    let policy = Policy {
        line: Line::Envelope(16),
        ..Policy::default()
    };
    let reduced = chart_reduce::prepare_with_options(
        &data,
        policy,
        100.,
        &options(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(reduced.source_values, 100_000);
    assert!(reduced.rendered_values <= 16 * 6 * 2 * 2);
    assert!(reduced.retained_bytes() < 100_000);
    assert_eq!(
        chart_reduce::prepare_with_options(&data, policy, 100., &options(), &AtomicBool::new(true)),
        Err(chart_reduce::Error::Cancelled)
    );
    let empty = numeric(vec![
        Layer::Area(series(1, &[])),
        Layer::Bar(series(2, &[])),
    ]);
    assert_eq!(plan(&empty, &options(), policy).marks.len(), 0);
}
