//! Shared meshes and bounded scene preparation. No window, font, image decode or
//! OCaml work occurs here; the host schedules this on bounded native workers.
use crate::canvas_mesh::{self as mesh, Geometry, Mesh, Style};
use gpuio_protocol::{
    canvas::{Point, Rect, Transform},
    canvas_scene::{Drawing, ResourceData, ResourceKey, Scene, Shape},
};
use std::{
    collections::BTreeMap,
    mem::size_of,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

pub const MAX_MESHES: usize = 4096;
pub const MAX_DRAW_VERTICES: usize = 1_048_576;
pub const MAX_RETAINED_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidScene,
    InvalidScale,
    LimitExceeded,
    Cancelled,
    Tessellation,
}
impl From<mesh::Error> for Error {
    fn from(value: mesh::Error) -> Self {
        match value {
            mesh::Error::InvalidGeometry => Self::InvalidScene,
            mesh::Error::LimitExceeded => Self::LimitExceeded,
            mesh::Error::Cancelled => Self::Cancelled,
            mesh::Error::Tessellation => Self::Tessellation,
        }
    }
}

#[derive(Clone, Default)]
pub struct Budget(Arc<AtomicUsize>);
impl Budget {
    pub fn used_bytes(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
    fn reserve(&self, bytes: usize) -> Result<Charge, Error> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(bytes)
                    .filter(|next| *next <= MAX_RETAINED_BYTES)
            })
            .map_err(|_| Error::LimitExceeded)?;
        Ok(Charge {
            budget: self.clone(),
            bytes,
        })
    }
}
struct Charge {
    budget: Budget,
    bytes: usize,
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.budget.0.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

/// Device scale is bounded separately from the scene's zoom. A host must pass
/// the actual value; it must not clamp an unsupported scale silently.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality {
    scale: f64,
}
impl Quality {
    pub fn new(zoom: f64, device_scale: f64) -> Result<Self, Error> {
        if !zoom.is_finite()
            || !(0.05..=64.).contains(&zoom)
            || !device_scale.is_finite()
            || !(0.25..=16.).contains(&device_scale)
        {
            return Err(Error::InvalidScale);
        }
        Ok(Self {
            scale: zoom * device_scale,
        })
    }
    fn tolerance(self, transform: Transform) -> i16 {
        // Frobenius norm bounds maximum stretch, including shears/reflections.
        // Round DOWN to a power of two: shared meshes are never coarser than
        // the requested quarter-device-pixel flattening tolerance.
        let stretch =
            (transform.a.powi(2) + transform.b.powi(2) + transform.c.powi(2) + transform.d.powi(2))
                .sqrt();
        (0.25 / (self.scale * stretch))
            .log2()
            .floor()
            .clamp(-29., 19.) as i16
    }
}

pub struct RetainedMesh {
    mesh: Mesh,
    _charge: Charge,
}
impl RetainedMesh {
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
}

pub struct ShapeMeshes {
    /// Rectangles/ellipses share geometry independent of their local origin.
    /// Apply this offset before the item's local-to-world transform.
    pub origin: Point,
    pub fill: Option<Arc<RetainedMesh>>,
    pub stroke: Option<Arc<RetainedMesh>>,
}

pub struct Plan {
    /// One slot per scene item; text and image items have no tessellated shape.
    pub shapes: Vec<Option<ShapeMeshes>>,
    pub draw_vertices: usize,
    pub unique_meshes: usize,
    _charge: Charge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ShapeKey {
    Rectangle(u64, u64),
    Ellipse(u64, u64),
    Path(ResourceKey),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    shape: ShapeKey,
    stroke: Option<u64>,
    tolerance: i16,
}

struct Build<'a> {
    cache: BTreeMap<Key, Arc<RetainedMesh>>,
    budget: &'a Budget,
    cancel: &'a AtomicBool,
    vertices: usize,
}
impl Build<'_> {
    fn mesh(
        &mut self,
        shape: ShapeKey,
        geometry: Geometry<'_>,
        style: Style,
        tolerance: i16,
    ) -> Result<Arc<RetainedMesh>, Error> {
        let key = Key {
            shape,
            stroke: match style {
                Style::Fill => None,
                Style::Stroke(width) => Some(width.to_bits()),
            },
            // Straight geometry has no flattening error and reuses all scales.
            tolerance: if matches!(shape, ShapeKey::Rectangle(..)) {
                0
            } else {
                tolerance
            },
        };
        let mesh = if let Some(mesh) = self.cache.get(&key) {
            mesh.clone()
        } else {
            if self.cache.len() == MAX_MESHES {
                return Err(Error::LimitExceeded);
            }
            let mesh = mesh::prepare(
                geometry,
                style,
                2f64.powi(i32::from(key.tolerance)),
                self.cancel,
            )?;
            // Includes Arc/cache metadata. One in-progress mesh is additionally
            // bounded by canvas_mesh input/output limits before reservation.
            let charge = self.budget.reserve(mesh.retained_bytes() + 512)?;
            let mesh = Arc::new(RetainedMesh {
                mesh,
                _charge: charge,
            });
            self.cache.insert(key, mesh.clone());
            mesh
        };
        let vertices = mesh.mesh().triangle_count() * 3;
        if vertices > MAX_DRAW_VERTICES - self.vertices {
            return Err(Error::LimitExceeded);
        }
        self.vertices += vertices;
        Ok(mesh)
    }
}

