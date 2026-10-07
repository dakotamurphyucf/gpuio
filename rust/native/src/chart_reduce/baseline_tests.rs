use super::*;
use gpuio_protocol::{
    chart_data::*,
    chart_options::{Options, Orientation, Stacking},
};
fn source(categorical: bool, values: &[Option<f64>], bases: &[f64]) -> Data {
    let points = values
        .iter()
        .enumerate()
        .map(|(i, &value)| CategoricalPoint {
            id: 100 - i as i64,
            category: i as i64 + 1,
            value,
            label: String::new(),
        })
        .collect::<Vec<_>>();
    let mut baselines = points
        .iter()
        .zip(bases)
        .map(|(p, &baseline)| BarBaseline {
            series: 7,
            datum: p.id,
            baseline,
        })
        .collect::<Vec<_>>();
    baselines.sort_by_key(|b| (b.series, b.datum));
    Data {
        version: 3,
        bar_backgrounds: vec![],
        bar_baselines: baselines,
        contents: if categorical {
            data::Contents::Categorical(
                (0..values.len())
                    .map(|i| Category {
                        id: i as i64 + 1,
                        label: format!("C{i}"),
                    })
                    .collect(),
                vec![CategoricalLayer::Bar(CategoricalSeries {
                    id: 7,
                    name: "Intervals".into(),
                    points,
                })],
            )
        } else {
            data::Contents::Cartesian(vec![Layer::Bar(data::Series {
                id: 7,
                name: "Intervals".into(),
                points: points
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| Point {
                        id: p.id,
                        x: i as f64,
                        y: p.value,
                        label: p.label,
                    })
                    .collect(),
            })])
        },
    }
}
fn reduced(data: &Data, bars: policy::Bar, options: &Options) -> Result<Reduction, Error> {
    prepare_with_options(
        data,
        policy::Policy {
            bars,
            ..Default::default()
        },
        600.,
        options,
        &AtomicBool::new(false),
    )
}
fn first(result: &Reduction) -> Bar {
    let super::Contents::Cartesian(layers) = &result.contents else {
        panic!()
    };
    layers[0].bars().next().unwrap().0
}
#[test]
fn intervals_aggregate_contributions_not_origins_and_reject_incompatible_bases() {
    for categorical in [false, true] {
        let data = source(categorical, &[Some(25.), Some(27.)], &[20., 20.]);
        assert_eq!(
            first(&reduced(&data, policy::Bar::Sum(1), &Options::default()).unwrap()).value,
            32.
        );
        assert_eq!(
            first(&reduced(&data, policy::Bar::Mean(1), &Options::default()).unwrap()).value,
            26.
        );
        let different = source(categorical, &[Some(25.), Some(27.)], &[20., 21.]);
        for bars in [policy::Bar::Sum(1), policy::Bar::Mean(1)] {
            assert_eq!(
                reduced(&different, bars, &Options::default()),
                Err(Error::IncompatibleBarBaselines)
            );
        }
        assert!(reduced(&different, policy::Bar::Exact, &Options::default()).is_ok());
    }
    let data = source(true, &[None, Some(27.)], &[999., 20.]);
    assert_eq!(
        first(&reduced(&data, policy::Bar::Sum(1), &Options::default()).unwrap()).value,
        27.
    );
}
#[test]
fn exact_and_singleton_aggregates_preserve_small_endpoint_against_huge_base() {
    let data = source(false, &[Some(1e-100)], &[1e100]);
    for bars in [
        policy::Bar::Exact,
        policy::Bar::Sum(1),
        policy::Bar::Mean(1),
    ] {
        let b = first(&reduced(&data, bars, &Options::default()).unwrap());
        assert_eq!(b.value, 1e-100);
        assert_eq!(b.baseline, 1e100);
    }
}
#[test]
fn interval_domains_geometry_original_browser_and_details_in_four_directions() {
    use crate::{chart_details, chart_geometry as g, chart_table};
    for categorical in [false, true] {
        for orientation in [
            Orientation::Vertical,
            Orientation::VerticalReversed,
            Orientation::Horizontal,
            Orientation::HorizontalReversed,
        ] {
            let data = source(categorical, &[Some(25.), Some(10.)], &[20., 20.]);
            let mut options = Options::default();
            options.cartesian.orientation = orientation;
            let policy = policy::Policy {
                bars: policy::Bar::Exact,
                ..Default::default()
            };
            let plan =
                g::prepare(&data, policy, &options, 600., 300., &AtomicBool::new(false)).unwrap();
            assert_eq!(plan.y_domain, Some(g::Domain { min: 10., max: 25. }));
            assert_eq!(plan.marks.len(), 2);
            assert!(matches!(plan.marks[0].shape, g::Shape::Bar(_)));
            assert!(
                chart_details::describe(&data, &policy, &options, &plan, 0)
                    .unwrap()
                    .text
                    .contains("Source baseline: 20\nEndpoint: 25")
            );
            assert!(
                chart_table::row(&data, 0)
                    .unwrap()
                    .value
                    .contains("baseline: 20")
            );
        }
    }
}
#[test]
fn stacked_intervals_share_origin_once_and_keep_endpoints() {
    let mut data = source(false, &[Some(25.)], &[20.]);
    let data::Contents::Cartesian(layers) = &mut data.contents else {
        panic!()
    };
    let mut second = layers[0].series().clone();
    second.id = 8;
    second.points[0].y = Some(27.);
    layers.push(Layer::Bar(second));
    data.bar_baselines.push(BarBaseline {
        series: 8,
        datum: 100,
        baseline: 20.,
    });
    let mut options = Options::default();
    options.cartesian.stacking = Stacking::Stacked;
    let result = reduced(&data, policy::Bar::Exact, &options).unwrap();
    let super::Contents::Cartesian(layers) = result.contents else {
        panic!()
    };
    assert_eq!(
        layers[0].bars().next().unwrap().1,
        Some(StackBounds {
            lower: 20.,
            upper: 25.
        })
    );
    assert_eq!(
        layers[1].bars().next().unwrap().1,
        Some(StackBounds {
            lower: 25.,
            upper: 32.
        })
    );
    assert_eq!(layers[1].bars().next().unwrap().0.value, 27.);
    data.bar_baselines[1].baseline = 21.;
    assert_eq!(
        reduced(&data, policy::Bar::Exact, &options),
        Err(Error::IncompatibleBarBaselines)
    );
}

