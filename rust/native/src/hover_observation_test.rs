use super::*;
use gpui::MouseMoveEvent;

#[test]
fn hover_link_loading_and_ancestor_gates_restore_without_action_rebinding() {
    hover_availability(Kind::Link);
}

#[test]
fn hover_command_availability_and_ancestor_gates_restore_without_action_rebinding() {
    hover_availability(Kind::CommandButton);
}

fn hover_availability(kind: Kind) {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    let action = HandlerId::from_parts(0, 1).unwrap();
    let observer = HandlerId::from_parts(1, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Hover policy", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let change = |cx: &mut VisualTestContext, operations| {
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| apply(view, window, cx, operations));
            window.draw(cx).clear(cx);
        });
    };
    let observations = || -> Vec<bool> {
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .into_iter()
            .filter_map(|event| match event {
                Event::HoverChanged(w, n, h, _, value) => {
                    assert_eq!((w, n, h), (wid, id(2), observer));
                    Some(value)
                }
                _ => None,
            })
            .collect()
    };
    let pointer = |cx: &mut VisualTestContext, x| {
        cx.simulate_event(MouseMoveEvent {
            position: gpui::point(px(x), px(40.)),
            pressed_button: None,
            modifiers: Default::default(),
        });
    };
    let availability = |available: bool| {
        if kind == Kind::Link {
            Op::SetLink(
                id(2),
                gpuio_protocol::link::Config {
                    label: "Guide".into(),
                    loading: !available,
                    disabled: false,
                    tab_stop: true,
                    tab_index: 0,
                },
            )
        } else {
            Op::SetCommands(
                id(0),
                vec![CommandConfig {
                    id: "run".into(),
                    generation: 1,
                    label: "Run".into(),
                    enabled: available,
                    checked: None,
                    shortcuts: vec![],
                    target: CommandTarget::Callback,
                }],
            )
        }
    };
    let mut setup = vec![
        Op::Create(id(0), Kind::CommandScope, "".into(), Some(action)),
        Op::SetCommands(id(0), vec![]),
        Op::Create(id(1), Kind::Container, "".into(), None),
        Op::SetStyle(id(1), vec![Style::Padding(20.)]),
        Op::Create(
            id(2),
            kind,
            String::new(),
            (kind == Kind::Link).then_some(action),
        ),
        Op::SetStyle(
            id(2),
            vec![
                Style::Width(Length::Px(100.)),
                Style::Height(Length::Px(40.)),
            ],
        ),
        Op::SetHoverObserver(id(2), Some(observer)),
        availability(true),
        Op::Splice(id(1), 0, 0, vec![id(2)]),
        Op::Splice(id(0), 0, 0, vec![id(1)]),
        Op::SetRoot(Some(id(0))),
    ];
    if kind == Kind::CommandButton {
        setup.push(Op::SetCommandRef(id(2), "run".into()));
    }
    change(cx, setup);
    assert_eq!(observations(), [false]);
    pointer(cx, 40.);
    assert_eq!(observations(), [true]);
    let focus = owner.read_with(cx, |view, _| view.buttons[&id(2)].focus.clone());
    change(cx, vec![availability(false)]);
    assert_eq!(observations(), [false]);
    change(cx, vec![availability(true)]);
    assert_eq!(observations(), [true]);
    assert_eq!(
        owner.read_with(cx, |view, _| view.buttons[&id(2)].focus.clone()),
        focus
    );
    for field in [
        Field::Disabled(true),
        Field::Inert(true),
        Field::Visibility(1),
        Field::Display(3),
    ] {
        change(
            cx,
            vec![Op::SetStyle(
                id(1),
                vec![Style::Padding(20.), Style::Fields(vec![field])],
            )],
        );
        assert_eq!(
            observations(),
            [false],
            "ancestor closes hover for {kind:?}"
        );
        change(cx, vec![Op::SetStyle(id(1), vec![Style::Padding(20.)])]);
        // A restored node may need a native hit test after being absent from paint.
        pointer(cx, 41.);
        assert_eq!(observations(), [true], "ancestor recovery for {kind:?}");
    }
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(observations(), [false], "keyboard modality ends hover");
    pointer(cx, 40.);
    assert_eq!(observations(), [true]);
    change(
        cx,
        vec![
            Op::Create(id(3), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                id(3),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(id(4), Kind::Button, "Modal action".into(), Some(action)),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(0), 1, 0, vec![id(3)]),
        ],
    );
    assert_eq!(observations(), [false], "modal gate closes outside hover");
    pointer(cx, 41.);
    assert!(observations().is_empty(), "outside pointer stays gated");
    change(
        cx,
        vec![
            Op::Splice(id(0), 1, 1, vec![]),
            Op::Remove(id(4)),
            Op::Remove(id(3)),
        ],
    );
    pointer(cx, 40.);
    assert_eq!(observations(), [true], "modal removal restores hover");
    cx.simulate_click(gpui::point(px(40.), px(40.)), Default::default());
    let events = transport.mailbox.lock().unwrap().drain(256);
    if kind == Kind::Link {
        assert_eq!(events.iter().filter(|e| matches!(e, Event::Press(w, n, h, _) if *w == wid && *n == id(2) && *h == action)).count(), 1);
    } else {
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::CommandInvoked(..)))
                .count(),
            1
        );
    }
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.session.borrow_mut().close(wid).unwrap();
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert!(owner.read_with(cx, |view, _| view.hover_observations.is_empty()));
}

