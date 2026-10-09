use super::*;

fn style() -> Style {
    Style {
        envelope_width: 16.,
        track_width: 12.,
        thumb_width: 6.,
        inset: 2.,
        radius: 3.,
        min_length: 12.,
    }
}

#[test]
fn separate_viewport_corner_preserves_semantic_range_and_endpoints() {
    let mut m = metrics();
    m.viewport[1] = 16.;
    let bar = measure_reserved(m, Selection::Horizontal, [style(); 2], [16., 0.])
        .unwrap()
        .horizontal
        .unwrap();
    assert_eq!(bar.envelope.size, [184., 16.]);
    assert_eq!(bar.max_offset, 600.);
    assert_eq!(bar.drag_offset(1000., 0.), Some(600.));
    m.viewport[0] = 10.;
    let bar = measure_reserved(m, Selection::Horizontal, [style(); 2], [16., 0.])
        .unwrap()
        .horizontal
        .unwrap();
    assert_eq!(
        bar.envelope.size[0], 5.,
        "tiny viewports retain usable edge space"
    );
    assert_eq!(bar.max_offset, 790.);
    assert!(measure_reserved(m, Selection::Horizontal, [style(); 2], [f64::NAN, 0.]).is_err());
}
fn metrics() -> Metrics {
    Metrics {
        viewport: [200., 100.],
        content: [800., 400.],
        offset: [0., 0.],
    }
}
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
fn contained(inner: Rect, outer: Rect) {
    for i in 0..2 {
        assert!(inner.origin[i].is_finite());
        assert!(inner.size[i].is_finite() && inner.size[i] >= 0.);
        assert!(inner.origin[i] >= outer.origin[i] - 1e-8);
        assert!(inner.origin[i] + inner.size[i] <= outer.origin[i] + outer.size[i] + 1e-8);
    }
}

#[test]
fn proportional_thumb_endpoints_and_one_axis_corner() {
    let g = measure(metrics(), Selection::Both, [style(); 2]).unwrap();
    let h = g.horizontal.unwrap();
    let v = g.vertical.unwrap();
    assert_eq!(h.envelope.size, [184., 16.]);
    assert_eq!(v.envelope.size, [16., 84.]);
    assert_eq!(h.track.origin, [0., 88.]);
    assert_eq!(h.thumb.origin, [2., 92.]);
    assert_eq!(h.thumb.size, [45., 6.]);
    assert_eq!(v.thumb.origin, [192., 2.]);
    assert_eq!(v.thumb.size, [6., 20.]);
    assert_eq!(h.centered_offset(-100.), Some(0.));
    assert_eq!(h.centered_offset(1000.), Some(600.));
    let mut m = metrics();
    m.offset = [300., 300.];
    let g = measure(m, Selection::Both, [style(); 2]).unwrap();
    assert_eq!(g.horizontal.unwrap().thumb.origin[0], 69.5);
    assert_eq!(g.vertical.unwrap().thumb.origin[1], 62.);
    m.content[0] = m.viewport[0];
    let g = measure(m, Selection::Both, [style(); 2]).unwrap();
    assert!(g.horizontal.is_none());
    assert_eq!(g.vertical.unwrap().envelope.size[1], 100.);
    let g = measure(metrics(), Selection::Horizontal, [style(); 2]).unwrap();
    assert!(g.vertical.is_none());
    assert_eq!(g.horizontal.unwrap().envelope.size[0], 200.);
}

#[test]
fn captures_keep_pixel_grip_but_use_current_extent_and_offset() {
    let bar = measure(metrics(), Selection::Horizontal, [style(); 2])
        .unwrap()
        .horizontal
        .unwrap();
    let grip = bar.grip([12., 94.]).unwrap();
    assert_eq!(grip, 10.);
    assert!(bar.grip([12., 80.]).is_none());
    close(bar.drag_offset(12., grip).unwrap(), 0.);
    close(bar.drag_offset(85.5, grip).unwrap(), 300.);
    let mut m = metrics();
    m.viewport[0] = 100.;
    m.content[0] = 1000.;
    let resized = measure(m, Selection::Horizontal, [style(); 2])
        .unwrap()
        .horizontal
        .unwrap();
    // New travel84 and max900, retaining ten-pixel grip. No stale147/600.
    close(resized.drag_offset(54., grip).unwrap(), 450.);
    assert_eq!(resized.drag_offset(-1e8, grip), Some(0.));
    assert_eq!(resized.drag_offset(1e8, grip), Some(900.));
    assert_eq!(resized.drag_offset(f64::NAN, grip), None);
    assert_eq!(resized.drag_offset(0., f64::INFINITY), None);
    m.content[0] = 99.;
    assert!(
        measure(m, Selection::Horizontal, [style(); 2])
            .unwrap()
            .horizontal
            .is_none()
    );
}

