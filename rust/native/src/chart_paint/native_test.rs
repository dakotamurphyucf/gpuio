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
        version: 1,
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
