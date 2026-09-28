//! Bounded chart mesh preparation and GPUI painting. Source reduction, curves,
//! polygon tessellation and brush resolution occur once per prepared revision.
use crate::{
    canvas_mesh as mesh,
    canvas_paint::{self, FrameBudget as MeshFrameBudget, Placement},
    chart_geometry as geometry,
};
use gpui::{Bounds, Pixels, Window, point, px, size};
use gpuio_protocol::{
    canvas::{Path as MeshPath, PathCommand, Point as MeshPoint, Transform},
    canvas_view::Viewport,
    chart_data::Data,
    chart_options::{Options, Orientation},
    chart_sampling::Policy,
    chart_style::Style,
};
use std::{
    f64::consts::{FRAC_PI_2, PI},
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_QUADS: usize = 300_000;
/// One shared budget per window frame, passed through every visible chart.
/// Admission reserves the whole plan before any of its primitives are emitted.
pub struct FrameBudget {
    meshes: MeshFrameBudget,
    vertices_left: usize,
    quads_left: usize,
}
impl Default for FrameBudget {
    fn default() -> Self {
        Self {
            meshes: MeshFrameBudget::default(),
            vertices_left: MAX_VERTICES,
            quads_left: MAX_QUADS,
        }
    }
}
impl FrameBudget {
    fn reserve(&mut self, vertices: usize, quads: usize) -> Result<(), Error> {
        if vertices > self.vertices_left || quads > self.quads_left {
            return Err(Error::RenderLimit);
        }
        self.vertices_left -= vertices;
        self.quads_left -= quads;
        Ok(())
    }
    pub fn used_vertices(&self) -> usize {
        MAX_VERTICES - self.vertices_left
    }
    pub fn used_quads(&self) -> usize {
        MAX_QUADS - self.quads_left
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Cancelled,
    RenderLimit,
    NativeFailure,
}
struct MeshDraw {
    mesh: mesh::Mesh,
    color: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    width: f64,
    height: f64,
    scale: f64,
}
impl Layout {
    pub(crate) fn dimensions(self) -> (f64, f64, f64) {
        (self.width, self.height, self.scale)
    }
    pub fn new(width: f64, height: f64, scale: f64) -> Result<Self, Error> {
        if width.is_finite()
            && height.is_finite()
            && scale.is_finite()
            && width > 0.
            && height > 0.
            && width <= 32768.
            && height <= 32768.
            && (0.5..=8.).contains(&scale)
        {
            Ok(Self {
                width,
                height,
                scale,
            })
        } else {
            Err(Error::InvalidInput)
        }
    }
}
#[derive(Clone, Copy)]
enum Draw {
    Mesh(usize),
    Quad(usize),
}
#[derive(Clone, Copy)]
struct Quad {
    rect: geometry::Rect,
    radius: f64,
    color: u32,
    gradient_end: Option<u32>,
    horizontal: bool,
    border: Option<(f64, u32)>,
}
/// Keep the exact immutable chart snapshot beside this plan. Publication/release
/// invalidation and application-wide admission belong to the mounted owner.
pub struct Prepared {
    geometry: geometry::Plan,
    meshes: Vec<MeshDraw>,
    quads: Vec<Quad>,
    draws: Vec<Draw>,
    vertices: usize,
}
impl Prepared {
    pub fn geometry(&self) -> &geometry::Plan {
        &self.geometry
    }
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + self.geometry.retained_bytes()
            + self.meshes.capacity() * size_of::<MeshDraw>()
            + self
                .meshes
                .iter()
                .map(|m| m.mesh.retained_bytes())
                .sum::<usize>()
            + self.quads.capacity() * size_of::<Quad>()
            + self.draws.capacity() * size_of::<Draw>()
    }
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }
    pub fn vertices(&self) -> usize {
        self.vertices
    }
    pub fn quad_count(&self) -> usize {
        self.quads.len()
    }
    /// The plot must have its prepared logical size. Layout owners request a new
    /// plan for resize; drawing never stretches a stale dataset's geometry.
    pub fn paint(
        &self,
        bounds: Bounds<Pixels>,
        budget: &mut FrameBudget,
        window: &mut Window,
    ) -> Result<(), Error> {
        if ![
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        ]
        .into_iter()
        .all(|v| f32::from(v).is_finite())
            || (f64::from(f32::from(bounds.size.width)) - self.geometry.width).abs() > 0.01
            || (f64::from(f32::from(bounds.size.height)) - self.geometry.height).abs() > 0.01
        {
            return Err(Error::InvalidInput);
        }
        if !bounds.intersects(&window.content_mask().bounds) {
            return Ok(());
        }
        budget.reserve(self.vertices, self.quads.len())?;
        let placement = Placement {
            origin: MeshPoint { x: 0., y: 0. },
            transform: Transform::IDENTITY,
            viewport: Viewport::default(),
            bounds,
            clips: &[],
        };
        // Preparation already admitted the entire plan; no partial plan is ever
        // exposed for rendering. Canvas's shared mesh painter also culls clips.
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            for draw in &self.draws {
                let q = match *draw {
                    Draw::Mesh(index) => {
                        let draw = &self.meshes[index];
                        canvas_paint::paint(
                            &draw.mesh,
                            &placement,
                            draw.color,
                            &mut budget.meshes,
                            window,
                        )
                        .map_err(|_| Error::RenderLimit)?;
                        continue;
                    }
                    Draw::Quad(index) => &self.quads[index],
                };
                let rect = Bounds::new(
                    point(
                        bounds.origin.x + px(q.rect.left as f32),
                        bounds.origin.y + px(q.rect.top as f32),
                    ),
                    size(
                        px((q.rect.right - q.rect.left) as f32),
                        px((q.rect.bottom - q.rect.top) as f32),
                    ),
                );
                if !rect.intersects(&window.content_mask().bounds) {
                    continue;
                }
                let background = match q.gradient_end {
                    None => gpui::Background::from(gpui::rgba(q.color)),
                    Some(end) => gpui::linear_gradient(
                        if q.horizontal { 90. } else { 180. },
                        gpui::linear_color_stop(gpui::rgba(q.color), 0.),
                        gpui::linear_color_stop(gpui::rgba(end), 1.),
                    ),
                };
                let (border, color) = q.border.unwrap_or((0., 0));
                window.paint_quad(gpui::quad(
                    rect,
                    px(q.radius as f32),
                    background,
                    px(border as f32),
                    gpui::rgba(color),
                    gpui::BorderStyle::Solid,
                ));
            }
            Ok(())
        })
    }
}
fn point_wire(p: geometry::Point) -> MeshPoint {
    MeshPoint { x: p.x, y: p.y }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn alpha(color: u32, opacity: f64) -> u32 {
    (color & 0xffffff00) | ((f64::from(color & 255) * opacity).round() as u32)
}
fn path(commands: &[geometry::Command]) -> MeshPath {
    MeshPath(
        commands
            .iter()
            .map(|c| match *c {
                geometry::Command::Move(p) => PathCommand::Move(point_wire(p)),
                geometry::Command::Line(p) => PathCommand::Line(point_wire(p)),
                geometry::Command::Cubic(a, b, c) => {
                    PathCommand::Cubic(point_wire(a), point_wire(b), point_wire(c))
                }
                geometry::Command::Close => PathCommand::Close,
            })
            .collect(),
    )
}
struct Build<'a> {
    meshes: Vec<MeshDraw>,
    quads: Vec<Quad>,
    draws: Vec<Draw>,
    vertices: usize,
    bytes: usize,
    tolerance: f64,
    cancel: &'a AtomicBool,
}
impl Build<'_> {
    fn mesh(&mut self, path: &MeshPath, style: mesh::Style, color: u32) -> Result<(), Error> {
        check(self.cancel)?;
        let prepared =
            mesh::prepare_chart(path, style, self.tolerance, self.cancel).map_err(|error| {
                match error {
                    mesh::Error::Cancelled => Error::Cancelled,
                    mesh::Error::LimitExceeded => Error::RenderLimit,
                    mesh::Error::InvalidGeometry => Error::RenderLimit,
                    mesh::Error::Tessellation => Error::NativeFailure,
                }
            })?;
        let vertices = prepared.triangle_count() * 3;
        if vertices > MAX_VERTICES - self.vertices {
            return Err(Error::RenderLimit);
        }
        let bytes = prepared.retained_bytes() + size_of::<MeshDraw>() + size_of::<Draw>();
        if bytes > MAX_BYTES - self.bytes {
            return Err(Error::RenderLimit);
        }
        self.vertices += vertices;
        self.bytes += bytes;
        self.draws.push(Draw::Mesh(self.meshes.len()));
        self.meshes.push(MeshDraw {
            mesh: prepared,
            color,
        });
        Ok(())
    }
    fn quad(
        &mut self,
        rect: geometry::Rect,
        radius: f64,
        color: u32,
        gradient_end: Option<u32>,
        horizontal: bool,
        border: Option<(f64, u32)>,
    ) -> Result<(), Error> {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return Ok(());
        }
        if self.quads.len() == MAX_QUADS
            || size_of::<Quad>() + size_of::<Draw>() > MAX_BYTES - self.bytes
        {
            return Err(Error::RenderLimit);
        }
        self.bytes += size_of::<Quad>() + size_of::<Draw>();
        self.draws.push(Draw::Quad(self.quads.len()));
        self.quads.push(Quad {
            rect,
            radius: radius
                .min((rect.right - rect.left) / 2.)
                .min((rect.bottom - rect.top) / 2.),
            color,
            gradient_end,
            horizontal,
            border,
        });
        Ok(())
    }
}
fn arc(out: &mut Vec<PathCommand>, center: geometry::Point, radius: f64, start: f64, end: f64) {
    let count = ((end - start).abs() / FRAC_PI_2).ceil().max(1.) as usize;
    let step = (end - start) / count as f64;
    for i in 0..count {
        let a = start + i as f64 * step;
        let b = a + step;
        let k = 4. / 3. * ((b - a) / 4.).tan();
        let first = MeshPoint {
            x: center.x + radius * (a.cos() - k * a.sin()),
            y: center.y + radius * (a.sin() + k * a.cos()),
        };
        let second = MeshPoint {
            x: center.x + radius * (b.cos() + k * b.sin()),
            y: center.y + radius * (b.sin() - k * b.cos()),
        };
        out.push(PathCommand::Cubic(
            first,
            second,
            MeshPoint {
                x: center.x + radius * b.cos(),
                y: center.y + radius * b.sin(),
            },
        ));
    }
}
fn wedge(center: geometry::Point, inner: f64, outer: f64, start: f64, end: f64) -> MeshPath {
    let mut out = vec![PathCommand::Move(MeshPoint {
        x: center.x + outer * start.cos(),
        y: center.y + outer * start.sin(),
    })];
    arc(&mut out, center, outer, start, end);
    if inner > 0. {
        out.push(PathCommand::Line(MeshPoint {
            x: center.x + inner * end.cos(),
            y: center.y + inner * end.sin(),
        }));
        arc(&mut out, center, inner, end, start);
    } else {
        out.push(PathCommand::Line(point_wire(center)));
    }
    out.push(PathCommand::Close);
    MeshPath(out)
}
fn ribbon(
    a: geometry::Point,
    b: geometry::Point,
    c: geometry::Point,
    d: geometry::Point,
) -> MeshPath {
    let middle = (a.x + c.x) / 2.;
    MeshPath(vec![
        PathCommand::Move(point_wire(a)),
        PathCommand::Cubic(
            MeshPoint { x: middle, y: a.y },
            MeshPoint { x: middle, y: c.y },
            point_wire(c),
        ),
        PathCommand::Line(point_wire(d)),
        PathCommand::Cubic(
            MeshPoint { x: middle, y: d.y },
            MeshPoint { x: middle, y: b.y },
            point_wire(b),
        ),
        PathCommand::Close,
    ])
}
/// Device scale selects tessellation accuracy only. All coordinates, strokes and
/// corners stay in logical pixels; GPUI applies device scale exactly once.
pub fn prepare(
    data: &Data,
    policy: Policy,
    options: &Options,
    style: &Style,
    layout: Layout,
    cancel: &AtomicBool,
) -> Result<Prepared, Error> {
    check(cancel)?;
    let Layout {
        width,
        height,
        scale,
    } = layout;
    if !style.is_valid() {
        return Err(Error::InvalidInput);
    }
    let geometry =
        geometry::prepare(data, policy, options, width, height, cancel).map_err(|e| match e {
            geometry::Error::InvalidInput => Error::InvalidInput,
            geometry::Error::Cancelled => Error::Cancelled,
            geometry::Error::RenderLimit => Error::RenderLimit,
        })?;
    let mut build = Build {
        meshes: vec![],
        quads: Vec::with_capacity(geometry.marks.len().min(MAX_QUADS)),
        draws: vec![],
        vertices: 0,
        bytes: geometry.retained_bytes(),
        tolerance: 0.25 / scale,
        cancel,
    };
    // All grid segments share a retained stroke mesh; axes are distinct from
    // grid visibility and use their own brush.
    let segments = |lines: &[(geometry::Point, geometry::Point)]| {
        MeshPath(
            lines
                .iter()
                .flat_map(|(a, b)| {
                    [
                        PathCommand::Move(point_wire(*a)),
                        PathCommand::Line(point_wire(*b)),
                    ]
                })
                .collect(),
        )
    };
    if !geometry.grid.is_empty() {
        build.mesh(
            &segments(&geometry.grid),
            mesh::Style::Stroke(1.),
            style.grid_color as u32,
        )?;
    }
    let horizontal = options.cartesian.orientation == Orientation::Horizontal
        && matches!(
            data.contents,
            gpuio_protocol::chart_data::Contents::Cartesian(_)
        );
    if geometry.x_domain.is_some() {
        let a = geometry::Point { x: 0., y: height };
        let b = geometry::Point {
            x: width,
            y: height,
        };
        let c = geometry::Point { x: 0., y: 0. };
        let mut lines = vec![];
        if options.axes.x {
            lines.push(if horizontal { (c, a) } else { (a, b) });
        }
        if options.axes.y {
            lines.push(if horizontal { (a, b) } else { (c, a) });
        }
        if !lines.is_empty() {
            build.mesh(
                &segments(&lines),
                mesh::Style::Stroke(1.),
                style.axis_color as u32,
            )?;
        }
    }
    let layers = geometry
        .paths
        .iter()
        .map(|p| p.layer)
        .chain(geometry.marks.iter().map(|m| m.layer))
        .max()
        .map_or(0, |n| n + 1);
    let sankey = matches!(
        data.contents,
        gpuio_protocol::chart_data::Contents::Sankey(..)
    );
    // Sankey ribbons are emitted before every node; other plots preserve source
    // layer order across meshes and native quads.
    let passes = if sankey { 2 } else { layers };
    for pass in 0..passes {
        for shape in geometry.paths.iter().filter(|s| s.layer == pass) {
            let color = style.color(shape.layer);
            build.mesh(
                &path(&shape.commands),
                if shape.fill {
                    mesh::Style::Fill
                } else {
                    mesh::Style::Stroke(style.stroke_width)
                },
                if shape.fill {
                    alpha(color, style.area_opacity)
                } else {
                    color
                },
            )?;
        }
        for (index, mark) in geometry.marks.iter().enumerate().filter(|(_, m)| {
            if sankey {
                matches!(m.shape, geometry::Shape::Ribbon { .. }) == (pass == 0)
            } else {
                m.layer == pass
            }
        }) {
            if index & 255 == 0 {
                check(cancel)?;
            }
            let color = style.color(mark.layer);
            match mark.shape {
                geometry::Shape::Dot {
                    center,
                    visible: true,
                } => {
                    let r = style.point_radius;
                    build.quad(
                        geometry::Rect {
                            left: center.x - r,
                            right: center.x + r,
                            top: center.y - r,
                            bottom: center.y + r,
                        },
                        r,
                        color,
                        None,
                        false,
                        None,
                    )?;
                }
                geometry::Shape::Dot { visible: false, .. } => {}
                geometry::Shape::Bar(rect) => build.quad(
                    rect,
                    style.bar_radius,
                    color,
                    style.gradient_end.map(|c| c as u32),
                    horizontal,
                    None,
                )?,
                geometry::Shape::Node(rect) => build.quad(rect, 1., color, None, false, None)?,
                geometry::Shape::Candle {
                    center,
                    left,
                    right,
                    open,
                    high,
                    low,
                    close,
                } => {
                    let rise = close < open;
                    let color = style.color(if rise { 0 } else { 1 });
                    build.quad(
                        geometry::Rect {
                            left: center - 0.5,
                            right: center + 0.5,
                            top: high,
                            bottom: open.min(close),
                        },
                        0.,
                        color,
                        None,
                        false,
                        None,
                    )?;
                    build.quad(
                        geometry::Rect {
                            left: center - 0.5,
                            right: center + 0.5,
                            top: open.max(close),
                            bottom: low,
                        },
                        0.,
                        color,
                        None,
                        false,
                        None,
                    )?;
                    let rect = geometry::Rect {
                        left,
                        right,
                        top: open.min(close),
                        bottom: open.max(close),
                    };
                    if open == close {
                        build.quad(
                            geometry::Rect {
                                top: open - 0.5,
                                bottom: open + 0.5,
                                ..rect
                            },
                            0.,
                            color,
                            None,
                            false,
                            None,
                        )?;
                    } else {
                        build.quad(
                            rect,
                            0.,
                            if rise { 0 } else { color },
                            None,
                            false,
                            rise.then_some((style.stroke_width.min((right - left) / 2.), color)),
                        )?;
                    }
                }
                geometry::Shape::Wedge {
                    center,
                    inner,
                    outer,
                    start,
                    end,
                } => {
                    if outer > 0. && end > start && end - start <= 2. * PI + 1e-8 {
                        build.mesh(
                            &wedge(center, inner, outer, start, end),
                            mesh::Style::Fill,
                            color,
                        )?;
                    }
                }
                geometry::Shape::Ribbon {
                    start_top,
                    start_bottom,
                    end_top,
                    end_bottom,
                } => {
                    build.mesh(
                        &ribbon(start_top, start_bottom, end_top, end_bottom),
                        mesh::Style::Fill,
                        alpha(color, 0.5),
                    )?;
                }
            }
        }
    }
    check(cancel)?;
    let prepared = Prepared {
        geometry,
        meshes: build.meshes,
        quads: build.quads,
        draws: build.draws,
        vertices: build.vertices,
    };
    if prepared.retained_bytes() > MAX_BYTES {
        return Err(Error::RenderLimit);
    }
    Ok(prepared)
}

#[cfg(feature = "native-canvas-tests")]
pub(crate) mod native_test;
#[cfg(test)]
mod tests;
