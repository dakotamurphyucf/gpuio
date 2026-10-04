//! Production Host integration on TestPlatform, not physical desktop acceptance.
use super::*;
use crate::scrollbar_geometry::Axis;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::scrollbar::{Appearance, Axis as Selection, Config, Mode, Motion};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn n(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn config() -> Config {
    Config {
        label: "Content".into(),
        axis: Selection::Both,
        mode: Mode::Always,
        appearance: Appearance::default(),
        motion: Motion::default(),
    }
}
fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
fn scrolling(inert: bool) -> Vec<Style> {
    let mut styles = dimensions(200., 100.);
    styles.push(Style::Fields(vec![
        Field::OverflowY(3),
        Field::Inert(inert),
    ]));
    styles
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(w()).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: w(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, window, cx);
            v.list_actions(&applied.lists, window, cx);
            cx.notify();
        })
    });
    draw(cx);
}
#[test]
fn ordinary_overlay_uses_scroll_owner_and_reset_preserves_it() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Scrollbars", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    apply(
        &owner,
        cx,
        vec![
            Op::Create(n(0), Kind::Container, "".into(), None),
            Op::SetStyle(n(0), scrolling(false)),
            Op::SetScrollbar(n(0), Some(Box::new(config()))),
            Op::Create(n(1), Kind::Text, "Content".into(), None),
            Op::SetStyle(n(1), dimensions(800., 400.)),
            Op::Splice(n(0), 0, 0, vec![n(1)]),
            Op::SetRoot(Some(n(0))),
        ],
    );
    let scroll = owner.read_with(cx, |v, _| v.scrolls[&n(0)].clone());
    let state = owner.read_with(cx, |v, _| {
        v.scrollbars[&(n(0), scrollbar_host::Owner::Viewport)].clone()
    });
    let bars: Vec<_> = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .filter(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(
        bars.len(),
        1,
        "overflow hidden X must not acquire a bar from wide content"
    );
    assert_eq!(bars[0].1.label(), Some("Content — vertical"));
    assert_eq!(bars[0].1.max_numeric_value(), Some(300.));
    let focus = state.borrow().focus(Axis::Vertical).clone();
    assert!(
        cx.update(|window, cx| owner.read(cx).focus.borrow().can_focus(&focus, window)),
        "range was registered in first paint"
    );
    cx.update(|window, cx| window.focus(&focus, cx));
    draw(cx);
    assert!(
        cx.update(|window, _| focus.is_focused(window)),
        "range stays focused after Host paint"
    );
    assert!(
        cx.update(|window, cx| owner.read(cx).focus.borrow().can_focus(&focus, window)),
        "Host admits the painted range"
    );
    cx.simulate_keystrokes("end");
    assert_eq!(
        scroll.handle.offset().y,
        px(-300.),
        "keyboard action updates the handle before redraw"
    );
    draw(cx);
    assert_eq!(scroll.handle.offset(), gpui::point(px(0.), px(-300.)));
    assert!(cx.update(|window, cx| owner.read(cx).focus.borrow().can_focus(&focus, window)));
    // Pointer dragging preserves another owner's focus, so Escape must be
    // routed by the Host rather than only by the range's focused key listener.
    let root_focus = owner.read_with(cx, |v, _| v.root_focus.clone().unwrap());
    cx.update(|window, cx| window.focus(&root_focus, cx));
    draw(cx);
    let bounds = scroll.handle.bounds();
    let thumb = gpui::point(bounds.right() - px(7.), bounds.bottom() - px(28.));
    cx.simulate_mouse_move(thumb, None, Default::default());
    draw(cx);
    cx.simulate_mouse_down(thumb, gpui::MouseButton::Left, Default::default());
    assert!(state.borrow().is_dragging());
    assert!(cx.update(|window, _| root_focus.is_focused(window)));
    cx.simulate_keystrokes("escape");
    assert!(!state.borrow().is_dragging());
    assert!(cx.update(|window, _| window.captured_hitbox().is_none()));
    assert!(cx.update(|window, _| root_focus.is_focused(window)));
    cx.simulate_mouse_up(thumb, gpui::MouseButton::Left, Default::default());
    cx.update(|window, cx| window.focus(&focus, cx));
    draw(cx);
    let mut custom = config();
    custom.appearance.track.width = Some(28.);
    apply(
        &owner,
        cx,
        vec![Op::SetScrollbar(n(0), Some(Box::new(custom)))],
    );
    assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&v.scrolls[&n(0)], &scroll)));
    assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(
        &v.scrollbars[&(n(0), scrollbar_host::Owner::Viewport)],
        &state
    )));
    assert_eq!(scroll.handle.offset().y, px(-300.));
    assert!(cx.update(|window, _| focus.is_focused(window)));
    apply(&owner, cx, vec![Op::SetStyle(n(0), scrolling(true))]);
    assert!(!cx.update(|window, _| focus.is_focused(window)));
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
    );
    apply(&owner, cx, vec![Op::SetStyle(n(0), scrolling(false))]);
    let weak = Rc::downgrade(&state);
    drop(state);
    apply(&owner, cx, vec![Op::SetScrollbar(n(0), None)]);
    assert!(
        weak.upgrade().is_none(),
        "reset releases the presentation owner"
    );
    assert!(owner.read_with(cx, |v, _| v.scrollbars.is_empty()));
    assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&v.scrolls[&n(0)], &scroll)));
    assert_eq!(scroll.handle.offset().y, px(-300.));
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| v.close_scrollbars(window, cx));
        window.remove_window();
    });
}

