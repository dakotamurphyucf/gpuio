//! All-tabs menu through exact public OCaml transactions and native TestPlatform.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn fixtures() -> Vec<Transaction> {
    include_str!("../../../test/fixtures/tab-menu-transactions.hex")
        .lines()
        .map(|hex| {
            let bytes: Vec<_> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
                panic!("expected transaction")
            };
            tx
        })
        .collect()
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        assert!(window.simulate_next_frame(cx) <= 1);
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 0);
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, tx: &Transaction) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let result = view.session.borrow_mut().apply(tx).unwrap();
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        });
    });
    draw(cx);
}
fn bounds(cx: &mut VisualTestContext, label: &str) -> gpui::accesskit::Rect {
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    let b = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, n)| n.label() == Some(label))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    gpui::accesskit::Rect::new(b.x0 / scale, b.y0 / scale, b.x1 / scale, b.y1 / scale)
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let b = bounds(cx, label);
    cx.simulate_click(
        gpui::point(
            px((b.x0 + b.width() / 2.) as f32),
            px((b.y0 + b.height() / 2.) as f32),
        ),
        Default::default(),
    );
    draw(cx);
}

fn key(cx: &mut VisualTestContext, text: &str) {
    for name in text.split(' ') {
        let keystroke = gpui::Keystroke::parse(name).unwrap();
        cx.update(|window, cx| {
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
        draw(cx);
    }
}

#[test]
fn menu_keeps_tab_owners_routes_stable_ids_and_bounds_popup_work() {
    let fixtures = fixtures();
    let initial = &fixtures[0];
    let tab = initial
        .operations
        .iter()
        .find_map(|op| match op {
            Op::Create(id, Kind::TabBar, _, _) => Some(*id),
            _ => None,
        })
        .unwrap();
    let (menu, handler) = fixtures[1]
        .operations
        .iter()
        .find_map(|op| match op {
            Op::Create(id, Kind::Select, _, Some(handler)) => Some((*id, *handler)),
            _ => None,
        })
        .unwrap();
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, initial.window, "Tab menu", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(initial.window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|w, _| w.activate_window());
    cx.run_until_parked();
    apply(&owner, cx, initial);
    let tab_state = owner.read_with(cx, |v, _| Rc::downgrade(&v.tab_viewports[&tab]));
    let tab_focus = owner.read_with(cx, |v, _| v.buttons[&tab].focus.clone());
    apply(&owner, cx, &fixtures[1]);
    let menu_focus = owner.read_with(cx, |v, _| v.buttons[&menu].focus.clone());
    let popup = owner.read_with(cx, |v, _| v.selects[&menu].borrow().popup.clone());
    let offset = owner.read_with(cx, |v, _| v.scrolls[&tab].handle.offset());
    let suffix = bounds(cx, "Suffix");
    transport.mailbox.lock().unwrap().drain(100);
    click(cx, "All tabs");
    assert!(popup.borrow().open);
    assert!(cx.update(|w, _| menu_focus.is_focused(w)));
    let nodes = cx.a11y_tree().unwrap().nodes;
    assert!(
        nodes.iter().any(
            |(_, n)| n.role() == gpui::accesskit::Role::Button && n.label() == Some("All tabs")
        )
    );
    assert!(
        nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::Menu)
    );
    assert!(
        nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::MenuItemRadio
                && n.label() == Some("Document 00")
                && n.toggled() == Some(gpui::accesskit::Toggled::True))
    );
    assert!(
        nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::MenuItemRadio
                && n.label() == Some("Document 01")
                && n.is_disabled())
    );
    assert!(
        popup.borrow().rendered_options.get() < 20,
        "virtualized, not the full 40-row catalog"
    );
    key(cx, "down");
    draw(cx);
    assert_eq!(popup.borrow().navigation.active.as_deref(), Some("2"));
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .any(|e| matches!(e, Event::Choice(..)))
    );
    key(cx, "end enter");
    draw(cx);
    let events = transport.mailbox.lock().unwrap().drain(100);
    assert_eq!(
        events
            .iter()
            .filter(
                |e| matches!(e, Event::Choice(_, id, _, _, choice) if *id == menu && choice == "39")
            )
            .count(),
        1
    );
    assert!(!popup.borrow().open);
    assert_eq!(
        owner.read_with(cx, |v, _| v.scrolls[&tab].handle.offset()),
        offset,
        "menu selection does not reveal tabs"
    );
    assert_eq!(bounds(cx, "Suffix"), suffix);
    key(cx, "space");
    draw(cx);
    apply(&owner, cx, &fixtures[2]);
    assert!(popup.borrow().open);
    assert_eq!(popup.borrow().navigation.active.as_deref(), Some(" "));
    assert!(
        session
            .borrow()
            .choose(initial.window, menu, handler, 2, "39")
            .is_none()
    );
    assert!(
        session
            .borrow()
            .choose(initial.window, menu, handler, 2, "2")
            .is_some()
    );
    key(cx, "home");
    draw(cx);
    assert_eq!(popup.borrow().navigation.active.as_deref(), Some("38"));
    let nodes = cx.a11y_tree().unwrap().nodes;
    assert!(
        nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::MenuItemRadio
                && n.label() == Some("Renamed 38")
                && !n.is_disabled())
    );
    let target = nodes
        .iter()
        .find(|(_, n)| {
            n.role() == gpui::accesskit::Role::MenuItemRadio && n.label() == Some("Renamed 38")
        })
        .unwrap()
        .0;
    transport.mailbox.lock().unwrap().drain(100);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: target,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert!(!popup.borrow().open);
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .filter(
                |e| matches!(e, Event::Choice(_, id, _, _, choice) if *id == menu && choice == "38")
            )
            .count(),
        1
    );
    key(cx, "space r");
    assert!(popup.borrow().open);
    assert_eq!(
        popup.borrow().navigation.active.as_deref(),
        Some("38"),
        "typeahead uses renamed labels and skips disabled rows"
    );
    key(cx, "escape");
    draw(cx);
    assert!(!popup.borrow().open);
    key(cx, "space tab");
    draw(cx);
    assert!(!popup.borrow().open);
    click(cx, "All tabs");
    apply(&owner, cx, &fixtures[3]);
    assert!(!popup.borrow().open);
    assert!(
        session
            .borrow()
            .choose(initial.window, menu, handler, 2, "2")
            .is_none()
    );
    apply(&owner, cx, &fixtures[4]);
    assert!(!popup.borrow().open, "availability does not reopen");
    click(cx, "All tabs");
    click(cx, "Suffix");
    assert!(!popup.borrow().open, "outside click closes");
    apply(&owner, cx, &fixtures[5]);
    assert!(
        session
            .borrow()
            .choose(initial.window, menu, handler, 2, " ")
            .is_none()
    );
    owner.read_with(cx, |v, _| {
        assert!(!v.selects.contains_key(&menu));
        assert_eq!(v.buttons[&tab].focus, tab_focus);
        assert!(Rc::ptr_eq(
            &tab_state.upgrade().unwrap(),
            &v.tab_viewports[&tab]
        ));
    });
    drop(popup);
    apply(&owner, cx, &fixtures[6]);
    assert!(tab_state.upgrade().is_none());
    owner.read_with(cx, |v, _| {
        assert!(v.buttons.is_empty());
        assert!(v.selects.is_empty());
        assert!(v.scrolls.is_empty());
    });
}
