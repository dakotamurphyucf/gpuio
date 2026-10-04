//! Measured element on GPUI TestPlatform, not OS keyboard/IME/GPU acceptance.
use crate::toast_geometry::Layering;
use crate::toast_stack_widget::{self as widget, Content, Frame, Shared, State};
use gpui::{
    Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, div, point,
    prelude::*, px,
};
use gpuio_protocol::{NodeId, toast_placement::Anchor};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    time::Duration,
};

fn n(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
#[derive(Default)]
struct Presentation {
    states: BTreeMap<NodeId, crate::toast_lifecycle::State>,
    painted: Vec<(NodeId, f32)>,
    commits: usize,
    reject: bool,
}
struct Fixture {
    state: Shared,
    presentation: Option<Rc<RefCell<Presentation>>>,
    motion: Option<crate::toast_lifecycle::Motion>,
    frames: Rc<RefCell<Vec<Frame>>>,
    clicks: Rc<Cell<usize>>,
    hover: Rc<RefCell<Vec<bool>>>,
    anchor: Anchor,
    expanded: bool,
    spring: Option<gpuio_protocol::animation::Spring>,
    now: u64,
    enabled: bool,
    width: f32,
    height: f32,
    cards: Vec<(NodeId, String, Option<f32>, bool)>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            state: State::new(),
            presentation: None,
            motion: Some(crate::toast_lifecycle::Motion::default()),
            frames: Default::default(),
            clicks: Default::default(),
            hover: Default::default(),
            anchor: Anchor::BottomRight,
            expanded: false,
            spring: None,
            now: 0,
            enabled: true,
            width: 240.,
            height: 260.,
            cards: vec![
                (n(0), "Older message".into(), Some(70.), false),
                (n(1), "Middle message".into(), Some(50.), false),
                (n(2), "Newest message".into(), Some(40.), false),
            ],
        }
    }
    fn frame(&self) -> Frame {
        self.frames.borrow().last().unwrap().clone()
    }
}
impl Render for Fixture {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let frames = self.frames.clone();
        let hover = self.hover.clone();
        let mut tokens = Vec::new();
        let motion = self
            .motion
            .filter(|_| self.enabled && window.is_window_active() && !cx.reduce_motion());
        let bottom = matches!(
            self.anchor,
            Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter
        );
        let contents = self
            .cards
            .iter()
            .filter_map(|(id, text, height, ending)| {
                let clicks = self.clicks.clone();
                let mut ending = *ending;
                let presentation = self.presentation.as_ref().map(|p| {
                    let mut p = p.borrow_mut();
                    let state = p.states.entry(*id).or_default();
                    ending |= !state.accepts_input();
                    state
                        .sample(Duration::from_millis(self.now), motion, bottom)
                        .unwrap()
                });
                if self
                    .presentation
                    .as_ref()
                    .is_some_and(|p| p.borrow().states[id].is_closed())
                {
                    return None;
                }
                if let Some(ref token) = presentation {
                    tokens.push((*id, token.clone()));
                }
                let mut body = div()
                    .id(("body", id.slot()))
                    .w_full()
                    .text_size(px(16.))
                    .line_height(px(20.))
                    .p(px(4.))
                    .occlude()
                    .role(gpui::Role::Button)
                    .aria_label(text.clone())
                    .child(text.clone())
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1));
                if let Some(h) = height {
                    body = body.h(px(*h));
                }
                if let Some(p) = self.presentation.clone() {
                    let id = *id;
                    body = body.child(
                        gpui::canvas(
                            |_, _, _| (),
                            move |_, _, w, _| {
                                p.borrow_mut().painted.push((id, w.element_opacity()));
                            },
                        )
                        .absolute()
                        .size_full(),
                    );
                }
                Some(Content {
                    id: *id,
                    ending,
                    presentation,
                    element: body.into_any_element(),
                })
            })
            .collect();
        let presentation = self.presentation.clone();
        div().pl(px(20.)).pt(px(30.)).child(
            div().w(px(self.width)).h(px(self.height)).child(
                widget::element(widget::Render {
                    state: self.state.clone(),
                    width: px(180.),
                    anchor: self.anchor,
                    layering: Layering::default(),
                    expanded: self.expanded,
                    oldest_first: false,
                    enabled: self.enabled,
                    pointer: true,
                    spring: self.spring,
                    now: std::time::Duration::from_millis(self.now),
                    contents,
                    observe: Rc::new(move |f, _, _| frames.borrow_mut().push(f.clone())),
                    after_paint: Rc::new(move |frame, _, _| {
                        let Some(ref p) = presentation else {
                            return false;
                        };
                        let mut p = p.borrow_mut();
                        if p.reject {
                            return false;
                        }
                        let mut needs_frame = false;
                        for (id, token) in &tokens {
                            if frame.items.iter().any(|i| i.id == *id && i.painted) {
                                assert!(
                                    p.painted.iter().any(|(painted, _)| painted == id),
                                    "child paints before lifecycle commit"
                                );
                                let state = p.states.get_mut(id).unwrap();
                                if state.painted(token) {
                                    needs_frame |= token.needs_frame();
                                    p.commits += 1;
                                }
                            } else {
                                p.states.get_mut(id).unwrap().suspend();
                            }
                        }
                        needs_frame
                    }),
                    hover: Rc::new(move |h, _, _| hover.borrow_mut().push(h)),
                })
                .unwrap(),
            ),
        )
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|w, cx| w.draw(cx).clear(cx));
}
fn click(cx: &mut VisualTestContext, position: gpui::Point<gpui::Pixels>) {
    cx.simulate_mouse_move(position, None, Default::default());
    cx.simulate_event(gpui::MouseDownEvent {
        button: gpui::MouseButton::Left,
        position,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(gpui::MouseUpEvent {
        button: gpui::MouseButton::Left,
        position,
        modifiers: Default::default(),
        click_count: 1,
    });
}
fn wheel(cx: &mut VisualTestContext, position: gpui::Point<gpui::Pixels>, delta: f32) {
    cx.simulate_mouse_move(position, None, Default::default());
    cx.simulate_event(gpui::ScrollWheelEvent {
        position,
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(delta))),
        touch_phase: gpui::TouchPhase::Moved,
        modifiers: Default::default(),
    });
    draw(cx);
}

