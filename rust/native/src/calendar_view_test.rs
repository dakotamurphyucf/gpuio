//! Mounted GPUI calendar input and lifecycle checks; no OCaml test double.
use super::super::{
    editor_test::{frame, key},
    native_test::{mouse, move_mouse},
    stop_application,
};
use super::*;
use crate::session::Session;
use gpuio_protocol::v1::{
    CAPABILITIES, Color, Event, Field, Fill, Kind, Op, Style, Transaction, VERSION,
};
use std::{
    cell::RefCell,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    rc::Rc,
};
#[cfg(feature = "native-image-tests")]
#[path = "calendar_appearance_test.rs"]
mod appearance;
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn date(year: i64, month: i64, day: i64) -> c::Date {
    c::Date::from_ymd(year, month, day).unwrap()
}
fn config() -> c::Config {
    c::Config {
        mode: c::Mode::Single,
        constraints: c::Constraints::unrestricted(),
        first_weekday: 1,
        labels: c::Labels::english(),
        today: Some(date(2024, 2, 29)),
        label: "Review calendar".into(),
        disabled: false,
        read_only: false,
        auto_focus: true,
    }
}
fn set(config: c::Config) -> Op {
    Op::SetCalendar(
        node(),
        Box::new(config),
        c::Selection::Empty,
        c::Month::new(2024, 2).unwrap(),
    )
}
fn apply(cx: &mut AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |v, w, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, w, cx);
            cx.notify();
        })
        .unwrap();
}
fn snapshot(cx: &mut AsyncApp, handle: WindowHandle<View>) -> c::Snapshot {
    handle
        .update(cx, |v, _, cx| {
            v.calendars[&node()].state.read(cx).model.snapshot()
        })
        .unwrap()
}
fn events(transport: &Transport) -> Vec<c::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|e| match e {
            Event::CalendarEvent(_, _, _, _, e) => Some(e),
            Event::Press(..) | Event::EditorEvent(..) | Event::Overloaded(_) => {
                panic!("unexpected calendar route: {e:?}")
            }
            _ => None,
        })
        .collect()
}
async fn click(cx: &mut AsyncApp, handle: WindowHandle<View>, x: f32, y: f32) {
    let p = point(px(x), px(y));
    move_mouse(cx, handle, p, false);
    frame(cx, handle).await;
    mouse(cx, handle, p, true);
    mouse(cx, handle, p, false);
    frame(cx, handle).await;
}
#[cfg(feature = "native-image-tests")]
fn capture(cx: &mut AsyncApp, handle: WindowHandle<View>, name: &str) {
    let Ok(directory) = std::env::var("GPUIO_CALENDAR_SCREENSHOT_DIR") else {
        return;
    };
    let image = cx
        .update_window(handle.into(), |_, w, cx| {
            w.draw(cx).clear(cx);
            w.render_to_image().unwrap()
        })
        .unwrap();
    std::fs::create_dir_all(&directory).unwrap();
    image
        .save(std::path::Path::new(&directory).join(format!("{name}.png")))
        .unwrap();
}
#[cfg(not(feature = "native-image-tests"))]
fn capture(_: &mut AsyncApp, _: WindowHandle<View>, _: &str) {}

