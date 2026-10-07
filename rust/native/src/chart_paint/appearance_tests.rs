use super::*;
use gpuio_protocol::{
    chart_appearance as a, chart_data as d,
    chart_options::{Curve, Orientation, Stacking},
    chart_sampling,
};

fn series(id: i64, values: &[f64]) -> d::Series {
    d::Series {
        id,
        name: format!("Series {id}"),
        points: values
            .iter()
            .enumerate()
            .map(|(i, y)| d::Point {
                id: i as i64 + 1,
                x: i as f64,
                y: Some(*y),
                label: String::new(),
            })
            .collect(),
    }
}
fn options() -> Options {
    let mut options = Options::default();
    options.axes.x = false;
    options.axes.y = false;
    options.axes.grid = false;
    options.cartesian.dots = true;
    options
}
fn plan(data: &Data, options: &Options, style: &Style) -> Result<Prepared, Error> {
    prepare(
        data,
        Policy::default(),
        options,
        style,
        Layout::new(600., 300., 1.).unwrap(),
        &AtomicBool::new(false),
    )
}
fn series_style(id: i64) -> a::Series {
    a::Series {
        series: id,
        path: None,
        marker: None,
        bar: None,
        legend: None,
        area_baseline: None,
    }
}

#[test]
fn marker_datum_radius_matches_exact_hit_bounds_and_hidden_markers_keep_selection() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![d::Layer::Line(series(11, &[1., 2., 3.]))]),
    };
    let options = options();
    let mut style = Style::default();
    style.appearance.series.push(a::Series {
        marker: Some(a::Marker {
            fill: Some(7),
            stroke: Some(9),
            stroke_width: Some(2.),
            ..Default::default()
        }),
        ..series_style(11)
    });
    style.appearance.data.push(a::Datum {
        series: 11,
        datum: 2,
        marker: Some(a::Marker {
            radius: Some(24.),
            ..Default::default()
        }),
        bar: None,
    });
    let p = plan(&data, &options, &style).unwrap();
    assert_eq!(p.quads[1].rect.right - p.quads[1].rect.left, 48.);
    assert_eq!(p.quads[1].brush, Brush::Solid(7));
    assert_eq!(p.quads[1].border, Some((2., 9)));
    let geometry::Shape::Dot { center, .. } = p.geometry.marks[1].shape else {
        panic!()
    };
    assert_eq!(
        p.hit_index.query(
            &p.geometry,
            geometry::Point {
                x: center.x,
                y: center.y + 23.
            },
            false
        ),
        Some(1)
    );
    style.appearance.data[0].marker.as_mut().unwrap().visible = Some(false);
    let hidden = plan(&data, &options, &style).unwrap();
    assert_eq!(hidden.quads.len(), 2);
    assert_eq!(hidden.geometry.marks, p.geometry.marks);
    assert_eq!(
        hidden.hit_index.query(&hidden.geometry, center, false),
        Some(1)
    );
    assert_eq!(
        hidden.hit_index.query(
            &hidden.geometry,
            geometry::Point {
                x: center.x,
                y: center.y + 23.
            },
            false
        ),
        None
    );
}

#[test]
fn series_paths_have_independent_brushes_widths_curves_and_legend_colors() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![d::Layer::Area(series(11, &[1., 3., 2.]))]),
    };
    let options = options();
    let mut style = Style::default();
    let default = plan(&data, &options, &style).unwrap();
    let fill = gradient(90., 0xff000040, 0x0000ff80);
    style.appearance.series.push(a::Series {
        path: Some(a::Path {
            stroke: Some(a::Stroke {
                visible: true,
                width: Some(7.),
                brush: solid(77),
            }),
            fill: Some(fill),
            curve: Some(Curve::StepAfter),
        }),
        legend: Some(99),
        area_baseline: None,
        ..series_style(11)
    });
    let p = plan(&data, &options, &style).unwrap();
    assert_eq!(
        p.meshes[0].brush, fill,
        "explicit alpha is not multiplied by area opacity"
    );
    assert_eq!(p.meshes[1].brush, solid(77));
    assert_eq!(p.series_color(0), 99);
    assert_ne!(p.geometry.paths, default.geometry.paths);
    assert_eq!(p.geometry.marks, default.geometry.marks);
    style.appearance.series[0]
        .path
        .as_mut()
        .unwrap()
        .stroke
        .as_mut()
        .unwrap()
        .visible = false;
    assert_eq!(plan(&data, &options, &style).unwrap().mesh_count(), 1);
}

