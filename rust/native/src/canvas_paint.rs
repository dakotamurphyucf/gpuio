//! GPUI mesh painting. The caller supplies one shared budget per window frame;
//! clipping/culling precede triangle expansion. Text/image painting is separate.
use crate::{canvas_mesh::Mesh, canvas_plan::MAX_DRAW_VERTICES};
use gpui::{Bounds, ContentMask, Pixels, Window, point, px, size};
use gpuio_protocol::{
    canvas::{Point, Rect, Transform},
    canvas_view::Viewport,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidGeometry,
    LimitExceeded,
}

pub struct FrameBudget {
    remaining: usize,
}
impl Default for FrameBudget {
    fn default() -> Self {
        Self {
            remaining: MAX_DRAW_VERTICES,
        }
    }
}
impl FrameBudget {
    pub fn used_vertices(&self) -> usize {
        MAX_DRAW_VERTICES - self.remaining
    }
    fn reserve(&mut self, vertices: usize) -> Result<(), Error> {
        if vertices > self.remaining {
            return Err(Error::LimitExceeded);
        }
        self.remaining -= vertices;
        Ok(())
    }
}

pub struct Placement<'a> {
    pub origin: Point,
    pub transform: Transform,
    pub viewport: Viewport,
    pub bounds: Bounds<Pixels>,
    pub clips: &'a [Rect],
}
impl Placement<'_> {
    fn project_world(&self, p: Point) -> gpui::Point<Pixels> {
        point(
            self.bounds.origin.x + px(((p.x - self.viewport.origin.x) * self.viewport.zoom) as f32),
            self.bounds.origin.y + px(((p.y - self.viewport.origin.y) * self.viewport.zoom) as f32),
        )
    }
    fn project_local(&self, p: Point) -> gpui::Point<Pixels> {
        self.project_world(self.transform.apply(Point {
            x: p.x + self.origin.x,
            y: p.y + self.origin.y,
        }))
    }
    fn clip(&self) -> Result<Option<Bounds<Pixels>>, Error> {
        if !self.viewport.is_valid()
            || !self.transform.is_valid()
            || !self.origin.is_valid()
            || self.clips.len() > 8
            || self.clips.iter().any(|r| !r.is_valid())
            || ![
                f32::from(self.bounds.origin.x),
                f32::from(self.bounds.origin.y),
                f32::from(self.bounds.size.width),
                f32::from(self.bounds.size.height),
            ]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(Error::InvalidGeometry);
        }
        let mut bounds = self.bounds;
        for clip in self.clips {
            let origin = self.project_world(Point {
                x: clip.x,
                y: clip.y,
            });
            let next = Bounds {
                origin,
                size: size(
                    px((clip.width * self.viewport.zoom) as f32),
                    px((clip.height * self.viewport.zoom) as f32),
                ),
            };
            bounds = bounds.intersect(&next);
        }
        Ok(
            (f32::from(bounds.size.width) > 0. && f32::from(bounds.size.height) > 0.)
                .then_some(bounds),
        )
    }
    fn mesh_bounds(&self, mesh: &Mesh) -> Option<Bounds<Pixels>> {
        let bounds = mesh.bounds()?;
        let points = [
            Point {
                x: bounds.left,
                y: bounds.top,
            },
            Point {
                x: bounds.right,
                y: bounds.top,
            },
            Point {
                x: bounds.left,
                y: bounds.bottom,
            },
            Point {
                x: bounds.right,
                y: bounds.bottom,
            },
        ]
        .map(|p| self.project_local(p));
        let left = points
            .iter()
            .map(|p| f32::from(p.x))
            .fold(f32::INFINITY, f32::min);
        let top = points
            .iter()
            .map(|p| f32::from(p.y))
            .fold(f32::INFINITY, f32::min);
        let right = points
            .iter()
            .map(|p| f32::from(p.x))
            .fold(f32::NEG_INFINITY, f32::max);
        let bottom = points
            .iter()
            .map(|p| f32::from(p.y))
            .fold(f32::NEG_INFINITY, f32::max);
        Some(Bounds {
            origin: point(px(left), px(top)),
            size: size(px(right - left), px(bottom - top)),
        })
    }
}

/// Paint only an admitted shape mesh. Local stroke geometry is transformed as a
/// whole. World clips stay fixed when an object moves; GPUI applies device scale
/// once. Returns expanded vertices (zero for fully clipped/degenerate geometry).
pub fn paint(
    mesh: &Mesh,
    placement: &Placement<'_>,
    color: u32,
    budget: &mut FrameBudget,
    window: &mut Window,
) -> Result<usize, Error> {
    let Some(clip) = placement.clip()? else {
        return Ok(0);
    };
    let Some(bounds) = placement.mesh_bounds(mesh) else {
        return Ok(0);
    };
    let visible = clip.intersect(&bounds);
    if f32::from(visible.size.width) <= 0. || f32::from(visible.size.height) <= 0. {
        return Ok(0);
    }
    let vertices = mesh.triangle_count() * 3;
    if vertices == 0 {
        return Ok(0);
    }
    budget.reserve(vertices)?;
    let first = mesh.triangles().next().expect("nonempty admitted mesh");
    let mut path = gpui::Path::new(placement.project_local(first[0]));
    path.vertices.reserve_exact(vertices);
    for triangle in mesh.triangles() {
        let [a, b, c] = triangle.map(|p| placement.project_local(p));
        path.push_triangle((a, b, c), (point(0., 1.), point(0., 1.), point(0., 1.)));
    }
    window.with_content_mask(Some(ContentMask { bounds: clip }), |window| {
        window.paint_path(path, gpui::rgba(color));
    });
    Ok(vertices)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_clips_intersect_after_viewport_and_ignore_item_transform() {
        let clips = [
            Rect {
                x: 10.,
                y: 20.,
                width: 30.,
                height: 40.,
            },
            Rect {
                x: 20.,
                y: 30.,
                width: 30.,
                height: 40.,
            },
        ];
        let mut placement = Placement {
            origin: Point { x: 0., y: 0. },
            transform: Transform {
                tx: 99.,
                ..Transform::IDENTITY
            },
            viewport: Viewport {
                origin: Point { x: 5., y: 10. },
                zoom: 2.,
            },
            bounds: Bounds {
                origin: point(px(10.), px(15.)),
                size: size(px(300.), px(300.)),
            },
            clips: &clips,
        };
        let clip = placement.clip().unwrap().unwrap();
        assert_eq!(clip.origin, point(px(40.), px(55.)));
        assert_eq!(clip.size, size(px(40.), px(60.)));
        let touching = [
            Rect {
                x: 0.,
                y: 0.,
                width: 10.,
                height: 10.,
            },
            Rect {
                x: 10.,
                y: 0.,
                width: 10.,
                height: 10.,
            },
        ];
        placement.clips = &touching;
        assert!(placement.clip().unwrap().is_none());
    }
    #[test]
    fn one_frame_budget_cannot_be_bypassed_by_multiple_draws() {
        let mut budget = FrameBudget::default();
        budget.reserve(MAX_DRAW_VERTICES - 3).unwrap();
        assert_eq!(budget.reserve(6), Err(Error::LimitExceeded));
        assert_eq!(budget.used_vertices(), MAX_DRAW_VERTICES - 3);
        budget.reserve(3).unwrap();
        assert_eq!(budget.used_vertices(), MAX_DRAW_VERTICES);
    }
}
