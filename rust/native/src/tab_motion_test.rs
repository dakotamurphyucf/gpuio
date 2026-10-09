//! Production host layout and scene paint on TestPlatform, no OS window.
use super::super::*;
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, tab_appearance};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream, time::Duration};
fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn wid() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn choices(selected: usize) -> ChoiceConfig {
    ChoiceConfig {
        label: "Motion tabs".into(),
        items: (0..3)
            .map(|i| ChoiceItem {
                id: format!("t{i}"),
                label: format!("Tab {i}"),
                disabled: false,
            })
            .collect(),
        selected: Some(format!("t{selected}")),
        disabled: false,
    }
}
fn presentation(variant: Variant) -> tab_appearance::Config {
    tab_appearance::Config {
        variant,
        gap: 10.,
        tab_style: vec![
            Style::Fields(vec![Field::Width(Length::Px(100.))]),
            Style::State(7, vec![Field::Foreground(Color::Rgba(0xff0000ff))]),
        ],
        ..Default::default()
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(wid()).unwrap().revision();
            let result = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: wid(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&result.dirty, w, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn tick(cx: &mut VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    draw(cx);
}
fn left(owner: &Entity<View>, cx: &mut VisualTestContext) -> f64 {
    owner.read_with(cx, |v, _| {
        v.tab_motions[&n(0)]
            .borrow()
            .history
            .as_ref()
            .unwrap()
            .left
            .position
    })
}
fn indicator(cx: &mut VisualTestContext) -> gpui::Quad {
    cx.update(|w, _| {
        let quads: Vec<_> = w
            .painted_quads()
            .into_iter()
            .filter(|q| {
                q.background
                    .as_solid()
                    .is_some_and(|color| (color.a - 0.15).abs() < 0.001)
            })
            .collect();
        assert_eq!(
            quads.len(),
            1,
            "one selected fill; no duplicate static fill"
        );
        quads[0]
    })
}
fn setup(f: impl FnOnce(&Entity<View>, &mut VisualTestContext, &Transport)) {
    let mut app = TestAppContext::single();
    app.update(crate::image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid(), "Motion", 500., 200.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid(), session, transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|w, cx| {
        w.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                n(0),
                Kind::TabBar,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetChoice(n(0), choices(0)),
            Op::SetTabAppearance(n(0), Some(presentation(Variant::Pill))),
            Op::SetTabMotion(n(0), Some(Config::default())),
            Op::SetStyle(
                n(0),
                vec![
                    Style::Width(Length::Px(330.)),
                    Style::Height(Length::Px(40.)),
                    Style::Foreground(Color::Rgba(0x0000ffff)),
                ],
            ),
            Op::SetRoot(Some(n(0))),
        ],
    );
    f(&owner, cx, &transport);
    drop(owner);
    cx.cx.update(|_| ());
    cx.run_until_parked();
}
#[test]
fn actual_indicator_moves_interrupts_and_resizes() {
    setup(|owner, cx, transport| {
        assert_eq!(left(owner, cx), 0.);
        let first = indicator(cx);
        let state = owner.read_with(cx, |v, _| Rc::downgrade(&v.tab_motions[&n(0)]));
        apply(owner, cx, vec![Op::SetChoice(n(0), choices(1))]);
        assert_eq!(left(owner, cx), 0.);
        tick(cx, 100);
        let mid = left(owner, cx);
        assert!(mid > 0. && mid < 110., "{mid}");
        assert!(indicator(cx).bounds.origin.x > first.bounds.origin.x);
        let velocity = state
            .upgrade()
            .unwrap()
            .borrow()
            .history
            .as_ref()
            .unwrap()
            .left
            .velocity;
        apply(owner, cx, vec![Op::SetChoice(n(0), choices(2))]);
        assert_eq!(left(owner, cx), mid);
        assert_eq!(
            state
                .upgrade()
                .unwrap()
                .borrow()
                .history
                .as_ref()
                .unwrap()
                .left
                .velocity,
            velocity
        );
        tick(cx, 2100);
        assert_eq!(left(owner, cx), 220.);
        indicator(cx);
        assert_eq!(
            cx.update(|w, cx| w.simulate_next_frame(cx)),
            0,
            "settled motion is idle"
        );
        let mut appearance = presentation(Variant::Pill);
        appearance.tab_style[0] = Style::Fields(vec![Field::Width(Length::Px(80.))]);
        apply(
            owner,
            cx,
            vec![Op::SetTabAppearance(n(0), Some(appearance))],
        );
        assert_eq!(left(owner, cx), 220.);
        tick(cx, 2100);
        assert_eq!(left(owner, cx), 180.);
        assert!(
            !transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .iter()
                .any(|e| matches!(e, Event::Choice(..)))
        );
        apply(owner, cx, vec![Op::SetTabMotion(n(0), None)]);
        assert!(state.upgrade().is_none());
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    });
}
#[test]
fn reduced_hidden_incompatible_and_removed_owners_retire_motion() {
    setup(|owner, cx, _| {
        apply(owner, cx, vec![Op::SetChoice(n(0), choices(2))]);
        tick(cx, 50);
        cx.update(|_, cx| cx.set_reduce_motion(true));
        draw(cx);
        assert_eq!(left(owner, cx), 220.);
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
        cx.update(|_, cx| cx.set_reduce_motion(false));
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                n(0),
                vec![Style::Fields(vec![Field::Display(3)])],
            )],
        );
        assert!(owner.read_with(cx, |v, _| v.tab_motions[&n(0)].borrow().history.is_none()));
        apply(
            owner,
            cx,
            vec![
                Op::SetChoice(n(0), choices(0)),
                Op::SetStyle(n(0), vec![Style::Width(Length::Px(330.))]),
            ],
        );
        assert_eq!(left(owner, cx), 0.);
        apply(
            owner,
            cx,
            vec![Op::SetTabAppearance(
                n(0),
                Some(presentation(Variant::Outline)),
            )],
        );
        assert!(owner.read_with(cx, |v, _| v.tab_motions[&n(0)].borrow().history.is_none()));
        let state = owner.read_with(cx, |v, _| Rc::downgrade(&v.tab_motions[&n(0)]));
        apply(owner, cx, vec![Op::SetRoot(None), Op::Remove(n(0))]);
        assert!(state.upgrade().is_none());
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    });
}

