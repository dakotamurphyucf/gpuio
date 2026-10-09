//! Materialize an already decoded, shape-checked template in AppKit-owned storage.
use gpui::MenuIcon;
use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSBitmapFormat, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage};
use objc2_foundation::NSSize;

pub(super) fn image(icon: &MenuIcon) -> Option<Retained<NSImage>> {
    let width = icon.width() as isize;
    let height = icon.height() as isize;
    // Null planes request AppKit-owned storage. MenuIcon validates dimensions and
    // the exact interleaved RGBA8 byte count before reaching this boundary.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(), std::ptr::null_mut(), width, height,
            8, 4, true, false, NSDeviceRGBColorSpace,
            NSBitmapFormat::AlphaNonpremultiplied, width * 4, 32,
        )
    }?;
    let data = bitmap.bitmapData();
    if data.is_null()
        || bitmap.bytesPerRow() != width * 4
        || bitmap.pixelsWide() != width
        || bitmap.pixelsHigh() != height
    {
        return None;
    }
    // Fresh unshared representation owns height * bytesPerRow writable bytes.
    unsafe { std::slice::from_raw_parts_mut(data, icon.rgba().len()) }.copy_from_slice(icon.rgba());
    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(16., 16.));
    image.addRepresentation(&bitmap);
    image.setTemplate(true);
    Some(image)
}
