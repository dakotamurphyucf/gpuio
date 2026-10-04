use super::*;
use gpui::size;

fn bounds(width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(10.), px(20.)), size(px(width), px(height)))
}
fn triangle_area(points: &[(f64, f64)]) -> f64 {
    let [a, b, c] = points else { unreachable!() };
    ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs() / 2.
}
fn triangles(geometry: &Geometry) -> Vec<Vec<(f64, f64)>> {
    geometry
        .path
        .vertices
        .chunks_exact(3)
        .map(|triangle| {
            triangle
                .iter()
                .map(|v| {
                    (
                        f32::from(v.xy_position.x) as f64,
                        f32::from(v.xy_position.y) as f64,
                    )
                })
                .collect()
        })
        .collect()
}

#[test]
fn ring_is_clockwise_inscribed_and_has_an_empty_center() {
    let bounds = bounds(80., 40.);
    let points = ring_points(bounds, 0., 0.25, 2.).unwrap();
    assert_eq!(points[0], (70., 40.));
    assert!(points[1].1 > 40.); // Positive screen Y is clockwise from 3 o'clock.
    for end in [0.25, 0.5, 0.999, 1.] {
        let geometry = ring(bounds, 0., end, 2.).unwrap().unwrap();
        let triangles = triangles(&geometry);
        assert!(!triangles.is_empty());
        let area: f64 = triangles.iter().map(|t| triangle_area(t)).sum();
        let expected = std::f64::consts::PI * (400. - 225.) * end;
        assert!(
            (area / expected - 1.).abs() < 0.02,
            "end={end}, area={area}"
        );
        for triangle in triangles {
            // Test edge/face samples of the actual tessellation, not just the
            // outline: a disk or seam-spanning triangle would fail here.
            for i in 0..=10 {
                for j in 0..=10 - i {
                    let weights = [i as f64 / 10., j as f64 / 10., (10 - i - j) as f64 / 10.];
                    let x: f64 = triangle.iter().zip(weights).map(|(p, w)| p.0 * w).sum();
                    let y: f64 = triangle.iter().zip(weights).map(|(p, w)| p.1 * w).sum();
                    let radius = (x - 50.).hypot(y - 40.);
                    assert!((14.85..=20.001).contains(&radius), "r={radius}");
                }
            }
        }
    }
    assert!(ring(bounds, 0., 0., 1.).unwrap().is_none());
}

#[test]
fn rounded_fill_stays_in_both_track_and_moving_edge_for_short_fills() {
    let bounds = bounds(200., 20.);
    for radii in [[10., 10., 10., 10.], [20., 0., 4., 2.], [0., 9., 1., 10.]] {
        let corners = Corners {
            top_left: px(radii[0]),
            top_right: px(radii[1]),
            bottom_right: px(radii[2]),
            bottom_left: px(radii[3]),
        };
        let radii = radii.map(f64::from);
        for (start, end) in [
            (0., 0.0001),
            (0., 0.005),
            (0., 0.5),
            (0., 1.),
            (0.4, 0.405),
            (0.995, 1.),
        ] {
            let geometry = linear(bounds, corners, start, end, 2.).unwrap().unwrap();
            let track = Rect::from_bounds(bounds).unwrap();
            let fill = Rect {
                left: 10. + 200. * start,
                right: 10. + 200. * end,
                ..track
            };
            for triangle in triangles(&geometry) {
                for i in 0..=6 {
                    for j in 0..=6 - i {
                        let weights = [i as f64 / 6., j as f64 / 6., (6 - i - j) as f64 / 6.];
                        let x: f64 = triangle.iter().zip(weights).map(|(p, w)| p.0 * w).sum();
                        let y: f64 = triangle.iter().zip(weights).map(|(p, w)| p.1 * w).sum();
                        for rect in [track, fill] {
                            let radii = rect.clamp(radii);
                            assert!(x >= rect.left - 0.0001 && x <= rect.right + 0.0001);
                            assert!(
                                y >= rect.edge(radii, x, false) - 0.002
                                    && y <= rect.edge(radii, x, true) + 0.002,
                                "start={start},end={end},x={x},y={y}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn geometry_is_bounded_and_rejects_invalid_inputs() {
    let corners = Corners::all(px(10000.));
    for scale in [0.5, 1., 2., 8., f32::MAX] {
        let ring = ring(bounds(100000., 100000.), 0., 1., scale)
            .unwrap()
            .unwrap();
        assert!(ring.points <= 2 * (MAX_RING_SEGMENTS + 1));
        assert!(ring.path.vertices.len() <= ring.points * 3);
        let bar = linear(bounds(100000., 20000.), corners, 0.0001, 0.9999, scale)
            .unwrap()
            .unwrap();
        assert!(bar.points <= MAX_LINEAR_POINTS);
        assert!(bar.path.vertices.len() <= bar.points * 3);
    }
    for bounds in [
        bounds(0., 20.),
        bounds(-1., 20.),
        bounds(f32::NAN, 20.),
        bounds(f32::INFINITY, 20.),
    ] {
        assert!(ring(bounds, 0., 1., 1.).is_err());
        assert!(linear(bounds, corners, 0., 1., 1.).is_err());
    }
    for scale in [0., -1., f32::NAN, f32::INFINITY] {
        assert!(ring(bounds(20., 20.), 0., 1., scale).is_err());
        assert!(linear(bounds(20., 20.), corners, 0., 1., scale).is_err());
    }
    assert!(linear(bounds(20., 20.), Corners::all(px(-1.)), 0., 1., 1.).is_err());
    assert!(ring(bounds(20., 20.), 0.5, 0.25, 1.).is_err());
    assert!(
        linear(bounds(20., 20.), corners, 0., 0., 1.)
            .unwrap()
            .is_none()
    );
}
