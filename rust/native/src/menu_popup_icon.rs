//! Bounded template bitmap snapshots for AppKit. SVG parsing/rasterization has
//! already completed on the image workers; this module snapshots ready alpha.
use gpui::RenderImage;
use gpuio_protocol::icon_transform::Transform;
use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSBitmapFormat, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage};
use objc2_foundation::NSSize;

const MAX_SIDE: u32 = 256; // 16 logical pixels at the image service's max density.
const MAX_BYTES: usize = 8 * 1024 * 1024;

pub(super) struct Icon {
    pub pixels: std::sync::Arc<RenderImage>,
    pub transform: Option<Transform>,
}

/// Inverse-map one destination pixel center into the ready alpha bitmap. This
/// bounded snapshot work happens once before AppKit starts tracking, not in GPUI
/// paint. The output stays in the existing 16-logical-pixel menu icon slot.
fn transformed_alpha(
    source: &[u8],
    width: u32,
    height: u32,
    sampling: &(Transform, f64, f64),
    x: u32,
    y: u32,
) -> u8 {
    let (transform, sine, cosine) = *sampling;
    if transform.scale_x == 0. || transform.scale_y == 0. {
        return 0;
    }
    let dx = x as f64 + 0.5 - width as f64 / 2. - transform.translate_x * width as f64 / 16.;
    let dy = y as f64 + 0.5 - height as f64 / 2. - transform.translate_y * height as f64 / 16.;
    let sx = (cosine * dx + sine * dy) / transform.scale_x + width as f64 / 2. - 0.5;
    let sy = (-sine * dx + cosine * dy) / transform.scale_y + height as f64 / 2. - 0.5;
    if !sx.is_finite()
        || !sy.is_finite()
        || sx < -1.
        || sy < -1.
        || sx > width as f64
        || sy > height as f64
    {
        return 0;
    }
    let (left, top) = (sx.floor() as i64, sy.floor() as i64);
    let (fx, fy) = (sx - left as f64, sy - top as f64);
    let alpha = |x: i64, y: i64| {
        if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
            0.
        } else {
            source[4 * (y as usize * width as usize + x as usize) + 3] as f64
        }
    };
    let upper = alpha(left, top) * (1. - fx) + alpha(left + 1, top) * fx;
    let lower = alpha(left, top + 1) * (1. - fx) + alpha(left + 1, top + 1) * fx;
    (upper * (1. - fy) + lower * fy).round().clamp(0., 255.) as u8
}