#[test]
fn stack_curves_require_matching_effective_area_curves() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![
            d::Layer::Area(series(11, &[1., 3., 2.])),
            d::Layer::Area(series(22, &[2., 1., 2.])),
        ]),
    };
    let mut options = options();
    options.cartesian.stacking = Stacking::Stacked;
    let mut style = Style::default();
    style.appearance.series.push(a::Series {
        path: Some(a::Path {
            curve: Some(Curve::Natural),
            ..Default::default()
        }),
        ..series_style(11)
    });
    assert!(matches!(
        plan(&data, &options, &style),
        Err(Error::InvalidConfiguration)
    ));
    style.appearance.series.push(a::Series {
        path: Some(a::Path {
            curve: Some(Curve::Natural),
            ..Default::default()
        }),
        ..series_style(22)
    });
    assert!(plan(&data, &options, &style).is_ok());
    options.cartesian.stacking = Stacking::Grouped;
    style.appearance.series.pop();
    assert!(plan(&data, &options, &style).is_ok());
}

#[test]
fn bar_brushes_follow_signed_stack_bounds_and_physical_corners() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![
            d::Layer::Bar(series(11, &[-2., 3.])),
            d::Layer::Bar(series(22, &[-4., 5.])),
        ]),
    };
    let mut options = options();
    options.cartesian.stacking = Stacking::Stacked;
    let mut style = Style::default();
    for id in [11, 22] {
        style.appearance.series.push(a::Series {
            bar: Some(a::Bar {
                fill: Some(a::BarFill::BaseToTip(7, 9)),
                corners: Some(Corners {
                    top_left: 1.,
                    top_right: 2.,
                    bottom_right: 3.,
                    bottom_left: 4.,
                }),
            }),
            ..series_style(id)
        });
    }
    for (orientation, positive) in [
        (Orientation::Vertical, 0.),
        (Orientation::Horizontal, 90.),
        (Orientation::VerticalReversed, 180.),
        (Orientation::HorizontalReversed, 270.),
    ] {
        options.cartesian.orientation = orientation;
        let p = plan(&data, &options, &style).unwrap();
        for (i, quad) in p.quads.iter().enumerate() {
            assert_eq!(
                quad.brush,
                gradient(
                    if i % 2 == 0 {
                        (positive + 180.) % 360.
                    } else {
                        positive
                    },
                    7,
                    9
                )
            );
            assert_eq!(
                quad.corners,
                Corners {
                    top_left: 1.,
                    top_right: 2.,
                    bottom_right: 3.,
                    bottom_left: 4.
                }
            );
        }
    }
}

#[test]
fn sampled_bar_uses_agreement_instead_of_first_source_color() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![d::Layer::Bar(series(11, &[1., 2., 3., 4.]))]),
    };
    let options = options();
    let mut style = Style::default();
    style.appearance.data = (1..=4)
        .map(|datum| a::Datum {
            series: 11,
            datum,
            marker: None,
            bar: Some(a::Bar {
                fill: Some(a::BarFill::Background(solid(9))),
                corners: None,
            }),
        })
        .collect();
    let make = |style: &Style| {
        prepare(
            &data,
            Policy {
                bars: chart_sampling::Bar::Sum(1),
                ..Default::default()
            },
            &options,
            style,
            Layout::new(600., 300., 1.).unwrap(),
            &AtomicBool::new(false),
        )
        .unwrap()
    };
    assert_eq!(make(&style).quads[0].brush, solid(style.color(0)));
    style.appearance.aggregates = a::Aggregates::Uniform;
    assert_eq!(make(&style).quads[0].brush, solid(9));
    style.appearance.data[3].bar = None;
    let p = make(&style);
    assert_eq!(p.quads[0].brush, solid(style.color(0)));
    assert_eq!(p.geometry.summary(0), Some(geometry::Summary::Bar(10.)));
}