async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(), Kind::Calendar, "".into(), Some(handler())),
            set(config()),
            Op::SetStyle(
                node(),
                vec![Style::Fields(vec![
                    Field::Background(Fill::Solid(Color::Rgba(0xf4f6faff))),
                    Field::Foreground(Color::Rgba(0x182332ff)),
                ])],
            ),
            Op::SetRoot(Some(node())),
        ],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).focused);
    assert_eq!(snapshot(cx, handle).focused_date, date(2024, 2, 29));
    capture(cx, handle, "calendar-days-light");
    let initial = events(transport);
    assert_eq!(
        initial
            .iter()
            .filter(|e| matches!(e,c::Event::Observed(s) if s.revision==0))
            .count(),
        1
    );
    key(cx, handle, "right");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).focused_date, date(2024, 3, 1));
    assert_eq!(snapshot(cx, handle).selection, c::Selection::Empty);
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    let selected = events(transport);
    assert!(
        matches!(selected.as_slice(),[c::Event::Changed(a),c::Event::Selected(b)] if b.revision==a.revision+1 && b.selection==c::Selection::Single(date(2024,3,1)))
    );
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(events(transport).is_empty());
    key(cx, handle, "pageup");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).month, c::Month::new(2024, 2).unwrap());
    assert_eq!(
        snapshot(cx, handle).selection,
        c::Selection::Single(date(2024, 3, 1))
    );
    // Actual pointer dispatch to the month title, then keyboard selection.
    click(cx, handle, 110., 24.).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Months);
    capture(cx, handle, "calendar-months-light");
    key(cx, handle, "right");
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Days);
    assert_eq!(snapshot(cx, handle).month, c::Month::new(2024, 3).unwrap());
    click(cx, handle, 180., 24.).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Years);
    capture(cx, handle, "calendar-years-light");
    key(cx, handle, "right");
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Months);
    assert_eq!(snapshot(cx, handle).month.year(), 2025);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    events(transport);
    key(cx, handle, "m");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Months);
    key(cx, handle, "y");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Years);
    key(cx, handle, "d");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Days);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![
                Field::Background(Fill::Solid(Color::Rgba(0x161b22ff))),
                Field::Foreground(Color::Rgba(0xd7e0edff)),
            ])],
        )],
    );
    frame(cx, handle).await;
    capture(cx, handle, "calendar-days-dark");
    let retained = snapshot(cx, handle);
    let mut updated = config();
    updated.read_only = true;
    updated.labels.months[2] = "März".into();
    updated.constraints = c::Constraints::new(
        c::Date::MIN,
        c::Date::MAX,
        vec![date(2024, 3, 1)],
        vec![],
        vec![],
        c::RangePolicy::EveryDay,
    )
    .unwrap();
    apply(cx, handle, vec![set(updated.clone())]);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).selection, retained.selection);
    assert_eq!(snapshot(cx, handle).month, retained.month);
    assert!(!snapshot(cx, handle).selection_allowed);
    key(cx, handle, "right");
    frame(cx, handle).await;
    assert_ne!(snapshot(cx, handle).focused_date, retained.focused_date);
    let readonly = snapshot(cx, handle);
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), readonly);
    assert!(events(transport).is_empty());
    updated.disabled = true;
    apply(cx, handle, vec![set(updated)]);
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(!snapshot(cx, handle).focused);
    let disabled = snapshot(cx, handle);
    key(cx, handle, "right");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), disabled);
    events(transport);
    let command = |cx: &mut AsyncApp, command: c::Command| {
        handle
            .update(cx, |v, w, cx| v.calendars[&node()].command(&command, w, cx))
            .unwrap()
    };
    assert_eq!(
        command(cx, c::Command::ReadSnapshot),
        c::Response::Applied(disabled.clone())
    );
    assert_eq!(
        command(cx, c::Command::FocusDate(date(2030, 1, 1))),
        c::Response::Failed(c::Error::FocusBlocked)
    );
    assert_eq!(snapshot(cx, handle), disabled);
    let result = command(
        cx,
        c::Command::Clear {
            if_revision: Some(disabled.revision),
        },
    );
    assert!(matches!(result, c::Response::Applied(ref s) if s.selection == c::Selection::Empty));
    assert!(
        matches!(events(transport).as_slice(), [c::Event::Observed(s)] if s.selection == c::Selection::Empty)
    );
    assert_eq!(
        command(
            cx,
            c::Command::Clear {
                if_revision: Some(disabled.revision)
            }
        ),
        c::Response::Failed(c::Error::StaleRevision)
    );
    assert!(events(transport).is_empty());
    // Programmatic focus followed by ancestor visibility cleanup must update the
    // snapshot even when GPUI drops the focused dispatch node before on_blur.
    let mut enabled = config();
    enabled.auto_focus = false;
    apply(cx, handle, vec![set(enabled)]);
    frame(cx, handle).await;
    assert!(matches!(command(cx, c::Command::Focus), c::Response::Applied(s) if s.focused));
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(!snapshot(cx, handle).focused);
    assert_eq!(
        command(cx, c::Command::Focus),
        c::Response::Failed(c::Error::FocusBlocked)
    );
    exercise_boundaries(cx, handle, transport).await;
    #[cfg(feature = "native-image-tests")]
    appearance::exercise(cx, handle, transport).await;
    let old_owner = handle
        .update(cx, |v, _, _| v.calendars[&node()].state.downgrade())
        .unwrap();
    apply(cx, handle, vec![Op::SetRoot(None), Op::Remove(node())]);
    frame(cx, handle).await;
    assert!(old_owner.upgrade().is_none());
    exercise_range(cx, handle, transport).await;
    handle
        .update(cx, |v, w, cx| {
            v.session.borrow_mut().close(v.id).unwrap();
            v.update_editors(&[], w, cx);
            assert!(v.calendars.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            w.remove_window();
            cx.notify();
        })
        .unwrap();
    eprintln!(
        "GPUIO_CALENDAR_NATIVE_OK: native keyboard/pointer navigation, completion order, retained history, read-only/disabled and disposal"
    );
}

