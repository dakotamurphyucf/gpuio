//! Actual element layout/paint/input through TestPlatform, not an OS window.
use super::*;
use gpuio_protocol::scrollbar::{Appearance, Axis as Selection, Mode, Motion};
struct Fixture {
    state: Shared,
    handle: ScrollHandle,
    editor: FocusHandle,
    content_focus: FocusHandle,
    omitted: bool,
    viewport_size: Size<Pixels>,
    content_size: Size<Pixels>,
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.state.borrow_mut().begin_frame();
        div()
            .size_full()
            .child(
                div()
                    .relative()
                    .w(self.viewport_size.width)
                    .h(self.viewport_size.height)
                    .overflow_hidden()
                    .child(
                        div()
                            .id("viewport")
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .overflow_scroll()
                            .track_scroll(&self.handle)
                            .child(
                                div()
                                    .id("content-focus")
                                    .track_focus(&self.content_focus)
                                    .w(self.content_size.width)
                                    .h(self.content_size.height)
                                    .flex_none(),
                            ),
                    )
                    .when(!self.omitted, |div| {
                        div.child(element(&self.state, 0x445566ff))
                    }),
            )
            .child(
                div()
                    .id("editor-focus")
                    .track_focus(&self.editor)
                    .w(px(100.))
                    .h(px(20.)),
            )
    }
}
fn setup(app: &mut TestAppContext) -> (Entity<Fixture>, &mut VisualTestContext) {
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|window, cx| {
        let handle = ScrollHandle::new();
        let config = Arc::new(Config {
            label: "Transcript".into(),
            axis: Selection::Both,
            mode: Mode::Always,
            appearance: Appearance::default(),
            motion: Motion::default(),
        });
        let state = State::new(
            "test-scrollbar".into(),
            config,
            Rc::new(handle.clone()),
            Viewport::Handle,
            window,
            cx,
        )
        .unwrap();
        Fixture {
            state,
            handle,
            editor: cx.focus_handle(),
            content_focus: cx.focus_handle(),
            omitted: false,
            viewport_size: size(px(200.), px(100.)),
            content_size: size(px(800.), px(400.)),
        }
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    cx.simulate_a11y_active(true);
    draw(&view, cx);
    (view, cx)
}
fn draw(view: &Entity<Fixture>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let state = view.read(cx).state.clone();
        state.borrow_mut().finish_frame(window, cx);
    });
}
fn focus_axis(view: &Entity<Fixture>, cx: &mut VisualTestContext, axis: Axis) {
    cx.update(|window, cx| {
        let focus = view.read(cx).state.borrow().focus(axis).clone();
        window.focus(&focus, cx);
    });
    draw(view, cx);
}
fn offset(view: &Entity<Fixture>, cx: &mut VisualTestContext) -> Point<Pixels> {
    view.read_with(cx, |v, _| v.handle.offset())
}
fn thumb(view: &Entity<Fixture>, cx: &mut VisualTestContext, axis: Axis) -> Point<Pixels> {
    view.read_with(cx, |v, _| {
        let s = v.state.borrow();
        let m = s.measured.expect("measured viewport");
        let b = get_bar(m, axis).unwrap();
        absolute(b.thumb, m.viewport).center()
    })
}
#[::core::prelude::v1::test]
fn actual_ranges_keyboard_and_accessibility_mutate_the_same_scroll_handle() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app);
    let tree = cx.a11y_tree().expect("native AX tree");
    let bars: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, n)| n.role() == accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(bars.len(), 2);
    let (_, vertical) = bars
        .iter()
        .find(|(_, n)| n.label() == Some("Transcript — vertical"))
        .unwrap();
    assert_eq!(vertical.numeric_value(), Some(0.));
    assert_eq!(vertical.min_numeric_value(), Some(0.));
    assert_eq!(vertical.max_numeric_value(), Some(300.));
    cx.update(|window, cx| {
        window.blur(cx);
        window.focus_next(cx);
        assert!(
            view.read(cx)
                .state
                .borrow()
                .focus(Axis::Horizontal)
                .is_focused(window)
        );
        window.focus_next(cx);
        assert!(
            view.read(cx)
                .state
                .borrow()
                .focus(Axis::Vertical)
                .is_focused(window)
        );
    });
    focus_axis(&view, cx, Axis::Vertical);
    cx.simulate_keystrokes("end");
    draw(&view, cx);
    assert_eq!(offset(&view, cx), point(px(0.), px(-300.)));
    cx.simulate_keystrokes("home");
    draw(&view, cx);
    assert_eq!(offset(&view, cx).y, px(0.));
    let tree = cx.a11y_tree().unwrap();
    let id = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Transcript — horizontal"))
        .unwrap()
        .0;
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::SetValue,
        target_node: id,
        target_tree: accesskit::TreeId::ROOT,
        data: Some(accesskit::ActionData::NumericValue(123.)),
    });
    cx.run_until_parked();
    draw(&view, cx);
    assert_eq!(offset(&view, cx).x, px(-123.));
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        state.borrow_mut().close(window, cx);
        window.remove_window();
    });
}
#[::core::prelude::v1::test]
fn dragging_keeps_editor_focus_and_omission_releases_capture() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app);
    cx.update(|window, cx| {
        let editor = view.read(cx).editor.clone();
        window.focus(&editor, cx);
    });
    draw(&view, cx);
    let start = thumb(&view, cx, Axis::Vertical);
    cx.simulate_mouse_move(start, None, Default::default());
    draw(&view, cx);
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    assert!(view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    assert!(
        cx.update(|w, cx| view.read(cx).editor.is_focused(w)),
        "scrollbar pointer must preserve editor focus"
    );
    cx.simulate_mouse_move(
        start + point(px(0.), px(20.)),
        Some(MouseButton::Left),
        Default::default(),
    );
    draw(&view, cx);
    assert!(offset(&view, cx).y < px(0.));
    assert!(cx.update(|w, _| w.captured_hitbox().is_some()));
    view.update(cx, |v, cx| {
        v.omitted = true;
        cx.notify();
    });
    draw(&view, cx);
    assert!(!view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
    let old = offset(&view, cx);
    cx.simulate_mouse_move(
        start + point(px(0.), px(80.)),
        Some(MouseButton::Left),
        Default::default(),
    );
    assert_eq!(offset(&view, cx), old);
    cx.update(|window, _| window.remove_window());
}

#[::core::prelude::v1::test]
fn slide_input_uses_painted_thumb_and_axis_removal_rejects_stale_actions() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app);
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        let mut config = (*state.borrow().config).clone();
        config.mode = Mode::Hover;
        config.motion.enter_ms = 200;
        config.motion.entrance = gpuio_protocol::scrollbar::Entrance::SlideAndFade;
        config.motion.thumb_hover_entrance = gpuio_protocol::scrollbar::Entrance::SlideAndFade;
        config.appearance.thumb_hover.width = Some(6.);
        state
            .borrow_mut()
            .reconcile(Arc::new(config), Policy::default(), window, cx)
            .unwrap();
    });
    draw(&view, cx);
    cx.simulate_mouse_move(point(px(196.), px(70.)), None, Default::default());
    draw(&view, cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(100));
    draw(&view, cx);
    let (painted, canonical) = view.read_with(cx, |v, _| {
        let s = v.state.borrow();
        let m = s.measured.unwrap();
        (
            s.painted_bars[1].unwrap(),
            absolute(get_bar(m, Axis::Vertical).unwrap().thumb, m.viewport),
        )
    });
    assert!(painted.translation.x > px(0.));
    let ghost = point(canonical.left() + px(0.5), canonical.center().y);
    assert!(!painted.thumb.contains(&ghost));
    cx.simulate_mouse_down(ghost, MouseButton::Left, Default::default());
    assert!(!view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    cx.simulate_mouse_up(ghost, MouseButton::Left, Default::default());
    cx.simulate_mouse_down(
        painted.thumb.center(),
        MouseButton::Left,
        Default::default(),
    );
    assert!(view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    let tree = cx.a11y_tree().unwrap();
    let vertical = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Transcript — vertical"))
        .unwrap()
        .0;
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        let mut config = (*state.borrow().config).clone();
        config.axis = Selection::Horizontal;
        state
            .borrow_mut()
            .reconcile(Arc::new(config), Policy::default(), window, cx)
            .unwrap();
        assert!(!state.borrow().is_dragging());
        assert!(window.captured_hitbox().is_none());
        assert!(
            !state
                .borrow_mut()
                .adjust(Axis::Vertical, input::Adjustment::Last, window, cx)
        );
    });
    let before = offset(&view, cx);
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::SetValue,
        target_node: vertical,
        target_tree: accesskit::TreeId::ROOT,
        data: Some(accesskit::ActionData::NumericValue(250.)),
    });
    cx.run_until_parked();
    assert_eq!(offset(&view, cx), before);
    draw(&view, cx);
    assert_eq!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .filter(|(_, n)| n.role() == accesskit::Role::ScrollBar)
            .count(),
        1
    );
    cx.update(|window, _| window.remove_window());
}

