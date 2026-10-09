//! Actual AppKit menu construction on the application main thread. This checks
//! row/submenu snapshots and action routing, not VoiceOver or popup screenshots.
use super::*;
use gpuio_protocol::icon_transform::Transform;
use objc2_app_kit::NSBitmapImageRep;
use std::sync::Arc;

pub(crate) fn verify() {
    let marker = MainThreadMarker::new().expect("AppKit main thread");
    for density in [1u32, 2] {
        let side = 16 * density;
        let mut bytes = image::RgbaImage::new(side, side);
        for y in 3 * density..5 * density {
            for x in 2 * density..4 * density {
                bytes.put_pixel(x, y, image::Rgba([255, 255, 255, 191]));
            }
        }
        let pixels = Arc::new(gpui::RenderImage::new(vec![image::Frame::new(bytes)]));
        let icon = |transform| super::super::popup_icon::Icon {
            pixels: pixels.clone(),
            transform: Some(transform),
        };
        let target = Target::new();
        let menu = build(
            &[
                Item::Row {
                    label: "Rotate".into(),
                    enabled: true,
                    checked: false,
                    action: Some(7),
                    icon: Some(icon(Transform {
                        rotation_degrees: 90.,
                        ..Default::default()
                    })),
                },
                Item::Submenu {
                    label: "Reflect".into(),
                    enabled: true,
                    items: vec![],
                    icon: Some(icon(Transform {
                        scale_x: -1.,
                        ..Default::default()
                    })),
                },
            ],
            &target,
            marker,
            &mut Default::default(),
        );
        for (index, left, top) in [(0, 11, 2), (1, 12, 3)] {
            let row = menu.itemAtIndex(index).unwrap();
            let image = row.image().expect("native icon snapshot");
            assert!(image.isTemplate());
            assert_eq!(image.size(), objc2_foundation::NSSize::new(16., 16.));
            let representations = image.representations();
            let representation = representations.objectAtIndex(0);
            let bitmap = representation.downcast_ref::<NSBitmapImageRep>().unwrap();
            assert_eq!(bitmap.bytesPerRow(), (side * 4) as isize);
            assert_eq!(
                (bitmap.pixelsWide(), bitmap.pixelsHigh()),
                (side as isize, side as isize)
            );
            // SAFETY: the retained native representation owns this validated RGBA8 payload.
            let data = unsafe {
                std::slice::from_raw_parts(bitmap.bitmapData(), (side * side * 4) as usize)
            };
            for y in 0..side {
                for x in 0..side {
                    let inside = (left * density..(left + 2) * density).contains(&x)
                        && (top * density..(top + 2) * density).contains(&y);
                    assert_eq!(
                        data[(4 * (y * side + x) + 3) as usize],
                        if inside { 191 } else { 0 }
                    );
                }
            }
        }
        let row = menu.itemAtIndex(0).unwrap();
        assert!(row.isEnabled());
        assert_eq!(row.tag(), 7);
        // Same action target and tag regardless of artwork transformation.
        // SAFETY: Target declares selectItem: with the NSMenuItem argument above;
        // both retained objects live on the AppKit main thread for this call.
        unsafe {
            let _: () = msg_send![&*target, selectItem: &*row];
        }
        assert_eq!(target.ivars().selected.get(), Some(7));
        assert!(menu.itemAtIndex(1).unwrap().submenu().is_some());
    }
    eprintln!(
        "GPUIO_MENU_ICON_TRANSFORMS_OK: actual AppKit row/submenu images, independent pixels at 1x/2x and preserved action route"
    );
}