#[test]
fn measured_cards_mirror_at_all_anchors_and_ignore_deeper_hidden_height() {
    let mut app = TestAppContext::single();
    let (owner, cx) = app.add_window_view(|_, _| Fixture::new());
    for anchor in [
        Anchor::TopLeft,
        Anchor::TopRight,
        Anchor::BottomLeft,
        Anchor::BottomRight,
        Anchor::TopCenter,
        Anchor::BottomCenter,
        Anchor::LeftCenter,
        Anchor::RightCenter,
    ] {
        owner.update(cx, |f, cx| {
            f.anchor = anchor;
            cx.notify();
        });
        draw(cx);
        let f = owner.read_with(cx, |f, _| f.frame());
        let x = match anchor {
            Anchor::TopLeft | Anchor::BottomLeft | Anchor::LeftCenter => 20.,
            Anchor::TopRight | Anchor::BottomRight | Anchor::RightCenter => 80.,
            _ => 50.,
        };
        let y = match anchor {
            Anchor::TopLeft | Anchor::TopRight | Anchor::TopCenter => 30.,
            Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter => 192.,
            _ => 111.,
        };
        assert_eq!(
            f.viewport,
            gpui::Bounds::new(point(px(x), px(y)), gpui::size(px(180.), px(98.)))
        );
        assert_eq!(
            f.items.iter().map(|i| i.interactive).collect::<Vec<_>>(),
            vec![false, false, true]
        );
        assert_eq!(
            f.items
                .iter()
                .map(|i| i.bounds.size.width)
                .collect::<Vec<_>>(),
            vec![px(162.), px(171.), px(180.)]
        );
        assert_eq!(
            f.items[2].bounds.origin.y,
            px(
                if matches!(
                    anchor,
                    Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter
                ) {
                    y + 58.
                } else {
                    y
                }
            )
        );
    }
    let before = owner.read_with(cx, |f, _| f.frame());
    owner.update(cx, |f, cx| {
        f.cards
            .insert(0, (n(3), "Hidden tall card".into(), Some(1200.), false));
        cx.notify();
    });
    draw(cx);
    let after = owner.read_with(cx, |f, _| f.frame());
    assert_eq!(before.viewport, after.viewport);
    assert!(!after.items[0].painted);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}

