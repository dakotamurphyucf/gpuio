//! Bounded, cancellable local-space geometry preparation, with no GPUI/OCaml
//! callbacks. Stroke tessellation precedes affine transformation, preserving
//! local stroke widths under shear, reflection and nonuniform scaling.
use gpuio_protocol::canvas::{Path, PathCommand, Point, Rect};
use lyon::{
    geom::{CubicBezierSegment, QuadraticBezierSegment},
    math::{Point as LyonPoint, point},
    path::Path as LyonPath,
    tessellation::{
        FillGeometryBuilder, FillOptions, FillRule, FillTessellator, FillVertex, GeometryBuilder,
        GeometryBuilderError, LineCap, LineJoin, StrokeGeometryBuilder, StrokeOptions,
        StrokeTessellator, StrokeVertex, VertexId,
    },
};
use std::{
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_SEGMENTS: usize = 16_384;
pub const MAX_VERTICES: usize = 65_536;
pub const MAX_INDICES: usize = 196_608;

#[derive(Clone, Copy)]
struct Limits {
    commands: usize,
    segments: usize,
    vertices: usize,
    indices: usize,
}
impl Limits {
    const CANVAS: Self = Self {
        commands: gpuio_protocol::canvas::MAX_PATH_COMMANDS,
        segments: MAX_SEGMENTS,
        vertices: MAX_VERTICES,
        indices: MAX_INDICES,
    };
    // Only native-generated chart geometry uses this allowance. Wire canvas
    // input keeps its existing command/count/scene admission contract.
    const CHART: Self = Self {
        commands: 200_004,
        segments: 262_144,
        vertices: 1_000_000,
        indices: 1_000_000,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidGeometry,
    LimitExceeded,
    Cancelled,
    Tessellation,
}

#[derive(Clone, Copy)]
pub enum Geometry<'a> {
    Rectangle(Rect),
    Ellipse(Rect),
    Path(&'a Path),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Fill,
    Stroke(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// Private indices always address admitted vertices. Positions are relative to
/// a nearby f64 anchor, avoiding f32 precision loss from large translations.
/// Retaining owners must charge [`Mesh::retained_bytes`], including vector capacities.
#[derive(Debug)]
pub struct Mesh {
    origin: Point,
    vertices: Vec<LyonPoint>,
    indices: Vec<u32>,
    bounds: Option<Bounds>,
    segments: usize,
}
impl Mesh {
    pub fn triangles(&self) -> impl Iterator<Item = [Point; 3]> + '_ {
        self.indices.chunks_exact(3).map(|ids| {
            [ids[0], ids[1], ids[2]].map(|id| {
                let vertex = self.vertices[id as usize];
                Point {
                    x: self.origin.x + f64::from(vertex.x),
                    y: self.origin.y + f64::from(vertex.y),
                }
            })
        })
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    pub fn segments(&self) -> usize {
        self.segments
    }
    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds
    }
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + self.vertices.capacity() * size_of::<LyonPoint>()
            + self.indices.capacity() * size_of::<u32>()
    }
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

struct Flatten<'a> {
    limits: Limits,
    builder: lyon::path::path::Builder,
    origin: Point,
    cancel: &'a AtomicBool,
    segments: usize,
    active: bool,
}
impl<'a> Flatten<'a> {
    fn new(origin: Point, cancel: &'a AtomicBool, limits: Limits) -> Self {
        Self {
            builder: LyonPath::builder(),
            limits,
            origin,
            cancel,
            segments: 0,
            active: false,
        }
    }
    fn point(&self, p: Point) -> Result<LyonPoint, Error> {
        let p = point((p.x - self.origin.x) as f32, (p.y - self.origin.y) as f32);
        if p.x.is_finite() && p.y.is_finite() {
            Ok(p)
        } else {
            Err(Error::InvalidGeometry)
        }
    }
    fn charge(&mut self) -> Result<(), Error> {
        check_cancel(self.cancel)?;
        if self.segments == self.limits.segments {
            return Err(Error::LimitExceeded);
        }
        self.segments += 1;
        Ok(())
    }
    fn begin(&mut self, p: Point) -> Result<(), Error> {
        self.charge()?;
        self.end(false);
        self.builder.begin(self.point(p)?);
        self.active = true;
        Ok(())
    }
    fn line(&mut self, p: Point) -> Result<(), Error> {
        self.charge()?;
        self.builder.line_to(self.point(p)?);
        Ok(())
    }
    fn end(&mut self, close: bool) {
        if self.active {
            self.builder.end(close);
            self.active = false;
        }
    }
    fn finish(mut self) -> (LyonPath, usize) {
        self.end(false);
        (self.builder.build(), self.segments)
    }
}

fn flatten_path(
    path: &Path,
    tolerance: f64,
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<(LyonPath, Point, usize), Error> {
    if !path.is_valid_up_to(limits.commands) {
        return Err(Error::InvalidGeometry);
    }
    let Some(PathCommand::Move(origin)) = path.0.first() else {
        return Err(Error::InvalidGeometry);
    };
    let mut result = Flatten::new(*origin, cancel, limits);
    let mut current = *origin;
    for command in &path.0 {
        match *command {
            PathCommand::Move(p) => result.begin(p)?,
            PathCommand::Line(p) => result.line(p)?,
            PathCommand::Quadratic(control, end) => {
                let curve = QuadraticBezierSegment {
                    from: lyon::geom::point(current.x, current.y),
                    ctrl: lyon::geom::point(control.x, control.y),
                    to: lyon::geom::point(end.x, end.y),
                };
                for p in curve.flattened(tolerance) {
                    result.line(Point { x: p.x, y: p.y })?;
                }
            }
            PathCommand::Cubic(first, second, end) => {
                let curve = CubicBezierSegment {
                    from: lyon::geom::point(current.x, current.y),
                    ctrl1: lyon::geom::point(first.x, first.y),
                    ctrl2: lyon::geom::point(second.x, second.y),
                    to: lyon::geom::point(end.x, end.y),
                };
                for p in curve.flattened(tolerance) {
                    result.line(Point { x: p.x, y: p.y })?;
                }
            }
            PathCommand::Close => result.end(true),
        }
        match *command {
            PathCommand::Move(p)
            | PathCommand::Line(p)
            | PathCommand::Quadratic(_, p)
            | PathCommand::Cubic(_, _, p) => current = p,
            PathCommand::Close => (),
        }
    }
    let (path, segments) = result.finish();
    Ok((path, *origin, segments))
}

fn flatten(
    geometry: Geometry<'_>,
    tolerance: f64,
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<(LyonPath, Point, usize), Error> {
    let (rect, ellipse) = match geometry {
        Geometry::Path(path) => return flatten_path(path, tolerance, cancel, limits),
        Geometry::Rectangle(rect) => (rect, false),
        Geometry::Ellipse(rect) => (rect, true),
    };
    if !rect.is_valid() {
        return Err(Error::InvalidGeometry);
    }
    let origin = Point {
        x: rect.x,
        y: rect.y,
    };
    let mut result = Flatten::new(origin, cancel, limits);
    if ellipse {
        // The maximum-radius circle bounds affine ellipse chord error. Compute
        // segment admission before allocating; no enormous curve iterator.
        let radius = rect.width.max(rect.height) * 0.5;
        let angle = 2. * (1. - (tolerance / radius).min(1.)).acos();
        let count = (std::f64::consts::TAU / angle).ceil().max(4.);
        if !count.is_finite() || count > limits.segments as f64 {
            return Err(Error::LimitExceeded);
        }
        let count = count as usize;
        for index in 0..count {
            let theta = std::f64::consts::TAU * index as f64 / count as f64;
            let p = Point {
                x: rect.x + rect.width * (1. + theta.cos()) * 0.5,
                y: rect.y + rect.height * (1. + theta.sin()) * 0.5,
            };
            if index == 0 {
                result.begin(p)?;
            } else {
                result.line(p)?;
            }
        }
    } else {
        result.begin(origin)?;
        result.line(Point {
            x: rect.x + rect.width,
            y: rect.y,
        })?;
        result.line(Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        })?;
        result.line(Point {
            x: rect.x,
            y: rect.y + rect.height,
        })?;
    }
    result.end(true);
    let (path, segments) = result.finish();
    Ok((path, origin, segments))
}

struct Output<'a> {
    limits: Limits,
    vertices: Vec<LyonPoint>,
    indices: Vec<u32>,
    cancel: &'a AtomicBool,
    failure: Option<Error>,
}
impl Output<'_> {
    fn check(&mut self) -> Result<(), GeometryBuilderError> {
        if self.cancel.load(Ordering::Relaxed) {
            self.failure = Some(Error::Cancelled);
        }
        if self.failure.is_some() {
            Err(GeometryBuilderError::TooManyVertices)
        } else {
            Ok(())
        }
    }
    fn vertex(&mut self, p: LyonPoint) -> Result<VertexId, GeometryBuilderError> {
        self.check()?;
        if !p.x.is_finite() || !p.y.is_finite() {
            self.failure = Some(Error::InvalidGeometry);
            return Err(GeometryBuilderError::InvalidVertex);
        }
        if self.vertices.len() >= self.limits.vertices {
            self.failure = Some(Error::LimitExceeded);
            return Err(GeometryBuilderError::TooManyVertices);
        }
        let id = VertexId(self.vertices.len() as u32);
        self.vertices.push(p);
        Ok(id)
    }
}
impl GeometryBuilder for Output<'_> {
    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        if self.check().is_err() {
            return;
        }
        if [a, b, c]
            .iter()
            .any(|id| id.0 as usize >= self.vertices.len())
        {
            self.failure = Some(Error::Tessellation);
            return;
        }
        if self.indices.len() > self.limits.indices - 3 {
            self.failure = Some(Error::LimitExceeded);
            return;
        }
        self.indices.extend_from_slice(&[a.0, b.0, c.0]);
    }
    fn abort_geometry(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}
