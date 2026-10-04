#![cfg(feature = "native-image-tests")]
use gpui::{
    Bounds, Context, Entity, IntoElement, Pixels, Render, TestAppContext, VisualTestContext,
    Window, canvas, div, point, prelude::*, px,
};
use gpui_base::ElementExt as _;
use gpuio_native::split_group_widget::{self as widget, Appearance, Content, Policy, Shared};
use gpuio_protocol::{
    split::Axis,
    split_group::{Config, Panel, ResizeRequest, Snapshot, Source},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};
fn config(axis: Axis) -> Config {
    Config {
        label: "Workspace".into(),
        axis,
        keyboard_step: 10.,
        reset_generation: 0,
        resize: None,
        panels: (0..3)
            .map(|i| Panel {
                id: format!("p{i}"),
                label: format!("Panel {i}"),
                initial_size: Some((i + 1) as f64 * 100.),
                minimum_size: 50.,
                maximum_size: 500.,
                visible: true,
            })
            .collect(),
    }
}
struct Harness {
    state: Shared,
    config: Arc<Config>,
    policy: Policy,
    extent: f32,
    events: Rc<RefCell<Vec<Snapshot>>>,
    bounds: Rc<RefCell<BTreeMap<String, Bounds<Pixels>>>>,
    appearance: Appearance,
    clicks: Rc<Cell<usize>>,
    hidden: bool,
    cross: f32,
    decorated: bool,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.state.borrow_mut().begin_frame();
        let contents = self
            .config
            .panels
            .iter()
            .map(|p| {
                let bounds = self.bounds.clone();
                let id = p.id.clone();
                let button_id = format!("button:{}", p.id);
                let button_bounds = bounds.clone();
                let clicks = self.clicks.clone();
                let panel = div()
                    .id(gpui::SharedString::from(format!("content:{}", p.id)))
                    .size_full()
                    .child(
                        div()
                            .id("child-button")
                            .w(px(30.))
                            .h(px(25.))
                            .on_prepaint(move |rect, _, _| {
                                button_bounds.borrow_mut().insert(button_id, rect);
                            })
                            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                            .child("Click"),
                    )
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |rect, _, _, _| {
                                bounds.borrow_mut().insert(id.clone(), rect);
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .into_any_element();
                Content {
                    panel,
                    handle: if self.decorated && p.id == "p0" {
                        Some(
                            div()
                                .size(px(5.))
                                .bg(gpui::rgb(0xff00ff))
                                .into_any_element(),
                        )
                    } else {
                        None
                    },
                }
            })
            .collect();
        let events = self.events.clone();
        let bounds = self.bounds.clone();
        let group = widget::element(widget::Render {
            state: self.state.clone(),
            contents,
            appearance: Arc::new(self.appearance.clone()),
            observe: Rc::new(move |snapshot, _, _| events.borrow_mut().push(snapshot)),
            record_focus: Rc::new(move |id, _, rect, _, _| {
                bounds.borrow_mut().insert(format!("handle:{id}"), rect);
            }),
            record_visibility: Rc::new(|_, _, _, _| {}),
            decorate_panel: Rc::new(|_, panel| panel.into_any_element()),
        })
        .unwrap();
        let mut sized = div().relative().child(group);
        sized = match self.config.axis {
            Axis::Horizontal => sized.w(px(self.extent)).h(px(self.cross)),
            Axis::Vertical => sized.w(px(self.cross)).h(px(self.extent)),
        };
        if self.hidden {
            sized = sized.invisible();
        }
        let weak = Rc::downgrade(&self.state);
        div().size_full().child(sized).child(
            canvas(
                |_, _, _| (),
                move |_, _, w, cx| {
                    let weak = weak.clone();
                    w.defer(cx, move |w, cx| {
                        if let Some(state) = weak.upgrade() {
                            state.borrow_mut().finish_frame(w, cx);
                        }
                    });
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn setup(axis: Axis, f: impl FnOnce(&Entity<Harness>, &mut VisualTestContext)) {
    let mut app = TestAppContext::single();
    let config = Arc::new(config(axis));
    let (owner, cx) = app.add_window_view(|w, cx| Harness {
        state: widget::State::new(config.clone(), w, cx).unwrap(),
        config,
        policy: Policy::default(),
        extent: 600.,
        events: Rc::default(),
        bounds: Rc::default(),
        appearance: Appearance::default(),
        clicks: Rc::new(Cell::new(0)),
        hidden: false,
        cross: 120.,
        decorated: false,
    });
    cx.simulate_a11y_active(true);
    cx.update(|w, _| w.activate_window());
    draw(cx);
    f(&owner, cx);
    cx.update(|w, cx| owner.update(cx, |h, cx| h.state.borrow_mut().close(w, cx)));
    cx.update(|w, _| w.remove_window());
    drop(owner);
    cx.cx.update(|_| ());
    cx.run_until_parked();
}
fn update(owner: &Entity<Harness>, cx: &mut VisualTestContext, f: impl FnOnce(&mut Harness)) {
    cx.update(|w, cx| {
        owner.update(cx, |h, cx| {
            f(h);
            h.state
                .borrow_mut()
                .reconcile(h.config.clone(), h.policy, w, cx)
                .unwrap();
            cx.notify();
        })
    });
    draw(cx);
}
fn bounds(owner: &Entity<Harness>, cx: &VisualTestContext, id: &str) -> Bounds<Pixels> {
    owner.read_with(cx, |h, _| h.bounds.borrow()[id])
}
fn extents(owner: &Entity<Harness>, cx: &VisualTestContext) -> Vec<f32> {
    owner.read_with(cx, |h, _| {
        h.config
            .panels
            .iter()
            .filter(|p| p.visible)
            .map(|p| {
                let b = h.bounds.borrow()[&p.id];
                f32::from(if h.config.axis == Axis::Horizontal {
                    b.size.width
                } else {
                    b.size.height
                })
            })
            .collect()
    })
}
fn events(owner: &Entity<Harness>, cx: &VisualTestContext) -> Vec<Snapshot> {
    owner.read_with(cx, |h, _| h.events.borrow_mut().drain(..).collect())
}
fn down(owner: &Entity<Harness>, cx: &mut VisualTestContext, id: &str) -> gpui::Point<Pixels> {
    let position = bounds(owner, cx, id).center();
    cx.simulate_mouse_move(position, None, Default::default());
    cx.simulate_mouse_down(position, gpui::MouseButton::Left, Default::default());
    draw(cx);
    position
}
fn moved(cx: &mut VisualTestContext, p: gpui::Point<Pixels>) {
    cx.simulate_mouse_move(p, Some(gpui::MouseButton::Left), Default::default());
    draw(cx);
}
fn up(cx: &mut VisualTestContext, p: gpui::Point<Pixels>) {
    cx.simulate_mouse_up(p, gpui::MouseButton::Left, Default::default());
    draw(cx);
}
#[test]
fn measured_first_frame_and_container_resize_have_no_followup_jump_or_idle_clock() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        setup(axis, |owner, cx| {
            assert_eq!(extents(owner, cx), vec![100., 200., 300.]);
            update(owner, cx, |h| h.extent = 900.);
            assert_eq!(extents(owner, cx), vec![150., 300., 450.]);
            draw(cx);
            assert_eq!(extents(owner, cx), vec![150., 300., 450.]);
            assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
            let visible = cx
                .a11y_tree()
                .unwrap()
                .nodes
                .into_iter()
                .filter(|(_, n)| n.role() == gpui::accesskit::Role::Splitter)
                .count();
            assert_eq!(visible, 2);
        });
    }
}
#[test]
fn captured_drag_previews_then_commits_once_and_cancel_restores() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        setup(axis, |owner, cx| {
            let start = down(owner, cx, "handle:p0");
            assert!(cx.update(|w, _| w.captured_hitbox().is_some()));
            let end = start
                + if axis == Axis::Horizontal {
                    point(px(60.), px(80.))
                } else {
                    point(px(80.), px(60.))
                };
            moved(cx, end);
            assert_eq!(extents(owner, cx), vec![160., 140., 300.]);
            assert!(events(owner, cx).is_empty());
            up(cx, end);
            let observed = events(owner, cx);
            assert_eq!(observed.len(), 1);
            assert_eq!(observed[0].source, Source::Pointer);
            assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
            let start = down(owner, cx, "handle:p0");
            moved(cx, start + point(px(30.), px(30.)));
            cx.simulate_keystrokes("escape");
            draw(cx);
            assert_eq!(extents(owner, cx), vec![160., 140., 300.]);
            assert!(events(owner, cx).is_empty());
            assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
            let _ = down(owner, cx, "handle:p0");
            // Coordinates deliberately outside the test window: native capture
            // keeps delivery and clamps both partitions to their feasible range.
            let outside = point(px(2000.), px(2000.));
            moved(cx, outside);
            assert_eq!(extents(owner, cx), vec![500., 50., 50.]);
            assert!(events(owner, cx).is_empty());
            up(cx, outside);
            assert_eq!(events(owner, cx).len(), 1);
            assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        });
    }
}
#[test]
fn keyboard_accessibility_and_child_input_use_separate_targets() {
    setup(Axis::Horizontal, |owner, cx| {
        let position = bounds(owner, cx, "button:p1").center();
        cx.simulate_mouse_move(position, None, Default::default());
        cx.simulate_click(position, Default::default());
        draw(cx);
        assert_eq!(
            owner.read_with(cx, |h, _| h.clicks.get()),
            1,
            "position={position:?} bounds={:?}",
            owner.read_with(cx, |h, _| h.bounds.borrow().clone())
        );
        assert!(events(owner, cx).is_empty());
        let focus = owner.read_with(cx, |h, _| h.state.borrow().focus("p0").unwrap().clone());
        cx.update(|w, cx| w.focus(&focus, cx));
        cx.simulate_keystrokes("right");
        draw(cx);
        assert_eq!(extents(owner, cx), vec![110., 190., 300.]);
        assert_eq!(events(owner, cx)[0].source, Source::Keyboard);
        let splitter = cx
            .a11y_tree()
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, n)| n.label() == Some("Resize Panel 0 and Panel 1"))
            .unwrap();
        assert_eq!(splitter.1.numeric_value(), Some(110.));
        cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
            action: gpui::accesskit::Action::SetValue,
            target_node: splitter.0,
            target_tree: gpui::accesskit::TreeId::ROOT,
            data: Some(gpui::accesskit::ActionData::NumericValue(180.)),
        });
        draw(cx);
        assert_eq!(extents(owner, cx), vec![180., 120., 300.]);
        assert_eq!(events(owner, cx)[0].source, Source::Accessibility);
        cx.simulate_keystrokes("home");
        draw(cx);
        assert_eq!(extents(owner, cx)[0], 50.);
        events(owner, cx);
        cx.simulate_keystrokes("end");
        draw(cx);
        assert_eq!(extents(owner, cx)[0], 500.);
    });
}
#[test]
fn policies_structure_deactivation_and_close_cancel_native_capture() {
    setup(Axis::Horizontal, |owner, cx| {
        let start = down(owner, cx, "handle:p0");
        moved(cx, start + point(px(30.), px(0.)));
        cx.deactivate_window();
        draw(cx);
        assert_eq!(extents(owner, cx), vec![100., 200., 300.]);
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        cx.update(|w, _| w.activate_window());
        draw(cx);
        let start = down(owner, cx, "handle:p0");
        moved(cx, start + point(px(20.), px(0.)));
        update(owner, cx, |h| h.policy.pointer = false);
        assert_eq!(extents(owner, cx), vec![100., 200., 300.]);
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        update(owner, cx, |h| h.policy.pointer = true);
        let start = down(owner, cx, "handle:p0");
        moved(cx, start + point(px(20.), px(0.)));
        update(owner, cx, |h| {
            let mut c = (*h.config).clone();
            c.panels.reverse();
            h.config = Arc::new(c);
        });
        assert_eq!(extents(owner, cx), vec![300., 200., 100.]);
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        assert!(events(owner, cx).is_empty());
        let start = down(owner, cx, "handle:p1");
        moved(cx, start + point(px(20.), px(0.)));
        cx.update(|w, cx| owner.update(cx, |h, cx| h.state.borrow_mut().close(w, cx)));
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
    });
}
#[test]
fn resize_requests_and_visibility_are_keyed_and_emit_after_paint_once() {
    setup(Axis::Horizontal, |owner, cx| {
        let focus = owner.read_with(cx, |h, _| h.state.borrow().focus("p0").unwrap().clone());
        update(owner, cx, |h| {
            let mut c = (*h.config).clone();
            c.resize = Some(ResizeRequest {
                id: "p1".into(),
                size: 250.,
                serial: 1,
            });
            h.config = Arc::new(c);
        });
        assert_eq!(extents(owner, cx), vec![100., 250., 250.]);
        assert_eq!(events(owner, cx)[0].source, Source::Request(1));
        draw(cx);
        assert!(events(owner, cx).is_empty());
        update(owner, cx, |h| {
            let mut c = (*h.config).clone();
            c.panels[1].visible = false;
            h.config = Arc::new(c);
        });
        assert_eq!(
            cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .filter(|(_, n)| n.role() == gpui::accesskit::Role::Splitter)
                .count(),
            1
        );
        assert_eq!(
            focus,
            owner.read_with(cx, |h, _| h.state.borrow().focus("p0").unwrap().clone())
        );
    });
}