/// One per pending/tracking popup. The app-wide popup lease admits only one
/// such snapshot; bytes count every retained NSBitmapImageRep payload, including
/// duplicate icons. AppKit's internal rendering allocations are not measured RSS.
#[derive(Default)]
pub(super) struct Budget {
    bytes: usize,
}
impl Budget {
    pub(super) fn image(
        &mut self,
        pixels: &RenderImage,
        transform: Option<Transform>,
    ) -> Option<Retained<NSImage>> {
        if pixels.frame_count() != 1 || transform.is_some_and(|value| !value.is_valid()) {
            return None;
        }
        let size = pixels.size(0);
        let (width, height) = (u32::from(size.width), u32::from(size.height));
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return None;
        }
        let bytes = width as usize * height as usize * 4;
        if bytes > MAX_BYTES.checked_sub(self.bytes)? {
            return None;
        }
        let source = pixels.as_bytes(0)?;
        if source.len() != bytes {
            return None;
        }
        // SAFETY: null planes request AppKit-owned storage. All dimensions and
        // row/pixel sizes are bounded above and specify interleaved RGBA8.
        let bitmap = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(), std::ptr::null_mut(), width as isize,
                height as isize, 8, 4, true, false, NSDeviceRGBColorSpace,
                NSBitmapFormat::AlphaNonpremultiplied, width as isize * 4, 32,
            )
        }?;
        let data = bitmap.bitmapData();
        if data.is_null()
            || bitmap.bytesPerRow() != width as isize * 4
            || bitmap.pixelsWide() != width as isize
            || bitmap.pixelsHigh() != height as isize
        {
            return None;
        }
        // SAFETY: this freshly allocated, unshared representation owns at least
        // height * bytesPerRow writable bytes. The slice ends before sharing it.
        let output = unsafe { std::slice::from_raw_parts_mut(data, bytes) };
        let sampling = transform.map(|transform| {
            let (sine, cosine) = transform.rotation_degrees.to_radians().sin_cos();
            (transform, sine, cosine)
        });
        for (index, output) in output.chunks_exact_mut(4).enumerate() {
            let alpha = match &sampling {
                Some(sampling) => transformed_alpha(
                    source,
                    width,
                    height,
                    sampling,
                    index as u32 % width,
                    index as u32 / width,
                ),
                None => source[index * 4 + 3],
            };
            output.copy_from_slice(&[0, 0, 0, alpha]);
        }
        let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(16., 16.));
        image.addRepresentation(&bitmap);
        image.setTemplate(true);
        self.bytes += bytes;
        Some(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pixels(side: u32) -> RenderImage {
        let mut data = image::RgbaImage::from_pixel(side, side, image::Rgba([1, 2, 3, 191]));
        data.put_pixel(0, 0, image::Rgba([255, 0, 0, 0]));
        RenderImage::new(vec![image::Frame::new(data)])
    }

    #[test]
    fn transformed_templates_rotate_reflect_translate_and_clip_at_both_densities() {
        objc2::rc::autoreleasepool(|_| {
            for density in [1u32, 2] {
                let side = 16 * density;
                let mut source = image::RgbaImage::new(side, side);
                // Asymmetric off-center opaque block, independent expected
                // positions below are in logical pixels (not computed by SRT).
                for y in 3 * density..5 * density {
                    for x in 2 * density..4 * density {
                        source.put_pixel(x, y, image::Rgba([255, 128, 0, 191]));
                    }
                }
                let pixels = RenderImage::new(vec![image::Frame::new(source)]);
                let identity = Transform::default();
                for (transform, expected) in [
                    (identity, Some((2, 3))),
                    (
                        Transform {
                            rotation_degrees: 90.,
                            ..identity
                        },
                        Some((11, 2)),
                    ),
                    (
                        Transform {
                            scale_x: -1.,
                            ..identity
                        },
                        Some((12, 3)),
                    ),
                    (
                        Transform {
                            translate_x: 3.,
                            translate_y: 2.,
                            ..identity
                        },
                        Some((5, 5)),
                    ),
                    (
                        Transform {
                            scale_x: 0.,
                            ..identity
                        },
                        None,
                    ),
                    (
                        Transform {
                            translate_x: 16.,
                            ..identity
                        },
                        None,
                    ),
                ] {
                    let mut budget = Budget::default();
                    let icon = budget.image(&pixels, Some(transform)).unwrap();
                    assert_eq!(icon.size(), NSSize::new(16., 16.));
                    assert!(icon.isTemplate());
                    let reps = icon.representations();
                    let rep = reps.objectAtIndex(0);
                    let bitmap = rep.downcast_ref::<NSBitmapImageRep>().unwrap();
                    assert_eq!(budget.bytes, (side * side * 4) as usize);
                    // SAFETY: validated fresh interleaved RGBA8 payload retained by bitmap.
                    let data =
                        unsafe { std::slice::from_raw_parts(bitmap.bitmapData(), budget.bytes) };
                    for y in 0..side {
                        for x in 0..side {
                            let inside = expected.is_some_and(|(left, top)| {
                                (left * density..(left + 2) * density).contains(&x)
                                    && (top * density..(top + 2) * density).contains(&y)
                            });
                            assert_eq!(
                                data[4 * (y * side + x) as usize + 3],
                                if inside { 191 } else { 0 },
                                "{transform:?} at {x},{y} density{density}"
                            );
                        }
                    }
                }
                let mut budget = Budget::default();
                assert!(
                    budget
                        .image(
                            &pixels,
                            Some(Transform {
                                scale_y: f64::NAN,
                                ..identity
                            })
                        )
                        .is_none()
                );
                assert_eq!(budget.bytes, 0);
            }
        });
    }

    #[test]
    fn template_preserves_alpha_and_logical_size_without_retaining_source() {
        objc2::rc::autoreleasepool(|_| {
            let mut budget = Budget::default();
            let icon = budget.image(&pixels(32), None).unwrap();
            assert!(icon.isTemplate());
            assert_eq!(icon.size(), NSSize::new(16., 16.));
            assert_eq!(budget.bytes, 4096);
            let reps = icon.representations();
            assert_eq!(reps.len(), 1);
            let bitmap = reps.objectAtIndex(0);
            let bitmap = bitmap.downcast_ref::<NSBitmapImageRep>().unwrap();
            assert_eq!((bitmap.pixelsWide(), bitmap.pixelsHigh()), (32, 32));
            // SAFETY: retained bitmap owns the validated 32*32 RGBA8 payload.
            let data = unsafe { std::slice::from_raw_parts(bitmap.bitmapData(), 4096) };
            assert_eq!(&data[..8], &[0, 0, 0, 0, 0, 0, 0, 191]);
        });
    }

    #[test]
    fn aggregate_limit_and_invalid_shapes_do_not_over_admit() {
        objc2::rc::autoreleasepool(|_| {
            let mut budget = Budget::default();
            assert!(budget.image(&pixels(257), None).is_none());
            let animated = RenderImage::new(vec![
                image::Frame::new(image::RgbaImage::new(1, 1)),
                image::Frame::new(image::RgbaImage::new(1, 1)),
            ]);
            assert!(budget.image(&animated, None).is_none());
            assert_eq!(budget.bytes, 0);
            let source = pixels(256);
            let mut retained = Vec::new();
            for _ in 0..32 {
                retained.push(budget.image(&source, None).unwrap());
            }
            assert_eq!(budget.bytes, MAX_BYTES);
            assert!(budget.image(&source, None).is_none());
            assert_eq!(retained.len(), 32);
        });
    }
}