#[test]
fn tiny_viewports_zero_area_and_zero_travel_never_jump_or_overlap() {
    let tiny = Metrics {
        viewport: [1., 1.],
        content: [100., 100.],
        offset: [999., -99.],
    };
    let g = measure(tiny, Selection::Both, [style(); 2]).unwrap();
    let h = g.horizontal.unwrap();
    let v = g.vertical.unwrap();
    assert_eq!(
        h.envelope,
        Rect {
            origin: [0., 0.5],
            size: [0.5, 0.5]
        }
    );
    assert_eq!(
        v.envelope,
        Rect {
            origin: [0.5, 0.],
            size: [0.5, 0.5]
        }
    );
    assert_eq!(h.thumb.size, [0., 0.]);
    assert_eq!(h.drag_offset(0., 0.), None);
    assert_eq!(v.grip(v.thumb.origin), None);
    let huge = Style {
        inset: 16384.,
        radius: 16384.,
        min_length: 16384.,
        ..style()
    };
    let bar = measure(metrics(), Selection::Horizontal, [huge; 2])
        .unwrap()
        .horizontal
        .unwrap();
    assert_eq!(bar.thumb.size, [0., 0.]);
    assert_eq!(bar.radius, 0.);
    assert_eq!(bar.centered_offset(50.), None);
    let long = Style {
        min_length: 16384.,
        ..style()
    };
    let bar = measure(metrics(), Selection::Horizontal, [long; 2])
        .unwrap()
        .horizontal
        .unwrap();
    assert_eq!(bar.thumb.size[0], 196.);
    assert_eq!(bar.drag_offset(50., 0.), None);
    let invisible = Style {
        track_width: 0.,
        ..style()
    };
    let bar = measure(metrics(), Selection::Horizontal, [invisible; 2])
        .unwrap()
        .horizontal
        .unwrap();
    assert_eq!(bar.centered_offset(50.), None);
    let zero = Style {
        envelope_width: 0.,
        track_width: 0.,
        ..style()
    };
    assert_eq!(
        measure(metrics(), Selection::Both, [zero; 2]).unwrap(),
        Geometry::default()
    );
}

#[test]
fn hover_width_changes_do_not_move_the_interaction_envelope_or_corner() {
    let resting = measure(metrics(), Selection::Both, [style(); 2]).unwrap();
    let narrow = Style {
        track_width: 2.,
        thumb_width: 1.,
        inset: 0.,
        ..style()
    };
    let hovering = measure(metrics(), Selection::Both, [narrow; 2]).unwrap();
    assert_eq!(
        resting.horizontal.unwrap().envelope,
        hovering.horizontal.unwrap().envelope
    );
    assert_eq!(
        resting.vertical.unwrap().envelope,
        hovering.vertical.unwrap().envelope
    );
    assert_ne!(
        resting.vertical.unwrap().track,
        hovering.vertical.unwrap().track
    );
}

#[test]
fn generated_geometry_stays_finite_contained_and_maps_drag_monotonically() {
    let mut seed = 717u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((seed >> 32) % 16385) as f64
    };
    for sample in 0..2000 {
        let viewport = [next(), next()];
        let m = Metrics {
            viewport,
            content: [next() * 10., next() * 10.],
            offset: [next() - 2000., next() - 2000.],
        };
        let mut styles = [style(); 2];
        for s in &mut styles {
            s.envelope_width = next();
            s.track_width = next().min(s.envelope_width);
            s.thumb_width = next();
            s.inset = if sample % 2 == 0 { next() % 8. } else { next() };
            s.min_length = if sample % 2 == 0 {
                next() % 64.
            } else {
                next()
            };
            s.radius = next();
        }
        let g = measure(m, Selection::Both, styles).unwrap();
        for bar in [g.horizontal, g.vertical].into_iter().flatten() {
            contained(
                bar.envelope,
                Rect {
                    origin: [0.; 2],
                    size: viewport,
                },
            );
            contained(bar.track, bar.envelope);
            contained(bar.thumb, bar.track);
            assert!(bar.radius.is_finite() && bar.radius >= 0.);
            let mut last = 0.;
            for step in 0..=10 {
                if let Some(offset) =
                    bar.centered_offset(viewport[bar.axis.index()] * step as f64 / 10.)
                {
                    assert!(offset >= last && offset <= bar.max_offset);
                    last = offset;
                }
            }
        }
    }
}

#[test]
fn invalid_measurements_fail_and_extreme_finite_measurements_are_safe() {
    for value in [f64::NAN, f64::INFINITY, -1., f64::MAX] {
        for index in 0..2 {
            let mut m = metrics();
            m.viewport[index] = value;
            assert!(measure(m, Selection::Both, [style(); 2]).is_err());
            let mut m = metrics();
            m.content[index] = value;
            assert!(measure(m, Selection::Both, [style(); 2]).is_err());
        }
    }
    let bad = Style {
        track_width: 17.,
        ..style()
    };
    assert!(measure(metrics(), Selection::Both, [bad; 2]).is_err());
    let m = Metrics {
        viewport: [f32::MAX as f64 / 2.; 2],
        content: [f32::MAX as f64; 2],
        offset: [f64::MAX; 2],
    };
    let g = measure(m, Selection::Both, [style(); 2]).unwrap();
    for bar in [g.horizontal.unwrap(), g.vertical.unwrap()] {
        contained(bar.thumb, bar.track);
        close(bar.offset, bar.max_offset);
    }
}
