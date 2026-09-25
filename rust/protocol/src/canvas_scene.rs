//! Canonical immutable scene schema. Painting and resource leases live in native.
use crate::{ResourceId, canvas::*};
use binprot::macros::BinProtWrite;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ITEMS: usize = 20_000;
pub const MAX_RESOURCES: usize = 4096;
pub const MAX_PATH_COMMANDS: usize = 65_536;
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;
pub const MAX_INTERACTIVE_ITEMS: usize = 2048;
pub const MAX_CLIPS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub struct ResourceKey {
    pub id: i64,
    pub generation: i64,
}
impl ResourceKey {
    pub fn is_valid(self) -> bool {
        self.id > 0 && self.generation > 0
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Text {
    pub value: String,
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: i64,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum ResourceData {
    Path(Path),
    Text(Text),
    Image(ResourceId),
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Resource {
    pub key: ResourceKey,
    pub data: ResourceData,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Stroke {
    pub color: i64,
    pub width: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Paint {
    pub fill: Option<i64>,
    pub stroke: Option<Stroke>,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Shape {
    Rectangle(Rect),
    Ellipse(Rect),
    Path(ResourceKey),
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Drawing {
    Shape(Shape, Paint),
    Text(ResourceKey, Point, i64),
    Image(ResourceKey, Rect),
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Interaction {
    pub label: String,
    pub hit_region: HitRegion,
    pub draggable: bool,
    pub activatable: bool,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Item {
    pub id: i64,
    pub transform: Transform,
    pub clips: Vec<Rect>,
    pub drawing: Drawing,
    pub interaction: Option<Interaction>,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Scene {
    pub version: i64,
    pub description: String,
    pub resources: Vec<Resource>,
    pub items: Vec<Item>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError {
    UnsupportedVersion,
    LimitExceeded,
    InvalidIdentity,
    DuplicateIdentity,
    InvalidGeometry,
    InvalidText,
    MissingResource,
    StaleResource,
    WrongResourceKind,
    UnsupportedTransform,
}

fn color(value: i64) -> bool {
    (0..=0xffff_ffff).contains(&value)
}
fn label(value: &str, maximum: usize) -> bool {
    value.bytes().any(|byte| !matches!(byte, 9..=13 | 32))
        && value.len() <= maximum
        && !value.contains('\0')
}
impl Paint {
    pub fn is_valid(self) -> bool {
        (self.fill.is_some() || self.stroke.is_some())
            && self.fill.is_none_or(color)
            && self.stroke.is_none_or(|s| {
                color(s.color) && s.width.is_finite() && s.width > 0. && s.width <= 256.
            })
    }
}
impl Text {
    pub fn is_valid(&self) -> bool {
        !self.value.is_empty()
            && self.value.len() <= 16_384
            && !self.value.contains(['\0', '\r', '\n'])
            && label(&self.font_family, 128)
            && self.font_size.is_finite()
            && (4. ..=256.).contains(&self.font_size)
            && (100..=900).contains(&self.font_weight)
    }
}

fn resolve<'a>(
    resources: &BTreeMap<i64, &'a Resource>,
    key: ResourceKey,
) -> Result<&'a ResourceData, ValidationError> {
    if !key.is_valid() {
        return Err(ValidationError::InvalidIdentity);
    }
    let resource = resources
        .get(&key.id)
        .ok_or(ValidationError::MissingResource)?;
    if resource.key != key {
        return Err(ValidationError::StaleResource);
    }
    Ok(&resource.data)
}
fn rect_points(rect: Rect) -> [Point; 4] {
    [
        Point {
            x: rect.x,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y,
        },
        Point {
            x: rect.x,
            y: rect.y + rect.height,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
    ]
}
fn transformed_rect(rect: Rect, transform: Transform) -> bool {
    rect.is_valid()
        && rect_points(rect)
            .iter()
            .all(|p| transform.apply(*p).is_valid())
}
fn axis_aligned_positive(transform: Transform) -> bool {
    transform.b == 0. && transform.c == 0. && transform.a > 0. && transform.d > 0.
}

impl Scene {
    /// Hit a validated scene in world coordinates, back-to-front. Decorative
    /// items do not intercept input. Viewport conversion/capture belongs to the
    /// native widget; this function neither paints nor calls OCaml.
    pub fn hit_test(&self, point: Point) -> Option<i64> {
        if !point.is_valid() {
            return None;
        }
        self.items.iter().rev().find_map(|item| {
            item.interaction.as_ref().and_then(|interaction| {
                interaction
                    .hit_region
                    .hit(item.transform, &item.clips, point)
                    .then_some(item.id)
            })
        })
    }

    /// Validate domain invariants after bounded decoding, before atomic publication.
    /// This does not resolve application asset handles or perform tessellation.
    pub fn validate(&self) -> Result<(), ValidationError> {
        use ValidationError::*;
        if self.version != 1 {
            return Err(UnsupportedVersion);
        }
        if self.items.len() > MAX_ITEMS || self.resources.len() > MAX_RESOURCES {
            return Err(LimitExceeded);
        }
        if !label(&self.description, 4096) {
            return Err(InvalidText);
        }
        let mut resources = BTreeMap::new();
        let mut path_bounds = BTreeMap::new();
        let mut path_commands = 0;
        let mut text_bytes = self.description.len();
        for resource in &self.resources {
            if !resource.key.is_valid() {
                return Err(InvalidIdentity);
            }
            if resources.insert(resource.key.id, resource).is_some() {
                return Err(DuplicateIdentity);
            }
            match &resource.data {
                ResourceData::Path(path) => {
                    if !path.is_valid() {
                        return Err(InvalidGeometry);
                    }
                    path_commands += path.0.len();
                    if path_commands > MAX_PATH_COMMANDS {
                        return Err(LimitExceeded);
                    }
                    // Cache the control-point hull once. Each item then checks four
                    // corners instead of traversing a shared path on every use.
                    let mut bounds = (
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    );
                    let mut extend = |p: &Point| {
                        bounds.0 = bounds.0.min(p.x);
                        bounds.1 = bounds.1.min(p.y);
                        bounds.2 = bounds.2.max(p.x);
                        bounds.3 = bounds.3.max(p.y);
                    };
                    for command in &path.0 {
                        match command {
                            PathCommand::Move(p) | PathCommand::Line(p) => extend(p),
                            PathCommand::Quadratic(c, p) => {
                                extend(c);
                                extend(p);
                            }
                            PathCommand::Cubic(a, b, p) => {
                                extend(a);
                                extend(b);
                                extend(p);
                            }
                            PathCommand::Close => {}
                        }
                    }
                    path_bounds.insert(resource.key.id, (bounds, path.is_closed()));
                }
                ResourceData::Text(text) => {
                    if !text.is_valid() {
                        return Err(InvalidText);
                    }
                    text_bytes += text.value.len() + text.font_family.len();
                }
                ResourceData::Image(_) => {}
            }
            if text_bytes > MAX_TEXT_BYTES {
                return Err(LimitExceeded);
            }
        }
        let mut identities = BTreeSet::new();
        let mut interactive = 0;
        for item in &self.items {
            if item.id <= 0 {
                return Err(InvalidIdentity);
            }
            if !identities.insert(item.id) {
                return Err(DuplicateIdentity);
            }
            if !item.transform.is_valid() {
                return Err(InvalidGeometry);
            }
            if item.clips.len() > MAX_CLIPS {
                return Err(LimitExceeded);
            }
            if !item.clips.iter().all(|r| r.is_valid()) {
                return Err(InvalidGeometry);
            }
            if let Some(interaction) = &item.interaction {
                interactive += 1;
                if interactive > MAX_INTERACTIVE_ITEMS {
                    return Err(LimitExceeded);
                }
                if !label(&interaction.label, 1024) {
                    return Err(InvalidText);
                }
                text_bytes += interaction.label.len();
                if text_bytes > MAX_TEXT_BYTES {
                    return Err(LimitExceeded);
                }
                if !interaction.hit_region.is_valid() {
                    return Err(InvalidGeometry);
                }
                let hit_in_domain = match &interaction.hit_region {
                    HitRegion::Rectangle(rect) | HitRegion::Ellipse(rect) => {
                        transformed_rect(*rect, item.transform)
                    }
                    HitRegion::Polygon(points) => {
                        points.iter().all(|p| item.transform.apply(*p).is_valid())
                    }
                };
                if !hit_in_domain {
                    return Err(InvalidGeometry);
                }
            }
            match &item.drawing {
                Drawing::Shape(shape, paint) => {
                    if !paint.is_valid() {
                        return Err(InvalidGeometry);
                    }
                    match shape {
                        Shape::Rectangle(rect) | Shape::Ellipse(rect) => {
                            if !transformed_rect(*rect, item.transform) {
                                return Err(InvalidGeometry);
                            }
                        }
                        Shape::Path(key) => {
                            let ResourceData::Path(_) = resolve(&resources, *key)? else {
                                return Err(WrongResourceKind);
                            };
                            let &((left, top, right, bottom), closed) =
                                path_bounds.get(&key.id).expect("validated path bounds");
                            if paint.fill.is_some() && !closed {
                                return Err(InvalidGeometry);
                            }
                            if ![
                                Point { x: left, y: top },
                                Point { x: right, y: top },
                                Point { x: left, y: bottom },
                                Point {
                                    x: right,
                                    y: bottom,
                                },
                            ]
                            .iter()
                            .all(|p| item.transform.apply(*p).is_valid())
                            {
                                return Err(InvalidGeometry);
                            }
                        }
                    }
                }
                Drawing::Text(key, origin, foreground) => {
                    if !matches!(resolve(&resources, *key)?, ResourceData::Text(_)) {
                        return Err(WrongResourceKind);
                    }
                    if !origin.is_valid()
                        || !item.transform.apply(*origin).is_valid()
                        || !color(*foreground)
                    {
                        return Err(InvalidGeometry);
                    }
                    if !axis_aligned_positive(item.transform)
                        || item.transform.a != item.transform.d
                    {
                        return Err(UnsupportedTransform);
                    }
                }
                Drawing::Image(key, rect) => {
                    if !matches!(resolve(&resources, *key)?, ResourceData::Image(_)) {
                        return Err(WrongResourceKind);
                    }
                    if !transformed_rect(*rect, item.transform) {
                        return Err(InvalidGeometry);
                    }
                    if !axis_aligned_positive(item.transform) {
                        return Err(UnsupportedTransform);
                    }
                }
            }
        }
        Ok(())
    }
}
