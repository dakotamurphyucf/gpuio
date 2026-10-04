//! Logical viewport observations through production native owners on TestPlatform.
use super::presentation_tests::{apply, command, config, node, read};
use super::*;
use crate::session::Session;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::v1::Style;
use gpuio_protocol::{calendar_presentation::Appearance, v1::*};
use std::{cell::RefCell, os::fd::AsRawFd, os::unix::net::UnixStream, rc::Rc};

fn drain(transport: &Transport) -> Vec<Event> {
    transport.mailbox.lock().unwrap().drain(128)
}
fn observations(events: Vec<Event>) -> Vec<(HandlerId, Observation)> {
    events
        .into_iter()
        .filter_map(|e| match e {
            Event::CalendarViewportChanged(_, _, handler, _, sample) => Some((handler, sample)),
            _ => None,
        })
        .collect()
}
fn mount(owner: &Entity<View>, cx: &mut VisualTestContext, observer: Option<HandlerId>) {
    apply(
        owner,
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
            Op::SetCalendarAppearance(
                node(),
                Some(Appearance {
                    months: 2,
                    ..Default::default()
                }),
            ),
            Op::SetCalendarViewportObserver(node(), observer),
            Op::SetRoot(Some(node())),
        ],
    );
}

#[::core::prelude::v1::test]
fn logical_panes_are_observed_independently_of_cursor_selection_and_paint() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let wid = WindowId::from_parts(0, 1).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Viewport", 1000., 700.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let handler = HandlerId::from_parts(1, 1).unwrap();
    mount(&owner, cx, Some(handler));
    let expected = Display::Days {
        first_month: c::Month::new(2024, 2).unwrap().index(),
        months: 2,
        first_weekday: 1,
    };
    let initial: Vec<_> = drain(&transport)
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                Event::CalendarEvent(..) | Event::CalendarViewportChanged(..)
            )
        })
        .collect();
    assert!(matches!(
        initial.as_slice(),
        [
            Event::CalendarEvent(_, _, _, _, c::Event::Observed(_)),
            Event::CalendarViewportChanged(..)
        ]
    ));
    assert_eq!(
        observations(initial),
        vec![(
            handler,
            Observation {
                sequence: 0,
                display: expected
            }
        )]
    );
    let identity = read(&owner, cx).2;
    // Native cursor walks into the second pane without changing the visible panes.
    cx.update(|_, cx| {
        owner.update(cx, |v, cx| {
            v.calendars[&node()]
                .state
                .update(cx, |s, cx| s.act(Action::MoveDays(31), cx))
        })
    });
    assert_eq!(read(&owner, cx).0.month, c::Month::new(2024, 3).unwrap());
    assert!(observations(drain(&transport)).is_empty());
    let before = read(&owner, cx).0;
    apply(
        &owner,
        cx,
        vec![Op::SetCalendarAppearance(
            node(),
            Some(Appearance {
                months: 3,
                ..Default::default()
            }),
        )],
    );
    assert_eq!(read(&owner, cx).0.revision, before.revision);
    assert_eq!(
        observations(drain(&transport)),
        vec![(
            handler,
            Observation {
                sequence: 1,
                display: Display::Days {
                    first_month: c::Month::new(2024, 2).unwrap().index(),
                    months: 3,
                    first_weekday: 1
                }
            }
        )]
    );
    let mut sunday = config();
    sunday.first_weekday = 0;
    apply(
        &owner,
        cx,
        vec![Op::SetCalendar(
            node(),
            Box::new(sunday),
            c::Selection::Empty,
            c::Month::new(1, 1).unwrap(),
        )],
    );
    assert!(matches!(
        observations(drain(&transport)).as_slice(),
        [(
            _,
            Observation {
                sequence: 2,
                display: Display::Days {
                    first_weekday: 0,
                    ..
                }
            }
        )]
    ));
    command(
        &owner,
        cx,
        c::Command::SetPresentation(c::Presentation::Months),
    );
    assert_eq!(
        observations(drain(&transport)),
        vec![(
            handler,
            Observation {
                sequence: 3,
                display: Display::Months { year: 2024 }
            }
        )]
    );
    command(
        &owner,
        cx,
        c::Command::SetPresentation(c::Presentation::Years),
    );
    assert_eq!(
        observations(drain(&transport)),
        vec![(
            handler,
            Observation {
                sequence: 4,
                display: Display::Years {
                    first: 2021,
                    last: 2040
                }
            }
        )]
    );
    command(
        &owner,
        cx,
        c::Command::SetPresentation(c::Presentation::Days),
    );
    drain(&transport);
    // A completed range remains an atomic pair before its resulting viewport.
    cx.update(|_, cx| {
        owner.update(cx, |v, cx| {
            v.calendars[&node()].state.update(cx, |s, cx| {
                s.act(
                    Action::Activate(c::Date::from_ymd(2024, 2, 28).unwrap()),
                    cx,
                );
            })
        })
    });
    drain(&transport);
    cx.update(|_, cx| {
        owner.update(cx, |v, cx| {
            v.calendars[&node()].state.update(cx, |s, cx| {
                s.act(Action::Activate(c::Date::from_ymd(2024, 6, 1).unwrap()), cx);
            })
        })
    });
    assert!(matches!(
        drain(&transport).as_slice(),
        [
            Event::CalendarEvent(_, _, _, _, c::Event::Changed(_)),
            Event::CalendarEvent(_, _, _, _, c::Event::Selected(_)),
            Event::CalendarViewportChanged(..)
        ]
    ));
    // Hidden mounted owners retain logical panes and remain commandable.
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    assert!(observations(drain(&transport)).is_empty());
    command(
        &owner,
        cx,
        c::Command::ShowMonth(c::Month::new(9999, 12).unwrap()),
    );
    assert!(matches!(
        observations(drain(&transport)).as_slice(),
        [(
            _,
            Observation {
                display: Display::Days {
                    first_month: 119985,
                    months: 3,
                    ..
                },
                ..
            }
        )]
    ));
    for _ in 0..3 {
        apply(&owner, cx, vec![]);
        command(&owner, cx, c::Command::ReadSnapshot);
    }
    assert!(observations(drain(&transport)).is_empty());
    assert_eq!(read(&owner, cx).2, identity);
}