#[test]
fn streamed_wrapping_height_is_anchored_on_first_frame_without_a_correction_jump() {
    let mut app = TestAppContext::single();
    let (owner, cx) = app.add_window_view(|_, _| {
        let mut f = Fixture::new();
        f.expanded = true;
        f.height = 900.;
        f.cards = vec![
            (n(0), "Earlier".into(), Some(50.), false),
            (n(1), "Short".into(), None, false),
        ];
        f
    });
    draw(cx);
    let first = owner.read_with(cx, |f, _| f.frame());
    owner.update(cx, |f, cx| {
        f.cards[1].1 =
            "Streamed content wraps into several measured lines in the current frame. ".repeat(4);
        cx.notify();
    });
    draw(cx);
    let changed = owner.read_with(cx, |f, _| f.frame());
    assert!(changed.items[1].bounds.size.height > first.items[1].bounds.size.height);
    assert_eq!(changed.items[1].bounds.bottom(), px(930.));
    assert_eq!(
        changed.items[0].bounds.bottom() + px(14.),
        changed.items[1].bounds.top()
    );
    draw(cx);
    let settled = owner.read_with(cx, |f, _| f.frame());
    assert_eq!(settled.viewport, changed.viewport);
    assert_eq!(
        settled.items.iter().map(|i| i.bounds).collect::<Vec<_>>(),
        changed.items.iter().map(|i| i.bounds).collect::<Vec<_>>()
    );
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}

#[test]
fn collapsed_back_cards_are_inert_hover_covers_front_and_expansion_scrolls_with_bounds() {
    let mut app = TestAppContext::single();
    let (owner, cx) = app.add_window_view(|_, _| Fixture::new());
    cx.simulate_a11y_active(true);
    draw(cx);
    let f = owner.read_with(cx, |f, _| f.frame());
    click(cx, f.items[0].bounds.origin + point(px(8.), px(8.)));
    assert_eq!(owner.read_with(cx, |f, _| f.clicks.get()), 0);
    click(cx, f.items[2].bounds.origin + point(px(8.), px(8.)));
    assert_eq!(owner.read_with(cx, |f, _| f.clicks.get()), 1);
    assert_eq!(
        owner.read_with(cx, |f, _| f.hover.borrow().last().copied()),
        Some(true)
    );
    cx.simulate_event(gpui::MouseExitEvent {
        position: point(px(-1.), px(-1.)),
        ..Default::default()
    });
    assert_eq!(
        owner.read_with(cx, |f, _| f.hover.borrow().last().copied()),
        Some(false),
        "leaving the window does not require a final mouse move"
    );
    let ax = cx.a11y_tree().unwrap();
    let older = ax
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Older message"))
        .unwrap()
        .0;
    assert!(
        ax.nodes
            .iter()
            .any(|(_, n)| n.is_hidden() && n.children().contains(&older)),
        "decorative card requires a real hidden semantic ancestor"
    );
    owner.update(cx, |f, cx| {
        f.expanded = true;
        f.height = 80.;
        cx.notify();
    });
    draw(cx);
    let f = owner.read_with(cx, |f, _| f.frame());
    assert_eq!(f.max_scroll, px(108.));
    assert_eq!(f.scroll, f.max_scroll);
    wheel(cx, f.viewport.origin + point(px(10.), px(10.)), 1000.);
    let top = owner.read_with(cx, |f, _| f.frame());
    assert_eq!(top.scroll, px(0.));
    assert!(top.items[0].painted);
    assert!(!top.items[2].painted);
    wheel(cx, top.viewport.origin + point(px(10.), px(10.)), -1000.);
    assert_eq!(owner.read_with(cx, |f, _| f.frame().scroll), px(108.));
    owner.update(cx, |f, cx| {
        f.expanded = false;
        cx.notify();
    });
    draw(cx);
    owner.update(cx, |f, cx| {
        f.cards.remove(0);
        f.height = 260.;
        cx.notify();
    });
    draw(cx);
    assert_eq!(owner.read_with(cx, |f, _| f.frame().max_scroll), px(0.));
}