impl FillGeometryBuilder for Output<'_> {
    fn add_fill_vertex(
        &mut self,
        vertex: FillVertex<'_>,
    ) -> Result<VertexId, GeometryBuilderError> {
        self.vertex(vertex.position())
    }
}
impl StrokeGeometryBuilder for Output<'_> {
    fn add_stroke_vertex(
        &mut self,
        vertex: StrokeVertex<'_, '_>,
    ) -> Result<VertexId, GeometryBuilderError> {
        self.vertex(vertex.position())
    }
}

/// Tolerance is in local coordinates; the renderer selects it from effective
/// scale. Fill uses even-odd; stroke uses butt caps/miter joins, miter limit four.
/// Failures never expose partial geometry. Internal Lyon working memory is not
/// included in the retained output charge; callers schedule bounded worker jobs.
pub fn prepare(
    geometry: Geometry<'_>,
    style: Style,
    tolerance: f64,
    cancel: &AtomicBool,
) -> Result<Mesh, Error> {
    prepare_with_limits(geometry, style, tolerance, cancel, Limits::CANVAS)
}

pub(crate) fn prepare_chart(
    path: &Path,
    style: Style,
    tolerance: f64,
    cancel: &AtomicBool,
) -> Result<Mesh, Error> {
    prepare_with_limits(
        Geometry::Path(path),
        style,
        tolerance,
        cancel,
        Limits::CHART,
    )
}

