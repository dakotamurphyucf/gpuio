//! Production retained content on TestPlatform; no OS window or OCaml renderer.
use super::presentation_tests::{apply, command, config, date, node, read};
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, accesskit};
use gpuio_protocol::v1::{Length, Style};
use gpuio_protocol::{
    calendar_content::{Config, Item},
    calendar_presentation::Appearance,
    v1::*,
};
use std::{cell::RefCell, os::fd::AsRawFd, os::unix::net::UnixStream, rc::Rc};
fn id(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}

#[::core::prelude::v1::test]
fn rich_day_and_headers_render_without_replacing_calendar_focus_or_selection() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let wid = WindowId::from_parts(0, 1).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Rich calendar", 1000., 700.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    let feb = c::Month::new(2024, 2).unwrap();
    let slots = [
        (Slot::Next, "Next artwork"),
        (Slot::Day(date(2, 29).ordinal()), "Two events"),
        (Slot::MonthHeading(feb.index()), "Month artwork"),
        (Slot::Weekday(feb.index(), 4), "Weekday artwork"),
    ];
    let mut ops = vec![
        Op::Create(
            node(),
            Kind::Calendar,
            "".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetCalendar(node(), Box::new(config()), c::Selection::Empty, feb),
        Op::SetCalendarAppearance(
            node(),
            Some(Appearance {
                months: 2,
                ..Appearance::default()
            }),
        ),
        Op::SetCalendarContent(
            node(),
            Some(Config {
                items: slots
                    .iter()
                    .map(|(slot, text)| Item {
                        slot: *slot,
                        description: Some((*text).into()),
                    })
                    .collect(),
            }),
        ),
        Op::SetRoot(Some(node())),
    ];
    for (index, (_, text)) in slots.iter().enumerate() {
        let wrapper = id(index as i64 * 2 + 1);
        let child = id(index as i64 * 2 + 2);
        ops.extend([
            Op::Create(wrapper, Kind::Container, "".into(), None),
            Op::Create(child, Kind::Text, (*text).into(), None),
            Op::SetStyle(
                child,
                vec![
                    Style::Width(Length::Px(50.)),
                    Style::Height(Length::Px(12.)),
                ],
            ),
            Op::Splice(wrapper, 0, 0, vec![child]),
        ]);
    }
    ops.push(Op::Splice(
        node(),
        0,
        0,
        (0..slots.len()).map(|i| id(i as i64 * 2 + 1)).collect(),
    ));
    apply(&owner, cx, ops);
    let entity = read(&owner, cx).2;
    let tree = cx.a11y_tree().unwrap();
    let day = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("February 29, 2024, Today"))
        .unwrap();
    assert_eq!(day.1.description(), Some("Two events"));
    let day_id = day.0;
    for label in [
        "Next artwork",
        "Two events",
        "Month artwork",
        "Weekday artwork",
    ] {
        let matching: Vec<_> = tree
            .nodes
            .iter()
            .filter(|(_, n)| n.label() == Some(label))
            .collect();
        assert_eq!(matching.len(), 1, "slot must render exactly once: {label}");
        let mut cursor = matching[0].0;
        loop {
            let node = &tree.nodes.iter().find(|(id, _)| *id == cursor).unwrap().1;
            if node.is_hidden() {
                break;
            }
            cursor = tree
                .nodes
                .iter()
                .find(|(_, parent)| parent.children().contains(&cursor))
                .expect("decorative content must have a hidden AX ancestor")
                .0;
        }
    }
    assert!(tree.nodes.iter().any(
        |(_, n)| n.label() == Some("February 2024") && n.description() == Some("Month artwork")
    ));
    assert!(
        tree.nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Thursday")
                && n.description() == Some("Weekday artwork"))
    );
    for index in 0..slots.len() {
        owner.read_with(cx, |v, _| {
            assert!(
                v.probes.borrow()[&id(index as i64 * 2 + 2)]
                    .bounds
                    .size
                    .height
                    > px(0.)
            )
        });
    }
    command(&owner, cx, c::Command::FocusDate(date(2, 29)));
    cx.simulate_keystrokes("enter");
    apply(&owner, cx, vec![]);
    let before = read(&owner, cx).0;
    assert_eq!(before.selection, c::Selection::RangeStart(date(2, 29)));
    apply(
        &owner,
        cx,
        vec![
            Op::SetText(id(4), "Updated events".into()),
            Op::SetStyle(id(4), vec![Style::Height(Length::Px(20.))]),
        ],
    );
    assert_eq!(read(&owner, cx).0, before);
    assert_eq!(read(&owner, cx).2, entity);
    owner.read_with(cx, |v, _| {
        assert_eq!(v.probes.borrow()[&id(4)].bounds.size.height, px(20.))
    });
    assert_eq!(cx.a11y_tree().unwrap().focus, day_id);
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_node: day_id,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert_eq!(
        read(&owner, cx).0.selection,
        c::Selection::Range(c::Range::new(date(2, 29), date(2, 29)).unwrap())
    );
    command(
        &owner,
        cx,
        c::Command::SetPresentation(c::Presentation::Months),
    );
    apply(&owner, cx, vec![]);
    assert_eq!(read(&owner, cx).0.presentation, c::Presentation::Months);
    let hidden_tree = cx.a11y_tree().unwrap();
    assert!(
        !hidden_tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Updated events"))
    );
    command(
        &owner,
        cx,
        c::Command::SetPresentation(c::Presentation::Days),
    );
    apply(&owner, cx, vec![]);
    assert_eq!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("February 29, 2024, Today"))
            .unwrap()
            .0,
        day_id
    );
    let point = owner.read_with(cx, |v, _| v.probes.borrow()[&id(4)].bounds.center());
    cx.simulate_click(point, gpui::Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        read(&owner, cx).0.selection,
        c::Selection::RangeStart(date(2, 29))
    );
    let mut readonly = config();
    readonly.read_only = true;
    apply(
        &owner,
        cx,
        vec![Op::SetCalendar(
            node(),
            Box::new(readonly),
            c::Selection::Empty,
            feb,
        )],
    );
    let before = read(&owner, cx).0;
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_node: day_id,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert_eq!(
        read(&owner, cx).0,
        before,
        "rich content must not bypass read-only selection"
    );
    let mut reset = vec![
        Op::SetCalendarContent(node(), None),
        Op::Splice(node(), 0, 4, vec![]),
    ];
    for n in (1..=8).rev() {
        reset.push(Op::Remove(id(n)));
    }
    apply(&owner, cx, reset);
    assert_eq!(read(&owner, cx).0, before);
    assert_eq!(read(&owner, cx).2, entity);
    let fallback = cx.a11y_tree().unwrap();
    assert_eq!(
        fallback
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("February 29, 2024, Today"))
            .unwrap()
            .0,
        day_id
    );
    assert_eq!(
        fallback
            .nodes
            .iter()
            .find(|(id, _)| *id == day_id)
            .unwrap()
            .1
            .description(),
        None
    );
    let weak = owner.read_with(cx, |v, _| v.calendars[&node()].state.downgrade());
    let mut close = vec![Op::SetRoot(None)];
    close.push(Op::Remove(node()));
    apply(&owner, cx, close);
    assert!(weak.upgrade().is_none());
}

