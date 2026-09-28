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
        dataset(data::Contents::Cartesian(vec![data::Layer::Bar(bars)])),
        options(),
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
                if case.name == "gradient" {
                    let top = pixel(&image, bounds, scale, 150., 20.);
                    let bottom = pixel(&image, bounds, scale, 150., 140.);
                    assert!(
                        top[0] > bottom[0] + 50 && bottom[2] > top[2] + 50,
                        "gradient follows vertical value axis: {top:?} {bottom:?}"
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
        "GPUIO_NATIVE_CHART_PAINT_OK scale={scale}: all seven families, mixed layer ordering, donut holes, area alpha, hollow/filled candles, bar corners/gradient and clipping; hidden-window GPU pixels only"
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