fn prepare_with_limits(
    geometry: Geometry<'_>,
    style: Style,
    tolerance: f64,
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<Mesh, Error> {
    check_cancel(cancel)?;
    if !tolerance.is_finite() || !(1e-9..=1_000_000.).contains(&tolerance) {
        return Err(Error::InvalidGeometry);
    }
    match style {
        Style::Stroke(width) if !width.is_finite() || width <= 0. || width > 256. => {
            return Err(Error::InvalidGeometry);
        }
        Style::Fill => {
            if let Geometry::Path(path) = geometry
                && !path.is_closed_up_to(limits.commands)
            {
                return Err(Error::InvalidGeometry);
            }
        }
        Style::Stroke(_) => (),
    }
    let (path, origin, segments) = flatten(geometry, tolerance, cancel, limits)?;
    let mut output = Output {
        limits,
        vertices: Vec::new(),
        indices: Vec::new(),
        cancel,
        failure: None,
    };
    let result = match style {
        Style::Fill => FillTessellator::new().tessellate_path(
            &path,
            &FillOptions::default().with_fill_rule(FillRule::EvenOdd),
            &mut output,
        ),
        Style::Stroke(width) => StrokeTessellator::new().tessellate_path(
            &path,
            &StrokeOptions::default()
                .with_line_width(width as f32)
                .with_line_cap(LineCap::Butt)
                .with_line_join(LineJoin::Miter)
                .with_miter_limit(4.),
            &mut output,
        ),
    };
    check_cancel(cancel)?;
    if let Some(error) = output.failure {
        return Err(error);
    }
    result.map_err(|_| Error::Tessellation)?;
    let mut bounds: Option<Bounds> = None;
    for p in &output.vertices {
        let x = origin.x + f64::from(p.x);
        let y = origin.y + f64::from(p.y);
        bounds = Some(match bounds {
            None => Bounds {
                left: x,
                top: y,
                right: x,
                bottom: y,
            },
            Some(b) => Bounds {
                left: b.left.min(x),
                top: b.top.min(y),
                right: b.right.max(x),
                bottom: b.bottom.max(y),
            },
        });
    }
    Ok(Mesh {
        origin,
        vertices: output.vertices,
        indices: output.indices,
        bounds,
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::canvas::Transform;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    fn rect() -> Rect {
        Rect {
            x: 0.,
            y: 0.,
            width: 20.,
            height: 10.,
        }
    }
    fn mesh(geometry: Geometry<'_>, style: Style) -> Mesh {
        prepare(geometry, style, 0.05, &AtomicBool::new(false)).unwrap()
    }
    fn area(mesh: &Mesh, transform: Transform) -> f64 {
        mesh.triangles()
            .map(|triangle| {
                let [a, b, c] = triangle.map(|p| transform.apply(p));
                ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() * 0.5
            })
            .sum()
    }
    #[test]
    fn filled_rectangle_and_affine_stroke_keep_local_geometry() {
        let fill = mesh(Geometry::Rectangle(rect()), Style::Fill);
        assert_eq!(fill.triangle_count(), 2);
        assert_eq!(area(&fill, Transform::IDENTITY), 200.);
        let line = Path(vec![
            PathCommand::Move(p(0., 0.)),
            PathCommand::Line(p(10., 0.)),
        ]);
        let stroke = mesh(Geometry::Path(&line), Style::Stroke(2.));
        assert!((area(&stroke, Transform::IDENTITY) - 20.).abs() < 1e-6);
        let transform = Transform {
            a: 2.,
            c: 1.,
            d: 3.,
            tx: 17.,
            ty: -8.,
            ..Transform::IDENTITY
        };
        assert!((area(&stroke, transform) - 120.).abs() < 1e-6);
        let reflected = Transform {
            a: -2.,
            ..transform
        };
        assert!((area(&stroke, reflected) - 120.).abs() < 1e-6);
        let bounds = stroke.bounds().unwrap();
        assert_eq!(
            bounds,
            Bounds {
                left: 0.,
                right: 10.,
                top: -1.,
                bottom: 1.
            }
        );
    }
    #[test]
    fn large_translation_does_not_erase_subpixel_rectangle_width() {
        let rect = Rect {
            x: 999_999.,
            y: 999_999.,
            width: 0.001,
            height: 0.002,
        };
        let mesh = mesh(Geometry::Rectangle(rect), Style::Fill);
        assert!((area(&mesh, Transform::IDENTITY) - 0.000002).abs() < 1e-11);
        assert!(mesh.triangles().flatten().all(Point::is_valid));
    }
    #[test]
    fn even_odd_holes_and_curves_use_bounded_flattened_contours() {
        let path = Path(vec![
            PathCommand::Move(p(0., 0.)),
            PathCommand::Line(p(20., 0.)),
            PathCommand::Line(p(20., 20.)),
            PathCommand::Line(p(0., 20.)),
            PathCommand::Close,
            PathCommand::Move(p(5., 5.)),
            PathCommand::Line(p(15., 5.)),
            PathCommand::Line(p(15., 15.)),
            PathCommand::Line(p(5., 15.)),
            PathCommand::Close,
        ]);
        let fill = mesh(Geometry::Path(&path), Style::Fill);
        assert!((area(&fill, Transform::IDENTITY) - 300.).abs() < 1e-5);
        let curves = Path(vec![
            PathCommand::Move(p(0., 0.)),
            PathCommand::Quadratic(p(10., -10.), p(20., 0.)),
            PathCommand::Cubic(p(30., 5.), p(10., 30.), p(0., 0.)),
            PathCommand::Close,
        ]);
        let fill = mesh(Geometry::Path(&curves), Style::Fill);
        assert!(fill.segments() > curves.0.len());
        assert!(fill.segments() <= MAX_SEGMENTS && fill.triangle_count() > 0);
        assert!(fill.retained_bytes() >= fill.vertex_count() * size_of::<LyonPoint>());
    }
    #[test]
    fn ellipse_accuracy_scales_and_extreme_flattening_is_rejected_early() {
        let shape = Geometry::Ellipse(rect());
        let cancel = AtomicBool::new(false);
        let coarse = prepare(shape, Style::Fill, 1., &cancel).unwrap();
        let fine = prepare(shape, Style::Fill, 0.001, &cancel).unwrap();
        let exact = std::f64::consts::PI * 10. * 5.;
        assert!((area(&fine, Transform::IDENTITY) - exact).abs() < 0.1);
        assert!(fine.segments() > coarse.segments());
        let huge = Geometry::Ellipse(Rect {
            x: -500_000.,
            y: -500_000.,
            width: 1_000_000.,
            height: 1_000_000.,
        });
        assert_eq!(
            prepare(huge, Style::Fill, 1e-9, &cancel).unwrap_err(),
            Error::LimitExceeded
        );
        let curve = Path(vec![
            PathCommand::Move(p(-500_000., 0.)),
            PathCommand::Cubic(
                p(-500_000., 500_000.),
                p(500_000., -500_000.),
                p(500_000., 0.),
            ),
        ]);
        assert_eq!(
            prepare(Geometry::Path(&curve), Style::Stroke(1.), 1e-9, &cancel).unwrap_err(),
            Error::LimitExceeded
        );
    }
    #[test]
    fn cancellation_and_invalid_inputs_never_return_partial_meshes() {
        let cancel = AtomicBool::new(true);
        assert_eq!(
            prepare(Geometry::Rectangle(rect()), Style::Fill, 0.1, &cancel).unwrap_err(),
            Error::Cancelled
        );
        cancel.store(false, Ordering::Relaxed);
        for width in [0., -1., 257., f64::NAN] {
            assert_eq!(
                prepare(
                    Geometry::Rectangle(rect()),
                    Style::Stroke(width),
                    0.1,
                    &cancel
                )
                .unwrap_err(),
                Error::InvalidGeometry
            );
        }
        let open = Path(vec![
            PathCommand::Move(p(0., 0.)),
            PathCommand::Line(p(10., 10.)),
        ]);
        assert_eq!(
            prepare(Geometry::Path(&open), Style::Fill, 0.1, &cancel).unwrap_err(),
            Error::InvalidGeometry
        );
        for tolerance in [0., f64::NAN, f64::INFINITY] {
            assert_eq!(
                prepare(Geometry::Rectangle(rect()), Style::Fill, tolerance, &cancel).unwrap_err(),
                Error::InvalidGeometry
            );
        }
        let mut output = Output {
            limits: Limits::CANVAS,
            vertices: vec![],
            indices: vec![],
            cancel: &cancel,
            failure: None,
        };
        assert_eq!(output.vertex(point(0., 0.)), Ok(VertexId(0)));
        cancel.store(true, Ordering::Relaxed);
        assert!(output.vertex(point(1., 1.)).is_err());
        assert_eq!(output.failure, Some(Error::Cancelled));
    }
    #[test]
    fn output_caps_apply_before_growth_and_invalid_indices_are_never_retained() {
        let cancel = AtomicBool::new(false);
        let mut output = Output {
            limits: Limits::CANVAS,
            vertices: vec![],
            indices: vec![],
            cancel: &cancel,
            failure: None,
        };
        for _ in 0..MAX_VERTICES {
            output.vertex(point(1., 2.)).unwrap();
        }
        assert!(output.vertex(point(1., 2.)).is_err());
        assert_eq!(output.vertices.len(), MAX_VERTICES);
        assert_eq!(output.failure, Some(Error::LimitExceeded));
        output.failure = None;
        for _ in 0..MAX_INDICES / 3 {
            output.add_triangle(VertexId(0), VertexId(1), VertexId(2));
        }
        output.add_triangle(VertexId(0), VertexId(1), VertexId(2));
        assert_eq!(output.indices.len(), MAX_INDICES);
        assert_eq!(output.failure, Some(Error::LimitExceeded));
        output.abort_geometry();
        output.failure = None;
        output.add_triangle(VertexId(0), VertexId(0), VertexId(0));
        assert!(output.indices.is_empty());
        assert_eq!(output.failure, Some(Error::Tessellation));
    }
}