#[test]
fn scroll_translation_is_not_a_new_target_and_reorder_follows_identity() {
    setup(|owner, cx, _| {
        apply(
            owner,
            cx,
            vec![
                Op::SetTabViewport(n(0), Some(Default::default())),
                Op::SetStyle(
                    n(0),
                    vec![
                        Style::Width(Length::Px(200.)),
                        Style::Height(Length::Px(40.)),
                    ],
                ),
                Op::SetChoice(n(0), choices(2)),
            ],
        );
        tick(cx, 2100);
        assert_eq!(left(owner, cx), 220.);
        let state = owner.read_with(cx, |v, _| v.tab_motions[&n(0)].clone());
        let scroll = owner.read_with(cx, |v, _| v.scrolls[&n(0)].clone());
        scroll.handle.set_offset(gpui::point(px(-80.), px(0.)));
        draw(cx);
        assert_eq!(left(owner, cx), 220.);
        assert!(state.borrow().history.as_ref().unwrap().travel.is_none());
        let mut reversed = choices(2);
        reversed.items.reverse();
        apply(owner, cx, vec![Op::SetChoice(n(0), reversed)]);
        assert_eq!(left(owner, cx), 220.);
        tick(cx, 2100);
        assert_eq!(left(owner, cx), 0.);
        // A real wheel event is independent of animation and controlled selection.
        let position = gpui::point(px(80.), px(16.));
        cx.simulate_mouse_move(position, None, Default::default());
        cx.update(|w, cx| {
            w.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(px(30.), px(0.))),
                    touch_phase: gpui::TouchPhase::Started,
                    modifiers: Default::default(),
                }),
                cx,
            );
        });
        draw(cx);
        assert_eq!(left(owner, cx), 0.);
        assert!(state.borrow().history.as_ref().unwrap().travel.is_none());
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    });
}
#[test]
fn segmented_underline_inactive_disabled_missing_and_zero_layout_stop_scheduling() {
    setup(|owner, cx, _| {
        for variant in [Variant::Segmented, Variant::Underline] {
            apply(
                owner,
                cx,
                vec![
                    Op::SetTabAppearance(n(0), Some(presentation(variant))),
                    Op::SetChoice(n(0), choices(0)),
                ],
            );
            apply(owner, cx, vec![Op::SetChoice(n(0), choices(2))]);
            tick(cx, 50);
            assert!(left(owner, cx) > 0. && left(owner, cx) < 220.);
            if variant == Variant::Underline {
                cx.update(|w, _| {
                    assert!(
                        w.painted_quads().iter().any(|q| q
                            .background
                            .as_solid()
                            .is_some_and(|c| c.a > 0.99)
                            && q.bounds.size.height < px(5.).scale(w.scale_factor()))
                    )
                });
            }
            cx.deactivate_window();
            draw(cx);
            assert_eq!(left(owner, cx), 220.);
            assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
            cx.update(|w, _| w.activate_window());
            draw(cx);
        }
        let mut disabled = choices(0);
        disabled.items[0].disabled = true;
        apply(owner, cx, vec![Op::SetChoice(n(0), disabled)]);
        assert_eq!(left(owner, cx), 0.);
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
        let mut missing = choices(0);
        missing.selected = None;
        apply(owner, cx, vec![Op::SetChoice(n(0), missing)]);
        assert!(owner.read_with(cx, |v, _| v.tab_motions[&n(0)].borrow().history.is_none()));
        apply(
            owner,
            cx,
            vec![
                Op::SetChoice(n(0), choices(2)),
                Op::SetStyle(
                    n(0),
                    vec![Style::Width(Length::Px(0.)), Style::Height(Length::Px(0.))],
                ),
            ],
        );
        assert!(owner.read_with(cx, |v, _| v.tab_motions[&n(0)].borrow().history.is_none()));
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                n(0),
                vec![
                    Style::Width(Length::Px(330.)),
                    Style::Height(Length::Px(40.)),
                ],
            )],
        );
        assert_eq!(left(owner, cx), 220.);
    });
}