#[::core::prelude::v1::test]
fn subscriptions_retain_native_owner_and_retire_on_disable_unmount_and_overload() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let wid = WindowId::from_parts(0, 1).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Viewport lifetime", 1000., 700.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    mount(&owner, cx, None);
    assert!(observations(drain(&transport)).is_empty());
    let identity = read(&owner, cx).2;
    let first = HandlerId::from_parts(1, 1).unwrap();
    let next = HandlerId::from_parts(1, 2).unwrap();
    apply(
        &owner,
        cx,
        vec![Op::SetCalendarViewportObserver(node(), Some(first))],
    );
    assert_eq!(observations(drain(&transport))[0].1.sequence, 0);
    apply(
        &owner,
        cx,
        vec![Op::SetCalendarViewportObserver(node(), None)],
    );
    command(
        &owner,
        cx,
        c::Command::ShowMonth(c::Month::new(2025, 1).unwrap()),
    );
    assert!(observations(drain(&transport)).is_empty());
    apply(
        &owner,
        cx,
        vec![Op::SetCalendarViewportObserver(node(), Some(next))],
    );
    let event = observations(drain(&transport));
    assert_eq!(event.len(), 1);
    assert_eq!(event[0].0, next);
    assert_eq!(event[0].1.sequence, 1);
    assert_eq!(read(&owner, cx).2, identity);
    let revision = session.borrow().tree(wid).unwrap().revision();
    assert!(
        session
            .borrow()
            .calendar_viewport_event(wid, node(), first, revision, event[0].1)
            .is_none()
    );
    assert!(
        session
            .borrow()
            .calendar_viewport_event(wid, node(), next, revision + 1, event[0].1)
            .is_none()
    );
    // Keep a stale entity alive: its route must not emit after removal.
    let stale = owner.read_with(cx, |v, _| v.calendars[&node()].state.clone());
    apply(&owner, cx, vec![Op::SetRoot(None), Op::Remove(node())]);
    drain(&transport);
    cx.update(|_, cx| {
        stale.update(cx, |s, _| {
            s.viewport_published = None;
            assert!(!s.publish_viewport());
        })
    });
    assert!(observations(drain(&transport)).is_empty());
    assert!(owner.read_with(cx, |v, _| v.calendars.is_empty()));
    // A new owner starts a fresh sequence. Overflow is explicit, never wraps.
    let replacement = NodeId::from_parts(0, 2).unwrap();
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                replacement,
                Kind::Calendar,
                "".into(),
                Some(HandlerId::from_parts(0, 2).unwrap()),
            ),
            Op::SetCalendar(
                replacement,
                Box::new(config()),
                c::Selection::Empty,
                c::Month::new(2024, 2).unwrap(),
            ),
            Op::SetCalendarViewportObserver(replacement, Some(next)),
            Op::SetRoot(Some(replacement)),
        ],
    );
    assert_eq!(observations(drain(&transport))[0].1.sequence, 0);
    cx.update(|_, cx| {
        owner.update(cx, |v, cx| {
            v.calendars[&replacement].state.update(cx, |s, _| {
                s.viewport_published = None;
                s.viewport_sequence = None;
                assert!(!s.publish_viewport());
            })
        })
    });
    assert!(!session.borrow().accepts_input(wid));
}
