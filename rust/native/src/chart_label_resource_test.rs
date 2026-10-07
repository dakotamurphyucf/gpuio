//! Hidden-window resource pixels and independent cardinal/diagonal layout checks.
use super::*;
use gpuio_protocol::chart_options::RadarRadius;

fn resource_config(source: ResourceId, shown: bool) -> Config {
    let mut config = content_config(source, shown);
    config.options.radar.radius = RadarRadius::Pixels(40.);
    config.options.radar.label_gap = 12.;
    config
}

fn color_pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, rgba: [u8; 4]) -> usize {
    draw(cx, handle);
    handle
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .filter(|p| p.0 == rgba)
                .count()
        })
        .unwrap()
}

async fn icon_pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, rgba: [u8; 4]) {
    for _ in 0..200 {
        if color_pixels(cx, handle, rgba) > 100 {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("radar SVG icon never painted {rgba:?}");
}

fn publish_data(
    cx: &mut gpui::AsyncApp,
    source: ResourceId,
    session: &SharedSession,
    data: &Data,
) -> i64 {
    let previous = session.borrow().chart(source).unwrap().snapshot().unwrap();
    stage_data(
        session,
        source,
        previous.revision(),
        previous.generation(),
        data,
    );
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, previous.revision() + 1))
    else {
        panic!("radar resource publication")
    };
    let result = work.run();
    cx.update(|cx| {
        assert_eq!(session.borrow_mut().complete_chart(result), Response::Ack);
        crate::host::chart_source_changed(Some(source), cx);
    });
    previous.revision() + 1
}

fn placement(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, index: usize, width: f32) {
    draw(cx, handle);
    handle
        .update(cx, |view, window, _| {
            let probes = view.probes.borrow();
            let actual = probes[&id(7)].bounds;
            assert_eq!(actual.size, gpui::size(px(width), px(16.)));
            let state = view.charts[&id(1)].borrow();
            let plot = state.ready_frame.unwrap().plot;
            let angle = index as f64 * std::f64::consts::FRAC_PI_4;
            let direction = (angle.sin(), -angle.cos());
            // The chart is the first child of the unpadded root. ChartView has
            // its own renderer, so the ordinary-node probes cover its parent.
            let chart = probes[&id(0)].bounds;
            let expected_x = f64::from(f32::from(chart.origin.x))
                + plot.x
                + plot.width / 2.
                + direction.0 * 52.
                + (direction.0 - 1.) * f64::from(width) / 2.;
            let expected_y = f64::from(f32::from(chart.origin.y))
                + plot.y
                + plot.height / 2.
                + direction.1 * 52.
                + (direction.1 - 1.) * 16. / 2.;
            let tolerance = 1. / f64::from(window.scale_factor()) + 0.01;
            assert!(
                (f64::from(f32::from(actual.origin.x)) - expected_x).abs() <= tolerance,
                "axis {index}: x {:?}, expected {expected_x}",
                actual
            );
            assert!(
                (f64::from(f32::from(actual.origin.y)) - expected_y).abs() <= tolerance,
                "axis {index}: y {:?}, expected {expected_y}",
                actual
            );
        })
        .unwrap();
}

