//! Retained composition/history and gates across layout, hiding and modal scopes.
use super::super::super::native_test::{mouse, move_mouse};
use super::*;

fn reset(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    apply(
        cx,
        handle,
        vec![
            Op::SetNumberInput(node(1), config(), n::Value::Empty),
            Op::SetStyle(
                node(1),
                vec![
                    Style::Width(Length::Px(300.)),
                    Style::Height(Length::Px(40.)),
                ],
            ),
        ],
    );
    assert!(matches!(
        command(
            cx,
            handle,
            n::Command::ReplaceValue {
                value: n::Value::Number(1.5),
                selection: n::SelectionPolicy::End,
                undo: n::UndoPolicy::Reset,
                if_revision: None,
            }
        ),
        n::Response::Applied(_)
    ));
    assert!(matches!(
        command(cx, handle, n::Command::Focus),
        n::Response::Applied(_)
    ));
}
fn press(cx: &mut AsyncApp, handle: WindowHandle<View>) -> Point<Pixels> {
    let point = handle
        .update(cx, |v, _, _| {
            v.numbers[&node(1)].owner.borrow().repeat.buttons[1].center()
        })
        .unwrap();
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    assert!(
        handle
            .update(cx, |v, _, _| v.numbers[&node(1)]
                .owner
                .borrow()
                .repeat
                .is_active())
            .unwrap()
    );
    point
}
fn stopped(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |v, w, _| {
            let repeat = &v.numbers[&node(1)].owner.borrow().repeat;
            assert!(!repeat.is_active() && !repeat.has_task());
            assert!(w.captured_hitbox().is_none());
        })
        .unwrap();
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    reset(cx, handle);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        replace(cx, handle, "");
        frame(cx, handle).await;
        native_text(cx, handle, "に", true);
        frame(cx, handle).await;
        let before = snapshot(cx, handle);
        assert!(before.composition.is_some());
        let (entity, history) = handle
            .update(cx, |v, _, cx| {
                let input = &v.numbers[&node(1)].state;
                (input.entity_id(), input.read(cx).bridge_history_bytes())
            })
            .unwrap();
        apply(
            cx,
            handle,
            vec![
                Op::SetNumberInput(
                    node(1),
                    n::Config {
                        domain: Domain::new(0., 4., 1.).unwrap(),
                        label: "Updated temperature".into(),
                        placeholder: "Updated hint".into(),
                        step_controls: n::StepControls::Stacked,
                        ..config()
                    },
                    n::Value::Number(4.),
                ),
                Op::SetStyle(
                    node(1),
                    vec![
                        Style::Width(Length::Px(280.)),
                        Style::Height(Length::Px(48.)),
                        Style::Foreground(Color::Rgba(0x2244aaff)),
                    ],
                ),
            ],
        );
        frame(cx, handle).await;
        let after = snapshot(cx, handle);
        assert_eq!(
            (
                &after.draft,
                after.selection,
                after.composition,
                after.focused
            ),
            (
                &before.draft,
                before.selection,
                before.composition,
                before.focused
            )
        );
        assert_eq!(after.committed, n::Value::Number(2.));
        handle
            .update(cx, |v, _, cx| {
                let input = &v.numbers[&node(1)].state;
                assert_eq!(input.entity_id(), entity);
                assert_eq!(input.read(cx).bridge_history_bytes(), history);
            })
            .unwrap();
        key(cx, handle, "escape");
        assert!(snapshot(cx, handle).composition.is_none());
        assert_eq!(snapshot(cx, handle).draft, before.draft);
        assert!(matches!(
            command(cx, handle, n::Command::Cancel),
            n::Response::Applied(_)
        ));
        assert_eq!(snapshot(cx, handle).draft, "2");
        events(transport);
    }
    reset(cx, handle);
    frame(cx, handle).await;
    events(transport);
    let entity = handle
        .update(cx, |v, _, _| v.numbers[&node(1)].state.entity_id())
        .unwrap();
    // The field remains mounted while its ancestor is hidden. Native repeat and
    // user focus stop; explicit programmatic state access remains available.
    let point = press(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    stopped(cx, handle);
    assert_eq!(
        command(cx, handle, n::Command::Focus),
        n::Response::Failed(n::Error::FocusBlocked)
    );
    let updated = replace(cx, handle, "3e-");
    assert_eq!(updated.draft, "3e-");
    mouse(cx, handle, point, false);
    assert_eq!(snapshot(cx, handle).draft, "3e-");
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(320.)),
                Style::Height(Length::Px(120.)),
            ],
        )],
    );
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).draft, "3e-");
    handle
        .update(cx, |v, _, _| {
            assert_eq!(v.numbers[&node(1)].state.entity_id(), entity)
        })
        .unwrap();

    reset(cx, handle);
    frame(cx, handle).await;
    events(transport);
    let point = press(cx, handle);
    let stepped = snapshot(cx, handle).committed;
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(2), Kind::Container, "".into(), None),
            Op::Create(node(3), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(3),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(
                node(4),
                Kind::Button,
                "Modal action".into(),
                Some(HandlerId::from_parts(4, 1).unwrap()),
            ),
            Op::Splice(node(3), 0, 0, vec![node(4)]),
            Op::Splice(node(2), 0, 0, vec![node(0), node(3)]),
            Op::SetRoot(Some(node(2))),
        ],
    );
    frame(cx, handle).await;
    stopped(cx, handle);
    assert_eq!(
        command(cx, handle, n::Command::Focus),
        n::Response::Failed(n::Error::FocusBlocked)
    );
    handle
        .update(cx, |v, w, cx| {
            let input = &v.numbers[&node(1)];
            let response = perform(
                &Rc::downgrade(&input.owner),
                &input.state.downgrade(),
                &n::Command::Step(Direction::Increase),
                n::Source::Accessibility,
                w,
                cx,
            );
            assert_eq!(response, n::Response::Failed(n::Error::FocusBlocked));
        })
        .unwrap();
    mouse(cx, handle, point, false);
    key(cx, handle, "up");
    assert_eq!(snapshot(cx, handle).committed, stepped);
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(2), 0, 2, vec![]),
            Op::SetRoot(Some(node(0))),
            Op::Remove(node(4)),
            Op::Remove(node(3)),
            Op::Remove(node(2)),
        ],
    );
    frame(cx, handle).await;
    assert!(matches!(
        command(cx, handle, n::Command::Focus),
        n::Response::Applied(_)
    ));
    handle
        .update(cx, |v, _, _| {
            assert_eq!(v.numbers[&node(1)].state.entity_id(), entity)
        })
        .unwrap();
    assert_eq!(snapshot(cx, handle).committed, stepped);
    events(transport);
    eprintln!(
        "GPUIO_NUMBER_POLICY_OK: retained IME/history/configuration, hidden ancestor, modal hold/focus/action gates and native identity"
    );
}