#[test]
fn radar_appearance_uses_axis_identity_and_preserves_original_selection() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Radar(
            [90, 7, 42]
                .into_iter()
                .map(|id| d::RadarAxis {
                    id,
                    label: format!("Axis {id}"),
                    maximum: 100.,
                })
                .collect(),
            vec![d::RadarSeries {
                id: 11,
                name: "Radar".into(),
                values: vec![(90, 50.), (7, 60.), (42, 70.)],
            }],
        ),
    };
    let options = options();
    let mut style = Style::default();
    style.appearance.series.push(a::Series {
        marker: Some(a::Marker {
            radius: Some(5.),
            ..Default::default()
        }),
        path: Some(a::Path {
            fill: Some(solid(17)),
            ..Default::default()
        }),
        ..series_style(11)
    });
    style.appearance.data.push(a::Datum {
        series: 11,
        datum: 7,
        marker: Some(a::Marker {
            radius: Some(19.),
            fill: Some(31),
            ..Default::default()
        }),
        bar: None,
    });
    // The same axis ID under an absent series must never style this radar.
    style.appearance.data.push(a::Datum {
        series: 99,
        datum: 90,
        marker: Some(a::Marker {
            visible: Some(false),
            ..Default::default()
        }),
        bar: None,
    });
    let p = plan(&data, &options, &style).unwrap();
    assert_eq!(p.quads.len(), 3);
    assert_eq!(p.quads[0].rect.right - p.quads[0].rect.left, 10.);
    assert_eq!(p.quads[1].rect.right - p.quads[1].rect.left, 38.);
    assert_eq!(p.quads[1].brush, solid(31));
    assert!(p.meshes.iter().any(|m| m.brush == solid(17)));
    assert_eq!(
        p.selection_index(gpuio_protocol::chart_selection::Selection::Radar {
            series: 11,
            axis: 7,
        }),
        Some(1)
    );
}

#[test]
fn area_baseline_changes_domain_and_fill_without_rewriting_source() {
    let mut options = options();
    let mut style = Style::default();
    style.appearance.series.push(a::Series {
        area_baseline: Some(5.),
        ..series_style(11)
    });
    for categorical in [false, true] {
        let source = series(11, &[10., 20., 15.]);
        let data = Data {
            version: 2,
            bar_backgrounds: vec![],
            contents: if categorical {
                d::Contents::Categorical(
                    (0..3)
                        .map(|i| d::Category {
                            id: i + 1,
                            label: format!("C{i}"),
                        })
                        .collect(),
                    vec![d::CategoricalLayer::Area(d::CategoricalSeries {
                        id: 11,
                        name: "Area".into(),
                        points: source
                            .points
                            .iter()
                            .map(|p| d::CategoricalPoint {
                                id: p.id,
                                category: p.id,
                                value: p.y,
                                label: p.label.clone(),
                            })
                            .collect(),
                    })],
                )
            } else {
                d::Contents::Cartesian(vec![d::Layer::Area(source)])
            },
        };
        let original = data.clone();
        for baseline in [-5., 5., 30.] {
            style.appearance.series[0].area_baseline = Some(baseline);
            for orientation in [
                Orientation::Vertical,
                Orientation::Horizontal,
                Orientation::VerticalReversed,
                Orientation::HorizontalReversed,
            ] {
                options.cartesian.orientation = orientation;
                let p = plan(&data, &options, &style).unwrap();
                let domain = p.geometry.y_domain.unwrap();
                assert_eq!(domain.min, baseline.min(10.));
                assert_eq!(domain.max, baseline.max(20.));
                let fill = p.geometry.paths.iter().find(|p| p.fill).unwrap();
                let end = &fill.commands[fill.commands.len() - 3..];
                let fraction = domain.unit(baseline);
                let fraction = if orientation.is_reversed() {
                    1. - fraction
                } else {
                    fraction
                };
                for command in &end[..2] {
                    let geometry::Command::Line(point) = command else {
                        panic!("closing boundary")
                    };
                    let (actual, expected) = if orientation.is_horizontal() {
                        (point.x, fraction * p.geometry.width)
                    } else {
                        (point.y, (1. - fraction) * p.geometry.height)
                    };
                    assert!((actual - expected).abs() < 1e-8);
                }
                assert_eq!(end[2], geometry::Command::Close);
                assert_eq!(
                    p.geometry.marks[0].source,
                    geometry::Source::Cartesian {
                        series: 0,
                        start: 0,
                        end: 1
                    }
                );
                assert_eq!(data, original);
            }
        }
    }
}

