//! Production pie caption text and leader pixels in the hidden chart window.
use super::*;
use crate::chart_geometry::LabelKind;
use gpuio_protocol::{
    chart_options::{LabelPlacement, SliceRadii},
    chart_pie_labels::Entry,
};

async fn resize_backing(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    extent: gpui::Size<gpui::Pixels>,
) {
    handle
        .update(cx, |_, window, _| window.resize(extent))
        .unwrap();
    for _ in 0..100 {
        draw(cx, handle);
        if handle
            .update(cx, |_, window, _| window.viewport_size() == extent)
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("pie test backing resize was not acknowledged: {extent:?}");
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    let snapshot = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: Contents::Pie(vec![
            Slice {
                id: 7,
                label: "raw seven".into(),
                value: 1.,
            },
            Slice {
                id: 3,
                label: "raw three".into(),
                value: 1.,
            },
        ]),
    };
    stage_data(
        session,
        source,
        snapshot.revision(),
        snapshot.generation(),
        &data,
    );
    let revision = snapshot.revision() + 1;
    cx.update(|cx| dispatch(cx, transport, 80, Request::Publish(source, revision)));
    let (original_scale, original_size) = handle
        .update(cx, |_, window, _| {
            (window.scale_factor(), window.viewport_size())
        })
        .unwrap();
    // The test-only scale override changes scene coordinates, not AppKit's
    // CAMetalLayer drawable size. Reserve a 480px backing even on a 1x screen
    // before drawing the fixed 240px root at synthetic densities up to 2x.
    // Otherwise a valid caption at logical x=149 can lie outside a 240px image.
    resize_backing(cx, handle, gpui::size(px(480.), px(480.))).await;
    for density in [1., 1.25, 1.5, 2.] {
        handle
            .update(cx, |_, window, _| window.set_scale_factor(density))
            .unwrap();
        for (placement, labels, hidden, width, weight, muted) in [
            (LabelPlacement::Outside, true, false, 200., 400, false),
            (LabelPlacement::Outside, true, false, 200., 700, false),
            (LabelPlacement::Outside, true, true, 120., 400, false),
            (LabelPlacement::Inside, true, false, 200., 400, false),
            (LabelPlacement::Outside, false, false, 200., 400, false),
            (LabelPlacement::Outside, true, false, 200., 400, true),
        ] {
            let mut config = config(source, 0x444444ff);
            config.options.pie.labels = labels;
            config.options.pie.label_placement = placement;
            config.options.pie.slice_radii = vec![SliceRadii {
                slice: 7,
                inner: 10.,
                outer: 30.,
            }];
            config.style.label_color = 0x00ff00ff;
            config.style.pie_label_line_color = Some(if muted { 0x444444ff } else { 0x0000ffff });
            config.style.pie_labels = vec![
                Entry {
                    slice: 7,
                    text: Some(if hidden { "" } else { "RIGHT" }.into()),
                    line_color: Some(if muted { 0x444444ff } else { 0xff0000ff }),
                },
                Entry {
                    slice: 3,
                    text: Some("LEFT".into()),
                    line_color: None,
                },
            ];
            apply(
                cx,
                handle,
                vec![
                    Op::SetChart(id(1), Box::new(config.clone())),
                    Op::SetStyle(
                        id(1),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(width)),
                            Field::Height(Length::Px(200.)),
                            Field::FontWeight(weight),
                        ])],
                    ),
                ],
            );
            let mut complete = false;
            for _ in 0..300 {
                draw(cx, handle);
                complete = handle
                    .update(cx, |view, _, _| {
                        let state = view.charts[&id(1)].borrow();
                        state.ready.as_ref().is_some_and(|r| {
                            r.snapshot.revision() == revision
                                && r.config.as_ref() == &config
                                && state.ready_frame.is_some_and(|f| f.width == width)
                                && (placement != LabelPlacement::Outside
                                    || !labels
                                    || r.label_style.as_ref().is_some_and(|s| {
                                        s.font.weight == gpui::FontWeight(weight as f32)
                                    }))
                        })
                    })
                    .unwrap();
                if complete {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
            }
            assert!(complete, "pie label request failed to settle");
            draw(cx, handle);
            handle
            .update(cx, |view, window, _| {
                let state = view.charts[&id(1)].borrow();
                let ready = state.ready.as_ref().unwrap();
                let frame = state.ready_frame.unwrap();
                let image = window.render_to_image().unwrap();
                let scale = f64::from(window.scale_factor());
                assert_eq!(scale, f64::from(density), "synthetic scale retained");
                assert!(image.width() >= 480 && image.height() >= 480,
                    "synthetic density requires a complete backing: actual={}x{}, scale={scale}",
                    image.width(), image.height());
                let geometry = ready.plan.geometry();
                assert_eq!(
                    geometry.labels.len(),
                    if !labels {
                        0
                    } else if hidden {
                        1
                    } else {
                        2
                    }
                );
                let count_color = |left: f64, top: f64, right: f64, bottom: f64, channel: usize| {
                    let mut count = 0;
                    for y in (top * scale).max(0.).floor() as u32
                        ..(bottom * scale).ceil().min(f64::from(image.height())) as u32
                    {
                        for x in (left * scale).max(0.).floor() as u32
                            ..(right * scale).ceil().min(f64::from(image.width())) as u32
                        {
                            let c = image.get_pixel(x, y).0;
                            if c[channel] > 150
                                && (0..3)
                                    .filter(|i| *i != channel)
                                    .all(|i| c[channel].saturating_sub(c[i]) > 80)
                            {
                                count += 1;
                            }
                        }
                    }
                    count
                };
                for label in &geometry.labels {
                    let rect = frame.label(label).rect;
                    let green = count_color(rect.x, rect.y, rect.x + rect.width, rect.y + rect.height, 1);
                    if green <= 4 {
                        if let Some(path) = std::env::var_os("GPUIO_PIE_LABEL_DIAGNOSTIC")
                            && let Err(error) = image.save(path) {
                            eprintln!("Could not save pie diagnostic: {error}");
                        }
                        panic!("missing green pie caption {:?} at {rect:?}; green={green}, scale={scale}, image={}x{}",
                            label.text, image.width(), image.height());
                    }
                    if let LabelKind::Pie {
                        slice_index,
                        placement: Some(p),
                    } = label.kind
                    {
                        // This two-half fixture has horizontal leaders, leaving a clean
                        // sample between gray wedge edge and green text.
                        let x = frame.plot.x + (p.edge.x + p.bend.x) / 2.;
                        let y = frame.plot.y + (p.edge.y + p.bend.y) / 2.;
                        let channel = if slice_index == 0 { 0 } else { 2 };
                        // A one-logical-pixel stroke can be split by rasterization:
                        // at scale 1 a blue sample is [8,8,136], not opaque blue.
                        // Require the right hue across multiple physical columns,
                        // and reject both other hues in the same isolated region.
                        // Gray wedge/background and green caption pixels cannot pass.
                        let mut columns = [0_usize; 3];
                        for xx in ((x - 2.) * scale).floor().max(0.) as u32
                            ..((x + 2.) * scale).ceil().min(f64::from(image.width())) as u32 {
                            let mut found = [false; 3];
                            for yy in ((y - 2.) * scale).floor().max(0.) as u32
                                ..((y + 2.) * scale).ceil().min(f64::from(image.height())) as u32 {
                                let pixel = image.get_pixel(xx, yy).0;
                                for candidate in 0..3 {
                                    found[candidate] |= (0..3).filter(|c| *c != candidate)
                                        .all(|c| pixel[candidate].saturating_sub(pixel[c]) > 80);
                                }
                            }
                            for (count, found) in columns.iter_mut().zip(found) {
                                *count += usize::from(found);
                            }
                        }
                        let minimum = (2. * scale).ceil() as usize;
                        let valid = if muted {
                            // Actual neutral leaders are the negative control: neither
                            // gray geometry/background nor green text may satisfy hue.
                            columns == [0; 3]
                        } else {
                            columns[channel] >= minimum
                                && (0..3).filter(|c| *c != channel).all(|c| columns[c] == 0)
                        };
                        eprintln!("PIE_LEADER_COVERAGE scale={scale} slice={slice_index} width={width} weight={weight} muted={muted} channel={channel} columns={columns:?} minimum={minimum}");
                        if !valid {
                            eprintln!("PIE_LEADER_FAILURE placement={p:?} plot={:?} sample={x},{y}", frame.plot);
                            if let Some(path) = std::env::var_os("GPUIO_PIE_LABEL_DIAGNOSTIC")
                                && let Err(error) = image.save(path) {
                                eprintln!("Could not save pie diagnostic: {error}");
                            }
                        }
                        assert!(valid, "missing or wrong independently colored pie leader for {slice_index}");
                    }
                }
                assert_eq!(ready.snapshot.data(), &data);
            })
            .unwrap();
            transport.mailbox.lock().unwrap().drain(128);
        }
    }
    handle
        .update(cx, |_, window, _| window.set_scale_factor(original_scale))
        .unwrap();
    resize_backing(cx, handle, original_size).await;
    eprintln!(
        "GPUIO_PIE_LABEL_VIEW_OK: four synthetic scales (1/1.25/1.5/2), measured text pixels, ID captions, per-slice/default leader colors and neutral negative controls, variable radius, hidden captions, inside/outside, narrow/font changes and unchanged data"
    );
}
