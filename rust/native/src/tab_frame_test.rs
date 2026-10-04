//! Public OCaml frame transactions through real layout/input on TestPlatform.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn fixtures() -> Vec<Transaction> {
    include_str!("../../../test/fixtures/tab-frame-transactions.hex")
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

#[test]
fn public_frame_keeps_fixed_controls_outside_scroll_and_trailing_inside() {
    let fixtures = fixtures();
    let initial = &fixtures[0];
    let node = |kind, text| {
        initial
            .operations
            .iter()
            .find_map(|op| match op {
                Op::Create(id, k, t, _) if *k == kind && t == text => Some(*id),
                _ => None,
            })
            .unwrap()
    };
    let tab = node(Kind::TabBar, "");
    let prefix = node(Kind::Button, "Prefix");
    let trailing = node(Kind::Button, "Trailing");
    let suffix = node(Kind::Button, "Suffix");
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, initial.window, "Tab frame", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(initial.window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|w, _| w.activate_window());
    cx.run_until_parked();
    apply(&owner, cx, initial);
    let prefix_bounds = bounds(cx, "Prefix");
    let suffix_bounds = bounds(cx, "Suffix");
    let viewport = bounds(cx, "Framed tabs");
    assert!((viewport.x0 - prefix_bounds.x1 - 10.).abs() < 0.1);
    assert!((suffix_bounds.x0 - viewport.x1 - 10.).abs() < 0.1);
    assert!((viewport.width() - 330.).abs() < 0.1, "{viewport:?}");
    assert!(bounds(cx, "Trailing").x0 > viewport.x1);
    let state = owner.read_with(cx, |v, _| Rc::downgrade(&v.tab_viewports[&tab]));
    let focus = owner.read_with(cx, |v, _| v.buttons[&tab].focus.clone());
    let trailing_focus = owner.read_with(cx, |v, _| v.buttons[&trailing].focus.clone());
    transport.mailbox.lock().unwrap().drain(100);
    click(cx, "Prefix");
    cx.simulate_keystrokes("right");
    let events = transport.mailbox.lock().unwrap().drain(100);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Press(_, id, _, _) if *id == prefix))
            .count(),
        1
    );
    assert!(!events.iter().any(|e| matches!(e, Event::Choice(..))));
    apply(&owner, cx, &fixtures[1]);
    assert_eq!(bounds(cx, "Prefix"), prefix_bounds);
    assert_eq!(bounds(cx, "Suffix"), suffix_bounds);
    let a = bounds(cx, "A");
    assert!(a.x0 >= viewport.x0 - 0.1 && a.x1 <= viewport.x1 + 0.1);
    assert!(owner.read_with(cx, |v, _| v.scrolls[&tab].handle.offset().x) < px(-200.));
    assert_eq!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::Tab)
            .count(),
        6
    );
    // Tab enters the independently focusable trailing child and reveals it.
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes("tab");
    draw(cx);
    assert!(cx.update(|window, _| trailing_focus.is_focused(window)));
    let tail = bounds(cx, "Trailing");
    assert!(
        tail.x0 >= viewport.x0 - 0.1 && tail.x1 <= viewport.x1 + 0.1,
        "{tail:?}, viewport={viewport:?}, scroll={:?}",
        owner.read_with(cx, |v, _| {
            let state = &v.scrolls[&tab];
            (
                state.mask.get(),
                state.handle.offset(),
                state.handle.max_offset(),
            )
        })
    );
    assert_eq!(bounds(cx, "Prefix"), prefix_bounds);
    assert_eq!(bounds(cx, "Suffix"), suffix_bounds);
    transport.mailbox.lock().unwrap().drain(100);
    click(cx, "Trailing");
    let events = transport.mailbox.lock().unwrap().drain(100);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Press(_, id, _, _) if *id == trailing))
            .count(),
        1
    );
    assert!(!events.iter().any(|e| matches!(e, Event::Choice(..))));
    apply(&owner, cx, &fixtures[2]);
    assert!(
        (bounds(cx, "Framed tabs").width() - 370.).abs() < 0.1,
        "an omitted prefix must not leave a second outer gap"
    );
    owner.read_with(cx, |v, _| {
        assert_eq!(v.buttons[&trailing].focus, trailing_focus);
        assert_eq!(v.buttons[&tab].focus, focus);
        assert!(v.buttons.contains_key(&suffix));
        assert!(!v.buttons.contains_key(&prefix));
        assert!(Rc::ptr_eq(
            &state.upgrade().unwrap(),
            &v.tab_viewports[&tab]
        ));
    });
    for tx in &fixtures[3..5] {
        apply(&owner, cx, tx);
        assert_eq!(
            cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .filter(|(_, n)| n.role() == gpui::accesskit::Role::Tab)
                .count(),
            6
        );
        owner.read_with(cx, |v, _| {
            assert_eq!(v.buttons[&trailing].focus, trailing_focus);
            assert!(Rc::ptr_eq(
                &state.upgrade().unwrap(),
                &v.tab_viewports[&tab]
            ));
        });
    }
    apply(&owner, cx, &fixtures[5]);
    assert!(state.upgrade().is_none());
    owner.read_with(cx, |v, _| {
        assert!(v.buttons.is_empty());
        assert!(v.scrolls.is_empty());
        assert!(v.tab_viewports.is_empty());
    });
}
