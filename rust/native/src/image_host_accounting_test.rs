//! Production service accounting on TestPlatform; no GPU or desktop acceptance.
use super::*;
use crate::asset_store::Store;
use gpui::{Context, IntoElement, Render, TestAppContext, canvas, prelude::*, rgb};
use gpuio_protocol::asset::Format;

fn window(app: &mut TestAppContext) -> AnyWindowHandle {
    app.add_empty_window()
        .update(|window, _| window.window_handle())
}
fn source(store: &mut Store, animated: bool) -> Lease {
    let (format, data) = if animated {
        let mut data = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut data);
            for color in [[255, 0, 0, 255], [0, 255, 0, 255]] {
                encoder
                    .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                        2,
                        2,
                        image::Rgba(color),
                    )))
                    .unwrap();
            }
        }
        (Format::Gif, data)
    } else {
        (
            Format::Pnm,
            b"P6\n2 2\n255\n\xff\x00\x00\xff\x00\x00\xff\x00\x00\xff\x00\x00".to_vec(),
        )
    };
    let id = store.begin(format, data.len()).unwrap();
    store.append(id, 0, &data).unwrap();
    store.finish(id).unwrap();
    store.acquire(id).unwrap()
}
fn request_ready(app: &mut TestAppContext, window: AnyWindowHandle, source: Lease) -> Handle {
    let handle = window
        .update(app, |_, window, cx| request(source, window, cx).unwrap())
        .unwrap();
    app.run_until_parked();
    handle
}
fn observe(
    app: &mut TestAppContext,
    window: AnyWindowHandle,
    handle: &Handle,
    representation: Representation,
) -> Result<Option<Arc<RenderImage>>, Error> {
    window
        .update(app, |_, window, cx| match representation {
            Representation::Color => image(handle, window, cx),
            Representation::Mask => image_mask(handle, window, cx),
        })
        .unwrap()
}
fn service(app: &mut TestAppContext) -> Shared {
    app.update(|cx| cx.global::<Global>().0.clone())
}
fn usage(service: &Shared) -> (usize, usize, usize) {
    let state = service.borrow();
    (state.atlas_entries, state.atlas_frames, state.atlas_bytes)
}

struct Paint {
    pixels: Arc<RenderImage>,
    color: bool,
}
impl Render for Paint {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let pixels = self.pixels.clone();
        let color = self.color;
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                window
                    .paint_image_mask(
                        bounds,
                        pixels.clone(),
                        0,
                        gpui::TransformationMatrix::unit(),
                        rgb(0xffffff).into(),
                    )
                    .unwrap();
                if color {
                    window
                        .paint_image(bounds, bounds, Default::default(), pixels, 0, false)
                        .unwrap();
                }
            },
        )
        .size_full()
    }
}

#[test]
fn loading_masks_register_the_second_window_before_decode_completes() {
    let mut app = TestAppContext::single();
    let a = window(&mut app);
    let b = window(&mut app);
    let mut store = Store::default();
    let handle = a
        .update(&mut app, |_, window, cx| {
            let handle = request(source(&mut store, false), window, cx).unwrap();
            assert!(image_mask(&handle, window, cx).unwrap().is_none());
            b.update(cx, |_, window, cx| {
                assert!(image_mask(&handle, window, cx).unwrap().is_none());
                let state = cx.global::<Global>().0.borrow();
                assert!(state.windows.contains_key(&b.window_id()));
                assert_eq!(state.atlas_bytes, 0);
            })
            .unwrap();
            handle
        })
        .unwrap();
    app.run_until_parked();
    assert!(
        observe(&mut app, b, &handle, Representation::Mask)
            .unwrap()
            .is_some()
    );
    assert_eq!(usage(&service(&mut app)), (1, 1, 4));
    app.update(finish_before_quit);
}