#[test]
fn stacked_area_baseline_offsets_bounds_and_requires_shared_effective_value() {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![
            d::Layer::Area(series(11, &[2., -3., 4.])),
            d::Layer::Area(series(22, &[5., -7., 6.])),
        ]),
    };
    let mut options = options();
    options.cartesian.stacking = Stacking::Stacked;
    let mut style = Style::default();
    style.appearance.series = [11, 22]
        .into_iter()
        .map(|id| a::Series {
            area_baseline: Some(20.),
            ..series_style(id)
        })
        .collect();
    for curve in [Curve::Linear, Curve::Natural, Curve::StepAfter] {
        options.cartesian.curve = curve;
        let p = plan(&data, &options, &style).unwrap();
        assert_eq!(
            p.geometry.y_domain,
            Some(geometry::Domain { min: 10., max: 30. })
        );
        assert_eq!(
            p.geometry.summary(0),
            Some(geometry::Summary::Stacked {
                value: 2.,
                lower: 20.,
                upper: 22.
            })
        );
        assert_eq!(
            p.geometry.summary(3),
            Some(geometry::Summary::Stacked {
                value: 5.,
                lower: 22.,
                upper: 27.
            })
        );
        assert_eq!(
            p.geometry.summary(4),
            Some(geometry::Summary::Stacked {
                value: -7.,
                lower: 17.,
                upper: 10.
            })
        );
    }
    style.appearance.series[1].area_baseline = None;
    assert!(matches!(
        plan(&data, &options, &style),
        Err(Error::InvalidConfiguration)
    ));
    style.appearance.series[0].area_baseline = Some(0.);
    assert!(plan(&data, &options, &style).is_ok());
    options.cartesian.stacking = Stacking::Grouped;
    style.appearance.series[0].area_baseline = Some(20.);
    assert!(plan(&data, &options, &style).is_ok());
}

#[test]
fn area_baseline_preserves_gaps_empty_domains_and_non_area_layers() {
    let options = options();
    let mut style = Style::default();
    style.appearance.series.push(a::Series {
        area_baseline: Some(20.),
        ..series_style(11)
    });
    let mut source = series(11, &[1., 2., 3.]);
    source.points[1].y = None;
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![d::Layer::Area(source.clone())]),
    };
    let p = plan(&data, &options, &style).unwrap();
    assert!(p.geometry.paths.is_empty());
    assert_eq!(p.geometry.marks.len(), 2);
    assert_eq!(
        p.geometry.marks[1].source,
        geometry::Source::Cartesian {
            series: 0,
            start: 2,
            end: 3
        }
    );
    for point in &mut source.points {
        point.y = None;
    }
    let empty = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: d::Contents::Cartesian(vec![d::Layer::Area(source)]),
    };
    let p = plan(&empty, &options, &style).unwrap();
    assert_eq!(
        p.geometry.y_domain,
        Some(geometry::Domain { min: 0., max: 1. })
    );
    assert!(p.geometry.paths.is_empty());
    assert!(p.geometry.marks.is_empty());
    for layer in [
        d::Layer::Line(series(11, &[10., 12.])),
        d::Layer::Bar(series(11, &[10., 12.])),
    ] {
        let data = Data {
            version: 2,
            bar_backgrounds: vec![],
            contents: d::Contents::Cartesian(vec![layer]),
        };
        let default = plan(&data, &options, &Style::default()).unwrap();
        let override_ = plan(&data, &options, &style).unwrap();
        assert_eq!(default.geometry.y_domain, override_.geometry.y_domain);
        assert_eq!(default.geometry.paths, override_.geometry.paths);
        assert_eq!(default.geometry.marks, override_.geometry.marks);
    }
}
