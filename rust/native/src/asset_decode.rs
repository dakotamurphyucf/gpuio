//! In-memory decoding. Call from a bounded worker, never from native paint.
//! Output limits are strict checks before retention. Codec-internal allocation
//! limits are best-effort, as documented by the pinned image library.
use gpuio_protocol::asset::{Format, Source};
use image::{AnimationDecoder, DynamicImage, Frame, ImageDecoder, ImageError, ImageReader};
use std::{io::Cursor, sync::Arc};

pub const MAX_DIMENSION: u32 = 16_384;
pub const MAX_PIXEL_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_FRAMES: usize = 120;
const MAX_DECODER_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidData,
    Unsupported,
    ResourceLimit,
}
impl From<ImageError> for Error {
    fn from(error: ImageError) -> Self {
        match error {
            ImageError::Limits(_) => Self::ResourceLimit,
            ImageError::Unsupported(_) => Self::Unsupported,
            ImageError::Decoding(_)
            | ImageError::Encoding(_)
            | ImageError::Parameter(_)
            | ImageError::IoError(_) => Self::InvalidData,
        }
    }
}

/// Pixels ready for GPUI, not yet uploaded to a window's sprite atlas.
/// Owning caches must charge [Self::pixel_bytes] until the last reader drops.
/// This object does not retain encoded source bytes.
pub struct Decoded {
    pub image: Arc<gpui::RenderImage>,
    pub pixel_bytes: usize,
    pub frame_count: usize,
}

fn limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODER_BYTES);
    limits
}

pub(crate) fn pixel_bytes(width: u32, height: u32) -> Result<usize, Error> {
    if width == 0 || height == 0 {
        return Err(Error::InvalidData);
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::ResourceLimit);
    }
    let bytes = u64::from(width) * u64::from(height) * 4;
    if bytes > MAX_PIXEL_BYTES as u64 {
        return Err(Error::ResourceLimit);
    }
    Ok(bytes as usize)
}

