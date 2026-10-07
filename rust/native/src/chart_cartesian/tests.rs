use super::*;
use crate::{chart_details, chart_geometry as geometry, chart_hit, chart_selection, chart_table};
use gpuio_protocol::{chart_data::*, chart_options::*, chart_sampling::*, chart_selection::*};
use std::sync::atomic::AtomicBool;
fn source() -> Data {
    Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: Contents::Categorical(
            [42, 7, 99]
                .into_iter()
                .map(|id| Category {
                    id,
                    label: format!("Category {id}"),
                })
                .collect(),
            vec![CategoricalLayer::Bar(CategoricalSeries {
                id: 9,
                name: "Load".into(),
                points: [42, 7, 99]
                    .into_iter()
                    .zip([Some(2.), None, Some(6.)])
                    .enumerate()
                    .map(|(i, (category, value))| CategoricalPoint {
                        id: [8, 3, 22][i],
                        category,
                        value,
                        label: String::new(),
                    })
                    .collect(),
            })],
        ),
    }
}
fn plan(data: &Data, options: &Options, policy: Policy) -> geometry::Plan {
    geometry::prepare(data, policy, options, 600., 300., &AtomicBool::new(false)).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
#[test]
fn categorical_bands_preserve_labels_gaps_selection_and_raw_data_in_four_directions() {
    let data = source();
    for orientation in [
        Orientation::Vertical,
        Orientation::Horizontal,
        Orientation::VerticalReversed,
        Orientation::HorizontalReversed,
    ] {
        let mut options = Options::default();
        options.cartesian.orientation = orientation;
        let plan = plan(&data, &options, Policy::default());
        assert_eq!((plan.source_values, plan.rendered_values), (3, 2));
        assert_eq!(
            plan.x_domain, None,
            "ranks must not become a public numeric domain"
        );
        let labels = plan
            .labels
            .iter()
            .filter(|l| l.kind == geometry::LabelKind::X)
            .collect::<Vec<_>>();
        assert_eq!(
            labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            vec!["Category 42", "Category 7", "Category 99"]
        );
        let extent = if orientation.is_horizontal() {
            300.
        } else {
            600.
        };
        for (i, label) in labels.iter().enumerate() {
            close(
                if orientation.is_horizontal() {
                    label.position.y
                } else {
                    label.position.x
                },
                extent * (i as f64 + 0.5) / 3.,
            );
        }
        let index =
            chart_hit::Index::prepare(&plan, orientation, 3., &AtomicBool::new(false)).unwrap();
        for (i, mark) in plan.marks.iter().enumerate() {
            let geometry::Shape::Bar(r) = mark.shape else {
                panic!()
            };
            close(
                if orientation.is_horizontal() {
                    r.bottom - r.top
                } else {
                    r.right - r.left
                },
                extent / 3. * 0.8 * 0.8,
            );
            let center = geometry::Point {
                x: (r.left + r.right) / 2.,
                y: (r.top + r.bottom) / 2.,
            };
            assert_eq!(index.query(&plan, center, false), Some(i));
            let selection =
                chart_selection::resolve(&data, &Policy::default(), mark.source).unwrap();
            let Selection::Cartesian {
                series,
                span,
                aggregation,
            } = selection
            else {
                panic!()
            };
            assert_eq!(series, 9);
            assert_eq!(span.first, [8, 22][i]);
            assert_eq!(aggregation, Aggregation::Exact);
            let details =
                chart_details::describe(&data, &Policy::default(), &options, &plan, i).unwrap();
            assert!(details.text.contains(&format!("Category {}", [42, 99][i])));
        }
    }
    assert_eq!(chart_table::count(&data), 3);
    let missing = chart_table::row(&data, 1).unwrap();
    assert_eq!(missing.value, "Category: Category 7 (7) · value: Missing");
    assert!(missing.detail.contains("datum 3"));
}
#[test]
fn categorical_aggregation_excludes_missing_arithmetic_but_preserves_domain_interval() {
    let data = source();
    let options = Options::default();
    for (bars, want) in [(Bar::Sum(1), 8.), (Bar::Mean(1), 4.)] {
        let policy = Policy {
            bars,
            ..Policy::default()
        };
        let plan = plan(&data, &options, policy);
        assert_eq!(plan.marks.len(), 1);
        assert_eq!(plan.summary(0), Some(geometry::Summary::Bar(want)));
        let geometry::Shape::Bar(r) = plan.marks[0].shape else {
            panic!()
        };
        close(r.right - r.left, (400. + 160.) * 0.8);
        let Selection::Cartesian { span, .. } =
            chart_selection::resolve(&data, &policy, plan.marks[0].source).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (span.start_index, span.length, span.first, span.last),
            (0, 3, 8, 22)
        );
        let text = chart_details::describe(&data, &policy, &options, &plan, 0)
            .unwrap()
            .text;
        assert!(text.contains("across 3 categories (missing values excluded)"));
        assert!(text.contains("Category 42 (category 42) – Category 99 (category 99)"));
    }
}
#[test]
fn point_band_empty_singleton_and_explicit_padding_are_finite() {
    for n in [0, 1, 2, 100_000] {
        for extent in [1., 600., 32768.] {
            for layout in [
                CategoryLayout::Point(0.),
                CategoryLayout::Point(1.),
                CategoryLayout::Band {
                    inner: 0.,
                    outer: 0.,
                },
                CategoryLayout::Band {
                    inner: 0.9,
                    outer: 1.,
                },
            ] {
                let p = Projection::new(n, extent, layout, false);
                assert!(p.center(0.).is_finite());
                assert!(p.band_width.is_finite());
                if n == 1 {
                    close(p.center(0.), extent / 2.);
                }
                if n > 0 {
                    assert!(p.center(0.) >= 0. && p.center((n - 1) as f64) <= extent + 1e-8);
                }
            }
        }
    }
    let p = Projection::new(3, 600., CategoryLayout::Point(0.), false);
    close(p.center(0.), 0.);
    close(p.center(1.), 300.);
    close(p.center(2.), 600.);
    let p = Projection::new(
        3,
        600.,
        CategoryLayout::Band {
            inner: 0.,
            outer: 0.,
        },
        false,
    );
    close(p.center(0.), 100.);
    close(p.band_width, 200.);
}