pub fn prepare(
    scene: &Scene,
    quality: Quality,
    budget: &Budget,
    cancel: &AtomicBool,
) -> Result<Plan, Error> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    scene.validate().map_err(|_| Error::InvalidScene)?;
    let mut shapes = Vec::with_capacity(scene.items.len());
    let charge = budget.reserve(shapes.capacity() * size_of::<Option<ShapeMeshes>>() + 4096)?;
    let resources: BTreeMap<_, _> = scene.resources.iter().map(|r| (r.key, &r.data)).collect();
    let mut build = Build {
        cache: BTreeMap::new(),
        budget,
        cancel,
        vertices: 0,
    };
    for item in &scene.items {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let Drawing::Shape(shape, paint) = &item.drawing else {
            shapes.push(None);
            continue;
        };
        let (key, geometry, origin) = match shape {
            Shape::Rectangle(rect) | Shape::Ellipse(rect) => {
                let normalized = Rect {
                    x: 0.,
                    y: 0.,
                    ..*rect
                };
                let (key, geometry) = match shape {
                    Shape::Rectangle(_) => (
                        ShapeKey::Rectangle(rect.width.to_bits(), rect.height.to_bits()),
                        Geometry::Rectangle(normalized),
                    ),
                    Shape::Ellipse(_) => (
                        ShapeKey::Ellipse(rect.width.to_bits(), rect.height.to_bits()),
                        Geometry::Ellipse(normalized),
                    ),
                    Shape::Path(_) => unreachable!(),
                };
                (
                    key,
                    geometry,
                    Point {
                        x: rect.x,
                        y: rect.y,
                    },
                )
            }
            Shape::Path(key) => {
                let Some(ResourceData::Path(path)) = resources.get(key) else {
                    return Err(Error::InvalidScene);
                };
                (
                    ShapeKey::Path(*key),
                    Geometry::Path(path),
                    Point { x: 0., y: 0. },
                )
            }
        };
        let tolerance = quality.tolerance(item.transform);
        let fill = paint
            .fill
            .map(|_| build.mesh(key, geometry, Style::Fill, tolerance))
            .transpose()?;
        let stroke = paint
            .stroke
            .map(|stroke| build.mesh(key, geometry, Style::Stroke(stroke.width), tolerance))
            .transpose()?;
        shapes.push(Some(ShapeMeshes {
            origin,
            fill,
            stroke,
        }));
    }
    Ok(Plan {
        shapes,
        draw_vertices: build.vertices,
        unique_meshes: build.cache.len(),
        _charge: charge,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::canvas_scene::{Item, Paint, Stroke};
    fn quality() -> Quality {
        Quality::new(1., 2.).unwrap()
    }
    fn scene(count: usize) -> Scene {
        Scene {
            version: 1,
            description: "Mesh workload".into(),
            resources: vec![],
            items: (0..count)
                .map(|index| Item {
                    id: index as i64 + 1,
                    transform: Transform::IDENTITY,
                    clips: vec![],
                    interaction: None,
                    drawing: Drawing::Shape(
                        Shape::Rectangle(Rect {
                            x: (index % 100) as f64 * 20.,
                            y: (index / 100) as f64 * 10.,
                            width: 20.,
                            height: 10.,
                        }),
                        Paint {
                            fill: Some(0x102030ff),
                            stroke: None,
                        },
                    ),
                })
                .collect(),
        }
    }
    #[test]
    fn twenty_thousand_rectangles_share_one_mesh_but_charge_draw_expansion() {
        let budget = Budget::default();
        let started = std::time::Instant::now();
        let plan = prepare(&scene(20_000), quality(), &budget, &AtomicBool::new(false)).unwrap();
        assert_eq!(plan.shapes.len(), 20_000);
        assert_eq!(plan.unique_meshes, 1);
        assert_eq!(plan.draw_vertices, 120_000);
        let first = plan.shapes[0].as_ref().unwrap().fill.as_ref().unwrap();
        let last = plan.shapes[19_999].as_ref().unwrap().fill.as_ref().unwrap();
        assert!(Arc::ptr_eq(first, last));
        assert_eq!(
            plan.shapes[101].as_ref().unwrap().origin,
            Point { x: 20., y: 10. }
        );
        assert!(budget.used_bytes() < 2 * 1024 * 1024);
        eprintln!(
            "canvas plan: 20k rectangles, 1 mesh, 120k draw vertices, {} charged bytes, {:?}",
            budget.used_bytes(),
            started.elapsed()
        );
        drop(plan);
        assert_eq!(budget.used_bytes(), 0);
    }
    #[test]
    fn stroke_width_and_curve_accuracy_are_cache_identity() {
        let mut scene = scene(3);
        for (index, item) in scene.items.iter_mut().enumerate() {
            item.drawing = Drawing::Shape(
                Shape::Ellipse(Rect {
                    x: 0.,
                    y: 0.,
                    width: 100.,
                    height: 100.,
                }),
                Paint {
                    fill: None,
                    stroke: Some(Stroke {
                        color: index as i64,
                        width: if index == 2 { 2. } else { 1. },
                    }),
                },
            );
        }
        scene.items[1].transform.a = 2.;
        let budget = Budget::default();
        let plan = prepare(&scene, quality(), &budget, &AtomicBool::new(false)).unwrap();
        assert_eq!(plan.unique_meshes, 3);
        drop(plan);
        assert_eq!(budget.used_bytes(), 0);
    }
    #[test]
    fn unique_mesh_admission_and_cancellation_release_partial_plans() {
        let mut scene = scene(MAX_MESHES + 1);
        for (index, item) in scene.items.iter_mut().enumerate() {
            let Drawing::Shape(Shape::Rectangle(rect), _) = &mut item.drawing else {
                unreachable!();
            };
            rect.width = index as f64 + 1.;
        }
        let budget = Budget::default();
        assert!(matches!(
            prepare(&scene, quality(), &budget, &AtomicBool::new(false)),
            Err(Error::LimitExceeded)
        ));
        assert_eq!(budget.used_bytes(), 0);
        assert!(matches!(
            prepare(&scene, quality(), &budget, &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
        assert_eq!(budget.used_bytes(), 0);
    }
    #[test]
    fn shared_geometry_does_not_bypass_expanded_output_admission() {
        let mut scene = scene(20_000);
        for item in &mut scene.items {
            item.drawing = Drawing::Shape(
                Shape::Ellipse(Rect {
                    x: 0.,
                    y: 0.,
                    width: 1000.,
                    height: 1000.,
                }),
                Paint {
                    fill: Some(0),
                    stroke: None,
                },
            );
        }
        let budget = Budget::default();
        assert!(matches!(
            prepare(&scene, quality(), &budget, &AtomicBool::new(false)),
            Err(Error::LimitExceeded)
        ));
        assert_eq!(budget.used_bytes(), 0);
    }
    #[test]
    fn shared_mesh_readers_keep_their_charge_until_final_drop() {
        let budget = Budget::default();
        let plan = prepare(&scene(1), quality(), &budget, &AtomicBool::new(false)).unwrap();
        let reader = plan.shapes[0]
            .as_ref()
            .unwrap()
            .fill
            .as_ref()
            .unwrap()
            .clone();
        let before = budget.used_bytes();
        drop(plan);
        assert!(budget.used_bytes() > 0 && budget.used_bytes() < before);
        let remaining = MAX_RETAINED_BYTES - budget.used_bytes();
        let pressure = budget.reserve(remaining).unwrap();
        assert!(matches!(
            prepare(&scene(1), quality(), &budget, &AtomicBool::new(false)),
            Err(Error::LimitExceeded)
        ));
        drop(pressure);
        drop(reader);
        assert_eq!(budget.used_bytes(), 0);
    }
}