#[test]
fn managed_list_shares_its_handle_and_visibility_flag_wins() {
    managed_collection(false, Axis::Vertical);
    managed_collection(false, Axis::Horizontal);
}
#[test]
fn tree_range_keeps_navigation_focus_separate() {
    managed_collection(true, Axis::Vertical);
}
fn managed_collection(tree: bool, axis: Axis) {
    use gpuio_protocol::list::{Config as ListConfig, IdRun, Order, ScrollPolicy};
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "List", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    let mut list = ListConfig {
        estimated_height: 20.,
        overscan: 0.,
        max_active: 32,
        scroll_policy: ScrollPolicy::KeepPosition,
        scrollbar: true,
        managed: true,
    };
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                n(0),
                Kind::VirtualList,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetStyle(
                n(0),
                match axis {
                    Axis::Vertical => dimensions(200., 100.),
                    Axis::Horizontal => dimensions(100., 200.),
                },
            ),
            Op::SetListConfig(n(0), list.clone()),
            Op::SetListAxis(
                n(0),
                match axis {
                    Axis::Vertical => gpuio_protocol::list::Axis::Vertical,
                    Axis::Horizontal => gpuio_protocol::list::Axis::Horizontal,
                },
            ),
            Op::SetListOrder(
                n(0),
                Order {
                    revision: 1,
                    runs: vec![IdRun {
                        first: 1,
                        count: 100,
                    }],
                },
            ),
            Op::SetScrollbar(n(0), Some(Box::new(config()))),
            Op::SetRoot(Some(n(0))),
        ],
    );
    if tree {
        use gpuio_protocol::accessibility::{Config, Live, Role};
        apply(
            &owner,
            cx,
            vec![
                Op::SetAccessibility(
                    n(0),
                    Some(Config {
                        role: Some(Role::Tree(true)),
                        label: Some("Outline".into()),
                        description: None,
                        live: Live::Off,
                        field: None,
                        current: None,
                    }),
                ),
                Op::SetTreeInput(n(0), true),
            ],
        );
    }
    let handle = owner.read_with(cx, |v, _| v.lists[&n(0)].borrow().native.handle().clone());
    let state = owner.read_with(cx, |v, _| {
        v.scrollbars[&(n(0), scrollbar_host::Owner::Viewport)].clone()
    });
    let bars: Vec<_> = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .filter(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(bars.len(), 1);
    let focus = state.borrow().focus(axis).clone();
    cx.update(|window, cx| window.focus(&focus, cx));
    draw(cx);
    cx.simulate_keystrokes("end");
    draw(cx);
    assert!(handle.logical_scroll_top().item_ix > 80);
    if tree {
        assert!(
            !cx.update(|window, cx| owner.read(cx).lists[&n(0)].borrow().owns_tree_focus(window)),
            "scrollbar focus is distinct from tree navigation focus"
        );
    }

    let logical_top = |cx: &mut VisualTestContext| {
        owner.read_with(cx, |v, _| {
            let top = v.lists[&n(0)].borrow().native.handle().logical_scroll_top();
            (top.item_ix, top.offset_in_item)
        })
    };
    let top = logical_top(cx);
    list.scrollbar = false;
    apply(&owner, cx, vec![Op::SetListConfig(n(0), list.clone())]);
    assert!(owner.read_with(cx, |v, _| v.scrollbars.is_empty()));
    assert!(!cx.update(|window, _| focus.is_focused(window)));
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
    );
    assert_eq!(logical_top(cx), top);
    list.scrollbar = true;
    apply(&owner, cx, vec![Op::SetListConfig(n(0), list.clone())]);
    assert_eq!(logical_top(cx), top);
    assert!(owner.read_with(cx, |v, _| !Rc::ptr_eq(
        &v.scrollbars[&(n(0), scrollbar_host::Owner::Viewport)],
        &state
    )));
    let previous = owner.read_with(cx, |v, _| {
        v.scrollbars[&(n(0), scrollbar_host::Owner::Viewport)].clone()
    });
    let old_handle = owner.read_with(cx, |v, _| v.lists[&n(0)].borrow().native.handle().clone());
    // Begin a real native thumb capture, then replace the list's measurement
    // configuration. Retirement must balance drag hooks on the old handle.
    let retiring_focus = previous.borrow().focus(axis).clone();
    cx.update(|window, cx| window.focus(&retiring_focus, cx));
    draw(cx);
    let bounds = old_handle.viewport_bounds();
    let thumb = match axis {
        Axis::Vertical => gpui::point(bounds.right() - px(7.), bounds.bottom() - px(28.)),
        Axis::Horizontal => gpui::point(bounds.right() - px(28.), bounds.bottom() - px(7.)),
    };
    cx.simulate_mouse_move(thumb, None, Default::default());
    draw(cx);
    cx.simulate_mouse_down(thumb, gpui::MouseButton::Left, Default::default());
    assert!(previous.borrow().is_dragging());
    assert!(old_handle.is_scrollbar_dragging());
    let anchor = logical_top(cx);
    list.estimated_height = 40.;
    apply(&owner, cx, vec![Op::SetListConfig(n(0), list)]);
    assert!(!previous.borrow().is_dragging());
    assert!(!old_handle.is_scrollbar_dragging());
    assert!(!cx.update(|window, _| retiring_focus.is_focused(window)));
    assert_eq!(
        logical_top(cx),
        anchor,
        "replacement preserves the logical item anchor"
    );
    assert!(cx.update(|window, _| window.captured_hitbox().is_none()));
    cx.simulate_mouse_up(thumb, gpui::MouseButton::Left, Default::default());
    let next = owner.read_with(cx, |v, _| {
        v.scrollbars[&(n(0), scrollbar_host::Owner::Viewport)].clone()
    });
    assert!(!Rc::ptr_eq(&previous, &next));
    let max = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
        .unwrap()
        .1
        .max_numeric_value();
    assert_eq!(
        max,
        Some(3900.),
        "new presentation reads the replacement handle"
    );
    let focus = next.borrow().focus(axis).clone();
    cx.update(|window, cx| window.focus(&focus, cx));
    draw(cx);
    cx.simulate_keystrokes("end");
    draw(cx);
    assert!(
        logical_top(cx).0 >= 97,
        "range key moves the current list owner"
    );
    if !tree {
        let current = owner.read_with(cx, |v, _| v.lists[&n(0)].borrow().native.handle().clone());
        current.scroll_to(gpui::ListOffset {
            item_ix: 25,
            offset_in_item: px(7.),
        });
        draw(cx);
        let anchor = logical_top(cx);
        let opposite = match axis {
            Axis::Vertical => gpuio_protocol::list::Axis::Horizontal,
            Axis::Horizontal => gpuio_protocol::list::Axis::Vertical,
        };
        apply(&owner, cx, vec![Op::SetListAxis(n(0), opposite)]);
        assert_eq!(
            logical_top(cx),
            anchor,
            "axis changes retain the logical anchor"
        );
        assert!(!cx.update(|window, _| focus.is_focused(window)));
        assert!(owner.read_with(cx, |v, _| !Rc::ptr_eq(
            &next,
            &v.scrollbars[&(n(0), scrollbar_host::Owner::Viewport)]
        )));
        let native_axis =
            owner.read_with(cx, |v, _| v.lists[&n(0)].borrow().native.handle().axis());
        assert_eq!(native_axis, current.axis().invert());
    }
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| v.close_scrollbars(window, cx));
        window.remove_window();
    });
}

#[path = "horizontal_list_host_test.rs"]
mod horizontal_lists;
