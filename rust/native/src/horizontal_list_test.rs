//! Axis-aware native list engine through TestPlatform; no physical GUI claim.
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

struct Fixture {
    handle: ListState,
    axis: Axis,
    focus: FocusHandle,
    main: f32,
    block_wheel: bool,
    padded: bool,
    sizing: ListSizingBehavior,
    autoscroll: Rc<Cell<Option<usize>>>,
    cross: f32,
    extents: Rc<RefCell<BTreeMap<usize, f32>>>,
    calls: Rc<Cell<usize>>,
    painted: Rc<RefCell<BTreeMap<usize, Bounds<Pixels>>>>,
}
fn dimensions(axis: Axis, main: f32, cross: f32) -> Size<Pixels> {
    match axis {
        Axis::Horizontal => size(px(main), px(cross)),
        Axis::Vertical => size(px(cross), px(main)),
    }
}
fn offset(axis: Axis, main: f32) -> Point<Pixels> {
    match axis {
        Axis::Horizontal => point(px(main), px(0.)),
        Axis::Vertical => point(px(0.), px(main)),
    }
}
fn extent(i: usize) -> f32 {
    40. + (i % 3) as f32 * 20.
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let block_wheel = self.block_wheel;
        let axis = self.axis;
        let focus = self.focus.clone();
        let cross = self.cross;
        let extents = self.extents.clone();
        let calls = self.calls.clone();
        let painted = self.painted.clone();
        let autoscroll = self.autoscroll.clone();
        let viewport = dimensions(axis, self.main, cross);
        div().relative().size_full().child(
            div()
                .absolute()
                .left(px(10.))
                .top(px(20.))
                .w(viewport.width)
                .h(viewport.height)
                .child(
                    list(self.handle.clone(), move |i, _, _| {
                        calls.set(calls.get() + 1);
                        let main = extents
                            .borrow()
                            .get(&i)
                            .copied()
                            .unwrap_or_else(|| extent(i));
                        let item = dimensions(axis, main, cross);
                        let painted = painted.clone();
                        let autoscroll = autoscroll.clone();
                        div()
                            .id(("item", i))
                            .w(item.width)
                            .h(item.height)
                            .flex_none()
                            .on_scroll_wheel(move |_, _, cx| {
                                if block_wheel {
                                    cx.stop_propagation();
                                }
                            })
                            .when(i == 25, |element| element.track_focus(&focus))
                            .role(Role::Button)
                            .aria_label(format!("Item {i}"))
                            .child(format!("Item {i}"))
                            .child(
                                canvas(
                                    move |bounds, window, _| {
                                        painted.borrow_mut().insert(i, bounds);
                                        if autoscroll.get() == Some(i) {
                                            autoscroll.set(None);
                                            window.request_autoscroll(Bounds {
                                                origin: bounds.origin + offset(axis, -30.),
                                                size: dimensions(axis, 35., cross),
                                            });
                                        }
                                    },
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full(),
                            )
                            .into_any_element()
                    })
                    .with_sizing_behavior(self.sizing)
                    .size_full()
                    .when(self.padded, |element| match axis {
                        Axis::Vertical => element.pt(px(7.)).pb(px(13.)),
                        Axis::Horizontal => element.pl(px(7.)).pr(px(13.)),
                    }),
                ),
        )
    }
}
fn setup(app: &mut TestAppContext, axis: Axis) -> (Entity<Fixture>, &mut VisualTestContext) {
    let (view, cx) = app.add_window_view(|_, cx| Fixture {
        handle: ListState::new_for_axis(axis, 100_000, ListAlignment::Top, px(40.))
            .with_uniform_item_extent(px(60.)),
        axis,
        focus: cx.focus_handle(),
        main: 200.,
        cross: 80.,
        block_wheel: false,
        padded: false,
        sizing: ListSizingBehavior::Auto,
        autoscroll: Rc::default(),
        extents: Rc::default(),
        calls: Rc::default(),
        painted: Rc::default(),
    });
    cx.simulate_a11y_active(true);
    draw(&view, cx);
    (view, cx)
}
fn draw(view: &Entity<Fixture>, cx: &mut VisualTestContext) {
    cx.run_until_parked();
    view.read_with(cx, |v, _| {
        v.calls.set(0);
        v.painted.borrow_mut().clear();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |v, _| v.calls.get()) < 32,
        "bounded native rendering for 100k items"
    );
}
#[::core::prelude::v1::test]
fn axes_share_anchor_measurement_and_physical_scrollbar_coordinates() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let mut app = TestAppContext::single();
        let (view, cx) = setup(&mut app, axis);
        let handle = view.read_with(cx, |v, _| v.handle.clone());
        assert_eq!(handle.axis(), axis);
        let bounds = handle.viewport_bounds();
        assert_eq!(bounds.origin, point(px(10.), px(20.)));
        assert_eq!(bounds.size, dimensions(axis, 200., 80.));
        let first = view.read_with(cx, |v, _| v.painted.borrow()[&0]);
        let second = view.read_with(cx, |v, _| v.painted.borrow()[&1]);
        assert_eq!(first.origin, bounds.origin);
        assert_eq!(first.size, dimensions(axis, 40., 80.));
        assert_eq!(second.origin, bounds.origin + offset(axis, 40.));
        assert_eq!(handle.bounds_for_item(1), Some(second));
        assert_eq!(
            handle.max_offset_for_scrollbar().along(axis.invert()),
            px(0.)
        );
        assert!(handle.max_offset_for_scrollbar().along(axis) > px(100_000.));
        // Accessibility remains in physical coordinates, like actual item paint.
        let tree = cx.a11y_tree().unwrap();
        let second_ax = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Item 1"))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        let scale = cx.update(|w, _| f64::from(w.scale_factor()));
        assert_eq!(second_ax.x0, f64::from(f32::from(second.origin.x)) * scale);
        assert_eq!(second_ax.y0, f64::from(f32::from(second.origin.y)) * scale);
        assert_eq!(
            second_ax.width(),
            f64::from(f32::from(second.size.width)) * scale
        );
        handle.scroll_to(ListOffset {
            item_ix: 25,
            offset_in_item: px(7.),
        });
        draw(&view, cx);
        assert_eq!(handle.logical_scroll_top().item_ix, 25);
        assert_eq!(handle.logical_scroll_top().offset_in_item, px(7.));
        handle.scrollbar_drag_started();
        handle.set_offset_from_scrollbar(offset(axis, -140.));
        handle.scrollbar_drag_ended();
        draw(&view, cx);
        assert_eq!(
            handle.scroll_px_offset_for_scrollbar().along(axis.invert()),
            px(0.)
        );
        assert!(handle.scroll_px_offset_for_scrollbar().along(axis) < px(0.));
        // A cross-axis resize remeasures; a main-axis resize only changes the
        // viewport. Both retain the same leading item and absolute inner grip.
        let before = handle.logical_scroll_top();
        view.update(cx, |v, cx| {
            v.cross = 96.;
            cx.notify();
        });
        draw(&view, cx);
        assert_eq!(
            (
                handle.logical_scroll_top().item_ix,
                handle.logical_scroll_top().offset_in_item
            ),
            (before.item_ix, before.offset_in_item)
        );
        view.update(cx, |v, cx| {
            v.main = 230.;
            cx.notify();
        });
        draw(&view, cx);
        assert_eq!(
            (
                handle.logical_scroll_top().item_ix,
                handle.logical_scroll_top().offset_in_item
            ),
            (before.item_ix, before.offset_in_item)
        );
        view.read_with(cx, |v, _| {
            v.extents.borrow_mut().insert(before.item_ix, 150.);
        });
        handle.remeasure_items(before.item_ix..before.item_ix + 1);
        draw(&view, cx);
        assert_eq!(
            (
                handle.logical_scroll_top().item_ix,
                handle.logical_scroll_top().offset_in_item
            ),
            (before.item_ix, before.offset_in_item)
        );
        handle.set_follow_mode(FollowMode::Tail);
        handle.scroll_to_end();
        draw(&view, cx);
        assert!(handle.is_following_tail());
        handle.splice(100_000..100_000, 5);
        draw(&view, cx);
        assert!(handle.is_following_tail());
        assert_eq!(handle.is_scrolled_to_end(), Some(true));
        handle.scroll_by(px(-80.));
        draw(&view, cx);
        assert!(!handle.is_following_tail());
        cx.update(|w, _| w.remove_window());
    }
}