#[test]
fn category_reduction_buckets_follow_padded_positions_and_keep_large_source_indices() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: Contents::Categorical(
            (1..=4)
                .map(|id| Category {
                    id,
                    label: id.to_string(),
                })
                .collect(),
            vec![CategoricalLayer::Bar(CategoricalSeries {
                id: 1,
                name: "Buckets".into(),
                points: (1..=4)
                    .map(|id| CategoricalPoint {
                        id,
                        category: id,
                        value: Some(id as f64),
                        label: String::new(),
                    })
                    .collect(),
            })],
        ),
    };
    let mut options = Options::default();
    options.cartesian.category_layout = CategoryLayout::Point(1.);
    let reduced = crate::chart_reduce::prepare_with_options(
        &data,
        Policy {
            bars: Bar::Sum(3),
            ..Policy::default()
        },
        600.,
        &options,
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::chart_reduce::Contents::Cartesian(layers) = reduced.contents else {
        panic!()
    };
    let crate::chart_reduce::Series::Bar(bars) = &layers[0] else {
        panic!()
    };
    assert_eq!(
        bars.iter().map(|b| b.value).collect::<Vec<_>>(),
        vec![1., 5., 4.]
    );
    assert_eq!((bars[1].source.start(), bars[1].source.end()), (1, 3));
    let count = 100_000;
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: Contents::Categorical(
            (0..count)
                .map(|i| Category {
                    id: (count - i) as i64,
                    label: "Same".into(),
                })
                .collect(),
            vec![CategoricalLayer::Line(CategoricalSeries {
                id: 1,
                name: "Large".into(),
                points: (0..count)
                    .map(|i| CategoricalPoint {
                        id: i as i64 + 1,
                        category: (count - i) as i64,
                        value: if i == 50_000 {
                            None
                        } else {
                            Some((i % 17) as f64)
                        },
                        label: String::new(),
                    })
                    .collect(),
            })],
        ),
    };
    let mut sources = None;
    for orientation in [Orientation::Horizontal, Orientation::HorizontalReversed] {
        options.cartesian.orientation = orientation;
        let plan = geometry::prepare(
            &data,
            Policy::default(),
            &options,
            20.,
            200.,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(plan.source_values, count);
        assert!(plan.marks.len() < 1000);
        assert!(
            plan.marks.len() > 80,
            "horizontal preparation uses category height"
        );
        assert!(
            plan.labels
                .iter()
                .filter(|l| l.kind == geometry::LabelKind::X)
                .count()
                <= 5
        );
        let indices = plan.marks.iter().map(|m| m.source).collect::<Vec<_>>();
        if let Some(previous) = &sources {
            assert_eq!(previous, &indices);
        } else {
            sources = Some(indices);
        }
        for mark in &plan.marks {
            let Selection::Cartesian { span, .. } =
                chart_selection::resolve(&data, &Policy::default(), mark.source).unwrap()
            else {
                panic!()
            };
            assert_eq!(span.first, span.start_index + 1);
            assert_ne!(span.start_index, 50_000);
        }
    }
}

