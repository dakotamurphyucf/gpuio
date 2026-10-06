//! Bounded template bitmap snapshots for AppKit. SVG parsing/rasterization has
//! already completed on the image workers; this module only copies ready alpha.
use gpui::RenderImage;
use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSBitmapFormat, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage};
use objc2_foundation::NSSize;

const MAX_SIDE: u32 = 256; // 16 logical pixels at the image service's max density.
const MAX_BYTES: usize = 8 * 1024 * 1024;

/// One per pending/tracking popup. The app-wide popup lease admits only one
/// such snapshot; bytes count every retained NSBitmapImageRep payload, including
/// duplicate icons. AppKit's internal rendering allocations are not measured RSS.
#[derive(Default)]
pub(super) struct Budget {
    bytes: usize,
}
impl Budget {
    pub(super) fn image(&mut self, pixels: &RenderImage) -> Option<Retained<NSImage>> {
        if pixels.frame_count() != 1 {
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
        for (source, output) in source.chunks_exact(4).zip(output.chunks_exact_mut(4)) {
            output.copy_from_slice(&[0, 0, 0, source[3]]);
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
    fn template_preserves_alpha_and_logical_size_without_retaining_source() {
        objc2::rc::autoreleasepool(|_| {
            let mut budget = Budget::default();
            let icon = budget.image(&pixels(32)).unwrap();
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
            assert!(budget.image(&pixels(257)).is_none());
            let animated = RenderImage::new(vec![
                image::Frame::new(image::RgbaImage::new(1, 1)),
                image::Frame::new(image::RgbaImage::new(1, 1)),
            ]);
            assert!(budget.image(&animated).is_none());
            assert_eq!(budget.bytes, 0);
            let source = pixels(256);
            let mut retained = Vec::new();
            for _ in 0..32 {
                retained.push(budget.image(&source).unwrap());
            }
            assert_eq!(budget.bytes, MAX_BYTES);
            assert!(budget.image(&source).is_none());
            assert_eq!(retained.len(), 32);
        });
    }
}
