use super::*;
use crate::{session::Session, transport::Transport};
use gpuio_protocol::{
    HandlerId, WindowId,
    numeric::Domain,
    slider as s,
    v1::{
        CAPABILITIES, Field as WireField, Kind, Length as WireLength, Op, Style as WireStyle,
        Transaction, VERSION,
    },
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream, time::Duration};
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn config() -> s::Config {
    s::Config {
        domain: Domain::new(0., 100., 1.).unwrap(),
        label: "Level".into(),
        lower_label: "Low".into(),
        upper_label: "High".into(),
        axis: Axis::Horizontal,
        scale: s::Scale::Linear,
        disabled: false,
        read_only: false,
    }
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, ops: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let result = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations: ops,
                })
                .unwrap();
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn tick(cx: &mut VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn hover(cx: &mut VisualTestContext, position: Point<Pixels>, pressed: bool) {
    cx.simulate_event(MouseMoveEvent {
        position,
        pressed_button: pressed.then_some(MouseButton::Left),
        modifiers: Default::default(),
    });
    draw(cx);
}
#[::core::prelude::v1::test]
fn hover_press_and_reduced_motion_are_native_and_do_not_own_idle_frames() {
    let mut app = TestAppContext::single();
    let (_read, write) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Slider ring", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|w, cx| {
        w.activate_window();
        cx.set_reduce_motion(false);
    });
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                node(),
                Kind::Slider,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetSlider(node(), config(), Value::Single(50.)),
            Op::SetSliderAppearance(
                node(),
                Some(Appearance {
                    ring_color: Some(0x00ff00ff),
                    ..Appearance::default()
                }),
            ),
            Op::SetStyle(
                node(),
                vec![
                    WireStyle::Width(WireLength::Px(300.)),
                    WireStyle::Height(WireLength::Px(40.)),
                ],
            ),
            Op::SetRoot(Some(node())),
        ],
    );
    let shared = owner.read_with(cx, |view, _| view.sliders[&node()].clone());
    let before = shared.borrow().model.snapshot();
    let center = shared.borrow().track_bounds.center();
    transport.mailbox.lock().unwrap().drain(256);
    hover(cx, center, false);
    assert!(shared.borrow().rings.pending.is_some());
    tick(cx, 45);
    let progress = shared.borrow().rings.items[0].value;
    assert!(progress > 0. && progress < 1., "{progress}");
    cx.update(|w, _| {
        let green: Hsla = rgba(0x00ff00ff).into();
        assert!(w.painted_quads().iter().any(|q| q.border_color.h == green.h
            && q.border_color.s == green.s
            && q.border_color.a > 0.
            && q.border_color.a < 0.5));
    });
    tick(cx, 2000);
    assert_eq!(shared.borrow().rings.items[0].value, 1.);
    assert!(shared.borrow().rings.pending.is_none());
    assert_eq!(shared.borrow().model.snapshot(), before);
    cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
    assert!(
        transport.mailbox.lock().unwrap().drain(256).is_empty(),
        "hover motion emits no bridge events"
    );
    cx.simulate_event(MouseDownEvent {
        position: center,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    hover(cx, point(px(350.), px(80.)), true);
    assert!(shared.borrow().model.snapshot().dragging.is_some());
    assert_eq!(
        shared.borrow().rings.items[0].target,
        1.,
        "pressed ring survives leaving thumb"
    );
    cx.simulate_event(MouseUpEvent {
        position: point(px(350.), px(80.)),
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(cx);
    tick(cx, 2000);
    assert_eq!(shared.borrow().rings.items[0].value, 0.);
    assert!(shared.borrow().rings.pending.is_none());
    // Reset the numeric value, then inspect immediate reduced-motion feedback.
    cx.update(|w, cx| {
        shared
            .borrow_mut()
            .command(
                Command::Replace {
                    value: Value::Single(50.),
                    if_revision: None,
                },
                w,
                cx,
            )
            .unwrap();
        cx.set_reduce_motion(true);
    });
    draw(cx);
    hover(cx, center, false);
    assert_eq!(shared.borrow().rings.items[0].value, 1.);
    assert!(shared.borrow().rings.pending.is_none());
    hover(cx, point(px(350.), px(80.)), false);
    assert_eq!(shared.borrow().rings.items[0].value, 0.);
    cx.update(|_, cx| cx.set_reduce_motion(false));
    hover(cx, center, false);
    assert!(shared.borrow().rings.pending.is_some());
    tick(cx, 45);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    tick(cx, 1);
    assert_eq!(shared.borrow().rings.items[0].value, 1.);
    assert!(shared.borrow().rings.pending.is_none());
    cx.update(|_, cx| cx.set_reduce_motion(false));
    let mut disabled = config();
    disabled.disabled = true;
    apply(
        &owner,
        cx,
        vec![Op::SetSlider(node(), disabled, Value::Single(50.))],
    );
    assert!(shared.borrow().rings.pending.is_none());
    assert_eq!(shared.borrow().rings.items[0].value, 0.);
    tick(cx, 2000);
    assert!(shared.borrow().rings.pending.is_none());
    // Each hidden/inert policy retires an in-flight spring without an idle wake.
    for field in [
        WireField::Visibility(1),
        WireField::Display(3),
        WireField::Opacity(0.),
        WireField::PointerEvents(false),
        WireField::Inert(true),
        WireField::MarginLeft(WireLength::Px(1000.)),
    ] {
        apply(
            &owner,
            cx,
            vec![
                Op::SetSlider(node(), config(), Value::Single(50.)),
                Op::SetStyle(
                    node(),
                    vec![
                        WireStyle::Width(WireLength::Px(300.)),
                        WireStyle::Height(WireLength::Px(40.)),
                    ],
                ),
            ],
        );
        hover(cx, point(px(350.), px(80.)), false);
        tick(cx, 2000);
        hover(cx, center, false);
        assert!(shared.borrow().rings.pending.is_some());
        apply(
            &owner,
            cx,
            vec![Op::SetStyle(node(), vec![WireStyle::Fields(vec![field])])],
        );
        tick(cx, 2000);
        assert!(shared.borrow().rings.pending.is_none());
        assert_eq!(shared.borrow().rings.items[0].value, 0.);
    }
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            node(),
            vec![
                WireStyle::Width(WireLength::Px(300.)),
                WireStyle::Height(WireLength::Px(40.)),
            ],
        )],
    );
    hover(cx, point(px(350.), px(80.)), false);
    tick(cx, 2000);
    hover(cx, center, false);
    cx.deactivate_window();
    tick(cx, 2000);
    assert!(shared.borrow().rings.pending.is_none());
    assert_eq!(shared.borrow().rings.items[0].value, 0.);
    cx.update(|w, _| w.activate_window());
    draw(cx);
    let mut readonly = config();
    readonly.read_only = true;
    apply(
        &owner,
        cx,
        vec![Op::SetSlider(node(), readonly, Value::Single(50.))],
    );
    tick(cx, 2000);
    assert!(shared.borrow().rings.pending.is_none());
    assert_eq!(shared.borrow().rings.items[0].value, 0.);
    // Keep an external Rc across explicit close: queued wakes and owner state retire.
    apply(
        &owner,
        cx,
        vec![Op::SetSlider(node(), config(), Value::Single(50.))],
    );
    hover(cx, center, false);
    let replacement = NodeId::from_parts(0, 2).unwrap();
    apply(
        &owner,
        cx,
        vec![
            Op::SetRoot(None),
            Op::Remove(node()),
            Op::Create(
                replacement,
                Kind::Slider,
                "".into(),
                Some(HandlerId::from_parts(0, 2).unwrap()),
            ),
            Op::SetSlider(
                replacement,
                config(),
                Value::Range {
                    lower: 20.,
                    upper: 80.,
                },
            ),
            Op::SetStyle(
                replacement,
                vec![
                    WireStyle::Width(WireLength::Px(300.)),
                    WireStyle::Height(WireLength::Px(40.)),
                ],
            ),
            Op::SetRoot(Some(replacement)),
        ],
    );
    assert!(shared.borrow().closed);
    assert!(shared.borrow().rings.pending.is_none());
    let shared = owner.read_with(cx, |view, _| view.sliders[&replacement].clone());
    let bounds = shared.borrow().track_bounds;
    let lower = point(bounds.left() + bounds.size.width * 0.2, bounds.center().y);
    let upper = point(bounds.left() + bounds.size.width * 0.8, bounds.center().y);
    hover(cx, lower, false);
    tick(cx, 45);
    assert_eq!(shared.borrow().rings.items[0].target, 1.);
    assert_eq!(shared.borrow().rings.items[1].target, 0.);
    hover(cx, upper, false);
    tick(cx, 2000);
    assert_eq!(shared.borrow().rings.items[0].value, 0.);
    assert_eq!(shared.borrow().rings.items[1].value, 1.);
    hover(cx, lower, false);
    cx.simulate_event(MouseDownEvent {
        position: lower,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    hover(cx, point(px(350.), px(80.)), true);
    assert_eq!(
        shared.borrow().model.snapshot().dragging,
        Some(Thumb::Lower)
    );
    assert_eq!(shared.borrow().rings.items[0].target, 1.);
    cx.update(|w, _| w.release_pointer());
    draw(cx);
    tick(cx, 2000);
    assert!(shared.borrow().model.snapshot().dragging.is_none());
    assert_eq!(shared.borrow().rings.items[0].value, 0.);
    assert!(shared.borrow().rings.pending.is_none());
    hover(cx, lower, false);
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            view.session.borrow_mut().close(wid).unwrap();
            view.sync_sliders(&[], w, cx);
        })
    });
    assert!(shared.borrow().closed);
    assert!(shared.borrow().rings.pending.is_none());
    transport.mailbox.lock().unwrap().drain(256);
    tick(cx, 2000);
    assert!(transport.mailbox.lock().unwrap().drain(256).is_empty());
}
