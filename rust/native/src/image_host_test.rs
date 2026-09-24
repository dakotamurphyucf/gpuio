//! Real background decode, GPUI image paint/readback and atlas teardown.
use super::*;
use crate::asset_store::Store;
use gpui::{
    Bounds, Context, Render, WindowBounds, WindowHandle, WindowOptions, div, img, prelude::*, px,
    rgb, size,
};
use gpuio_protocol::asset::Format;
use std::time::Duration;

struct Scene {
    image: Option<Handle>,
    probe: Option<Arc<RenderImage>>,
    paints: usize,
}
impl Render for Scene {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.paints += 1;
        let image = self
            .image
            .as_ref()
            .and_then(|handle| image(handle, window, cx).unwrap());
        let probe = self.probe.clone().map(|probe| {
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    window
                        .paint_image(bounds, bounds, Default::default(), probe.clone(), 0, false)
                        .unwrap();
                },
            )
            .size_full()
        });
        div()
            .size_full()
            .bg(rgb(0x000000))
            .children(image.map(|image| img(image).size_full()))
            .children(probe)
    }
}
fn lease(store: &mut Store, color: [u8; 3]) -> Lease {
    let mut bytes = b"P6\n4 4\n255\n".to_vec();
    for _ in 0..16 {
        bytes.extend(color);
    }
    let id = store.begin(Format::Pnm, bytes.len()).unwrap();
    store.append(id, 0, &bytes).unwrap();
    store.finish(id).unwrap();
    store.acquire(id).unwrap()
}
async fn pause(cx: &mut AsyncApp) {
    cx.background_executor()
        .timer(Duration::from_millis(25))
        .await;
}
async fn ready(cx: &mut AsyncApp, handle: WindowHandle<Scene>) -> Arc<RenderImage> {
    for _ in 0..200 {
        let image = handle
            .update(cx, |scene, window, cx| {
                image(scene.image.as_ref().unwrap(), window, cx).unwrap()
            })
            .unwrap();
        if let Some(image) = image {
            pause(cx).await;
            pause(cx).await;
            return image;
        }
        pause(cx).await;
    }
    panic!("native image did not finish decoding");
}
fn pixels(cx: &mut AsyncApp, handle: WindowHandle<Scene>, expected: [u8; 4]) {
    handle
        .update(cx, |scene, window, _| {
            assert!(scene.paints >= 2, "completion refreshed the loading scene");
            let image = window.render_to_image().expect("native GPU readback");
            assert_eq!(
                image.get_pixel(image.width() / 2, image.height() / 2).0,
                expected
            );
        })
        .unwrap();
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        init(cx);
        let mut store = Store::default();
        let source = lease(&mut store, [255, 7, 32]);
        let first = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(96.), px(96.)), cx))),
            focus: false,
            ..Default::default()
        }, |window, cx| {
            let image = request(source.clone(), window, cx).unwrap();
            cx.new(|_| Scene { image: Some(image), probe: None, paints: 0 })
        }).unwrap();
        let shared_handle = first.update(cx, |scene, _, _| scene.image.as_ref().unwrap().clone()).unwrap();
        let second = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(96.), px(96.)), cx))),
            focus: false,
            ..Default::default()
        }, |window, cx| {
            assert!(image(&shared_handle, window, cx).unwrap().is_none());
            assert!(cx.global::<Global>().0.borrow().windows.contains_key(&window.window_handle().window_id()));
            let image = shared_handle;
            cx.new(|_| Scene { image: Some(image), probe: None, paints: 0 })
        }).unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(async {
            let red = ready(cx, first).await;
            let shared = ready(cx, second).await;
            assert!(Arc::ptr_eq(&red, &shared));
            pixels(cx, first, [255, 7, 32, 255]);
            pixels(cx, second, [255, 7, 32, 255]);
            cx.update(|cx| {
                let service = cx.global::<Global>().0.borrow();
                assert_eq!(service.atlas_entries, 2);
                assert_eq!(service.atlas_bytes, 128);
            });
            store.release(source.id()).unwrap();
            drop(source);
            // Existing mounts survive registration retirement.
            pixels(cx, second, [255, 7, 32, 255]);
            first.update(cx, |scene, window, cx| {
                scene.image = None;
                window.remove_window();
                cx.notify();
            }).unwrap();
            pause(cx).await;
            cx.update(|cx| assert_eq!(cx.global::<Global>().0.borrow().atlas_entries, 1));
            let blue = lease(&mut store, [11, 35, 240]);
            second.update(cx, |scene, window, cx| {
                scene.image = Some(request(blue, window, cx).unwrap());
                cx.notify();
            }).unwrap();
            let blue = ready(cx, second).await;
            pixels(cx, second, [11, 35, 240, 255]);
            assert_eq!(store.stats().retired, 0, "warm red pixels no longer hold the encoded lease");
            // The real Metal atlas does not implement PlatformAtlas::contains.
            // Use a diagnostic image with the SAME ID and different pixels:
            // before eviction its paint must reuse red; afterwards it must
            // upload new green pixels. Production code never reassigns image IDs.
            let mut probe = RenderImage::new(vec![::image::Frame::new(
                ::image::RgbaImage::from_pixel(4, 4, ::image::Rgba([2, 220, 19, 255])))]);
            probe.id = red.id;
            second.update(cx, |scene, _, cx| {
                scene.image = None;
                scene.probe = Some(Arc::new(probe));
                cx.notify();
            }).unwrap();
            pause(cx).await;
            pause(cx).await;
            pixels(cx, second, [255, 7, 32, 255]);
            // Admit two real host workers, then start shutdown before their
            // foreground completion delivery can run. Either CPU work or its
            // queued result is still charged at this exact boundary.
            let pending = second.update(cx, |_, window, cx| {
                vec![request(lease(&mut store, [4, 8, 12]), window, cx).unwrap(),
                     request(lease(&mut store, [12, 8, 4]), window, cx).unwrap()]
            }).unwrap();
            cx.update(|cx| {
                let service = cx.global::<Global>().0.clone();
                pump(&service, cx);
                assert_eq!(service.borrow_mut().cache.stats().running, 2);
            });
            shutdown(cx).await;
            second.update(cx, |_, window, cx| {
                for handle in &pending { assert!(matches!(image(handle, window, cx), Err(Error::Closed))); }
            }).unwrap();
            drop(pending);
            second.update(cx, |_, _, cx| cx.notify()).unwrap();
            pause(cx).await;
            pause(cx).await;
            pixels(cx, second, [19, 220, 2, 255]);
            cx.update(|cx| {
                let mut service = cx.global::<Global>().0.borrow_mut();
                assert_eq!(service.atlas_entries, 0);
                assert_eq!(service.atlas_bytes, 0);
                assert_eq!(service.atlas_frames, 0);
                assert_eq!(service.cache.stats().running, 0);
                assert!(service.workers.is_empty());
                assert!(service.delivering.is_none());
                assert!(service.input.is_empty());
            });
            second.update(cx, |scene, window, _| { scene.image = None; window.remove_window(); }).unwrap();
            drop((red, shared, blue));
            eprintln!("GPUIO_NATIVE_IMAGES_OK: background/shared decode, two-window GPU pixels, retired-source retention, atlas teardown and shutdown");
            }).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(crate::host::stop_application);
        }).detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
