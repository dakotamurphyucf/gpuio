//! Public menu icon descriptions through retained asset readers and native layout.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{ResourceId, asset::Format};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn fixtures() -> Vec<Transaction> {
    include_str!("../../../test/fixtures/tab-menu-icons-transactions.hex")
        .lines()
        .map(|hex| {
            let bytes: Vec<_> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
                panic!("transaction")
            };
            tx
        })
        .collect()
}
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..8 {
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
        });
    }
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, tx: &Transaction) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let result = view.session.borrow_mut().apply(tx).unwrap();
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        })
    });
    settle(cx);
}
fn key(cx: &mut VisualTestContext, name: &str) {
    cx.update(|window, cx| {
        let keystroke = gpui::Keystroke::parse(name).unwrap();
        window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }),
            cx,
        );
    });
    settle(cx);
}
fn ready(owner: &Entity<View>, cx: &mut VisualTestContext, id: NodeId, tint: u32) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let binding = view.images[&id].binding.borrow();
            assert!(binding.mask);
            assert_eq!(binding.rendered.tint, None);
            assert!(binding.pending.is_none());
            assert!(binding.resize_error.is_none());
            assert!(matches!(binding.rendered.size, asset_svg::Size::Exact(_)));
            let handle = binding.current.as_ref().unwrap();
            let image = image_host::image_mask(handle, window, cx).unwrap().unwrap();
            assert!(window.has_image_mask_atlas_entry(&image, 0));
            assert!(!window.has_image_atlas_entry(&image));
            let bytes = image.as_bytes(0).unwrap();
            // Both source SVGs are opaque. Tint is now native scene state,
            // while decoded alpha and the mask atlas entry remain reusable.
            assert_eq!(bytes[bytes.len() / 8 * 4 + 3], 255);
            let center = view.probes.borrow()[&id]
                .bounds
                .center()
                .scale(window.scale_factor());
            assert!(
                window
                    .painted_monochrome_sprites()
                    .iter()
                    .any(|sprite| sprite.bounds.contains(&center)
                        && sprite.color == crate::host::color(&Color::Rgba(tint.into())))
            );
        })
    });
}

#[test]
fn menu_icons_keep_leases_across_closed_reordered_and_virtualized_rows() {
    run(false);
}
#[test]
fn window_close_releases_open_menu_icons_and_weak_render_callbacks() {
    run(true);
}
fn run(close_window: bool) {
    let fixtures = fixtures();
    let wid = fixtures[0].window;
    let menu = fixtures[0]
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetChoiceMenu(id, true) => Some(*id),
            _ => None,
        })
        .unwrap();
    let icons: Vec<_> = fixtures[0]
        .operations
        .iter()
        .filter_map(|op| match op {
            Op::Create(id, Kind::Icon, _, _) => Some(*id),
            _ => None,
        })
        .collect();
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    app.update(image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(crate::transport::Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(crate::session::Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Menu icons", 800., 600.)
        .unwrap();
    let svg=br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="red"/></svg>"#;
    for index in 0..2 {
        let mut s = session.borrow_mut();
        let assets = s.assets().unwrap();
        let id = assets.begin(Format::Svg, svg.len()).unwrap();
        assert_eq!(id, ResourceId::from_parts(index, 1).unwrap());
        assets.append(id, 0, svg).unwrap();
        assets.finish(id).unwrap();
    }
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|w, _| w.activate_window());
    apply(&owner, cx, &fixtures[0]);
    let original = owner.read_with(cx, |v, _| v.images[&icons[0]].binding.clone());
    owner.read_with(cx, |v, _| {
        assert_eq!(v.images.len(), 2);
        assert!(v.images[&icons[0]].binding.borrow().requested.is_none());
        assert!(v.images[&icons[1]].binding.borrow().requested.is_none());
    });
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(ResourceId::from_parts(0, 1).unwrap())
        .unwrap();
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 1);
    let focus = owner.read_with(cx, |v, _| v.buttons[&menu].focus.clone());
    cx.update(|w, cx| w.focus(&focus, cx));
    settle(cx);
    key(cx, "space");
    ready(&owner, cx, icons[0], 0x22aa66ff);
    owner.read_with(cx, |v, _| {
        assert!(
            v.images[&icons[1]].binding.borrow().requested.is_none(),
            "offscreen icon is leased but not measured/rasterized"
        )
    });
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::Image)
    );
    key(cx, "end");
    ready(&owner, cx, icons[1], 0x22aa66ff);
    key(cx, "escape");
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 1);
    apply(&owner, cx, &fixtures[1]);
    owner.read_with(cx, |v, _| {
        assert!(Rc::ptr_eq(&original, &v.images[&icons[0]].binding))
    });
    key(cx, "space");
    apply(&owner, cx, &fixtures[2]);
    ready(&owner, cx, icons[0], 0xaa2266ff);
    owner.read_with(cx, |v, _| {
        assert!(
            !Rc::ptr_eq(&original, &v.images[&icons[0]].binding),
            "source replacement retires only its binding"
        )
    });
    drop(original);
    apply(&owner, cx, &fixtures[3]);
    assert!(!owner.read_with(cx, |v, _| v.images.contains_key(&icons[1])));
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
    apply(&owner, cx, &fixtures[4]);
    assert!(owner.read_with(cx, |v, _| v.images.is_empty()));
    apply(&owner, cx, &fixtures[5]);
    let readded = owner.read_with(cx, |v, _| *v.images.keys().next().unwrap());
    assert_ne!(readded, icons[0]);
    ready(&owner, cx, readded, 0xaa2266ff);
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(ResourceId::from_parts(1, 1).unwrap())
        .unwrap();
    if close_window {
        let weak = owner.downgrade();
        session.borrow_mut().close(wid).unwrap();
        cx.update(|window, _| window.remove_window());
        drop(owner);
        cx.cx.update(|_| ());
        cx.run_until_parked();
        assert!(weak.upgrade().is_none());
        assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
        assert_eq!(session.borrow().retained_bytes(), 0);
        transport.mailbox.lock().unwrap().drain(128);
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(20));
        cx.run_until_parked();
        assert!(transport.mailbox.lock().unwrap().drain(128).is_empty());
        cx.cx.update(image_host::finish_before_quit);
        return;
    }
    apply(&owner, cx, &fixtures[6]);
    assert!(owner.read_with(cx, |v, _| v.images.is_empty() && v.selects.is_empty()));
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
    apply(&owner, cx, &fixtures[7]);
    assert_eq!(session.borrow().retained_bytes(), 0);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    cx.cx.update(image_host::finish_before_quit);
}