#[::core::prelude::v1::test]
fn narrowing_hover_keeps_a_stable_boundary_but_does_not_expand_the_drag_target() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app);
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        let mut config = (*state.borrow().config).clone();
        config.appearance.thumb.width = Some(8.);
        config.appearance.thumb_hover.width = Some(2.);
        state
            .borrow_mut()
            .reconcile(Arc::new(config), Policy::default(), window, cx)
            .unwrap();
    });
    draw(&view, cx);
    let point = view.read_with(cx, |v, _| {
        let thumb = v.state.borrow().painted_bars[1].unwrap().thumb;
        point(thumb.left() + px(1.), thumb.center().y)
    });
    for _ in 0..4 {
        cx.simulate_mouse_move(point, None, Default::default());
        draw(&view, cx);
        view.read_with(cx, |v, _| {
            let s = v.state.borrow();
            assert_eq!(s.clocks[1].interaction(), Interaction::ThumbHover);
            assert!(!s.painted_bars[1].unwrap().thumb.contains(&point));
        });
    }
    cx.simulate_mouse_down(point, MouseButton::Left, Default::default());
    assert!(!view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    cx.simulate_mouse_up(point, MouseButton::Left, Default::default());
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        state.borrow_mut().close(window, cx);
        window.remove_window();
    });
}

