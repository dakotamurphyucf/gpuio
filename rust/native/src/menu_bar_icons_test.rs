//! Real AppKit artwork fed by the production retained tree and image workers.
use super::*;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSBitmapImageRep, NSMenu};

fn main_menu() -> Retained<NSMenu> {
    NSApplication::sharedApplication(MainThreadMarker::new().unwrap())
        .mainMenu()
        .unwrap()
}
fn actions(main: &NSMenu) -> Retained<NSMenu> {
    let item = main.itemAtIndex(0).unwrap();
    assert_eq!(item.title().to_string(), "Actions");
    item.submenu().unwrap()
}
fn icon_node(index: i64) -> NodeId {
    node(match index {
        0 => 9,
        1 => 10,
        3 => 11,
        4 => 12,
        _ => unreachable!(),
    })
}
fn has_icon(menu: &NSMenu, index: isize) -> bool {
    menu.itemAtIndex(index).unwrap().image().is_some()
}
fn assert_icon(menu: &NSMenu, index: isize, left: u32) {
    let image = menu
        .itemAtIndex(index)
        .unwrap()
        .image()
        .expect("ready native artwork");
    assert!(image.isTemplate());
    assert_eq!(image.size(), objc2_foundation::NSSize::new(16., 16.));
    let representations = image.representations();
    let rep = representations.objectAtIndex(0);
    let bitmap = rep.downcast_ref::<NSBitmapImageRep>().unwrap();
    let side = bitmap.pixelsWide() as u32;
    assert_eq!(bitmap.pixelsHigh(), side as isize);
    assert!(side >= 16 && side.is_multiple_of(16));
    assert_eq!(bitmap.bytesPerRow(), side as isize * 4);
    let density = side / 16;
    // Validated native representation retains this tightly packed RGBA8 buffer.
    let bytes =
        unsafe { std::slice::from_raw_parts(bitmap.bitmapData(), (side * side * 4) as usize) };
    for y in 0..side {
        for x in 0..side {
            let opaque = (left * density..(left + 2) * density).contains(&x)
                && (3 * density..5 * density).contains(&y);
            assert_eq!(
                &bytes[((y * side + x) * 4) as usize..((y * side + x) * 4 + 4) as usize],
                &[0, 0, 0, if opaque { 255 } else { 0 }]
            );
        }
    }
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    primary: WindowHandle<View>,
    transport: &Transport,
) {
    // Use a fresh tree so fixture slot generations cannot affect later controls.
    let (session, native_transport) = primary
        .update(cx, |view, _, _| {
            (view.session.clone(), view.transport.clone())
        })
        .unwrap();
    let window_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(3, window_id, "Native menu artwork", 400., 280.)
        .unwrap();
    let handle = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                focus: false,
                inactive_frame_interval: Some(std::time::Duration::from_millis(16)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id, session.clone(), native_transport)),
        )
        .unwrap()
    });
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0))),
            Op::SetCommands(node(0), commands()),
            Op::Create(node(1), Kind::Menu, "".into(), None),
            Op::SetMenu(node(1), config(MenuPresentation::PlatformBar)),
            Op::Create(
                node(2),
                Kind::Text,
                "Native menu artwork and ownership test".into(),
                None,
            ),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    frame(cx, handle).await;
    let source = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect x="2" y="3" width="2" height="2" fill="red"/></svg>"#;
    let asset = handle
        .update(cx, |view, _, _| {
            let mut session = view.session.borrow_mut();
            let assets = session.assets().unwrap();
            let asset = assets
                .begin(gpuio_protocol::asset::Format::Svg, source.len())
                .unwrap();
            assets.append(asset, 0, source).unwrap();
            assets.finish(asset).unwrap();
            asset
        })
        .unwrap();
    let mut ops: Vec<_> = (3..9)
        .map(|slot| Op::Create(node(slot), Kind::Container, "".into(), None))
        .collect();
    for index in [0, 1, 3, 4] {
        ops.extend([
            Op::Create(icon_node(index), Kind::Icon, "".into(), None),
            Op::SetImage(
                icon_node(index),
                ImageConfig {
                    source: ImageSource::Reference(asset),
                    fit: ImageFit::Contain,
                    label: None,
                },
            ),
            Op::Splice(node(3 + index), 0, 0, vec![icon_node(index)]),
        ]);
    }
    ops.push(Op::Splice(node(1), 0, 0, (3..9).map(node).collect()));
    apply(cx, handle, ops);
    // The accepted mounted image readers own the asset before workers finish.
    handle
        .update(cx, |view, _, _| {
            view.session
                .borrow_mut()
                .assets()
                .unwrap()
                .release(asset)
                .unwrap();
        })
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        frame(cx, handle).await;
        if has_icon(&actions(&main_menu()), 0) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "native bar icon readiness"
        );
    }
    let original = main_menu();
    let menu = actions(&original);
    for index in [0, 1, 3] {
        assert_icon(&menu, index, 2);
    }
    assert!(!has_icon(&menu, 2));
    let nested = menu.itemAtIndex(3).unwrap().submenu().unwrap();
    assert_icon(&nested, 0, 2);
    menu.update(); // AppKit validates dispatch when the menu opens.
    assert!(!menu.itemAtIndex(1).unwrap().isEnabled());
    emitted(transport);
    native_menu_press("Actions", 0);
    frame(cx, handle).await;
    assert_eq!(
        emitted(transport),
        [("run".into(), CommandSource::Menu(node(1)))]
    );
    for _ in 0..3 {
        handle.update(cx, |_, window, _| window.refresh()).unwrap();
        frame(cx, handle).await;
        assert!(
            std::ptr::eq(&*original, &*main_menu()),
            "unrelated render replaced native artwork"
        );
    }
    apply(
        cx,
        handle,
        vec![Op::SetIconTransform(
            icon_node(0),
            Some(gpuio_protocol::icon_transform::Transform {
                scale_x: -1.,
                ..Default::default()
            }),
        )],
    );
    frame(cx, handle).await;
    assert_icon(&actions(&main_menu()), 0, 12);
    assert_icon(&nested, 0, 2); // Retained old native snapshots remain valid.
    apply(
        cx,
        handle,
        vec![Op::SetImage(
            icon_node(0),
            ImageConfig {
                source: ImageSource::Unavailable(ImageError::Released),
                fit: ImageFit::Contain,
                label: None,
            },
        )],
    );
    frame(cx, handle).await;
    assert!(!has_icon(&actions(&main_menu()), 0));
    assert_icon(&actions(&main_menu()), 3, 2);
    let mut ops = vec![Op::Splice(node(1), 0, 6, vec![])];
    for index in [0, 1, 3, 4] {
        ops.push(Op::Remove(icon_node(index)));
    }
    for index in 0..6 {
        ops.push(Op::Remove(node(3 + index)));
    }
    apply(cx, handle, ops);
    frame(cx, handle).await;
    let cleared = actions(&main_menu());
    for index in [0, 1, 3] {
        assert!(!has_icon(&cleared, index));
    }
    handle
        .update(cx, |view, _, _| {
            assert!(view.session.borrow().acquire_image(asset).is_err());
            for index in [0, 1, 3, 4] {
                assert!(!view.images.contains_key(&icon_node(index)));
            }
        })
        .unwrap();
    handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    session.borrow_mut().close(window_id).unwrap();
    primary
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    frame(cx, primary).await;
    assert!(
        !has_icon(&actions(&main_menu()), 0),
        "surviving window restored its own bar"
    );
    println!(
        "GPUIO_MENU_BAR_ICONS_OK: nested/disabled templates, released registration, unchanged-render identity, transformed/source updates, command dispatch and clear"
    );
}
