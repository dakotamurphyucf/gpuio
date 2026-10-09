//! Actual retained-overlay GPU pixels; interval expectations are independent of
//! the renderer's span resolver. No source publications occur during styling.
use super::*;
use gpuio_protocol::chart_inspection::{Axis, Pattern, Span};

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    transport: &Transport,
) {
    let cases = [
        (Span::Full, (0., 1.), true),
        (Span::Pixels(-20., 60.), (0., 40.), false),
        (Span::Pixels(24., 120.), (24., 144.), false),
        (Span::Fraction(0.25, 0.5), (0.25, 0.75), true),
        (Span::Fraction(0.75, 0.5), (0.75, 1.), true),
        (Span::Pixels(0., 0.), (0., 0.), false),
    ];
    let mut checked = 0;
    for size in [200., 160.] {
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                id(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(size)),
                    Field::Height(Length::Px(size)),
                ])],
            )],
        );
        for axis in [Axis::Vertical, Axis::Horizontal, Axis::Both] {
            for pattern in [Pattern::Solid, Pattern::Dashed] {
                for (span, (start, end), fractional) in cases {
                    let mut config = config(source, 0xff0000ff);
                    config.style.inspection.card.visible = false;
                    config.style.inspection.marker.visible = false;
                    let cross = &mut config.style.inspection.crosshair;
                    cross.axis = axis;
                    cross.pattern = pattern;
                    cross.thickness = 4.;
                    cross.color = Some(0xff00ffff);
                    cross.vertical_span = span;
                    // Both tests deliberately use different units/intervals.
                    cross.horizontal_span = if axis == Axis::Both {
                        Span::Pixels(12., 36.)
                    } else {
                        span
                    };
                    apply(cx, handle, vec![Op::SetChart(id(1), Box::new(config))]);
                    ready(cx, handle, 1, 0xff0000ff).await;
                    move_mouse(cx, handle, position(-10., -10.), false);
                    handle
                        .update(cx, |view, window, cx| {
                            window.focus(&view.charts[&id(1)].borrow().input.focus, cx);
                        })
                        .unwrap();
                    key(cx, handle, "home");
                    key(cx, handle, "enter");
                    draw(cx, handle);
                    let events = observations(transport);
                    assert!(events.iter().all(|v| *v == Some(Selection::Slice(7))));
                    handle.update(cx, |view, window, _| {
                        let state = view.charts[&id(1)].borrow();
                        let ready = state.ready.as_ref().unwrap();
                        assert_eq!(ready.snapshot.revision(), 1);
                        assert_eq!(state.input.selected, Some(Selection::Slice(7)));
                        let frame = state.ready_frame.unwrap();
                        let plot = frame.plot;
                        let details = crate::chart_details::describe_with_radar_labels(
                            ready.snapshot.data(), &ready.config.sampling,
                            &ready.config.options, ready.plan.geometry(),
                            state.input.selected_index.unwrap(), &[],
                        ).unwrap();
                        let mut rectangles = Vec::new();
                        if axis != Axis::Horizontal {
                            let factor = if fractional { plot.height } else { 1. };
                            let a = start * factor;
                            let b = (end * factor).min(plot.height);
                            let x = (details.anchor.x - 2.).clamp(0., plot.width - 4.);
                            if b > a { rectangles.push((plot.x + x, plot.y + a, 4., b - a)); }
                        }
                        if axis != Axis::Vertical {
                            let (a,b) = if axis == Axis::Both { (12., 48.) } else {
                                let factor = if fractional { plot.width } else { 1. };
                                (start * factor, (end * factor).min(plot.width))
                            };
                            let y = (details.anchor.y - 2.).clamp(0., plot.height - 4.);
                            if b > a { rectangles.push((plot.x + a, plot.y + y, b - a, 4.)); }
                        }
                        let image = window.render_to_image().unwrap();
                        let scale = f64::from(window.scale_factor());
                        let mut colored = 0;
                        let mut expected_interior = 0;
                        let mut painted_interior = 0;
                        for (x,y,pixel) in image.enumerate_pixels() {
                            let x = (f64::from(x) + 0.5) / scale;
                            let y = (f64::from(y) + 0.5) / scale;
                            let magenta = pixel.0 == [255,0,255,255];
                            let inside = rectangles.iter().any(|&(l,t,w,h)| x > l + 1. && x < l+w-1. && y > t+1. && y < t+h-1.);
                            if inside { expected_interior += 1; painted_interior += usize::from(magenta); }
                            if magenta {
                                colored += 1;
                                assert!(rectangles.iter().any(|&(l,t,w,h)| x >= l-0.5 && x <= l+w+0.5 && y >= t-0.5 && y <= t+h+0.5), "guide escaped bounds: {axis:?} {pattern:?} {span:?} at {x},{y}; {rectangles:?}");
                            }
                        }
                        if rectangles.is_empty() { assert_eq!(colored, 0); }
                        else {
                            assert!(colored > 8, "missing guide {axis:?} {pattern:?} {span:?}");
                            if pattern == Pattern::Solid {
                                assert!(expected_interior > 0 && painted_interior == expected_interior, "guide interior: {painted_interior}/{expected_interior} {axis:?} {span:?}");
                            }
                        }
                    }).unwrap();
                    checked += 1;
                }
            }
        }
    }
    key(cx, handle, "escape");
    observations(transport);
    apply(cx, handle, mount_size());
    eprintln!(
        "GPUIO_CHART_GUIDE_SPANS_OK: {checked} retained GPU cases; axes/solid/dashed/resize/empty/clipping; source revision and original selection unchanged"
    );
}

fn mount_size() -> Vec<Op> {
    vec![Op::SetStyle(
        id(1),
        vec![Style::Fields(vec![
            Field::Width(Length::Px(200.)),
            Field::Height(Length::Px(200.)),
        ])],
    )]
}