#[::core::prelude::v1::test]
fn real_virtual_list_drag_hooks_and_tail_follow_survive_deactivation() {
    struct ListFixture {
        state: Shared,
        handle: ListState,
    }
    impl Render for ListFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.state.borrow_mut().begin_frame();
            div()
                .relative()
                .w(px(200.))
                .h(px(100.))
                .overflow_hidden()
                .child(
                    list(self.handle.clone(), |_, _, _| {
                        div().w_full().h(px(20.)).flex_none().into_any_element()
                    })
                    .size_full(),
                )
                .child(element(&self.state, 0x445566ff))
        }
    }
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|window, cx| {
        let handle =
            ListState::new(100, ListAlignment::Top, px(20.)).with_uniform_item_height(px(20.));
        handle.set_follow_mode(FollowMode::Tail);
        let config = Arc::new(Config {
            label: "Messages".into(),
            axis: Selection::Vertical,
            mode: Mode::Always,
            appearance: Appearance::default(),
            motion: Motion::default(),
        });
        let state = State::new(
            "list-scrollbar".into(),
            config,
            Rc::new(handle.clone()),
            Viewport::Handle,
            window,
            cx,
        )
        .unwrap();
        ListFixture { state, handle }
    });
    let draw_list = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            let state = view.read(cx).state.clone();
            state.borrow_mut().finish_frame(window, cx);
        });
    };
    cx.update(|w, _| w.activate_window());
    cx.run_until_parked();
    draw_list(cx);
    assert!(view.read_with(cx, |v, _| v.handle.is_following_tail()));
    let start = view.read_with(cx, |v, _| {
        let s = v.state.borrow();
        let m = s.measured.unwrap();
        absolute(m.bars.vertical.unwrap().thumb, m.viewport).center()
    });
    cx.simulate_mouse_move(start, None, Default::default());
    draw_list(cx);
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    assert!(view.read_with(cx, |v, _| v.handle.is_scrollbar_dragging()));
    cx.simulate_mouse_move(
        start - point(px(0.), px(20.)),
        Some(MouseButton::Left),
        Default::default(),
    );
    draw_list(cx);
    assert!(!view.read_with(cx, |v, _| v.handle.is_following_tail()));
    let position = view.read_with(cx, |v, _| v.handle.logical_scroll_top());
    assert!(position.item_ix > 0 && position.item_ix < 95);
    cx.deactivate_window();
    cx.run_until_parked();
    assert!(!view.read_with(cx, |v, _| v.handle.is_scrollbar_dragging()));
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
    cx.update(|w, _| w.activate_window());
    cx.run_until_parked();
    draw_list(cx);
    cx.update(|w, cx| {
        let f = view.read(cx).state.borrow().focus(Axis::Vertical).clone();
        w.focus(&f, cx);
    });
    draw_list(cx);
    cx.simulate_keystrokes("end");
    draw_list(cx);
    assert!(view.read_with(cx, |v, _| v.handle.is_following_tail()));
    assert!(!view.read_with(cx, |v, _| v.handle.is_scrollbar_dragging()));
    cx.update(|w, cx| {
        let state = view.read(cx).state.clone();
        state.borrow_mut().close(w, cx);
        w.remove_window();
    });
}

