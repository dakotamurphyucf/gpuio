//! Production renderer checks on TestPlatform; these do not certify OS input/AX.
use super::*;
use gpui::TestAppContext;
use gpuio_protocol::{
    HandlerId,
    control_appearance::{Config, LabelPosition},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}

fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let applied = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations,
        })
        .unwrap();
    view.update_editors(&applied.dirty, window, cx);
    cx.notify();
}

#[test]
fn appearance_updates_preserve_focus_and_release_replaced_configs() {
    for kind in [Kind::Checkbox, Kind::Switch, Kind::RadioGroup] {
        let mut app = TestAppContext::single();
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        let window_id = WindowId::from_parts(0, 1).unwrap();
        let handler = HandlerId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Appearance", 400., 200.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(window_id, session.clone(), transport));
        let config = |size, label_position| Config {
            size,
            switch_width: size * 2.,
            label_position,
            indicator_style: vec![
                Style::Fields(vec![Field::Background(Fill::Solid(Color::Rgba(
                    0xff0000ff,
                )))]),
                Style::State(
                    6,
                    vec![Field::Background(Fill::Solid(Color::Rgba(0x0000ffff)))],
                ),
            ],
            ..Config::default()
        };
        let value = match kind {
            Kind::Checkbox => {
                Op::SetControl(id(1), Control::Checkbox(CheckState::Indeterminate, false))
            }
            Kind::Switch => Op::SetControl(id(1), Control::Switch(true, false)),
            Kind::RadioGroup => Op::SetChoice(
                id(1),
                ChoiceConfig {
                    label: "Choice".into(),
                    items: vec![ChoiceItem {
                        id: "one".into(),
                        label: "Label".into(),
                        disabled: false,
                    }],
                    selected: Some("one".into()),
                    disabled: false,
                },
            ),
            _ => unreachable!(),
        };
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::Create(id(0), Kind::Container, "".into(), None),
                        Op::Create(id(1), kind, "Label".into(), Some(handler)),
                        value,
                        Op::SetStyle(
                            id(1),
                            vec![Style::Fields(vec![Field::Display(1), Field::Direction(0)])],
                        ),
                        Op::SetControlAppearance(id(1), Some(config(18., LabelPosition::After))),
                        Op::Splice(id(0), 0, 0, vec![id(1)]),
                        Op::SetRoot(Some(id(0))),
                    ],
                )
            });
            window.draw(cx).clear(cx);
        });
        let focus = owner.read_with(cx, |view, _| view.buttons[&id(1)].focus.clone());
        let weak_config = || {
            Arc::downgrade(
                session
                    .borrow()
                    .tree(window_id)
                    .unwrap()
                    .get(id(1))
                    .unwrap()
                    .control_appearance
                    .as_ref()
                    .unwrap(),
            )
        };
        let old = weak_config();
        let mut positions = Vec::new();
        for position in [LabelPosition::After, LabelPosition::Before] {
            cx.update(|window, cx| {
                window.focus(&focus, cx);
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![Op::SetControlAppearance(id(1), Some(config(32., position)))],
                    )
                });
                window.draw(cx).clear(cx);
                assert!(focus.is_focused(window));
                let red: gpui::Background = rgba(0xff0000ff).into();
                let quads = window.painted_quads();
                let indicator = quads
                    .iter()
                    .find(|quad| quad.background == red)
                    .expect("indicator paint");
                assert_eq!(indicator.bounds.size.height.0, 32. * window.scale_factor());
                assert_eq!(
                    indicator.bounds.size.width.0,
                    if kind == Kind::Switch { 64. } else { 32. } * window.scale_factor()
                );
                positions.push(indicator.bounds.origin.x.0);
            });
        }
        assert!(
            positions[1] > positions[0],
            "label-before moves indicator after text: {kind:?}"
        );
        assert!(old.upgrade().is_none());
        let current = weak_config();
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetStyle(
                        id(0),
                        vec![Style::Fields(vec![Field::Disabled(true)])],
                    )],
                )
            });
            window.draw(cx).clear(cx);
            let blue: gpui::Background = rgba(0x0000ffff).into();
            assert!(
                window
                    .painted_quads()
                    .iter()
                    .any(|quad| quad.background == blue)
            );
        });
        owner.read_with(cx, |view, _| assert!(!view.focus.borrow().allows(id(1))));
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::SetStyle(id(0), vec![]),
                        Op::SetControlAppearance(id(1), None),
                    ],
                )
            });
            window.draw(cx).clear(cx);
            window.focus(&focus, cx);
            assert!(focus.is_focused(window));
            // Focusing schedules a normal redraw; appearance adds no recurring wake.
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            assert_eq!(window.simulate_next_frame(cx), 0);
        });
        assert!(current.upgrade().is_none());
        owner.read_with(cx, |view, _| {
            assert_eq!(view.buttons[&id(1)].focus, focus);
            assert!(view.focus.borrow().allows(id(1)));
        });
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::SetRoot(None),
                        Op::Splice(id(0), 0, 1, vec![]),
                        Op::Remove(id(1)),
                        Op::Remove(id(0)),
                    ],
                )
            });
            window.draw(cx).clear(cx);
            // Removing the focused node also schedules focus reconciliation.
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
            assert_eq!(window.simulate_next_frame(cx), 0);
        });
        owner.read_with(cx, |view, _| {
            assert!(view.buttons.is_empty() && view.radios.is_empty())
        });
        assert_eq!(session.borrow().retained_bytes(), 0);
        assert!(
            session
                .borrow_mut()
                .press(window_id, id(1), handler, 1)
                .is_none()
        );
    }
}
