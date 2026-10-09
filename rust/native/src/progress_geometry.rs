//! Bounded progress paths. Rounded linear fill is intersected with its rounded
//! track in geometry, because GPUI's content mask is rectangular.
use gpui::{Bounds, Corners, Path, PathBuilder, Pixels, point, px};
use std::f64::consts::{FRAC_PI_2, TAU};

pub(crate) const MAX_LINEAR_POINTS: usize = 16_384;
pub(crate) const MAX_RING_SEGMENTS: usize = 4096;
const MAX_CORNER_SEGMENTS: usize = 256;

pub(crate) struct Geometry {
    pub path: Path<Pixels>,
    pub points: usize,
}
#[derive(Debug)]
pub(crate) struct InvalidGeometry;

#[derive(Clone, Copy)]
struct Rect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}
impl Rect {
    fn from_bounds(bounds: Bounds<Pixels>) -> Result<Self, InvalidGeometry> {
        let rect = Self {
            left: f32::from(bounds.left()) as f64,
            top: f32::from(bounds.top()) as f64,
            right: f32::from(bounds.right()) as f64,
            bottom: f32::from(bounds.bottom()) as f64,
        };
        if [rect.left, rect.top, rect.right, rect.bottom]
            .into_iter()
            .any(|x| !x.is_finite())
            || rect.right <= rect.left
            || rect.bottom <= rect.top
        {
            return Err(InvalidGeometry);
        }
        Ok(rect)
    }
    fn clamp(self, radii: [f64; 4]) -> [f64; 4] {
        let limit = ((self.right - self.left).min(self.bottom - self.top) / 2.).max(0.);
        radii.map(|r| r.min(limit))
    }
    fn edge(self, radii: [f64; 4], x: f64, bottom: bool) -> f64 {
        let (left, right) = if bottom {
            (radii[3], radii[2])
        } else {
            (radii[0], radii[1])
        };
        let inset = |radius: f64, distance: f64| {
            if radius == 0. || distance >= radius {
                0.
            } else {
                radius
                    - (radius * radius - (radius - distance).powi(2))
                        .max(0.)
                        .sqrt()
            }
        };
        let delta = inset(left, x - self.left).max(inset(right, self.right - x));
        if bottom {
            self.bottom - delta
        } else {
            self.top + delta
        }
    }
    fn knots(self, radii: [f64; 4], scale: f64, clip: Self, xs: &mut Vec<f64>) {
        for (index, radius) in radii.into_iter().enumerate() {
            let segments = segments(FRAC_PI_2, radius * scale, MAX_CORNER_SEGMENTS);
            for i in 0..=segments {
                let offset = radius * (1. - (FRAC_PI_2 * i as f64 / segments as f64).cos());
                let x = if index == 0 || index == 3 {
                    self.left + offset
                } else {
                    self.right - offset
                };
                if x > clip.left && x < clip.right {
                    xs.push(x);
                }
            }
        }
    }
}

fn segments(angle: f64, radius_px: f64, maximum: usize) -> usize {
    // Chord sagitta target 0.125 physical pixels, with explicit worst-case caps.
    (angle.abs() * radius_px.max(1.).sqrt())
        .ceil()
        .max(1.)
        .min(maximum as f64) as usize
}
fn polygon(points: &[(f64, f64)]) -> Result<Geometry, InvalidGeometry> {
    if points.len() < 3
        || points
            .iter()
            .any(|(x, y)| !(*x as f32).is_finite() || !(*y as f32).is_finite())
    {
        return Err(InvalidGeometry);
    }
    let mut builder = PathBuilder::fill();
    builder.move_to(point(px(points[0].0 as f32), px(points[0].1 as f32)));
    for &(x, y) in &points[1..] {
        builder.line_to(point(px(x as f32), px(y as f32)));
    }
    builder.close();
    Ok(Geometry {
        path: builder.build().map_err(|_| InvalidGeometry)?,
        points: points.len(),
    })
}