#[::core::prelude::v1::test]
fn live_policy_gates_old_actions_preserves_offsets_and_close_cannot_revive() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app);
    let vertical = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Transcript — vertical"))
        .unwrap()
        .0;
    let set_value = |cx: &mut VisualTestContext, data| {
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::SetValue,
            target_node: vertical,
            target_tree: accesskit::TreeId::ROOT,
            data,
        });
        cx.run_until_parked();
    };
    for data in [
        None,
        Some(accesskit::ActionData::NumericValue(f64::NAN)),
        Some(accesskit::ActionData::NumericValue(f64::INFINITY)),
    ] {
        set_value(cx, data);
        assert_eq!(offset(&view, cx).y, px(0.));
    }
    let start = thumb(&view, cx, Axis::Vertical);
    cx.simulate_mouse_move(start, None, Default::default());
    draw(&view, cx);
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    assert!(view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        let config = state.borrow().config.clone();
        state
            .borrow_mut()
            .reconcile(
                config,
                Policy {
                    pointer: false,
                    ..Policy::default()
                },
                window,
                cx,
            )
            .unwrap();
        assert!(!state.borrow().is_dragging());
        assert!(window.captured_hitbox().is_none());
        assert_eq!(state.borrow().clocks[1].interaction(), Interaction::Rest);
    });
    set_value(cx, Some(accesskit::ActionData::NumericValue(123.)));
    assert_eq!(
        offset(&view, cx).y,
        px(-123.),
        "pointer policy does not disable AX"
    );
    draw(&view, cx);
    focus_axis(&view, cx, Axis::Vertical);
    cx.simulate_keystrokes("home");
    assert_eq!(offset(&view, cx).y, px(0.));
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        let config = state.borrow().config.clone();
        state
            .borrow_mut()
            .reconcile(
                config,
                Policy {
                    enabled: false,
                    ..Policy::default()
                },
                window,
                cx,
            )
            .unwrap();
        assert!(!state.borrow().focus(Axis::Vertical).is_focused(window));
    });
    set_value(cx, Some(accesskit::ActionData::NumericValue(234.)));
    assert_eq!(
        offset(&view, cx).y,
        px(0.),
        "old AX callbacks obey current disabled gate"
    );
    draw(&view, cx);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == accesskit::Role::ScrollBar)
    );
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        state.borrow_mut().close(window, cx);
        let config = state.borrow().config.clone();
        state
            .borrow_mut()
            .reconcile(config, Policy::default(), window, cx)
            .unwrap();
    });
    draw(&view, cx);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == accesskit::Role::ScrollBar)
    );
    assert_eq!(offset(&view, cx).y, px(0.));
    cx.update(|window, _| window.remove_window());
}

