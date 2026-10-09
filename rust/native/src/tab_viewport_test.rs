//! Production GPUI layout and input on TestPlatform; not OS desktop qualification.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{
    HandlerId,
    tab_viewport::{Config, Reveal},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn n(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn wid() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn choices() -> ChoiceConfig {
    ChoiceConfig {
        label: "Scrollable tabs".into(),
        items: (0..8)
            .map(|i| ChoiceItem {
                id: format!("tab-{i}"),
                label: format!("Workspace {i}"),
                disabled: i == 6,
            })
            .collect(),
        selected: Some("tab-0".into()),
        disabled: false,
    }
}
fn dimensions(width: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(42.)),
    ])]
}
fn initial(config: ChoiceConfig) -> Vec<Op> {
    vec![
        Op::Create(
            n(0),
            Kind::TabBar,
            String::new(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetChoice(n(0), config),
        Op::SetTabViewport(n(0), Some(Config::default())),
        Op::SetTabAppearance(
            n(0),
            Some(gpuio_protocol::tab_appearance::Config {
                tab_style: vec![Style::Fields(vec![Field::Width(Length::Px(100.))])],
                ..Default::default()
            }),
        ),
        Op::SetStyle(n(0), dimensions(240.)),
        Op::SetRoot(Some(n(0))),
    ]
}
fn settle(cx: &mut VisualTestContext) -> usize {
    cx.run_until_parked();
    let frames = cx.update(|window, cx| {
        let count = window.simulate_next_frame(cx);
        if count > 0 {
            window.draw(cx).clear(cx);
        }
        count
    });
    cx.run_until_parked();
    assert!(frames <= 1, "reveal needs at most one follow-up frame");
    assert_eq!(
        cx.update(|window, cx| window.simulate_next_frame(cx)),
        0,
        "settled reveal must stay idle"
    );
    frames
}
fn draw_and_settle(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    settle(cx);
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, ops: Vec<Op>) -> usize {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let base = view.session.borrow().tree(wid()).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: wid(),
                    base,
                    revision: base + 1,
                    operations: ops,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    settle(cx)
}
fn reveal(serial: i64, target: &str) -> Op {
    Op::SetTabViewport(
        n(0),
        Some(Config {
            reveal: Some(Reveal {
                serial,
                target: target.into(),
            }),
        }),
    )
}
fn offset(owner: &Entity<View>, cx: &mut VisualTestContext) -> f32 {
    owner.read_with(cx, |v, _| v.scrolls[&n(0)].handle.offset().x.into())
}
fn visible(cx: &mut VisualTestContext, label: &str) {
    let nodes = cx.a11y_tree().unwrap().nodes;
    let bounds = |label| {
        nodes
            .iter()
            .find(|(_, n)| n.label() == Some(label))
            .unwrap()
            .1
            .bounds()
            .unwrap()
    };
    let outer = bounds("Scrollable tabs");
    let tab = bounds(label);
    assert!(
        tab.x0 >= outer.x0 - 0.1 && tab.x1 <= outer.x1 + 0.1,
        "{label}: {tab:?} outside {outer:?}"
    );
}
fn wheel(cx: &mut VisualTestContext, delta: f32) {
    let position = gpui::point(px(50.), px(15.));
    cx.simulate_mouse_move(position, None, Default::default());
    cx.update(|window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position,
                delta: gpui::ScrollDelta::Pixels(gpui::point(px(delta), px(0.))),
                touch_phase: gpui::TouchPhase::Started,
                modifiers: Default::default(),
            }),
            cx,
        );
    });
    draw_and_settle(cx);
}
#[test]
fn native_tabs_reveal_by_identity_once_and_keyboard_navigation_keeps_active_visible() {
    let mut app = TestAppContext::single();
    app.update(crate::image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid(), "Viewport", 500., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|w, _| w.activate_window());
    cx.run_until_parked();
    let mut config = choices();
    apply(&owner, cx, initial(config.clone()));
    assert_eq!(offset(&owner, cx), 0.);
    visible(cx, "Workspace 0");
    assert!(owner.read_with(cx, |v, _| v.scrolls[&n(0)].handle.max_offset().x) > px(500.));
    let focus = owner.read_with(cx, |v, _| v.buttons[&n(0)].focus.clone());
    let viewport_owner = owner.read_with(cx, |v, _| Rc::downgrade(&v.tab_viewports[&n(0)]));
    // Controlled selection alone is not an imperative scroll request.
    config.selected = Some("tab-7".into());
    apply(&owner, cx, vec![Op::SetChoice(n(0), config.clone())]);
    assert_eq!(offset(&owner, cx), 0.);
    transport.mailbox.lock().unwrap().drain(100);
    assert_eq!(apply(&owner, cx, vec![reveal(1, "tab-7")]), 1);
    visible(cx, "Workspace 7");
    let right = offset(&owner, cx);
    assert!(right < -500.);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .any(|e| matches!(e, Event::Choice(..)))
    );
    // Dynamic root presentation cannot take over the viewport axes or wrapping.
    let mut hover = dimensions(240.);
    hover.push(Style::State(
        2,
        vec![
            Field::Direction(1),
            Field::Wrap(1),
            Field::OverflowX(1),
            Field::OverflowY(3),
            Field::Background(Fill::Solid(Color::Rgba(0x22cc44ff))),
        ],
    ));
    apply(&owner, cx, vec![Op::SetStyle(n(0), hover)]);
    // Native wheel motion wins over a retained or reused command.
    wheel(cx, 80.);
    let user_offset = offset(&owner, cx);
    assert!(user_offset > right);
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let green: gpui::Background = gpui::rgba(0x22cc44ff).into();
        assert!(window.painted_quads().iter().any(|q| q.background == green));
    });
    apply(&owner, cx, vec![reveal(1, "tab-0")]);
    assert_eq!(offset(&owner, cx), user_offset);
    apply(&owner, cx, vec![Op::SetChoice(n(0), config.clone())]);
    assert_eq!(offset(&owner, cx), user_offset);
    apply(&owner, cx, vec![reveal(2, "tab-0")]);
    visible(cx, "Workspace 0");
    assert_eq!(offset(&owner, cx), 0.);
    // A pending operation consumes a removed ID rather than targeting a new index.
    apply(&owner, cx, vec![reveal(3, "missing")]);
    assert_eq!(offset(&owner, cx), 0.);
    apply(&owner, cx, vec![reveal(2, "tab-7")]);
    assert_eq!(offset(&owner, cx), 0.);
    // Keyboard navigation skips disabled 6 and reveals immediate active 7 without
    // waiting for application selection to echo back.
    config.selected = Some("tab-0".into());
    apply(&owner, cx, vec![Op::SetChoice(n(0), config.clone())]);
    cx.update(|w, cx| w.focus(&focus, cx));
    transport.mailbox.lock().unwrap().drain(100);
    cx.simulate_keystrokes("end");
    draw_and_settle(cx);
    visible(cx, "Workspace 7");
    let events = transport.mailbox.lock().unwrap().drain(100);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e,Event::Choice(_,_,_,_,s)if s=="tab-7"))
            .count(),
        1
    );
    assert_eq!(
        session
            .borrow()
            .tree(wid())
            .unwrap()
            .get(n(0))
            .unwrap()
            .choice
            .as_ref()
            .unwrap()
            .selected
            .as_deref(),
        Some("tab-0")
    );
    cx.simulate_keystrokes("left");
    draw_and_settle(cx);
    visible(cx, "Workspace 5");
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .filter(|e| matches!(e,Event::Choice(_,_,_,_,s)if s=="tab-5"))
            .count(),
        1
    );
    let tab = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, n)| n.label() == Some("Workspace 0"))
        .unwrap()
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Focus,
        target_node: tab,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw_and_settle(cx);
    visible(cx, "Workspace 0");
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .any(|e| matches!(e, Event::Choice(..)))
    );
    // Reorder plus explicit reveal uses the new same-transaction layout.
    config.items.reverse();
    apply(
        &owner,
        cx,
        vec![Op::SetChoice(n(0), config.clone()), reveal(4, "tab-7")],
    );
    visible(cx, "Workspace 7");
    assert_eq!(offset(&owner, cx), 0.);
    apply(
        &owner,
        cx,
        vec![reveal(5, "tab-0"), Op::SetStyle(n(0), dimensions(170.))],
    );
    visible(cx, "Workspace 0");
    // Hidden retained panels keep serial history and don't wake idle frames.
    let mut hidden = dimensions(170.);
    hidden.push(Style::Fields(vec![Field::Display(3)]));
    let before_hidden = offset(&owner, cx);
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(n(0), hidden), reveal(6, "tab-7")],
    );
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
        assert_eq!(window.simulate_next_frame(cx), 0);
    });
    apply(
        &owner,
        cx,
        vec![
            Op::SetStyle(n(0), dimensions(170.)),
            Op::SetTabViewport(n(0), Some(Config::default())),
        ],
    );
    assert_eq!(
        offset(&owner, cx),
        before_hidden,
        "cancelled hidden reveal must preserve the earlier offset"
    );
    apply(&owner, cx, vec![reveal(7, "tab-7")]);
    visible(cx, "Workspace 7");
    assert_eq!(offset(&owner, cx), 0.);
    // A zero-width viewport waits for layout rather than consuming a reveal or polling.
    assert_eq!(
        apply(
            &owner,
            cx,
            vec![Op::SetStyle(n(0), dimensions(0.)), reveal(8, "tab-0")]
        ),
        0
    );
    apply(&owner, cx, vec![Op::SetStyle(n(0), dimensions(170.))]);
    visible(cx, "Workspace 0");
    // Unselectable tabs can still be explicitly revealed, e.g. to reach Close.
    apply(&owner, cx, vec![reveal(9, "tab-6")]);
    visible(cx, "Workspace 6");
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .any(|e| matches!(e, Event::Choice(..)))
    );
    assert!(viewport_owner.upgrade().is_some());
    apply(&owner, cx, vec![Op::SetTabViewport(n(0), None)]);
    assert!(viewport_owner.upgrade().is_none());
    owner.read_with(cx, |v, _| {
        assert!(v.tab_viewports.is_empty());
        assert!(v.scrolls.is_empty());
        assert_eq!(v.buttons[&n(0)].focus, focus);
    });
    apply(&owner, cx, vec![reveal(1, "tab-0")]);
    visible(cx, "Workspace 0");
    assert!(offset(&owner, cx) < -500.);
    apply(&owner, cx, vec![Op::SetRoot(None), Op::Remove(n(0))]);
    assert_eq!(session.borrow().retained_bytes(), 0);
    owner.read_with(cx, |v, _| {
        assert!(v.tab_viewports.is_empty());
        assert!(v.scrolls.is_empty());
    });
}