#[test]
fn closing_a_window_during_travel_releases_the_native_owner() {
    let mut retired = None;
    setup(|owner, cx, transport| {
        apply(owner, cx, vec![Op::SetChoice(n(0), choices(2))]);
        tick(cx, 50);
        retired = Some(owner.read_with(cx, |v, _| Rc::downgrade(&v.tab_motions[&n(0)])));
        owner.update(cx, |v, _| {
            v.session.borrow_mut().close(wid()).unwrap();
        });
        cx.update(|w, _| w.remove_window());
        cx.run_until_parked();
        transport.mailbox.lock().unwrap().drain(100);
        cx.executor().advance_clock(Duration::from_secs(10));
        cx.run_until_parked();
        assert!(transport.mailbox.lock().unwrap().drain(100).is_empty());
        assert_eq!(
            owner.read_with(cx, |v, _| v.session.borrow().retained_bytes()),
            0
        );
    });
    assert!(retired.unwrap().upgrade().is_none());
}

struct ForegroundProbe {
    choices: Arc<ChoiceConfig>,
    motion: Rc<RefCell<State>>,
    choice_state: Rc<RefCell<super::super::choice::State>>,
    focus: gpui::FocusHandle,
    gate: super::super::focus::Shared,
    painted: Rc<RefCell<Vec<gpui::Rgba>>>,
}
impl gpui::Render for ForegroundProbe {
    fn render(&mut self, w: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let labels = (0..3)
            .map(|i| {
                let painted = self.painted.clone();
                let probe = gpui::canvas(
                    |_, _, _| (),
                    move |bounds, _, w, _| {
                        let color = w.text_style().color;
                        painted.borrow_mut().push(color.to_rgb());
                        w.paint_quad(gpui::fill(bounds, color));
                    },
                )
                .w(px(8.))
                .h(px(8.));
                let frame = gpui::div().child(probe);
                Some(if i == 2 {
                    frame.text_color(gpui::rgb(0x00ff00)).into_any_element()
                } else {
                    frame.into_any_element()
                })
            })
            .collect();
        super::super::radio::element(
            gpui::div()
                .id("probe")
                .flex()
                .flex_row()
                .w(px(330.))
                .h(px(40.))
                .text_color(gpui::rgb(0x0000ff)),
            super::super::radio::Render {
                tabs: true,
                tab_content: None,
                viewport: None,
                motion: Some(Owner {
                    state: self.motion.clone(),
                    config: Config::default(),
                }),
                parts: vec![],
                trailing: None,
                gate: self.gate.clone(),
                node: n(0),
                tab_appearance: Some(&presentation(Variant::Pill)),
                labels,
                disabled: false,
                appearance: crate::control_appearance::default(),
                config: &self.choices,
                state: self.choice_state.clone(),
                focus: self.focus.clone(),
                route: None,
                pointer: false,
                selected_style: None,
            },
            w,
            cx,
        )
    }
}
#[test]
fn selected_foreground_is_inherited_at_native_paint_and_explicit_descendants_win() {
    let mut app = TestAppContext::single();
    let painted = Rc::new(RefCell::new(vec![]));
    let (probe, cx) = app.add_window_view(|_, cx| ForegroundProbe {
        choices: Arc::new(choices(0)),
        motion: Rc::default(),
        choice_state: Rc::default(),
        focus: cx.focus_handle(),
        gate: super::super::focus::Manager::new(wid(), Rc::new(RefCell::new(Session::default()))),
        painted: painted.clone(),
    });
    cx.update(|w, cx| {
        w.activate_window();
        cx.set_reduce_motion(false);
    });
    draw(cx);
    probe.update(cx, |p, cx| {
        p.choices = Arc::new(choices(1));
        cx.notify();
    });
    painted.borrow_mut().clear();
    draw(cx);
    assert!(painted.borrow().iter().any(|c| c.b > 0.99 && c.r < 0.01));
    painted.borrow_mut().clear();
    tick(cx, 100);
    assert!(
        painted
            .borrow()
            .iter()
            .any(|c| (c.r - 0.5).abs() < 0.01 && (c.b - 0.5).abs() < 0.01),
        "{:?}",
        painted.borrow()
    );
    assert!(painted.borrow().iter().any(|c| c.g > 0.99 && c.r < 0.01));
    painted.borrow_mut().clear();
    tick(cx, 100);
    assert!(painted.borrow().iter().any(|c| c.r > 0.99 && c.b < 0.01));
    probe.update(cx, |p, cx| {
        p.choices = Arc::new(choices(2));
        cx.notify();
    });
    painted.borrow_mut().clear();
    draw(cx);
    assert!(painted.borrow().iter().any(|c| c.g > 0.99 && c.r < 0.01));
}