#[::core::prelude::v1::test]
fn hidden_range_is_focusable_and_overflow_removal_retires_it() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app);
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        let mut config = (*state.borrow().config).clone();
        config.mode = Mode::Scrolling;
        config.motion.idle_ms = 0;
        state
            .borrow_mut()
            .reconcile(Arc::new(config), Policy::default(), window, cx)
            .unwrap();
    });
    draw(&view, cx);
    assert!(!view.read_with(cx, |v, _| v.state.borrow().clocks[1].accepts_pointer()));
    let point = thumb(&view, cx, Axis::Vertical);
    cx.simulate_mouse_move(point, None, Default::default());
    cx.simulate_mouse_down(point, MouseButton::Left, Default::default());
    assert!(!view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    assert!(
        cx.update(|w, cx| view.read(cx).content_focus.is_focused(w)),
        "hidden bars must let underlying content receive its default focus"
    );
    cx.simulate_mouse_up(point, MouseButton::Left, Default::default());
    let vertical = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Transcript — vertical"))
        .unwrap()
        .0;
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Focus,
        target_node: vertical,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    draw(&view, cx);
    assert!(view.read_with(cx, |v, _| v.state.borrow().clocks[1].accepts_pointer()));
    assert!(cx.update(|w, cx| {
        view.read(cx)
            .state
            .borrow()
            .focus(Axis::Vertical)
            .is_focused(w)
    }));
    cx.simulate_keystrokes("pagedown");
    draw(&view, cx);
    assert_eq!(offset(&view, cx).y, px(-100.));
    view.update(cx, |v, cx| {
        v.content_size = size(px(800.), px(50.));
        cx.notify();
    });
    draw(&view, cx);
    assert!(!cx.update(|w, cx| {
        view.read(cx)
            .state
            .borrow()
            .focus(Axis::Vertical)
            .is_focused(w)
    }));
    let roles: Vec<_> = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .filter(|(_, n)| n.role() == accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].1.label(), Some("Transcript — horizontal"));
    view.update(cx, |v, cx| {
        v.viewport_size.height = px(0.);
        cx.notify();
    });
    draw(&view, cx);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == accesskit::Role::ScrollBar)
    );
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        state.borrow_mut().close(window, cx);
        window.remove_window();
    });
}

#[::core::prelude::v1::test]
fn captured_drag_remeasures_native_resize_and_preserves_the_other_axis() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app);
    view.update(cx, |v, _| v.handle.set_offset(point(px(-50.), px(0.))));
    draw(&view, cx);
    let start = thumb(&view, cx, Axis::Vertical);
    assert_eq!(start.y, px(28.));
    cx.simulate_mouse_move(start, None, Default::default());
    draw(&view, cx);
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    assert!(view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    view.update(cx, |v, cx| {
        v.viewport_size.height = px(200.);
        v.content_size.height = px(1000.);
        cx.notify();
    });
    draw(&view, cx);
    cx.simulate_mouse_move(
        point(start.x, px(100.)),
        Some(MouseButton::Left),
        Default::default(),
    );
    // New track: 184px; inset:4; thumb:48; travel:128; retained grip:24.
    // New maximum:800, so (100-4-24)/128*800 = 450, not the old geometry.
    assert_eq!(offset(&view, cx), point(px(-50.), px(-450.)));
    draw(&view, cx);
    // The final mouseup coordinate is authoritative even without a last move.
    cx.simulate_mouse_up(
        point(start.x, px(108.)),
        MouseButton::Left,
        Default::default(),
    );
    assert_eq!(offset(&view, cx), point(px(-50.), px(-500.)));
    assert!(!view.read_with(cx, |v, _| v.state.borrow().is_dragging()));
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
    cx.update(|window, cx| {
        let state = view.read(cx).state.clone();
        state.borrow_mut().close(window, cx);
        window.remove_window();
    });
}