#[test]
fn ending_disabled_and_retired_frames_cannot_accept_pointer_actions() {
    let mut app = TestAppContext::single();
    let (owner, cx) = app.add_window_view(|_, _| Fixture::new());
    cx.simulate_a11y_active(true);
    owner.update(cx, |f, cx| {
        f.cards[2].3 = true;
        cx.notify();
    });
    draw(cx);
    let frame = owner.read_with(cx, |f, _| f.frame());
    assert!(frame.items[2].painted && !frame.items[2].interactive);
    click(cx, frame.items[2].bounds.origin + point(px(8.), px(8.)));
    assert_eq!(owner.read_with(cx, |f, _| f.clicks.get()), 0);
    owner.update(cx, |f, cx| {
        f.cards[2].3 = false;
        f.enabled = false;
        cx.notify();
    });
    draw(cx);
    click(cx, frame.items[2].bounds.origin + point(px(8.), px(8.)));
    assert_eq!(owner.read_with(cx, |f, _| f.clicks.get()), 0);
    let nodes = cx.a11y_tree().unwrap().nodes;
    assert!(
        nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Newest message") && n.is_disabled())
    );
    let state = owner.read_with(cx, |f, _| f.state.clone());
    state.borrow_mut().suspend();
    let previous = owner.read_with(cx, |f, _| f.hover.borrow().len());
    cx.simulate_mouse_move(point(px(1.), px(1.)), None, Default::default());
    assert_eq!(owner.read_with(cx, |f, _| f.hover.borrow().len()), previous);
    owner.update(cx, |f, cx| {
        f.width = 0.;
        cx.notify();
    });
    draw(cx);
    assert_eq!(state.borrow().max_scroll(), px(0.));
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}

#[test]
fn native_reflow_measures_animated_width_and_keeps_streaming_bottom_attached() {
    let mut app = TestAppContext::single();
    let (owner, cx) = app.add_window_view(|_, _| {
        let mut f = Fixture::new();
        f.spring = Some(gpuio_protocol::tab_motion::Config::default().spring);
        f.height = 900.;
        f.cards[0].2 = None;
        f.cards[0].1 = "A longer message wraps as its actual native width changes. ".repeat(4);
        f
    });
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    draw(cx);
    let collapsed = owner.read_with(cx, |f, _| f.frame());
    owner.update(cx, |f, cx| {
        f.expanded = true;
        f.now = 16;
        cx.notify();
    });
    draw(cx);
    let expanding = owner.read_with(cx, |f, _| f.frame());
    assert!(expanding.needs_frame);
    assert!(expanding.items[0].bounds.size.width > collapsed.items[0].bounds.size.width);
    assert!(expanding.items[0].bounds.size.width < px(180.));
    assert_eq!(expanding.items[2].bounds.bottom(), px(930.));
    // Every frame changes front height. The older card must still move, while
    // the front stays attached to the bottom and width reaches its own endpoint.
    let mut previous_edge = expanding.items[0].bounds.bottom();
    for tick in 2..=120 {
        owner.update(cx, |f, cx| {
            f.now = tick * 16;
            f.cards[2].2 = Some(40. + tick as f32);
            cx.notify();
        });
        draw(cx);
        let frame = owner.read_with(cx, |f, _| f.frame());
        assert_eq!(frame.items[2].bounds.bottom(), px(930.));
        assert!(
            frame.items[0].bounds.bottom() < previous_edge,
            "reflow stalled at {tick}"
        );
        previous_edge = frame.items[0].bounds.bottom();
    }
    let frame = owner.read_with(cx, |f, _| f.frame());
    assert!((f32::from(frame.items[0].bounds.size.width) - 180.).abs() < 0.01);
    owner.update(cx, |f, cx| {
        f.now = 5000;
        cx.notify();
    });
    draw(cx);
    assert!(!owner.read_with(cx, |f, _| f.frame().needs_frame));
    assert_eq!(
        owner.read_with(cx, |f, _| f.frame().items[0].bounds.size.width),
        px(180.)
    );
}