#[::core::prelude::v1::test]
fn main_axis_wheel_respects_child_dispatch_and_reveal_uses_physical_bounds() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let mut app = TestAppContext::single();
        let (view, cx) = setup(&mut app, axis);
        let handle = view.read_with(cx, |v, _| v.handle.clone());
        let wheel = |cx: &mut VisualTestContext, delta| {
            cx.simulate_event(ScrollWheelEvent {
                position: point(px(30.), px(40.)),
                delta: ScrollDelta::Pixels(delta),
                ..Default::default()
            })
        };
        wheel(cx, offset(axis.invert(), -50.));
        draw(&view, cx);
        assert_eq!(
            handle.scroll_px_offset_for_scrollbar(),
            point(px(0.), px(0.))
        );
        wheel(cx, offset(axis, -50.));
        draw(&view, cx);
        assert_eq!(handle.scroll_px_offset_for_scrollbar(), offset(axis, -50.));
        view.update(cx, |v, cx| {
            v.block_wheel = true;
            cx.notify();
        });
        draw(&view, cx);
        wheel(cx, offset(axis, -50.));
        draw(&view, cx);
        assert_eq!(
            handle.scroll_px_offset_for_scrollbar(),
            offset(axis, -50.),
            "child owns wheel first"
        );
        handle.scroll_to_reveal_item(1234);
        draw(&view, cx);
        let item = handle
            .bounds_for_item(1234)
            .expect("revealed item measured");
        let viewport = handle.viewport_bounds();
        assert!(item.origin.along(axis) >= viewport.origin.along(axis));
        assert!(
            item.origin.along(axis) + item.size.along(axis)
                <= viewport.origin.along(axis) + viewport.size.along(axis)
        );
        assert_eq!(handle.item_is_above_viewport(0), Some(true));
        assert_eq!(handle.item_is_below_viewport(1234), Some(false));
        handle.scroll_to_reveal_item(0);
        draw(&view, cx);
        assert_eq!(handle.logical_scroll_top().item_ix, 0);
        assert_eq!(handle.bounds_for_item(0).unwrap().origin, viewport.origin);
        cx.update(|w, _| w.remove_window());
    }
}

