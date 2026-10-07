use super::*;
use gpuio_protocol::{
    chart_appearance::{Datum, Series},
    chart_data as data,
};

#[test]
fn bar_ramps_follow_value_direction_and_preserve_clipped_plateaus() {
    let domain = geometry::Domain {
        min: -10.,
        max: 30.,
    };
    for (orientation, angle) in [
        (Orientation::Vertical, 0.),
        (Orientation::Horizontal, 90.),
        (Orientation::VerticalReversed, 180.),
        (Orientation::HorizontalReversed, 270.),
    ] {
        for (values, expected) in [
            ((0., 5.), angle),
            ((0., -5.), (angle + 180.) % 360.),
            ((7., 12.), angle),
            ((-7., -12.), (angle + 180.) % 360.),
        ] {
            assert_eq!(
                bar_brush(
                    BarFill::BaseToTip(0xff0000ff, 0x0000ffff),
                    values,
                    domain,
                    orientation
                ),
                gradient(expected, 0xff0000ff, 0x0000ffff)
            );
        }
        assert_eq!(
            bar_brush(
                BarFill::Values(0., 0x000000ff, 20., 0xffffffff),
                (-10., 30.),
                domain,
                orientation
            ),
            Brush::Linear {
                oklab: false,
                angle,
                from: 0x000000ff,
                start: 0.25,
                to: 0xffffffff,
                stop: 0.75
            }
        );
        assert_eq!(
            bar_brush(
                BarFill::Values(0., 0x000000ff, 20., 0xffffffff),
                (5., 15.),
                domain,
                orientation
            ),
            Brush::Linear {
                oklab: false,
                angle,
                from: 0x404040ff,
                start: 0.,
                to: 0xbfbfbfff,
                stop: 1.
            }
        );
        assert_eq!(
            bar_brush(
                BarFill::Values(0., 7, 20., 9),
                (-10., -1.),
                domain,
                orientation
            ),
            Brush::Solid(7)
        );
        assert_eq!(
            bar_brush(
                BarFill::Values(0., 7, 20., 9),
                (21., 30.),
                domain,
                orientation
            ),
            Brush::Solid(9)
        );
        assert_eq!(
            bar_brush(
                BarFill::Domain(7, 9),
                (2., 2.),
                geometry::Domain { min: 2., max: 2. },
                orientation
            ),
            Brush::Solid(7)
        );
        assert!(
            bar_brush(
                BarFill::Values(-1e100, 0, 1e100, 0xffff_ffff),
                (-1e105, 1e105),
                domain,
                orientation
            )
            .is_valid()
        );
        let fixed = gradient(123., 3, 7);
        assert_eq!(
            bar_brush(BarFill::Background(fixed), (0., -1.), domain, orientation),
            fixed
        );
    }
}

fn source() -> Data {
    Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Cartesian(vec![data::Layer::Bar(data::Series {
            id: 55,
            name: "Source".into(),
            points: vec![
                data::Point {
                    id: 90,
                    x: 0.,
                    y: Some(1.),
                    label: String::new(),
                },
                data::Point {
                    id: 3,
                    x: 1.,
                    y: None,
                    label: String::new(),
                },
                data::Point {
                    id: 7,
                    x: 2.,
                    y: Some(2.),
                    label: String::new(),
                },
            ],
        })]),
    }
}