#[test]
fn representations_share_decode_but_charge_each_window_and_all_frames() {
    for first in [Representation::Color, Representation::Mask] {
        let mut app = TestAppContext::single();
        let a = window(&mut app);
        let b = window(&mut app);
        let mut store = Store::default();
        let source = source(&mut store, true);
        let handle = request_ready(&mut app, a, source.clone());
        let service = service(&mut app);
        let pixels = observe(&mut app, a, &handle, first).unwrap().unwrap();
        assert_eq!(pixels.frame_count(), 2);
        let first_bytes = if first == Representation::Color {
            32
        } else {
            8
        };
        assert_eq!(usage(&service), (1, 2, first_bytes));
        for representation in [Representation::Mask, Representation::Color] {
            for _ in 0..3 {
                let shared = observe(&mut app, a, &handle, representation)
                    .unwrap()
                    .unwrap();
                assert!(Arc::ptr_eq(&pixels, &shared));
            }
        }
        assert_eq!(usage(&service), (2, 4, 40));
        for representation in [Representation::Color, Representation::Mask] {
            let shared = observe(&mut app, b, &handle, representation)
                .unwrap()
                .unwrap();
            assert!(Arc::ptr_eq(&pixels, &shared));
        }
        assert_eq!(usage(&service), (4, 8, 80));
        // Registration retirement does not invalidate mounted owners.
        store.release(source.id()).unwrap();
        drop(source);
        assert!(
            observe(&mut app, b, &handle, Representation::Mask)
                .unwrap()
                .is_some()
        );
        a.update(&mut app, |_, window, _| window.remove_window())
            .unwrap();
        app.run_until_parked();
        assert_eq!(usage(&service), (2, 4, 40));
        b.update(&mut app, |_, window, _| window.remove_window())
            .unwrap();
        app.run_until_parked();
        assert_eq!(usage(&service), (0, 0, 0));
        app.update(finish_before_quit);
    }
}

#[test]
fn failed_admission_is_atomic_and_existing_representation_remains_usable() {
    for requested in [Representation::Color, Representation::Mask] {
        let existing = match requested {
            Representation::Color => Representation::Mask,
            Representation::Mask => Representation::Color,
        };
        let mut app = TestAppContext::single();
        let window = window(&mut app);
        let mut store = Store::default();
        let handle = request_ready(&mut app, window, source(&mut store, true));
        let pixels = observe(&mut app, window, &handle, existing)
            .unwrap()
            .unwrap();
        let service = service(&mut app);
        let baseline = usage(&service);
        let required_bytes = requested.bytes(&pixels);
        // Simulate other windows consuming each global quota independently.
        // No huge allocations are necessary to test the production admission.
        for (entries, frames, bytes) in [
            (MAX_ATLAS_ENTRIES, baseline.1, baseline.2),
            (baseline.0, MAX_ATLAS_FRAMES - 1, baseline.2),
            (baseline.0, baseline.1, MAX_ATLAS_BYTES - required_bytes + 1),
        ] {
            {
                let mut state = service.borrow_mut();
                state.atlas_entries = entries;
                state.atlas_frames = frames;
                state.atlas_bytes = bytes;
            }
            assert!(matches!(
                observe(&mut app, window, &handle, requested),
                Err(Error::ResourceLimit)
            ));
            assert_eq!(usage(&service), (entries, frames, bytes));
            assert_eq!(
                service.borrow().windows[&window.window_id()].images.len(),
                1
            );
            assert!(Arc::ptr_eq(
                &pixels,
                &observe(&mut app, window, &handle, existing)
                    .unwrap()
                    .unwrap()
            ));
        }
        {
            let mut state = service.borrow_mut();
            state.atlas_entries = MAX_ATLAS_ENTRIES - 1;
            state.atlas_frames = MAX_ATLAS_FRAMES - 2;
            state.atlas_bytes = MAX_ATLAS_BYTES - required_bytes;
        }
        assert!(
            observe(&mut app, window, &handle, requested)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            usage(&service),
            (MAX_ATLAS_ENTRIES, MAX_ATLAS_FRAMES, MAX_ATLAS_BYTES)
        );
        // Restore the actual two reservations before exercising real teardown.
        {
            let mut state = service.borrow_mut();
            state.atlas_entries = 2;
            state.atlas_frames = 4;
            state.atlas_bytes = 40;
        }
        app.update(finish_before_quit);
        assert_eq!(usage(&service), (0, 0, 0));
    }
}