async fn density_ready(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, scale: f32) {
    for _ in 0..200 {
        draw(cx, handle);
        if handle
            .update(cx, |view, _, _| {
                let state = view.charts[&id(1)].borrow();
                state.ready.as_ref().is_some_and(|ready| {
                    ready.layout.dimensions().2 == f64::from(scale)
                        && state.requested.as_ref().is_some_and(|request| {
                            ready.layout == request.layout
                                && ready.snapshot.revision() == request.snapshot.revision()
                        })
                })
            })
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("radar plan did not settle at test density {scale}");
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
) {
    let original = session
        .borrow()
        .chart(source)
        .unwrap()
        .snapshot()
        .unwrap()
        .data()
        .clone();
    let baseline = session.borrow_mut().assets().unwrap().stats();
    let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="16"><rect width="24" height="16" fill="red"/></svg>"#;
    let image = {
        let mut session = session.borrow_mut();
        let assets = session.assets().unwrap();
        let image = assets
            .begin(gpuio_protocol::asset::Format::Svg, bytes.len())
            .unwrap();
        assets.append(image, 0, bytes).unwrap();
        assets.finish(image).unwrap();
        image
    };
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(resource_config(source, true))),
            Op::Create(id(7), Kind::Icon, "".into(), None),
            Op::SetImage(
                id(7),
                ImageConfig {
                    source: ImageSource::Reference(image),
                    fit: ImageFit::Fill,
                    label: Some("Radar resource label".into()),
                },
            ),
            Op::SetStyle(
                id(2),
                vec![Style::Fields(vec![Field::Foreground(Color::Rgba(
                    0x00cc44ff,
                ))])],
            ),
            Op::Splice(id(2), 0, 1, vec![id(7)]),
            Op::Splice(id(0), 1, 0, vec![id(3)]),
            Op::SetStyle(id(3), vec![Style::Fields(vec![Field::Display(3)])]),
        ],
    );
    // The mounted child must own its lease even before the first decode/paint.
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(image)
        .unwrap();
    assert!(
        session
            .borrow_mut()
            .assets()
            .unwrap()
            .acquire(image)
            .is_err()
    );
    icon_pixels(cx, handle, [0, 204, 68, 255]).await;
    let scale = handle
        .update(cx, |_, window, _| window.scale_factor())
        .unwrap();
    for index in 0..8 {
        let mut axes: Vec<_> = (0..8)
            .map(|i| RadarAxis {
                id: 7 + i,
                label: format!("Axis {i}"),
                maximum: 100.,
            })
            .collect();
        axes.rotate_right(index);
        let data = Data {
            version: 2,
            bar_backgrounds: vec![],
            contents: Contents::Radar(axes, vec![]),
        };
        let revision = publish_data(cx, source, session, &data);
        ready(cx, handle, revision, 0xff0000ff).await;
        for density in [1., 1.5, 2., scale] {
            handle
                .update(cx, |_, window, _| window.set_scale_factor(density))
                .unwrap();
            // This is a test override, not a physical monitor transition.
            density_ready(cx, handle, density).await;
            placement(cx, handle, index, 24.);
        }
        icon_pixels(cx, handle, [0, 204, 68, 255]).await;
    }
    // Child-only size/foreground changes must not rebuild the chart polygons.
    let before = cx.update(|cx| renderer::measurements(cx).unwrap().0);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                id(2),
                vec![Style::Fields(vec![Field::Foreground(Color::Rgba(
                    0x6633ccff,
                ))])],
            ),
            Op::SetStyle(
                id(7),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(36.)),
                    Field::Height(Length::Px(16.)),
                ])],
            ),
        ],
    );
    icon_pixels(cx, handle, [102, 51, 204, 255]).await;
    placement(cx, handle, 7, 36.);
    assert_eq!(
        cx.update(|cx| renderer::measurements(cx).unwrap().0),
        before,
        "label-only changes must not prepare another chart plan"
    );
    apply(
        cx,
        handle,
        vec![Op::SetChart(
            id(1),
            Box::new(resource_config(source, false)),
        )],
    );
    assert_eq!(color_pixels(cx, handle, [102, 51, 204, 255]), 0);
    handle
        .update(cx, |view, _, _| {
            assert!(!view.focus.borrow().allows(id(7)));
            assert!(
                view.images.contains_key(&id(7)),
                "hidden resource stays mounted"
            );
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(resource_config(source, true)))],
    );
    icon_pixels(cx, handle, [102, 51, 204, 255]).await;
    assert_eq!(
        session.borrow_mut().assets().unwrap().stats().retired,
        baseline.retired + 1
    );
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(2), 0, 1, vec![id(3)]),
            Op::Splice(id(0), 1, 1, vec![]),
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::FontSize(12.),
                    Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
                ])],
            ),
            Op::Remove(id(7)),
            Op::SetStyle(id(2), vec![]),
            Op::SetChart(id(1), Box::new(content_config(source, true))),
        ],
    );
    let revision = publish_data(cx, source, session, &original);
    ready(cx, handle, revision, 0xff0000ff).await;
    inherited_font(cx, handle);
    for _ in 0..200 {
        if session.borrow_mut().assets().unwrap().stats() == baseline {
            break;
        }
        draw(cx, handle);
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert_eq!(
        session.borrow_mut().assets().unwrap().stats(),
        baseline,
        "unmount releases the retired encoded SVG lease"
    );
    handle
        .update(cx, |view, _, _| assert!(!view.images.contains_key(&id(7))))
        .unwrap();
    eprintln!(
        "GPUIO_RADAR_RESOURCE_OK: intrinsic SVG pixels, inherited tint/size/font, retired source retention, hide/return, eight anchors at test densities, label-only plan reuse and unmount cleanup"
    );
}

fn inherited_font(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let before = cx.update(|cx| renderer::measurements(cx).unwrap().0);
    apply(
        cx,
        handle,
        vec![
            Op::SetText(id(3), "Axis".into()),
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![Field::Background(Fill::Solid(
                    Color::Rgba(0x00ff00ff),
                ))])],
            ),
            Op::SetStyle(id(2), vec![Style::Fields(vec![Field::FontSize(12.)])]),
        ],
    );
    assert!(green_pixels(cx, handle) > 20);
    let small = handle
        .update(cx, |view, _, _| view.probes.borrow()[&id(3)].bounds.size)
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(2),
            vec![Style::Fields(vec![Field::FontSize(20.)])],
        )],
    );
    assert!(green_pixels(cx, handle) > 20);
    let large = handle
        .update(cx, |view, _, _| view.probes.borrow()[&id(3)].bounds.size)
        .unwrap();
    assert!(
        large.width > small.width && large.height > small.height,
        "inherited font must remeasure ordinary label text: {small:?} -> {large:?}"
    );
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(2),
            vec![Style::Fields(vec![Field::FontSize(12.)])],
        )],
    );
    draw(cx, handle);
    assert_eq!(
        handle
            .update(cx, |view, _, _| view.probes.borrow()[&id(3)].bounds.size)
            .unwrap(),
        small
    );
    assert_eq!(
        cx.update(|cx| renderer::measurements(cx).unwrap().0),
        before,
        "inherited label font does not rebuild the chart plan"
    );
    apply(cx, handle, vec![Op::SetStyle(id(2), vec![])]);
}