#[test]
fn dense_fills_compose_with_sparse_corners_and_aggregate_provenance() {
    let mut data = source();
    let fill = Brush::Checkerboard(0xff00ffff, 8.);
    data.bar_backgrounds = vec![
        data::BarBackground {
            series: 55,
            datum: 3,
            brush: Brush::Solid(3),
        },
        data::BarBackground {
            series: 55,
            datum: 7,
            brush: fill,
        },
        data::BarBackground {
            series: 55,
            datum: 90,
            brush: fill,
        },
    ];
    let mut style = Style::default();
    let options = Options::default();
    let cancel = AtomicBool::new(false);
    let resolved = |style: &Style, start, end| {
        Index::new(&data, style, &options, &cancel)
            .unwrap()
            .bar(&mark(start, end), 1)
            .unwrap()
    };
    assert_eq!(
        resolved(&style, 0, 1).fill,
        Some(BarFill::Background(fill)),
        "no sparse map needed"
    );
    assert_eq!(
        resolved(&style, 0, 2).fill,
        Some(BarFill::Background(fill)),
        "gap is not a participant"
    );
    assert_eq!(
        resolved(&style, 0, 3).fill,
        Some(BarFill::Background(Brush::Solid(1))),
        "multi-value default inherits series"
    );
    style.appearance.aggregates = Aggregates::Uniform;
    assert_eq!(resolved(&style, 0, 3).fill, Some(BarFill::Background(fill)));
    let corners = Corners {
        top_left: 4.,
        top_right: 3.,
        bottom_right: 2.,
        bottom_left: 1.,
    };
    style.appearance.data.push(Datum {
        series: 55,
        datum: 90,
        marker: None,
        bar: Some(Bar {
            fill: None,
            corners: Some(corners),
        }),
    });
    let single = resolved(&style, 0, 2);
    assert_eq!(single.fill, Some(BarFill::Background(fill)));
    assert_eq!(single.corners, Some(corners));
    assert_eq!(
        resolved(&style, 0, 3).fill,
        Some(BarFill::Background(Brush::Solid(1))),
        "corners also must agree"
    );
    style.appearance.data[0].bar.as_mut().unwrap().fill =
        Some(BarFill::Background(Brush::Solid(9)));
    assert_eq!(
        resolved(&style, 0, 1).fill,
        Some(BarFill::Background(Brush::Solid(9))),
        "sparse fill wins"
    );
    let index = Index::new(&data, &style, &options, &cancel).unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert_eq!(index.bar(&mark(0, 3), 1), Err(Error::Cancelled));
}
fn mark(start: usize, end: usize) -> geometry::Mark {
    geometry::Mark {
        layer: 0,
        source: geometry::Source::Cartesian {
            series: 0,
            start,
            end,
        },
        shape: geometry::Shape::Bar(geometry::Rect {
            left: 0.,
            top: 0.,
            right: 10.,
            bottom: 10.,
        }),
    }
}
#[test]
fn aggregates_compare_inherited_resolved_styles_and_ignore_missing_observations() {
    let data = source();
    let options = Options::default();
    let cancel = AtomicBool::new(false);
    let fill = Some(BarFill::Background(Brush::Solid(9)));
    let mut style = Style::default();
    style.appearance.series.push(Series {
        series: 55,
        path: None,
        marker: None,
        bar: Some(Bar {
            fill,
            corners: None,
        }),
        legend: None,
        area_baseline: None,
    });
    style.appearance.data.push(Datum {
        series: 55,
        datum: 90,
        marker: None,
        bar: Some(Bar {
            fill: Some(BarFill::Background(Brush::Solid(7))),
            corners: None,
        }),
    });
    let index = Index::new(&data, &style, &options, &cancel).unwrap();
    assert_eq!(
        index.bar(&mark(0, 3), 1).unwrap().fill,
        fill,
        "default aggregate inherits series"
    );
    assert_eq!(
        index.bar(&mark(0, 2), 1).unwrap().fill,
        Some(BarFill::Background(Brush::Solid(7))),
        "only one defined observation"
    );
    style.appearance.aggregates = Aggregates::Uniform;
    assert_eq!(
        Index::new(&data, &style, &options, &cancel)
            .unwrap()
            .bar(&mark(0, 3), 1)
            .unwrap()
            .fill,
        fill,
        "unoverridden observation participates"
    );
    style.appearance.data.push(Datum {
        series: 55,
        datum: 7,
        marker: None,
        bar: style.appearance.data[0].bar,
    });
    style.appearance.data.push(Datum {
        series: 55,
        datum: 3,
        marker: None,
        bar: Some(Bar {
            fill: Some(BarFill::Background(Brush::Solid(99))),
            corners: None,
        }),
    });
    assert_eq!(
        Index::new(&data, &style, &options, &cancel)
            .unwrap()
            .bar(&mark(0, 3), 1)
            .unwrap()
            .fill,
        Some(BarFill::Background(Brush::Solid(7))),
        "missing sample does not break agreement"
    );
    // Override equality is evaluated after inheritance: explicit series fill and
    // no datum fill are the same effective appearance.
    style.appearance.data[0].bar = Some(Bar {
        fill,
        corners: None,
    });
    style.appearance.data[1].bar = Some(Bar::default());
    assert_eq!(
        Index::new(&data, &style, &options, &cancel)
            .unwrap()
            .bar(&mark(0, 3), 1)
            .unwrap()
            .fill,
        fill
    );
    assert!(matches!(
        Index::new(&data, &style, &options, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
}