#[test]
fn warm_eviction_retires_both_representations_in_all_windows() {
    let mut app = TestAppContext::single();
    let a = window(&mut app);
    let b = window(&mut app);
    let mut store = Store::default();
    let handle = request_ready(&mut app, a, source(&mut store, false));
    let service = service(&mut app);
    let retained_pixels = observe(&mut app, a, &handle, Representation::Color)
        .unwrap()
        .unwrap();
    observe(&mut app, a, &handle, Representation::Mask)
        .unwrap()
        .unwrap();
    observe(&mut app, b, &handle, Representation::Mask)
        .unwrap()
        .unwrap();
    assert_eq!(usage(&service), (3, 3, 24));
    for (window, color) in [(a, true), (b, false)] {
        window
            .update(&mut app, |_, window, cx| {
                window.replace_root(cx, |_, _| Paint {
                    pixels: retained_pixels.clone(),
                    color,
                });
                window.draw(cx).clear(cx);
                assert!(window.has_image_mask_atlas_entry(&retained_pixels, 0));
                assert_eq!(window.has_image_atlas_entry(&retained_pixels), color);
                // Stop painting before eviction, leaving the atlas entries alive.
                window.replace_root(cx, |_, _| gpui::Empty);
                window.draw(cx).clear(cx);
            })
            .unwrap();
    }
    drop(handle);
    // Fill the real warm cache, forcing its least-recently-used decoded image
    // through the host eviction path while an external Arc still owns pixels.
    for _ in 0..asset_cache::MAX_ENTRIES {
        drop(request_ready(&mut app, a, source(&mut store, false)));
    }
    assert_eq!(usage(&service), (0, 0, 0));
    for window in [a, b] {
        window
            .update(&mut app, |_, window, _| {
                assert!(!window.has_image_atlas_entry(&retained_pixels));
                assert!(!window.has_image_mask_atlas_entry(&retained_pixels, 0));
            })
            .unwrap();
    }
    assert!(
        service
            .borrow()
            .windows
            .values()
            .all(|window| window.images.is_empty())
    );
    assert_eq!(retained_pixels.as_bytes(0).unwrap().len(), 16);
    assert_eq!(service.borrow_mut().cache.stats().retired, 1);
    drop(retained_pixels);
    assert_eq!(service.borrow_mut().cache.stats().retired, 0);
    app.update(finish_before_quit);
}

#[test]
fn shutdown_releases_reservations_and_rejects_both_representations_with_live_handles() {
    for asynchronous in [false, true] {
        let mut app = TestAppContext::single();
        let window = window(&mut app);
        let mut store = Store::default();
        let handle = request_ready(&mut app, window, source(&mut store, true));
        for representation in [Representation::Color, Representation::Mask] {
            observe(&mut app, window, &handle, representation)
                .unwrap()
                .unwrap();
        }
        let service = service(&mut app);
        if asynchronous {
            let complete = Rc::new(std::cell::Cell::new(false));
            let completed = complete.clone();
            app.spawn(async move |mut cx| {
                shutdown(&mut cx).await;
                completed.set(true);
            })
            .detach();
            app.run_until_parked();
            assert!(complete.get(), "shutdown future must finish");
        } else {
            app.update(finish_before_quit);
        }
        assert_eq!(usage(&service), (0, 0, 0));
        for representation in [Representation::Color, Representation::Mask] {
            assert!(matches!(
                observe(&mut app, window, &handle, representation),
                Err(Error::Closed)
            ));
        }
        assert_eq!(service.borrow_mut().cache.stats().running, 0);
        assert!(service.borrow().workers.is_empty());
        app.update(finish_before_quit);
        assert_eq!(usage(&service), (0, 0, 0));
    }
}
