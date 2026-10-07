//! Actual GPU readback of transformed SVG masks. Synthetic density overrides do
//! not claim monitor migration, input acceptance or physical presentation timing.
use super::*;
use gpuio_protocol::icon_transform::Transform;

fn root() -> NodeId {
    NodeId::from_parts(0, 2).unwrap()
}
fn icon() -> NodeId {
    NodeId::from_parts(1, 2).unwrap()
}
fn clip() -> NodeId {
    NodeId::from_parts(2, 2).unwrap()
}
fn appearance(color: u32, alpha: f64, radius: f64, origin: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(origin)),
        Field::Top(Length::Px(if origin == 0. { 0. } else { 32. })),
        Field::Width(Length::Px(32.)),
        Field::Height(Length::Px(32.)),
        Field::Foreground(Color::Rgba(color.into())),
        Field::Opacity(alpha),
        Field::TopLeftRadius(radius),
    ])]
}
async fn settled(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    corners: asset_svg::ClipRadii,
) {
    // Permit the accepted transaction to paint and issue its worker request.
    pause(cx).await;
    for _ in 0..200 {
        let ready = window
            .update(cx, |view, window, _| {
                let binding = view.images[&icon()].binding.borrow();
                let bounds = view.probes.borrow()[&icon()].bounds;
                let width = (f32::from(bounds.size.width) * window.scale_factor()).ceil() as u32;
                let height = (f32::from(bounds.size.height) * window.scale_factor()).ceil() as u32;
                let fit = view
                    .session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .get(icon())
                    .unwrap()
                    .image
                    .as_ref()
                    .unwrap()
                    .fit;
                binding.mask
                    && binding.pending.is_none()
                    && binding.resize_error.is_none()
                    && binding.requested == Some(binding.rendered)
                    && binding.rendered.size
                        == asset_svg::Size::Exact(
                            asset_svg::RasterSize::new(width, height).unwrap(),
                        )
                    && binding.rendered.fit == fit
                    && binding.rendered.corners == corners
                    && binding.rendered.tint.is_none()
            })
            .unwrap();
        if ready {
            pause(cx).await;
            return;
        }
        pause(cx).await;
    }
    panic!("transformed icon did not reach measured mask state");
}
fn snapshot(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
) -> (gpui::ImageId, asset_svg::Request) {
    window
        .update(cx, |view, window, cx| {
            let binding = view.images[&icon()].binding.borrow();
            let image = image_host::image_mask(binding.current.as_ref().unwrap(), window, cx)
                .unwrap()
                .unwrap();
            // Atlas membership introspection exists only on TestPlatform. On
            // the real Metal backend, pixels below establish mask rendering.
            (image.id, binding.rendered)
        })
        .unwrap()
}
fn pixels(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, samples: &[(f32, f32, [u8; 3])]) {
    window
        .update(cx, |_, window, _| {
            let scale = window.scale_factor();
            let image = window.render_to_image().unwrap();
            for &(x, y, expected) in samples {
                let actual = image
                    .get_pixel((x * scale).floor() as u32, (y * scale).floor() as u32)
                    .0;
                assert!(
                    actual[..3]
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(b) <= 2),
                    "density {scale}, point {x},{y}: {actual:?} != {expected:?}"
                );
            }
        })
        .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    let bytes=br#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect x="2" y="4" width="8" height="6" fill="white"/><rect x="18" y="22" width="10" height="6" fill="white" fill-opacity="0.5"/></svg>"#;
    let source = {
        let mut s = session.borrow_mut();
        let store = s.assets().unwrap();
        let id = store.begin(Format::Svg, bytes.len()).unwrap();
        store.append(id, 0, bytes).unwrap();
        store.finish(id).unwrap();
        id
    };
    apply(
        cx,
        window,
        vec![
            Op::Create(root(), Kind::Container, "".into(), None),
            Op::SetStyle(
                root(),
                vec![Style::Fields(vec![
                    Field::Width(Length::Percent(100.)),
                    Field::Height(Length::Percent(100.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                ])],
            ),
            Op::Create(icon(), Kind::Icon, "".into(), None),
            Op::SetImage(
                icon(),
                ImageConfig {
                    source: ImageSource::Reference(source),
                    fit: ImageFit::Fill,
                    label: Some("Transformed sample".into()),
                },
            ),
            Op::SetStyle(icon(), appearance(0x00ff00ff, 1., 0., 48.)),
            Op::Splice(root(), 0, 0, vec![icon()]),
            Op::SetRoot(Some(root())),
        ],
    );
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(source)
        .unwrap();
    let original_scale = window.update(cx, |_, w, _| w.scale_factor()).unwrap();
    let cases = [
        (Transform::default(), (54., 39.), (71., 57.), (73., 38.)),
        (
            Transform {
                rotation_degrees: 90.,
                ..Default::default()
            },
            (73., 38.),
            (55., 55.),
            (54., 39.),
        ),
        (
            Transform {
                translate_x: 20.,
                translate_y: -8.,
                ..Default::default()
            },
            (74., 31.),
            (91., 49.),
            (54., 39.),
        ),
        (
            Transform {
                scale_x: -1.,
                ..Default::default()
            },
            (74., 39.),
            (57., 57.),
            (54., 39.),
        ),
        (
            Transform {
                scale_x: 2.,
                scale_y: 0.5,
                ..Default::default()
            },
            (44., 43.5),
            (78., 52.5),
            (54., 39.),
        ),
        (
            Transform {
                scale_x: -0.5,
                scale_y: 2.,
                rotation_degrees: 90.,
                translate_x: 5.,
                translate_y: 3.,
            },
            (87., 56.),
            (51., 47.5),
            (54., 39.),
        ),
    ];
    for scale in [1., 1.5, 2.] {
        eprintln!("GPUIO_ICON_TRANSFORM_PHASE: density {scale}");
        window
            .update(cx, |_, w, _| w.set_scale_factor(scale))
            .unwrap();
        apply(cx, window, vec![Op::SetIconTransform(icon(), None)]);
        settled(cx, window, Default::default()).await;
        let before = snapshot(cx, window);
        let bounds = window
            .update(cx, |v, _, _| v.probes.borrow()[&icon()].bounds)
            .unwrap();
        for (transform, a, b, negative) in cases {
            eprintln!("GPUIO_ICON_TRANSFORM_PHASE: {transform:?}");
            apply(
                cx,
                window,
                vec![Op::SetIconTransform(icon(), Some(transform))],
            );
            settled(cx, window, Default::default()).await;
            pixels(
                cx,
                window,
                &[
                    (a.0, a.1, [0, 255, 0]),
                    (b.0, b.1, [0, 128, 0]),
                    (negative.0, negative.1, [0, 0, 0]),
                ],
            );
            assert_eq!(
                snapshot(cx, window),
                before,
                "transform must reuse decoded pixels/request"
            );
            assert_eq!(
                window
                    .update(cx, |v, _, _| v.probes.borrow()[&icon()].bounds)
                    .unwrap(),
                bounds,
                "visual transform must preserve assigned layout"
            );
        }
        for transform in [
            Transform {
                scale_x: 0.,
                ..Default::default()
            },
            Transform {
                scale_y: 0.,
                ..Default::default()
            },
        ] {
            eprintln!("GPUIO_ICON_TRANSFORM_PHASE: collapsed {transform:?}");
            apply(
                cx,
                window,
                vec![Op::SetIconTransform(icon(), Some(transform))],
            );
            settled(cx, window, Default::default()).await;
            pixels(cx, window, &[(54., 39., [0, 0, 0]), (71., 57., [0, 0, 0])]);
            assert_eq!(snapshot(cx, window), before);
        }
        apply(
            cx,
            window,
            vec![
                Op::SetIconTransform(icon(), None),
                Op::SetStyle(icon(), appearance(0xff00ff80, 0.5, 0., 48.)),
            ],
        );
        settled(cx, window, Default::default()).await;
        pixels(
            cx,
            window,
            &[(54., 39., [64, 0, 64]), (71., 57., [32, 0, 32])],
        );
        assert_eq!(
            snapshot(cx, window),
            before,
            "foreground/opacity must not rasterize SVG"
        );
        apply(
            cx,
            window,
            vec![
                Op::SetStyle(icon(), appearance(0x00ff00ff, 1., 16., 48.)),
                Op::SetIconTransform(
                    icon(),
                    Some(Transform {
                        rotation_degrees: 90.,
                        ..Default::default()
                    }),
                ),
            ],
        );
        settled(
            cx,
            window,
            asset_svg::ClipRadii::new([16. * scale, 0., 0., 0.]).unwrap(),
        )
        .await;
        pixels(
            cx,
            window,
            &[(73., 38., [0, 255, 0]), (75.5, 34.5, [0, 0, 0])],
        );
        apply(
            cx,
            window,
            vec![
                Op::SetStyle(icon(), appearance(0x00ff00ff, 1., 0., 48.)),
                Op::SetIconTransform(icon(), None),
            ],
        );
        settled(cx, window, Default::default()).await;
        pixels(
            cx,
            window,
            &[(54., 39., [0, 255, 0]), (71., 57., [0, 128, 0])],
        );
    }
    // Different fitting policies operate in the original 48 x 32 viewport,
    // before rotation about (72, 48). Coordinates are derived from the two SVG
    // rectangles, not from the renderer's matrix or fit helpers.
    for scale in [1., 1.5, 2.] {
        window
            .update(cx, |_, w, _| w.set_scale_factor(scale))
            .unwrap();
        for (fit, a, b) in [
            (ImageFit::Fill, (81., 33.), (63., 58.5)),
            (ImageFit::Contain, (81., 38.), (63., 55.)),
            (ImageFit::ScaleDown, (81., 38.), (63., 55.)),
            (ImageFit::Cover, (85.5, 33.), (58.5, 58.5)),
            (ImageFit::None, (81., 30.), (63., 47.)),
        ] {
            eprintln!("GPUIO_ICON_TRANSFORM_PHASE: fit {fit:?} density {scale}");
            let mut style = appearance(0x00ff00ff, 1., 0., 48.);
            style.push(Style::Fields(vec![Field::Width(Length::Px(48.))]));
            apply(
                cx,
                window,
                vec![
                    Op::SetStyle(icon(), style),
                    Op::SetImage(
                        icon(),
                        ImageConfig {
                            source: ImageSource::Reference(source),
                            fit,
                            label: Some("Transformed sample".into()),
                        },
                    ),
                    Op::SetIconTransform(
                        icon(),
                        Some(Transform {
                            rotation_degrees: 90.,
                            ..Default::default()
                        }),
                    ),
                ],
            );
            settled(cx, window, Default::default()).await;
            pixels(
                cx,
                window,
                &[
                    (a.0, a.1, [0, 255, 0]),
                    (b.0, b.1, [0, 128, 0]),
                    (72., 48., [0, 0, 0]),
                ],
            );
        }
    }
    // Parent clipping acts after the visual transform, independently from the
    // icon mask's own pre-transform fitting and corner clip.
    window
        .update(cx, |_, w, _| w.set_scale_factor(original_scale))
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::SetImage(
                icon(),
                ImageConfig {
                    source: ImageSource::Reference(source),
                    fit: ImageFit::Fill,
                    label: Some("Transformed sample".into()),
                },
            ),
            Op::Create(clip(), Kind::Container, "".into(), None),
            Op::SetStyle(
                clip(),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(48.)),
                    Field::Top(Length::Px(32.)),
                    Field::Width(Length::Px(32.)),
                    Field::Height(Length::Px(32.)),
                    Field::OverflowX(1),
                    Field::OverflowY(1),
                ])],
            ),
            Op::Splice(root(), 0, 1, vec![]),
            Op::Splice(clip(), 0, 0, vec![icon()]),
            Op::Splice(root(), 0, 0, vec![clip()]),
            Op::SetStyle(icon(), appearance(0x00ff00ff, 1., 0., 0.)),
            Op::SetIconTransform(
                icon(),
                Some(Transform {
                    translate_x: 20.,
                    translate_y: -8.,
                    ..Default::default()
                }),
            ),
        ],
    );
    settled(cx, window, Default::default()).await;
    pixels(
        cx,
        window,
        &[
            (74., 31., [0, 0, 0]),
            (91., 49., [0, 0, 0]),
            (74., 33., [0, 255, 0]),
        ],
    );
    assert!(
        session
            .borrow_mut()
            .assets()
            .unwrap()
            .acquire(source)
            .is_err(),
        "all resampling uses retired mounted lease"
    );
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Splice(root(), 0, 1, vec![]),
            Op::Splice(clip(), 0, 1, vec![]),
            Op::Remove(icon()),
            Op::Remove(clip()),
            Op::Remove(root()),
        ],
    );
    assert!(window.update(cx, |v, _, _| v.images.is_empty()).unwrap());
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
    eprintln!(
        "GPUIO_ICON_TRANSFORM_GPU_OK: SRT/reset/zero/reflection, source/foreground/element alpha, rounded and ancestor clipping, stable layout/cache and retired source cleanup at three test densities"
    );
}
