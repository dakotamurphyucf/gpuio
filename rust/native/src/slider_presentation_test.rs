//! Retained slider paint and capture on TestPlatform; no OS windows.
use super::*;
use crate::{session::Session, transport::Transport};
use gpuio_protocol::{
    HandlerId, WindowId,
    numeric::Domain,
    v1::{CAPABILITIES, Kind, Length as WireLength, Op, Style as WireStyle, Transaction, VERSION},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
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
fn config(axis: Axis) -> gpuio_protocol::slider::Config {
    gpuio_protocol::slider::Config {
        domain: Domain::new(0., 100., 1.).unwrap(),
        label: "Level".into(),
        lower_label: "Low".into(),
        upper_label: "High".into(),
        axis,
        scale: gpuio_protocol::slider::Scale::Linear,
        disabled: false,
        read_only: false,
    }
}
fn exercise(value: Value) {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Slider presentation", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|w, _| w.activate_window());
    let appearance = Appearance {
        track_thickness: 6.,
        track_radius: 3.,
        thumb_size: 16.,
        target_size: 28.,
        track_color: Some(0x112233ff),
        fill_color: Some(0x00ff00ff),
        thumb_color: Some(0xff0000ff),
        ring_color: Some(0x0000ffff),
        ..Appearance::default()
    };
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
            Op::SetSlider(node(), config(Axis::Horizontal), value),
            Op::SetSliderAppearance(node(), Some(appearance.clone())),
            Op::SetStyle(
                node(),
                vec![
                    WireStyle::Width(WireLength::Px(240.)),
                    WireStyle::Height(WireLength::Px(240.)),
                ],
            ),
            Op::SetRoot(Some(node())),
        ],
    );
    let shared = owner.read_with(cx, |view, _| view.sliders[&node()].clone());
    let focus = shared.borrow().focus[0].1.clone();
    cx.update(|w, cx| {
        w.focus(&focus, cx);
        w.draw(cx).clear(cx);
    });
    for axis in [Axis::Horizontal, Axis::Vertical] {
        apply(&owner, cx, vec![Op::SetSlider(node(), config(axis), value)]);
        let before = shared.borrow().model.snapshot();
        for fill_mode in [Fill::Selected, Fill::Remaining] {
            apply(
                &owner,
                cx,
                vec![Op::SetSliderAppearance(
                    node(),
                    Some(Appearance {
                        fill: fill_mode,
                        ..appearance.clone()
                    }),
                )],
            );
            assert!(Rc::ptr_eq(
                &shared,
                &owner.read_with(cx, |view, _| view.sliders[&node()].clone())
            ));
            assert_eq!(before, shared.borrow().model.snapshot());
            cx.update(|w, _| {
                assert!(focus.is_focused(w));
                let quads = w.painted_quads();
                let rail = quads
                    .iter()
                    .find(|q| q.background == rgba(0x112233ff).into())
                    .unwrap();
                let selected = quads
                    .iter()
                    .find(|q| q.background == rgba(0x00ff00ff).into())
                    .unwrap();
                let thumbs: Vec<_> = quads
                    .iter()
                    .filter(|q| q.background == rgba(0xff0000ff).into())
                    .collect();
                assert_eq!(
                    thumbs.len(),
                    if matches!(value, Value::Single(_)) {
                        1
                    } else {
                        2
                    }
                );
                assert!(
                    thumbs
                        .iter()
                        .all(|q| q.bounds.size.width == px(16.).scale(w.scale_factor()))
                );
                assert!(
                    quads
                        .iter()
                        .any(|q| q.border_color == rgba(0x0000ffff).into())
                );
                let (low, high) = match value {
                    Value::Single(v) => {
                        if fill_mode == Fill::Remaining {
                            (v / 100., 1.)
                        } else {
                            (0., v / 100.)
                        }
                    }
                    Value::Range { lower, upper } => (lower / 100., upper / 100.),
                };
                let (offset, length, total) = match axis {
                    Axis::Horizontal => (
                        selected.bounds.left() - rail.bounds.left(),
                        selected.bounds.size.width,
                        rail.bounds.size.width,
                    ),
                    Axis::Vertical => (
                        rail.bounds.bottom() - selected.bounds.bottom(),
                        selected.bounds.size.height,
                        rail.bounds.size.height,
                    ),
                };
                assert!(((offset / total) as f64 - low).abs() < 0.001);
                assert!(((length / total) as f64 - (high - low)).abs() < 0.001);
            });
        }
    }
    let point = shared.borrow().track_bounds.center();
    cx.simulate_event(MouseMoveEvent {
        position: point,
        pressed_button: None,
        modifiers: Default::default(),
    });
    cx.simulate_event(MouseDownEvent {
        position: point,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    assert!(shared.borrow().model.snapshot().dragging.is_some());
    let dragging = shared.borrow().model.snapshot();
    apply(
        &owner,
        cx,
        vec![Op::SetSliderAppearance(node(), Some(appearance.clone()))],
    );
    assert_eq!(
        shared.borrow().model.snapshot(),
        dragging,
        "paint-only changes preserve capture"
    );
    apply(
        &owner,
        cx,
        vec![Op::SetSliderAppearance(
            node(),
            Some(Appearance {
                target_size: 36.,
                ..appearance
            }),
        )],
    );
    assert!(shared.borrow().model.snapshot().dragging.is_none());
    assert!(shared.borrow().capture.is_none());
    assert_eq!(shared.borrow().model.snapshot().value, dragging.committed);
    let before = shared.borrow().model.snapshot();
    apply(&owner, cx, vec![Op::SetSliderAppearance(node(), None)]);
    assert_eq!(shared.borrow().model.snapshot(), before);
    assert_eq!(shared.borrow().appearance.as_ref(), &Appearance::default());
}
#[::core::prelude::v1::test]
fn appearance_retains_values_focus_and_capture_but_geometry_change_cancels() {
    exercise(Value::Single(25.));
    exercise(Value::Range {
        lower: 20.,
        upper: 70.,
    });
}
