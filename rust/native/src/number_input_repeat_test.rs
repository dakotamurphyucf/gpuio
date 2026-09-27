use super::super::super::native_test::{mouse, move_mouse};
use super::*;
use std::time::Duration;

fn status(cx: &mut AsyncApp, handle: WindowHandle<View>) -> (bool, bool, usize, bool) {
    handle
        .update(cx, |v, w, _| {
            let owner = v.numbers[&node(1)].owner.borrow();
            (
                owner.repeat.is_active(),
                owner.repeat.has_task(),
                owner.repeat.ticks,
                w.captured_hitbox().is_some(),
            )
        })
        .unwrap()
}
fn idle(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    let (active, task, _, captured) = status(cx, handle);
    assert!(
        !active && !task && !captured,
        "repeat retained after cancellation: {:?}",
        status(cx, handle)
    );
}
fn press(cx: &mut AsyncApp, handle: WindowHandle<View>, direction: Direction) -> Point<Pixels> {
    let point = handle
        .update(cx, |v, _, _| {
            v.numbers[&node(1)].owner.borrow().repeat.buttons[if direction == Direction::Increase {
                1
            } else {
                0
            }]
            .center()
        })
        .unwrap();
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    point
}
fn reset(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    apply(
        cx,
        handle,
        vec![
            Op::SetNumberInput(node(1), config(), n::Value::Empty),
            Op::SetStyle(
                node(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(40.)),
                ])],
            ),
        ],
    );
    assert!(matches!(
        command(
            cx,
            handle,
            n::Command::ReplaceValue {
                value: n::Value::Number(1.),
                selection: n::SelectionPolicy::End,
                undo: n::UndoPolicy::Reset,
                if_revision: None,
            }
        ),
        n::Response::Applied(_)
    ));
}
async fn activation(cx: &mut AsyncApp, handle: WindowHandle<View>, active: bool) {
    for _ in 0..100 {
        if handle
            .update(cx, |_, w, _| w.is_window_active() == active)
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(20))
            .await;
    }
    panic!("window did not reach activation={active}");
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    reset(cx, handle);
    frame(cx, handle).await;
    events(transport);
    idle(cx, handle);
    let point = press(cx, handle, Direction::Increase);
    assert_eq!(snapshot(cx, handle).committed, n::Value::Number(1.5));
    assert!(status(cx, handle).0, "button must begin a captured hold");
    // A repaint changes hitbox identity; the held gesture must rebind safely.
    frame(cx, handle).await;
    assert!(status(cx, handle).0);
    apply(
        cx,
        handle,
        vec![
            Op::SetNumberInput(
                node(1),
                n::Config {
                    label: "Updated temperature label".into(),
                    ..config()
                },
                n::Value::Empty,
            ),
            Op::SetStyle(
                node(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(40.)),
                    Field::Foreground(gpuio_protocol::v1::Color::Rgba(0x3366aaff)),
                ])],
            ),
        ],
    );
    frame(cx, handle).await;
    assert!(
        status(cx, handle).0,
        "label/color update interrupted held step"
    );
    for _ in 0..100 {
        if status(cx, handle).2 >= 2 {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(20))
            .await;
    }
    assert!(status(cx, handle).2 >= 2, "native timer did not step");
    assert!(matches!(snapshot(cx, handle).committed, n::Value::Number(v) if v >= 2.5));
    mouse(cx, handle, point, false);
    idle(cx, handle);
    let ticks = status(cx, handle).2;
    let value = snapshot(cx, handle).committed;
    cx.background_executor()
        .timer(repeat::DELAY + repeat::INTERVAL)
        .await;
    assert_eq!(
        status(cx, handle).2,
        ticks,
        "idle field woke its repeat timer"
    );
    assert_eq!(snapshot(cx, handle).committed, value);
    assert!(
        events(transport)
            .iter()
            .filter(|e| matches!(e, n::Event::Committed(n::Source::Stepper, _)))
            .count()
            >= 3
    );

    for case in 0..15 {
        reset(cx, handle);
        frame(cx, handle).await;
        let point = press(cx, handle, Direction::Decrease);
        assert!(status(cx, handle).0, "case {case}: hold did not start");
        match case {
            0 => move_mouse(cx, handle, point + gpui::point(px(0.), px(100.)), true),
            1 => {
                handle.update(cx, |_, w, _| w.release_pointer()).unwrap();
                frame(cx, handle).await;
            }
            2 => key(cx, handle, "escape"),
            3 => {
                replace(cx, handle, "3e-");
            }
            4 => apply(
                cx,
                handle,
                vec![Op::SetNumberInput(
                    node(1),
                    n::Config {
                        read_only: true,
                        ..config()
                    },
                    n::Value::Empty,
                )],
            ),
            5 => apply(
                cx,
                handle,
                vec![Op::SetNumberInput(
                    node(1),
                    n::Config {
                        disabled: true,
                        ..config()
                    },
                    n::Value::Empty,
                )],
            ),
            6 => apply(
                cx,
                handle,
                vec![Op::SetNumberInput(
                    node(1),
                    n::Config {
                        step_controls: n::StepControls::Hidden,
                        ..config()
                    },
                    n::Value::Empty,
                )],
            ),
            7 => apply(
                cx,
                handle,
                vec![Op::SetStyle(
                    node(1),
                    vec![Style::Fields(vec![Field::PointerEvents(false)])],
                )],
            ),
            8 => {
                apply(
                    cx,
                    handle,
                    vec![Op::SetStyle(
                        node(1),
                        vec![Style::Fields(vec![Field::Display(3)])],
                    )],
                );
                frame(cx, handle).await;
            }
            9 => {
                apply(
                    cx,
                    handle,
                    vec![Op::SetStyle(
                        node(1),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(240.)),
                            Field::Height(Length::Px(60.)),
                        ])],
                    )],
                );
                frame(cx, handle).await;
            }
            10 => {
                native_text(cx, handle, "1", false);
            }
            11 => {
                handle.update(cx, |_, w, cx| w.blur(cx)).unwrap();
                frame(cx, handle).await;
            }
            12 => key(cx, handle, "up"),
            13 => apply(
                cx,
                handle,
                vec![Op::SetNumberInput(
                    node(1),
                    n::Config {
                        domain: Domain::new(-4., 20., 1.).unwrap(),
                        ..config()
                    },
                    n::Value::Empty,
                )],
            ),
            14 => apply(
                cx,
                handle,
                vec![Op::Bind(
                    node(1),
                    Some(HandlerId::from_parts(2, 1).unwrap()),
                )],
            ),
            _ => unreachable!(),
        }
        assert!(
            !status(cx, handle).0,
            "case {case}: hold survived cancellation"
        );
        idle(cx, handle);
        if case == 2 {
            assert!(
                events(transport)
                    .iter()
                    .any(|e| matches!(e, n::Event::Cancelled(n::CancelReason::Escape, _)))
            );
        }
        let before = snapshot(cx, handle);
        mouse(cx, handle, point, false);
        assert_eq!(
            snapshot(cx, handle).committed,
            before.committed,
            "late release edited cancelled gesture"
        );
        events(transport);
    }
    reset(cx, handle);
    frame(cx, handle).await;
    replace(cx, handle, "-");
    let point = press(cx, handle, Direction::Increase);
    assert_eq!(snapshot(cx, handle).draft, "-");
    idle(cx, handle);
    mouse(cx, handle, point, false);
    assert!(
        events(transport)
            .iter()
            .any(|e| matches!(e, n::Event::Rejected(n::Rejection::Incomplete, _)))
    );
    replace(cx, handle, "7.5");
    let point = press(cx, handle, Direction::Increase);
    assert_eq!(snapshot(cx, handle).committed, n::Value::Number(8.));
    idle(cx, handle);
    mouse(cx, handle, point, false);
    events(transport);

    // Two vertically adjacent buttons must not cancel one another's capture.
    reset(cx, handle);
    apply(
        cx,
        handle,
        vec![
            Op::SetNumberInput(
                node(1),
                n::Config {
                    step_controls: n::StepControls::Stacked,
                    ..config()
                },
                n::Value::Empty,
            ),
            Op::SetStyle(
                node(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(64.)),
                ])],
            ),
        ],
    );
    frame(cx, handle).await;
    let prior_ticks = status(cx, handle).2;
    let point = press(cx, handle, Direction::Increase);
    frame(cx, handle).await;
    for _ in 0..100 {
        if status(cx, handle).2 > prior_ticks {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(20))
            .await;
    }
    assert!(
        status(cx, handle).2 > prior_ticks,
        "stacked button did not repeat"
    );
    mouse(cx, handle, point, false);
    idle(cx, handle);
    let before = snapshot(cx, handle).committed;
    let point = press(cx, handle, Direction::Decrease);
    mouse(cx, handle, point, false);
    assert!(
        matches!((before, snapshot(cx, handle).committed), (n::Value::Number(before), n::Value::Number(after)) if after == before - 0.5)
    );
    idle(cx, handle);
    events(transport);
    #[cfg(target_os = "macos")]
    {
        replace(cx, handle, "");
        frame(cx, handle).await;
        native_text(cx, handle, "に", true);
        let before = snapshot(cx, handle);
        let point = press(cx, handle, Direction::Increase);
        mouse(cx, handle, point, false);
        idle(cx, handle);
        let after = snapshot(cx, handle);
        assert_eq!(after.draft, before.draft);
        assert_eq!(after.composition, before.composition);
        assert_eq!(after.committed, before.committed);
        assert!(
            events(transport)
                .iter()
                .any(|e| matches!(e, n::Event::Rejected(n::Rejection::Composing, _)))
        );
        key(cx, handle, "escape");
    }

    // Real OS activation loss cancels, including before the first repeat tick.
    reset(cx, handle);
    frame(cx, handle).await;
    let point = press(cx, handle, Direction::Increase);
    assert!(status(cx, handle).0);
    let (session, shared_transport) = handle
        .update(cx, |v, _, _| (v.session.clone(), v.transport.clone()))
        .unwrap();
    let other_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, other_id, "Numeric hold cancellation", 240., 100.)
        .unwrap();
    let other = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                focus: true,
                show: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(240.), px(100.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(other_id, session.clone(), shared_transport.clone())),
        )
        .unwrap()
    });
    activation(cx, handle, false).await;
    idle(cx, handle);
    let ticks = status(cx, handle).2;
    cx.background_executor()
        .timer(repeat::DELAY + repeat::INTERVAL)
        .await;
    assert_eq!(status(cx, handle).2, ticks);
    let first_value = snapshot(cx, handle).committed;
    apply(
        cx,
        other,
        vec![
            Op::Create(node(0), Kind::Container, "".into(), None),
            Op::Create(
                node(1),
                Kind::NumberInput,
                "".into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::SetNumberInput(node(1), config(), n::Value::Number(4.)),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, other).await;
    press(cx, other, Direction::Increase);
    assert_eq!(snapshot(cx, other).committed, n::Value::Number(4.5));
    assert_eq!(snapshot(cx, handle).committed, first_value);
    let (closed_owner, closed_input) = other
        .update(cx, |v, _, _| {
            let instance = &v.numbers[&node(1)];
            (Rc::downgrade(&instance.owner), instance.state.downgrade())
        })
        .unwrap();
    other
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    handle.update(cx, |_, w, _| w.activate_window()).unwrap();
    activation(cx, handle, true).await;
    frame(cx, handle).await;
    assert!(closed_owner.upgrade().is_none());
    assert!(closed_input.upgrade().is_none());
    assert!(session.borrow().tree(other_id).is_none());
    assert_eq!(snapshot(cx, handle).committed, first_value);
    mouse(cx, handle, point, false);
    idle(cx, handle);
    events(transport);
    // Leave a held gesture for the parent suite's actual unmount/disposal check.
    reset(cx, handle);
    frame(cx, handle).await;
    press(cx, handle, Direction::Increase);
    assert!(status(cx, handle).0);
    eprintln!(
        "GPUIO_NUMBER_REPEAT_OK: native delayed repeat, repaint capture, release/leave/capture/Escape/edit/policy/hide/geometry/window cancellation, bounds, independent-window close and idle timer disposal"
    );
}
