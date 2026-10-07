//! Bounded native text/image painting for admitted canvas snapshots. GPUI owns
//! font-system caches; our retained-line accounting is not a process RSS quota.
use crate::{
    asset_decode, asset_svg,
    canvas_paint::{FrameBudget, Placement},
    canvas_store::Snapshot,
    image_host,
};
use gpui::{App, Bounds, ContentMask, Pixels, ShapedLine, Window, point, px, size};
use gpuio_protocol::{
    canvas::{Point, Rect, Transform},
    canvas_scene::{Drawing, ResourceData, ResourceKey, Text},
    canvas_view::{Error, Viewport},
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    mem::size_of,
    rc::Rc,
    sync::Arc,
};

pub const MAX_TEXT_VARIANTS: usize = 512;
pub const MAX_TEXT_MEASUREMENTS: usize = gpuio_protocol::canvas_scene::MAX_ITEMS;
pub const MAX_IMAGE_VARIANTS: usize = 256;
pub const MAX_TEXT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_LINE_GLYPHS: usize = 32_768;
pub const MAX_PHYSICAL_FONT_SIZE: f32 = 512.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    /// Native image completion will wake the window; do not poll for it.
    Loading,
    /// This frame admitted its work allowance; request a subsequent frame.
    Deferred,
}
#[derive(Clone, Default)]
struct TextBudget(Rc<Cell<usize>>);
impl gpui::Global for TextBudget {}
struct Charge {
    budget: TextBudget,
    bytes: usize,
}
impl TextBudget {
    fn reserve(&self, bytes: usize) -> Option<Charge> {
        let next = self.0.get().checked_add(bytes)?;
        if next > MAX_TEXT_BYTES {
            return None;
        }
        self.0.set(next);
        Some(Charge {
            budget: self.clone(),
            bytes,
        })
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.budget.0.set(self.budget.0.get() - self.bytes);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TextKey {
    resource: ResourceKey,
    size: u32,
    color: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MeasureKey {
    resource: ResourceKey,
    size: u32,
}
struct Measurement {
    ink: Option<Bounds<Pixels>>,
    touched: u64,
    _charge: Charge,
}
struct Line {
    shaped: ShapedLine,
    ink: Option<Bounds<Pixels>>,
    glyphs: usize,
    logical_pixels: f64,
    touched: u64,
    _charge: Charge,
}
struct LineGeometry {
    ink: Option<Bounds<Pixels>>,
    glyphs: usize,
    logical_pixels: f64,
    bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ImageSize {
    Raster,
    Vector {
        width: u32,
        height: u32,
        density: u32,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ImageKey {
    resource: ResourceKey,
    size: ImageSize,
}

pub struct Content {
    snapshot: Arc<Snapshot>,
    resources: BTreeMap<ResourceKey, usize>,
    text_budget: TextBudget,
    lines: BTreeMap<TextKey, Line>,
    measurements: BTreeMap<MeasureKey, Measurement>,
    seen_measurements: BTreeSet<MeasureKey>,
    images: BTreeMap<ImageKey, image_host::Handle>,
    seen_text: BTreeSet<TextKey>,
    seen_images: BTreeSet<ImageKey>,
    touched: u64,
    shape_calls: usize,
    image_requests: usize,
}
impl Content {
    /// Recreate on mounted source changes. A publication must belong to this
    /// same acquired scene; resource history guarantees canonical key identity.
    pub fn new(snapshot: Arc<Snapshot>, cx: &mut App) -> Self {
        if !cx.has_global::<TextBudget>() {
            cx.set_global(TextBudget::default());
        }
        Self {
            resources: index(&snapshot),
            snapshot,
            text_budget: cx.global::<TextBudget>().clone(),
            lines: BTreeMap::new(),
            measurements: BTreeMap::new(),
            seen_measurements: BTreeSet::new(),
            images: BTreeMap::new(),
            seen_text: BTreeSet::new(),
            seen_images: BTreeSet::new(),
            touched: 0,
            shape_calls: 0,
            image_requests: 0,
        }
    }
    pub fn publish(&mut self, snapshot: Arc<Snapshot>) {
        if Arc::ptr_eq(&self.snapshot, &snapshot) {
            return;
        }
        if self.snapshot.generation != snapshot.generation {
            self.clear();
        }
        self.resources = index(&snapshot);
        self.measurements
            .retain(|key, _| self.resources.contains_key(&key.resource));
        self.lines
            .retain(|key, _| self.resources.contains_key(&key.resource));
        self.images
            .retain(|key, _| self.resources.contains_key(&key.resource));
        self.snapshot = snapshot;
    }
    /// Release mounted caches on hiding/disposal. The snapshot's resource leases
    /// remain owned by the scene/state until their corresponding owners drop.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.measurements.clear();
        self.seen_measurements.clear();
        self.images.clear();
        self.seen_text.clear();
        self.seen_images.clear();
    }
    pub fn begin_frame(&mut self) {
        self.seen_measurements.clear();
        self.seen_text.clear();
        self.seen_images.clear();
    }
    pub fn end_frame(&mut self) {
        self.images.retain(|key, _| self.seen_images.contains(key));
    }
    /// Paint Text/Image only, in scene order alongside admitted mesh draws. A
    /// shared FrameBudget must span all canvases in the window's frame.
    pub fn paint(
        &mut self,
        drawing: &Drawing,
        placement: &Placement<'_>,
        budget: &mut FrameBudget,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Status, Error> {
        let Some(clip) = placement.clip().map_err(paint_error)? else {
            return Ok(Status::Ready);
        };
        let clip = clip.intersect(&window.content_mask().bounds);
        if clip.size.width <= px(0.) || clip.size.height <= px(0.) {
            return Ok(Status::Ready);
        }
        match drawing {
            Drawing::Text(key, origin, color) => self.text(
                *key,
                *origin,
                *color as u32,
                placement,
                clip,
                budget,
                window,
                cx,
            ),
            Drawing::Image(key, rect) => {
                self.image(*key, *rect, placement, clip, budget, window, cx)
            }
            Drawing::Shape(_, _) => Err(Error::NativeFailure),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        resource: ResourceKey,
        origin: Point,
        color: u32,
        placement: &Placement<'_>,
        clip: Bounds<Pixels>,
        budget: &mut FrameBudget,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Status, Error> {
        let Some(&index) = self.resources.get(&resource) else {
            return Err(Error::NativeFailure);
        };
        let ResourceData::Text(text) = &self.snapshot.scene.resources[index].data else {
            return Err(Error::NativeFailure);
        };
        let logical_size = effective_font_size(
            text,
            placement.transform,
            placement.viewport,
            window.scale_factor(),
        )?;
        let key = TextKey {
            resource,
            size: logical_size.to_bits(),
            color,
        };
        let measure_key = MeasureKey {
            resource,
            size: logical_size.to_bits(),
        };
        if self.seen_measurements.len() == MAX_TEXT_MEASUREMENTS
            && !self.seen_measurements.contains(&measure_key)
        {
            return Err(Error::RenderLimit);
        }
        self.seen_measurements.insert(measure_key);
        self.touched = self.touched.saturating_add(1);
        let origin = placement.project_local(origin);
        let known_ink = self
            .measurements
            .get_mut(&measure_key)
            .map(|measure| {
                measure.touched = self.touched;
                measure.ink
            })
            .or_else(|| self.lines.get(&key).map(|line| line.ink));
        if let Some(ink) = known_ink
            && !visible(ink, origin, clip)
        {
            return Ok(Status::Ready);
        }
        if known_ink.is_some() {
            self.mark_visible(key)?;
        }
        if !self.lines.contains_key(&key) {
            // Borrow resource data again after cache maintenance (no text clone
            // is needed until an admitted shaping call).
            let ResourceData::Text(text) = &self.snapshot.scene.resources[index].data else {
                unreachable!()
            };
            if !budget.shape(text.value.len()) {
                return Ok(Status::Deferred);
            }
            self.shape_calls = self.shape_calls.saturating_add(1);
            let shaped = shape(text, logical_size, color, window)?;
            let LineGeometry {
                ink,
                glyphs,
                logical_pixels,
                bytes,
            } = line_geometry(&shaped, cx)?;
            if !self.measurements.contains_key(&measure_key) {
                while self.measurements.len() >= MAX_TEXT_MEASUREMENTS {
                    if !self.evict_measurement() {
                        return Err(Error::RenderLimit);
                    }
                }
                let charge = self.reserve_text(256)?;
                self.measurements.insert(
                    measure_key,
                    Measurement {
                        ink,
                        touched: self.touched,
                        _charge: charge,
                    },
                );
            }
            if !visible(ink, origin, clip) {
                return Ok(Status::Ready);
            }
            self.mark_visible(key)?;
            while self.lines.len() >= MAX_TEXT_VARIANTS {
                if !self.evict_line() {
                    return Err(Error::RenderLimit);
                }
            }
            let charge = self.reserve_text(bytes)?;
            self.lines.insert(
                key,
                Line {
                    shaped,
                    ink,
                    glyphs,
                    logical_pixels,
                    touched: self.touched,
                    _charge: charge,
                },
            );
        }
        let line = self.lines.get_mut(&key).expect("prepared line");
        line.touched = self.touched;
        let density = f64::from(window.scale_factor());
        budget
            .glyphs(line.glyphs, line.logical_pixels * density * density)
            .map_err(paint_error)?;
        let result = window.with_content_mask(Some(ContentMask { bounds: clip }), |window| {
            line.shaped.paint(
                origin,
                line.shaped.ascent + line.shaped.descent,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            )
        });
        result.map_err(|_| Error::NativeFailure)?;
        Ok(Status::Ready)
    }
    fn mark_visible(&mut self, key: TextKey) -> Result<(), Error> {
        if self.seen_text.len() == MAX_TEXT_VARIANTS && !self.seen_text.contains(&key) {
            return Err(Error::RenderLimit);
        }
        self.seen_text.insert(key);
        Ok(())
    }
    fn evict_line(&mut self) -> bool {
        let oldest = self
            .lines
            .iter()
            .filter(|(key, _)| !self.seen_text.contains(key))
            .min_by_key(|(_, line)| line.touched)
            .map(|(key, _)| *key);
        oldest.is_some_and(|key| self.lines.remove(&key).is_some())
    }
    fn evict_measurement(&mut self) -> bool {
        let oldest = self
            .measurements
            .iter()
            .filter(|(key, _)| !self.seen_measurements.contains(key))
            .min_by_key(|(_, value)| value.touched)
            .map(|(key, _)| *key);
        oldest.is_some_and(|key| self.measurements.remove(&key).is_some())
    }
    fn reserve_text(&mut self, bytes: usize) -> Result<Charge, Error> {
        loop {
            if let Some(charge) = self.text_budget.reserve(bytes) {
                return Ok(charge);
            }
            if !self.evict_line() && !self.evict_measurement() {
                return Err(Error::RenderLimit);
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn image(
        &mut self,
        resource: ResourceKey,
        rect: Rect,
        placement: &Placement<'_>,
        clip: Bounds<Pixels>,
        budget: &mut FrameBudget,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Status, Error> {
        let transform = placement.transform;
        if transform.b != 0. || transform.c != 0. || transform.a <= 0. || transform.d <= 0. {
            return Err(Error::NativeFailure);
        }
        let bounds = Bounds {
            origin: placement.project_local(Point {
                x: rect.x,
                y: rect.y,
            }),
            size: size(
                px((rect.width * transform.a * placement.viewport.zoom) as f32),
                px((rect.height * transform.d * placement.viewport.zoom) as f32),
            ),
        };
        let visible = bounds.intersect(&clip);
        if visible.size.width <= px(0.) || visible.size.height <= px(0.) {
            return Ok(Status::Ready);
        }
        let Some(&index) = self.resources.get(&resource) else {
            return Err(Error::NativeFailure);
        };
        let ResourceData::Image(id) = self.snapshot.scene.resources[index].data else {
            return Err(Error::NativeFailure);
        };
        let source = self
            .snapshot
            .images
            .get(&id)
            .ok_or(Error::UnavailableImage)?;
        let svg = source.source().format() == gpuio_protocol::asset::Format::Svg;
        let image_size = if svg {
            vector_size(bounds.size, window.scale_factor())?
        } else {
            ImageSize::Raster
        };
        let key = ImageKey {
            resource,
            size: image_size,
        };
        if self.seen_images.len() == MAX_IMAGE_VARIANTS && !self.seen_images.contains(&key) {
            return Err(Error::RenderLimit);
        }
        self.seen_images.insert(key);
        if !self.images.contains_key(&key) {
            if !budget.image_request() {
                return Ok(Status::Deferred);
            }
            // Keep still-useful variants until frame end, but never exceed the
            // mounted cache bound. Obsolete zoom sizes are dropped at frame end.
            if self.images.len() == MAX_IMAGE_VARIANTS {
                let stale = self
                    .images
                    .keys()
                    .find(|key| !self.seen_images.contains(key))
                    .copied()
                    .ok_or(Error::RenderLimit)?;
                self.images.remove(&stale);
            }
            let handle = match image_size {
                ImageSize::Raster => image_host::request(source.clone(), window, cx),
                ImageSize::Vector {
                    width,
                    height,
                    density,
                } => image_host::request_svg(
                    source.clone(),
                    asset_svg::Request {
                        size: asset_svg::Size::Exact(
                            asset_svg::RasterSize::new(width, height)
                                .map_err(|_| Error::RenderLimit)?,
                        ),
                        density: asset_svg::Density::new(f32::from_bits(density))
                            .map_err(|_| Error::RenderLimit)?,
                        fit: asset_svg::Fit::Fill,
                        tint: None,
                        corners: Default::default(),
                    },
                    window,
                    cx,
                ),
            }
            .map_err(image_error)?;
            self.images.insert(key, handle);
            self.image_requests = self.image_requests.saturating_add(1);
        }
        let handle = &self.images[&key];
        let Some(image) = image_host::image(handle, window, cx).map_err(image_error)? else {
            return Ok(Status::Loading);
        };
        budget.image().map_err(paint_error)?;
        window
            .with_content_mask(Some(ContentMask { bounds: clip }), |window| {
                window.paint_image(bounds, bounds, Default::default(), image, 0, false)
            })
            .map_err(|_| Error::NativeFailure)?;
        Ok(Status::Ready)
    }
}
#[cfg(feature = "native-tests")]
impl Content {
    pub fn statistics(&self) -> (usize, usize, usize, usize) {
        (
            self.lines.len(),
            self.images.len(),
            self.shape_calls,
            self.image_requests,
        )
    }
}
fn visible(ink: Option<Bounds<Pixels>>, origin: gpui::Point<Pixels>, clip: Bounds<Pixels>) -> bool {
    ink.is_some_and(|ink| {
        let intersection = Bounds {
            origin: origin + ink.origin,
            size: ink.size,
        }
        .intersect(&clip);
        intersection.size.width > px(0.) && intersection.size.height > px(0.)
    })
}
fn index(snapshot: &Snapshot) -> BTreeMap<ResourceKey, usize> {
    snapshot
        .scene
        .resources
        .iter()
        .enumerate()
        .map(|(index, resource)| (resource.key, index))
        .collect()
}
fn paint_error(error: crate::canvas_paint::Error) -> Error {
    match error {
        crate::canvas_paint::Error::LimitExceeded => Error::RenderLimit,
        crate::canvas_paint::Error::InvalidGeometry => Error::NativeFailure,
    }
}
fn image_error(error: image_host::Error) -> Error {
    match error {
        image_host::Error::ResourceLimit
        | image_host::Error::Decode(asset_decode::Error::ResourceLimit) => Error::RenderLimit,
        image_host::Error::Closed | image_host::Error::Decode(_) => Error::UnavailableImage,
        image_host::Error::WorkerFailed => Error::NativeFailure,
    }
}
fn effective_font_size(
    text: &Text,
    transform: Transform,
    viewport: Viewport,
    density: f32,
) -> Result<f32, Error> {
    if transform.b != 0. || transform.c != 0. || transform.a <= 0. || transform.a != transform.d {
        return Err(Error::NativeFailure);
    }
    let logical = (text.font_size * transform.a * viewport.zoom) as f32;
    if !logical.is_finite()
        || logical <= 0.
        || !density.is_finite()
        || !(0.25..=16.).contains(&density)
        || logical * density > MAX_PHYSICAL_FONT_SIZE
    {
        return Err(Error::RenderLimit);
    }
    Ok(logical)
}
fn shape(text: &Text, font_size: f32, color: u32, window: &Window) -> Result<ShapedLine, Error> {
    let family = if text.font_family == "system" {
        ".SystemUIFont"
    } else {
        &text.font_family
    };
    let mut font = gpui::font(family.to_owned());
    font.weight = gpui::FontWeight(text.font_weight as f32);
    let run = gpui::TextRun {
        len: text.value.len(),
        font,
        color: gpui::rgba(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        window
            .text_system()
            .shape_line(text.value.clone().into(), px(font_size), &[run], None)
    }))
    .map_err(|_| Error::NativeFailure)
}
fn line_geometry(line: &ShapedLine, cx: &App) -> Result<LineGeometry, Error> {
    let glyphs = line
        .runs
        .iter()
        .try_fold(0usize, |total, run| total.checked_add(run.glyphs.len()))
        .ok_or(Error::RenderLimit)?;
    if glyphs > MAX_LINE_GLYPHS {
        return Err(Error::RenderLimit);
    }
    if ![line.width(), line.ascent, line.descent]
        .into_iter()
        .all(|v| f32::from(v).is_finite() && v >= px(0.))
    {
        return Err(Error::NativeFailure);
    }
    let mut extent: Option<Bounds<Pixels>> = None;
    let mut pixels = 0.;
    let mut bytes = size_of::<ShapedLine>()
        + line.text.len()
        + 2048
        + line.runs.capacity() * size_of::<gpui::ShapedRun>();
    for run in &line.runs {
        bytes += run.glyphs.capacity() * size_of::<gpui::ShapedGlyph>();
        let font_bounds = cx.text_system().bounding_box(run.font_id, line.font_size);
        if ![
            font_bounds.origin.x,
            font_bounds.origin.y,
            font_bounds.size.width,
            font_bounds.size.height,
        ]
        .into_iter()
        .all(|v| f32::from(v).is_finite())
            || font_bounds.size.width < px(0.)
            || font_bounds.size.height < px(0.)
        {
            return Err(Error::NativeFailure);
        }
        pixels += f64::from(f32::from(font_bounds.size.width))
            * f64::from(f32::from(font_bounds.size.height))
            * run.glyphs.len() as f64;
        for glyph in &run.glyphs {
            if ![glyph.position.x, glyph.position.y]
                .into_iter()
                .all(|value| f32::from(value).is_finite())
            {
                return Err(Error::NativeFailure);
            }
            let bounds = Bounds {
                origin: font_bounds.origin + glyph.position + point(px(0.), line.ascent),
                size: font_bounds.size,
            };
            extent = Some(extent.map_or(bounds, |previous| previous.union(&bounds)));
        }
    }
    Ok(LineGeometry {
        ink: extent,
        glyphs,
        logical_pixels: pixels,
        bytes,
    })
}
fn vector_size(size: gpui::Size<Pixels>, density: f32) -> Result<ImageSize, Error> {
    if !density.is_finite() || !(0.25..=16.).contains(&density) {
        return Err(Error::RenderLimit);
    }
    let width = (f32::from(size.width) * density).ceil();
    let height = (f32::from(size.height) * density).ceil();
    if !width.is_finite() || !height.is_finite() || width < 1. || height < 1. {
        return Err(Error::RenderLimit);
    }
    let raster =
        asset_svg::RasterSize::new(width as u32, height as u32).map_err(|_| Error::RenderLimit)?;
    Ok(ImageSize::Vector {
        width: raster.width(),
        height: raster.height(),
        density: density.to_bits(),
    })
}
#[cfg(feature = "native-tests")]
pub fn retained_text_bytes(cx: &App) -> usize {
    cx.try_global::<TextBudget>()
        .map_or(0, |budget| budget.0.get())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_text_reservations_release_only_after_their_owner_drops() {
        let budget = TextBudget::default();
        let other = budget.clone();
        let first = budget.reserve(MAX_TEXT_BYTES - 1).unwrap();
        let second = other.reserve(1).unwrap();
        assert!(budget.reserve(1).is_none());
        drop(first);
        assert_eq!(other.0.get(), 1);
        let reused = budget.reserve(MAX_TEXT_BYTES - 1).unwrap();
        drop((second, reused));
        assert_eq!(budget.0.get(), 0);
        assert!(budget.reserve(usize::MAX).is_none());
    }
    #[test]
    fn text_quality_applies_uniform_scale_zoom_and_actual_density_once() {
        let text = Text {
            value: "label".into(),
            font_family: "system".into(),
            font_size: 20.,
            font_weight: 400,
        };
        let transform = Transform {
            a: 2.,
            d: 2.,
            tx: 40.,
            ..Transform::IDENTITY
        };
        let viewport = Viewport {
            zoom: 1.5,
            ..Viewport::default()
        };
        assert_eq!(effective_font_size(&text, transform, viewport, 2.), Ok(60.));
        assert_eq!(
            effective_font_size(&text, transform, viewport, 16.),
            Err(Error::RenderLimit)
        );
        assert_eq!(
            effective_font_size(&text, Transform { b: 1., ..transform }, viewport, 2.),
            Err(Error::NativeFailure)
        );
        assert_eq!(
            effective_font_size(&text, transform, viewport, f32::NAN),
            Err(Error::RenderLimit)
        );
        assert!(
            effective_font_size(
                &text,
                Transform::IDENTITY,
                Viewport {
                    zoom: 0.05,
                    ..viewport
                },
                1.
            )
            .is_ok()
        );
    }
    #[test]
    fn svg_quality_uses_ceil_physical_size_and_rejects_excessive_rasterization() {
        assert_eq!(
            vector_size(size(px(10.25), px(5.125)), 2.),
            Ok(ImageSize::Vector {
                width: 21,
                height: 11,
                density: 2f32.to_bits()
            })
        );
        for size in [
            size(px(1e9), px(1.)),
            size(px(f32::INFINITY), px(1.)),
            size(px(0.), px(1.)),
        ] {
            assert_eq!(vector_size(size, 1.), Err(Error::RenderLimit));
        }
        assert_eq!(
            vector_size(size(px(1.), px(1.)), 0.),
            Err(Error::RenderLimit)
        );
    }
}