fn bgra(frame: &mut Frame) {
    for pixel in frame.buffer_mut().chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

fn decoded(frames: Vec<Frame>, bytes: usize) -> Result<Decoded, Error> {
    if frames.is_empty() {
        return Err(Error::InvalidData);
    }
    Ok(Decoded {
        frame_count: frames.len(),
        image: Arc::new(gpui::RenderImage::new(frames)),
        pixel_bytes: bytes,
    })
}

fn static_image(mut decoder: impl ImageDecoder) -> Result<Decoded, Error> {
    let (width, height) = decoder.dimensions();
    let bytes = pixel_bytes(width, height)?;
    // DynamicImage allocates the native color depth before RGBA8 conversion.
    // Check it separately: e.g. 16-bit pixels cost more than the final texture.
    if decoder.total_bytes() > MAX_DECODER_BYTES {
        return Err(Error::ResourceLimit);
    }
    decoder.set_limits(limits())?;
    let orientation = decoder.orientation()?;
    let mut pixels = DynamicImage::from_decoder(decoder)?;
    pixels.apply_orientation(orientation);
    let mut frame = Frame::new(pixels.into_rgba8());
    bgra(&mut frame);
    decoded(vec![frame], bytes)
}

fn gif(source: &Source) -> Result<Decoded, Error> {
    let mut decoder = image::codecs::gif::GifDecoder::new(Cursor::new(source.as_bytes()))?;
    decoder.set_limits(limits())?;
    let (width, height) = decoder.dimensions();
    let canvas_bytes = pixel_bytes(width, height)?;
    let mut frames = Vec::new();
    let mut total = 0;
    // Do not collect unbounded frames or silently skip broken frames. The
    // iterator keeps its own compositing canvas; returned frames are full RGBA.
    for frame in decoder.into_frames() {
        let mut frame = frame?;
        if frames.len() >= MAX_FRAMES || canvas_bytes > MAX_PIXEL_BYTES - total {
            return Err(Error::ResourceLimit);
        }
        if frame.buffer().dimensions() != (width, height) {
            return Err(Error::InvalidData);
        }
        total += canvas_bytes;
        bgra(&mut frame);
        frames.push(frame);
    }
    decoded(frames, total)
}

/// Decode the pinned raster families with explicit declared format. GIF keeps
/// frames/delays; PNG/WebP use the pinned GPUI static-image behavior. SVG needs
/// a separate size/theme-aware rasterization path and is not implemented here.
pub fn raster(source: &Source) -> Result<Decoded, Error> {
    let format = match source.format() {
        Format::Png => image::ImageFormat::Png,
        Format::Jpeg => image::ImageFormat::Jpeg,
        Format::Webp => image::ImageFormat::WebP,
        Format::Gif => return gif(source),
        Format::Bmp => image::ImageFormat::Bmp,
        Format::Tiff => image::ImageFormat::Tiff,
        Format::Ico => image::ImageFormat::Ico,
        Format::Pnm => image::ImageFormat::Pnm,
        Format::Svg => return Err(Error::Unsupported),
    };
    let mut reader = ImageReader::with_format(Cursor::new(source.as_bytes()), format);
    reader.limits(limits());
    static_image(reader.into_decoder()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Delay, Rgb, RgbImage, Rgba, RgbaImage};

    fn source(format: Format, bytes: Vec<u8>) -> Source {
        Source::new(format, bytes).unwrap()
    }
    fn animated(count: usize) -> Source {
        let mut bytes = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
            for index in 0..count {
                encoder
                    .encode_frame(Frame::from_parts(
                        RgbaImage::from_pixel(2, 1, Rgba([index as u8, 7, 211, 255])),
                        0,
                        0,
                        Delay::from_numer_denom_ms(30 + index as u32 * 10, 1),
                    ))
                    .unwrap();
            }
        }
        source(Format::Gif, bytes)
    }

    #[test]
    fn all_raster_families_produce_gpui_bgra_pixels() {
        for (format, codec) in [
            (Format::Png, image::ImageFormat::Png),
            (Format::Jpeg, image::ImageFormat::Jpeg),
            (Format::Webp, image::ImageFormat::WebP),
            (Format::Gif, image::ImageFormat::Gif),
            (Format::Bmp, image::ImageFormat::Bmp),
            (Format::Tiff, image::ImageFormat::Tiff),
            (Format::Ico, image::ImageFormat::Ico),
            (Format::Pnm, image::ImageFormat::Pnm),
        ] {
            let mut bytes = Cursor::new(Vec::new());
            let pixels = if matches!(format, Format::Jpeg | Format::Pnm) {
                DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 1, Rgb([220, 11, 45])))
            } else {
                DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 1, Rgba([220, 11, 45, 255])))
            };
            pixels.write_to(&mut bytes, codec).unwrap();
            let decoded = raster(&source(format, bytes.into_inner()))
                .unwrap_or_else(|error| panic!("{format:?}: {error:?}"));
            assert_eq!(decoded.pixel_bytes, 8, "{format:?}");
            assert_eq!(decoded.frame_count, 1);
            let bytes = decoded.image.as_bytes(0).unwrap();
            assert_eq!(bytes.len(), 8);
            for pixel in bytes.chunks_exact(4) {
                // JPEG is lossy. Every other codec must preserve exact pixels.
                let tolerance = if format == Format::Jpeg { 3 } else { 0 };
                for (actual, expected) in pixel.iter().zip([45u8, 11, 220, 255]) {
                    assert!(
                        actual.abs_diff(expected) <= tolerance,
                        "{format:?}: {pixel:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn gif_preserves_full_frames_and_delays_and_rejects_excess_frames() {
        let decoded = raster(&animated(2)).unwrap();
        assert_eq!(decoded.frame_count, 2);
        assert_eq!(decoded.pixel_bytes, 16);
        assert_eq!(
            decoded.image.as_bytes(0).unwrap(),
            [211, 7, 0, 255, 211, 7, 0, 255]
        );
        assert_eq!(
            decoded.image.as_bytes(1).unwrap(),
            [211, 7, 1, 255, 211, 7, 1, 255]
        );
        assert_eq!(decoded.image.delay(0).numer_denom_ms(), (30, 1));
        assert_eq!(decoded.image.delay(1).numer_denom_ms(), (40, 1));
        assert_eq!(
            raster(&animated(MAX_FRAMES)).unwrap().frame_count,
            MAX_FRAMES
        );
        assert!(matches!(
            raster(&animated(MAX_FRAMES + 1)),
            Err(Error::ResourceLimit)
        ));
        let mut truncated = animated(2).as_bytes().to_vec();
        truncated.truncate(truncated.len() - 10);
        assert!(
            matches!(
                raster(&source(Format::Gif, truncated)),
                Err(Error::InvalidData)
            ),
            "a valid first frame must not hide a broken later frame"
        );
    }

    #[test]
    fn malformed_and_oversized_headers_fail_without_output_publication() {
        for format in [
            Format::Png,
            Format::Jpeg,
            Format::Webp,
            Format::Gif,
            Format::Bmp,
            Format::Tiff,
            Format::Ico,
            Format::Pnm,
        ] {
            assert!(raster(&source(format, vec![0, 255, 0])).is_err());
        }
        // Independent PNM headers: reject declared dimensions/output bytes
        // before attempting to read the absent pixel payload.
        for header in [b"P6\n16385 1\n255\n".as_slice(), b"P6\n8192 8192\n255\n"] {
            assert!(matches!(
                raster(&source(Format::Pnm, header.to_vec())),
                Err(Error::ResourceLimit)
            ));
        }
        let mut bytes = animated(1).as_bytes().to_vec();
        bytes[6..8].copy_from_slice(&8192u16.to_le_bytes());
        bytes[8..10].copy_from_slice(&8192u16.to_le_bytes());
        assert!(matches!(
            raster(&source(Format::Gif, bytes)),
            Err(Error::ResourceLimit)
        ));
        assert_eq!(pixel_bytes(4096, 4096), Ok(MAX_PIXEL_BYTES));
        assert_eq!(pixel_bytes(4096, 4097), Err(Error::ResourceLimit));
    }

    #[test]
    fn jpeg_exif_orientation_changes_the_decoded_dimensions() {
        let mut encoded = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 1, Rgb([220, 11, 45])))
            .write_to(&mut encoded, image::ImageFormat::Jpeg)
            .unwrap();
        let jpeg = encoded.into_inner();
        // APP1 Exif with one little-endian TIFF orientation SHORT, value 6
        // (rotate 90 degrees clockwise). Constructed independently of image.
        let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let mut oriented = jpeg[..2].to_vec();
        oriented.extend_from_slice(&[0xff, 0xe1]);
        oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        oriented.extend_from_slice(exif);
        oriented.extend_from_slice(&jpeg[2..]);
        let decoded = raster(&source(Format::Jpeg, oriented)).unwrap();
        let dimensions = decoded.image.size(0);
        assert_eq!((dimensions.width.0, dimensions.height.0), (1, 2));
        assert_eq!(decoded.pixel_bytes, 8);
    }
}
