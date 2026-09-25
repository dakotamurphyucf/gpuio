//! Real native worker scheduling and GPU mesh pixels. This is geometry/host
//! acceptance, not the public OCaml widget or keyboard/accessibility scenario.
use crate::{
    asset_store,
    canvas_content::{self, Content, Status},
    canvas_host,
    canvas_jobs::{Ready, Request},
    canvas_paint::{self, FrameBudget, Placement},
    canvas_plan::Quality,
    canvas_state::{PointerMode, State},
    canvas_store, image_host,
};
use binprot::BinProtWrite;
use gpui::{
    App, AsyncApp, Bounds, Context, Render, Window, WindowHandle, WindowOptions, div, prelude::*,
    px, size,
};
use gpuio_protocol::{
    canvas::*,
    canvas_resource::Update,
    canvas_scene::*,
    canvas_view::{Action, Command, Config, Observation, Viewport},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

struct View {
    job: Option<canvas_host::Handle>,
    ready: Option<Rc<Ready>>,
    state: Option<State>,
    content: Option<Rc<RefCell<Content>>>,
    pending: Rc<Cell<bool>>,
    pressure: Option<usize>,
    pressure_origin: Point,
    pressure_scale: f64,
    shaping_frames: Rc<RefCell<Vec<(usize, bool)>>>,
    pressure_error: Rc<Cell<Option<gpuio_protocol::canvas_view::Error>>>,
    bounds: Rc<Cell<Bounds<gpui::Pixels>>>,
    vertices: Rc<Cell<usize>>,
}

#[derive(Clone, Copy)]
struct SceneAssets {
    raster: gpuio_protocol::ResourceId,
    svg: gpuio_protocol::ResourceId,
}
fn scene_assets(store: &mut asset_store::Store) -> SceneAssets {
    let mut raster = b"P6\n4 4\n255\n".to_vec();
    for _ in 0..16 {
        raster.extend([255, 0, 255]);
    }
    let mut register = |format, bytes: &[u8]| {
        let id = store.begin(format, bytes.len()).unwrap();
        store.append(id, 0, bytes).unwrap();
        store.finish(id).unwrap();
        id
    };
    let raster = register(gpuio_protocol::asset::Format::Pnm, &raster);
    let svg = register(gpuio_protocol::asset::Format::Svg, br##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4" viewBox="0 0 8 4"><rect width="4" height="4" fill="#ff8000"/><rect x="4" width="4" height="4" fill="#008080"/></svg>"##);
    SceneAssets { raster, svg }
}

fn text_pixels(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    draw(cx, handle);
    handle
        .update(cx, |view, window, _| {
            let image = window.render_to_image().unwrap();
            let viewport = view.state.as_ref().unwrap().viewport();
            let bounds = view.bounds.get();
            let scale = f64::from(window.scale_factor());
            let mut inside = 0;
            let mut outside = 0;
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let world_x = (f64::from(x) / scale - f64::from(f32::from(bounds.origin.x)))
                        / viewport.zoom
                        + viewport.origin.x;
                    let world_y = (f64::from(y) / scale - f64::from(f32::from(bounds.origin.y)))
                        / viewport.zoom
                        + viewport.origin.y;
                    if !(155. ..180.).contains(&world_y) {
                        continue;
                    }
                    let [r, g, b, _] = image.get_pixel(x, y).0;
                    if r > 220 && g > 220 && b > 220 {
                        if (20. ..60.).contains(&world_x) {
                            inside += 1;
                        }
                        if (61. ..85.).contains(&world_x) {
                            outside += 1;
                        }
                    }
                }
            }
            assert!(inside > 10, "white native text is visible: {inside}");
            assert_eq!(outside, 0, "world clip excludes overflowing glyphs");
        })
        .unwrap();
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if let Some(ready) = self.job.as_ref().and_then(|job| job.take_ready()) {
            let ready = ready.expect("canvas preparation");
            self.state
                .as_mut()
                .unwrap()
                .publish(ready.snapshot.clone())
                .unwrap();
            self.content
                .as_ref()
                .unwrap()
                .borrow_mut()
                .publish(ready.snapshot.clone());
            self.ready = Some(Rc::new(ready));
        }
        let ready = self.ready.clone();
        let viewport = self
            .state
            .as_ref()
            .map_or(Viewport::default(), State::viewport);
        let transforms: Vec<_> = ready.as_ref().map_or_else(Vec::new, |ready| {
            ready
                .snapshot
                .scene
                .items
                .iter()
                .map(|item| self.state.as_ref().unwrap().transform(item))
                .collect()
        });
        let bounds_cell = self.bounds.clone();
        let vertices = self.vertices.clone();
        let content = self.content.clone();
        let pending = self.pending.clone();
        let pressure = self.pressure;
        let pressure_origin = self.pressure_origin;
        let pressure_scale = self.pressure_scale;
        let shaping_frames = self.shaping_frames.clone();
        let pressure_error = self.pressure_error.clone();
        div().size_full().bg(gpui::rgb(0x101010)).child(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    bounds_cell.set(bounds);
                    let mut budget = FrameBudget::default();
                    let mut deferred = false;
                    let shapes_before = content
                        .as_ref()
                        .map_or(0, |content| content.borrow().statistics().2);
                    pending.set(false);
                    pressure_error.set(None);
                    if let Some(content) = &content {
                        content.borrow_mut().begin_frame();
                    }
                    if let Some(ready) = ready {
                        for ((item, meshes), transform) in ready
                            .snapshot
                            .scene
                            .items
                            .iter()
                            .zip(&ready.plan.shapes)
                            .zip(transforms)
                        {
                            let placement = Placement {
                                origin: meshes
                                    .as_ref()
                                    .map_or(Point { x: 0., y: 0. }, |m| m.origin),
                                transform,
                                viewport,
                                bounds,
                                clips: &item.clips,
                            };
                            let Drawing::Shape(_, paint) = item.drawing else {
                                match content
                                    .as_ref()
                                    .unwrap()
                                    .borrow_mut()
                                    .paint(&item.drawing, &placement, &mut budget, window, cx)
                                    .unwrap()
                                {
                                    Status::Ready => {}
                                    Status::Loading => pending.set(true),
                                    Status::Deferred => {
                                        pending.set(true);
                                        deferred = true;
                                    }
                                }
                                continue;
                            };
                            let meshes = meshes.as_ref().unwrap();
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
                    if let (Some(content), Some(count)) = (&content, pressure) {
                        let mut content = content.borrow_mut();
                        let placement = Placement {
                            origin: Point { x: 0., y: 0. },
                            transform: Transform {
                                a: pressure_scale,
                                d: pressure_scale,
                                ..Transform::IDENTITY
                            },
                            viewport,
                            bounds,
                            clips: &[],
                        };
                        for index in 0..count {
                            let drawing = Drawing::Text(
                                ResourceKey {
                                    id: 2,
                                    generation: 1,
                                },
                                pressure_origin,
                                ((index as i64) << 8) | 255,
                            );
                            match content.paint(&drawing, &placement, &mut budget, window, cx) {
                                Ok(Status::Ready) => {}
                                Ok(Status::Deferred) => {
                                    pending.set(true);
                                    deferred = true;
                                }
                                Ok(Status::Loading) => panic!("text cannot await an image worker"),
                                Err(error) => {
                                    pressure_error.set(Some(error));
                                    break;
                                }
                            }
                        }
                    }
                    if let Some(content) = &content {
                        content.borrow_mut().end_frame();
                    }
                    if deferred && pressure_error.get().is_none() {
                        window.refresh();
                    }
                    if pressure.is_some() && shaping_frames.borrow().len() < 64 {
                        let shapes_after = content.as_ref().unwrap().borrow().statistics().2;
                        shaping_frames
                            .borrow_mut()
                            .push((shapes_after - shapes_before, deferred));
                    }
                    vertices.set(budget.used_vertices());
                },
            )
            .size_full(),
        )
    }
}
fn scene(color: i64, assets: SceneAssets) -> Scene {
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
            interaction: Some(Interaction {
                label: "Green ellipse".into(),
                hit_region: HitRegion::Ellipse(Rect {
                    x: 110.,
                    y: 20.,
                    width: 60.,
                    height: 60.,
                }),
                draggable: true,
                activatable: true,
            }),
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
    items.extend([
        Item {
            id: 5,
            transform: Transform::IDENTITY,
            clips: vec![Rect {
                x: 20.,
                y: 155.,
                width: 40.,
                height: 25.,
            }],
            interaction: None,
            drawing: Drawing::Text(
                ResourceKey {
                    id: 2,
                    generation: 1,
                },
                Point { x: 20., y: 155. },
                0xffffffff,
            ),
        },
        Item {
            id: 6,
            transform: Transform::IDENTITY,
            clips: vec![Rect {
                x: 170.,
                y: 160.,
                width: 16.,
                height: 12.,
            }],
            interaction: None,
            drawing: Drawing::Image(
                ResourceKey {
                    id: 3,
                    generation: 1,
                },
                Rect {
                    x: 160.,
                    y: 155.,
                    width: 32.,
                    height: 24.,
                },
            ),
        },
        Item {
            id: 7,
            transform: Transform::IDENTITY,
            clips: vec![],
            interaction: None,
            drawing: Drawing::Image(
                ResourceKey {
                    id: 4,
                    generation: 1,
                },
                Rect {
                    x: 90.,
                    y: 155.,
                    width: 60.,
                    height: 24.,
                },
            ),
        },
    ]);
    Scene {
        version: 1,
        description: "Canvas GPU fixture".into(),
        resources: vec![
            Resource {
                key: ResourceKey {
                    id: 1,
                    generation: 1,
                },
                data: ResourceData::Path(path),
            },
            Resource {
                key: ResourceKey {
                    id: 2,
                    generation: 1,
                },
                data: ResourceData::Text(Text {
                    value: "Canvas".into(),
                    font_family: "system".into(),
                    font_size: 18.,
                    font_weight: 600,
                }),
            },
            Resource {
                key: ResourceKey {
                    id: 3,
                    generation: 1,
                },
                data: ResourceData::Image(assets.raster),
            },
            Resource {
                key: ResourceKey {
                    id: 4,
                    generation: 1,
                },
                data: ResourceData::Image(assets.svg),
            },
        ],
        items,
    }
}
fn publish(
    store: &mut canvas_store::Store,
    id: gpuio_protocol::ResourceId,
    base: i64,
    color: i64,
    assets: SceneAssets,
    asset_store: &asset_store::Store,
) -> Arc<canvas_store::Snapshot> {
    let mut bytes = vec![];
    scene(color, assets).binprot_write(&mut bytes).unwrap();
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
    store.publish(id, base + 1, asset_store).unwrap();
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
                    && !view.pending.get()
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
            let viewport = view.state.as_ref().unwrap().viewport();
            for &(world_x, world_y, expected) in samples {
                let x = ((f64::from(f32::from(bounds.origin.x))
                    + (world_x - viewport.origin.x) * viewport.zoom)
                    * scale) as u32;
                let y = ((f64::from(f32::from(bounds.origin.y))
                    + (world_y - viewport.origin.y) * viewport.zoom)
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
    mut assets: asset_store::Store,
    scene_assets: SceneAssets,
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
    pixels(
        cx,
        handle,
        &[
            (176., 167., [255, 0, 255, 255]),
            (165., 167., [0, 0, 255, 255]),
            (105., 167., [255, 128, 0, 255]),
            (135., 167., [0, 128, 128, 255]),
        ],
    );
    text_pixels(cx, handle);
    assets.release(scene_assets.raster).unwrap();
    assets.release(scene_assets.svg).unwrap();
    assert!(assets.acquire(scene_assets.raster).is_err());
    assert!(assets.acquire(scene_assets.svg).is_err());
    // Drive the native state directly, not OS pointer dispatch. Pixel readback
    // verifies that painting and hit testing use the same preview/override.
    handle
        .update(cx, |view, _, cx| {
            let state = view.state.as_mut().unwrap();
            assert_eq!(
                state.begin_pointer(Point { x: 140., y: 50. }, PointerMode::Select),
                vec![Observation::SelectionChanged(Some(3))]
            );
            assert!(state.move_pointer(Point { x: 200., y: 50. }));
            assert_eq!(state.hit_test(Point { x: 200., y: 50. }), Some(3));
            cx.notify();
        })
        .unwrap();
    pixels(
        cx,
        handle,
        &[(140., 50., [0, 0, 255, 255]), (200., 50., [0, 255, 0, 255])],
    );
    handle
        .update(cx, |view, _, cx| {
            assert!(view.state.as_mut().unwrap().cancel());
            cx.notify();
        })
        .unwrap();
    pixels(
        cx,
        handle,
        &[
            (140., 50., [0, 255, 0, 255]),
            (200., 50., [16, 16, 16, 255]),
        ],
    );
    handle
        .update(cx, |view, _, cx| {
            let state = view.state.as_mut().unwrap();
            state.begin_pointer(Point { x: 140., y: 50. }, PointerMode::Select);
            state.move_pointer(Point { x: 160., y: 50. });
            assert_eq!(
                state.finish_pointer(),
                vec![Observation::Moved(
                    3,
                    Transform {
                        tx: 20.,
                        ..Transform::IDENTITY
                    }
                )]
            );
            cx.notify();
        })
        .unwrap();
    pixels(
        cx,
        handle,
        &[(120., 50., [0, 0, 255, 255]), (160., 50., [0, 255, 0, 255])],
    );
    let snapshot = publish(&mut store, id, 1, 0x00ffffff, scene_assets, &assets);
    let viewport = Viewport {
        origin: Point { x: 20., y: 20. },
        zoom: 1.5,
    };
    let quality = handle
        .update(cx, |view, window, cx| {
            view.state.as_mut().unwrap().command(&Command {
                sequence: 1,
                action: Action::SetViewport(viewport),
            });
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
            (120., 50., [0, 255, 255, 255]),
            (160., 50., [0, 255, 0, 255]),
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
    ready(cx, handle, 2, quality).await;
    pixels(
        cx,
        handle,
        &[
            (176., 167., [255, 0, 255, 255]),
            (165., 167., [0, 255, 255, 255]),
            (105., 167., [255, 128, 0, 255]),
            (135., 167., [0, 128, 128, 255]),
        ],
    );
    text_pixels(cx, handle);
    let before = handle
        .update(cx, |view, _, cx| {
            let stats = view.content.as_ref().unwrap().borrow().statistics();
            assert_eq!(
                stats,
                (2, 2, 2, 3),
                "stable frames reuse text; zoom adds one SVG size"
            );
            view.pressure = Some(40);
            cx.notify();
            stats.2
        })
        .unwrap();
    ready(cx, handle, 2, quality).await;
    handle
        .update(cx, |view, _, cx| {
            assert_eq!(
                view.content.as_ref().unwrap().borrow().statistics().2,
                before + 40
            );
            let frames = view.shaping_frames.borrow();
            assert!(
                frames
                    .iter()
                    .any(|&(calls, deferred)| calls == 32 && deferred),
                "observed the first deferred frame: {frames:?}"
            );
            assert!(frames.iter().all(|&(calls, _)| calls <= 32));
            drop(frames);
            view.pressure = Some(513);
            cx.notify();
        })
        .unwrap();
    draw(cx, handle);
    handle.update(cx, |view, _, cx| {
        assert_eq!(view.pressure_error.get(), Some(gpuio_protocol::canvas_view::Error::RenderLimit));
        let stats = view.content.as_ref().unwrap().borrow().statistics();
        assert!(stats.0 <= canvas_content::MAX_TEXT_VARIANTS);
        assert!(view.shaping_frames.borrow().iter().all(|&(calls, _)| calls <= 32));
        view.pressure = None;
        cx.notify();
        eprintln!("GPUIO_CANVAS_CONTENT_METRICS: {stats:?}; deferred work and typed variant limit pass");
    }).unwrap();
    let prior_calls = handle
        .update(cx, |view, _, cx| {
            view.pressure = Some(513);
            view.pressure_origin = Point {
                x: 10_000.,
                y: 155.,
            };
            view.pressure_scale = 2.;
            cx.notify();
            view.content.as_ref().unwrap().borrow().statistics().2
        })
        .unwrap();
    ready(cx, handle, 2, quality).await;
    draw(cx, handle);
    handle.update(cx, |view, _, cx| {
        assert_eq!(view.pressure_error.get(), None, "offscreen variants do not consume visible-line admission");
        assert_eq!(view.content.as_ref().unwrap().borrow().statistics().2, prior_calls + 1,
            "one cold offscreen measurement, then culling without shaped-line retention or repeated shaping");
        view.pressure = None;
        cx.notify();
    }).unwrap();
    // Discard mounted state, then shut down queued native work. A native scene
    // snapshot can outlive registration, but cannot retain prepared mesh quota.
    handle
        .update(cx, |view, window, cx| {
            view.ready = None;
            view.job = None;
            view.state = None;
            view.content = None;
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
    image_host::shutdown(cx).await;
    cx.update(|cx| {
        assert_eq!(canvas_content::retained_text_bytes(cx), 0);
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
    assert_eq!(
        assets.stats().reserved_bytes,
        0,
        "all retired image readers released"
    );
    eprintln!(
        "GPUIO_NATIVE_CANVAS_OK: hidden-window GPU shape/curve/text/raster/SVG pixels, retired image leases, world clipping, pan/zoom, resize, drag preview/cancel/retention through direct state calls, native publication, worker/scene disposal; no OS input/AX acceptance claim"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    gpui_platform::application().run(move |cx: &mut App| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let mut store = canvas_store::Store::default();
        let id = store.create().unwrap();
        let mut assets = asset_store::Store::default();
        let scene_assets = scene_assets(&mut assets);
        let snapshot = publish(&mut store, id, 0, 0x0000ffff, scene_assets, &assets);
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
                    let (mut state, _) = State::new(
                        Config {
                            source: Some(id),
                            label: "GPU canvas".into(),
                            initial_viewport: viewport,
                            minimum_zoom: 0.05,
                            maximum_zoom: 64.,
                            selectable: true,
                            draggable: true,
                            pan_zoom: true,
                            disabled: false,
                            selection_color: 0xffffffff,
                            command: None,
                        },
                        snapshot.clone(),
                    )
                    .unwrap();
                    state.set_input_enabled(true);
                    let content = Content::new(snapshot.clone(), cx);
                    let job = canvas_host::request(request(snapshot, viewport, window), window, cx)
                        .unwrap();
                    cx.new(|_| View {
                        job: Some(job),
                        ready: None,
                        state: Some(state),
                        content: Some(Rc::new(RefCell::new(content))),
                        pending: Rc::new(Cell::new(true)),
                        pressure: None,
                        pressure_origin: Point { x: 20., y: 155. },
                        pressure_scale: 1.,
                        shaping_frames: Rc::new(RefCell::new(Vec::new())),
                        pressure_error: Rc::new(Cell::new(None)),
                        bounds: Rc::new(Cell::new(Bounds::default())),
                        vertices: Rc::new(Cell::new(0)),
                    })
                },
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(exercise(
                cx,
                handle,
                store,
                id,
                assets,
                scene_assets,
            ))
            .await;
            let _ = handle.update(cx, |view, window, _| {
                view.ready = None;
                view.job = None;
                view.state = None;
                view.content = None;
                window.remove_window();
            });
            canvas_host::shutdown(cx).await;
            image_host::shutdown(cx).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