#[test]
fn interrupted_native_reflow_settles_on_scroll_inactive_and_motion_reset() {
    let mut app = TestAppContext::single();
    let (owner, cx) = app.add_window_view(|_, _| {
        let mut f = Fixture::new();
        f.spring = Some(gpuio_protocol::tab_motion::Config::default().spring);
        f
    });
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    draw(cx);
    owner.update(cx, |f, cx| {
        f.expanded = true;
        f.now = 16;
        cx.notify();
    });
    draw(cx);
    let before = owner.read_with(cx, |f, _| f.frame());
    owner.update(cx, |f, cx| {
        f.expanded = false;
        f.now = 32;
        cx.notify();
    });
    draw(cx);
    let interrupted = owner.read_with(cx, |f, _| f.frame());
    assert!(interrupted.needs_frame);
    assert!(
        (f32::from(interrupted.items[0].bounds.top() - before.items[0].bounds.top())).abs() < 20.
    );
    owner.update(cx, |f, cx| {
        f.spring = None;
        f.now = 48;
        cx.notify();
    });
    draw(cx);
    assert!(!owner.read_with(cx, |f, _| f.frame().needs_frame));
    owner.update(cx, |f, cx| {
        f.spring = Some(gpuio_protocol::tab_motion::Config::default().spring);
        f.expanded = true;
        f.height = 80.;
        f.now = 64;
        cx.notify();
    });
    draw(cx);
    let frame = owner.read_with(cx, |f, _| f.frame());
    wheel(cx, frame.viewport.origin + point(px(10.), px(10.)), 1000.);
    assert!(
        !owner.read_with(cx, |f, _| f.frame().needs_frame),
        "scrolling immediately settles layout motion"
    );
    assert_eq!(owner.read_with(cx, |f, _| f.frame().scroll), px(0.));
    // Disabled policy uses the same immediate renderer path as inactive/reduced.
    owner.update(cx, |f, cx| {
        f.enabled = false;
        f.expanded = false;
        f.now = 80;
        cx.notify();
    });
    draw(cx);
    assert!(!owner.read_with(cx, |f, _| f.frame().needs_frame));
}

#[test]
fn resize_outside_old_surface_and_reduced_motion_do_not_leave_blank_or_replaying_cards() {
    let mut app = TestAppContext::single();
    let (owner, cx) = app.add_window_view(|_, _| {
        let mut f = Fixture::new();
        f.spring = Some(gpuio_protocol::tab_motion::Config::default().spring);
        f.width = 1000.;
        f
    });
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    draw(cx);
    owner.update(cx, |f, cx| {
        f.width = 120.;
        f.now = 16;
        cx.notify();
    });
    draw(cx);
    let frame = owner.read_with(cx, |f, _| f.frame());
    assert!(
        frame.items[2].painted,
        "new usable rectangle must not stay blank"
    );
    assert!(frame.items[2].bounds.left() < px(140.));
    cx.update(|_, cx| cx.set_reduce_motion(true));
    owner.update(cx, |f, cx| {
        f.now = 32;
        f.expanded = true;
        cx.notify();
    });
    draw(cx);
    let settled = owner.read_with(cx, |f, _| f.frame());
    assert!(!settled.needs_frame);
    assert_eq!(settled.items[2].bounds.size.width, px(120.));
    cx.update(|_, cx| cx.set_reduce_motion(false));
    owner.update(cx, |f, cx| {
        f.now = 48;
        cx.notify();
    });
    draw(cx);
    assert!(
        !owner.read_with(cx, |f, _| f.frame().needs_frame),
        "reenabling does not replay old travel"
    );
}

