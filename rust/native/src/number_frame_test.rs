//! Retained numeric presentation on TestPlatform, without OS windows.
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::{
    number_presentation::Config as Appearance,
    numeric::Domain,
    v1::{Color as WireColor, Field as WireField, Length as WireLength, Style as WireStyle},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn id(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn config() -> n::Config {
    n::Config {
        domain: Domain::new(0., 100., 1.).unwrap(),
        label: "Amount".into(),
        placeholder: "Quantity".into(),
        increment_label: "Increase amount".into(),
        decrement_label: "Decrease amount".into(),
        step_controls: n::StepControls::Sides,
        allow_empty: false,
        disabled: false,
        read_only: false,
        auto_focus: false,
    }
}
fn appearance(width: f64) -> Appearance {
    Appearance {
        button_width: width,
        border_width: Some(2.),
        frame_style: vec![WireStyle::State(
            1,
            vec![WireField::BorderColor(WireColor::Rgba(0x00ff00ff))],
        )],
        editor_style: vec![WireStyle::Fields(vec![WireField::FontSize(16.)])],
        ..Default::default()
    }
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
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
                .unwrap_or_else(|error| panic!("transaction after revision {base}: {error:?}"));
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
    });
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn command(owner: &Entity<View>, cx: &mut VisualTestContext, command: n::Command) -> n::Snapshot {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            match view.numbers[&id(0)].command(&command, window, cx) {
                n::Response::Applied(snapshot) => snapshot,
                other => panic!("unexpected numeric result: {other:?}"),
            }
        })
    })
}
#[::core::prelude::v1::test]
fn numeric_disabled_part_styles_follow_current_policy_without_replacing_editor() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Numeric states", 600., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let colors = [0xa10000ff, 0x00a200ff, 0x0000a3ff, 0xa4a400ff];
    let disabled_style = |color| {
        vec![WireStyle::State(
            6,
            vec![WireField::Background(gpuio_protocol::v1::Fill::Solid(
                WireColor::Rgba(color),
            ))],
        )]
    };
    let appearance = Appearance {
        frame_style: disabled_style(colors[0]),
        editor_style: disabled_style(colors[1]),
        decrement_style: disabled_style(colors[2]),
        increment_style: disabled_style(colors[3]),
        ..Default::default()
    };
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                id(0),
                Kind::NumberInput,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetNumberInput(id(0), config(), n::Value::Number(12.)),
            Op::SetNumberPresentation(id(0), Some(appearance)),
            Op::Create(id(1), Kind::Container, "".into(), None),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::Create(id(3), Kind::Container, "".into(), None),
            Op::Create(id(4), Kind::Container, "".into(), None),
            Op::Splice(id(0), 0, 0, (1..=4).map(id).collect()),
            Op::SetStyle(
                id(0),
                vec![
                    WireStyle::Width(WireLength::Px(500.)),
                    WireStyle::Height(WireLength::Px(44.)),
                ],
            ),
            Op::SetRoot(Some(id(0))),
        ],
    );
    let editor = owner.read_with(cx, |view, _| view.numbers[&id(0)].state.entity_id());
    for (disabled, read_only, expected) in [
        (false, false, [false; 4]),
        (true, false, [true; 4]),
        (false, true, [false, false, true, true]),
        (false, false, [false; 4]),
    ] {
        let config = n::Config {
            disabled,
            read_only,
            ..config()
        };
        apply(
            &owner,
            cx,
            vec![Op::SetNumberInput(id(0), config, n::Value::Number(12.))],
        );
        draw(cx);
        owner.read_with(cx, |view, _| {
            assert_eq!(view.numbers[&id(0)].state.entity_id(), editor)
        });
        cx.update(|window, _| {
            for (index, color) in colors.into_iter().enumerate() {
                let background: gpui::Background = rgba(color as u32).into();
                // The host dims an explicitly disabled control as a whole.
                let background = background.opacity(if disabled { 0.5 } else { 1. });
                let painted = window
                    .painted_quads()
                    .iter()
                    .any(|quad| quad.background == background);
                assert_eq!(
                    painted, expected[index],
                    "part={index}, disabled={disabled}, read_only={read_only}"
                );
            }
        });
    }
}
#[::core::prelude::v1::test]
fn number_frame_retains_editor_composition_and_routes_auxiliary_actions_through_current_policy() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Numeric frame", 600., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                id(0),
                Kind::NumberInput,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetNumberInput(id(0), config(), n::Value::Number(12.)),
            Op::SetNumberInputDraft(id(0), Some("1e-".into())),
            Op::SetStyle(
                id(0),
                vec![
                    WireStyle::Width(WireLength::Px(500.)),
                    WireStyle::Height(WireLength::Px(44.)),
                ],
            ),
            Op::SetRoot(Some(id(0))),
        ],
    );
    draw(cx);
    let state = owner.read_with(cx, |view, _| view.numbers[&id(0)].state.clone());
    command(&owner, cx, n::Command::Focus);
    draw(cx);
    let before = owner.read_with(cx, |view, _| {
        view.numbers[&id(0)].owner.borrow().model.snapshot().clone()
    });
    assert_eq!(before.draft, "1e-");
    assert_eq!(before.committed, n::Value::Number(12.));
    let mut ops = vec![Op::SetNumberPresentation(id(0), Some(appearance(42.)))];
    for slot in 1..=4 {
        ops.push(Op::Create(id(slot), Kind::Container, "".into(), None));
    }
    ops.extend([
        Op::Create(
            id(5),
            Kind::Button,
            "Currency".into(),
            Some(HandlerId::from_parts(1, 1).unwrap()),
        ),
        Op::Create(id(6), Kind::Text, "USD".into(), None),
        Op::Create(id(7), Kind::Text, "−".into(), None),
        Op::Create(id(8), Kind::Text, "+".into(), None),
        Op::Splice(id(1), 0, 0, vec![id(5)]),
        Op::Splice(id(2), 0, 0, vec![id(6)]),
        Op::Splice(id(3), 0, 0, vec![id(7)]),
        Op::Splice(id(4), 0, 0, vec![id(8)]),
        Op::Splice(id(0), 0, 0, (1..=4).map(id).collect()),
    ]);
    apply(&owner, cx, ops);
    draw(cx);
    owner.read_with(cx, |view, _| {
        assert_eq!(view.numbers[&id(0)].state.entity_id(), state.entity_id());
        assert_eq!(
            view.numbers[&id(0)].owner.borrow().model.snapshot(),
            &before
        );
        assert!(view.editors.is_empty());
        assert_eq!(
            view.buttons.len(),
            1,
            "only auxiliary Currency creates an ordinary button owner"
        );
    });
    cx.update(|window, _| {
        assert!(
            window
                .painted_quads()
                .iter()
                .any(|q| q.border_color == rgba(0x00ff00ff).into()
                    && q.bounds.size.width >= px(500.).scale(window.scale_factor()))
        )
    });
    // One accepted edit creates history; presentation changes during preedit retain it.
    command(
        &owner,
        cx,
        n::Command::ReplaceDraft {
            text: "12".into(),
            selection: n::SelectionPolicy::End,
            undo: n::UndoPolicy::Record,
            if_revision: None,
        },
    );
    cx.update(|window, cx| {
        state.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(Some(0..2), "三", Some(0..1), window, cx)
        })
    });
    draw(cx);
    let composing = cx.update(|window, cx| editor::snapshot(state.read(cx), window, cx));
    assert!(composing.composition.is_some());
    let history = state.read_with(cx, |state, _| state.bridge_history_bytes());
    apply(
        &owner,
        cx,
        vec![Op::SetNumberPresentation(id(0), Some(appearance(50.)))],
    );
    draw(cx);
    cx.update(|window, cx| assert_eq!(editor::snapshot(state.read(cx), window, cx), composing));
    assert_eq!(
        state.read_with(cx, |state, _| state.bridge_history_bytes()),
        history
    );
    cx.update(|window, cx| state.update(cx, |state, cx| state.unmark_text(window, cx)));
    draw(cx);
    command(
        &owner,
        cx,
        n::Command::ReplaceValue {
            value: n::Value::Number(12.),
            selection: n::SelectionPolicy::End,
            undo: n::UndoPolicy::Record,
            if_revision: None,
        },
    );
    draw(cx);
    // Existing native keyboard stepping remains available under custom presentation.
    cx.simulate_keystrokes("up");
    draw(cx);
    let stepped = command(&owner, cx, n::Command::ReadSnapshot);
    assert_eq!(stepped.committed, n::Value::Number(13.));
    let mut painted = appearance(50.);
    painted.increment_style = vec![
        WireStyle::Fields(vec![WireField::Background(
            gpuio_protocol::v1::Fill::Solid(WireColor::Rgba(0x0000ffff)),
        )]),
        WireStyle::State(
            2,
            vec![WireField::Background(gpuio_protocol::v1::Fill::Solid(
                WireColor::Rgba(0xffff00ff),
            ))],
        ),
        WireStyle::State(
            3,
            vec![WireField::Background(gpuio_protocol::v1::Fill::Solid(
                WireColor::Rgba(0xff0000ff),
            ))],
        ),
    ];
    apply(
        &owner,
        cx,
        vec![Op::SetNumberPresentation(id(0), Some(painted.clone()))],
    );
    draw(cx);
    let has_color = |cx: &mut VisualTestContext, color| {
        cx.update(|window, _| {
            let color: gpui::Background = rgba(color).into();
            window.painted_quads().iter().any(|q| q.background == color)
        })
    };
    assert!(has_color(cx, 0x0000ffff));
    let button = owner.read_with(cx, |view, _| {
        view.numbers[&id(0)].owner.borrow().repeat.buttons[1].center()
    });
    cx.simulate_event(MouseMoveEvent {
        position: button,
        pressed_button: None,
        modifiers: Default::default(),
    });
    draw(cx);
    assert!(has_color(cx, 0xffff00ff));
    cx.simulate_event(MouseDownEvent {
        position: button,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    draw(cx);
    assert!(has_color(cx, 0xff0000ff));
    assert!(owner.read_with(cx, |view, _| {
        view.numbers[&id(0)].owner.borrow().repeat.has_task()
    }));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    cx.run_until_parked();
    draw(cx);
    assert!(
        owner.read_with(cx, |view, _| view.numbers[&id(0)]
            .owner
            .borrow()
            .repeat
            .ticks)
            > 0
    );
    painted.button_width = 60.;
    apply(
        &owner,
        cx,
        vec![Op::SetNumberPresentation(id(0), Some(painted))],
    );
    draw(cx);
    assert!(!owner.read_with(cx, |view, _| {
        view.numbers[&id(0)].owner.borrow().repeat.has_task()
    }));
    cx.update(|window, _| assert!(window.captured_hitbox().is_none()));
    cx.simulate_event(MouseUpEvent {
        position: button,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(cx);
    // Layout changes preserve the same editor and value. Hidden buttons also
    // remove their decorative content and AX button nodes, while the spinbutton
    // retains native accessible increment/decrement actions.
    let retained = command(&owner, cx, n::Command::ReadSnapshot);
    for controls in [
        n::StepControls::Stacked,
        n::StepControls::Hidden,
        n::StepControls::Sides,
    ] {
        let mut mode = config();
        mode.step_controls = controls;
        apply(
            &owner,
            cx,
            vec![Op::SetNumberInput(id(0), mode, n::Value::Number(12.))],
        );
        draw(cx);
        let mut after = command(&owner, cx, n::Command::ReadSnapshot);
        // Editing-policy updates advance the numeric revision even when the
        // retained draft, value, selection and focus do not change.
        assert!(after.revision > retained.revision);
        after.revision = retained.revision;
        assert_eq!(after, retained);
        assert_eq!(
            owner.read_with(cx, |view, _| view.numbers[&id(0)].state.entity_id()),
            state.entity_id()
        );
        let tree = cx.a11y_tree().unwrap();
        let buttons = tree
            .nodes
            .iter()
            .filter(|(_, node)| matches!(node.label(), Some("Increase amount" | "Decrease amount")))
            .count();
        assert_eq!(
            buttons,
            if controls == n::StepControls::Hidden {
                0
            } else {
                2
            }
        );
    }
    // Until the opt-in event adapter is connected, inject a model intent here
    // and exercise the real mounted resolution/undo path rather than a stub.
    let pending = cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let instance = &view.numbers[&id(0)];
            let live = editor::snapshot(instance.state.read(cx), window, cx);
            instance
                .owner
                .borrow_mut()
                .model
                .request_application_step(&live, Direction::Increase, n::Source::Keyboard)
                .unwrap()
                .request
                .unwrap()
        })
    });
    let resolved = command(
        &owner,
        cx,
        n::Command::ResolveStep {
            request_id: pending.id,
            revision: pending.snapshot.revision,
            value: Some(n::Value::Number(20.)),
        },
    );
    assert_eq!(resolved.committed, n::Value::Number(20.));
    let undone = command(&owner, cx, n::Command::Undo);
    assert_eq!(undone.draft, pending.snapshot.draft);
    assert_eq!(undone.committed, n::Value::Number(20.));
    let tree = cx.a11y_tree().unwrap();
    let currency = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Currency"))
        .unwrap()
        .0;
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: currency,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    let mut disabled = config();
    disabled.disabled = true;
    apply(
        &owner,
        cx,
        vec![Op::SetNumberInput(id(0), disabled, n::Value::Number(12.))],
    );
    cx.run_until_parked();
    draw(cx);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|e| matches!(e, Event::Press(..))),
        "queued auxiliary activation must respect disabled numeric parent"
    );
    // Read-only leaves the independent auxiliary action enabled.
    let mut readonly = config();
    readonly.read_only = true;
    apply(
        &owner,
        cx,
        vec![Op::SetNumberInput(id(0), readonly, n::Value::Number(12.))],
    );
    draw(cx);
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: currency,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|e| matches!(e,Event::Press(_,node,_,_) if *node==id(5)))
    );
    let mut remove = vec![
        Op::SetNumberPresentation(id(0), None),
        Op::Splice(id(0), 0, 4, vec![]),
    ];
    for n in (1..=8).rev() {
        remove.push(Op::Remove(id(n)));
    }
    apply(&owner, cx, remove);
    draw(cx);
    owner.read_with(cx, |view, _| {
        assert_eq!(view.numbers[&id(0)].state.entity_id(), state.entity_id());
        assert!(view.buttons.is_empty());
    });
}