#[test]
fn signed_categorical_mixed_curves_share_centers_and_keep_area_gaps() {
    let categories = (0..7)
        .map(|i| Category {
            id: 100 - i,
            label: format!("Group {i}"),
        })
        .collect::<Vec<_>>();
    let series = |id| CategoricalSeries {
        id,
        name: format!("Series {id}"),
        points: [
            Some(2.),
            Some(-3.),
            Some(6.),
            None,
            Some(4.),
            Some(1.),
            Some(-1.),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, value)| CategoricalPoint {
            id: i as i64 + 1,
            category: 100 - i as i64,
            value,
            label: String::new(),
        })
        .collect(),
    };
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: Contents::Categorical(
            categories,
            vec![
                CategoricalLayer::Area(series(1)),
                CategoricalLayer::Bar(series(2)),
                CategoricalLayer::Line(series(3)),
            ],
        ),
    };
    for curve in [Curve::Linear, Curve::Natural, Curve::StepAfter] {
        for orientation in [
            Orientation::Vertical,
            Orientation::Horizontal,
            Orientation::VerticalReversed,
            Orientation::HorizontalReversed,
        ] {
            let mut options = Options::default();
            options.cartesian.curve = curve;
            options.cartesian.orientation = orientation;
            options.cartesian.category_layout = CategoryLayout::Band {
                inner: 0.,
                outer: 0.,
            };
            let plan = geometry::prepare(
                &data,
                Policy::default(),
                &options,
                700.,
                700.,
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(plan.marks.len(), 18);
            assert_eq!(plan.paths.len(), 6);
            for mark in &plan.marks {
                let geometry::Source::Cartesian { start, .. } = mark.source else {
                    panic!()
                };
                assert_ne!(start, 3);
                let center = match mark.shape {
                    geometry::Shape::Dot { center, .. } => center,
                    geometry::Shape::Bar(r) => geometry::Point {
                        x: (r.left + r.right) / 2.,
                        y: (r.top + r.bottom) / 2.,
                    },
                    _ => panic!(),
                };
                close(
                    if orientation.is_horizontal() {
                        center.y
                    } else {
                        center.x
                    },
                    50. + 100. * start as f64,
                );
            }
            let fills = plan
                .paths
                .iter()
                .filter(|p| p.layer == 0 && p.fill)
                .collect::<Vec<_>>();
            assert_eq!(fills.len(), 2);
            for (path, (low, high)) in fills.into_iter().zip([(50., 250.), (450., 650.)]) {
                for command in &path.commands {
                    let points = match *command {
                        geometry::Command::Move(p) | geometry::Command::Line(p) => vec![p],
                        geometry::Command::Cubic(a, b, c) => vec![a, b, c],
                        geometry::Command::Close => vec![],
                    };
                    for point in points {
                        let coordinate = if orientation.is_horizontal() {
                            point.y
                        } else {
                            point.x
                        };
                        assert!(
                            (low - 1e-8..=high + 1e-8).contains(&coordinate),
                            "area crossed a missing category"
                        );
                    }
                }
            }
        }
    }
}