#[test]
fn offscreen_tab_editor_keeps_its_owner_and_native_focus_reveals_without_selecting() {
    let mut app = TestAppContext::single();
    app.update(crate::image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid(), "Parts viewport", 500., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|w, _| w.activate_window());
    cx.run_until_parked();
    let mut ops = initial(choices());
    for i in 1..=32 {
        ops.push(Op::Create(n(i), Kind::Container, String::new(), None));
    }
    ops.push(Op::Create(
        n(33),
        Kind::Input,
        "Draft".into(),
        Some(HandlerId::from_parts(1, 1).unwrap()),
    ));
    for i in 0..8 {
        let base = 1 + 4 * i;
        ops.push(Op::Splice(
            n(base),
            0,
            0,
            vec![n(base + 1), n(base + 2), n(base + 3)],
        ));
    }
    ops.extend([
        Op::SetEditor(
            n(33),
            EditorConfig {
                label: "Tab draft".into(),
                placeholder: String::new(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::SetStyle(
            n(33),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(70.)),
                Field::Height(Length::Px(24.)),
            ])],
        ),
        Op::Splice(n(32), 0, 0, vec![n(33)]),
        Op::Splice(n(0), 0, 0, (0..8).map(|i| n(1 + 4 * i)).collect()),
        Op::SetTabContent(
            n(0),
            Some(gpuio_protocol::tab_content::Config {
                max_width: None,
                labels: (0..8)
                    .map(|i| {
                        if i == 7 {
                            gpuio_protocol::tab_content::Label::Hidden
                        } else {
                            gpuio_protocol::tab_content::Label::Default
                        }
                    })
                    .collect(),
            }),
        ),
    ]);
    apply(&owner, cx, ops);
    let root = owner.read_with(cx, |v, _| v.buttons[&n(0)].focus.clone());
    cx.update(|window, cx| window.focus(&root, cx));
    draw_and_settle(cx);
    transport.mailbox.lock().unwrap().drain(100);
    cx.simulate_keystrokes("tab");
    draw_and_settle(cx);
    // Generic focus reveal is deferred until all children/popups finish paint.
    draw_and_settle(cx);
    assert!(offset(&owner, cx) < -400.);
    visible(cx, "Tab draft");
    cx.update(|window, cx| assert!(owner.read(cx).editors[&n(33)].snapshot(window, cx).focused));
    cx.simulate_keystrokes("left");
    cx.simulate_keystrokes("x");
    draw_and_settle(cx);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .any(|e| matches!(e, Event::Choice(..)))
    );
    let draft = cx.update(|window, cx| owner.read(cx).editors[&n(33)].snapshot(window, cx).text);
    assert!(draft.contains('x'));
    let mut config = choices();
    config.items.reverse();
    let mut modes = vec![gpuio_protocol::tab_content::Label::Default; 8];
    modes[0] = gpuio_protocol::tab_content::Label::Hidden;
    apply(
        &owner,
        cx,
        vec![
            Op::SetChoice(n(0), config),
            Op::Splice(n(0), 0, 8, (0..8).rev().map(|i| n(1 + 4 * i)).collect()),
            Op::SetTabContent(
                n(0),
                Some(gpuio_protocol::tab_content::Config {
                    max_width: None,
                    labels: modes,
                }),
            ),
            reveal(1, "tab-7"),
        ],
    );
    visible(cx, "Tab draft");
    cx.update(|window, cx| {
        let snapshot = owner.read(cx).editors[&n(33)].snapshot(window, cx);
        assert_eq!(snapshot.text, draft);
        assert!(snapshot.focused);
    });
    // A wheel shift does not get undone by the unchanged child focus or old reveal.
    wheel(cx, -80.);
    let moved = offset(&owner, cx);
    assert!(moved < 0.);
    apply(&owner, cx, vec![Op::SetText(n(0), "Updated root".into())]);
    assert_eq!(offset(&owner, cx), moved);
    let mut cleanup = vec![Op::SetRoot(None), Op::Remove(n(33))];
    for base in (0..8).map(|i| 1 + 4 * i) {
        for i in [base + 1, base + 2, base + 3, base] {
            cleanup.push(Op::Remove(n(i)));
        }
    }
    cleanup.push(Op::Remove(n(0)));
    apply(&owner, cx, cleanup);
    assert_eq!(session.borrow().retained_bytes(), 0);
    owner.read_with(cx, |v, _| {
        assert!(v.editors.is_empty() && v.tab_viewports.is_empty() && v.scrolls.is_empty())
    });
}