fn requests(transport: &Transport) -> Vec<n::StepRequest> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::NumberInputEvent(_, _, _, _, n::Event::StepRequested(request)) => Some(request),
            _ => None,
        })
        .collect()
}
fn resolve_request(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
    request: &n::StepRequest,
    value: Option<f64>,
) -> n::Response {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.numbers[&id(0)].command(
                &n::Command::ResolveStep {
                    request_id: request.id,
                    revision: request.snapshot.revision,
                    value: value.map(n::Value::Number),
                },
                window,
                cx,
            )
        })
    })
}
fn repeat_status(owner: &Entity<View>, cx: &VisualTestContext) -> (bool, bool) {
    owner.read_with(cx, |view, _| {
        let owner = view.numbers[&id(0)].owner.borrow();
        (owner.repeat.is_active(), owner.repeat.has_task())
    })
}
#[::core::prelude::v1::test]
fn application_steps_use_native_events_and_pause_held_repeat_until_resolution() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Application steps", 600., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                id(0),
                Kind::NumberInput,
                String::new(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetNumberInput(id(0), config(), n::Value::Number(12.)),
            Op::SetNumberStepMode(id(0), n::StepMode::Application),
            Op::SetStyle(
                id(0),
                vec![
                    WireStyle::Width(WireLength::Px(500.)),
                    WireStyle::Height(WireLength::Px(44.)),
                ],
            ),
            Op::SetRoot(Some(id(0))),
        ],
    );
    draw(cx);
    command(&owner, cx, n::Command::Focus);
    draw(cx);
    requests(&transport);
    cx.simulate_keystrokes("up");
    draw(cx);
    let pending = requests(&transport);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].source, n::Source::Keyboard);
    assert_eq!(pending[0].snapshot.draft, "12");
    cx.simulate_keystrokes("up");
    draw(cx);
    assert!(requests(&transport).is_empty());
    assert_eq!(command(&owner, cx, n::Command::ReadSnapshot).draft, "12");
    assert!(
        matches!(resolve_request(&owner,cx,&pending[0],Some(20.)),n::Response::Applied(s) if s.draft=="20")
    );
    draw(cx);
    // Explicit programmatic steps retain their immediate fixed-domain semantics.
    assert_eq!(
        command(&owner, cx, n::Command::Step(Direction::Increase)).draft,
        "21"
    );
    draw(cx);
    assert!(requests(&transport).is_empty());
    let tree = cx.a11y_tree().unwrap();
    let numeric = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == gpui::accesskit::Role::SpinButton && node.label() == Some("Amount")
        })
        .unwrap()
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Increment,
        target_node: numeric,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    draw(cx);
    let accessible = requests(&transport);
    assert_eq!(accessible.len(), 1);
    assert_eq!(accessible[0].source, n::Source::Accessibility);
    assert_eq!(accessible[0].snapshot.draft, "21");
    assert!(matches!(
        resolve_request(&owner, cx, &accessible[0], None),
        n::Response::Applied(_)
    ));
    let button = owner.read_with(cx, |view, _| {
        view.numbers[&id(0)].owner.borrow().repeat.buttons[1].center()
    });
    let press = |cx: &mut VisualTestContext| {
        cx.simulate_event(MouseDownEvent {
            position: button,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        draw(cx);
    };
    let release = |cx: &mut VisualTestContext| {
        cx.simulate_event(MouseUpEvent {
            position: button,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        draw(cx);
    };
    cx.simulate_event(MouseMoveEvent {
        position: button,
        pressed_button: None,
        modifiers: Default::default(),
    });
    draw(cx);
    press(cx);
    let pending = requests(&transport);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].source, n::Source::Stepper);
    assert_eq!(repeat_status(&owner, cx), (true, false));
    release(cx);
    // A second click cannot focus-cancel an already-issued pending intent.
    press(cx);
    release(cx);
    assert!(requests(&transport).is_empty());
    assert_eq!(repeat_status(&owner, cx), (false, false));
    assert!(
        matches!(resolve_request(&owner,cx,&pending[0],Some(30.)),n::Response::Applied(s) if s.draft=="30")
    );
    draw(cx);
    assert_eq!(repeat_status(&owner, cx), (false, false));
    press(cx);
    let pending = requests(&transport);
    assert_eq!(pending.len(), 1);
    assert_eq!(repeat_status(&owner, cx), (true, false));
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(3));
    cx.run_until_parked();
    draw(cx);
    assert!(requests(&transport).is_empty());
    assert_eq!(repeat_status(&owner, cx), (true, false));
    assert!(matches!(
        resolve_request(&owner, cx, &pending[0], Some(40.)),
        n::Response::Applied(_)
    ));
    draw(cx);
    assert_eq!(repeat_status(&owner, cx), (true, true));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(399));
    cx.run_until_parked();
    draw(cx);
    assert!(requests(&transport).is_empty());
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(1));
    cx.run_until_parked();
    draw(cx);
    let pending = requests(&transport);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].snapshot.draft, "40");
    assert_eq!(repeat_status(&owner, cx), (true, false));
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(5));
    cx.run_until_parked();
    draw(cx);
    assert!(requests(&transport).is_empty());
    assert!(matches!(
        resolve_request(&owner, cx, &pending[0], Some(50.)),
        n::Response::Applied(_)
    ));
    draw(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(75));
    cx.run_until_parked();
    draw(cx);
    let pending = requests(&transport);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].snapshot.draft, "50");
    assert!(matches!(
        resolve_request(&owner, cx, &pending[0], None),
        n::Response::Applied(_)
    ));
    draw(cx);
    assert_eq!(repeat_status(&owner, cx), (false, false));
    release(cx);
    // A mode change retires an issued request without replacing the native editor.
    cx.simulate_keystrokes("up");
    draw(cx);
    let pending = requests(&transport);
    assert_eq!(pending.len(), 1);
    let state = owner.read_with(cx, |view, _| view.numbers[&id(0)].state.entity_id());
    apply(
        &owner,
        cx,
        vec![Op::SetNumberStepMode(id(0), n::StepMode::Native)],
    );
    draw(cx);
    assert!(matches!(
        resolve_request(&owner, cx, &pending[0], Some(99.)),
        n::Response::Failed(n::Error::StaleRevision)
    ));
    assert_eq!(command(&owner, cx, n::Command::ReadSnapshot).draft, "50");
    assert_eq!(
        owner.read_with(cx, |view, _| view.numbers[&id(0)].state.entity_id()),
        state
    );
    // Pending user work is retired by editing, policy, visibility and activation
    // changes; delayed replies never replace the resulting native draft.
    for case in 0..5 {
        cx.update(|window, _| window.activate_window());
        apply(
            &owner,
            cx,
            vec![
                Op::SetNumberInput(id(0), config(), n::Value::Number(12.)),
                Op::SetNumberStepMode(id(0), n::StepMode::Application),
                Op::SetStyle(
                    id(0),
                    vec![
                        WireStyle::Width(WireLength::Px(500.)),
                        WireStyle::Height(WireLength::Px(44.)),
                    ],
                ),
            ],
        );
        draw(cx);
        command(
            &owner,
            cx,
            n::Command::ReplaceValue {
                value: n::Value::Number(50.),
                selection: n::SelectionPolicy::End,
                undo: n::UndoPolicy::Record,
                if_revision: None,
            },
        );
        command(&owner, cx, n::Command::Focus);
        draw(cx);
        requests(&transport);
        if case == 4 {
            press(cx);
            assert_eq!(repeat_status(&owner, cx), (true, false));
        } else {
            cx.simulate_keystrokes("up");
        }
        draw(cx);
        let pending = requests(&transport);
        assert_eq!(pending.len(), 1);
        match case {
            0 => {
                cx.simulate_keystrokes("1");
                draw(cx);
            }
            1 => {
                let mut policy = config();
                policy.read_only = true;
                apply(
                    &owner,
                    cx,
                    vec![Op::SetNumberInput(id(0), policy, n::Value::Number(12.))],
                );
                draw(cx);
            }
            2 => {
                apply(
                    &owner,
                    cx,
                    vec![Op::SetStyle(
                        id(0),
                        vec![WireStyle::Fields(vec![WireField::Display(3)])],
                    )],
                );
                draw(cx);
            }
            3 => {
                cx.deactivate_window();
                cx.run_until_parked();
                draw(cx);
            }
            4 => {
                let mut metadata = config();
                metadata.label = "Updated amount".into();
                apply(
                    &owner,
                    cx,
                    vec![Op::SetNumberInput(id(0), metadata, n::Value::Number(12.))],
                );
                draw(cx);
                assert_eq!(repeat_status(&owner, cx), (false, false));
                release(cx);
            }
            _ => unreachable!(),
        }
        if case == 3 {
            cx.simulate_keystrokes("up");
            draw(cx);
            assert!(
                requests(&transport).is_empty(),
                "inactive input must not issue a request"
            );
        }
        let before = command(&owner, cx, n::Command::ReadSnapshot);
        assert!(!owner.read_with(cx, |view, _| {
            view.numbers[&id(0)].owner.borrow().model.has_step_request()
        }));
        assert!(matches!(
            resolve_request(&owner, cx, &pending[0], Some(99.)),
            n::Response::Failed(_)
        ));
        assert_eq!(command(&owner, cx, n::Command::ReadSnapshot), before);
    }
    let weak = owner.read_with(cx, |view, _| Rc::downgrade(&view.numbers[&id(0)].owner));
    apply(&owner, cx, vec![Op::SetRoot(None), Op::Remove(id(0))]);
    draw(cx);
    assert!(
        weak.upgrade().is_none(),
        "request and timer must not retain a removed owner"
    );
}