async fn exercise_boundaries(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut localized = config();
    localized.auto_focus = false;
    localized.first_weekday = 0;
    localized.labels.months[0] = "Janvier".into();
    localized.labels.months[1] = "Février".into();
    localized.labels.months[2] = "März".into();
    localized.labels.months[11] = "Décembre".into();
    localized.labels.today = "Aujourd’hui".into();
    apply(
        cx,
        handle,
        vec![
            set(localized.clone()),
            Op::SetStyle(
                node(),
                vec![Style::Fields(vec![
                    Field::Background(Fill::Solid(Color::Rgba(0xf4f6faff))),
                    Field::Foreground(Color::Rgba(0x182332ff)),
                ])],
            ),
        ],
    );
    frame(cx, handle).await;
    let command = |cx: &mut AsyncApp, command: c::Command| {
        handle
            .update(cx, |v, w, cx| v.calendars[&node()].command(&command, w, cx))
            .unwrap()
    };
    let applied = |response| match response {
        c::Response::Applied(s) => s,
        other => panic!("calendar boundary command failed: {other:?}"),
    };
    applied(command(cx, c::Command::Clear { if_revision: None }));
    applied(command(cx, c::Command::FocusDate(c::Date::MIN)));
    frame(cx, handle).await;
    events(transport);
    let minimum = snapshot(cx, handle);
    for key_name in ["left", "up", "pageup", "home"] {
        key(cx, handle, key_name);
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle), minimum, "minimum boundary {key_name}");
        assert!(events(transport).is_empty());
    }
    capture(cx, handle, "calendar-minimum-localized");
    applied(command(
        cx,
        c::Command::SetPresentation(c::Presentation::Years),
    ));
    frame(cx, handle).await;
    events(transport);
    let years = snapshot(cx, handle);
    key(cx, handle, "left");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), years);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).presentation, c::Presentation::Months);
    events(transport);
    let months = snapshot(cx, handle);
    key(cx, handle, "left");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), months);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    key(cx, handle, "right");
    frame(cx, handle).await;
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle).selection,
        c::Selection::Single(date(1, 1, 2))
    );
    assert!(matches!(
        events(transport).as_slice(),
        [c::Event::Changed(_), c::Event::Selected(_)]
    ));

    applied(command(cx, c::Command::FocusDate(c::Date::MAX)));
    frame(cx, handle).await;
    events(transport);
    let maximum = snapshot(cx, handle);
    for key_name in ["right", "down", "pagedown", "end"] {
        key(cx, handle, key_name);
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle), maximum, "maximum boundary {key_name}");
        assert!(events(transport).is_empty());
    }
    capture(cx, handle, "calendar-maximum-localized");
    for presentation in [c::Presentation::Months, c::Presentation::Years] {
        applied(command(cx, c::Command::SetPresentation(presentation)));
        frame(cx, handle).await;
        events(transport);
        let before = snapshot(cx, handle);
        key(cx, handle, "right");
        key(cx, handle, "pagedown");
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle), before);
        assert!(events(transport).is_empty());
    }
    capture(cx, handle, "calendar-maximum-years");
    applied(command(cx, c::Command::FocusDate(date(2024, 2, 29))));
    let clamped = applied(command(
        cx,
        c::Command::ShowMonth(c::Month::new(2025, 2).unwrap()),
    ));
    assert_eq!(clamped.focused_date, date(2025, 2, 28));
    assert_eq!(clamped.selection, c::Selection::Single(date(1, 1, 2)));
    // Locale/week-start updates are configuration changes, never date parsing.
    localized.first_weekday = 1;
    localized.labels.months[1] = "February with a deliberately long localized label".into();
    apply(cx, handle, vec![set(localized)]);
    frame(cx, handle).await;
    let retained = snapshot(cx, handle);
    assert_eq!(retained.selection, clamped.selection);
    assert_eq!(retained.focused_date, clamped.focused_date);
    assert_eq!(retained.month, clamped.month);
    capture(cx, handle, "calendar-long-label");
    events(transport);
    eprintln!(
        "GPUIO_CALENDAR_BOUNDARY_OK: civil endpoints, day/month/year guards, leap clamping, locale and retained selection"
    );
}

