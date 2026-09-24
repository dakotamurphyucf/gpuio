//! Production SVG canvas: measured native raster size, inherited foreground,
//! synthetic GPUI hover dispatch, and disposal of weak paint references.
use super::*;
fn config(source: ImageSource) -> ImageConfig {
    ImageConfig {
        fit: ImageFit::Fill,
        ..super::config(source)
    }
}
fn vector_source(session: &Rc<RefCell<Session>>) -> ResourceId {
    let bytes=br#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="2" height="4" fill="red"/><rect x="2" width="2" height="4" fill="blue"/></svg>"#;
    let mut session = session.borrow_mut();
    let store = session.assets().unwrap();
    let id = store.begin(Format::Svg, bytes.len()).unwrap();
    store.append(id, 0, bytes).unwrap();
    store.finish(id).unwrap();
    id
}
fn style(color: u32) -> Vec<Style> {
    vec![
        Style::Fields(vec![
            Field::Width(Length::Percent(100.)),
            Field::Height(Length::Percent(100.)),
            Field::Foreground(Color::Rgba(color as i64)),
        ]),
        Style::State(2, vec![Field::Foreground(Color::Rgba(0xff8800ff))]),
    ]
}
async fn rasterized(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    id: NodeId,
    tint: Option<u32>,
) {
    for _ in 0..200 {
        let done = window
            .update(cx, |view, window, _| {
                let state = view.images[&id].binding.borrow();
                let width =
                    (f32::from(window.viewport_size().width) * window.scale_factor()).ceil() as u32;
                let height = (f32::from(window.viewport_size().height) * window.scale_factor())
                    .ceil() as u32;
                state.pending.is_none()
                    && state.resize_error.is_none()
                    && state.rendered.size
                        == asset_svg::Size::Exact(
                            asset_svg::RasterSize::new(width, height).unwrap(),
                        )
                    && state.rendered.tint == tint
                    && state.rendered.fit == ImageFit::Fill
            })
            .unwrap();
        if done {
            pause(cx).await;
            pause(cx).await;
            return;
        }
        pause(cx).await;
    }
    let details = window
        .update(cx, |view, window, _| {
            let state = view.images[&id].binding.borrow();
            (
                state.rendered,
                state.requested,
                state.resize_error,
                window.viewport_size(),
                window.mouse_position(),
            )
        })
        .unwrap();
    panic!("SVG raster did not settle for tint {tint:?}: {details:?}");
}
fn halves(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, left: [u8; 4], right: [u8; 4]) {
    window
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            assert_eq!(
                image.get_pixel(image.width() / 4, image.height() / 2).0,
                left
            );
            assert_eq!(
                image.get_pixel(image.width() * 3 / 4, image.height() / 2).0,
                right
            );
        })
        .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    let id = NodeId::from_parts(1, 1).unwrap();
    let source = vector_source(session);
    apply(
        cx,
        window,
        vec![
            Op::Create(id, Kind::Image, "".into(), None),
            Op::SetImage(id, config(ImageSource::Reference(source))),
            Op::SetStyle(id, style(0x00cc44ff)),
            Op::SetRoot(Some(id)),
        ],
    );
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(source)
        .unwrap();
    rasterized(cx, window, id, None).await;
    halves(cx, window, [255, 0, 0, 255], [0, 0, 255, 255]);
    let revision = session.borrow().tree(window_id()).unwrap().revision();
    window
        .update(cx, |_, window, _| window.resize(size(px(144.), px(96.))))
        .unwrap();
    let mut resized = false;
    for _ in 0..200 {
        resized = window
            .update(cx, |_, window, _| {
                window.viewport_size() == size(px(144.), px(96.))
            })
            .unwrap();
        if resized {
            break;
        }
        pause(cx).await;
    }
    assert!(resized, "native viewport did not receive the resize");
    rasterized(cx, window, id, None).await;
    assert_eq!(
        session.borrow().tree(window_id()).unwrap().revision(),
        revision,
        "native resize must not require a tree transaction"
    );
    halves(cx, window, [255, 0, 0, 255], [0, 0, 255, 255]);
    assert!(
        session
            .borrow_mut()
            .assets()
            .unwrap()
            .acquire(source)
            .is_err(),
        "resampling used the mounted lease after retirement"
    );
    // GPUI's test override exercises host resampling without pretending that this
    // moved a physical window between monitors. Restore before pixel readback.
    let original_scale = window
        .update(cx, |_, window, _| window.scale_factor())
        .unwrap();
    for scale in [1., 1.5, 2., original_scale] {
        window
            .update(cx, |_, window, _| window.set_scale_factor(scale))
            .unwrap();
        rasterized(cx, window, id, None).await;
        window
            .update(cx, |view, window, _| {
                assert_eq!(window.viewport_size(), size(px(144.), px(96.)));
                let binding = view.images[&id].binding.borrow();
                assert_eq!(
                    binding.rendered.density,
                    asset_svg::Density::new(scale).unwrap()
                );
                assert_eq!(
                    view.session.borrow().tree(view.id).unwrap().revision(),
                    revision
                );
            })
            .unwrap();
    }
    let icon = NodeId::from_parts(2, 1).unwrap();
    let source = vector_source(session);
    crate::host::native_test::move_mouse(cx, window, gpui::point(px(-10.), px(-10.)), false);
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(id),
            Op::Create(icon, Kind::Icon, "".into(), None),
            Op::SetImage(icon, config(ImageSource::Reference(source))),
            Op::SetStyle(icon, style(0x00cc44ff)),
            Op::SetRoot(Some(icon)),
        ],
    );
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(source)
        .unwrap();
    rasterized(cx, window, icon, Some(0x00cc44ff)).await;
    halves(cx, window, [0, 204, 68, 255], [0, 204, 68, 255]);
    crate::host::native_test::move_mouse(cx, window, gpui::point(px(20.), px(20.)), false);
    rasterized(cx, window, icon, Some(0xff8800ff)).await;
    halves(cx, window, [255, 136, 0, 255], [255, 136, 0, 255]);
    crate::host::native_test::move_mouse(cx, window, gpui::point(px(-10.), px(-10.)), false);
    rasterized(cx, window, icon, Some(0x00cc44ff)).await;
    apply(cx, window, vec![Op::SetStyle(icon, style(0x6633ccff))]);
    rasterized(cx, window, icon, Some(0x6633ccff)).await;
    halves(cx, window, [102, 51, 204, 255], [102, 51, 204, 255]);
    let mut rounded = style(0x6633ccff);
    rounded.push(Style::Fields(vec![Field::TopLeftRadius(40.)]));
    rounded.push(Style::State(
        2,
        vec![Field::TopLeftRadius(0.), Field::TopRightRadius(40.)],
    ));
    apply(cx, window, vec![Op::SetStyle(icon, rounded)]);
    pause(cx).await;
    pause(cx).await;
    rounded_pixels(cx, window, [102, 51, 204, 255]);
    crate::host::native_test::move_mouse(cx, window, gpui::point(px(20.), px(20.)), false);
    rasterized(cx, window, icon, Some(0xff8800ff)).await;
    window
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            assert_eq!(
                image.get_pixel(4, 4).0,
                [255, 136, 0, 255],
                "hover clears top-left rounding"
            );
            assert_ne!(
                image.get_pixel(image.width() - 5, 4).0,
                [255, 136, 0, 255],
                "hover rounds top-right pixels"
            );
        })
        .unwrap();
    let weak = window
        .update(cx, |view, _, _| Rc::downgrade(&view.images[&icon].binding))
        .unwrap();
    apply(cx, window, vec![Op::SetRoot(None), Op::Remove(icon)]);
    assert!(
        weak.upgrade().is_none(),
        "stale canvas closures must not retain mounted sources"
    );
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
    eprintln!(
        "GPUIO_NATIVE_SVG_VIEWS_OK: native density/resize, full color, retired-source resampling, foreground/hover tint and weak disposal"
    );
}