fn linear_points(
    bounds: Bounds<Pixels>,
    corners: Corners<Pixels>,
    start: f64,
    end: f64,
    scale: f32,
) -> Result<Vec<(f64, f64)>, InvalidGeometry> {
    if !start.is_finite() || !end.is_finite() || !scale.is_finite() || scale <= 0. {
        return Err(InvalidGeometry);
    }
    let radii = [
        corners.top_left,
        corners.top_right,
        corners.bottom_right,
        corners.bottom_left,
    ]
    .map(|r| f32::from(r) as f64);
    if radii.iter().any(|r| !r.is_finite() || *r < 0.) {
        return Err(InvalidGeometry);
    }
    let track = Rect::from_bounds(bounds)?;
    let width = track.right - track.left;
    let fill = Rect {
        left: track.left + width * start.clamp(0., 1.),
        right: track.left + width * end.clamp(0., 1.),
        ..track
    };
    if fill.right <= fill.left {
        return Ok(vec![]);
    }
    let outer = track.clamp(radii);
    let inner = fill.clamp(radii);
    let mut xs = vec![fill.left, fill.right];
    track.knots(outer, scale as f64, fill, &mut xs);
    fill.knots(inner, scale as f64, fill, &mut xs);
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    // Split at curve crossings so a chord does not bridge the envelope's kink.
    let mut crossings = Vec::new();
    for pair in xs.windows(2) {
        for bottom in [false, true] {
            let difference = |x| track.edge(outer, x, bottom) - fill.edge(inner, x, bottom);
            let (mut left, mut right) = (pair[0], pair[1]);
            let sign = difference(left).signum();
            if difference(left) == 0.
                || difference(right) == 0.
                || sign == difference(right).signum()
            {
                continue;
            }
            for _ in 0..40 {
                let mid = (left + right) / 2.;
                if difference(mid).signum() == sign {
                    left = mid;
                } else {
                    right = mid;
                }
            }
            crossings.push((left + right) / 2.);
        }
    }
    xs.extend(crossings);
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    if xs.len() * 2 > MAX_LINEAR_POINTS {
        return Err(InvalidGeometry);
    }
    let top = xs.iter().map(|&x| {
        (
            x,
            track.edge(outer, x, false).max(fill.edge(inner, x, false)),
        )
    });
    let bottom = xs
        .iter()
        .rev()
        .map(|&x| (x, track.edge(outer, x, true).min(fill.edge(inner, x, true))));
    Ok(top.chain(bottom).collect())
}

pub(crate) fn linear(
    bounds: Bounds<Pixels>,
    corners: Corners<Pixels>,
    start: f64,
    end: f64,
    scale: f32,
) -> Result<Option<Geometry>, InvalidGeometry> {
    let points = linear_points(bounds, corners, start, end, scale)?;
    if points.is_empty() {
        Ok(None)
    } else {
        polygon(&points).map(Some)
    }
}

fn ring_points(
    bounds: Bounds<Pixels>,
    start: f64,
    end: f64,
    scale: f32,
) -> Result<Vec<(f64, f64)>, InvalidGeometry> {
    if !start.is_finite()
        || !end.is_finite()
        || !(0. ..=1.).contains(&start)
        || !(start..=1.).contains(&end)
        || !scale.is_finite()
        || scale <= 0.
    {
        return Err(InvalidGeometry);
    }
    let rect = Rect::from_bounds(bounds)?;
    if start == end {
        return Ok(vec![]);
    }
    let diameter = (rect.right - rect.left).min(rect.bottom - rect.top);
    let outer = diameter / 2.;
    let inner = (outer - (diameter * 0.15).min(5.)).max(0.);
    let center = ((rect.left + rect.right) / 2., (rect.top + rect.bottom) / 2.);
    let steps = segments((end - start) * TAU, outer * scale as f64, MAX_RING_SEGMENTS);
    let mut points = Vec::with_capacity((steps + 1) * 2);
    for (radius, reverse) in [(outer, false), (inner, true)] {
        for index in 0..=steps {
            let index = if reverse { steps - index } else { index };
            let angle = (start + (end - start) * index as f64 / steps as f64) * TAU;
            points.push((
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            ));
        }
    }
    Ok(points)
}
pub(crate) fn ring(
    bounds: Bounds<Pixels>,
    start: f64,
    end: f64,
    scale: f32,
) -> Result<Option<Geometry>, InvalidGeometry> {
    let points = ring_points(bounds, start, end, scale)?;
    if points.is_empty() {
        Ok(None)
    } else {
        polygon(&points).map(Some)
    }
}

#[cfg(test)]
#[path = "progress_geometry_test.rs"]
mod tests;