#[test]
fn zero_cross_size_hidden_and_clipped_handles_do_not_keep_capture_focus_or_requests() {
    setup(Axis::Horizontal, |owner, cx| {
        let start = down(owner, cx, "handle:p0");
        moved(cx, start + point(px(30.), px(0.)));
        update(owner, cx, |h| {
            h.cross = 0.;
            let mut c = (*h.config).clone();
            c.resize = Some(ResizeRequest {
                id: "p1".into(),
                size: 260.,
                serial: 1,
            });
            h.config = Arc::new(c);
        });
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        assert!(events(owner, cx).is_empty());
        update(owner, cx, |h| h.cross = 120.);
        assert_eq!(extents(owner, cx), vec![100., 260., 240.]);
        assert_eq!(events(owner, cx)[0].source, Source::Request(1));
        let start = down(owner, cx, "handle:p0");
        moved(cx, start + point(px(20.), px(0.)));
        update(owner, cx, |h| {
            h.hidden = true;
            h.policy.visible = false;
        });
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        assert!(events(owner, cx).is_empty());
        update(owner, cx, |h| {
            h.hidden = false;
            h.policy.visible = true;
        });
        assert_eq!(extents(owner, cx), vec![100., 260., 240.]);
        update(owner, cx, |h| {
            h.extent = 100.;
            let mut c = (*h.config).clone();
            for p in &mut c.panels {
                p.minimum_size = 200.;
                p.initial_size = Some(200.);
            }
            h.config = Arc::new(c);
        });
        assert_eq!(extents(owner, cx), vec![200.; 3]);
        let nodes = cx.a11y_tree().unwrap().nodes;
        let splitters: Vec<_> = nodes
            .iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::Splitter)
            .collect();
        assert!(splitters.iter().all(|(_, n)| n.is_hidden()));
        let focus = owner.read_with(cx, |h, _| h.state.borrow().focus("p0").unwrap().clone());
        assert!(!cx.update(|w, _| focus.is_focused(w)));
        if let Some((id, _)) = splitters.first() {
            cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
                action: gpui::accesskit::Action::Increment,
                target_node: *id,
                target_tree: gpui::accesskit::TreeId::ROOT,
                data: None,
            });
            draw(cx);
        }
        assert!(events(owner, cx).is_empty());
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    });
}
#[test]
fn handle_paint_overrides_decorations_and_disabled_accessibility_are_native() {
    setup(Axis::Horizontal, |owner, cx| {
        update(owner, cx, |h| {
            h.appearance.thickness = 3.;
            h.appearance.hit_extent = 16.;
            use gpuio_protocol::v1::{Color, Field, Fill, Style};
            let bg = |rgba| Field::Background(Fill::Solid(Color::Rgba(rgba)));
            h.appearance.handle_style = vec![
                Style::Fields(vec![bg(0xff0000ff)]),
                Style::State(1, vec![bg(0x00ff00ff)]),
            ];
            h.appearance.item_styles =
                vec![("p1".into(), vec![Style::Fields(vec![bg(0x0000ffff)])])];
        });
        assert_eq!(bounds(owner, cx, "handle:p0").size.width, px(16.));
        cx.update(|w, _| {
            for color in [0xff0000, 0x0000ff] {
                let q = w
                    .painted_quads()
                    .into_iter()
                    .find(|q| q.background.as_solid() == Some(gpui::rgb(color).into()))
                    .unwrap();
                assert_eq!(q.bounds.size.width, px(3.).scale(w.scale_factor()));
            }
        });
        let focus = owner.read_with(cx, |h, _| h.state.borrow().focus("p0").unwrap().clone());
        cx.update(|w, cx| w.focus(&focus, cx));
        draw(cx);
        cx.update(|w, _| {
            assert!(
                w.painted_quads()
                    .iter()
                    .any(|q| q.background.as_solid() == Some(gpui::rgb(0x00ff00).into()))
            )
        });
        update(owner, cx, |h| h.decorated = true);
        cx.update(|w, _| {
            assert!(w.painted_quads().iter().any(|q| q.background.as_solid()
                == Some(gpui::rgb(0xff00ff).into())
                && q.bounds.size.width == px(5.).scale(w.scale_factor())))
        });
        let position = bounds(owner, cx, "handle:p0").center();
        cx.simulate_mouse_move(position, None, Default::default());
        cx.simulate_mouse_down(position, gpui::MouseButton::Left, Default::default());
        draw(cx);
        assert!(cx.update(|w, _| w.captured_hitbox().is_some()));
        up(cx, position);
        update(owner, cx, |h| h.policy.enabled = false);
        assert!(!cx.update(|w, _| focus.is_focused(w)));
        let nodes = cx.a11y_tree().unwrap().nodes;
        assert!(
            nodes
                .iter()
                .filter(|(_, n)| n.role() == gpui::accesskit::Role::Splitter)
                .all(|(_, n)| n.is_disabled())
        );
        cx.simulate_click(position, Default::default());
        draw(cx);
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        assert!(events(owner, cx).is_empty());
    });
}
#[test]
fn lost_capture_and_owner_replacement_release_weak_frame_callbacks() {
    setup(Axis::Horizontal, |owner, cx| {
        let start = down(owner, cx, "handle:p0");
        moved(cx, start + point(px(40.), px(0.)));
        cx.update(|w, _| w.release_pointer());
        moved(cx, start + point(px(50.), px(0.)));
        assert_eq!(extents(owner, cx), vec![100., 200., 300.]);
        assert!(events(owner, cx).is_empty());
        let start = down(owner, cx, "handle:p0");
        moved(cx, start + point(px(20.), px(0.)));
        let retired = owner.read_with(cx, |h, _| Rc::downgrade(&h.state));
        cx.update(|w, cx| {
            owner.update(cx, |h, cx| {
                h.state.borrow_mut().close(w, cx);
                h.state = widget::State::new(h.config.clone(), w, cx).unwrap();
                cx.notify();
            })
        });
        assert!(retired.upgrade().is_none());
        up(cx, start);
        assert!(events(owner, cx).is_empty());
        assert_eq!(extents(owner, cx), vec![100., 200., 300.]);
    });
}
