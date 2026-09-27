//! Actual GPU/AX fallback and shared image ownership/resize regressions.
use super::*;
use gpuio_protocol::avatar::Config;

fn id() -> NodeId {
    NodeId::from_parts(11, 1).unwrap()
}
fn callback() -> HandlerId {
    HandlerId::from_parts(11, 1).unwrap()
}
fn avatar(source: Option<ImageSource>) -> Config {
    Config {
        source,
        fit: ImageFit::Cover,
        label: Some("Dakota avatar".into()),
        fallback: "DM".into(),
    }
}
fn style(width: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(64.)),
        Field::Foreground(Color::Rgba(0xffffffff)),
        Field::Background(Fill::Solid(Color::Rgba(0x223344ff))),
        Field::FontSize(24.),
    ])]
}
async fn settled(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, expected: ImageState) {
    for _ in 0..200 {
        let found = window
            .update(cx, |view, _, _| {
                view.images
                    .get(&id())
                    .and_then(|s| s.emitted.map(|(_, s)| s))
            })
            .unwrap();
        if found == Some(expected) {
            pause(cx).await;
            pause(cx).await;
            return;
        }
        pause(cx).await;
    }
    let state = window
        .update(cx, |view, _, _| {
            view.images.get(&id()).map(|s| {
                (
                    s.emitted,
                    s.binding.borrow().layout_error,
                    s.binding.borrow().resize_error,
                )
            })
        })
        .unwrap();
    panic!("avatar did not reach {expected:?}: {state:?}");
}
fn sample(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, expected: Option<[u8; 4]>) {
    window
        .update(cx, |view, window, _| {
            let bounds = view.probes.borrow()[&id()].bounds;
            let scale = window.scale_factor();
            let image = window.render_to_image().unwrap();
            let x = (f32::from(bounds.origin.x) * scale) as u32;
            let y = (f32::from(bounds.origin.y) * scale) as u32;
            let w = (f32::from(bounds.size.width) * scale) as u32;
            let h = (f32::from(bounds.size.height) * scale) as u32;
            let mut white = 0;
            for dy in h / 4..h * 3 / 4 {
                for dx in w / 4..w * 3 / 4 {
                    let pixel = image.get_pixel(x + dx, y + dy).0;
                    if let Some(expected) = expected {
                        assert_eq!(pixel, expected, "ready avatar interior");
                    }
                    if pixel[0] > 220 && pixel[1] > 220 && pixel[2] > 220 {
                        white += 1;
                    }
                }
            }
            if let Some(expected) = expected {
                assert_ne!(
                    image.get_pixel(x + 1, y + 1).0,
                    expected,
                    "default circular clipping"
                );
            } else {
                assert!(
                    white > 8,
                    "fallback must paint actual centered glyphs: {white}"
                );
            }
        })
        .unwrap();
}
async fn rasterized(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    for _ in 0..200 {
        let done = window
            .update(cx, |view, window, _| {
                let binding = view.images[&id()].binding.borrow();
                let physical = (64. * window.scale_factor()).ceil() as u32;
                binding.pending.is_none()
                    && binding.layout_error.is_none()
                    && binding.resize_error.is_none()
                    && binding.rendered.size
                        == asset_svg::Size::Exact(
                            asset_svg::RasterSize::new(physical, physical).unwrap(),
                        )
                    && binding.rendered.density
                        == asset_svg::Density::new(window.scale_factor()).unwrap()
            })
            .unwrap();
        if done {
            return;
        }
        pause(cx).await;
    }
    panic!("avatar raster did not match measured logical size and density");
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    apply(
        cx,
        window,
        vec![
            Op::Create(id(), Kind::Avatar, "".into(), None),
            Op::SetAvatar(id(), avatar(None)),
            Op::SetRoot(Some(id())),
        ],
    );
    pause(cx).await;
    pause(cx).await;
    window
        .update(cx, |view, _, _| {
            assert!(
                !view.images.contains_key(&id()),
                "no source needs no image owner"
            );
            assert_eq!(
                view.probes.borrow()[&id()].bounds.size,
                size(px(32.), px(32.))
            );
        })
        .unwrap();
    apply(cx, window, vec![Op::SetStyle(id(), style(64.))]);
    pause(cx).await;
    pause(cx).await;
    sample(cx, window, None);
    #[cfg(target_os = "macos")]
    {
        let _ = crate::host::control_test::accessible_role(cx, window, "Dakota avatar");
        pause(cx).await;
        assert_eq!(
            crate::host::control_test::accessible_role(cx, window, "Dakota avatar").as_deref(),
            Some("AXImage")
        );
        assert!(
            crate::host::control_test::accessible_role(cx, window, "DM").is_none(),
            "painted fallback is not a second semantic child"
        );
    }
    apply(
        cx,
        window,
        vec![Op::SetAvatar(
            id(),
            Config {
                label: None,
                ..avatar(None)
            },
        )],
    );
    pause(cx).await;
    pause(cx).await;
    #[cfg(target_os = "macos")]
    assert!(crate::host::control_test::accessible_role(cx, window, "Dakota avatar").is_none());

    let green = source(&mut session.borrow_mut(), [10, 220, 30]);
    apply(
        cx,
        window,
        vec![
            Op::SetAvatar(id(), avatar(Some(ImageSource::Reference(green)))),
            Op::Bind(id(), Some(callback())),
        ],
    );
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(green)
        .unwrap();
    let ready = ImageState::Ready(ImageMetadata {
        width_px: 4,
        height_px: 4,
        frames: 1,
    });
    settled(cx, window, ready).await;
    sample(cx, window, Some([10, 220, 30, 255]));
    let weak = window
        .update(cx, |view, _, _| Rc::downgrade(&view.images[&id()].binding))
        .unwrap();
    let bad = upload(&mut session.borrow_mut(), b"not a PNM image");
    apply(
        cx,
        window,
        vec![Op::SetAvatar(
            id(),
            avatar(Some(ImageSource::Reference(bad))),
        )],
    );
    session.borrow_mut().assets().unwrap().release(bad).unwrap();
    settled(cx, window, ImageState::Failed(ImageError::InvalidData)).await;
    sample(cx, window, None);
    assert!(
        weak.upgrade().is_none(),
        "replacement drops the prior mounted owner"
    );

    let bytes=br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="#0adc1e"/></svg>"##;
    let vector = {
        let mut session = session.borrow_mut();
        let store = session.assets().unwrap();
        let source = store.begin(Format::Svg, bytes.len()).unwrap();
        store.append(source, 0, bytes).unwrap();
        store.finish(source).unwrap();
        source
    };
    apply(
        cx,
        window,
        vec![Op::SetAvatar(
            id(),
            avatar(Some(ImageSource::Reference(vector))),
        )],
    );
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(vector)
        .unwrap();
    settled(cx, window, ready).await;
    sample(cx, window, Some([10, 220, 30, 255]));
    // The invalid measured size must not poison returning to the same request.
    rasterized(cx, window).await;
    apply(cx, window, vec![Op::SetStyle(id(), style(20000.))]);
    settled(cx, window, ImageState::Failed(ImageError::ResourceLimit)).await;
    apply(cx, window, vec![Op::SetStyle(id(), style(64.))]);
    settled(cx, window, ready).await;
    rasterized(cx, window).await;
    sample(cx, window, Some([10, 220, 30, 255]));
    let statuses: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::ImageState(_, node, _, _, status) if node == id() => Some(status),
            _ => None,
        })
        .collect();
    assert!(
        statuses
            .windows(2)
            .any(|pair| pair == [ImageState::Failed(ImageError::ResourceLimit), ready]),
        "deferred bridge must report failure then recovery: {statuses:?}"
    );
    let original_scale = window
        .update(cx, |_, window, _| window.scale_factor())
        .unwrap();
    let revision = session.borrow().tree(window_id()).unwrap().revision();
    // Synthetic GPUI density changes; this does not claim physical monitor moves.
    for scale in [1., 1.5, 2., original_scale] {
        window
            .update(cx, |_, window, _| window.set_scale_factor(scale))
            .unwrap();
        rasterized(cx, window).await;
        assert_eq!(
            session.borrow().tree(window_id()).unwrap().revision(),
            revision
        );
    }
    let weak = window
        .update(cx, |view, _, _| Rc::downgrade(&view.images[&id()].binding))
        .unwrap();
    transport.mailbox.lock().unwrap().drain(128);
    apply(
        cx,
        window,
        vec![Op::SetAvatar(id(), avatar(None)), Op::Bind(id(), None)],
    );
    pause(cx).await;
    pause(cx).await;
    sample(cx, window, None);
    assert!(weak.upgrade().is_none());
    let renders = window.update(cx, |view, _, _| view.render_count).unwrap();
    for _ in 0..6 {
        pause(cx).await;
    }
    assert_eq!(
        window.update(cx, |view, _, _| view.render_count).unwrap(),
        renders,
        "static fallback must become idle"
    );
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event,Event::ImageState(_,node,..) if *node==id())),
        "source removal rejects obsolete observations"
    );
    apply(cx, window, vec![Op::SetRoot(None), Op::Remove(id())]);
    pause(cx).await;
    pause(cx).await;
    window
        .update(cx, |view, _, _| {
            assert!(!view.images.contains_key(&id()));
            assert_eq!(view.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    eprintln!(
        "GPUIO_AVATAR_NATIVE_OK: GPU fallback, AX, source leases, SVG recovery/density, idle and disposal"
    );
}