#[::core::prelude::v1::test]
fn padding_inferred_layout_and_scrollbar_endpoints_agree_on_both_axes() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for sizing in [ListSizingBehavior::Auto, ListSizingBehavior::Infer] {
            let mut app = TestAppContext::single();
            let (view, cx) = setup(&mut app, axis);
            let handle = view.read_with(cx, |v, _| v.handle.clone());
            view.update(cx, |v, cx| {
                v.main = 100.;
                v.padded = true;
                v.sizing = sizing;
                for i in 0..4 {
                    v.extents.borrow_mut().insert(i, 48.);
                }
                v.handle.reset_with_uniform_height(4, px(48.));
                cx.notify();
            });
            draw(&view, cx);
            let viewport = handle.viewport_bounds();
            let first = view.read_with(cx, |v, _| v.painted.borrow()[&0]);
            assert_eq!(first.origin, viewport.origin + offset(axis, 7.));
            assert_eq!(
                handle.bounds_for_item(0),
                Some(first),
                "public item bounds include leading padding"
            );
            assert_eq!(handle.max_offset_for_scrollbar(), offset(axis, 112.));
            handle.set_offset_from_scrollbar(offset(axis, -112.));
            draw(&view, cx);
            assert_eq!(handle.scroll_px_offset_for_scrollbar(), offset(axis, -112.));
            assert_eq!(handle.is_scrolled_to_end(), Some(true));
            let last = view.read_with(cx, |v, _| v.painted.borrow()[&3]);
            assert_eq!(
                last.origin.along(axis) + last.size.along(axis) + px(13.),
                viewport.origin.along(axis) + viewport.size.along(axis)
            );
            cx.update(|w, _| w.remove_window());
        }
    }
}

#[::core::prelude::v1::test]
fn child_autoscroll_before_its_leading_edge_reveals_previous_items_on_both_axes() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let mut app = TestAppContext::single();
        let (view, cx) = setup(&mut app, axis);
        let handle = view.read_with(cx, |v, _| v.handle.clone());
        let focus = view.read_with(cx, |v, _| v.focus.clone());
        handle.splice_focusable(25..26, [Some(focus.clone())]);
        cx.update(|w, cx| w.focus(&focus, cx));
        handle.scroll_to(ListOffset {
            item_ix: 25,
            offset_in_item: px(0.),
        });
        draw(&view, cx);
        // Settle the normal focus reveal before requesting a child subregion.
        view.read_with(cx, |v, _| v.autoscroll.set(Some(25)));
        draw(&view, cx);
        let leading = handle.logical_scroll_top();
        assert_eq!((leading.item_ix, leading.offset_in_item), (24, px(10.)));
        let item = view.read_with(cx, |v, _| v.painted.borrow()[&25]);
        assert_eq!(
            item.origin.along(axis) - px(30.),
            handle.viewport_bounds().origin.along(axis)
        );
        let tree = cx.a11y_tree().unwrap();
        let focused_node = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Item 25"))
            .unwrap();
        assert_eq!(
            tree.focus, focused_node.0,
            "retry retains one correct accessibility focus"
        );
        let scale = cx.update(|w, _| f64::from(w.scale_factor()));
        let bounds = focused_node.1.bounds().unwrap();
        assert_eq!(bounds.x0, f64::from(f32::from(item.origin.x)) * scale);
        assert_eq!(bounds.y0, f64::from(f32::from(item.origin.y)) * scale);
        cx.update(|w, cx| w.blur(cx));
        handle.splice(0..0, 2);
        draw(&view, cx);
        assert_eq!(
            handle.logical_scroll_top().item_ix,
            26,
            "prepend retains existing logical leading item"
        );
        cx.update(|w, _| w.remove_window());
    }
}