#[::core::prelude::v1::test]
fn offscreen_slot_stops_animation_and_removal_releases_its_owner() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let wid = WindowId::from_parts(0, 1).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Calendar progress", 1000., 700.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session.clone(), transport));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                node(),
                Kind::Calendar,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetCalendar(
                node(),
                Box::new(config()),
                c::Selection::Empty,
                c::Month::new(2024, 2).unwrap(),
            ),
            Op::SetCalendarContent(
                node(),
                Some(Config {
                    items: vec![Item {
                        slot: Slot::Day(date(2, 29).ordinal()),
                        description: Some("Loading events".into()),
                    }],
                }),
            ),
            Op::Create(id(1), Kind::Container, "".into(), None),
            Op::Create(id(2), Kind::Progress, "".into(), None),
            Op::SetProgress(
                id(2),
                ProgressConfig {
                    label: "Loading events".into(),
                    fraction: None,
                },
            ),
            Op::SetStyle(
                id(2),
                vec![Style::Width(Length::Px(20.)), Style::Height(Length::Px(6.))],
            ),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(node(), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(node())),
        ],
    );
    let probe = owner.read_with(cx, |v, _| Rc::downgrade(&v.progress_probes[&id(2)]));
    assert!(probe.upgrade().unwrap().get().count > 0);
    for color in [0xff0000ff, 0x00ff00ff] {
        apply(
            &owner,
            cx,
            vec![Op::SetStyle(
                id(2),
                vec![
                    Style::Width(Length::Px(20.)),
                    Style::Height(Length::Px(6.)),
                    Style::Foreground(Color::Rgba(color)),
                ],
            )],
        );
        assert_eq!(
            probe.upgrade().unwrap().get().color,
            gpui::Hsla::from(rgba(color as u32))
        );
    }
    cx.update(|window, cx| assert!(window.simulate_next_frame(cx) > 0));
    command(
        &owner,
        cx,
        c::Command::SetPresentation(c::Presentation::Months),
    );
    apply(&owner, cx, vec![]);
    let hidden_count = probe.upgrade().unwrap().get().count;
    for _ in 0..3 {
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    assert_eq!(probe.upgrade().unwrap().get().count, hidden_count);
    cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
    command(
        &owner,
        cx,
        c::Command::SetPresentation(c::Presentation::Days),
    );
    apply(&owner, cx, vec![]);
    assert!(probe.upgrade().unwrap().get().count > hidden_count);
    owner.read_with(cx, |v, _| {
        assert!(Rc::ptr_eq(
            &probe.upgrade().unwrap(),
            &v.progress_probes[&id(2)]
        ))
    });
    apply(
        &owner,
        cx,
        vec![
            Op::SetCalendarContent(node(), None),
            Op::Splice(node(), 0, 1, vec![]),
            Op::Remove(id(2)),
            Op::Remove(id(1)),
        ],
    );
    assert!(probe.upgrade().is_none());
    owner.read_with(cx, |v, _| assert!(!v.progresses.contains_key(&id(2))));
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
}