#[test]
fn signed_stacks_keep_distinct_position_baselines_and_missing_values_do_not_constrain() {
    for categorical in [false, true] {
        let mut data = source(categorical, &[Some(25.), Some(-25.)], &[20., -20.]);
        match &mut data.contents {
            data::Contents::Cartesian(layers) => {
                let mut next = layers[0].series().clone();
                next.id = 8;
                next.points[0].y = Some(27.);
                next.points[1].y = Some(-10.);
                layers.push(Layer::Bar(next));
            }
            data::Contents::Categorical(_, layers) => {
                let mut next = layers[0].series().clone();
                next.id = 8;
                next.points[0].value = Some(27.);
                next.points[1].value = Some(-10.);
                layers.push(CategoricalLayer::Bar(next));
            }
            _ => panic!(),
        }
        data.bar_baselines.extend([
            BarBaseline {
                series: 8,
                datum: 99,
                baseline: -20.,
            },
            BarBaseline {
                series: 8,
                datum: 100,
                baseline: 20.,
            },
        ]);
        let mut options = Options::default();
        options.cartesian.stacking = Stacking::Stacked;
        let result = reduced(&data, policy::Bar::Exact, &options).unwrap();
        let super::Contents::Cartesian(layers) = result.contents else {
            panic!()
        };
        assert_eq!(
            layers[1]
                .bars()
                .map(|(_, b)| b.unwrap())
                .collect::<Vec<_>>(),
            vec![
                StackBounds {
                    lower: 25.,
                    upper: 32.
                },
                StackBounds {
                    lower: -25.,
                    upper: -15.
                }
            ]
        );
        if categorical {
            let data::Contents::Categorical(_, layers) = &mut data.contents else {
                panic!()
            };
            let CategoricalLayer::Bar(first) = &mut layers[0] else {
                panic!()
            };
            first.points[0].value = None;
            data.bar_baselines
                .iter_mut()
                .find(|b| b.series == 7 && b.datum == 100)
                .unwrap()
                .baseline = 999.;
            let result = reduced(&data, policy::Bar::Exact, &options).unwrap();
            let super::Contents::Cartesian(layers) = result.contents else {
                panic!()
            };
            assert_eq!(
                layers[1].bars().next().unwrap().1,
                Some(StackBounds {
                    lower: 20.,
                    upper: 27.
                })
            );
        }
    }
}
