//! Real GPU readback for prepared charts. Hidden window; no claim about the
//! OCaml mounted view, resource invalidation, keyboard, IME or accessibility.
use super::*;
use gpui::{App, AsyncApp, Context, Render, WindowHandle, WindowOptions, div, prelude::*};
use gpuio_protocol::chart_data as data;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
struct View {
    prepared: Arc<Prepared>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let prepared = self.prepared.clone();
        let actual = self.bounds.clone();
        div().size_full().bg(gpui::rgb(0)).p(px(20.)).child(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    actual.set(bounds);
                    prepared
                        .paint(bounds, &mut FrameBudget::default(), window)
                        .unwrap();
                },
            )
            .w(px(200.))
            .h(px(160.)),
        )
    }
}
fn series() -> data::Series {
    data::Series {
        id: 1,
        name: "Values".into(),
        points: vec![(0., 0.), (1., 1.), (2., 0.)]
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| data::Point {
                id: (i + 1) as i64,
                x,
                y: Some(y),
                label: String::new(),
            })
            .collect(),
    }
}
fn dataset(contents: data::Contents) -> Data {
    Data {
        version: 2,
        bar_backgrounds: vec![],
        contents,
    }
}
fn options() -> Options {
    let mut o = Options::default();
    o.axes.x = false;
    o.axes.y = false;
    o.axes.grid = false;
    o
}
fn style() -> Style {
    Style {
        palette: vec![0xff0000ff, 0x0000ffff, 0x00ff00ff],
        stroke_width: 4.,
        area_opacity: 0.25,
        bar_radius: 8.,
        ..Style::default()
    }
}
struct Case {
    name: &'static str,
    data: Data,
    options: Options,
    style: Style,
    samples: Vec<(f32, f32, [u8; 4])>,
}
fn cases() -> Vec<Case> {
    let sample = |x, y, r, g, b| (x, y, [r, g, b, 255]);
    let mut out = vec![];
    let mut add = |name, data, options, style, samples| {
        out.push(Case {
            name,
            data,
            options,
            style,
            samples,
        })
    };
    use gpuio_protocol::chart_appearance as a;
    use gpuio_protocol::chart_options::Orientation;
    let series_override = |bar, path, marker| a::Series {
        series: 1,
        path,
        marker,
        bar,
        legend: None,
        area_baseline: None,
    };
    let mut single = series();
    single.points.truncate(1);
    single.points[0].y = Some(2.);
    for orientation in [
        Orientation::Vertical,
        Orientation::Horizontal,
        Orientation::VerticalReversed,
        Orientation::HorizontalReversed,
    ] {
        for value in [2., -2.] {
            let mut source = single.clone();
            source.points[0].y = Some(value);
            let mut config = options();
            config.cartesian.orientation = orientation;
            let mut appearance = style();
            appearance.appearance.series.push(series_override(
                Some(a::Bar {
                    fill: Some(a::BarFill::BaseToTip(0xff0000ff, 0x0000ffff)),
                    corners: Some(Corners {
                        top_left: 0.,
                        top_right: 0.,
                        bottom_left: 0.,
                        bottom_right: 0.,
                    }),
                }),
                None,
                None,
            ));
            add(
                "appearance-base-tip",
                dataset(data::Contents::Cartesian(vec![data::Layer::Bar(source)])),
                config,
                appearance,
                vec![],
            );
        }
        let mut source = single.clone();
        source.points[0].y = Some(4.);
        let mut config = options();
        config.cartesian.orientation = orientation;
        let mut appearance = style();
        appearance.appearance.series.push(series_override(
            Some(a::Bar {
                fill: Some(a::BarFill::Values(1., 0xff0000ff, 3., 0x0000ffff)),
                corners: None,
            }),
            None,
            None,
        ));
        add(
            "appearance-value-plateaus",
            dataset(data::Contents::Cartesian(vec![data::Layer::Bar(source)])),
            config,
            appearance,
            vec![],
        );
    }
    let mut corners = style();
    corners.appearance.series.push(series_override(
        Some(a::Bar {
            fill: Some(a::BarFill::Background(solid(0x00ff00ff))),
            corners: Some(Corners {
                top_left: 24.,
                top_right: 0.,
                bottom_right: 16.,
                bottom_left: 0.,
            }),
        }),
        None,
        None,
    ));
    add(
        "appearance-corners",
        dataset(data::Contents::Cartesian(vec![data::Layer::Bar(single)])),
        options(),
        corners,
        vec![
            sample(62., 2., 0, 0, 0),
            sample(138., 2., 0, 255, 0),
            sample(138., 158., 0, 0, 0),
            sample(62., 158., 0, 255, 0),
        ],
    );
    let mut marker_source = series();
    for (i, point) in marker_source.points.iter_mut().enumerate() {
        point.y = Some(i as f64);
    }
    let mut markers = style();
    markers.appearance.series.push(series_override(
        None,
        None,
        Some(a::Marker {
            fill: Some(0x00ff00ff),
            stroke: Some(0x0000ffff),
            radius: Some(12.),
            stroke_width: Some(4.),
            ..Default::default()
        }),
    ));
    markers.appearance.data.push(a::Datum {
        series: 1,
        datum: 2,
        bar: None,
        marker: Some(a::Marker {
            radius: Some(24.),
            ..Default::default()
        }),
    });
    let mut dots = options();
    dots.cartesian.dots = true;
    add(
        "appearance-markers",
        dataset(data::Contents::Cartesian(vec![data::Layer::Line(
            marker_source,
        )])),
        dots,
        markers,
        vec![
            sample(100., 80., 0, 255, 0),
            sample(122., 80., 0, 0, 255),
            sample(126., 80., 0, 0, 0),
        ],
    );
    let mut area = style();
    area.area_opacity = 0.01;
    area.appearance.series.push(series_override(
        None,
        Some(a::Path {
            fill: Some(solid(0x0000ff80)),
            stroke: Some(a::Stroke {
                visible: true,
                width: Some(8.),
                brush: solid(0x00ff00ff),
            }),
            curve: None,
        }),
        None,
    ));
    add(
        "appearance-area",
        dataset(data::Contents::Cartesian(vec![data::Layer::Area(series())])),
        options(),
        area,
        vec![sample(100., 80., 0, 0, 128), sample(50., 80., 0, 255, 0)],
    );
    for baseline in [0., 1.] {
        for orientation in [
            Orientation::Vertical,
            Orientation::Horizontal,
            Orientation::VerticalReversed,
            Orientation::HorizontalReversed,
        ] {
            let mut config = options();
            config.cartesian.orientation = orientation;
            let mut appearance = style();
            appearance.appearance.series.push(a::Series {
                area_baseline: Some(baseline),
                ..series_override(
                    None,
                    Some(a::Path {
                        fill: Some(solid(0x0000ffff)),
                        stroke: Some(a::Stroke {
                            visible: false,
                            width: None,
                            brush: solid(0),
                        }),
                        curve: Some(gpuio_protocol::chart_options::Curve::Linear),
                    }),
                    None,
                )
            });
            let at = |value: f32, blue| {
                let value = if orientation.is_reversed() {
                    1. - value
                } else {
                    value
                };
                let (x, y) = if orientation.is_horizontal() {
                    (value * 200., 40.)
                } else {
                    (50., (1. - value) * 160.)
                };
                sample(x, y, 0, 0, blue)
            };
            add(
                "area-baseline",
                dataset(data::Contents::Cartesian(vec![data::Layer::Area(series())])),
                config,
                appearance,
                vec![
                    at(0.25, if baseline == 0. { 255 } else { 0 }),
                    at(0.875, if baseline == 1. { 255 } else { 0 }),
                ],
            );
        }
    }
    let mut line = series();
    for point in &mut line.points {
        point.y = Some(1.);
    }
    let mut path_gradient = style();
    path_gradient.appearance.series.push(series_override(
        None,
        Some(a::Path {
            stroke: Some(a::Stroke {
                visible: true,
                width: Some(8.),
                brush: gradient(90., 0xff0000ff, 0x0000ffff),
            }),
            ..Default::default()
        }),
        None,
    ));
    add(
        "appearance-path-gradient",
        dataset(data::Contents::Cartesian(vec![data::Layer::Line(line)])),
        options(),
        path_gradient,
        vec![],
    );
    add(
        "line",
        dataset(data::Contents::Cartesian(vec![data::Layer::Line(series())])),
        options(),
        style(),
        vec![sample(50., 80., 255, 0, 0), sample(100., 80., 0, 0, 0)],
    );
    add(
        "area",
        dataset(data::Contents::Cartesian(vec![data::Layer::Area(series())])),
        options(),
        style(),
        vec![sample(100., 80., 64, 0, 0), sample(50., 20., 0, 0, 0)],
    );
    let mut bars = series();
    bars.points.truncate(2);
    bars.points[0].y = Some(1.);
    bars.points[1].y = Some(2.);
    for orientation in [
        Orientation::Vertical,
        Orientation::Horizontal,
        Orientation::VerticalReversed,
        Orientation::HorizontalReversed,
    ] {
        for variant in 0..3 {
            let mut dense_series = bars.clone();
            dense_series.points[0].id = 90;
            dense_series.points[1].id = 7;
            dense_series.points[0].y = Some(2.);
            let mut data = dataset(data::Contents::Cartesian(vec![data::Layer::Bar(
                dense_series,
            )]));
            data.bar_backgrounds = vec![
                data::BarBackground {
                    series: 1,
                    datum: 7,
                    brush: if variant == 2 {
                        a::Brush::Checkerboard(0x0000ffff, 8.)
                    } else {
                        a::Brush::Solid(0x0000ffff)
                    },
                },
                data::BarBackground {
                    series: 1,
                    datum: 90,
                    brush: a::Brush::Solid(0x00ff00ff),
                },
            ];
            let mut options = options();
            options.cartesian.orientation = orientation;
            let mut style = style();
            if variant == 1 {
                style.appearance.data.push(a::Datum {
                    series: 1,
                    datum: 7,
                    marker: None,
                    bar: Some(a::Bar {
                        fill: Some(a::BarFill::Background(a::Brush::Solid(0xffff00ff))),
                        corners: None,
                    }),
                });
            }
            let (first, second, empty) = if orientation.is_horizontal() {
                ((100., 40.), (100., 120.), (100., 2.))
            } else {
                ((50., 80.), (150., 80.), (2., 80.))
            };
            let mut samples = vec![
                sample(first.0, first.1, 0, 255, 0),
                sample(empty.0, empty.1, 0, 0, 0),
            ];
            if variant != 2 {
                let (r, g, b) = if variant == 1 {
                    (255, 255, 0)
                } else {
                    (0, 0, 255)
                };
                samples.push(sample(second.0, second.1, r, g, b));
            }
            add(
                if variant == 2 {
                    "dense-background-pattern"
                } else {
                    "dense-background-solid"
                },
                data,
                options,
                style,
                samples,
            );
        }
    }
    add(
        "bar",
        dataset(data::Contents::Cartesian(vec![data::Layer::Bar(
            bars.clone(),
        )])),
        options(),
        style(),
        vec![
            sample(50., 100., 255, 0, 0),
            sample(150., 80., 255, 0, 0),
            sample(10., 80., 0, 0, 0),
        ],
    );
    for (name, orientation, samples) in [
        (
            "bar-top",
            gpuio_protocol::chart_options::Orientation::VerticalReversed,
            vec![sample(50., 30., 255, 0, 0), sample(50., 120., 0, 0, 0)],
        ),
        (
            "bar-left",
            gpuio_protocol::chart_options::Orientation::Horizontal,
            vec![sample(50., 40., 255, 0, 0), sample(150., 40., 0, 0, 0)],
        ),
        (
            "bar-right",
            gpuio_protocol::chart_options::Orientation::HorizontalReversed,
            vec![sample(150., 40., 255, 0, 0), sample(50., 40., 0, 0, 0)],
        ),
    ] {
        let mut options = options();
        options.cartesian.orientation = orientation;
        add(
            name,
            dataset(data::Contents::Cartesian(vec![data::Layer::Bar(
                bars.clone(),
            )])),
            options,
            style(),
            samples,
        );
    }
    for (orientation, suffix) in [
        (gpuio_protocol::chart_options::Orientation::Vertical, 0),
        (gpuio_protocol::chart_options::Orientation::Horizontal, 1),
        (
            gpuio_protocol::chart_options::Orientation::VerticalReversed,
            2,
        ),
        (
            gpuio_protocol::chart_options::Orientation::HorizontalReversed,
            3,
        ),
    ] {
        for area in [false, true] {
            let mut a = series();
            let mut b = series();
            b.id = 2;
            for p in &mut a.points {
                p.y = Some(1.);
            }
            for p in &mut b.points {
                p.y = Some(2.);
            }
            let wrap = if area {
                data::Layer::Area
            } else {
                data::Layer::Bar
            };
            let mut options = options();
            options.cartesian.stacking = gpuio_protocol::chart_options::Stacking::Stacked;
            options.cartesian.orientation = orientation;
            let names = if area {
                [
                    "stack-area-up",
                    "stack-area-right",
                    "stack-area-down",
                    "stack-area-left",
                ]
            } else {
                [
                    "stack-bar-up",
                    "stack-bar-right",
                    "stack-bar-down",
                    "stack-bar-left",
                ]
            };
            let at = |fraction: f32, red, blue| {
                let fraction = if orientation.is_reversed() {
                    1. - fraction
                } else {
                    fraction
                };
                if orientation.is_horizontal() {
                    sample(200. * fraction, 80., red, 0, blue)
                } else {
                    sample(100., 160. * (1. - fraction), red, 0, blue)
                }
            };
            let value = if area { 64 } else { 255 };
            add(
                names[suffix],
                dataset(data::Contents::Cartesian(vec![wrap(a), wrap(b)])),
                options,
                style(),
                vec![at(0.2, value, 0), at(0.7, 0, value)],
            );
        }
    }
    let category_data = |bar| {
        let categories = [42, 7, 99]
            .into_iter()
            .map(|id| data::Category {
                id,
                label: format!("Category {id}"),
            })
            .collect();
        let points = [42, 7, 99]
            .into_iter()
            .enumerate()
            .map(|(i, category)| data::CategoricalPoint {
                id: i as i64 + 1,
                category,
                value: if bar {
                    [Some(1.), None, Some(2.)][i]
                } else {
                    Some([0., 1., 0.][i])
                },
                label: String::new(),
            })
            .collect();
        let series = data::CategoricalSeries {
            id: 1,
            name: "Categories".into(),
            points,
        };
        dataset(data::Contents::Categorical(
            categories,
            vec![if bar {
                data::CategoricalLayer::Bar(series)
            } else {
                data::CategoricalLayer::Line(series)
            }],
        ))
    };
    add(
        "categorical-point",
        category_data(false),
        options(),
        style(),
        vec![sample(50., 80., 255, 0, 0), sample(100., 80., 0, 0, 0)],
    );
    add(
        "categorical-band-missing",
        category_data(true),
        options(),
        style(),
        vec![
            sample(33., 120., 255, 0, 0),
            sample(33., 40., 0, 0, 0),
            sample(100., 120., 0, 0, 0),
            sample(167., 40., 255, 0, 0),
        ],
    );
    for categorical in [false, true] {
        for orientation in [
            gpuio_protocol::chart_options::Orientation::Vertical,
            gpuio_protocol::chart_options::Orientation::Horizontal,
            gpuio_protocol::chart_options::Orientation::VerticalReversed,
            gpuio_protocol::chart_options::Orientation::HorizontalReversed,
        ] {
            for (x, y) in [(true, false), (false, true), (true, true), (false, false)] {
                let mut options = options();
                options.axes.x = x;
                options.axes.y = y;
                options.cartesian.orientation = orientation;
                let mut style = style();
                style.axis_color = 0x00ff00ff;
                let contents = if categorical {
                    data::Contents::Categorical(
                        vec![data::Category {
                            id: 42,
                            label: "Alpha".into(),
                        }],
                        vec![],
                    )
                } else {
                    data::Contents::Cartesian(vec![])
                };
                add(
                    if categorical {
                        "categorical-axes"
                    } else {
                        "numeric-axes"
                    },
                    dataset(contents),
                    options,
                    style,
                    vec![],
                );
            }
        }
    }
    for categorical in [false, true] {
        for orientation in [
            gpuio_protocol::chart_options::Orientation::Vertical,
            gpuio_protocol::chart_options::Orientation::Horizontal,
            gpuio_protocol::chart_options::Orientation::VerticalReversed,
            gpuio_protocol::chart_options::Orientation::HorizontalReversed,
        ] {
            let mut options = options();
            options.axes.x = true;
            options.axes.y = true;
            options.axes.grid = true;
            options.cartesian.orientation = orientation;
            let mut style = style();
            style.x_axis.position = Some(0.25);
            style.x_axis.line_width = 4.;
            style.x_axis.line_color = Some(0x00ff00ff);
            style.y_axis.position = Some(0.75);
            style.y_axis.line_width = 4.;
            style.y_axis.line_color = Some(0xff0000ff);
            style.grid.x = Some(vec![gpuio_protocol::chart_axis::TickPosition::Fraction(
                0.5,
            )]);
            style.grid.y = Some(vec![]);
            style.grid.dashes = vec![8., 4., 2.];
            style.grid.width = 4.;
            style.grid.color = Some(0x0000ffff);
            let horizontal = orientation.is_horizontal();
            let mut samples = if horizontal {
                vec![
                    sample(50., 20., 0, 255, 0),
                    sample(20., 120., 255, 0, 0),
                    sample(100., 159., 0, 0, 0),
                ]
            } else {
                vec![
                    sample(20., 40., 0, 255, 0),
                    sample(150., 20., 255, 0, 0),
                    sample(0., 80., 0, 0, 0),
                ]
            };
            for (distance, painted) in [
                (3., true),
                (10., false),
                (13., true),
                (18., false),
                (24., true),
                (27., false),
            ] {
                let (x, y) = if horizontal {
                    (distance, 80.)
                } else {
                    (100., distance)
                };
                samples.push(sample(x, y, 0, 0, if painted { 255 } else { 0 }));
            }
            let contents = if categorical {
                data::Contents::Categorical(
                    vec![data::Category {
                        id: 42,
                        label: "Alpha".into(),
                    }],
                    vec![],
                )
            } else {
                data::Contents::Cartesian(vec![])
            };
            add(
                "custom-axis-grid",
                dataset(contents),
                options,
                style,
                samples,
            );
        }
    }
    let slices = vec![
        data::Slice {
            id: 1,
            label: "right".into(),
            value: 1.,
        },
        data::Slice {
            id: 2,
            label: "left".into(),
            value: 1.,
        },
    ];
    add(
        "pie",
        dataset(data::Contents::Pie(slices.clone())),
        options(),
        style(),
        vec![sample(140., 80., 255, 0, 0), sample(60., 80., 0, 0, 255)],
    );
    for (name, ids, unknown, samples) in [
        (
            "ordinal-pie-before",
            [1, 2],
            Some(0x00ff00ff),
            vec![sample(140., 80., 0, 0, 255), sample(60., 80., 255, 0, 0)],
        ),
        (
            "ordinal-pie-reordered",
            [2, 1],
            Some(0x00ff00ff),
            vec![sample(140., 80., 255, 0, 0), sample(60., 80., 0, 0, 255)],
        ),
        (
            "ordinal-pie-unknown",
            [3, 1],
            Some(0x00ff00ff),
            vec![sample(140., 80., 0, 255, 0), sample(60., 80., 0, 0, 255)],
        ),
        (
            "ordinal-pie-fallback",
            [3, 1],
            None,
            vec![sample(140., 80., 255, 0, 0), sample(60., 80., 0, 0, 255)],
        ),
    ] {
        let mut style = style();
        style.ordinal = Some(gpuio_protocol::chart_style::Ordinal {
            domain: vec![
                gpuio_protocol::chart_style::Key::Slice(2),
                gpuio_protocol::chart_style::Key::Slice(1),
            ],
            range: vec![0xff0000ff, 0x0000ffff],
            unknown,
        });
        add(
            name,
            dataset(data::Contents::Pie(
                ids.map(|id| data::Slice {
                    id,
                    label: format!("Slice {id}"),
                    value: 1.,
                })
                .to_vec(),
            )),
            options(),
            style,
            samples,
        );
    }
    let mut fixed_pie = options();
    fixed_pie.pie.radius = gpuio_protocol::chart_options::PieRadius::Pixels(50.);
    fixed_pie.pie.labels = false;
    add(
        "pie-fixed-radius",
        dataset(data::Contents::Pie(slices.clone())),
        fixed_pie.clone(),
        style(),
        vec![
            sample(140., 80., 255, 0, 0),
            sample(160., 80., 0, 0, 0),
            sample(60., 80., 0, 0, 255),
            sample(40., 80., 0, 0, 0),
        ],
    );
    fixed_pie.pie.slice_radii = vec![gpuio_protocol::chart_options::SliceRadii {
        slice: 1,
        inner: 20.,
        outer: 35.,
    }];
    add(
        "pie-variable-radius",
        dataset(data::Contents::Pie(slices.clone())),
        fixed_pie.clone(),
        style(),
        vec![
            sample(110., 80., 0, 0, 0),
            sample(128., 80., 255, 0, 0),
            sample(144., 80., 0, 0, 0),
            sample(60., 80., 0, 0, 255),
        ],
    );
    let mut reordered = slices.clone();
    reordered.reverse();
    add(
        "pie-radius-reordered",
        dataset(data::Contents::Pie(reordered)),
        fixed_pie.clone(),
        style(),
        vec![
            sample(90., 80., 0, 0, 0),
            sample(72., 80., 0, 0, 255),
            sample(56., 80., 0, 0, 0),
            sample(140., 80., 255, 0, 0),
        ],
    );
    fixed_pie.pie.slice_radii[0].inner = 35.;
    add(
        "pie-equal-radii",
        dataset(data::Contents::Pie(slices.clone())),
        fixed_pie,
        style(),
        vec![sample(128., 80., 0, 0, 0), sample(60., 80., 0, 0, 255)],
    );
    let mut donut = options();
    donut.pie.inner_radius = 0.5;
    add(
        "donut",
        dataset(data::Contents::Pie(slices)),
        donut,
        style(),
        vec![
            sample(160., 80., 255, 0, 0),
            sample(40., 80., 0, 0, 255),
            sample(100., 80., 0, 0, 0),
        ],
    );
    let axes = (1..=3)
        .map(|id| data::RadarAxis {
            id,
            label: format!("Axis {id}"),
            maximum: 10.,
        })
        .collect();
    add(
        "radar",
        dataset(data::Contents::Radar(
            axes,
            vec![data::RadarSeries {
                id: 1,
                name: "radar".into(),
                values: vec![(1, 10.), (2, 10.), (3, 10.)],
            }],
        )),
        options(),
        style(),
        vec![sample(90., 65., 64, 0, 0), sample(100., 140., 0, 0, 0)],
    );
    let candles = vec![
        data::Candle {
            id: 1,
            x: 0.,
            label: "rise".into(),
            open_: 0.,
            close: 2.,
            high: 3.,
            low: -1.,
        },
        data::Candle {
            id: 2,
            x: 1.,
            label: "fall".into(),
            open_: 2.,
            close: 0.,
            high: 3.,
            low: -1.,
        },
    ];
    add(
        "candlestick",
        dataset(data::Contents::Candlestick(candles)),
        options(),
        style(),
        vec![
            sample(50., 80., 0, 0, 0),
            sample(150., 80., 0, 0, 255),
            sample(16., 80., 255, 0, 0),
            // At 1x, x=50 samples a pixel centered on the wick's exclusive
            // right edge (50.5). Select a pixel inside the one-unit wick.
            sample(49.75, 20., 255, 0, 0),
        ],
    );
    add(
        "sankey",
        dataset(data::Contents::Sankey(
            vec![
                data::Node {
                    id: 1,
                    label: "source".into(),
                },
                data::Node {
                    id: 2,
                    label: "target".into(),
                },
            ],
            vec![data::Edge {
                id: 1,
                source: 1,
                target: 2,
                value: 1e100,
            }],
        )),
        options(),
        style(),
        vec![
            sample(5., 80., 255, 0, 0),
            sample(195., 80., 0, 0, 255),
            sample(100., 80., 128, 0, 0),
        ],
    );
    for (name, mode) in [
        ("sankey-target", LinkColor::Target),
        ("sankey-gradient", LinkColor::Gradient),
    ] {
        let mut o = options();
        o.sankey.link_color = mode;
        add(
            name,
            dataset(data::Contents::Sankey(
                vec![
                    data::Node {
                        id: 1,
                        label: "source".into(),
                    },
                    data::Node {
                        id: 2,
                        label: "target".into(),
                    },
                ],
                vec![data::Edge {
                    id: 1,
                    source: 1,
                    target: 2,
                    value: 1.,
                }],
            )),
            o,
            style(),
            if mode == LinkColor::Target {
                vec![sample(100., 80., 0, 0, 128)]
            } else {
                vec![]
            },
        );
    }
    // A later line must paint above an earlier bar, across primitive types.
    let mut line = bars.clone();
    line.id = 2;
    line.points[0].y = Some(1.);
    line.points[1].y = Some(1.);
    add(
        "mixed-order",
        dataset(data::Contents::Cartesian(vec![
            data::Layer::Bar(bars.clone()),
            data::Layer::Line(line),
        ])),
        options(),
        style(),
        vec![sample(145., 80., 0, 0, 255)],
    );
    let mut gradient = style();
    gradient.gradient_end = Some(0x0000ffff);
    add(
        "gradient",
        dataset(data::Contents::Cartesian(vec![data::Layer::Bar(
            bars.clone(),
        )])),
        options(),
        gradient.clone(),
        vec![],
    );
    let mut reversed = options();
    reversed.cartesian.orientation = gpuio_protocol::chart_options::Orientation::VerticalReversed;
    add(
        "gradient-reversed",
        dataset(data::Contents::Cartesian(vec![data::Layer::Bar(bars)])),
        reversed,
        gradient,
        vec![],
    );
    out
}
fn pixel(image: &image::RgbaImage, bounds: Bounds<Pixels>, scale: f32, x: f32, y: f32) -> [u8; 4] {
    image
        .get_pixel(
            ((f32::from(bounds.origin.x) + x) * scale) as u32,
            ((f32::from(bounds.origin.y) + y) * scale) as u32,
        )
        .0
}
async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    cases: Vec<(Case, Arc<Prepared>)>,
    scale: f32,
) {
    handle
        .update(cx, |_, window, _| window.set_scale_factor(scale))
        .unwrap();
    for (case, prepared) in cases {
        handle
            .update(cx, |view, _, cx| {
                view.prepared = prepared.clone();
                cx.notify();
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        handle
            .update(cx, |view, window, _| {
                let image = window.render_to_image().expect("chart GPU readback");
                let bounds = view.bounds.get();
                let scale = window.scale_factor();
                for (x, y, color) in case.samples {
                    let actual = pixel(&image, bounds, scale, x, y);
                    for channel in 0..4 {
                        assert!(
                            actual[channel].abs_diff(color[channel]) <= 2,
                            "{} at{x},{y}: {actual:?} vs {color:?}",
                            case.name
                        );
                    }
                }
                // A stroke centered at the boundary must be clipped to the plot.
                assert_eq!(
                    pixel(&image, bounds, scale, -3., 80.),
                    [0, 0, 0, 255],
                    "{} plot clip",
                    case.name
                );
                if matches!(case.name, "appearance-base-tip" | "appearance-value-plateaus") {
                    let horizontal = case.options.cartesian.orientation.is_horizontal();
                    let (first, last) = if horizontal {
                        (pixel(&image, bounds, scale, 20., 80.), pixel(&image, bounds, scale, 180., 80.))
                    } else { (pixel(&image, bounds, scale, 100., 20.), pixel(&image, bounds, scale, 100., 140.)) };
                    let data::Contents::Cartesian(layers) = &case.data.contents else { panic!() };
                    let negative = layers[0].series().points[0].y.unwrap() < 0.;
                    let tip_at_start = !horizontal ^ case.options.cartesian.orientation.is_reversed() ^ negative;
                    let (base, tip) = if tip_at_start { (last, first) } else { (first, last) };
                    assert!(base[0] > tip[0] + 100 && tip[2] > base[2] + 100,
                        "{} signed orientation {:?}, negative={negative}: base{base:?} tip{tip:?}", case.name, case.options.cartesian.orientation);
                    if case.name == "appearance-value-plateaus" {
                        assert!(base[0] > 247 && base[2] < 8 && tip[2] > 247 && tip[0] < 8,
                            "value endpoints remain plateaus: {base:?} {tip:?}");
                    }
                }
                if case.name == "dense-background-pattern" {
                    let (left, top) = if case.options.cartesian.orientation.is_horizontal() { (80., 110.) } else { (140., 60.) };
                    let mut blue = 0;
                    let mut gap = 0;
                    for y in 0..24 {
                        for x in 0..24 {
                            let p = pixel(&image, bounds, scale, left + x as f32, top + y as f32);
                            blue += usize::from(p[2] > 180 && p[0] < 30 && p[1] < 30);
                            gap += usize::from(p[0] < 30 && p[1] < 30 && p[2] < 30);
                        }
                    }
                    assert!(blue > 32 && gap > 32, "dense checker must retain colored and transparent cells: blue={blue} gap={gap}");
                }
                if case.name == "appearance-path-gradient" {
                    let first = pixel(&image, bounds, scale, 20., 80.);
                    let last = pixel(&image, bounds, scale, 180., 80.);
                    assert!(first[0] > last[0] + 100 && last[2] > first[2] + 100,
                        "path brush spans its prepared bounds: {first:?} {last:?}");
                }
                if case.name == "sankey-gradient" {
                    let left = pixel(&image, bounds, scale, 35., 80.);
                    let right = pixel(&image, bounds, scale, 165., 80.);
                    assert!(left[0] > right[0] + 50 && right[2] > left[2] + 50,
                        "ribbon blends source red to target blue: {left:?} {right:?}");
                    // The pinned Metal shader adds +/-2/255 RGB and +/-3/255
                    // alpha dither. On black at half opacity, adjacent samples
                    // may differ by ~8 levels plus the gradient slope/rounding.
                    // A restarted triangle gradient is much larger than this.
                    for x in 40..160 {
                        let before = pixel(&image, bounds, scale, (x - 1) as f32, 80.);
                        let after = pixel(&image, bounds, scale, x as f32, 80.);
                        assert!(before[0].abs_diff(after[0]) <= 10 && before[2].abs_diff(after[2]) <= 10,
                            "gradient must not restart at triangle boundaries: {x}: {before:?} {after:?}");
                    }
                }
                if case.name == "gradient" {
                    let top = pixel(&image, bounds, scale, 150., 20.);
                    let bottom = pixel(&image, bounds, scale, 150., 140.);
                    assert!(
                        top[0] > bottom[0] + 50 && bottom[2] > top[2] + 50,
                        "gradient follows vertical value axis: {top:?} {bottom:?}"
                    );
                }
                if case.name == "gradient-reversed" {
                    let top = pixel(&image, bounds, scale, 150., 20.);
                    let bottom = pixel(&image, bounds, scale, 150., 140.);
                    assert!(
                        bottom[0] > top[0] + 50 && top[2] > bottom[2] + 50,
                        "gradient mirrors reversed value axis: {top:?} {bottom:?}"
                    );
                }
                if matches!(case.name, "numeric-axes" | "categorical-axes") {
                    let green = |left: f32, top: f32, right: f32, bottom: f32| {
                        let mut count = 0;
                        for y in ((f32::from(bounds.origin.y) + top) * scale).ceil() as u32
                            ..((f32::from(bounds.origin.y) + bottom) * scale).floor() as u32 {
                            for x in ((f32::from(bounds.origin.x) + left) * scale).ceil() as u32
                                ..((f32::from(bounds.origin.x) + right) * scale).floor() as u32 {
                                let p = image.get_pixel(x, y).0;
                                count += usize::from(p[1] > 32 && p[1].saturating_sub(p[0]) > 30
                                    && p[1].saturating_sub(p[2]) > 30);
                            }
                        }
                        count
                    };
                    let horizontal = case.options.cartesian.orientation.is_horizontal();
                    let (left, bottom) = if horizontal {
                        (case.options.axes.x, case.options.axes.y)
                    } else { (case.options.axes.y, case.options.axes.x) };
                    assert_eq!(green(0., 40., 2., 120.) > 0, left,
                        "{} left axis at scale {scale}, {:?}, x={} y={}", case.name,
                        case.options.cartesian.orientation, case.options.axes.x, case.options.axes.y);
                    assert_eq!(green(40., 158., 160., 160.) > 0, bottom,
                        "{} bottom axis at scale {scale}, {:?}, x={} y={}", case.name,
                        case.options.cartesian.orientation, case.options.axes.x, case.options.axes.y);
                    assert_eq!(pixel(&image, bounds, scale, 100., 80.), [0, 0, 0, 255]);
                }
                eprintln!(
                    "CHART_PAINT_GPU {} scale={} meshes={} quads={} vertices={} retained_bytes={}",
                    case.name,
                    scale,
                    prepared.mesh_count(),
                    prepared.quad_count(),
                    prepared.vertices(),
                    prepared.retained_bytes()
                );
            })
            .unwrap();
        // Drawing the same retained plan again must not prepare another mesh.
        cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    }
    eprintln!(
        "GPUIO_NATIVE_CHART_PAINT_OK scale={scale}: all seven families, mixed layer ordering, donut holes, area alpha, hollow/filled candles, bar corners/gradient, Sankey endpoint gradients and clipping; hidden-window GPU pixels only"
    );
}
pub(crate) fn run() {
    // Preparation really runs off the native/UI thread.
    let cases = std::thread::spawn(|| {
        [1.0_f32, 1.25, 1.5, 2.]
            .into_iter()
            .map(|scale| {
                let prepared_cases = cases()
                    .into_iter()
                    .map(|case| {
                        let prepared = prepare(
                            &case.data,
                            Policy::default(),
                            &case.options,
                            &case.style,
                            Layout::new(200., 160., f64::from(scale)).unwrap(),
                            &AtomicBool::new(false),
                        )
                        .unwrap();
                        (case, Arc::new(prepared))
                    })
                    .collect::<Vec<_>>();
                (scale, prepared_cases)
            })
            .collect::<Vec<_>>()
    })
    .join()
    .unwrap();
    let failure = Rc::new(RefCell::new(None));
    let result = failure.clone();
    gpui_platform::application().run(move |cx: &mut App| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let initial = cases[0].1[0].1.clone();
        let handle = cx
            .open_window(
                WindowOptions {
                    focus: false,
                    show: false,
                    window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(
                        None,
                        // Synthetic scale changes do not resize the platform
                        // drawable. Reserve room for a 2x plot on a 1x display.
                        size(px(480.), px(400.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| View {
                        prepared: initial,
                        bounds: Rc::new(Cell::new(Bounds::default())),
                    })
                },
            )
            .unwrap();
        cx.spawn(async move |cx| {
            for (scale, cases) in cases {
                if let Err(error) =
                    crate::host::native_test::protect(exercise(cx, handle, cases, scale)).await
                {
                    *result.borrow_mut() = Some(error);
                    break;
                }
            }
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