async fn exercise_range(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let id = NodeId::from_parts(1, 1).unwrap();
    let handler = HandlerId::from_parts(1, 1).unwrap();
    let snapshot = |cx: &mut AsyncApp| {
        handle
            .update(cx, |v, _, cx| {
                v.calendars[&id].state.read(cx).model.snapshot()
            })
            .unwrap()
    };
    apply(
        cx,
        handle,
        vec![
            Op::Create(id, Kind::Calendar, "".into(), Some(handler)),
            Op::SetCalendar(
                id,
                Box::new(c::Config {
                    mode: c::Mode::Range,
                    ..config()
                }),
                c::Selection::RangeStart(date(2024, 2, 29)),
                c::Month::new(2024, 2).unwrap(),
            ),
            Op::SetCalendar(
                id,
                Box::new(c::Config {
                    mode: c::Mode::Range,
                    constraints: c::Constraints::new(
                        c::Date::MIN,
                        c::Date::MAX,
                        vec![date(2024, 2, 29)],
                        vec![],
                        vec![],
                        c::RangePolicy::EveryDay,
                    )
                    .unwrap(),
                    ..config()
                }),
                c::Selection::Empty,
                c::Month::new(2030, 1).unwrap(),
            ),
            Op::SetRoot(Some(id)),
        ],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    events(transport);
    assert!(snapshot(cx).focused);
    assert_eq!(
        snapshot(cx).selection,
        c::Selection::RangeStart(date(2024, 2, 29))
    );
    assert!(!snapshot(cx).selection_allowed);
    assert_eq!(snapshot(cx).month, c::Month::new(2024, 2).unwrap());
    apply(
        cx,
        handle,
        vec![Op::SetCalendar(
            id,
            Box::new(c::Config {
                mode: c::Mode::Range,
                ..config()
            }),
            c::Selection::Empty,
            c::Month::new(2030, 1).unwrap(),
        )],
    );
    frame(cx, handle).await;
    assert!(snapshot(cx).selection_allowed);
    key(cx, handle, "backspace");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx).selection, c::Selection::Empty);
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        matches!(events(transport).as_slice(),[c::Event::Changed(s)] if s.selection==c::Selection::RangeStart(date(2024,2,29)))
    );
    key(cx, handle, "right");
    frame(cx, handle).await;
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        matches!(events(transport).as_slice(),[c::Event::Changed(a),c::Event::Selected(b)] if a.revision+1==b.revision && b.selection==c::Selection::Range(c::Range::new(date(2024,2,29),date(2024,3,1)).unwrap()))
    );
    let selected = snapshot(cx).selection;
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id,
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(!snapshot(cx).focused);
    let hidden = snapshot(cx);
    key(cx, handle, "right");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx), hidden);
    apply(cx, handle, vec![Op::SetStyle(id, vec![])]);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx).selection, selected);
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.calendars[&id].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    // Start another range, move to its endpoint, then leave exactly one queue
    // slot. A two-event completion must fault without exposing half a pair.
    key(cx, handle, "enter");
    frame(cx, handle).await;
    key(cx, handle, "right");
    frame(cx, handle).await;
    events(transport);
    let before = snapshot(cx);
    assert!(matches!(before.selection, c::Selection::RangeStart(_)));
    handle
        .update(cx, |v, _, _| {
            let revision = v.session.borrow().tree(v.id).unwrap().revision();
            for _ in 0..crate::mailbox::MAX_INPUT_EVENTS - 1 {
                assert!(transport.input(Event::CalendarEvent(
                    v.id,
                    id,
                    handler,
                    revision,
                    c::Event::Observed(before.clone())
                )));
            }
        })
        .unwrap();
    key(cx, handle, "enter");
    frame(cx, handle).await;
    let after = snapshot(cx);
    assert!(after.selection.is_complete());
    let drained = transport.mailbox.lock().unwrap().drain(256);
    assert_eq!(
        drained
            .iter()
            .filter(|e| matches!(e, Event::Overloaded(_)))
            .count(),
        1
    );
    assert!(!drained.iter().any(|e| matches!(
        e,
        Event::CalendarEvent(_, _, _, _, c::Event::Changed(_) | c::Event::Selected(_))
    )));
    key(cx, handle, "left");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx), after);
    handle
        .update(cx, |v, _, _| {
            assert!(!v.session.borrow().accepts_input(v.id))
        })
        .unwrap();
    handle
        .update(cx, |v, w, cx| {
            assert_eq!(
                v.calendars[&id].command(&c::Command::Clear { if_revision: None }, w, cx),
                c::Response::Failed(c::Error::NativeFailure)
            );
            assert_eq!(v.calendars[&id].state.read(cx).model.snapshot(), after);
        })
        .unwrap();
    eprintln!(
        "GPUIO_CALENDAR_RANGE_PRESSURE_OK: partial/complete ordering, hidden focus isolation and atomic pair overload"
    );
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        gpui_base::init(cx);
        cx.set_quit_mode(QuitMode::Explicit);
        let id = WindowId::from_parts(0, 1).unwrap();
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, id, "GPUIO Calendar test", 340., 360.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    inactive_frame_interval: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(340.), px(360.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::super::native_test::protect(exercise(cx, handle, &transport)).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