#[test]
fn hover_subscription_follows_native_pointer_and_availability_without_replacing_actions() {
    hover_contract(false);
}
#[test]
fn hover_subscription_overflow_defers_fault_until_render_borrow_ends() {
    hover_contract(true);
}
fn hover_contract(overload: bool) {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    let click = HandlerId::from_parts(0, 1).unwrap();
    let observer = HandlerId::from_parts(1, 1).unwrap();
    let next = HandlerId::from_parts(1, 2).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Hover", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let change = |cx: &mut VisualTestContext, operations| {
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| apply(view, window, cx, operations));
            window.draw(cx).clear(cx);
        });
    };
    let observations = || -> Vec<(HandlerId, bool)> {
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .into_iter()
            .filter_map(|event| match event {
                Event::HoverChanged(_, node, handler, _, hovered) => {
                    assert_eq!(node, id(2));
                    Some((handler, hovered))
                }
                _ => None,
            })
            .collect()
    };
    let pointer = |cx: &mut VisualTestContext, x| {
        cx.simulate_event(MouseMoveEvent {
            position: gpui::point(px(x), px(40.)),
            pressed_button: None,
            modifiers: Default::default(),
        });
    };
    change(
        cx,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(id(0), vec![Style::Padding(20.)]),
            Op::Create(id(1), Kind::Text, "Rich label".into(), None),
            Op::Create(id(2), Kind::Button, "Action".into(), Some(click)),
            Op::SetStyle(
                id(2),
                vec![
                    Style::Width(Length::Px(100.)),
                    Style::Height(Length::Px(40.)),
                ],
            ),
            config(false, Focus::default()),
            Op::Splice(id(2), 0, 0, vec![id(1)]),
            Op::SetHoverObserver(id(2), Some(observer)),
            Op::Splice(id(0), 0, 0, vec![id(2)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    assert_eq!(observations(), [(observer, false)]);
    pointer(cx, 40.);
    assert_eq!(observations(), [(observer, true)]);
    pointer(cx, 41.);
    assert!(observations().is_empty());
    pointer(cx, 200.);
    assert_eq!(observations(), [(observer, false)]);
    pointer(cx, 40.);
    assert_eq!(observations(), [(observer, true)]);
    // Exercise the native-owner eviction path while the accepted tree/subscriber
    // and GPUI element identity survive. A stationary pointer must recover.
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.visited.remove(&id(2));
            view.retire_unvisited_hover(window, cx);
            view.buttons.remove(&id(2));
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        observations(),
        [(observer, false), (observer, true)],
        "cull/remount must recover stationary hover"
    );
    let focus = owner.read_with(cx, |view, _| view.buttons[&id(2)].focus.clone());
    change(cx, vec![config(true, Focus::default())]);
    assert_eq!(observations(), [(observer, false)]);
    change(cx, vec![config(false, Focus::default())]);
    assert_eq!(observations(), [(observer, true)]);
    assert_eq!(
        owner.read_with(cx, |view, _| view.buttons[&id(2)].focus.clone()),
        focus
    );
    change(cx, vec![Op::SetHoverObserver(id(2), None)]);
    assert!(observations().is_empty());
    pointer(cx, 200.);
    assert!(observations().is_empty());
    change(cx, vec![Op::SetHoverObserver(id(2), Some(next))]);
    assert_eq!(observations(), [(next, false)]);
    let revision = session.borrow().tree(wid).unwrap().revision();
    assert!(
        session
            .borrow()
            .hover_changed(wid, id(2), observer, revision, true)
            .is_none()
    );
    assert!(
        session
            .borrow()
            .press(wid, id(2), click, revision)
            .is_some()
    );
    cx.simulate_click(gpui::point(px(40.), px(40.)), Default::default());
    assert_eq!(transport.mailbox.lock().unwrap().drain(256).iter().filter(|event|
        matches!(event, Event::Press(_, node, handler, _) if *node == id(2) && *handler == click)).count(), 1);
    if overload {
        pointer(cx, 200.);
        observations();
        for index in 0..crate::mailbox::MAX_INPUT_EVENTS {
            pointer(cx, if index % 2 == 0 { 40. } else { 200. });
        }
        let replacement = HandlerId::from_parts(1, 3).unwrap();
        change(cx, vec![Op::SetHoverObserver(id(2), Some(replacement))]);
        let events = transport.mailbox.lock().unwrap().drain(256);
        assert_eq!(events.len(), crate::mailbox::MAX_INPUT_EVENTS + 1);
        for (index, event) in events[..crate::mailbox::MAX_INPUT_EVENTS]
            .iter()
            .enumerate()
        {
            assert!(matches!(event, Event::HoverChanged(w, n, h, _, value)
                if *w == wid && *n == id(2) && *h == next && *value == (index % 2 == 0)));
        }
        assert!(matches!(events.last(), Some(Event::Overloaded(w)) if *w == wid));
        let revision = session.borrow().tree(wid).unwrap().revision();
        assert!(
            session
                .borrow()
                .hover_changed(wid, id(2), replacement, revision, false)
                .is_none()
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(transport.mailbox.lock().unwrap().drain(256).is_empty());
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                view.session.borrow_mut().close(wid).unwrap();
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
    } else {
        change(
            cx,
            vec![
                Op::SetRoot(None),
                Op::Remove(id(1)),
                Op::Remove(id(2)),
                Op::Remove(id(0)),
            ],
        );
        assert!(
            observations().is_empty(),
            "removed subscriptions have no final callback"
        );
        session.borrow_mut().close(wid).unwrap();
    }
    assert!(owner.read_with(cx, |view, _| view.buttons.is_empty()
        && view.hover_observations.is_empty()));
}
