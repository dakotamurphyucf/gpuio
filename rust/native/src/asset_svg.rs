//! In-memory SVG/SVGZ rasterization for the bounded image workers. Document
//! resources never open files or URLs. Native system fonts are discovered lazily,
//! as with GPUI text; this cache is separate from image pixel accounting.
use crate::asset_decode::{self, Decoded, Error};
use gpuio_protocol::asset::{Format, Source};
pub use gpuio_protocol::image::ImageFit as Fit;
use image::ImageEncoder;
use resvg::{tiny_skia, usvg};
use std::{
    borrow::Cow,
    io::Read,
    sync::{Arc, LazyLock, Mutex},
};

const MAX_XML_BYTES: usize = 32 * 1024 * 1024;
const MAX_NODES: u32 = 100_000;
const MAX_DEPTH: usize = 128;
const MAX_EMBEDDED: usize = 32;
const MAX_EMBEDDED_DEPTH: usize = 8;
const MAX_EMBEDDED_BYTES: usize = 64 * 1024 * 1024;

/// Validated physical-pixel raster size. Independent of logical layout size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RasterSize {
    width: u32,
    height: u32,
}
impl RasterSize {
    pub fn new(width: u32, height: u32) -> Result<Self, Error> {
        asset_decode::pixel_bytes(width, height)?;
        Ok(Self { width, height })
    }
    pub fn width(self) -> u32 {
        self.width
    }
    pub fn height(self) -> u32 {
        self.height
    }
}
/// Natural dimensions or a physical-pixel viewport. Fitting happens within the
/// viewport before rasterization, so Cover retains only viewport-sized pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size {
    #[default]
    Intrinsic,
    Exact(RasterSize),
}
/// Positive finite device pixels per logical pixel. Exact float bits form a
/// stable native cache key; no floating-point value crosses the view protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Density(u32);
impl Density {
    pub fn new(value: f32) -> Result<Self, Error> {
        if !value.is_finite() || value <= 0. || value > 16. {
            return Err(Error::InvalidData);
        }
        Ok(Self(value.to_bits()))
    }
    pub fn get(self) -> f32 {
        f32::from_bits(self.0)
    }
}
impl Default for Density {
    fn default() -> Self {
        Self(1.0f32.to_bits())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Request {
    pub size: Size,
    pub density: Density,
    pub fit: Fit,
    /// RRGGBBAA for a monochrome icon; None preserves all SVG colors. The SVG's
    /// resulting alpha mask is multiplied by the supplied color's alpha.
    pub tint: Option<u32>,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            size: Size::Intrinsic,
            density: Density::default(),
            fit: Fit::Contain,
            tint: None,
        }
    }
}
#[derive(Default)]
struct Budget {
    xml_bytes: usize,
    nodes: u32,
    embedded: usize,
    embedded_pixels: usize,
    normalized_bytes: usize,
    nesting: usize,
    failure: Option<Error>,
}
type Shared = Arc<Mutex<Budget>>;
fn fail(budget: &Shared, error: Error) {
    budget.lock().unwrap().failure.get_or_insert(error);
}
fn xml_bytes(bytes: &[u8]) -> Result<Cow<'_, [u8]>, Error> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok(Cow::Borrowed(bytes));
    }
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(MAX_XML_BYTES as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|_| Error::InvalidData)?;
    if decoded.len() > MAX_XML_BYTES {
        return Err(Error::ResourceLimit);
    }
    Ok(Cow::Owned(decoded))
}
struct Nesting(Shared);
impl Drop for Nesting {
    fn drop(&mut self) {
        self.0.lock().unwrap().nesting -= 1;
    }
}
fn parse(bytes: &[u8], options: &usvg::Options<'_>, budget: &Shared) -> Result<usvg::Tree, Error> {
    {
        let mut state = budget.lock().unwrap();
        if state.nesting >= MAX_EMBEDDED_DEPTH {
            return Err(Error::ResourceLimit);
        }
        state.nesting += 1;
    }
    let _nesting = Nesting(budget.clone());
    let bytes = xml_bytes(bytes)?;
    let remaining_nodes = {
        let mut state = budget.lock().unwrap();
        if bytes.len() > MAX_XML_BYTES - state.xml_bytes {
            return Err(Error::ResourceLimit);
        }
        state.xml_bytes += bytes.len();
        MAX_NODES - state.nodes
    };
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidData)?;
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: remaining_nodes,
            ..Default::default()
        },
    )
    .map_err(|error| match error {
        roxmltree::Error::NodesLimitReached => Error::ResourceLimit,
        _ => Error::InvalidData,
    })?;
    let mut nodes = 0;
    for node in doc.descendants() {
        nodes += 1;
        if node.ancestors().take(MAX_DEPTH + 1).count() > MAX_DEPTH {
            return Err(Error::ResourceLimit);
        }
    }
    budget.lock().unwrap().nodes += nodes;
    let tree = usvg::Tree::from_xmltree(&doc, options).map_err(|error| match error {
        usvg::Error::ElementsLimitReached => Error::ResourceLimit,
        _ => Error::InvalidData,
    })?;
    if let Some(error) = budget.lock().unwrap().failure {
        return Err(error);
    }
    Ok(tree)
}
fn embedded(
    mime: &str,
    data: Arc<Vec<u8>>,
    options: &usvg::Options<'_>,
    budget: &Shared,
) -> Result<usvg::ImageKind, Error> {
    {
        let mut state = budget.lock().unwrap();
        if state.embedded >= MAX_EMBEDDED {
            return Err(Error::ResourceLimit);
        }
        state.embedded += 1;
    }
    if mime == "image/svg+xml" {
        return parse(&data, options, budget).map(usvg::ImageKind::SVG);
    }
    let format = match mime {
        "image/png" => Format::Png,
        "image/jpeg" | "image/jpg" => Format::Jpeg,
        "image/gif" => Format::Gif,
        "image/webp" => Format::Webp,
        "text/plain" => match image::guess_format(&data) {
            Ok(image::ImageFormat::Png) => Format::Png,
            Ok(image::ImageFormat::Jpeg) => Format::Jpeg,
            Ok(image::ImageFormat::Gif) => Format::Gif,
            Ok(image::ImageFormat::WebP) => Format::Webp,
            _ => return parse(&data, options, budget).map(usvg::ImageKind::SVG),
        },
        _ => return Err(Error::Unsupported),
    };
    let source = Source::new(format, (*data).clone()).map_err(|_| Error::ResourceLimit)?;
    let decoded = asset_decode::raster(&source)?;
    {
        let mut state = budget.lock().unwrap();
        if decoded.pixel_bytes > MAX_EMBEDDED_BYTES - state.embedded_pixels {
            return Err(Error::ResourceLimit);
        }
        state.embedded_pixels += decoded.pixel_bytes;
    }
    // Normalize to a validated static RGBA PNG. resvg otherwise ignores corrupt
    // embedded codec data rather than failing the enclosing image. SVG image
    // elements are static; animated raster sources use their first frame.
    let size = decoded.image.size(0);
    let mut rgba = decoded
        .image
        .as_bytes(0)
        .ok_or(Error::InvalidData)?
        .to_vec();
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let mut normalized = Vec::new();
    image::codecs::png::PngEncoder::new(&mut normalized).write_image(
        &rgba,
        u32::from(size.width),
        u32::from(size.height),
        image::ExtendedColorType::Rgba8,
    )?;
    {
        let mut state = budget.lock().unwrap();
        if normalized.len() > MAX_EMBEDDED_BYTES - state.normalized_bytes {
            return Err(Error::ResourceLimit);
        }
        state.normalized_bytes += normalized.len();
    }
    Ok(usvg::ImageKind::PNG(Arc::new(normalized)))
}
fn options(budget: &Shared, fonts: Option<Arc<usvg::fontdb::Database>>) -> usvg::Options<'static> {
    static SYSTEM_FONTS: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
        let mut fonts = usvg::fontdb::Database::new();
        fonts.load_system_fonts();
        Arc::new(fonts)
    });
    let resources = budget.clone();
    let external = budget.clone();
    let failures = budget.clone();
    let default_selector = usvg::FontResolver::default_font_selector();
    let select_font = Box::new(
        move |font: &usvg::Font, db: &mut Arc<usvg::fontdb::Database>| {
            if db.is_empty() {
                *db = fonts.clone().unwrap_or_else(|| SYSTEM_FONTS.clone());
            }
            let selected = default_selector(font, db)
                .or_else(|| {
                    db.query(&usvg::fontdb::Query {
                        families: &[usvg::fontdb::Family::SansSerif],
                        ..Default::default()
                    })
                })
                .or_else(|| db.faces().next().map(|face| face.id));
            if selected.is_none() {
                fail(&failures, Error::Unsupported);
            }
            selected
        },
    );
    usvg::Options {
        font_family: "sans-serif".into(),
        font_resolver: usvg::FontResolver {
            select_font,
            ..Default::default()
        },
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(move |_, _| {
                fail(&external, Error::Unsupported);
                None
            }),
            resolve_data: Box::new(move |mime, data, options| {
                match embedded(mime, data, options, &resources) {
                    Ok(image) => Some(image),
                    Err(error) => {
                        fail(&resources, error);
                        None
                    }
                }
            }),
        },
        ..Default::default()
    }
}
fn tree_limits(group: &usvg::Group, depth: usize, count: &mut usize) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::ResourceLimit);
    }
    for node in group.children() {
        *count += 1;
        if *count > MAX_NODES as usize {
            return Err(Error::ResourceLimit);
        }
        if let usvg::Node::Group(group) = node {
            tree_limits(group, depth + 1, count)?;
        }
        let mut result = Ok(());
        node.subroots(|group| {
            if result.is_ok() {
                result = tree_limits(group, depth + 1, count);
            }
        });
        result?;
    }
    Ok(())
}
/// Rasterize on a bounded background worker. Strict input/output and tree-count
/// checks do not impose a hard RSS bound on font discovery, parsing, filters or
/// codec working buffers. The result contains straight-alpha BGRA for GPUI.
pub fn render(source: &Source, request: Request) -> Result<Decoded, Error> {
    render_with_fonts(source, request, None)
}
fn render_with_fonts(
    source: &Source,
    request: Request,
    fonts: Option<Arc<usvg::fontdb::Database>>,
) -> Result<Decoded, Error> {
    if source.format() != Format::Svg {
        return Err(Error::Unsupported);
    }
    let budget = Shared::default();
    let options = options(&budget, fonts);
    let tree = parse(source.as_bytes(), &options, &budget)?;
    tree_limits(tree.root(), 0, &mut 0)?;
    let size = match request.size {
        Size::Intrinsic => {
            let size = tree.size();
            let width = (size.width() * request.density.get()).ceil();
            let height = (size.height() * request.density.get()).ceil();
            if width > asset_decode::MAX_DIMENSION as f32
                || height > asset_decode::MAX_DIMENSION as f32
            {
                return Err(Error::ResourceLimit);
            }
            RasterSize::new(width as u32, height as u32)?
        }
        Size::Exact(size) => size,
    };
    let pixel_bytes = asset_decode::pixel_bytes(size.width, size.height)?;
    let mut pixmap = tiny_skia::Pixmap::new(size.width, size.height).ok_or(Error::ResourceLimit)?;
    let source_width = tree.size().width();
    let source_height = tree.size().height();
    let width = size.width as f32;
    let height = size.height as f32;
    let (scale_x, scale_y) = match request.fit {
        Fit::Fill => (width / source_width, height / source_height),
        Fit::Contain | Fit::Cover | Fit::ScaleDown | Fit::None => {
            let contain = (width / source_width).min(height / source_height);
            let scale = match request.fit {
                Fit::Contain => contain,
                Fit::Cover => (width / source_width).max(height / source_height),
                Fit::ScaleDown => contain.min(request.density.get()),
                Fit::None => request.density.get(),
                Fit::Fill => unreachable!(),
            };
            (scale, scale)
        }
    };
    let (x, y) = if request.fit == Fit::None {
        // Pinned GPUI ObjectFit::None anchors at the element origin. ScaleDown,
        // including its unscaled case, is centered instead.
        (0., 0.)
    } else {
        (
            (width - source_width * scale_x) / 2.,
            (height - source_height * scale_y) / 2.,
        )
    };
    if ![scale_x, scale_y, x, y]
        .iter()
        .all(|value| value.is_finite())
        || scale_x <= 0.
        || scale_y <= 0.
    {
        return Err(Error::ResourceLimit);
    }
    let transform = tiny_skia::Transform::from_row(scale_x, 0., 0., scale_y, x, y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut bytes = pixmap.take();
    for pixel in bytes.chunks_exact_mut(4) {
        let alpha = pixel[3] as u32;
        if let Some(tint) = request.tint {
            pixel.copy_from_slice(&[
                (tint >> 8) as u8,
                (tint >> 16) as u8,
                (tint >> 24) as u8,
                ((alpha * (tint & 255) + 127) / 255) as u8,
            ]);
        } else if alpha > 0 {
            let straight =
                |value: u8| ((u32::from(value) * 255 + alpha / 2) / alpha).min(255) as u8;
            let (red, green, blue) = (straight(pixel[0]), straight(pixel[1]), straight(pixel[2]));
            pixel.copy_from_slice(&[blue, green, red, alpha as u8]);
        }
    }
    let buffer =
        image::RgbaImage::from_raw(size.width, size.height, bytes).ok_or(Error::InvalidData)?;
    Ok(Decoded {
        image: Arc::new(gpui::RenderImage::new(vec![image::Frame::new(buffer)])),
        pixel_bytes,
        frame_count: 1,
    })
}

#[cfg(test)]
#[path = "asset_svg_test.rs"]
mod tests;
