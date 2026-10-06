//! Snapshot construction uses the production View and registry on TestPlatform.
//! This does not run AppKit or certify physical interaction.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{TestAppContext, px};
use gpuio_protocol::{HandlerId, WindowId};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn command(name: &str, enabled: bool) -> CommandConfig {
    CommandConfig {
        id: name.into(),
        label: format!("Action {name}"),
        generation: 1,
        enabled,
        checked: Some(true),
        shortcuts: vec![],
        target: CommandTarget::Callback,
    }
}

#[test]
fn popup_snapshot_preserves_registry_state_and_disabled_ancestry() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Popup snapshot", 400., 300.)
        .unwrap();
    let config = MenuConfig {
        presentation: MenuPresentation::PlatformContext,
        menus: vec![MenuDefinition {
            label: "Root".into(),
            disabled: false,
            items: vec![
                MenuItem::Label("Section".into()),
                MenuItem::Command("run".into()),
                MenuItem::Submenu(MenuDefinition {
                    label: "Disabled".into(),
                    disabled: true,
                    items: vec![MenuItem::Command("run".into())],
                }),
                MenuItem::Separator,
                MenuItem::Command("off".into()),
            ],
        }],
    };
    session
        .borrow_mut()
        .apply(&Transaction {
            window: window_id,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(
                    id(0),
                    Kind::CommandScope,
                    "".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetCommands(id(0), vec![command("run", true), command("off", false)]),
                Op::Create(id(1), Kind::Menu, "".into(), None),
                Op::SetMenu(id(1), config.clone()),
                Op::Create(id(2), Kind::Text, "Target".into(), None),
                Op::Splice(id(1), 0, 0, vec![id(2)]),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        })
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.simulate_resize(gpui::size(px(400.), px(300.)));
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| {
            let session = session.borrow();
            let tree = session.tree(window_id).unwrap();
            let mut routes = Vec::new();
            let items = view.popup_items(tree, id(1), (&config.menus[0], &[0]), (window, cx), &mut routes, false);
            assert!(matches!(&items[0], popup::Item::Row { label, enabled: false, action: None, .. } if label == "Section"));
            assert!(matches!(&items[1], popup::Item::Row { label, enabled: true, checked: true, action: Some(0), .. } if label == "Action run"));
            let popup::Item::Submenu { enabled: false, items: children, .. } = &items[2] else { panic!("disabled submenu") };
            assert!(matches!(&children[0], popup::Item::Row { enabled: false, action: Some(1), .. }));
            assert!(matches!(&items[3], popup::Item::Separator));
            assert!(matches!(&items[4], popup::Item::Row { enabled: false, action: Some(2), .. }));
            assert_eq!(routes.len(), 3);
            assert_eq!(routes[0].request().command, "run");
            assert_eq!(routes[1].request().command, "run");
            assert_eq!(routes[2].request().command, "off");
            assert_eq!(routes[0].request().source, CommandSource::Menu(id(1)));
            let mut disabled_routes = Vec::new();
            let disabled = view.popup_items(tree, id(1), (&config.menus[0], &[0]), (window, cx), &mut disabled_routes, true);
            assert!(matches!(&disabled[1], popup::Item::Row { enabled: false, .. }));
        });
    });
}

#[test]
fn mounted_native_icon_survives_release_and_resamples_bounded_target_pixels() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Icon lifetime", 400., 300.)
        .unwrap();
    let source = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10000" height="10000"><rect width="10000" height="10000" fill="red"/></svg>"#;
    let asset = {
        let mut session = session.borrow_mut();
        let assets = session.assets().unwrap();
        let id = assets
            .begin(gpuio_protocol::asset::Format::Svg, source.len())
            .unwrap();
        assets.append(id, 0, source).unwrap();
        assets.finish(id).unwrap();
        id
    };
    let config = MenuConfig {
        presentation: MenuPresentation::PlatformContext,
        menus: vec![MenuDefinition {
            label: "Icons".into(),
            disabled: false,
            items: vec![MenuItem::Label("Label".into())],
        }],
    };
    let transaction = Transaction {
        window: wid,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(id(0), Kind::Menu, "".into(), None),
            Op::SetMenu(id(0), config.clone()),
            Op::Create(id(1), Kind::Text, "Target".into(), None),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::Create(id(3), Kind::Icon, "".into(), None),
            Op::SetImage(
                id(3),
                ImageConfig {
                    source: ImageSource::Reference(asset),
                    fit: ImageFit::Contain,
                    label: None,
                },
            ),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
            Op::SetRoot(Some(id(0))),
        ],
    };
    let (view, cx) = app.add_window_view(|_, _| View::new(wid, session.clone(), transport));
    cx.update(|window, cx| {
        let applied = session.borrow_mut().apply(&transaction).unwrap();
        view.update(cx, |view, cx| {
            view.update_editors(&applied.dirty, window, cx)
        });
        // Release immediately after acceptance, before the image worker runs.
        session
            .borrow_mut()
            .assets()
            .unwrap()
            .release(asset)
            .unwrap();
    });
    for _ in 0..8 {
        cx.run_until_parked();
    }
    cx.update(|window, cx| view.update(cx, |view, cx| {
        let image = view.platform_menu_pixels(id(3), window, cx).unwrap();
        assert_eq!(u32::from(image.size(0).width), (16. * window.scale_factor()).ceil() as u32);
        assert!(image.as_bytes(0).unwrap().chunks_exact(4).all(|p| p == [0, 0, 0, 255]));
        let session = session.borrow();
        assert!(session.acquire_image(asset).is_err());
        let items = view.popup_items(session.tree(wid).unwrap(), id(0), (&config.menus[0], &[0]), (window, cx), &mut Vec::new(), false);
        assert!(matches!(&items[0], popup::Item::Row { icon: Some(icon), .. } if Arc::ptr_eq(icon, &image)));
    }));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            window.set_scale_factor(3.);
            // Keep the previous raster while a new density is being prepared.
            assert!(view.platform_menu_pixels(id(3), window, cx).is_some());
        })
    });
    for _ in 0..8 {
        cx.run_until_parked();
    }
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            let image = view.platform_menu_pixels(id(3), window, cx).unwrap();
            assert_eq!(u32::from(image.size(0).width), 48);
        })
    });
    for (base, presentation) in [
        (1, MenuPresentation::Context),
        (2, MenuPresentation::PlatformContext),
    ] {
        cx.update(|window, cx| {
            let applied = session
                .borrow_mut()
                .apply(&Transaction {
                    window: wid,
                    base,
                    revision: base + 1,
                    operations: vec![Op::SetMenu(
                        id(0),
                        MenuConfig {
                            presentation,
                            ..config.clone()
                        },
                    )],
                })
                .unwrap();
            view.update(cx, |view, cx| {
                view.update_editors(&applied.dirty, window, cx)
            });
        });
        for _ in 0..8 {
            cx.run_until_parked();
        }
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                assert_eq!(
                    view.platform_menu_pixels(id(3), window, cx).is_some(),
                    presentation == MenuPresentation::PlatformContext
                );
            })
        });
    }
}
