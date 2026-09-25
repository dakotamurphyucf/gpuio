//! Real native worker scheduling and GPU mesh pixels. This is geometry/host
//! acceptance, not the public OCaml widget or keyboard/accessibility scenario.
use crate::{
    asset_store, canvas_host,
    canvas_jobs::{Ready, Request},
    canvas_paint::{self, FrameBudget, Placement},
    canvas_plan::Quality,
    canvas_store,
};
use binprot::BinProtWrite;
use gpui::{
    App, AsyncApp, Bounds, Context, Render, Window, WindowHandle, WindowOptions, div, prelude::*,
    px, size,
};
use gpuio_protocol::{canvas::*, canvas_resource::Update, canvas_scene::*, canvas_view::Viewport};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

struct View {
    job: Option<canvas_host::Handle>,
    ready: Option<Rc<Ready>>,
    viewport: Viewport,
    bounds: Rc<Cell<Bounds<gpui::Pixels>>>,
    vertices: Rc<Cell<usize>>,
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if let Some(ready) = self.job.as_ref().and_then(|job| job.take_ready()) {
            self.ready = Some(Rc::new(ready.expect("canvas preparation")));
        }
        let ready = self.ready.clone();
        let viewport = self.viewport;
        let bounds_cell = self.bounds.clone();
        let vertices = self.vertices.clone();
        div().size_full().bg(gpui::rgb(0x101010)).child(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    bounds_cell.set(bounds);
                    let mut budget = FrameBudget::default();
                    if let Some(ready) = ready {
                        for (item, meshes) in
                            ready.snapshot.scene.items.iter().zip(&ready.plan.shapes)
                        {
                            let Drawing::Shape(_, paint) = item.drawing else {
                                panic!("shape-only native fixture");
                            };
                            let meshes = meshes.as_ref().unwrap();
                            let placement = Placement {
                                origin: meshes.origin,
                                transform: item.transform,
                                viewport,
                                bounds,
                                clips: &item.clips,
                            };
                            if let (Some(mesh), Some(color)) = (&meshes.fill, paint.fill) {
                                canvas_paint::paint(
                                    mesh.mesh(),
                                    &placement,
                                    color as u32,
                                    &mut budget,
                                    window,
                                )
                                .unwrap();
                            }
                            if let (Some(mesh), Some(stroke)) = (&meshes.stroke, paint.stroke) {
                                canvas_paint::paint(
                                    mesh.mesh(),
                                    &placement,
                                    stroke.color as u32,
                                    &mut budget,
                                    window,
                                )
                                .unwrap();
                            }
                        }
                    }
                    vertices.set(budget.used_vertices());
                },
            )
            .size_full(),
        )
    }
}
fn scene(color: i64) -> Scene {
    let mut items = vec![
        Item {
            id: 1,
            transform: Transform::IDENTITY,
            clips: vec![],
            interaction: None,
            drawing: Drawing::Shape(
                Shape::Rectangle(Rect {
                    x: 0.,
                    y: 0.,
                    width: 200.,
                    height: 200.,
                }),
                Paint {
                    fill: Some(color),
                    stroke: None,
                },
            ),
        },
        Item {
            id: 2,
            transform: Transform {
                tx: 40.,
                ty: 40.,
                ..Transform::IDENTITY
            },
            clips: vec![Rect {
                x: 60.,
                y: 60.,
                width: 40.,
                height: 40.,
            }],
            interaction: None,
            drawing: Drawing::Shape(
                Shape::Rectangle(Rect {
                    x: 0.,
                    y: 0.,
                    width: 80.,
                    height: 80.,
                }),
                Paint {
                    fill: Some(0xff0000ff),
                    stroke: None,
                },
            ),
        },
        Item {
            id: 3,
            transform: Transform::IDENTITY,
            clips: vec![],
            interaction: None,
            drawing: Drawing::Shape(
                Shape::Ellipse(Rect {
                    x: 110.,
                    y: 20.,
                    width: 60.,
                    height: 60.,
                }),
                Paint {
                    fill: Some(0x00ff00ff),
                    stroke: None,
                },
            ),
        },
    ];
    let path = Path(vec![
        PathCommand::Move(Point { x: 20., y: 130. }),
        PathCommand::Line(Point { x: 80., y: 130. }),
        PathCommand::Quadratic(Point { x: 100., y: 100. }, Point { x: 140., y: 140. }),
    ]);
    items.push(Item {
        id: 4,
        transform: Transform::IDENTITY,
        clips: vec![],
        interaction: None,
        drawing: Drawing::Shape(
            Shape::Path(ResourceKey {
                id: 1,
                generation: 1,
            }),
            Paint {
                fill: None,
                stroke: Some(Stroke {
                    color: 0xffff00ff,
                    width: 4.,
                }),
            },
        ),
    });
    Scene {
        version: 1,
        description: "Canvas GPU fixture".into(),
        resources: vec![Resource {
            key: ResourceKey {
                id: 1,
                generation: 1,
            },
            data: ResourceData::Path(path),
        }],
        items,
    }
}
fn publish(
    store: &mut canvas_store::Store,
    id: gpuio_protocol::ResourceId,
    base: i64,
    color: i64,
) -> Arc<canvas_store::Snapshot> {
    let mut bytes = vec![];
    scene(color).binprot_write(&mut bytes).unwrap();
    store
        .begin(Update {
            id,
            base,
            revision: base + 1,
            generation: 1,
            bytes: bytes.len() as i64,
        })
        .unwrap();
    store.chunk(id, base + 1, 0, &bytes).unwrap();
    store
        .publish(id, base + 1, &asset_store::Store::default())
        .unwrap();
    store.acquire(id).unwrap().snapshot()
}
fn request(snapshot: Arc<canvas_store::Snapshot>, viewport: Viewport, window: &Window) -> Request {
    Request {
        observer: None,
        snapshot,
        quality: Quality::new(viewport.zoom, f64::from(window.scale_factor())).unwrap(),
    }
}
fn draw(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
}
async fn ready(cx: &mut AsyncApp, handle: WindowHandle<View>, revision: i64, quality: Quality) {
    for _ in 0..300 {
        draw(cx, handle);
        if handle
            .update(cx, |view, _, _| {
                view.ready
                    .as_ref()
                    .is_some_and(|r| r.snapshot.revision == revision && r.quality == quality)
            })
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("canvas worker did not settle");
}
fn pixels(cx: &mut AsyncApp, handle: WindowHandle<View>, samples: &[(f64, f64, [u8; 4])]) {
    draw(cx, handle);
    handle
        .update(cx, |view, window, _| {
            let image = window
                .render_to_image()
                .expect("native canvas GPU readback");
            let bounds = view.bounds.get();
            let scale = f64::from(window.scale_factor());
            for &(world_x, world_y, expected) in samples {
                let x = ((f64::from(f32::from(bounds.origin.x))
                    + (world_x - view.viewport.origin.x) * view.viewport.zoom)
                    * scale) as u32;
                let y = ((f64::from(f32::from(bounds.origin.y))
                    + (world_y - view.viewport.origin.y) * view.viewport.zoom)
                    * scale) as u32;
                assert_eq!(
                    image.get_pixel(x, y).0,
                    expected,
                    "world pixel {world_x},{world_y} at {x},{y}"
                );
            }
            assert!(view.vertices.get() > 6);
        })
        .unwrap();
}
async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    mut store: canvas_store::Store,
    id: gpuio_protocol::ResourceId,
) {
    let quality = handle
        .update(cx, |_, window, _| {
            Quality::new(1., f64::from(window.scale_factor())).unwrap()
        })
        .unwrap();
    ready(cx, handle, 1, quality).await;
    pixels(
        cx,
        handle,
        &[
            (10., 10., [0, 0, 255, 255]),
            (80., 80., [255, 0, 0, 255]),
            (50., 50., [0, 0, 255, 255]),
            (140., 50., [0, 255, 0, 255]),
            (40., 130., [255, 255, 0, 255]),
            (105., 117.5, [255, 255, 0, 255]),
        ],
    );
    let snapshot = publish(&mut store, id, 1, 0x00ffffff);
    let viewport = Viewport {
        origin: Point { x: 20., y: 20. },
        zoom: 1.5,
    };
    let quality = handle
        .update(cx, |view, window, cx| {
            view.viewport = viewport;
            let request = request(snapshot.clone(), viewport, window);
            let quality = request.quality;
            view.job.as_ref().unwrap().update(request, cx).unwrap();
            cx.notify();
            quality
        })
        .unwrap();
    ready(cx, handle, 2, quality).await;
    pixels(
        cx,
        handle,
        &[
            (80., 80., [255, 0, 0, 255]),
            (50., 50., [0, 255, 255, 255]),
            (140., 50., [0, 255, 0, 255]),
        ],
    );
    handle
        .update(cx, |_, window, _| window.resize(size(px(320.), px(280.))))
        .unwrap();
    let mut resized = false;
    for _ in 0..200 {
        resized = handle
            .update(cx, |_, window, _| {
                window.viewport_size() == size(px(320.), px(280.))
            })
            .unwrap();
        if resized {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(resized, "native canvas viewport did not resize");
    pixels(
        cx,
        handle,
        &[
            (80., 80., [255, 0, 0, 255]),
            (105., 117.5, [255, 255, 0, 255]),
            (190., 190., [0, 255, 255, 255]),
        ],
    );
    // Discard mounted state, then shut down queued native work. A native scene
    // snapshot can outlive registration, but cannot retain prepared mesh quota.
    handle
        .update(cx, |view, window, cx| {
            view.ready = None;
            view.job = None;
            let pending: Vec<_> = (0..32)
                .map(|_| {
                    canvas_host::request(request(snapshot.clone(), viewport, window), window, cx)
                        .unwrap()
                })
                .collect();
            drop(pending);
            cx.notify();
        })
        .unwrap();
    draw(cx, handle);
    canvas_host::shutdown(cx).await;
    cx.update(|cx| {
        let stats = canvas_host::measurements(cx).unwrap();
        assert!(stats.0 >= 2 && stats.2 <= 2);
        assert_eq!(stats.3, 0, "all prepared mesh readers released");
        eprintln!("GPUIO_CANVAS_HOST_METRICS: {stats:?}");
    });
    store.release(id).unwrap();
    assert!(
        store.reserved_bytes() > 0,
        "external snapshot is still charged"
    );
    drop(snapshot);
    assert_eq!(store.reserved_bytes(), 0, "all scene readers released");
    eprintln!(
        "GPUIO_NATIVE_CANVAS_OK: hidden-window GPU shape/curve pixels, world clipping, pan/zoom, resize, native publication, worker/scene disposal; no input/AX acceptance claim"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    gpui_platform::application().run(move |cx: &mut App| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let mut store = canvas_store::Store::default();
        let id = store.create().unwrap();
        let snapshot = publish(&mut store, id, 0, 0x0000ffff);
        let handle = cx
            .open_window(
                WindowOptions {
                    focus: false,
                    show: false,
                    window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(240.), px(240.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |window, cx| {
                    let viewport = Viewport::default();
                    let job = canvas_host::request(request(snapshot, viewport, window), window, cx)
                        .unwrap();
                    cx.new(|_| View {
                        job: Some(job),
                        ready: None,
                        viewport,
                        bounds: Rc::new(Cell::new(Bounds::default())),
                        vertices: Rc::new(Cell::new(0)),
                    })
                },
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(exercise(cx, handle, store, id)).await;
            let _ = handle.update(cx, |view, window, _| {
                view.ready = None;
                view.job = None;
                window.remove_window();
            });
            canvas_host::shutdown(cx).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