#[test]
fn explicit_selected_paint_theme_updates_and_disabled_opacity_are_preserved() {
    setup(|owner, cx, _| {
        let mut custom = presentation(Variant::Pill);
        custom.item_styles = vec![(
            "t0".into(),
            vec![Style::State(
                7,
                vec![
                    Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
                    Field::Foreground(Color::Rgba(0xffff00ff)),
                ],
            )],
        )];
        apply(owner, cx, vec![Op::SetTabAppearance(n(0), Some(custom))]);
        cx.update(|w, _| {
            let quads = w.painted_quads();
            let custom = quads
                .iter()
                .find(|q| q.background.as_solid() == Some(gpui::rgb(0x00ff00).into()))
                .unwrap();
            let moving = quads
                .iter()
                .find(|q| {
                    q.background
                        .as_solid()
                        .is_some_and(|c| (c.a - 0.15).abs() < 0.001)
                })
                .unwrap();
            assert!(
                custom.order >= moving.order,
                "explicit target fill paints over default indicator"
            );
        });
        apply(
            owner,
            cx,
            vec![
                Op::SetTabAppearance(n(0), Some(presentation(Variant::Pill))),
                Op::SetStyle(n(0), vec![Style::Foreground(Color::Rgba(0xaa22ffff))]),
            ],
        );
        let q = indicator(cx);
        assert_eq!(
            q.background.as_solid().unwrap().alpha(1.),
            gpui::rgba(0xaa22ffff).into()
        );
        let mut disabled = choices(0);
        disabled.items[0].disabled = true;
        apply(owner, cx, vec![Op::SetChoice(n(0), disabled)]);
        cx.update(|w, _| {
            assert!(w.painted_quads().iter().any(|q| {
                q.background
                    .as_solid()
                    .is_some_and(|c| (c.a - 0.075).abs() < 0.001)
            }))
        });
    });
}