#[test]
fn entry_commits_after_transparent_child_paint_and_interrupted_exit_retires_input() {
    use crate::toast_lifecycle::{Motion, State as Phase};
    use gpuio_protocol::v1::ToastDismissal;
    let mut app = TestAppContext::single();
    let presentation = Rc::new(RefCell::new(Presentation::default()));
    let tracker = presentation.clone();
    let (owner, cx) = app.add_window_view(|_, _| {
        let mut fixture = Fixture::new();
        fixture.cards.truncate(1);
        fixture.presentation = Some(tracker);
        fixture
    });
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    // Activation itself can draw; establish a fresh mounted phase afterwards.
    presentation
        .borrow_mut()
        .states
        .insert(n(0), Phase::default());
    presentation.borrow_mut().painted.clear();
    draw(cx);
    let first = owner.read_with(cx, |f, _| f.frame());
    assert!(
        first.items[0].painted,
        "transparent first entry must commit"
    );
    assert!(!first.items[0].interactive);
    assert!(
        first.items[0].bounds.top() >= first.viewport.bottom(),
        "initial slide is beyond the viewport"
    );
    assert!(
        presentation
            .borrow()
            .painted
            .iter()
            .any(|(_, alpha)| *alpha == 0.)
    );
    assert!(presentation.borrow().states[&n(0)].deadline().is_some());
    assert!(!presentation.borrow().states[&n(0)].allows_timeout());
    assert!(cx.update(|w, cx| w.simulate_next_frame(cx)) > 0);
    click(cx, first.viewport.center());
    assert_eq!(owner.read_with(cx, |f, _| f.clicks.get()), 0);

    owner.update(cx, |f, cx| {
        f.now = 200;
        cx.notify();
    });
    draw(cx);
    let middle = owner.read_with(cx, |f, _| f.frame());
    let alpha = presentation.borrow().painted.last().unwrap().1;
    assert!(alpha > 0. && alpha < 1.);
    assert!(middle.items[0].interactive);
    assert!(
        presentation
            .borrow_mut()
            .states
            .get_mut(&n(0))
            .unwrap()
            .dismiss(
                ToastDismissal::CloseButton,
                Duration::from_millis(200),
                Some(Motion::default()),
                true,
            )
            .unwrap()
    );
    draw(cx);
    let ending = owner.read_with(cx, |f, _| f.frame());
    assert_eq!(
        ending.items[0].bounds, middle.items[0].bounds,
        "exit starts at last painted slide"
    );
    assert_eq!(presentation.borrow().painted.last().unwrap().1, alpha);
    assert!(!ending.items[0].interactive);
    click(
        cx,
        ending.items[0].bounds.intersect(&ending.viewport).center(),
    );
    assert_eq!(
        owner.read_with(cx, |f, _| f.clicks.get()),
        0,
        "Ending content is pointer-inert"
    );
    assert!(
        presentation
            .borrow_mut()
            .states
            .get_mut(&n(0))
            .unwrap()
            .take_dismissal()
            .is_none()
    );
    owner.update(cx, |f, cx| {
        f.now = 400;
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        presentation
            .borrow_mut()
            .states
            .get_mut(&n(0))
            .unwrap()
            .take_dismissal(),
        Some(ToastDismissal::CloseButton)
    );
    assert!(
        presentation
            .borrow_mut()
            .states
            .get_mut(&n(0))
            .unwrap()
            .take_dismissal()
            .is_none()
    );
    assert!(presentation.borrow().states[&n(0)].is_closed());
    draw(cx);
    assert!(owner.read_with(cx, |f, _| f.frame().items.is_empty()));
    cx.update(|w, cx| w.simulate_next_frame(cx));
    draw(cx);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}

#[test]
fn hidden_layers_rejected_paints_and_zero_area_never_start_an_entry_clock() {
    let mut app = TestAppContext::single();
    let presentation = Rc::new(RefCell::new(Presentation::default()));
    let tracker = presentation.clone();
    let (owner, cx) = app.add_window_view(|_, _| {
        let mut fixture = Fixture::new();
        fixture.presentation = Some(tracker);
        fixture
            .cards
            .insert(0, (n(3), "Hidden".into(), Some(80.), false));
        fixture
    });
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    draw(cx);
    assert!(!owner.read_with(cx, |f, _| f.frame().items[0].painted));
    assert!(presentation.borrow().states[&n(3)].deadline().is_none());
    assert!(!presentation.borrow().states[&n(3)].allows_timeout());
    assert!(
        !presentation
            .borrow()
            .painted
            .iter()
            .any(|(id, _)| *id == n(3))
    );
    owner.update(cx, |f, cx| {
        f.now = 400;
        cx.notify();
    });
    draw(cx);
    // Completed visible phases must not keep a hidden Pending phase ticking.
    cx.update(|w, cx| w.simulate_next_frame(cx));
    draw(cx);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);

    presentation.borrow_mut().reject = true;
    presentation.borrow_mut().states.clear();
    let commits = presentation.borrow().commits;
    draw(cx);
    assert_eq!(presentation.borrow().commits, commits);
    assert!(
        presentation
            .borrow()
            .states
            .values()
            .all(|s| s.deadline().is_none())
    );
    assert_eq!(
        cx.update(|w, cx| w.simulate_next_frame(cx)),
        0,
        "a rejected preview cannot schedule motion"
    );
    owner.update(cx, |f, cx| {
        f.height = 0.;
        cx.notify();
    });
    presentation.borrow_mut().reject = false;
    draw(cx);
    assert_eq!(presentation.borrow().commits, commits);
    assert_eq!(
        owner.read_with(cx, |f, _| f.state.borrow().retained_items()),
        0
    );
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}
